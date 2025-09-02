use crate::types::DeviceDescriptor;
use anyhow::Context;
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone)]
pub struct DeviceRegistry {
    pub devices: HashMap<String, DeviceDescriptor>,
}

impl DeviceRegistry {
    pub fn insert(&mut self, desc: DeviceDescriptor) {
        self.devices.insert(desc.id.clone(), desc);
    }
}

pub fn load_descriptor_file(path: impl AsRef<Path>) -> anyhow::Result<DeviceDescriptor> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading descriptor: {}", path.display()))?;
    let val: Value =
        serde_yaml::from_str(&raw).with_context(|| format!("parsing yaml: {}", path.display()))?;
    let desc: DeviceDescriptor = serde_yaml::from_value(val)
        .with_context(|| format!("decoding descriptor: {}", path.display()))?;
    Ok(desc)
}

pub fn load_descriptors_dir(dir: impl AsRef<Path>) -> anyhow::Result<DeviceRegistry> {
    let mut reg = DeviceRegistry::default();
    let mut entries: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(dir.as_ref())? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "yml" || ext == "yaml" {
                entries.push(path);
            }
        }
    }
    entries.sort();
    for p in entries {
        let desc = load_descriptor_file(&p)?;
        reg.insert(desc);
    }
    Ok(reg)
}
