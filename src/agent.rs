use std::ffi::CString;
use std::ptr;
use std::sync::{Arc, Mutex};

use libjuice_sys as sys;

use crate::agent_config::Config;
use crate::log::ensure_logging;
use crate::IceHander;

pub struct Builder {
    stun_server: String,
    stun_server_port: u16,
    port_range: Option<(u16, u16)>,
    handler: Box<dyn IceHander>,
}

impl Builder {
    pub fn new(handler: Box<dyn IceHander>) -> Self {
        Builder {
            stun_server: String::from("stun.l.google.com"),
            stun_server_port: 19302,
            port_range: None,
            handler,
        }
    }

    pub fn build(self) -> Box<Agent> {
        ensure_logging();
        let mut agent = Box::new(Agent {
            agent: ptr::null_mut(),
            handler: Arc::new(Mutex::new(self.handler)),
        });
        let cfg = Config {
            stun_server_host: CString::new(self.stun_server).expect("invalid stun server host"),
            stun_server_port: self.stun_server_port,
            parent: agent.as_mut(),
            port_range: self.port_range,
        };
        let ptr = unsafe { sys::juice_create(&cfg.as_raw() as *const _) };
        agent.agent = ptr;
        agent
    }
}

pub struct Agent {
    agent: *mut sys::juice_agent_t,
    handler: Arc<Mutex<Box<dyn IceHander>>>,
}

impl Drop for Agent {
    fn drop(&mut self) {
        unsafe { sys::juice_destroy(self.agent) }
    }
}

impl Agent {
    pub(crate) fn on_state_changed(&self) {
        let mut h = self.handler.lock().unwrap();
        h.on_state_changed()
    }

    pub(crate) fn on_candidate(&self) {
        todo!()
    }

    pub(crate) fn on_gathering_done(&self) {
        todo!()
    }

    pub(crate) fn on_recv(&self) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::IceHander;

    use super::*;

    struct DummyHandler {}

    impl IceHander for DummyHandler {
        fn on_state_changed(&mut self) {
            todo!()
        }

        fn on_candidate(&mut self) {
            todo!()
        }

        fn on_gathering_done(&mut self) {
            todo!()
        }

        fn on_recv(&mut self) {
            todo!()
        }
    }

    #[test]
    fn build() {
        crate::test_util::logger_init();
        let _ = Builder::new(Box::new(DummyHandler {})).build();
    }
}
