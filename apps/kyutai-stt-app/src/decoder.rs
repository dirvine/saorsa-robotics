use crate::model::ModelConfig;
use anyhow::{anyhow, Result};
use candle_core as candle;
use safetensors::SafeTensors;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub struct SttDecoder {
    // Placeholder: we will wire Candle modules here
    dim: usize,
    layers: usize,
    heads: usize,
    pub(crate) tensors: usize,
    pub(crate) device: candle::Device,
    pub(crate) weights_bytes: Vec<u8>,
    pub(crate) names: Vec<String>,
    graph: DecoderGraph,
}

impl SttDecoder {
    pub fn load(weights: &Path, cfg: &ModelConfig) -> Result<Self> {
        let data = fs::read(weights)?;
        let st = SafeTensors::deserialize(&data).map_err(|e| anyhow!("safetensors: {e}"))?;
        let tensors = st.len();
        let mut names: Vec<String> = st.names().iter().map(|s| s.to_string()).collect();
        names.sort();
        let device = select_device();
        println!(
            "[decoder] loaded {} tensors from {} (dim={}, layers={}, heads={}) on {}",
            tensors,
            weights.display(),
            cfg.dim,
            cfg.num_layers,
            cfg.num_heads,
            device_name(&device)
        );
        let mut dec = Self {
            dim: cfg.dim,
            layers: cfg.num_layers,
            heads: cfg.num_heads,
            tensors,
            device,
            weights_bytes: data,
            names,
            graph: DecoderGraph::new(cfg.dim, cfg.num_layers, cfg.num_heads),
        };
        dec.graph.load_named_parameters(&dec.weights_bytes)?; // non-strict: only catalogs names for now
        Ok(dec)
    }

    pub fn dump_tensors(&self) -> Result<()> {
        let st = SafeTensors::deserialize(&self.weights_bytes)
            .map_err(|e| anyhow!("safetensors: {e}"))?;
        println!("[decoder] {} tensors:", st.len());
        let mut names: Vec<String> = st.names().iter().map(|s| s.to_string()).collect();
        names.sort();
        for n in names {
            if let Some(t) = st.tensor(&n).ok() {
                println!("decoder: {n}\t{:?}", t.shape());
            } else {
                println!("decoder: {n}");
            }
        }
        Ok(())
    }

    pub fn print_groups(&self) {
        let mut groups: BTreeMap<String, usize> = BTreeMap::new();
        for n in &self.names {
            let key = n.split(['.', '/']).next().unwrap_or("").to_string();
            *groups.entry(key).or_insert(0) += 1;
        }
        println!("[decoder] tensor groups (by prefix):");
        for (k, v) in groups {
            println!("  {k}: {v}");
        }
    }

    pub fn print_summary(&self) {
        let total = self.names.len();
        let mapped = self.graph.consumed.len();
        let unmapped = total.saturating_sub(mapped);
        println!(
            "[decoder] mapping summary: total={}, mapped={}, unmapped={}",
            total, mapped, unmapped
        );
        if unmapped > 0 {
            let mut shown = 0usize;
            print!("[decoder] example unmapped: ");
            for n in &self.names {
                if !self.graph.consumed.contains(n) {
                    shown += 1;
                    if shown > 1 {
                        print!(", ");
                    }
                    print!("{}", n);
                    if shown >= 10 {
                        break;
                    }
                }
            }
            if shown > 0 {
                println!("");
            }
        }
    }

    /// Skeleton decode step that validates shapes and returns a placeholder token sequence.
    pub fn decode_step(&self, _audio_tokens: &[usize]) -> Result<Vec<usize>> {
        // Placeholder: produce a few dummy text tokens to validate integration.
        Ok(vec![0; 8])
    }
}

struct DecoderGraph {
    dim: usize,
    layers: usize,
    heads: usize,
    names: Vec<String>,
    consumed: HashSet<String>,
}

impl DecoderGraph {
    fn new(dim: usize, layers: usize, heads: usize) -> Self {
        Self {
            dim,
            layers,
            heads,
            names: Vec::new(),
            consumed: HashSet::new(),
        }
    }

    fn load_named_parameters(&mut self, bytes: &[u8]) -> Result<()> {
        let st = SafeTensors::deserialize(bytes).map_err(|e| anyhow!("safetensors: {e}"))?;
        self.names = st.names().iter().map(|s| s.to_string()).collect();
        self.names.sort();
        // Mapping not yet implemented; keep non-fatal.
        println!(
            "[decoder] loader: {} tensors discovered; mapping pending",
            self.names.len()
        );
        Ok(())
    }
}

fn select_device() -> candle::Device {
    #[cfg(all(target_os = "macos", feature = "metal"))]
    {
        if let Ok(d) = candle::Device::new_metal(0) {
            return d;
        }
    }
    candle::Device::Cpu
}

fn device_name(d: &candle::Device) -> &'static str {
    match d {
        candle::Device::Cpu => "cpu",
        #[cfg(feature = "cuda")]
        candle::Device::Cuda(_) => "cuda",
        #[cfg(feature = "metal")]
        candle::Device::Metal(_) => "metal",
        _ => "device",
    }
}
