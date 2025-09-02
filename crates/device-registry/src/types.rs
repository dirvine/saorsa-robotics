use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDescriptor {
    pub id: String,
    pub bus: String,
    pub protocol: String,
    pub node_id: Option<u32>,
    #[serde(default)]
    pub joints: Vec<Joint>,
    #[serde(default)]
    pub telemetry: Vec<TelemetryMap>,
    pub heartbeat: Option<Heartbeat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Joint {
    pub name: String,
    #[serde(default)]
    pub limits: JointLimits,
    #[serde(default)]
    pub r#map: JointMap,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JointLimits {
    #[serde(default)]
    pub pos_deg: Option<(f64, f64)>,
    #[serde(default)]
    pub vel_dps: Option<f64>,
    #[serde(default)]
    pub torque_nm: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JointMap {
    pub mode: Option<String>,
    #[serde(default)]
    pub scale: Scale,
    #[serde(default)]
    pub frames: Vec<CanFrameFmt>,
    #[serde(default)]
    pub pd: Option<PdGains>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Scale {
    #[serde(default)]
    pub k_t: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PdGains {
    #[serde(default)]
    pub kp: Option<f32>,
    #[serde(default)]
    pub kd: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanFrameFmt {
    pub id: String,
    pub fmt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryMap {
    pub id: String,
    pub fmt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub id: String,
    pub period_ms: u64,
}
