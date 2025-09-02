use crate::{CameraSource, Error, Frame, PixelFormat, Result};
use opencv::prelude::*;
use opencv::{core, imgproc, videoio};
use time::OffsetDateTime;

pub struct OpenCvCamera {
    cap: videoio::VideoCapture,
}

impl CameraSource for OpenCvCamera {
    fn open(spec: &str) -> Result<Self> {
        // Parse spec as index if numeric, else try to open as path
        let mut cap = if let Ok(idx) = spec.parse::<i32>() {
            videoio::VideoCapture::new(idx, videoio::CAP_ANY)
                .map_err(|e| Error::Backend(e.to_string()))?
        } else {
            videoio::VideoCapture::from_file(spec, videoio::CAP_ANY)
                .map_err(|e| Error::Backend(e.to_string()))?
        };
        let opened =
            videoio::VideoCapture::is_opened(&cap).map_err(|e| Error::Backend(e.to_string()))?;
        if !opened {
            return Err(Error::NotFound(spec.to_string()));
        }
        Ok(Self { cap })
    }

    fn read(&mut self) -> Result<Frame> {
        let mut mat = core::Mat::default();
        self.cap
            .read(&mut mat)
            .map_err(|e| Error::Backend(e.to_string()))?;
        if mat.empty() {
            return Err(Error::Io("empty frame".into()));
        }

        let width = mat.cols() as u32;
        let height = mat.rows() as u32;

        // Convert to RGB8
        let mut rgb = core::Mat::default();
        imgproc::cvt_color(&mat, &mut rgb, imgproc::COLOR_BGR2RGB, 0)
            .map_err(|e| Error::Backend(e.to_string()))?;

        let data = rgb
            .data_bytes()
            .map_err(|e| Error::Backend(e.to_string()))?
            .to_vec();
        Ok(Frame {
            width,
            height,
            pixel_format: PixelFormat::Rgb8,
            data,
            ts: Some(OffsetDateTime::now_utc()),
        })
    }
}
