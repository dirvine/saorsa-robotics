use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::{self, Receiver, Sender};

type Result<T, E = anyhow::Error> = core::result::Result<T, E>;

pub struct MicStream {
    _stream: cpal::Stream,
}

pub struct MicConfig {
    pub sample_rate_hz: u32,
    pub channels: u16,
}

pub fn start_default_input_i16() -> Result<(MicStream, MicConfig, Receiver<Vec<i16>>)> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("no default input device"))?;
    let config = device
        .default_input_config()
        .map_err(|e| anyhow::anyhow!("input config: {e}"))?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels();

    let (tx, rx) = mpsc::channel::<Vec<i16>>();
    let err_fn = |err| eprintln!("input stream error: {err}");

    let stream = match config.sample_format() {
        cpal::SampleFormat::I16 => {
            build_i16_stream(&device, &config.into(), channels, tx.clone(), err_fn)?
        }
        cpal::SampleFormat::U16 => {
            build_u16_stream(&device, &config.into(), channels, tx.clone(), err_fn)?
        }
        cpal::SampleFormat::F32 => {
            build_f32_stream(&device, &config.into(), channels, tx.clone(), err_fn)?
        }
        other => return Err(anyhow::anyhow!("unsupported sample format: {other:?}")),
    };
    stream
        .play()
        .map_err(|e| anyhow::anyhow!("stream play: {e}"))?;
    Ok((
        MicStream { _stream: stream },
        MicConfig {
            sample_rate_hz: sample_rate,
            channels,
        },
        rx,
    ))
}

fn on_data_i16(data: &[i16], channels: u16, tx: &Sender<Vec<i16>>, buf: &mut Vec<i16>) {
    let mut mono = Vec::with_capacity(data.len() / channels as usize + 1);
    for frame in data.chunks_exact(channels as usize) {
        let s = frame[0]; // Take first channel for mono
        mono.push(s);
    }
    buf.extend_from_slice(&mono);
    if buf.len() >= 2048 {
        let mut out = Vec::with_capacity(buf.len());
        std::mem::swap(&mut out, buf);
        let _ = tx.send(out);
    }
}

fn build_i16_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: u16,
    tx: Sender<Vec<i16>>,
    err_fn: impl Fn(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream> {
    let mut buf = Vec::<i16>::with_capacity(4096);
    let stream = device.build_input_stream(
        config,
        move |data: &[i16], _| on_data_i16(data, channels, &tx, &mut buf),
        err_fn,
        None,
    )?;
    Ok(stream)
}

fn build_u16_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: u16,
    tx: Sender<Vec<i16>>,
    err_fn: impl Fn(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream> {
    let mut buf = Vec::<i16>::with_capacity(4096);
    let stream = device.build_input_stream(
        config,
        move |data: &[u16], _| {
            let mut mono = Vec::with_capacity(data.len() / channels as usize + 1);
            for frame in data.chunks_exact(channels as usize) {
                let s = frame[0] as i32 - 32768; // u16 to i16 centered
                mono.push(s as i16);
            }
            buf.extend_from_slice(&mono);
            if buf.len() >= 2048 {
                let mut out = Vec::with_capacity(buf.len());
                std::mem::swap(&mut out, &mut buf);
                let _ = tx.send(out);
            }
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}

fn build_f32_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: u16,
    tx: Sender<Vec<i16>>,
    err_fn: impl Fn(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream> {
    let mut buf = Vec::<i16>::with_capacity(4096);
    let stream = device.build_input_stream(
        config,
        move |data: &[f32], _| {
            let mut mono = Vec::with_capacity(data.len() / channels as usize + 1);
            for frame in data.chunks_exact(channels as usize) {
                let s = (frame[0].clamp(-1.0, 1.0) * 32767.0) as i16;
                mono.push(s);
            }
            buf.extend_from_slice(&mono);
            if buf.len() >= 2048 {
                let mut out = Vec::with_capacity(buf.len());
                std::mem::swap(&mut out, &mut buf);
                let _ = tx.send(out);
            }
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}
