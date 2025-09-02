use crate::{BusInfo, CanBus, CanFilter, CanFrame, Result, Timestamp, TransportError};
use time::OffsetDateTime;

/// A simple in-process mock bus. Each bus instance is independent.
pub struct MockBus {
    name: String,
}

impl CanBus for MockBus {
    fn open(name: &str) -> Result<Self> {
        Ok(Self {
            name: name.to_string(),
        })
    }

    fn list() -> Result<Vec<BusInfo>> {
        Ok(vec![BusInfo {
            name: "mock0".to_string(),
            driver: "mock".to_string(),
        }])
    }

    fn set_filters(&mut self, _filters: &[CanFilter]) -> Result<()> {
        let _ = _filters;
        // Mock supports no filters
        Err(TransportError::Unsupported(
            "mock backend has no hardware filters",
        ))
    }

    fn recv(&mut self, _timeout_ms: Option<u64>) -> Result<CanFrame> {
        // Produce an idle heartbeat every time we're called so flows are testable
        let id = crate::CanId::standard(0x700).ok_or(TransportError::InvalidFrame("id"))?;
        let mut frame = CanFrame::new(id, &[0x00, 0x00, 0x00, 0x00])
            .ok_or(TransportError::InvalidFrame("len"))?;
        frame.timestamp = Some(Timestamp(OffsetDateTime::now_utc()));
        Ok(frame)
    }

    fn send(&mut self, frame: &CanFrame) -> Result<()> {
        // Accept any frame; pretend it was sent
        let _ = (&self.name, frame);
        Ok(())
    }
}
