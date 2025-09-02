use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriverKind {
    Odrive,
    TmotorAk,
    Cia402,
    Cyphal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverSpec {
    pub kind: DriverKind,
}

// Placeholder for future driver implementations mapping to CAN frames.
