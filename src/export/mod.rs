use std::path::Path;

use crate::types::FrameBuffer;

/// Errors that can occur during image export.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum ExportError {
    #[error("PNG encoding failed: {0}")]
    Encode(String),

    #[error("failed to write file: {0}")]
    Io(#[from] std::io::Error),
}

/// Save a captured frame as a PNG file.
#[allow(dead_code)]
pub fn save_png(_frame: &FrameBuffer, _path: &Path) -> Result<(), ExportError> {
    todo!("implemented in Task 3")
}
