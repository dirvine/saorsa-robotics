use crate::{Error, Frame, PixelFormat, Result};

#[cfg(feature = "opencv")]
use opencv::{core, imgcodecs, imgproc, prelude::*};

/// Read an image from disk and return grayscale bytes (row-major), width, height.
#[cfg(feature = "opencv")]
pub fn read_gray8(path: &str) -> Result<(Vec<u8>, usize, usize)> {
    // Read as grayscale directly
    let mat = imgcodecs::imread(path, imgcodecs::IMREAD_GRAYSCALE)
        .map_err(|e| Error::Backend(e.to_string()))?;
    if mat.empty() {
        return Err(Error::Io("failed to read image or empty".to_string()));
    }
    let rows = mat.rows();
    let cols = mat.cols();
    if rows <= 0 || cols <= 0 {
        return Err(Error::Backend("invalid image size".to_string()));
    }
    let size = (rows as usize) * (cols as usize);
    let mut out = vec![0u8; size];
    let src = mat
        .data_bytes()
        .map_err(|e| Error::Backend(e.to_string()))?;
    if src.len() < size {
        return Err(Error::Backend("unexpected buffer size".to_string()));
    }
    out.copy_from_slice(&src[..size]);
    Ok((out, cols as usize, rows as usize))
}

#[cfg(feature = "opencv")]
pub fn write_png(path: &str, frame: &Frame) -> Result<()> {
    match frame.pixel_format {
        PixelFormat::Rgb8 => write_rgb8_png(path, frame),
        PixelFormat::Bgr8 => write_bgr8_png(path, frame),
        PixelFormat::Gray8 => write_gray8_png(path, frame),
    }
}

#[cfg(feature = "opencv")]
fn write_rgb8_png(path: &str, frame: &Frame) -> Result<()> {
    // Build Mat from slice: 1 x (w*h*3) then reshape to (h rows, 3 channels)
    let mat = core::Mat::from_slice(&frame.data).map_err(|e| Error::Backend(e.to_string()))?;
    let rgb = mat
        .reshape(3, frame.height as i32)
        .map_err(|e| Error::Backend(e.to_string()))?;
    let mut bgr = core::Mat::default();
    imgproc::cvt_color(&rgb, &mut bgr, imgproc::COLOR_RGB2BGR, 0)
        .map_err(|e| Error::Backend(e.to_string()))?;
    imgcodecs::imwrite(path, &bgr, &opencv::types::VectorOfi32::new())
        .map_err(|e| Error::Backend(e.to_string()))?;
    Ok(())
}

#[cfg(feature = "opencv")]
fn write_bgr8_png(path: &str, frame: &Frame) -> Result<()> {
    let mat = core::Mat::from_slice(&frame.data).map_err(|e| Error::Backend(e.to_string()))?;
    let bgr = mat
        .reshape(3, frame.height as i32)
        .map_err(|e| Error::Backend(e.to_string()))?;
    imgcodecs::imwrite(path, &bgr, &opencv::types::VectorOfi32::new())
        .map_err(|e| Error::Backend(e.to_string()))?;
    Ok(())
}

#[cfg(feature = "opencv")]
fn write_gray8_png(path: &str, frame: &Frame) -> Result<()> {
    let mat = core::Mat::from_slice(&frame.data).map_err(|e| Error::Backend(e.to_string()))?;
    let gray = mat
        .reshape(1, frame.height as i32)
        .map_err(|e| Error::Backend(e.to_string()))?;
    imgcodecs::imwrite(path, &gray, &opencv::types::VectorOfi32::new())
        .map_err(|e| Error::Backend(e.to_string()))?;
    Ok(())
}
