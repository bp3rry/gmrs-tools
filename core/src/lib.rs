//! gmrs-core: everything that knows about radios, with no external dependencies.
//!
//! Front-ends (CLI, desktop, mobile) call into this crate; they never speak a
//! radio protocol themselves. See `docs/ARCHITECTURE.md`.

pub mod uv5r;

use std::fmt;

/// Errors that can happen anywhere in the core.
#[derive(Debug)]
pub enum Error {
    /// The radio didn't answer in time.
    Timeout(String),
    /// The radio answered, but not with what the protocol expects.
    Protocol(String),
    /// A file or image is malformed.
    Format(String),
    /// Low-level I/O failure (cable unplugged, permission denied, ...).
    Io(std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Timeout(m) => write!(f, "timeout: {m}"),
            Error::Protocol(m) => write!(f, "protocol error: {m}"),
            Error::Format(m) => write!(f, "format error: {m}"),
            Error::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => {
                Error::Timeout("no data from radio".into())
            }
            _ => Error::Io(e),
        }
    }
}
