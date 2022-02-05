use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::sync::{Arc, Mutex};

use libjuice_sys as sys;

use crate::agent_config::Config;
use crate::agent_error::AgentError;
use crate::agent_state::AgentState;
use crate::ice_hander::Handler;
use crate::log::ensure_logging;

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
    handler: Box<Handler>,
}

impl Builder {
    pub fn new(handler: Box<Handler>) -> Self {
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
            _agent: PhantomData::default(),
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
    handler: Arc<Mutex<Box<Handler>>>,
    _agent: PhantomData<sys::juice_agent_t>,
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
        let mut buf: Vec<c_char> = vec![0; sys::JUICE_MAX_SDP_STRING_LEN as usize];
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

    pub fn gather_candidates(&mut self) -> Result<()> {
        unsafe { raw_to_result(sys::juice_gather_candidates(self.agent)) }
    }

    pub fn set_remote_description(&mut self, sdp: String) -> Result<()> {
        let s = CString::new(sdp).map_err(|_| AgentError::InvalidArgument)?;
        unsafe { raw_to_result(sys::juice_set_remote_description(self.agent, s.as_ptr())) }
    }

    pub(crate) fn on_state_changed(&self, state: AgentState) {
        let mut h = self.handler.lock().unwrap();
        h.on_state_changed(state)
    }

    pub(crate) fn on_candidate(&self, candidate: String) {
        let mut h = self.handler.lock().unwrap();
        h.on_candidate(candidate)
    }

    pub(crate) fn on_gathering_done(&self) {
        let mut h = self.handler.lock().unwrap();
        h.on_gathering_done()
    }

    pub(crate) fn on_recv(&self) {
        let mut h = self.handler.lock().unwrap();
        h.on_recv()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Barrier;

    use crate::agent_state::AgentState;
    use crate::Handler;

    use super::*;

    #[test]
    fn build() {
        crate::test_util::logger_init();

        let handler = Handler::default();
        let agent = Builder::new(handler.to_box()).build();

        assert_eq!(agent.state(), AgentState::Disconnected);
        log::debug!(
            "local description \n\"{}\"",
            agent.local_description().unwrap()
        );
    }

    #[test]
    fn gather() {
        crate::test_util::logger_init();

        let gathering_barrier = Arc::new(Barrier::new(2));

        let handler = Handler::default()
            .state_handler(|state| log::debug!("State changed to: {:?}", state))
            .gathering_finished_handler({
                let barrier = gathering_barrier.clone();
                move || {
                    log::debug!("Gathering finished");
                    barrier.wait();
                }
            })
            .candidate_handler(|candidate| log::debug!("Local candidate: \"{}\"", candidate));

        let mut agent = Builder::new(handler.to_box()).build();

        assert_eq!(agent.state(), AgentState::Disconnected);
        log::debug!(
            "local description \n\"{}\"",
            agent.local_description().unwrap()
        );

        agent.gather_candidates().unwrap();
        assert_eq!(agent.state(), AgentState::Gathering);

        let _ = gathering_barrier.wait();

        log::debug!(
            "local description \n\"{}\"",
            agent.local_description().unwrap()
        );
    }
}
