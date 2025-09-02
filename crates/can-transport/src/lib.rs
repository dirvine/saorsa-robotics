//! can-transport: cross-platform CAN bus abstractions
//!
//! This crate provides traits and types for interacting with Controller Area Network (CAN)
//! interfaces, with feature-gated backends. The default build enables a `mock` backend so
//! that binaries can compile on any host without native drivers.

mod types;
pub use types::{BusInfo, CanFilter, CanFrame, CanId, Timestamp};

mod error;
pub use error::{Result, TransportError};

mod traits;
pub use traits::CanBus;

#[cfg(feature = "mock")]
mod mock;

#[cfg(feature = "mock")]
pub use mock::MockBus;

#[cfg(feature = "slcan")]
mod slcan;

#[cfg(feature = "slcan")]
pub use slcan::{SlcanBitrate, SlcanBus};
