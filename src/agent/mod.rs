mod config;
pub mod handler;

use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::os::raw::c_int;
use std::ptr;
use std::sync::Mutex;

use libjuice_sys as sys;

use config::Config;
use handler::Handler;

use crate::error::Error;
use crate::log::ensure_logging;
use crate::Result;

/// Convert c function retcode to result
fn raw_retcode_to_result(retcode: c_int) -> Result<()> {
    match retcode {
        0 => Ok(()),
        sys::JUICE_ERR_INVALID => Err(Error::InvalidArgument),
        sys::JUICE_ERR_FAILED => Err(Error::Failed),
        sys::JUICE_ERR_NOT_AVAIL => Err(Error::NotAvailable),
        _ => unreachable!(),
    }
}

/// Agent builder.
pub struct Builder {
    stun_server: Option<StunServer>,
    port_range: Option<(u16, u16)>,
    handler: Handler,
}

impl Builder {
    /// Create new builder with given handler
    fn new(handler: Handler) -> Self {
        Builder {
            stun_server: None,
            port_range: None,
            handler,
        }
    }

    /// Set alternative stun server (default is "stun.l.google.com:19302")
    pub fn set_stun(mut self, host: String, port: u16) -> Self {
        self.stun_server = Some(StunServer::new(host, port).unwrap());
        self
    }

    /// Set port range
    pub fn set_port_range(mut self, begin: u16, end: u16) -> Self {
        self.port_range = Some((begin, end));
        self
    }

    /// Build agent
    pub fn build(self) -> crate::Result<Agent> {
        ensure_logging();

        let mut holder = Box::new(Holder {
            agent: ptr::null_mut(),
            handler: Mutex::new(self.handler),
            _marker: PhantomData::default(),
        });

        let cfg = Config {
            stun_server: self.stun_server.unwrap_or_default(),
            parent: &holder as _,
            port_range: self.port_range,
        };
        let ptr = unsafe { sys::juice_create(&cfg.as_raw() as *const _) };
        if ptr.is_null() {
            Err(Error::Failed)
        } else {
            holder.agent = ptr;
            Ok(Agent { holder })
        }
    }
}

/// ICE agent.
pub struct Agent {
    holder: Box<Holder>,
}

impl Agent {
    /// Create agent builder
    pub fn builder(h: Handler) -> Builder {
        Builder::new(h)
    }

    /// Get ICE state
    pub fn get_state(&self) -> State {
        unsafe {
            sys::juice_get_state(self.holder.agent)
                .try_into()
                .expect("failed to convert state")
        }
    }

    /// Get local sdp
    pub fn get_local_description(&self) -> crate::Result<String> {
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
    pub fn gather_candidates(&self) -> crate::Result<()> {
        let ret = unsafe { sys::juice_gather_candidates(self.holder.agent) };
        raw_retcode_to_result(ret)
    }

    /// Set remote description
    pub fn set_remote_description(&self, sdp: String) -> crate::Result<()> {
        let s = CString::new(sdp).map_err(|_| Error::InvalidArgument)?;
        let ret = unsafe { sys::juice_set_remote_description(self.holder.agent, s.as_ptr()) };
        raw_retcode_to_result(ret)
    }

    /// Add remote candidate
    pub fn add_remote_candidate(&self, sdp: String) -> crate::Result<()> {
        let s = CString::new(sdp).map_err(|_| Error::InvalidArgument)?;
        let ret = unsafe { sys::juice_add_remote_candidate(self.holder.agent, s.as_ptr()) };
        raw_retcode_to_result(ret)
    }

    /// Signal remote candidates exhausted
    pub fn set_remote_gathering_done(&self) -> crate::Result<()> {
        let ret = unsafe { sys::juice_set_remote_gathering_done(self.holder.agent) };
        raw_retcode_to_result(ret)
    }

    /// Send packet to remote endpoint
    pub fn send(&self, data: &[u8]) -> crate::Result<()> {
        let ret =
            unsafe { sys::juice_send(self.holder.agent, data.as_ptr() as _, data.len() as _) };
        raw_retcode_to_result(ret)
    }

    /// Get selected candidates pair (local,remote)
    pub fn get_selected_candidates(&self) -> crate::Result<(String, String)> {
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

    pub fn get_selected_addresses(&self) -> crate::Result<(String, String)> {
        let mut local = vec![0; sys::JUICE_MAX_SDP_STRING_LEN as _];
        let mut remote = vec![0; sys::JUICE_MAX_SDP_STRING_LEN as _];
        let ret = unsafe {
            let res = sys::juice_get_selected_addresses(
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
    handler: Mutex<Handler>,
    _marker: PhantomData<(sys::juice_agent, std::marker::PhantomPinned)>,
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
    pub(crate) fn on_state_changed(&self, state: State) {
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum State {
    Disconnected,
    Gathering,
    Connecting,
    Connected,
    Completed,
    Failed,
}

impl TryFrom<sys::juice_state> for State {
    type Error = ();

    fn try_from(value: sys::juice_state) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            sys::juice_state_JUICE_STATE_DISCONNECTED => State::Disconnected,
            sys::juice_state_JUICE_STATE_GATHERING => State::Gathering,
            sys::juice_state_JUICE_STATE_CONNECTING => State::Connecting,
            sys::juice_state_JUICE_STATE_CONNECTED => State::Connected,
            sys::juice_state_JUICE_STATE_COMPLETED => State::Completed,
            sys::juice_state_JUICE_STATE_FAILED => State::Failed,
            _ => return Err(()),
        })
    }
}

/// Stun server (host:port)
pub(crate) struct StunServer(pub(crate) CString, pub(crate) u16);

impl Default for StunServer {
    fn default() -> Self {
        Self(CString::new("stun.l.google.com").unwrap(), 19302)
    }
}

impl StunServer {
    /// Construct from `std::String` and port value
    pub(crate) fn new(host: String, port: u16) -> std::result::Result<Self, std::ffi::NulError> {
        Ok(Self(CString::new(host)?, port))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Handler;
    use std::sync::{Arc, Barrier};

    #[test]
    fn build() {
        crate::test_util::logger_init();

        let handler = Handler::default();
        let agent = Agent::builder(handler).build().unwrap();

        assert_eq!(agent.get_state(), State::Disconnected);
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

        let agent = Agent::builder(handler).build().unwrap();

        assert_eq!(agent.get_state(), State::Disconnected);
        log::debug!(
            "local description \n\"{}\"",
            agent.get_local_description().unwrap()
        );

        agent.gather_candidates().unwrap();
        assert_eq!(agent.get_state(), State::Gathering);

        let _ = gathering_barrier.wait();

        log::debug!(
            "local description \n\"{}\"",
            agent.get_local_description().unwrap()
        );
    }
}
