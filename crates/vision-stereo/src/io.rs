use crate::{Error, Frame, PixelFormat, Result};

#[cfg(feature = "opencv")]
use opencv::{core, imgcodecs, imgproc, prelude::*};

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
