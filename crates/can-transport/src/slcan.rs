use crate::{BusInfo, CanBus, CanFilter, CanFrame, CanId, Result, Timestamp, TransportError};
use serialport::{SerialPort, SerialPortType};
use std::io::{Read, Write};
use std::time::Duration;
use time::OffsetDateTime;

/// SLCAN text protocol over serial (common on macOS USB-CAN dongles)
pub struct SlcanBus {
    _port_path: String,
    port: Box<dyn SerialPort>,
}

impl SlcanBus {
    /// Common SLCAN bitrates mapped to `Sx` codes.
    #[allow(dead_code)]
    pub fn open_with(path: &str, bitrate: Option<SlcanBitrate>) -> Result<Self> {
        let mut port = serialport::new(path, 115200)
            .timeout(Duration::from_millis(200))
            .open()
            .map_err(|e| TransportError::Io(e.to_string()))?;
        // Close, set bitrate (if provided) else default S6 (500k), then open
        let _ = Self::write_cmd(&mut *port, b"C\r");
        let code = bitrate.unwrap_or(SlcanBitrate::B500k).code();
        let mut cmd = [0u8; 3];
        cmd[0] = b'S';
        cmd[1] = code;
        cmd[2] = b'\r';
        let _ = Self::write_cmd(&mut *port, &cmd);
        let _ = Self::write_cmd(&mut *port, b"O\r");
        Ok(SlcanBus {
            _port_path: path.to_string(),
            port,
        })
    }

    fn encode_frame(frame: &CanFrame) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(32);
        if frame.rtr {
            return Err(TransportError::Unsupported("RTR not implemented"));
        }
        if frame.id.is_extended() {
            out.push(b'T');
            // 8-hex id
            let s = format!("{:08X}", frame.id.raw());
            out.extend_from_slice(s.as_bytes());
        } else {
            out.push(b't');
            let s = format!("{:03X}", frame.id.raw());
            out.extend_from_slice(s.as_bytes());
        }
        // DLC
        if frame.len > 8 {
            return Err(TransportError::InvalidFrame("dlc > 8"));
        }
        let dlc_char = b'0' + frame.len;
        out.push(dlc_char);
        // Data
        for i in 0..(frame.len as usize) {
            out.extend_from_slice(format!("{:02X}", frame.data[i]).as_bytes());
        }
        // Terminator
        out.push(b'\r');
        Ok(out)
    }

    fn parse_frame(line: &[u8]) -> Result<CanFrame> {
        if line.is_empty() {
            return Err(TransportError::InvalidFrame("empty"));
        }
        let kind = line[0];
        match kind {
            b't' | b'r' => {
                if line.len() < 1 + 3 + 1 {
                    return Err(TransportError::InvalidFrame("short std"));
                }
                let id = u16::from_str_radix(
                    std::str::from_utf8(&line[1..4])
                        .map_err(|_| TransportError::InvalidFrame("utf8"))?,
                    16,
                )
                .map_err(|_| TransportError::InvalidFrame("id"))?;
                let dlc = (line[4] - b'0') as usize;
                let mut data_idx = 5;
                let mut data = [0u8; 8];
                for i in 0..dlc {
                    let end = data_idx + 2;
                    if end > line.len() {
                        return Err(TransportError::InvalidFrame("short data"));
                    }
                    let byte = u8::from_str_radix(
                        std::str::from_utf8(&line[data_idx..end])
                            .map_err(|_| TransportError::InvalidFrame("utf8"))?,
                        16,
                    )
                    .map_err(|_| TransportError::InvalidFrame("byte"))?;
                    if i < 8 {
                        data[i] = byte;
                    }
                    data_idx = end;
                }
                let id = CanId::standard(id).ok_or(TransportError::InvalidFrame("id range"))?;
                let mut frame = CanFrame {
                    id,
                    len: dlc as u8,
                    data,
                    rtr: kind == b'r',
                    timestamp: None,
                };
                frame.timestamp = Some(Timestamp(OffsetDateTime::now_utc()));
                Ok(frame)
            }
            b'T' | b'R' => {
                if line.len() < 1 + 8 + 1 {
                    return Err(TransportError::InvalidFrame("short ext"));
                }
                let id = u32::from_str_radix(
                    std::str::from_utf8(&line[1..9])
                        .map_err(|_| TransportError::InvalidFrame("utf8"))?,
                    16,
                )
                .map_err(|_| TransportError::InvalidFrame("id"))?;
                let dlc = (line[9] - b'0') as usize;
                let mut data_idx = 10;
                let mut data = [0u8; 8];
                for i in 0..dlc {
                    let end = data_idx + 2;
                    if end > line.len() {
                        return Err(TransportError::InvalidFrame("short data"));
                    }
                    let byte = u8::from_str_radix(
                        std::str::from_utf8(&line[data_idx..end])
                            .map_err(|_| TransportError::InvalidFrame("utf8"))?,
                        16,
                    )
                    .map_err(|_| TransportError::InvalidFrame("byte"))?;
                    if i < 8 {
                        data[i] = byte;
                    }
                    data_idx = end;
                }
                let id = CanId::extended(id).ok_or(TransportError::InvalidFrame("id range"))?;
                let mut frame = CanFrame {
                    id,
                    len: dlc as u8,
                    data,
                    rtr: kind == b'R',
                    timestamp: None,
                };
                frame.timestamp = Some(Timestamp(OffsetDateTime::now_utc()));
                Ok(frame)
            }
            _ => Err(TransportError::InvalidFrame("unknown header")),
        }
    }

    fn write_cmd(port: &mut dyn SerialPort, cmd: &[u8]) -> Result<()> {
        port.write_all(cmd)
            .map_err(|e| TransportError::Io(e.to_string()))?;
        Ok(())
    }
}

impl CanBus for SlcanBus {
    fn open(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        // Default to 500k to keep prior behavior
        Self::open_with(path, Some(SlcanBitrate::B500k))
    }

    fn list() -> Result<Vec<BusInfo>> {
        let mut out = Vec::new();
        for p in serialport::available_ports().map_err(|e| TransportError::Io(e.to_string()))? {
            match p.port_type {
                SerialPortType::UsbPort(_u) => {
                    out.push(BusInfo {
                        name: p.port_name,
                        driver: "slcan-serial".to_string(),
                    });
                }
                _ => {
                    // Still include other serial ports; user can pick
                    out.push(BusInfo {
                        name: p.port_name,
                        driver: "serial".to_string(),
                    });
                }
            }
        }
        Ok(out)
    }

    fn set_filters(&mut self, _filters: &[CanFilter]) -> Result<()> {
        let _ = _filters;
        // SLCAN hardware filters are not standardized; ignore
        Err(TransportError::Unsupported("slcan filters not supported"))
    }

    fn recv(&mut self, timeout_ms: Option<u64>) -> Result<CanFrame> {
        if let Some(ms) = timeout_ms {
            self.port.set_timeout(Duration::from_millis(ms)).ok();
        }
        let mut buf = [0u8; 128];
        let mut acc: Vec<u8> = Vec::with_capacity(64);
        loop {
            match self.port.read(&mut buf) {
                Ok(n) if n > 0 => {
                    acc.extend_from_slice(&buf[..n]);
                    if let Some(pos) = acc.iter().position(|&b| b == b'\r') {
                        let line = acc.drain(..=pos).collect::<Vec<u8>>();
                        // Drop terminator
                        let trim = &line[..line.len().saturating_sub(1)];
                        if trim.is_empty() {
                            continue;
                        }
                        return Self::parse_frame(trim);
                    }
                }
                Ok(_) => continue,
                Err(e) => {
                    // Map timeout separately if possible
                    let msg = e.to_string();
                    if msg.contains("Operation timed out") || msg.contains("timed out") {
                        return Err(TransportError::Timeout);
                    }
                    return Err(TransportError::Io(msg));
                }
            }
        }
    }

    fn send(&mut self, frame: &CanFrame) -> Result<()> {
        let line = Self::encode_frame(frame)?;
        self.port
            .write_all(&line)
            .map_err(|e| TransportError::Io(e.to_string()))?;
        Ok(())
    }
}

/// Supported SLCAN bitrates (mapped to Sx codes)
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SlcanBitrate {
    B10k,  // S0
    B20k,  // S1
    B50k,  // S2
    B100k, // S3
    B125k, // S4
    B250k, // S5
    B500k, // S6
    B800k, // S7
    B1M,   // S8
}

impl SlcanBitrate {
    pub fn code(self) -> u8 {
        match self {
            SlcanBitrate::B10k => b'0',
            SlcanBitrate::B20k => b'1',
            SlcanBitrate::B50k => b'2',
            SlcanBitrate::B100k => b'3',
            SlcanBitrate::B125k => b'4',
            SlcanBitrate::B250k => b'5',
            SlcanBitrate::B500k => b'6',
            SlcanBitrate::B800k => b'7',
            SlcanBitrate::B1M => b'8',
        }
    }
}
