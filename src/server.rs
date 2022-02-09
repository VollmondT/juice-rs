//! Embedded TURN server.

use std::cmp::max;
use std::ffi::CString;
use std::marker::{PhantomData, PhantomPinned};
use std::net::{IpAddr, SocketAddr};
use std::ptr;

use libjuice_sys as sys;

use crate::log::ensure_logging;
use crate::{Error, Result};

pub struct Credentials {
    username: CString,
    password: CString,
    quota: Option<i32>,
}

impl Credentials {
    pub fn new<T: Into<Vec<u8>>>(username: T, password: T, quota: Option<i32>) -> Result<Self> {
        Ok(Self {
            username: CString::new(username).map_err(|_| Error::InvalidArgument)?,
            password: CString::new(password).map_err(|_| Error::InvalidArgument)?,
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
    realm: Option<CString>,
}

/// TURN server.
pub struct Server {
    server: *mut sys::juice_server_t,
    _marker: PhantomData<(sys::juice_server, PhantomPinned)>,
}

impl Builder {
    /// Build [`Server`].
    pub fn build(self) -> Result<Server> {
        ensure_logging();

        let mut credentials = self
            .credentials
            .iter()
            .map(|cred| sys::juice_server_credentials {
                username: cred.username.as_ptr(),
                password: cred.password.as_ptr(),
                allocations_quota: cred.quota.unwrap_or_default(),
            })
            .collect::<Vec<_>>();

        let credentials = if credentials.is_empty() {
            return Err(Error::InvalidArgument);
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

        let realm = self
            .realm
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
            realm,
        };

        // finally try to build
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

    /// Set several credentials at once.
    ///
    /// This function will overwrite credentials list entirely. Alternatively, you can
    /// sequentially call [`Builder::add_credentials`]
    pub fn with_credentials<I: Iterator<Item = Credentials>>(
        mut self,
        credentials_list: I,
    ) -> Self {
        self.credentials = credentials_list.collect();
        self
    }

    /// Append credentials to the list.
    pub fn add_credentials(mut self, cred: Credentials) -> Self {
        self.credentials.push(cred);
        self
    }

    /// Bind to specific interface and port.
    pub fn bind_address(mut self, addr: &SocketAddr) -> Self {
        self.bind_address = Some(CString::new(addr.ip().to_string()).unwrap());
        self.port = addr.port();
        self
    }

    pub fn with_external_address(mut self, addr: &IpAddr) -> Self {
        self.external_address = Some(CString::new(addr.to_string()).unwrap());
        self
    }

    /// Set relayed port range.
    pub fn with_port_range(mut self, begin: u16, end: u16) -> Self {
        self.relay_port_range = Some((begin, end));
        self
    }

    /// Set realm.
    pub fn with_realm<T: Into<Vec<u8>>>(mut self, realm: T) -> Result<Self> {
        self.realm = Some(CString::new(realm).map_err(|_| Error::InvalidArgument)?);
        Ok(self)
    }

    pub fn with_allocations_limit(mut self, limit: u32) -> Self {
        self.max_allocations = max(limit, i32::MAX as u32) as i32;
        self
    }

    pub fn with_peers_limit(mut self, limit: u32) -> Self {
        self.max_peers = max(limit, i32::MAX as u32) as i32;
        self
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build() {
        crate::test_util::logger_init();
        let creds = Credentials::new("a", "b", None).unwrap();

        let _ = Server::builder()
            .add_credentials(creds)
            .build()
            .ok()
            .unwrap();
    }
}
