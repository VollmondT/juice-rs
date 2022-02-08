use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum AgentError {
    InvalidArgument,
    Failed,
    NotAvailable,
}

impl std::error::Error for AgentError {}

impl Display for AgentError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::InvalidArgument => write!(f, "invalid argument"),
            AgentError::Failed => write!(f, "failure"),
            AgentError::NotAvailable => write!(f, "not available"),
        }
    }
}
