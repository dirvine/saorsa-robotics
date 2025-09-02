use crate::{BusInfo, CanFilter, CanFrame, Result, TransportError};

/// A minimal blocking CAN bus interface.
pub trait CanBus {
    /// Open a CAN interface by name (e.g., "can0", "slcan0").
    fn open(name: &str) -> Result<Self>
    where
        Self: Sized;

    /// Attempt to list available interfaces for this backend.
    fn list() -> Result<Vec<BusInfo>>;

    /// Set acceptance filters if supported.
    fn set_filters(&mut self, _filters: &[CanFilter]) -> Result<()> {
        let _ = _filters;
        Err(TransportError::Unsupported("filters not supported"))
    }

    /// Receive one frame (blocking with optional timeout in milliseconds).
    fn recv(&mut self, _timeout_ms: Option<u64>) -> Result<CanFrame>;

    /// Send one frame.
    fn send(&mut self, frame: &CanFrame) -> Result<()>;
}
