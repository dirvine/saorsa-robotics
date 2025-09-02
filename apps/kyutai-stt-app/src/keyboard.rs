use anyhow::Result;
use rdev::{listen, EventType, Key};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct KeyboardListener {
    hotkey: String,
    callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>,
}

impl KeyboardListener {
    pub fn new(hotkey: String) -> Result<Self> {
        Ok(Self {
            hotkey,
            callback: Arc::new(Mutex::new(None)),
        })
    }

    pub fn start<F>(&self, callback: F) -> Result<()>
    where
        F: Fn() + Send + Sync + 'static,
    {
        if let Ok(mut slot) = self.callback.lock() {
            *slot = Some(Box::new(callback));
        } else {
            // If the lock is poisoned, return a non-fatal error instead of panicking
            return Err(anyhow::anyhow!("keyboard listener lock poisoned"));
        }

        let callback = self.callback.clone();
        let hotkey = self.hotkey.clone();

        thread::spawn(move || {
            let _ = listen(move |event| {
                if let EventType::KeyPress(key) = event.event_type {
                    if Self::matches_hotkey(&hotkey, key) {
                        if let Ok(cb_guard) = callback.lock() {
                            if let Some(ref cb) = *cb_guard {
                                cb();
                            }
                        }
                    }
                }
            });
        });
        Ok(())
    }

    fn matches_hotkey(hotkey: &str, key: Key) -> bool {
        match hotkey.to_lowercase().as_str() {
            "f12" => matches!(key, Key::F12),
            "f11" => matches!(key, Key::F11),
            "f10" => matches!(key, Key::F10),
            "f9" => matches!(key, Key::F9),
            "f8" => matches!(key, Key::F8),
            "f7" => matches!(key, Key::F7),
            "f6" => matches!(key, Key::F6),
            "f5" => matches!(key, Key::F5),
            "f4" => matches!(key, Key::F4),
            "f3" => matches!(key, Key::F3),
            "f2" => matches!(key, Key::F2),
            "f1" => matches!(key, Key::F1),
            "space" => matches!(key, Key::Space),
            "enter" => matches!(key, Key::Return),
            "escape" => matches!(key, Key::Escape),
            _ => false,
        }
    }
}
