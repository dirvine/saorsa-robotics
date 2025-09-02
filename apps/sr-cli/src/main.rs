use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{error, info};

use can_transport as can;
use can_transport::CanBus;
use device_registry as devreg;
use vision_stereo::CameraSource;
use voice_local::{AsrStream, TtsEngine};

#[derive(Parser, Debug)]
#[command(
    name = "sr",
    version,
    about = "Saorsa Robotics CLI",
    disable_help_subcommand = true
)]
struct Cli {
    /// Enable mock CAN backend (portable)
    #[arg(long, action = ArgAction::SetTrue, global = true)]
    mock: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum Backend {
    Mock,
    Slcan,
}

// Descriptor selection for live decode
enum DescSel {
    None,
    One(devreg::DeviceDescriptor),
    Many(devreg::DeviceRegistry),
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List available CAN interfaces
    CanList {
        /// Backend to query
        #[arg(long, value_enum, default_value_t = Backend::Mock)]
        backend: Backend,
        /// Bitrate for SLCAN (alias for future open defaults)
        #[arg(long, value_enum)]
        bitrate: Option<Bitrate>,
    },
    /// Vision: list and test camera backends
    VisionList,
    VisionTest {
        /// Device spec: index like 0 or a path
        #[arg(long, default_value = "0")]
        device: String,
        /// Use OpenCV backend if available; otherwise mock
        #[arg(long, action = ArgAction::SetTrue)]
        opencv: bool,
    },
    /// Vision: stereo calibration with chessboard
    VisionCalibStereo {
        /// Left images directory
        #[arg(long)]
        left_dir: String,
        /// Right images directory
        #[arg(long)]
        right_dir: String,
        /// Grid spec as COLSxROWS (e.g., 9x6)
        #[arg(long, default_value = "9x6")]
        grid: String,
        /// Chessboard square size in millimeters
        #[arg(long, default_value_t = 25.0)]
        square_mm: f64,
        /// Output YAML file path
        #[arg(long, default_value = "configs/calib/stereo.yaml")]
        out: String,
    },
    /// Vision: stereo capture helper (two devices)
    VisionCaptureStereo {
        /// Left device spec (index like 0 or a path)
        #[arg(long, default_value = "0")]
        left_device: String,
        /// Right device spec (index like 1 or a path)
        #[arg(long, default_value = "1")]
        right_device: String,
        /// Number of pairs to capture
        #[arg(long, default_value_t = 30u32)]
        count: u32,
        /// Output directories
        #[arg(long, default_value = "data/left")]
        left_dir: String,
        #[arg(long, default_value = "data/right")]
        right_dir: String,
    },
    /// Vision: AprilTag tag-frame alignment (single image)
    VisionTagAlign {
        /// Image path (grayscale or color)
        #[arg(long)]
        image: String,
        /// Stereo YAML intrinsics file (use left/right selection)
        #[arg(long)]
        intrinsics: String,
        /// Use left (1) or right (2) camera intrinsics from stereo YAML
        #[arg(long, default_value_t = 1u32)]
        which: u32,
        /// Tag size in meters
        #[arg(long)]
        tag_size_m: f64,
    },
    /// Vision: compute disparity and export PLY point cloud (OpenCV)
    VisionDepth {
        /// Left rectified image
        #[arg(long)]
        left: String,
        /// Right rectified image
        #[arg(long)]
        right: String,
        /// Stereo calibration YAML with Q matrix
        #[arg(long)]
        calib: String,
        /// Optional output disparity PNG path
        #[arg(long)]
        out_depth: Option<String>,
        /// Optional output PLY path
        #[arg(long)]
        out_ply: Option<String>,
        /// ROI as x,y,w,h (optional)
        #[arg(long)]
        roi: Option<String>,
        /// SGBM: minimum disparity (default 0)
        #[arg(long, default_value_t = 0)]
        min_disp: i32,
        /// SGBM: number of disparities (must be divisible by 16; default 128)
        #[arg(long, default_value_t = 128)]
        num_disp: i32,
        /// SGBM: block size (odd >=3; default 5)
        #[arg(long, default_value_t = 5)]
        block_size: i32,
        /// SGBM: uniqueness ratio (default 10)
        #[arg(long, default_value_t = 10)]
        uniq: i32,
        /// SGBM: speckle window size (default 100)
        #[arg(long, default_value_t = 100)]
        speckle_window: i32,
        /// SGBM: speckle range (default 32)
        #[arg(long, default_value_t = 32)]
        speckle_range: i32,
        /// SGBM: disp12 max diff (default 1)
        #[arg(long, default_value_t = 1)]
        disp12_maxdiff: i32,
        /// SGBM mode: sgbm or sgbm3 (default sgbm3)
        #[arg(long, value_enum, default_value = "sgbm3")]
        mode: DepthMode,
    },
    /// Voice: list available ASR/TTS backends
    VoiceList,
    /// Voice: run mock ASR stream (prints segments)
    VoiceAsrMock {
        /// Language code
        #[arg(long, default_value = "en")]
        language: String,
        /// Sample rate Hz
        #[arg(long, default_value_t = 16000u32)]
        sample_rate: u32,
    },
    /// Voice: run mock TTS (prints info)
    VoiceTtsMock {
        /// Text to synthesize
        #[arg(long)]
        text: String,
        /// Sample rate Hz
        #[arg(long, default_value_t = 22050u32)]
        sample_rate: u32,
    },
    /// Voice: launch Kyutai STT hotkey app (from this workspace)
    VoiceKyutai {
        /// Config file path for the Kyutai app
        #[arg(long, default_value = "config.json")]
        config: String,
        /// Hotkey to trigger recording (e.g., F12)
        #[arg(long)]
        hotkey: Option<String>,
        /// Recording duration in seconds
        #[arg(long, default_value_t = 5u32)]
        duration: u32,
        /// Dump Mimi/decoder safetensors keys and exit
        #[arg(long, action = ArgAction::SetTrue)]
        dump_weights: bool,
    },
    /// Voice: capture mic audio and stream to ASR backend
    VoiceAsrCapture {
        /// Backend
        #[arg(long, value_enum, default_value = "mock")]
        backend: VoiceBackend,
        /// Language code
        #[arg(long, default_value = "en")]
        language: String,
        /// Duration seconds (0 for indefinite; Ctrl-C to stop)
        #[arg(long, default_value_t = 10u32)]
        duration: u32,
    },
    /// Voice: record mic audio to a WAV file
    VoiceRecord {
        /// Duration seconds
        #[arg(long, default_value_t = 5u32)]
        duration: u32,
        /// Output WAV path
        #[arg(long, default_value = "out.wav")]
        out: String,
    },
    /// Learning: show learning system status
    LearnStatus,
    /// Learning: start a training job
    LearnTrain {
        /// Model name to train
        #[arg(long)]
        model: String,
        /// Dataset path
        #[arg(long, default_value = "data/learning")]
        dataset: String,
        /// Output model name
        #[arg(long)]
        output: String,
    },
    /// Learning: list available models
    LearnModels,
    /// Learning: record a human intervention
    LearnIntervention {
        /// Original action (JSON)
        #[arg(long)]
        original_action: String,
        /// Corrected action (JSON)
        #[arg(long)]
        corrected_action: String,
        /// Reason for intervention
        #[arg(long)]
        reason: String,
    },
    /// Replay frames from an .srlog file; optionally re-send
    CanReplay {
        /// Path to .srlog file
        #[arg(long)]
        from: String,
        /// Backend to use when sending
        #[arg(long, value_enum, default_value_t = Backend::Mock)]
        backend: Backend,
        /// Device to send to (required when --send)
        #[arg(long)]
        device: Option<String>,
        /// Actually send frames back out (otherwise just print)
        #[arg(long, action = ArgAction::SetTrue)]
        send: bool,
        /// Bitrate for SLCAN when sending
        #[arg(long, value_enum)]
        bitrate: Option<Bitrate>,
        /// Respect timestamps to approximate real-time playback
        #[arg(long, action = ArgAction::SetTrue)]
        realtime: bool,
    },
    /// Validate and list device descriptors
    DeviceValidate {
        /// YAML file path
        #[arg(long)]
        file: Option<String>,
        /// Directory containing YAML descriptors
        #[arg(long)]
        dir: Option<String>,
        /// Print JSON after validation
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// List devices from a directory
    DeviceList {
        /// Directory containing YAML descriptors
        #[arg(long, default_value = "configs/devices")]
        dir: String,
    },
    /// Build frames from a device descriptor and optionaly send
    DeviceBuild {
        /// YAML descriptor file path
        #[arg(long)]
        file: String,
        /// Joint name to target
        #[arg(long)]
        joint: String,
        /// Command mode
        #[arg(long, value_enum)]
        mode: CmdMode,
        /// Command value (units depend on mode)
        #[arg(long)]
        value: f32,
        /// Actually send the frames (otherwise just print)
        #[arg(long, action = ArgAction::SetTrue)]
        send: bool,
        /// Backend to use when sending
        #[arg(long, value_enum, default_value_t = Backend::Mock)]
        backend: Backend,
        /// Device path/name to send to
        #[arg(long)]
        device: Option<String>,
        /// Bitrate for SLCAN when sending
        #[arg(long, value_enum)]
        bitrate: Option<Bitrate>,
    },
    /// Decode a telemetry frame using a descriptor
    DeviceDecode {
        /// YAML descriptor file path
        #[arg(long)]
        file: String,
        /// CAN ID (hex like 0x240)
        #[arg(long)]
        id: String,
        /// Data bytes as hex (8 bytes typical)
        #[arg(long, value_delimiter = ' ')]
        data: Vec<String>,
    },
    /// Sniff frames from a CAN interface
    CanSniff {
        /// Interface name (e.g., can0)
        #[arg(long, default_value = "mock0")]
        device: String,
        /// Number of frames to read before exiting
        #[arg(long, default_value_t = 10)]
        count: u32,
        /// Backend to use
        #[arg(long, value_enum, default_value_t = Backend::Mock)]
        backend: Backend,
        /// Bitrate for SLCAN (only when backend=slcan)
        #[arg(long, value_enum)]
        bitrate: Option<Bitrate>,
        /// Write frames to .srlog (NDJSON) file
        #[arg(long)]
        to: Option<String>,
        /// Decode telemetry using a single descriptor file
        #[arg(long)]
        desc_file: Option<String>,
        /// Decode telemetry using all descriptors in a directory
        #[arg(long)]
        desc_dir: Option<String>,
        /// Print decoded telemetry JSON to stdout
        #[arg(long, action = ArgAction::SetTrue)]
        decode: bool,
        /// Write decoded telemetry to a JSONL file
        #[arg(long)]
        decode_to: Option<String>,
    },
    /// Send a CAN frame
    CanSend {
        #[arg(long, default_value = "mock0")]
        device: String,
        /// CAN ID in hex (e.g., 0x123)
        #[arg(long)]
        id: String,
        /// Data bytes as hex, space-separated (e.g., "01 02 03")
        #[arg(long, value_delimiter = ' ')]
        data: Vec<String>,
        /// Backend to use
        #[arg(long, value_enum, default_value_t = Backend::Mock)]
        backend: Backend,
        /// Bitrate for SLCAN (only when backend=slcan)
        #[arg(long, value_enum)]
        bitrate: Option<Bitrate>,
    },
    /// Validate CAN device by opening, setting bitrate, and sending a probe frame
    CanDoctor {
        /// Device path or name (e.g., /dev/tty.usbserial-0001 or mock0)
        #[arg(long, default_value = "mock0")]
        device: String,
        /// Backend to use
        #[arg(long, value_enum, default_value_t = Backend::Mock)]
        backend: Backend,
        /// Bitrate for SLCAN
        #[arg(long, value_enum)]
        bitrate: Option<Bitrate>,
        /// Probe CAN ID in hex (e.g., 0x123)
        #[arg(long, default_value = "0x123")]
        id: String,
        /// Probe data bytes as hex
        #[arg(long, value_delimiter = ' ', default_values_t = vec!["00".to_string(), "00".to_string()])]
        data: Vec<String>,
        /// Milliseconds to listen for a frame after send (0 to skip)
        #[arg(long, default_value_t = 300u32)]
        recv_ms: u32,
    },
}

fn main() -> Result<()> {
    setup_tracing();
    let cli = Cli::parse();

    match cli.command {
        Commands::CanList { backend, bitrate } => can_list_backend(backend, bitrate),
        Commands::CanSniff {
            device,
            count,
            backend,
            bitrate,
            to,
            desc_file,
            desc_dir,
            decode,
            decode_to,
        } => can_sniff_backend(
            backend,
            &device,
            count,
            bitrate,
            to.as_deref(),
            desc_file.as_deref(),
            desc_dir.as_deref(),
            decode,
            decode_to.as_deref(),
        ),
        Commands::CanSend {
            device,
            id,
            data,
            backend,
            bitrate,
        } => can_send_backend(backend, &device, &id, &data, bitrate),
        Commands::CanDoctor {
            device,
            backend,
            bitrate,
            id,
            data,
            recv_ms,
        } => can_doctor_backend(backend, &device, &id, &data, bitrate, recv_ms),
        Commands::CanReplay {
            from,
            backend,
            device,
            send,
            bitrate,
            realtime,
        } => can_replay(&from, backend, device.as_deref(), send, bitrate, realtime),
        Commands::DeviceValidate { file, dir, json } => {
            device_validate(file.as_deref(), dir.as_deref(), json)
        }
        Commands::DeviceList { dir } => device_list(&dir),
        Commands::DeviceBuild {
            file,
            joint,
            mode,
            value,
            send,
            backend,
            device,
            bitrate,
        } => device_build(
            &file,
            &joint,
            mode,
            value,
            send,
            backend,
            device.as_deref(),
            bitrate,
        ),
        Commands::DeviceDecode { file, id, data } => device_decode(&file, &id, &data),
        Commands::VisionList => vision_list(),
        Commands::VisionTest { device, opencv } => vision_test(&device, opencv),
        Commands::VisionCalibStereo {
            left_dir,
            right_dir,
            grid,
            square_mm,
            out,
        } => vision_calib_stereo(&left_dir, &right_dir, &grid, square_mm, &out),
        Commands::VisionCaptureStereo {
            left_device,
            right_device,
            count,
            left_dir,
            right_dir,
        } => vision_capture_stereo(&left_device, &right_device, count, &left_dir, &right_dir),
        Commands::VisionTagAlign {
            image,
            intrinsics,
            which,
            tag_size_m,
        } => vision_tag_align(&image, &intrinsics, which, tag_size_m),
        Commands::VisionDepth {
            left,
            right,
            calib,
            out_depth,
            out_ply,
            roi,
            min_disp,
            num_disp,
            block_size,
            uniq,
            speckle_window,
            speckle_range,
            disp12_maxdiff,
            mode,
        } => vision_depth(
            &left,
            &right,
            &calib,
            out_depth.as_deref(),
            out_ply.as_deref(),
            roi.as_deref(),
            min_disp,
            num_disp,
            block_size,
            uniq,
            speckle_window,
            speckle_range,
            disp12_maxdiff,
            mode,
        ),
        Commands::VoiceList => voice_list(),
        Commands::VoiceAsrMock {
            language,
            sample_rate,
        } => voice_asr_mock(&language, sample_rate),
        Commands::VoiceTtsMock { text, sample_rate } => voice_tts_mock(&text, sample_rate),
        Commands::VoiceAsrCapture {
            backend,
            language,
            duration,
        } => voice_asr_capture(backend, &language, duration),
        Commands::VoiceRecord { duration, out } => voice_record(duration, &out),
        Commands::VoiceKyutai {
            config,
            hotkey,
            duration,
            dump_weights,
        } => voice_kyutai(&config, hotkey.as_deref(), duration, dump_weights),
        Commands::LearnStatus => learn_status(),
        Commands::LearnTrain {
            model,
            dataset,
            output,
        } => learn_train(&model, &dataset, &output),
        Commands::LearnModels => learn_models(),
        Commands::LearnIntervention {
            original_action,
            corrected_action,
            reason,
        } => learn_intervention(&original_action, &corrected_action, &reason),
    }
}

fn setup_tracing() {
    // Best-effort; avoid panics if already set
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
}

fn can_list_backend(backend: Backend, bitrate: Option<Bitrate>) -> Result<()> {
    match backend {
        Backend::Mock => {
            for bus in can::MockBus::list()? {
                println!("{}\t{}", bus.name, bus.driver);
            }
        }
        Backend::Slcan => {
            let suffix = bitrate
                .map(|b| format!("\tbitrate={}", b.as_str()))
                .unwrap_or_default();
            for bus in can::SlcanBus::list()? {
                println!("{}\t{}{}", bus.name, bus.driver, suffix);
            }
        }
    }
    Ok(())
}

fn can_doctor_backend(
    backend: Backend,
    device: &str,
    id_str: &str,
    data_hex: &[String],
    bitrate: Option<Bitrate>,
    recv_ms: u32,
) -> Result<()> {
    let id = parse_id(id_str).ok_or_else(|| anyhow::anyhow!("invalid CAN id: {id_str}"))?;
    let bytes = parse_hex_bytes(data_hex)?;
    let frame =
        can::CanFrame::new(id, &bytes).ok_or_else(|| anyhow::anyhow!("invalid frame length"))?;

    println!(
        "doctor: backend={backend:?} device={device} bitrate={}",
        bitrate.map(|b| b.as_str()).unwrap_or("default")
    );
    match backend {
        Backend::Mock => {
            let mut bus = can::MockBus::open(device)?;
            println!("open: ok");
            bus.send(&frame)?;
            println!("send: ok ({} bytes)", bytes.len());
            if recv_ms > 0 {
                match bus.recv(Some(recv_ms as u64)) {
                    Ok(f) => {
                        print!("recv: ");
                        print_frame(&f);
                    }
                    Err(e) => {
                        eprintln!("recv: no frame within {} ms ({e})", recv_ms);
                    }
                }
            }
        }
        Backend::Slcan => {
            let br = bitrate.map(|b| b.into_transport());
            let mut bus = can::SlcanBus::open_with(device, br)?;
            println!("open: ok");
            bus.send(&frame)?;
            println!("send: ok ({} bytes)", bytes.len());
            if recv_ms > 0 {
                match bus.recv(Some(recv_ms as u64)) {
                    Ok(f) => {
                        print!("recv: ");
                        print_frame(&f);
                    }
                    Err(e) => {
                        eprintln!("recv: no frame within {} ms ({e})", recv_ms);
                    }
                }
            }
        }
    }
    println!("doctor: done");
    Ok(())
}

fn can_sniff_backend(
    backend: Backend,
    device: &str,
    count: u32,
    bitrate: Option<Bitrate>,
    to: Option<&str>,
    desc_file: Option<&str>,
    desc_dir: Option<&str>,
    decode_stdout: bool,
    decode_to: Option<&str>,
) -> Result<()> {
    // Optional log writers
    let mut writer = match to {
        Some(path) => {
            let file = File::create(path)?;
            let mut w = BufWriter::new(file);
            // Header line
            let header = srlog_header_line(backend, device, bitrate);
            w.write_all(header.as_bytes())?;
            w.write_all(b"\n")?;
            Some(w)
        }
        None => None,
    };
    let mut dec_writer = match decode_to {
        Some(path) => {
            let file = File::create(path)?;
            let w = BufWriter::new(file);
            Some(w)
        }
        None => None,
    };

    // Optional descriptors
    let desc_sel = if let Some(p) = desc_file {
        DescSel::One(devreg::load_descriptor_file(p)?)
    } else if let Some(d) = desc_dir {
        DescSel::Many(devreg::load_descriptors_dir(d)?)
    } else {
        DescSel::None
    };
    match backend {
        Backend::Mock => {
            let mut bus = can::MockBus::open(device)?;
            for _ in 0..count {
                handle_frame(
                    &mut writer,
                    &mut dec_writer,
                    &desc_sel,
                    decode_stdout,
                    &bus.recv(Some(250))?,
                )?;
            }
        }
        Backend::Slcan => {
            let br = bitrate.map(|b| b.into_transport());
            let mut bus = can::SlcanBus::open_with(device, br)?;
            for _ in 0..count {
                handle_frame(
                    &mut writer,
                    &mut dec_writer,
                    &desc_sel,
                    decode_stdout,
                    &bus.recv(Some(500))?,
                )?;
            }
        }
    }
    Ok(())
}

fn handle_frame(
    writer: &mut Option<BufWriter<File>>,
    dec_writer: &mut Option<BufWriter<File>>,
    desc_sel: &DescSel,
    decode_stdout: bool,
    frame: &can::CanFrame,
) -> Result<()> {
    print_frame(frame);
    if let Some(w) = writer.as_mut() {
        let line = srlog_record_line(frame);
        w.write_all(line.as_bytes())?;
        w.write_all(b"\n")?;
    }
    // Decode if requested
    if !matches!(desc_sel, DescSel::None) {
        let ts = frame.timestamp.map(|t| t.0);
        let len = usize::from(frame.len);
        let data = &frame.data[..len.min(frame.data.len())];
        let rec_opt = match desc_sel {
            DescSel::None => None,
            DescSel::One(desc) => devreg::decode_by_id(desc, frame.id, data, ts),
            DescSel::Many(reg) => {
                let mut rec = None;
                for d in reg.devices.values() {
                    if let Some(r) = devreg::decode_by_id(d, frame.id, data, ts) {
                        rec = Some(r);
                        break;
                    }
                }
                rec
            }
        };
        if let Some(rec) = rec_opt {
            let json = serde_json::to_string(&rec)?;
            if decode_stdout {
                println!("{json}");
            }
            if let Some(w) = dec_writer.as_mut() {
                w.write_all(json.as_bytes())?;
                w.write_all(b"\n")?;
            }
        }
    }
    Ok(())
}

fn can_send_backend(
    backend: Backend,
    device: &str,
    id_str: &str,
    data_hex: &[String],
    bitrate: Option<Bitrate>,
) -> Result<()> {
    let id = parse_id(id_str).ok_or_else(|| anyhow::anyhow!("invalid CAN id: {id_str}"))?;
    let bytes = parse_hex_bytes(data_hex)?;
    let frame =
        can::CanFrame::new(id, &bytes).ok_or_else(|| anyhow::anyhow!("invalid frame length"))?;

    match backend {
        Backend::Mock => {
            let mut bus = can::MockBus::open(device)?;
            bus.send(&frame)?;
            info!(device, "sent frame (mock)");
        }
        Backend::Slcan => {
            let br = bitrate.map(|b| b.into_transport());
            let mut bus = can::SlcanBus::open_with(device, br)?;
            bus.send(&frame)?;
            info!(device, "sent frame (slcan)");
        }
    }
    Ok(())
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum Bitrate {
    #[value(name = "10k")]
    B10k,
    #[value(name = "20k")]
    B20k,
    #[value(name = "50k")]
    B50k,
    #[value(name = "100k")]
    B100k,
    #[value(name = "125k")]
    B125k,
    #[value(name = "250k")]
    B250k,
    #[value(name = "500k")]
    B500k,
    #[value(name = "800k")]
    B800k,
    #[value(name = "1m")]
    B1M,
}

impl Bitrate {
    fn into_transport(self) -> can::SlcanBitrate {
        match self {
            Bitrate::B10k => can::SlcanBitrate::B10k,
            Bitrate::B20k => can::SlcanBitrate::B20k,
            Bitrate::B50k => can::SlcanBitrate::B50k,
            Bitrate::B100k => can::SlcanBitrate::B100k,
            Bitrate::B125k => can::SlcanBitrate::B125k,
            Bitrate::B250k => can::SlcanBitrate::B250k,
            Bitrate::B500k => can::SlcanBitrate::B500k,
            Bitrate::B800k => can::SlcanBitrate::B800k,
            Bitrate::B1M => can::SlcanBitrate::B1M,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Bitrate::B10k => "10k",
            Bitrate::B20k => "20k",
            Bitrate::B50k => "50k",
            Bitrate::B100k => "100k",
            Bitrate::B125k => "125k",
            Bitrate::B250k => "250k",
            Bitrate::B500k => "500k",
            Bitrate::B800k => "800k",
            Bitrate::B1M => "1m",
        }
    }
}

fn parse_id(s: &str) -> Option<can::CanId> {
    let s_trim = s.trim();
    let no_prefix = s_trim.strip_prefix("0x").unwrap_or(s_trim);
    let parsed = u32::from_str_radix(no_prefix, 16).ok()?;
    if parsed <= 0x7FF {
        can::CanId::standard(parsed as u16)
    } else if parsed <= 0x1FFF_FFFF {
        can::CanId::extended(parsed)
    } else {
        None
    }
}

fn parse_hex_bytes(items: &[String]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(items.len());
    for s in items {
        let t = s.trim();
        let no_prefix = t.strip_prefix("0x").unwrap_or(t);
        let b = u8::from_str_radix(no_prefix, 16)
            .map_err(|e| anyhow::anyhow!("invalid hex byte '{t}': {e}"))?;
        out.push(b);
    }
    Ok(out)
}

fn print_frame(f: &can::CanFrame) {
    let ts = f
        .timestamp
        .map(|t| {
            t.0.format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "".into())
        })
        .unwrap_or_else(|| "".into());
    let mut data_s = String::new();
    let len = usize::from(f.len);
    for i in 0..len {
        let _ = core::fmt::Write::write_fmt(&mut data_s, format_args!("{:02X} ", f.data[i]));
    }
    println!(
        "{id}\tlen={len}\t{data}\t{ts}",
        id = f.id,
        data = data_s.trim_end(),
        ts = ts
    );
}

fn srlog_header_line(backend: Backend, device: &str, bitrate: Option<Bitrate>) -> String {
    let header = SrlogHeader {
        format: "srlog".to_string(),
        version: 1,
        backend: format!("{backend:?}"),
        device: device.to_string(),
        bitrate: bitrate.map(|b| b.as_str().to_string()),
    };
    serde_json::to_string(&header).unwrap_or_else(|_| "{}".to_string())
}

fn srlog_record_line(f: &can::CanFrame) -> String {
    let ts = f
        .timestamp
        .map(|t| {
            t.0.format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "".into())
        })
        .unwrap_or_else(|| "".into());
    let mut data_hex = String::new();
    let len = usize::from(f.len);
    for i in 0..len {
        let _ = core::fmt::Write::write_fmt(&mut data_hex, format_args!("{:02X}", f.data[i]));
    }
    let rec = SrlogRecord {
        ts,
        id: format!("{}", f.id),
        ext: f.id.is_extended(),
        len,
        data: data_hex,
    };
    serde_json::to_string(&rec).unwrap_or_else(|_| "{}".to_string())
}

#[derive(Serialize, Deserialize)]
struct SrlogHeader {
    format: String,
    version: u32,
    backend: String,
    device: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    bitrate: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct SrlogRecord {
    ts: String,
    id: String,
    ext: bool,
    len: usize,
    data: String,
}

fn can_replay(
    path: &str,
    backend: Backend,
    device: Option<&str>,
    send: bool,
    bitrate: Option<Bitrate>,
    realtime: bool,
) -> Result<()> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut first = String::new();
    let _ = reader.read_line(&mut first)?;
    let _header: SrlogHeader = match serde_json::from_str(&first) {
        Ok(h) => h,
        Err(e) => {
            error!("invalid srlog header: {e}");
            return Err(anyhow::anyhow!("invalid srlog header"));
        }
    };
    println!(
        "replay: {path} backend={backend:?} device={} send={send} realtime={realtime}",
        device.unwrap_or("(none)")
    );

    let mut out_bus_mock;
    let mut out_bus_slcan;
    enum BusSel<'a> {
        None,
        Mock(&'a mut can::MockBus),
        Slcan(&'a mut can::SlcanBus),
    }

    // Optional bus open
    let mut bus_sel = BusSel::None;
    if send {
        let dev = device.ok_or_else(|| anyhow::anyhow!("--device required when --send"))?;
        bus_sel = match backend {
            Backend::Mock => {
                out_bus_mock = can::MockBus::open(dev)?;
                BusSel::Mock(&mut out_bus_mock)
            }
            Backend::Slcan => {
                let br = bitrate.map(|b| b.into_transport());
                out_bus_slcan = can::SlcanBus::open_with(dev, br)?;
                BusSel::Slcan(&mut out_bus_slcan)
            }
        };
    }

    // For real-time, track previous ts
    let mut last_ts: Option<time::OffsetDateTime> = None;
    let start = Instant::now();
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            break;
        }
        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }
        let rec: SrlogRecord = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                error!("bad record: {e}");
                continue;
            }
        };
        // Convert record to frame
        let id = match parse_id(&rec.id) {
            Some(i) => i,
            None => {
                error!("bad id: {}", rec.id);
                continue;
            }
        };
        let bytes = match parse_hex_compact(&rec.data) {
            Ok(b) => b,
            Err(e) => {
                error!("bad data: {e}");
                continue;
            }
        };
        let mut frame = match can::CanFrame::new(id, &bytes) {
            Some(f) => f,
            None => {
                error!("bad frame len");
                continue;
            }
        };
        frame.timestamp = None;

        if realtime {
            if let Ok(ts) =
                time::OffsetDateTime::parse(&rec.ts, &time::format_description::well_known::Rfc3339)
            {
                if let Some(prev) = last_ts {
                    let delta = ts - prev;
                    let ms = delta.whole_milliseconds().max(0) as u64;
                    if ms > 0 {
                        thread::sleep(Duration::from_millis(ms));
                    }
                } else {
                    // Align start to now
                    let _ = start; // reserved
                }
                last_ts = Some(ts);
            }
        }

        print_frame(&frame);
        if send {
            match &mut bus_sel {
                BusSel::None => {}
                BusSel::Mock(b) => {
                    b.send(&frame)?;
                }
                BusSel::Slcan(b) => {
                    b.send(&frame)?;
                }
            }
        }
    }
    println!("replay: done");
    Ok(())
}

fn parse_hex_compact(s: &str) -> Result<Vec<u8>> {
    let t = s.trim();
    if t.len() % 2 != 0 {
        return Err(anyhow::anyhow!("odd hex length"));
    }
    let mut out = Vec::with_capacity(t.len() / 2);
    let bytes = t.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = bytes[i] as char;
        let lo = bytes[i + 1] as char;
        let hx = u8::from_str_radix(&format!("{hi}{lo}"), 16)
            .map_err(|e| anyhow::anyhow!("invalid hex: {e}"))?;
        out.push(hx);
        i += 2;
    }
    Ok(out)
}

fn device_validate(file: Option<&str>, dir: Option<&str>, json: bool) -> Result<()> {
    match (file, dir) {
        (Some(f), None) => {
            let desc = devreg::load_descriptor_file(f)?;
            println!(
                "ok: {} (protocol={}, bus={})",
                desc.id, desc.protocol, desc.bus
            );
            if json {
                println!("{}", serde_json::to_string_pretty(&desc)?);
            }
        }
        (None, Some(d)) => {
            let reg = devreg::load_descriptors_dir(d)?;
            println!("ok: loaded {} devices", reg.devices.len());
            if json {
                println!("{}", serde_json::to_string_pretty(&reg.devices)?);
            }
        }
        _ => {
            return Err(anyhow::anyhow!("provide --file <path> or --dir <dir>"));
        }
    }
    Ok(())
}

fn device_list(dir: &str) -> Result<()> {
    let reg = devreg::load_descriptors_dir(dir)?;
    for (id, d) in reg.devices {
        println!("{id}\tbus={}\tprotocol={}", d.bus, d.protocol);
    }
    Ok(())
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum CmdMode {
    Torque,
    Position,
    Velocity,
}

fn device_build(
    file: &str,
    joint: &str,
    mode: CmdMode,
    value: f32,
    send: bool,
    backend: Backend,
    device: Option<&str>,
    bitrate: Option<Bitrate>,
) -> Result<()> {
    let desc = devreg::load_descriptor_file(file)?;
    let cmd = match mode {
        CmdMode::Torque => devreg::JointCommand::TorqueNm(value),
        CmdMode::Position => devreg::JointCommand::Position(value),
        CmdMode::Velocity => devreg::JointCommand::Velocity(value),
    };
    let frames = devreg::build_frames_for_joint(&desc, joint, cmd)?;
    if frames.is_empty() {
        println!("no frames built (check descriptor frames mapping)");
        return Ok(());
    }
    for f in &frames {
        print_frame(f);
    }

    if send {
        let dev = device.ok_or_else(|| anyhow::anyhow!("--device required when --send"))?;
        match backend {
            Backend::Mock => {
                let mut bus = can::MockBus::open(dev)?;
                for f in &frames {
                    bus.send(f)?;
                }
                info!(device = dev, n = frames.len(), "sent frames (mock)");
            }
            Backend::Slcan => {
                let br = bitrate.map(|b| b.into_transport());
                let mut bus = can::SlcanBus::open_with(dev, br)?;
                for f in &frames {
                    bus.send(f)?;
                }
                info!(device = dev, n = frames.len(), "sent frames (slcan)");
            }
        }
    }
    Ok(())
}

fn device_decode(file: &str, id_str: &str, data_hex: &[String]) -> Result<()> {
    let desc = devreg::load_descriptor_file(file)?;
    let id = parse_id(id_str).ok_or_else(|| anyhow::anyhow!("invalid CAN id: {id_str}"))?;
    let bytes = parse_hex_bytes(data_hex)?;
    let mut buf = [0u8; 8];
    for (i, b) in bytes.iter().enumerate() {
        if i < 8 {
            buf[i] = *b;
        }
    }
    let rec = devreg::decode_by_id(&desc, id, &buf, None)
        .ok_or_else(|| anyhow::anyhow!("no decoder for id in descriptor"))?;
    println!("{}", serde_json::to_string_pretty(&rec)?);
    Ok(())
}

fn vision_list() -> Result<()> {
    // Basic info; full enumeration depends on backend
    println!("vision backends: mock (enable opencv via cargo features)");
    println!("hint: use 'sr vision-test --device 0 --opencv' to test OpenCV backend");
    Ok(())
}

fn vision_test(device: &str, opencv: bool) -> Result<()> {
    if opencv {
        #[cfg(feature = "vision-stereo/opencv")]
        {
            let mut cam = vision_stereo::OpenCvCamera::open(device)
                .map_err(|e| anyhow::anyhow!("opencv open failed: {e}"))?;
            let f = cam
                .read()
                .map_err(|e| anyhow::anyhow!("opencv read failed: {e}"))?;
            println!(
                "opencv: {}x{} {:?} ts={:?}",
                f.width, f.height, f.pixel_format, f.ts
            );
            return Ok(());
        }
        #[cfg(not(feature = "vision-stereo/opencv"))]
        {
            println!("OpenCV backend not enabled at compile time");
            return Ok(());
        }
    }
    let mut cam = vision_stereo::MockCamera::open(device)
        .map_err(|e| anyhow::anyhow!("mock open failed: {e}"))?;
    let f = cam
        .read()
        .map_err(|e| anyhow::anyhow!("mock read failed: {e}"))?;
    println!(
        "mock: {}x{} {:?} ts={:?}",
        f.width, f.height, f.pixel_format, f.ts
    );
    Ok(())
}

fn vision_calib_stereo(
    left_dir: &str,
    right_dir: &str,
    grid: &str,
    square_mm: f64,
    out: &str,
) -> Result<()> {
    // Parse grid like "9x6"
    let parts: Vec<&str> = grid.split('x').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!("grid must be COLSxROWS, e.g., 9x6"));
    }
    let cols: i32 = parts[0]
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid cols"))?;
    let rows: i32 = parts[1]
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid rows"))?;

    #[cfg(feature = "vision-stereo/opencv")]
    {
        let calib =
            vision_stereo::calib::stereo_calibrate(left_dir, right_dir, rows, cols, square_mm)
                .map_err(|e| anyhow::anyhow!("stereo calibrate failed: {e}"))?;
        vision_stereo::calib::write_yaml(&calib, out)
            .map_err(|e| anyhow::anyhow!("write yaml failed: {e}"))?;
        println!("wrote calibration to {out}");
        return Ok(());
    }
    #[cfg(not(feature = "vision-stereo/opencv"))]
    {
        println!("OpenCV backend not enabled at compile time; rebuild with --features vision-stereo/opencv");
        Ok(())
    }
}

fn vision_capture_stereo(
    left_device: &str,
    right_device: &str,
    count: u32,
    left_dir: &str,
    right_dir: &str,
) -> Result<()> {
    #[cfg(feature = "vision-stereo/opencv")]
    {
        std::fs::create_dir_all(left_dir)?;
        std::fs::create_dir_all(right_dir)?;

        let mut cam_l = vision_stereo::OpenCvCamera::open(left_device)
            .map_err(|e| anyhow::anyhow!("left open failed: {e}"))?;
        let mut cam_r = vision_stereo::OpenCvCamera::open(right_device)
            .map_err(|e| anyhow::anyhow!("right open failed: {e}"))?;

        for i in 0..count {
            let f_l = cam_l
                .read()
                .map_err(|e| anyhow::anyhow!("left read failed: {e}"))?;
            let f_r = cam_r
                .read()
                .map_err(|e| anyhow::anyhow!("right read failed: {e}"))?;

            let name = format!("{:05}.png", i);
            let path_l = format!("{}/{}", left_dir.trim_end_matches('/'), name);
            let path_r = format!("{}/{}", right_dir.trim_end_matches('/'), name);
            vision_stereo::io::write_png(&path_l, &f_l)
                .map_err(|e| anyhow::anyhow!("write left failed: {e}"))?;
            vision_stereo::io::write_png(&path_r, &f_r)
                .map_err(|e| anyhow::anyhow!("write right failed: {e}"))?;
            println!("saved {} and {}", path_l, path_r);
        }
        return Ok(());
    }
    #[cfg(not(feature = "vision-stereo/opencv"))]
    {
        println!("OpenCV backend not enabled at compile time; rebuild with --features vision-stereo/opencv");
        Ok(())
    }
}

fn vision_tag_align(image_path: &str, intr_path: &str, which: u32, tag_size_m: f64) -> Result<()> {
    #[cfg(all(feature = "vision-stereo/opencv", feature = "vision-stereo/apriltag"))]
    {
        // Load stereo YAML intrinsics and pick left/right
        #[derive(serde::Deserialize)]
        struct StereoYaml {
            k1: Vec<Vec<f64>>,
            d1: Vec<f64>,
            k2: Vec<Vec<f64>>,
            d2: Vec<f64>,
        }
        let s: StereoYaml = serde_yaml::from_str(&std::fs::read_to_string(intr_path)?)?;
        let (k, d) = if which == 1 {
            (&s.k1, &s.d1)
        } else {
            (&s.k2, &s.d2)
        };
        if k.len() != 3 || k[0].len() != 3 {
            return Err(anyhow::anyhow!("invalid K matrix"));
        }
        let intr = vision_stereo::tags::CameraIntrinsics {
            fx: k[0][0],
            fy: k[1][1],
            cx: k[0][2],
            cy: k[1][2],
        };

        // Load image via OpenCV and convert to grayscale buffer
        let mat = opencv::imgcodecs::imread(image_path, opencv::imgcodecs::IMREAD_GRAYSCALE)
            .map_err(|e| anyhow::anyhow!("cv imread failed: {e}"))?;
        if mat.empty() {
            return Err(anyhow::anyhow!("empty image"));
        }
        let width = mat.cols() as usize;
        let height = mat.rows() as usize;
        let buf = mat
            .data_bytes()
            .map_err(|e| anyhow::anyhow!("cv data_bytes: {e}"))?;

        let poses = vision_stereo::tags::estimate_tag_pose_from_image(
            buf,
            width,
            height,
            &intr,
            tag_size_m,
            Some(d.as_slice()),
        )
        .map_err(|e| anyhow::anyhow!("detect failed: {e}"))?;
        if poses.is_empty() {
            println!("no tags detected");
        } else {
            for (i, p) in poses.iter().enumerate() {
                println!("pose#{i}: R={:?} t={:?}", p.r, p.t);
            }
        }
        return Ok(());
    }
    #[cfg(not(all(feature = "vision-stereo/opencv", feature = "vision-stereo/apriltag")))]
    {
        println!(
            "AprilTag alignment requires --features vision-stereo/opencv,vision-stereo/apriltag"
        );
        Ok(())
    }
}

fn parse_roi(s: &str) -> Option<(i32, i32, i32, i32)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 4 {
        return None;
    }
    let x = parts[0].parse().ok()?;
    let y = parts[1].parse().ok()?;
    let w = parts[2].parse().ok()?;
    let h = parts[3].parse().ok()?;
    Some((x, y, w, h))
}

fn vision_depth(
    left: &str,
    right: &str,
    calib: &str,
    out_depth: Option<&str>,
    out_ply: Option<&str>,
    roi: Option<&str>,
    min_disp: i32,
    num_disp: i32,
    block_size: i32,
    uniq: i32,
    speckle_window: i32,
    speckle_range: i32,
    disp12_maxdiff: i32,
    mode: DepthMode,
) -> Result<()> {
    #[cfg(feature = "vision-stereo/opencv")]
    {
        let roi_parsed = roi.and_then(parse_roi);
        let params = vision_stereo::depth::StereoSgbmParams {
            min_disp,
            num_disp,
            block_size,
            uniqueness_ratio: uniq,
            speckle_window_size: speckle_window,
            speckle_range,
            disp12_max_diff: disp12_maxdiff,
            mode: match mode {
                DepthMode::Sgbm => opencv::calib3d::StereoSGBM_MODE_SGBM,
                DepthMode::Sgbm3 => opencv::calib3d::StereoSGBM_MODE_SGBM_3WAY,
            },
        };
        let stats = vision_stereo::depth::depth_from_rectified_to_ply(
            left, right, calib, out_depth, out_ply, roi_parsed, params,
        )
        .map_err(|e| anyhow::anyhow!("vision depth failed: {e}"))?;
        if let Some(p) = out_depth {
            println!("wrote disparity PNG: {p}");
        }
        if let Some(p) = out_ply {
            println!("wrote point cloud PLY: {p}");
        }
        println!(
            "disparity: {} ms, reproject: {} ms, points: {}",
            stats.disparity_ms, stats.reproject_ms, stats.points
        );
        return Ok(());
    }
    #[cfg(not(feature = "vision-stereo/opencv"))]
    {
        println!("OpenCV backend not enabled at compile time; rebuild sr-cli with OpenCV support");
        Ok(())
    }
}

fn voice_list() -> Result<()> {
    println!("ASR backends: mock (whisper_cpp, faster_whisper, vosk — planned)");
    println!("TTS backends: mock (chatterbox, kokoro, piper — planned)");
    Ok(())
}

fn voice_asr_mock(language: &str, sample_rate: u32) -> Result<()> {
    let cfg = voice_local::AsrStreamConfig {
        language: Some(language.to_string()),
        sample_rate_hz: sample_rate,
        wake_words: vec!["Tektra".to_string()],
        wake_word_sensitivity: 0.7,
    };
    let mut asr = voice_local::MockAsr::new(cfg);
    loop {
        if let Some(seg) = asr.poll() {
            println!("{}ms-{}ms: {}", seg.start_ms, seg.end_ms, seg.text);
        } else {
            break;
        }
    }
    Ok(())
}

fn voice_tts_mock(text: &str, sample_rate: u32) -> Result<()> {
    let cfg = voice_local::TtsConfig {
        voice: Some("default".into()),
        sample_rate_hz: sample_rate,
    };
    let mut tts = voice_local::MockTts::new(cfg);
    let pcm = tts.synthesize(text);
    println!(
        "tts synthesized {} samples at {} Hz",
        pcm.len(),
        sample_rate
    );
    Ok(())
}

fn voice_kyutai(
    config: &str,
    hotkey: Option<&str>,
    duration: u32,
    dump_weights: bool,
) -> Result<()> {
    use std::process::Command;
    let candidate = std::path::Path::new("target/debug/kyutai-stt-app");
    let mut args: Vec<String> = Vec::new();
    let (program, use_cargo) = if candidate.exists() {
        (candidate.to_string_lossy().to_string(), false)
    } else {
        ("cargo".to_string(), true)
    };
    if use_cargo {
        args.extend([
            "run".into(),
            "-p".into(),
            "kyutai-stt-app".into(),
            "--".into(),
        ]);
    }
    args.extend(["--config".into(), config.into()]);
    if let Some(hk) = hotkey {
        args.extend(["--hotkey".into(), hk.into()]);
    }
    args.extend(["--duration".into(), duration.to_string()]);
    if dump_weights {
        args.push("--dump-weights".into());
    }
    println!("Launching Kyutai STT app: {} {:?}", program, args);
    let status = Command::new(&program).args(&args).status()?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "kyutai-stt-app exited with status {:?}",
            status.code()
        ));
    }
    Ok(())
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum VoiceBackend {
    Mock,
    WhisperCpp,
    FasterWhisper,
    Vosk,
    KyutaiMoshi,
}

fn voice_asr_capture(backend: VoiceBackend, language: &str, duration: u32) -> Result<()> {
    #[cfg(feature = "voice-local/audio")]
    {
        let (_stream, mic_cfg, rx) = voice_local::mic::start_default_input_i16()
            .map_err(|e| anyhow::anyhow!("mic open failed: {e}"))?;
        let cfg = voice_local::AsrStreamConfig {
            language: Some(language.to_string()),
            sample_rate_hz: mic_cfg.sample_rate_hz,
        };
        let kind = match backend {
            VoiceBackend::Mock => voice_local::plugin::AsrBackendKind::Mock,
            VoiceBackend::WhisperCpp => voice_local::plugin::AsrBackendKind::WhisperCpp,
            VoiceBackend::FasterWhisper => voice_local::plugin::AsrBackendKind::FasterWhisper,
            VoiceBackend::Vosk => voice_local::plugin::AsrBackendKind::Vosk,
            VoiceBackend::KyutaiMoshi => voice_local::plugin::AsrBackendKind::KyutaiMoshi,
        };
        let mut asr = match voice_local::plugin::new_asr_backend(kind, cfg.clone()) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("backend not available: {e}; falling back to mock");
                Box::new(voice_local::MockAsr::new(cfg))
            }
        };
        let start = std::time::Instant::now();
        loop {
            if duration > 0 && start.elapsed().as_secs() >= duration as u64 {
                break;
            }
            match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(chunk) => asr.push_audio(&chunk),
                Err(mpsc_err) => {
                    // timeout or disconnected
                }
            }
            if let Some(seg) = asr.poll() {
                println!("{}ms-{}ms: {}", seg.start_ms, seg.end_ms, seg.text);
            }
        }
        return Ok(());
    }
    #[cfg(not(feature = "voice-local/audio"))]
    {
        println!("audio capture requires --features voice-local/audio");
        Ok(())
    }
}

fn voice_record(duration: u32, out: &str) -> Result<()> {
    #[cfg(feature = "voice-local/audio")]
    {
        let (_stream, mic_cfg, rx) = voice_local::mic::start_default_input_i16()
            .map_err(|e| anyhow::anyhow!("mic open failed: {e}"))?;
        let sample_rate = mic_cfg.sample_rate_hz;
        let channels = mic_cfg.channels as u16;
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(out, spec)?;
        let start = std::time::Instant::now();
        loop {
            if duration > 0 && start.elapsed().as_secs() >= duration as u64 {
                break;
            }
            match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(chunk) => {
                    for s in chunk {
                        w.write_sample(s)?;
                    }
                }
                Err(_) => {}
            }
        }
        w.finalize()?;
        println!("recorded {}s to {} at {} Hz", duration, out, sample_rate);
        return Ok(());
    }
    #[cfg(not(feature = "voice-local/audio"))]
    {
        println!("audio capture requires --features voice-local/audio");
        Ok(())
    }
}
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum DepthMode {
    Sgbm,
    Sgbm3,
}

fn learn_status() -> Result<()> {
    // Initialize continual learning system
    continual_learning::init().map_err(|e| anyhow::anyhow!("Failed to init continual learning: {}", e))?;

    // Create data collector to get stats
    let collector = continual_learning::create_data_collector()
        .map_err(|e| anyhow::anyhow!("Failed to create data collector: {}", e))?;
    let stats = collector.get_stats();

    println!("Learning System Status:");
    println!("  Total samples collected: {}", stats.total_samples);
    println!("  Buffer size: {}", stats.buffer_size);
    println!("  Current file size: {} bytes", stats.current_file_size);

    // Try to create reward model for status
    if let Ok(model) = continual_learning::create_reward_model() {
        if model.is_trained() {
            println!("  Reward model: trained");
            if let Some(train_stats) = model.get_training_stats() {
                println!("  Training epochs: {}", train_stats.epochs_completed);
                println!("  Final loss: {:.4}", train_stats.final_train_loss);
            }
        } else {
            println!("  Reward model: not trained");
        }
    }

    Ok(())
}

fn learn_train(model: &str, dataset: &str, output: &str) -> Result<()> {
    println!("Starting training job:");
    println!("  Model: {}", model);
    println!("  Dataset: {}", dataset);
    println!("  Output: {}", output);

    // Initialize learning system
    continual_learning::init()
        .map_err(|e| anyhow::anyhow!("Failed to init continual learning: {}", e))?;

    // Load dataset (simplified - would need proper dataset loading)
    let mut data_samples = Vec::new();

    // For now, create some dummy samples
    for i in 0..10 {
        let sample = continual_learning::DataSample {
            id: uuid::Uuid::new_v4(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs_f64(),
            observation: vla_policy::Observation::default(),
            action: vla_policy::Action::default(),
            reward: Some(continual_learning::RewardSignal {
                total_reward: (i as f32 * 0.1).sin(),
                components: std::collections::HashMap::new(),
                reward_type: continual_learning::RewardType::Dense,
                is_terminal: false,
                discount_factor: 0.99,
            }),
            is_intervention: false,
            metadata: std::collections::HashMap::new(),
        };
        data_samples.push(sample);
    }

    // Create data collector and record samples
    let mut collector = continual_learning::create_data_collector()
        .map_err(|e| anyhow::anyhow!("Failed to create data collector: {}", e))?;
    for sample in &data_samples {
        collector.record_sample(
            sample.observation.clone(),
            sample.action.clone(),
            sample.reward.clone(),
        ).map_err(|e| anyhow::anyhow!("Failed to record sample: {}", e))?;
    }

    // Train reward model
    let mut reward_model = continual_learning::create_reward_model()
        .map_err(|e| anyhow::anyhow!("Failed to create reward model: {}", e))?;
    reward_model.train(&data_samples)
        .map_err(|e| anyhow::anyhow!("Failed to train reward model: {}", e))?;

    println!("Training completed successfully!");
    println!("  Samples processed: {}", data_samples.len());
    if let Some(stats) = reward_model.get_training_stats() {
        println!("  Final training loss: {:.4}", stats.final_train_loss);
        println!("  Final validation loss: {:.4}", stats.final_val_loss);
    }

    Ok(())
}

fn learn_models() -> Result<()> {
    println!("Available models:");
    println!("  (Model registry integration pending)");
    println!("  Use 'sr learn train' to train new models");
    Ok(())
}

fn learn_intervention(original_action: &str, corrected_action: &str, reason: &str) -> Result<()> {
    println!("Recording intervention:");
    println!("  Reason: {}", reason);

    // Parse actions from JSON (simplified)
    let original: vla_policy::Action = serde_json::from_str(original_action)
        .map_err(|e| anyhow::anyhow!("Invalid original action JSON: {}", e))?;

    let corrected: vla_policy::Action = serde_json::from_str(corrected_action)
        .map_err(|e| anyhow::anyhow!("Invalid corrected action JSON: {}", e))?;

    // Initialize learning system
    continual_learning::init()
        .map_err(|e| anyhow::anyhow!("Failed to init continual learning: {}", e))?;
    let mut collector = continual_learning::create_data_collector()
        .map_err(|e| anyhow::anyhow!("Failed to create data collector: {}", e))?;

    // Create dummy observation for intervention
    let observation = vla_policy::Observation::default();

    // Record intervention
    collector.record_intervention(observation, original, corrected, reason.to_string())
        .map_err(|e| anyhow::anyhow!("Failed to record intervention: {}", e))?;

    println!("Intervention recorded successfully!");
    println!("  This will be used to improve future model predictions");

    Ok(())
}
