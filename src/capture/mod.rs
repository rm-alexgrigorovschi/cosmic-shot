mod screencopy;

use crate::types::FrameBuffer;

/// Errors that can occur during screen capture.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum CaptureError {
    #[error("failed to connect to Wayland display: {0}")]
    Connection(String),

    #[error("Wayland global error: {0}")]
    Global(#[from] wayland_client::globals::GlobalError),

    #[error("compositor does not support wlr-screencopy-unstable-v1")]
    NoScreencopy,

    #[error("no wl_shm support from compositor")]
    NoShm(String),

    #[error("no active output found")]
    NoOutput,

    #[error("compositor offered no supported pixel format (need Argb8888 or Xrgb8888)")]
    UnsupportedFormat,

    #[error("screencopy frame capture failed")]
    FrameFailed,

    #[error("Wayland dispatch error: {0}")]
    Dispatch(#[from] wayland_client::DispatchError),

    #[error("shm pool creation failed: {0}")]
    ShmPool(String),
}

/// Capture the first available output and return its pixel data.
#[allow(dead_code)]
pub fn capture_output() -> Result<FrameBuffer, CaptureError> {
    todo!("implemented in Task 2")
}
