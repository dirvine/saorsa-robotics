use anyhow::{anyhow, Result};
use candle_core as candle;
use candle_core::Tensor;
use safetensors::SafeTensors;
use std::fs;
use std::path::Path;
// use candle_nn::conv2d; // reserved for future learned conv front-end
use crate::model::ModelConfig;
use safetensors::tensor::Dtype as StDtype;
use std::collections::HashSet;
// use candle_nn::ops as nnops;

pub struct MimiEncoder {
    // Placeholder for weights/graph
    pub(crate) tensors: usize,
    pub(crate) device: candle::Device,
    pub(crate) graph: MimiGraph,
    pub(crate) weights_bytes: Vec<u8>,
}

impl MimiEncoder {
    pub fn load(path: &Path) -> Result<Self> {
        let data = fs::read(path)?;
        let st = SafeTensors::deserialize(&data).map_err(|e| anyhow!("safetensors: {e}"))?;
        let tensors = st.len();
        let device = select_device();
        println!(
            "[mimi] loaded {} tensors from {} on {}",
            tensors,
            path.display(),
            device_name(&device)
        );
        let graph = MimiGraph::new();
        Ok(Self {
            tensors,
            device,
            graph,
            weights_bytes: data,
        })
    }

    // TODO: add forward(frame) -> [usize; 32]
    pub fn encode_frames(
        &self,
        frames: &Vec<&[f32]>,
        tokens_per_frame: usize,
    ) -> Result<Vec<Vec<usize>>> {
        if tokens_per_frame == 0 {
            return Ok(vec![]);
        }
        let mut all = Vec::with_capacity(frames.len());
        for f in frames {
            all.push(self.encode_single_frame_candle(f, tokens_per_frame)?);
        }
        Ok(all)
    }

    fn encode_single_frame_candle(
        &self,
        frame: &[f32],
        tokens_per_frame: usize,
    ) -> Result<Vec<usize>> {
        if frame.is_empty() {
            return Ok(vec![0; tokens_per_frame]);
        }
        let n = frame.len();
        let t = tokens_per_frame.max(1);
        let chunk = (n + t - 1) / t; // ceil div
        let pad = t * chunk - n;
        let dev = &self.device;
        let x =
            Tensor::from_slice(frame, (n,), dev).map_err(|e| anyhow!("tensor from_slice: {e}"))?;
        let x = if pad > 0 {
            let zeros = Tensor::zeros((pad,), x.dtype(), dev).map_err(|e| anyhow!("zeros: {e}"))?;
            Tensor::cat(&[x, zeros], 0).map_err(|e| anyhow!("cat: {e}"))?
        } else {
            x
        };
        let x_tc = x.reshape((t, chunk)).map_err(|e| anyhow!("reshape: {e}"))?; // [tokens, chunk]
                                                                                // Optional learned conv1 front-end when provided via env mapping
        let x4 = x_tc.unsqueeze(0).map_err(|e| anyhow!("unsqueeze: {e}"))?; // [1, t, chunk]
        let x4 = x4.transpose(1, 2).map_err(|e| anyhow!("transpose: {e}"))?; // [1, chunk, t]
        let x4 = x4.unsqueeze(1).map_err(|e| anyhow!("unsq2: {e}"))?; // [1, 1, chunk, t]
        let mut feat = x4; // [N, C=1, H=chunk, W=t]
                           // Learned conv front-end can be applied here once conv ops are enabled/mapped.
                           // Pool along time axis (last dim) to tokens_per_frame buckets
        let shape_vec = feat.dims().to_vec(); // [1, C, H, T'] with H=chunk
        let tdim = *shape_vec.last().unwrap_or(&1);
        let cdim = if shape_vec.len() >= 2 {
            shape_vec[1]
        } else {
            1
        };
        let hdim = if shape_vec.len() >= 3 {
            shape_vec[2]
        } else {
            1
        };
        let tprim = tdim;
        let pool = (tprim + t - 1) / t; // ceil
        let need_pad = t * pool - tprim;
        feat = if need_pad > 0 {
            let pad = Tensor::zeros((1, cdim, hdim, need_pad), feat.dtype(), feat.device())
                .map_err(|e| anyhow!("pad zeros: {e}"))?;
            Tensor::cat(&[feat, pad], 3).map_err(|e| anyhow!("cat pad: {e}"))?
        } else {
            feat
        };
        let feat = feat
            .reshape((1, cdim, hdim, t, pool))
            .map_err(|e| anyhow!("reshape pool: {e}"))?;
        let feat = feat.mean(4).map_err(|e| anyhow!("mean time: {e}"))?; // [1, C, H, tokens]
        let feat = feat.mean(2).map_err(|e| anyhow!("mean H: {e}"))?; // [1, C, tokens]
        let feat = feat.squeeze(0).map_err(|e| anyhow!("squeeze: {e}"))?; // [C, tokens]
        let feat = feat
            .transpose(0, 1)
            .map_err(|e| anyhow!("transpose ct: {e}"))?; // [tokens, C]
        let mean = feat.mean(1).map_err(|e| anyhow!("mean: {e}"))?; // [tokens]
                                                                    // A touch of energy: stddev over chunks, approximated via second-moment trick
        let x2 = x_tc.sqr().map_err(|e| anyhow!("sqr: {e}"))?;
        let mean2 = x2.mean(1).map_err(|e| anyhow!("mean2: {e}"))?;
        let mean_sqr = mean.sqr().map_err(|e| anyhow!("mean.sqr: {e}"))?;
        let var = mean2.sub(&mean_sqr).map_err(|e| anyhow!("sub: {e}"))?;
        let varc = var
            .clamp(0.0, f32::MAX)
            .map_err(|e| anyhow!("clamp: {e}"))?;
        let std = varc.sqrt().map_err(|e| anyhow!("sqrt: {e}"))?;
        let mean_t = mean.tanh().map_err(|e| anyhow!("tanh mean: {e}"))?;
        let std_t = std.tanh().map_err(|e| anyhow!("tanh std: {e}"))?;
        let scaled = (std_t * 0.1).map_err(|e| anyhow!("mul scalar: {e}"))?;
        let score = mean_t.add(&scaled).map_err(|e| anyhow!("add: {e}"))?;
        let score = score.clamp(-1.0, 1.0).map_err(|e| anyhow!("clamp2: {e}"))?;
        let v = score
            .to_vec1::<f32>()
            .map_err(|e| anyhow!("to_vec1: {e}"))?;
        let mut out = Vec::with_capacity(t);
        for s in v {
            out.push(map_to_uint(s, -1.0, 1.0, 12) as usize);
        }
        Ok(out)
    }

    pub fn configure(&mut self, cfg: Option<&ModelConfig>) -> Result<()> {
        self.graph.build_skeleton(&self.device, cfg)?;
        if std::env::var("KYUTAI_MIMI_STRICT_LOAD").ok().as_deref() == Some("1") {
            // Attempt to load named parameters and fail if mapping is not yet implemented
            self.graph.load_named_parameters(&self.weights_bytes)?;
        }
        // Try to load a first conv layer if the environment specifies a prefix
        let _ = self
            .graph
            .try_load_conv1_from_env(&self.weights_bytes, &self.device);
        let _ = self
            .graph
            .try_load_conv2_from_env(&self.weights_bytes, &self.device);
        Ok(())
    }

    pub fn dump_tensors(&self) -> Result<()> {
        let st = SafeTensors::deserialize(&self.weights_bytes)
            .map_err(|e| anyhow!("safetensors: {e}"))?;
        println!("[mimi] {} tensors:", st.len());
        let mut names: Vec<String> = st.names().iter().map(|s| s.to_string()).collect();
        names.sort();
        for n in names {
            if let Some(t) = st.tensor(&n).ok() {
                println!("mimi: {n}\t{:?}", t.shape());
            } else {
                println!("mimi: {n}");
            }
        }
        Ok(())
    }
}

fn select_device() -> candle::Device {
    // Prefer Metal (Apple Silicon) when available; fallback to CPU
    #[cfg(all(target_os = "macos", feature = "metal"))]
    {
        if let Ok(d) = candle::Device::new_metal(0) {
            return d;
        }
    }
    #[allow(unused_mut)]
    let mut dev = candle::Device::Cpu;
    // Future: enable CUDA if present on Linux
    dev
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

fn map_to_uint(x: f32, min: f32, max: f32, nbits: u32) -> u32 {
    let hi = ((1u64 << nbits) - 1) as f32;
    if !(min < max) {
        return 0;
    }
    let mut y = (x - min) as f32 / (max - min) as f32;
    if y.is_nan() {
        y = 0.0;
    }
    if y < 0.0 {
        y = 0.0;
    }
    if y > 1.0 {
        y = 1.0;
    }
    (y * hi).round() as u32
}

// Skeleton graph for Mimi encoder. This provides a place to wire Candle
// modules and a named-parameter loader. It currently does not execute a
// real forward pass; callers should continue using encode_frames() until
// the mapping is implemented.
pub struct MimiGraph {
    configured: bool,
    // Parameter storage and bookkeeping
    names: Vec<String>,
    consumed: HashSet<String>,
    conv1: Option<Conv1dWeights>,
    conv2: Option<Conv1dWeights>,
}

impl MimiGraph {
    fn new() -> Self {
        Self {
            configured: false,
            names: Vec::new(),
            consumed: HashSet::new(),
            conv1: None,
            conv2: None,
        }
    }

    #[allow(dead_code)]
    fn build_skeleton(
        &mut self,
        _device: &candle::Device,
        _cfg: Option<&ModelConfig>,
    ) -> Result<()> {
        // TODO: define Candle layers (e.g., conv stacks) once shapes are confirmed.
        self.configured = true;
        Ok(())
    }

    #[allow(dead_code)]
    fn load_named_parameters(&mut self, safetensors_bytes: &[u8]) -> Result<()> {
        // Record names for inspection
        let st =
            SafeTensors::deserialize(safetensors_bytes).map_err(|e| anyhow!("safetensors: {e}"))?;
        self.names = st.names().iter().map(|s| s.to_string()).collect();
        self.names.sort();

        // Placeholder mapping strategy:
        // - We currently do not know exact key layout. Print a summary and return an error to indicate mapping is pending.
        println!(
            "[mimi] loader: {} tensors discovered; mapping not implemented yet",
            self.names.len()
        );
        println!(
            "[mimi] example keys: {}",
            self.names
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
        Err(anyhow!("mimi named-parameter mapping not yet implemented"))
    }

    pub fn print_groups(&self) {
        let mut groups: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for n in &self.names {
            let key = n.split(['.', '/']).next().unwrap_or("").to_string();
            *groups.entry(key).or_insert(0) += 1;
        }
        println!("[mimi] tensor groups (by prefix):");
        for (k, v) in groups {
            println!("  {k}: {v}");
        }
    }

    pub fn print_summary(&self) {
        let total = self.names.len();
        let mapped = self.consumed.len();
        let unmapped = total.saturating_sub(mapped);
        println!(
            "[mimi] mapping summary: total={}, mapped={}, unmapped={}",
            total, mapped, unmapped
        );
        if unmapped > 0 {
            let set: std::collections::BTreeSet<_> = self
                .names
                .iter()
                .filter(|n| !self.consumed.contains(*n))
                .take(10)
                .cloned()
                .collect();
            if !set.is_empty() {
                println!(
                    "[mimi] example unmapped: {}",
                    set.into_iter().collect::<Vec<_>>().join(", ")
                );
            }
        }
    }

    fn try_load_conv1_from_env(&mut self, bytes: &[u8], device: &candle::Device) -> Result<()> {
        let prefix = match std::env::var("KYUTAI_MIMI_CONV1_PREFIX") {
            Ok(s) if !s.is_empty() => s,
            _ => return Ok(()),
        };
        let stride = std::env::var("KYUTAI_MIMI_CONV1_STRIDE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1);
        let padding = std::env::var("KYUTAI_MIMI_CONV1_PAD")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let dilation = std::env::var("KYUTAI_MIMI_CONV1_DILATION")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1);
        let groups = std::env::var("KYUTAI_MIMI_CONV1_GROUPS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1);

        let (wname, bname) = (format!("{}.weight", &prefix), format!("{}.bias", &prefix));
        let st = SafeTensors::deserialize(bytes).map_err(|e| anyhow!("safetensors: {e}"))?;
        let wv = st
            .tensor(&wname)
            .map_err(|_| anyhow!("conv1 weight not found: {}", wname))?;
        let dtype = wv.dtype();
        if dtype != StDtype::F32 {
            return Err(anyhow!("unsupported dtype for conv1: {:?}", dtype));
        }
        let wshape = wv.shape();
        if wshape.len() != 3 {
            return Err(anyhow!(
                "conv1 weight must be 3D [out,in,k], got {:?}",
                wshape
            ));
        }
        let wbytes = wv.data();
        let wf32: Vec<f32> = bytemuck::try_cast_slice::<u8, f32>(&wbytes)
            .map_err(|_| anyhow!("conv1 weight not f32 bytes"))?
            .to_vec();
        let w = Tensor::from_vec(
            wf32,
            (wshape[0] as usize, wshape[1] as usize, wshape[2] as usize),
            device,
        )
        .map_err(|e| anyhow!("tensor from_vec: {e}"))?;

        let bias = match st.tensor(&bname) {
            Ok(bv) => {
                if bv.dtype() != StDtype::F32 {
                    return Err(anyhow!("unsupported dtype for conv1 bias"));
                }
                let bshape = bv.shape();
                if bshape.len() != 1 {
                    return Err(anyhow!("conv1 bias must be 1D, got {:?}", bshape));
                }
                let bbytes = bv.data();
                let bf32: Vec<f32> = bytemuck::try_cast_slice::<u8, f32>(&bbytes)
                    .map_err(|_| anyhow!("conv1 bias not f32 bytes"))?
                    .to_vec();
                Some(
                    Tensor::from_vec(bf32, (bshape[0] as usize,), device)
                        .map_err(|e| anyhow!("tensor from_vec: {e}"))?,
                )
            }
            Err(_) => None,
        };
        self.conv1 = Some(Conv1dWeights {
            weight: w,
            bias,
            stride,
            padding,
            dilation,
            groups,
        });
        // Mark mapped names
        self.consumed.insert(wname);
        if st.tensor(&bname).is_ok() {
            self.consumed.insert(bname);
        }
        println!(
            "[mimi] conv1 loaded from prefix '{}' (stride={}, pad={}, dil={}, groups={})",
            prefix, stride, padding, dilation, groups
        );
        Ok(())
    }
}

struct Conv1dWeights {
    weight: Tensor,
    bias: Option<Tensor>,
    stride: usize,
    padding: usize,
    dilation: usize,
    groups: usize,
}

impl MimiGraph {
    fn try_load_conv2_from_env(&mut self, bytes: &[u8], device: &candle::Device) -> Result<()> {
        let prefix = match std::env::var("KYUTAI_MIMI_CONV2_PREFIX") {
            Ok(s) if !s.is_empty() => s,
            _ => return Ok(()),
        };
        let stride = std::env::var("KYUTAI_MIMI_CONV2_STRIDE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1);
        let padding = std::env::var("KYUTAI_MIMI_CONV2_PAD")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let dilation = std::env::var("KYUTAI_MIMI_CONV2_DILATION")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1);
        let groups = std::env::var("KYUTAI_MIMI_CONV2_GROUPS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1);

        let (wname, bname) = (format!("{}.weight", &prefix), format!("{}.bias", &prefix));
        let st = SafeTensors::deserialize(bytes).map_err(|e| anyhow!("safetensors: {e}"))?;
        let wv = st
            .tensor(&wname)
            .map_err(|_| anyhow!("conv2 weight not found: {}", wname))?;
        if wv.dtype() != StDtype::F32 {
            return Err(anyhow!("unsupported dtype for conv2"));
        }
        let wshape = wv.shape();
        if wshape.len() != 3 {
            return Err(anyhow!(
                "conv2 weight must be 3D [out,in,k], got {:?}",
                wshape
            ));
        }
        let wbytes = wv.data();
        let wf32: Vec<f32> = bytemuck::try_cast_slice::<u8, f32>(&wbytes)
            .map_err(|_| anyhow!("conv2 weight not f32 bytes"))?
            .to_vec();
        let w = Tensor::from_vec(
            wf32,
            (wshape[0] as usize, wshape[1] as usize, wshape[2] as usize),
            device,
        )
        .map_err(|e| anyhow!("tensor from_vec: {e}"))?;

        let bias = match st.tensor(&bname) {
            Ok(bv) => {
                if bv.dtype() != StDtype::F32 {
                    return Err(anyhow!("unsupported dtype for conv2 bias"));
                }
                let bshape = bv.shape();
                if bshape.len() != 1 {
                    return Err(anyhow!("conv2 bias must be 1D, got {:?}", bshape));
                }
                let bbytes = bv.data();
                let bf32: Vec<f32> = bytemuck::try_cast_slice::<u8, f32>(&bbytes)
                    .map_err(|_| anyhow!("conv2 bias not f32 bytes"))?
                    .to_vec();
                Some(
                    Tensor::from_vec(bf32, (bshape[0] as usize,), device)
                        .map_err(|e| anyhow!("tensor from_vec: {e}"))?,
                )
            }
            Err(_) => None,
        };
        self.conv2 = Some(Conv1dWeights {
            weight: w,
            bias,
            stride,
            padding,
            dilation,
            groups,
        });
        self.consumed.insert(wname);
        if st.tensor(&bname).is_ok() {
            self.consumed.insert(bname);
        }
        println!(
            "[mimi] conv2 loaded from prefix '{}' (stride={}, pad={}, dil={}, groups={})",
            prefix, stride, padding, dilation, groups
        );
        Ok(())
    }
}
