//! ICE Agent.

pub mod handler;

use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::net::IpAddr;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::Mutex;

pub use handler::Handler;
use libjuice_sys as sys;

use crate::error::Error;
use crate::log::ensure_logging;
use crate::Result;

/// Convert c function retcode to result
fn raw_retcode_to_result(retcode: c_int) -> Result<()> {
    match retcode {
        //sys::JUICE_ERR_SUCCESS => Ok(()),
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
    bind_address: Option<CString>,
    turn_servers: Vec<TurnServer>,
    handler: Handler,
    concurrency_mode: ConcurrencyMode,
}

impl Builder {
    /// Create new builder with given handler
    fn new(handler: Handler) -> Self {
        Builder {
            stun_server: None,
            port_range: None,
            bind_address: None,
            turn_servers: vec![],
            handler,
            concurrency_mode: ConcurrencyMode::Poll,
        }
    }

    /// Set alternative stun server (default is "stun.l.google.com:19302")
    pub fn with_stun(mut self, host: String, port: u16) -> Self {
        self.stun_server = Some(StunServer::new(host, port).unwrap());
        self
    }

    /// Set port range
    pub fn with_port_range(mut self, begin: u16, end: u16) -> Self {
        self.port_range = Some((begin, end));
        self
    }

    /// Bind to specific address
    pub fn with_bind_address(mut self, addr: &IpAddr) -> Self {
        self.bind_address = Some(CString::new(addr.to_string()).unwrap()); // can't fail
        self
    }

    /// Add TURN server
    pub fn add_turn_server<T>(mut self, host: T, port: u16, user: T, pass: T) -> Result<Self>
    where
        T: Into<Vec<u8>>,
    {
        let server = TurnServer {
            host: CString::new(host).map_err(|_| Error::InvalidArgument)?,
            port,
            username: CString::new(user).map_err(|_| Error::InvalidArgument)?,
            password: CString::new(pass).map_err(|_| Error::InvalidArgument)?,
        };
        self.turn_servers.push(server);

        Ok(self)
    }

    pub fn concurrency(mut self, mode: ConcurrencyMode) -> Self {
        self.concurrency_mode = mode;
        self
    }

    /// Build agent
    pub fn build(self) -> Result<Agent> {
        ensure_logging();

        let mut holder = Box::new(Holder {
            agent: ptr::null_mut(),
            handler: Mutex::new(self.handler),
            _marker: PhantomData::default(),
        });

        // [0..0] == no range
        let port_range = self.port_range.unwrap_or((0, 0));
        // default is google
        let stun_server = self.stun_server.unwrap_or_default();
        let bind_address = self
            .bind_address
            .as_ref()
            .map(|v| v.as_ptr())
            .unwrap_or(ptr::null());

        let servers = self
            .turn_servers
            .iter()
            .map(|turn| sys::juice_turn_server {
                host: turn.host.as_ptr(),
                port: turn.port,
                username: turn.username.as_ptr(),
                password: turn.password.as_ptr(),
            })
            .collect::<Vec<_>>();

        let turn_servers = if servers.is_empty() {
            (ptr::null(), 0)
        } else {
            (servers.as_ptr(), servers.len() as _)
        };

        let config = &sys::juice_config {
            concurrency_mode: self.concurrency_mode.into(),
            stun_server_host: stun_server.0.as_ptr(),
            stun_server_port: stun_server.1,
            turn_servers: turn_servers.0 as _,
            turn_servers_count: turn_servers.1,
            bind_address,
            local_port_range_begin: port_range.0,
            local_port_range_end: port_range.1,
            cb_state_changed: Some(on_state_changed),
            cb_candidate: Some(on_candidate),
            cb_gathering_done: Some(on_gathering_done),
            cb_recv: Some(on_recv),
            user_ptr: holder.as_mut() as *mut Holder as _,
        };

        let ptr = unsafe { sys::juice_create(config as _) };
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
    pub fn get_local_description(&self) -> Result<String> {
        let mut buf = vec![0; sys::JUICE_MAX_SDP_STRING_LEN as _];
        let res = unsafe {
            let res = sys::juice_get_local_description(
                self.holder.agent,
                buf.as_mut_ptr(),
                buf.len() as _,
            );
            raw_retcode_to_result(res)?;
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
        let s = CString::new(sdp).map_err(|_| Error::InvalidArgument)?;
        let ret = unsafe { sys::juice_set_remote_description(self.holder.agent, s.as_ptr()) };
        raw_retcode_to_result(ret)
    }

    /// Add remote candidate
    pub fn add_remote_candidate(&self, sdp: String) -> Result<()> {
        let s = CString::new(sdp).map_err(|_| Error::InvalidArgument)?;
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
            raw_retcode_to_result(res)?;
            let l = CStr::from_ptr(local.as_mut_ptr());
            let r = CStr::from_ptr(remote.as_mut_ptr());
            (
                String::from_utf8_lossy(l.to_bytes()).to_string(),
                String::from_utf8_lossy(r.to_bytes()).to_string(),
            )
        };
        Ok(ret)
    }

    pub fn get_selected_addresses(&self) -> Result<(String, String)> {
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
            raw_retcode_to_result(res)?;
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Default)]
pub enum ConcurrencyMode {
    /// Single poll thread for all agents
    #[default]
    Poll,
    Mux,
    /// Thread per agent
    Thread,
}

impl From<ConcurrencyMode> for sys::juice_concurrency_mode {
    fn from(mode: ConcurrencyMode) -> Self {
        match mode {
            ConcurrencyMode::Poll => sys::juice_concurrency_mode_JUICE_CONCURRENCY_MODE_POLL,
            ConcurrencyMode::Mux => sys::juice_concurrency_mode_JUICE_CONCURRENCY_MODE_MUX,
            ConcurrencyMode::Thread => sys::juice_concurrency_mode_JUICE_CONCURRENCY_MODE_THREAD,
        }
    }
}

/// Stun server (host:port)
struct StunServer(CString, u16);

impl Default for StunServer {
    fn default() -> Self {
        Self(CString::new("stun.l.google.com").unwrap(), 19302)
    }
}

impl StunServer {
    /// Construct from host and port value
    fn new<T: Into<Vec<u8>>>(host: T, port: u16) -> Result<Self> {
        Ok(Self(
            CString::new(host).map_err(|_| Error::InvalidArgument)?,
            port,
        ))
    }
}

/// Turn server
struct TurnServer {
    pub host: CString,
    pub username: CString,
    pub password: CString,
    pub port: u16,
}

unsafe extern "C" fn on_state_changed(
    _: *mut sys::juice_agent_t,
    state: sys::juice_state_t,
    user_ptr: *mut c_void,
) {
    let agent: &Holder = &*(user_ptr as *const _);

    if let Err(e) = state.try_into().map(|s| agent.on_state_changed(s)) {
        log::error!("failed to map state {:?}", e)
    }
}

unsafe extern "C" fn on_candidate(
    _: *mut sys::juice_agent_t,
    sdp: *const c_char,
    user_ptr: *mut c_void,
) {
    let agent: &Holder = &*(user_ptr as *const _);
    let candidate = {
        let s = CStr::from_ptr(sdp);
        String::from_utf8_lossy(s.to_bytes())
    };
    agent.on_candidate(candidate.to_string())
}

unsafe extern "C" fn on_gathering_done(_: *mut sys::juice_agent_t, user_ptr: *mut c_void) {
    let agent: &Holder = &*(user_ptr as *const _);
    agent.on_gathering_done()
}

unsafe extern "C" fn on_recv(
    _: *mut sys::juice_agent_t,
    data: *const c_char,
    len: sys::size_t,
    user_ptr: *mut c_void,
) {
    let agent: &Holder = &*(user_ptr as *const _);
    let packet = core::slice::from_raw_parts(data as _, len as _);
    agent.on_recv(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::logger_init;
    use crate::Handler;
    use std::sync::{Arc, Barrier};

    #[test]
    fn build() {
        logger_init();

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
        logger_init();

        let gathering_barrier = Arc::new(Barrier::new(2));

        let handler = Handler::default()
            .state_handler(|state| log::debug!("State changed to: {:?}", state))
            .gathering_done_handler({
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

        let _ = gathering_barrier.wait();

        log::debug!(
            "local description \n\"{}\"",
            agent.get_local_description().unwrap()
        );
    }
}
