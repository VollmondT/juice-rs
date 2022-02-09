use std::fmt::{Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Error {
    InvalidArgument,
    Failed,
    NotAvailable,
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidArgument => write!(f, "invalid argument"),
            Error::Failed => write!(f, "failure"),
            Error::NotAvailable => write!(f, "not available"),
        }
    }
}
