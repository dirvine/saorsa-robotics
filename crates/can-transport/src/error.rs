use thiserror::Error;

pub type Result<T, E = TransportError> = core::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("interface not found: {0}")]
    InterfaceNotFound(String),
    #[error("operation not supported on this backend: {0}")]
    Unsupported(&'static str),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("timeout")]
    Timeout,
    #[error("invalid frame: {0}")]
    InvalidFrame(&'static str),
}
