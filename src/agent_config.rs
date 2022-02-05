use std::ffi::{c_void, CStr, CString};
use std::ptr;

use libjuice_sys as sys;

use crate::agent::Agent;

#[derive(Clone)]
pub(crate) struct Config<'a> {
    pub(crate) stun_server_host: CString,
    pub(crate) stun_server_port: u16,
    pub(crate) parent: &'a crate::agent::Agent,
    pub(crate) port_range: Option<(u16, u16)>,
}

impl Config<'_> {
    pub(crate) fn as_raw(&self) -> sys::juice_config {
        let port_range = &self.port_range.unwrap_or((0, 0));
        sys::juice_config {
            stun_server_host: self.stun_server_host.as_ptr(),
            stun_server_port: self.stun_server_port,
            turn_servers: ptr::null_mut(), // TODO
            turn_servers_count: 0,         // TODO
            bind_address: ptr::null(),     // TODO
            local_port_range_begin: port_range.0,
            local_port_range_end: port_range.1,
            cb_state_changed: Some(on_state_changed),
            cb_candidate: Some(on_candidate),
            cb_gathering_done: Some(on_gathering_done),
            cb_recv: Some(on_recv),
            user_ptr: self.parent as *const _ as *mut c_void,
        }
    }
}

unsafe extern "C" fn on_state_changed(
    _: *mut sys::juice_agent_t,
    state: sys::juice_state_t,
    user_ptr: *mut std::os::raw::c_void,
) {
    let agent = &mut *(user_ptr as *mut Agent);

    if let Err(e) = state.try_into().map(|s| agent.on_state_changed(s)) {
        log::error!("failed to map state {:?}", e)
    }
}

unsafe extern "C" fn on_candidate(
    _: *mut sys::juice_agent_t,
    sdp: *const std::os::raw::c_char,
    user_ptr: *mut std::os::raw::c_void,
) {
    let agent = &mut *(user_ptr as *mut Agent);
    let candidate = {
        let s = CStr::from_ptr(sdp);
        String::from_utf8_lossy(s.to_bytes())
    };
    agent.on_candidate(candidate.to_string())
}

unsafe extern "C" fn on_gathering_done(_: *mut sys::juice_agent, user_ptr: *mut std::ffi::c_void) {
    let agent = &mut *(user_ptr as *mut Agent);
    agent.on_gathering_done()
}

unsafe extern "C" fn on_recv(
    _: *mut libjuice_sys::juice_agent,
    _data: *const i8,
    _len: u64,
    user_ptr: *mut std::ffi::c_void,
) {
    let agent = &mut *(user_ptr as *mut Agent);
    agent.on_recv()
}
