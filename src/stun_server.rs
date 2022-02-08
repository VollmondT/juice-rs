use std::ffi::CString;

/// Stun server (host:port)
pub(crate) struct StunServer(pub(crate) CString, pub(crate) u16);

impl Default for StunServer {
    fn default() -> Self {
        Self(CString::new("stun.l.google.com").unwrap(), 19302)
    }
}

impl StunServer {
    /// Construct from `std::String` and port value
    pub(crate) fn new(host: String, port: u16) -> Result<Self, std::ffi::NulError> {
        Ok(Self(CString::new(host)?, port))
    }
}
