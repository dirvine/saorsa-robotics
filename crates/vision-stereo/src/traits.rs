use crate::{Frame, Result};

pub trait CameraSource {
    /// Open a camera source by device index or path string.
    fn open(spec: &str) -> Result<Self>
    where
        Self: Sized;

    /// Read a single frame.
    fn read(&mut self) -> Result<Frame>;
}
