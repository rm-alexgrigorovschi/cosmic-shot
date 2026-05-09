/// A cropped region of a captured frame, in RGBA8 format.
pub struct CroppedImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Placeholder — full implementation arrives in Task 3.
pub fn crop_selection(_placeholder: ()) -> Option<CroppedImage> {
    None
}
