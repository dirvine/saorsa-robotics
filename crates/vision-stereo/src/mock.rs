use crate::{CameraSource, Frame, PixelFormat, Result};
use time::OffsetDateTime;

pub struct MockCamera {
    counter: u64,
}

impl CameraSource for MockCamera {
    fn open(_spec: &str) -> Result<Self> {
        Ok(Self { counter: 0 })
    }

    fn read(&mut self) -> Result<Frame> {
        self.counter += 1;
        // Produce a simple gray ramp image
        let width = 320u32;
        let height = 240u32;
        let mut data = vec![0u8; (width * height) as usize];
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                data[idx] = ((x + y) % 256) as u8;
            }
        }
        Ok(Frame {
            width,
            height,
            pixel_format: PixelFormat::Gray8,
            data,
            ts: Some(OffsetDateTime::now_utc()),
        })
    }
}
