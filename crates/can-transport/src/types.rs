use core::fmt;
use time::OffsetDateTime;

/// 11-bit or 29-bit CAN identifier
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct CanId {
    raw: u32,
    extended: bool,
}

impl CanId {
    pub fn standard(id11: u16) -> Option<Self> {
        if id11 <= 0x7FF {
            Some(Self {
                raw: id11 as u32,
                extended: false,
            })
        } else {
            None
        }
    }

    pub fn extended(id29: u32) -> Option<Self> {
        if id29 <= 0x1FFF_FFFF {
            Some(Self {
                raw: id29,
                extended: true,
            })
        } else {
            None
        }
    }

    pub fn raw(&self) -> u32 {
        self.raw
    }
    pub fn is_extended(&self) -> bool {
        self.extended
    }
}

impl fmt::Display for CanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.extended {
            write!(f, "0x{raw:08X}", raw = self.raw)
        } else {
            write!(f, "0x{raw:03X}", raw = self.raw)
        }
    }
}

/// A CAN data frame (no CAN FD features yet)
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanFrame {
    pub id: CanId,
    pub len: u8,
    pub data: [u8; 8],
    pub rtr: bool,
    pub timestamp: Option<Timestamp>,
}

impl CanFrame {
    pub fn new(id: CanId, data: &[u8]) -> Option<Self> {
        if data.len() > 8 {
            return None;
        }
        let mut buf = [0u8; 8];
        let len = data.len() as u8;
        // Copy without panic on length
        for (i, b) in data.iter().enumerate() {
            if i >= 8 {
                break;
            }
            buf[i] = *b;
        }
        Some(Self {
            id,
            len,
            data: buf,
            rtr: false,
            timestamp: None,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanFilter {
    pub id: CanId,
    pub mask: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timestamp(pub OffsetDateTime);

#[derive(Clone, Debug)]
pub struct BusInfo {
    pub name: String,
    pub driver: String,
}
