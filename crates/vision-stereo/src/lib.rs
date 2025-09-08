//! vision-stereo: camera abstraction and optional OpenCV backend

mod types;
pub use types::{Frame, PixelFormat};

mod error;
pub use error::{Error, Result};

mod traits;
pub use traits::CameraSource;

#[cfg(feature = "mock")]
mod mock;
#[cfg(feature = "mock")]
pub use mock::MockCamera;

#[cfg(feature = "opencv")]
mod opencv_backend;
#[cfg(feature = "opencv")]
pub use opencv_backend::OpenCvCamera;

#[cfg(feature = "opencv")]
pub mod calib;

#[cfg(feature = "opencv")]
pub mod io;

#[cfg(feature = "opencv")]
pub mod depth;

pub mod tags;

/// Grasp pose helpers (ROI- and tag-based)
pub mod grasp;
