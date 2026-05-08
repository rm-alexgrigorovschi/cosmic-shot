/// Pixel format of a captured frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// ARGB8888 — bytes in memory: [B, G, R, A] (little-endian).
    Argb8888,
    /// XRGB8888 — bytes in memory: [B, G, R, X] (little-endian), alpha ignored.
    Xrgb8888,
    /// ABGR8888 — bytes in memory: [R, G, B, A] (little-endian).
    Abgr8888,
    /// XBGR8888 — bytes in memory: [R, G, B, X] (little-endian), alpha ignored.
    Xbgr8888,
}

/// Raw pixel data from a screen capture.
#[derive(Debug, Clone)]
pub struct FrameBuffer {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
}

impl FrameBuffer {
    /// Convert the raw pixel data to RGBA8 byte order.
    ///
    /// Handles both ARGB/XRGB ([B,G,R,A/X] in memory) and ABGR/XBGR ([R,G,B,A/X] in memory).
    /// Output is always [R, G, B, A] per pixel (standard RGBA for image/iced).
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut rgba = Vec::with_capacity((self.width * self.height * 4) as usize);
        for y in 0..self.height {
            let row_start = (y * self.stride) as usize;
            for x in 0..self.width {
                let offset = row_start + (x * 4) as usize;
                let (r, g, b, a) = match self.format {
                    // [B, G, R, A] in memory → swap B and R
                    PixelFormat::Argb8888 => (
                        self.data[offset + 2],
                        self.data[offset + 1],
                        self.data[offset],
                        self.data[offset + 3],
                    ),
                    // [B, G, R, X] in memory → swap B and R, alpha = 255
                    PixelFormat::Xrgb8888 => (
                        self.data[offset + 2],
                        self.data[offset + 1],
                        self.data[offset],
                        255,
                    ),
                    // [R, G, B, A] in memory → already RGBA
                    PixelFormat::Abgr8888 => (
                        self.data[offset],
                        self.data[offset + 1],
                        self.data[offset + 2],
                        self.data[offset + 3],
                    ),
                    // [R, G, B, X] in memory → already RGB, alpha = 255
                    PixelFormat::Xbgr8888 => (
                        self.data[offset],
                        self.data[offset + 1],
                        self.data[offset + 2],
                        255,
                    ),
                };
                rgba.extend_from_slice(&[r, g, b, a]);
            }
        }
        rgba
    }
}
