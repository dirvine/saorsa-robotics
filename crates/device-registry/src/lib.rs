//! device-registry: YAML-driven registry of CAN devices and joints

mod types;
pub use types::*;

mod loader;
pub use loader::{load_descriptor_file, load_descriptors_dir, DeviceRegistry};

mod drivers;
pub use drivers::{DriverKind, DriverSpec};

mod metrics;
pub use metrics::{DeviceMetrics, MetricsHub};

mod encode;
pub use encode::{build_frames_for_joint, JointCommand};

mod decode;
pub use decode::{decode_by_id, decode_fmt, TelemetryRecord, TelemetryValue};
