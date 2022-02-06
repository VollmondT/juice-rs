use std::ffi::{CStr, CString};
use std::os::raw::c_int;
use std::ptr;
use std::sync::Mutex;

use libjuice_sys as sys;

use crate::agent_config::Config;
use crate::agent_error::AgentError;
use crate::agent_state::AgentState;
use crate::ice_hander::Handler;
use crate::log::ensure_logging;

type Result<T> = std::result::Result<T, AgentError>;

/// Convert c function retcode to result
fn raw_retcode_to_result(retcode: c_int) -> Result<()> {
    match retcode {
        0 => Ok(()),
        sys::JUICE_ERR_INVALID => Err(AgentError::InvalidArgument),
        sys::JUICE_ERR_FAILED => Err(AgentError::Failed),
        sys::JUICE_ERR_NOT_AVAIL => Err(AgentError::NotAvailable),
        _ => unreachable!(),
    }
}

/// Agent builder.
pub struct Builder {
    stun_server: String,
    stun_server_port: u16,
    port_range: Option<(u16, u16)>,
    handler: Box<Handler>,
}

impl Builder {
    /// Create new builder with given handler
    pub fn new(handler: Box<Handler>) -> Self {
        Builder {
            stun_server: String::from("stun.l.google.com"),
            stun_server_port: 19302,
            port_range: None,
            handler,
        }
    }

    /// Build agent
    pub fn build(self) -> Result<Agent> {
        ensure_logging();

        let mut holder = Box::new(Holder {
            agent: ptr::null_mut(),
            handler: Mutex::new(self.handler),
        });

        let cfg = Config {
            stun_server_host: CString::new(self.stun_server).expect("invalid stun server host"),
            stun_server_port: self.stun_server_port,
            parent: &holder as _,
            port_range: self.port_range,
        };
        let ptr = unsafe { sys::juice_create(&cfg.as_raw() as *const _) };
        if ptr.is_null() {
            Err(AgentError::Failed)
        } else {
            holder.agent = ptr;
            Ok(Agent { holder })
        }
    }
}

pub struct Agent {
    holder: Box<Holder>,
}
impl Agent {
    /// Get ICE state
    pub fn get_state(&self) -> AgentState {
        unsafe {
            sys::juice_get_state(self.holder.agent)
                .try_into()
                .expect("failed to convert state")
        }
    }

    /// Get local sdp
    pub fn get_local_description(&self) -> Result<String> {
        let mut buf = vec![0; sys::JUICE_MAX_SDP_STRING_LEN as _];
        let res = unsafe {
            let res = sys::juice_get_local_description(
                self.holder.agent,
                buf.as_mut_ptr(),
                buf.len() as _,
            );
            let _ = raw_retcode_to_result(res)?;
            let s = CStr::from_ptr(buf.as_mut_ptr());
            String::from_utf8_lossy(s.to_bytes())
        };
        Ok(res.to_string())
    }

    /// Start ICE candidates gathering
    pub fn gather_candidates(&self) -> Result<()> {
        let ret = unsafe { sys::juice_gather_candidates(self.holder.agent) };
        raw_retcode_to_result(ret)
    }

    /// Set remote description
    pub fn set_remote_description(&self, sdp: String) -> Result<()> {
        let s = CString::new(sdp).map_err(|_| AgentError::InvalidArgument)?;
        let ret = unsafe { sys::juice_set_remote_description(self.holder.agent, s.as_ptr()) };
        raw_retcode_to_result(ret)
    }

    /// Add remote candidate
    pub fn add_remote_candidate(&self, sdp: String) -> Result<()> {
        let s = CString::new(sdp).map_err(|_| AgentError::InvalidArgument)?;
        let ret = unsafe { sys::juice_add_remote_candidate(self.holder.agent, s.as_ptr()) };
        raw_retcode_to_result(ret)
    }

    /// Signal remote candidates exhausted
    pub fn set_remote_gathering_done(&self) -> Result<()> {
        let ret = unsafe { sys::juice_set_remote_gathering_done(self.holder.agent) };
        raw_retcode_to_result(ret)
    }

    /// Send packet to remote endpoint
    pub fn send(&self, data: &[u8]) -> Result<()> {
        let ret =
            unsafe { sys::juice_send(self.holder.agent, data.as_ptr() as _, data.len() as _) };
        raw_retcode_to_result(ret)
    }

    /// Get selected candidates pair (local,remote)
    pub fn get_selected_candidates(&self) -> Result<(String, String)> {
        let mut local = vec![0; sys::JUICE_MAX_SDP_STRING_LEN as _];
        let mut remote = vec![0; sys::JUICE_MAX_SDP_STRING_LEN as _];
        let ret = unsafe {
            let res = sys::juice_get_selected_candidates(
                self.holder.agent,
                local.as_mut_ptr() as _,
                local.len() as _,
                remote.as_mut_ptr() as _,
                remote.len() as _,
            );
            let _ = raw_retcode_to_result(res)?;
            let l = CStr::from_ptr(local.as_mut_ptr());
            let r = CStr::from_ptr(remote.as_mut_ptr());
            (
                String::from_utf8_lossy(l.to_bytes()).to_string(),
                String::from_utf8_lossy(r.to_bytes()).to_string(),
            )
        };
        Ok(ret)
    }
}

pub(crate) struct Holder {
    agent: *mut sys::juice_agent_t,
    handler: Mutex<Box<Handler>>,
}

impl Drop for Holder {
    fn drop(&mut self) {
        unsafe { sys::juice_destroy(self.agent) }
    }
}

// SAFETY: All juice calls protected by mutex internally and can be invoked from any thread
unsafe impl Sync for Holder {}

unsafe impl Send for Holder {}

impl Holder {
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

    pub(crate) fn on_recv(&self, packet: &[u8]) {
        let mut h = self.handler.lock().unwrap();
        h.on_recv(packet)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};

    use crate::agent_state::AgentState;
    use crate::Handler;

    use super::*;

    #[test]
    fn build() {
        crate::test_util::logger_init();

        let handler = Handler::default();
        let agent = Builder::new(handler.to_box()).build().unwrap();

        assert_eq!(agent.get_state(), AgentState::Disconnected);
        log::debug!(
            "local description \n\"{}\"",
            agent.get_local_description().unwrap()
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

        let agent = Builder::new(handler.to_box()).build().unwrap();

        assert_eq!(agent.get_state(), AgentState::Disconnected);
        log::debug!(
            "local description \n\"{}\"",
            agent.get_local_description().unwrap()
        );

        agent.gather_candidates().unwrap();
        assert_eq!(agent.get_state(), AgentState::Gathering);

        let _ = gathering_barrier.wait();

        log::debug!(
            "local description \n\"{}\"",
            agent.get_local_description().unwrap()
        );
    }
}
