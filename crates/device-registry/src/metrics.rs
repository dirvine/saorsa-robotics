use prometheus::{Encoder, IntCounter, IntGauge, Registry, TextEncoder};

#[derive(Clone)]
pub struct DeviceMetrics {
    pub tx_frames: IntCounter,
    pub rx_frames: IntCounter,
    pub devices_loaded: IntGauge,
}

#[derive(Clone)]
pub struct MetricsHub {
    pub registry: Registry,
    pub dev: DeviceMetrics,
}

impl MetricsHub {
    pub fn new() -> Result<Self, String> {
        let registry = Registry::new();
        let tx_frames = IntCounter::new("sr_can_tx_frames", "Total CAN frames sent")
            .map_err(|e| format!("metrics init error: {e}"))?;
        let rx_frames = IntCounter::new("sr_can_rx_frames", "Total CAN frames received")
            .map_err(|e| format!("metrics init error: {e}"))?;
        let devices_loaded =
            IntGauge::new("sr_devices_loaded", "Number of device descriptors loaded")
                .map_err(|e| format!("metrics init error: {e}"))?;
        let dev = DeviceMetrics {
            tx_frames,
            rx_frames,
            devices_loaded,
        };
        let _ = registry.register(Box::new(dev.tx_frames.clone()));
        let _ = registry.register(Box::new(dev.rx_frames.clone()));
        let _ = registry.register(Box::new(dev.devices_loaded.clone()));
        Ok(Self { registry, dev })
    }

    pub fn encode_text(&self) -> String {
        let mut buf = Vec::new();
        let encoder = TextEncoder::new();
        if let Err(e) = encoder.encode(&self.registry.gather(), &mut buf) {
            return format!("error encoding metrics: {e}");
        }
        String::from_utf8(buf).unwrap_or_default()
    }
}
