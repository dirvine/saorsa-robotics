use crate::types::ViolationSeverity;
use crate::types::{SafetyEvent, SafetyEventType};
use crate::WatchdogStatus;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

/// Base trait for all watchdogs
pub trait Watchdog: Send + Sync {
    fn name(&self) -> &str;
    fn check(&mut self) -> Result<WatchdogStatus, Box<dyn std::error::Error>>;
    fn reset(&mut self);
    fn timeout_duration(&self) -> Duration;
}

/// Camera watchdog - monitors camera frame rate and health
pub struct CameraWatchdog {
    name: String,
    min_fps: f32,
    timeout: Duration,
    last_frame_time: Option<Instant>,
    consecutive_failures: u32,
    frame_count: u32,
    fps_window_start: Instant,
}

impl CameraWatchdog {
    pub fn new(min_fps: f32, timeout: Duration) -> Self {
        Self {
            name: "camera_watchdog".to_string(),
            min_fps,
            timeout,
            last_frame_time: None,
            consecutive_failures: 0,
            frame_count: 0,
            fps_window_start: Instant::now(),
        }
    }

    pub fn record_frame(&mut self) {
        self.last_frame_time = Some(Instant::now());
        self.frame_count += 1;
    }
}

impl Watchdog for CameraWatchdog {
    fn name(&self) -> &str {
        &self.name
    }

    fn check(&mut self) -> Result<WatchdogStatus, Box<dyn std::error::Error>> {
        let now = Instant::now();

        // Check if we've received frames recently
        let healthy = if let Some(last_frame) = self.last_frame_time {
            now.duration_since(last_frame) < self.timeout
        } else {
            false
        };

        // Calculate FPS over the last second
        let elapsed = now.duration_since(self.fps_window_start).as_secs_f32();
        let current_fps = if elapsed > 0.0 {
            self.frame_count as f32 / elapsed
        } else {
            0.0
        };

        // Reset counters every second
        if elapsed >= 1.0 {
            self.frame_count = 0;
            self.fps_window_start = now;
        }

        let mut last_error = None;
        if !healthy {
            self.consecutive_failures += 1;
            last_error = Some("No camera frames received within timeout".to_string());
        } else if current_fps < self.min_fps {
            self.consecutive_failures += 1;
            last_error = Some(format!(
                "Camera FPS {:.1} below minimum {:.1}",
                current_fps, self.min_fps
            ));
        } else {
            self.consecutive_failures = 0;
        }

        Ok(WatchdogStatus {
            name: self.name.clone(),
            healthy,
            last_check: SystemTime::now(),
            last_error,
            timeout_duration: self.timeout,
            consecutive_failures: self.consecutive_failures,
        })
    }

    fn reset(&mut self) {
        self.last_frame_time = Some(Instant::now());
        self.consecutive_failures = 0;
        self.frame_count = 0;
        self.fps_window_start = Instant::now();
    }

    fn timeout_duration(&self) -> Duration {
        self.timeout
    }
}

/// CAN bus watchdog - monitors CAN communication health
pub struct CanWatchdog {
    name: String,
    timeout: Duration,
    last_message_time: Option<Instant>,
    consecutive_failures: u32,
    message_count: u32,
}

impl CanWatchdog {
    pub fn new(timeout: Duration) -> Self {
        Self {
            name: "can_watchdog".to_string(),
            timeout,
            last_message_time: None,
            consecutive_failures: 0,
            message_count: 0,
        }
    }

    pub fn record_message(&mut self) {
        self.last_message_time = Some(Instant::now());
        self.message_count += 1;
    }
}

impl Watchdog for CanWatchdog {
    fn name(&self) -> &str {
        &self.name
    }

    fn check(&mut self) -> Result<WatchdogStatus, Box<dyn std::error::Error>> {
        let now = Instant::now();

        let healthy = if let Some(last_message) = self.last_message_time {
            now.duration_since(last_message) < self.timeout
        } else {
            false
        };

        let mut last_error = None;
        if !healthy {
            self.consecutive_failures += 1;
            last_error = Some("No CAN messages received within timeout".to_string());
        } else {
            self.consecutive_failures = 0;
        }

        Ok(WatchdogStatus {
            name: self.name.clone(),
            healthy,
            last_check: SystemTime::now(),
            last_error,
            timeout_duration: self.timeout,
            consecutive_failures: self.consecutive_failures,
        })
    }

    fn reset(&mut self) {
        self.last_message_time = Some(Instant::now());
        self.consecutive_failures = 0;
        self.message_count = 0;
    }

    fn timeout_duration(&self) -> Duration {
        self.timeout
    }
}

/// E-stop watchdog - monitors emergency stop button
pub struct EStopWatchdog {
    name: String,
    timeout: Duration,
    e_stop_pressed: Arc<Mutex<bool>>,
    last_check_time: Instant,
    consecutive_failures: u32,
}

impl EStopWatchdog {
    pub fn new(e_stop_pressed: Arc<Mutex<bool>>) -> Self {
        Self {
            name: "estop_watchdog".to_string(),
            timeout: Duration::from_millis(100), // Very responsive
            e_stop_pressed,
            last_check_time: Instant::now(),
            consecutive_failures: 0,
        }
    }

    #[cfg(feature = "gpio")]
    pub fn new_with_gpio(
        e_stop_pressed: Arc<Mutex<bool>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // GPIO-specific initialization would go here
        Ok(Self::new(e_stop_pressed))
    }

    pub fn set_e_stop_pressed(&self, pressed: bool) {
        if let Ok(mut e_stop) = self.e_stop_pressed.lock() {
            *e_stop = pressed;
        }
    }
}

impl Watchdog for EStopWatchdog {
    fn name(&self) -> &str {
        &self.name
    }

    fn check(&mut self) -> Result<WatchdogStatus, Box<dyn std::error::Error>> {
        let now = Instant::now();

        let e_stop_pressed = *self.e_stop_pressed.lock().map_err(|_| "Mutex poisoned")?;

        let healthy = !e_stop_pressed;

        let mut last_error = None;
        if e_stop_pressed {
            self.consecutive_failures += 1;
            last_error = Some("Emergency stop button pressed".to_string());
        } else {
            self.consecutive_failures = 0;
        }

        self.last_check_time = now;

        Ok(WatchdogStatus {
            name: self.name.clone(),
            healthy,
            last_check: SystemTime::now(),
            last_error,
            timeout_duration: self.timeout,
            consecutive_failures: self.consecutive_failures,
        })
    }

    fn reset(&mut self) {
        if let Ok(mut e_stop) = self.e_stop_pressed.lock() {
            *e_stop = false;
        }
        self.consecutive_failures = 0;
    }

    fn timeout_duration(&self) -> Duration {
        self.timeout
    }
}

/// Watchdog manager - coordinates multiple watchdogs
pub struct WatchdogManager {
    watchdogs: Vec<Box<dyn Watchdog>>,
    event_callback: Option<Box<dyn Fn(SafetyEvent) + Send + Sync>>,
}

impl WatchdogManager {
    pub fn new() -> Self {
        Self {
            watchdogs: Vec::new(),
            event_callback: None,
        }
    }

    pub fn add_watchdog(
        &mut self,
        watchdog: Box<dyn Watchdog>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check for duplicate names
        for existing in &self.watchdogs {
            if existing.name() == watchdog.name() {
                return Err(format!("Watchdog '{}' already exists", watchdog.name()).into());
            }
        }

        self.watchdogs.push(watchdog);
        Ok(())
    }

    pub fn remove_watchdog(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let initial_len = self.watchdogs.len();
        self.watchdogs.retain(|w| w.name() != name);

        if self.watchdogs.len() == initial_len {
            return Err(format!("Watchdog '{}' not found", name).into());
        }

        Ok(())
    }

    pub fn check_all(&mut self) -> Result<Vec<WatchdogStatus>, Box<dyn std::error::Error>> {
        let mut statuses = Vec::new();
        let mut events = Vec::new();

        for watchdog in &mut self.watchdogs {
            let status = watchdog.check()?;

            // Check for state changes
            if !status.healthy && status.consecutive_failures == 1 {
                // First failure - generate event
                events.push(SafetyEvent {
                    timestamp: SystemTime::now(),
                    event_type: SafetyEventType::WatchdogFailure,
                    message: format!(
                        "Watchdog '{}' failed: {}",
                        status.name,
                        status.last_error.as_deref().unwrap_or("Unknown error")
                    ),
                    severity: ViolationSeverity::Critical,
                    context: HashMap::new(),
                });
            }

            statuses.push(status);
        }

        // Send events to callback
        if let Some(callback) = &self.event_callback {
            for event in events {
                callback(event);
            }
        }

        Ok(statuses)
    }

    pub fn reset_all(&mut self) {
        for watchdog in &mut self.watchdogs {
            watchdog.reset();
        }
    }

    pub fn get_watchdog(&mut self, name: &str) -> Option<&mut Box<dyn Watchdog>> {
        self.watchdogs.iter_mut().find(|w| w.name() == name)
    }

    pub fn set_event_callback<F>(&mut self, callback: F)
    where
        F: Fn(SafetyEvent) + Send + Sync + 'static,
    {
        self.event_callback = Some(Box::new(callback));
    }

    pub fn get_watchdogs(&self) -> Vec<String> {
        self.watchdogs
            .iter()
            .map(|w| w.name().to_string())
            .collect()
    }
}

/// Helper functions for specific watchdog types
pub mod helpers {
    use super::*;

    pub fn record_camera_frame(manager: &mut WatchdogManager) {
        if let Some(watchdog) = manager.get_watchdog("camera_watchdog") {
            if let Some(camera_watchdog) = watchdog.as_any().downcast_mut::<CameraWatchdog>() {
                camera_watchdog.record_frame();
            }
        }
    }

    pub fn record_can_message(manager: &mut WatchdogManager) {
        if let Some(watchdog) = manager.get_watchdog("can_watchdog") {
            if let Some(can_watchdog) = watchdog.as_any().downcast_mut::<CanWatchdog>() {
                can_watchdog.record_message();
            }
        }
    }

    pub fn set_e_stop_pressed(manager: &mut WatchdogManager, pressed: bool) {
        if let Some(watchdog) = manager.get_watchdog("estop_watchdog") {
            if let Some(estop_watchdog) = watchdog.as_any().downcast_mut::<EStopWatchdog>() {
                estop_watchdog.set_e_stop_pressed(pressed);
            }
        }
    }
}

// Add downcast support for watchdogs
impl dyn Watchdog {
    pub fn as_any(&mut self) -> &mut dyn std::any::Any {
        // This is a simplified version - in practice you'd need to implement
        // proper downcasting for each watchdog type
        panic!("Downcasting not implemented for this watchdog type")
    }
}

// Implement as_any for each watchdog type
macro_rules! impl_as_any {
    ($type:ty) => {
        impl $type {
            pub fn as_any(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }
    };
}

impl_as_any!(CameraWatchdog);
impl_as_any!(CanWatchdog);
impl_as_any!(EStopWatchdog);
