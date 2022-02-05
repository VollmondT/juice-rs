use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::sync::{Arc, Mutex};

use libjuice_sys as sys;

use crate::agent_config::Config;
use crate::agent_error::AgentError;
use crate::agent_state::AgentState;
use crate::log::ensure_logging;
use crate::IceHander;

type Result<T> = std::result::Result<T, AgentError>;

fn raw_to_result(retcode: c_int) -> Result<()> {
    match retcode {
        0 => Ok(()),
        sys::JUICE_ERR_INVALID => Err(AgentError::InvalidArgument),
        sys::JUICE_ERR_FAILED => Err(AgentError::Failed),
        sys::JUICE_ERR_NOT_AVAIL => Err(AgentError::NotAvailable),
        _ => unreachable!(),
    }
}

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

unsafe impl Sync for Agent {}

impl Agent {
    pub fn state(&self) -> AgentState {
        unsafe {
            sys::juice_get_state(self.agent)
                .try_into()
                .expect("failed to convert state")
        }
    }

    pub fn local_description(&self) -> Result<String> {
        let mut buf: Vec<c_char> = Vec::with_capacity(sys::JUICE_MAX_SDP_STRING_LEN as usize);
        let res = unsafe {
            let res = sys::juice_get_local_description(
                self.agent,
                buf.as_mut_ptr(),
                buf.len() as sys::size_t,
            );
            let _ = raw_to_result(res)?;
            let s = CStr::from_ptr(buf.as_mut_ptr());
            String::from_utf8_lossy(s.to_bytes())
        };
        Ok(res.to_string())
    }

    pub(crate) fn on_state_changed(&self, state: AgentState) {
        let mut h = self.handler.lock().unwrap();
        h.on_state_changed(state)
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
    use crate::agent_state::AgentState;
    use crate::IceHander;

    use super::*;

    struct DummyHandler {}

    impl IceHander for DummyHandler {
        fn on_state_changed(&mut self, _state: AgentState) {
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
        let client = Builder::new(Box::new(DummyHandler {})).build();
        assert_eq!(client.state(), AgentState::Disconnected);
        let _ = client.local_description().unwrap();
    }
}
