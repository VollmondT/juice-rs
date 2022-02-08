use std::ffi::{CString, NulError};
use std::marker::{PhantomData, PhantomPinned};
use std::ptr;

use libjuice_sys as sys;

use crate::Error;

pub struct Credentials {
    username: CString,
    password: CString,
    quota: i32,
}

impl Credentials {
    pub fn new<T: Into<Vec<u8>>>(username: T, password: T, quota: i32) -> Result<Self, NulError> {
        Ok(Self {
            username: CString::new(username)?,
            password: CString::new(password)?,
            quota,
        })
    }
}

/// TURN server builder.
#[derive(Default)]
pub struct Builder {
    credentials: Vec<Credentials>,
    bind_address: Option<CString>,
    external_address: Option<CString>,
    port: u16,
    max_allocations: i32,
    max_peers: i32,
    relay_port_range: Option<(u16, u16)>,
    realm: CString,
}

/// TURN server.
pub struct Server {
    server: *mut sys::juice_server_t,
    _marker: PhantomData<(sys::juice_server, PhantomPinned)>,
}

impl Builder {
    pub fn build(self) -> Result<Server, Error> {
        let mut credentials = self
            .credentials
            .iter()
            .map(|cred| sys::juice_server_credentials {
                username: cred.username.as_ptr(),
                password: cred.password.as_ptr(),
                allocations_quota: cred.quota,
            })
            .collect::<Vec<_>>();

        let credentials = if credentials.is_empty() {
            ptr::null_mut()
        } else {
            credentials.as_mut_ptr()
        };

        let port_range = self.relay_port_range.unwrap_or_default();

        let bind_address = self
            .bind_address
            .as_ref()
            .map(|v| v.as_ptr())
            .unwrap_or(ptr::null());

        let external_address = self
            .external_address
            .as_ref()
            .map(|v| v.as_ptr())
            .unwrap_or(ptr::null());

        let config = sys::juice_server_config {
            credentials,
            credentials_count: self.credentials.len() as _,
            bind_address,
            external_address,
            max_allocations: self.max_allocations,
            max_peers: self.max_peers,
            port: self.port,
            relay_port_range_begin: port_range.0,
            relay_port_range_end: port_range.1,
            realm: self.realm.as_ptr(),
        };

        // finally create
        let ptr = unsafe { sys::juice_server_create(&config as _) };

        if ptr.is_null() {
            Err(Error::Failed)
        } else {
            Ok(Server {
                server: ptr,
                _marker: Default::default(),
            })
        }
    }
}

unsafe impl Send for Server {}

unsafe impl Sync for Server {}

impl Server {
    /// Create server builder
    pub fn builder() -> Builder {
        Default::default()
    }

    /// Get listen port
    pub fn get_port(&self) -> u16 {
        unsafe { sys::juice_server_get_port(self.server) }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        unsafe { sys::juice_server_destroy(self.server) }
    }
}
