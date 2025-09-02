use thiserror::Error;

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("camera not found: {0}")]
    NotFound(String),
    #[error("unsupported operation: {0}")]
    Unsupported(&'static str),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("backend error: {0}")]
    Backend(String),
}
