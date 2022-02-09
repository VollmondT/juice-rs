extern crate core;

pub use agent::{handler::Handler, Agent, Builder, State};
pub use error::{Error, Result};
pub use server::{Builder as ServerBuilder, Credentials as ServerCredentials, Server};

mod agent;
mod error;
mod log;
mod server;

#[cfg(test)]
mod test_util;
