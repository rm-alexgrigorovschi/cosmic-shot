# M1: wlr-screencopy Capture + Minimal iced Window — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Capture the first available Wayland output via `wlr-screencopy-unstable-v1`, save to `./capture.png`, and display in a minimal iced window.

**Architecture:** Sequential two-phase pipeline (capture → display) with no async runtime. Capture uses `wayland-client` + `smithay-client-toolkit` for shm/output + raw `wayland-protocols-wlr` for screencopy. Display uses iced 0.13. PNG export via `image` crate.

**Tech Stack:** Rust 2021, wayland-client 0.31, smithay-client-toolkit 0.19, wayland-protocols-wlr 0.3, iced 0.13, image 0.25, thiserror 2, anyhow 1, tracing 0.1

---

## File Structure

```
src/
├── main.rs              # Binary entry: tracing init, capture → export → overlay
├── types.rs             # FrameBuffer, PixelFormat, to_rgba() conversion
├── capture/
│   ├── mod.rs           # Public API capture_output(), CaptureError, AppState struct
│   └── screencopy.rs    # Dispatch impls for ZwlrScreencopyFrameV1 + WlBuffer
├── export/
│   └── mod.rs           # save_png(), ExportError, pixel format conversion
└── overlay/
    └── mod.rs           # iced App, run(frame), Escape-to-close
```

---

## Task 1: Scaffold project with Cargo.toml and module structure

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/types.rs`
- Create: `src/capture/mod.rs`
- Create: `src/capture/screencopy.rs`
- Create: `src/export/mod.rs`
- Create: `src/overlay/mod.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "cosmic-shot"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
description = "Fast native screenshot tool for COSMIC DE"
license = "MIT"

[dependencies]
anyhow = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
wayland-client = "0.31"
wayland-protocols-wlr = { version = "0.3", features = ["client"] }
smithay-client-toolkit = "0.19"
image = { version = "0.25", default-features = false, features = ["png"] }
iced = { version = "0.13", features = ["image"] }
```

- [ ] **Step 2: Create src/types.rs with FrameBuffer and PixelFormat**

```rust
/// Pixel format of a captured frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// ARGB8888 — bytes in memory: [B, G, R, A] (little-endian).
    Argb8888,
    /// XRGB8888 — bytes in memory: [B, G, R, X] (little-endian), alpha ignored.
    Xrgb8888,
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
    /// Input is [B, G, R, A/X] per pixel (Wayland little-endian convention).
    /// Output is [R, G, B, A] per pixel (standard RGBA for image/iced).
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut rgba = Vec::with_capacity((self.width * self.height * 4) as usize);
        for y in 0..self.height {
            let row_start = (y * self.stride) as usize;
            for x in 0..self.width {
                let offset = row_start + (x * 4) as usize;
                let b = self.data[offset];
                let g = self.data[offset + 1];
                let r = self.data[offset + 2];
                let a = match self.format {
                    PixelFormat::Argb8888 => self.data[offset + 3],
                    PixelFormat::Xrgb8888 => 255,
                };
                rgba.extend_from_slice(&[r, g, b, a]);
            }
        }
        rgba
    }
}
```

- [ ] **Step 3: Create src/capture/mod.rs with error type and empty public API**

```rust
mod screencopy;

use crate::types::FrameBuffer;

/// Errors that can occur during screen capture.
#[derive(Debug, thiserror::Error)]
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
pub fn capture_output() -> Result<FrameBuffer, CaptureError> {
    todo!("implemented in Task 2")
}
```

- [ ] **Step 4: Create src/capture/screencopy.rs as empty placeholder**

```rust
// Wayland Dispatch implementations for wlr-screencopy-unstable-v1.
// Isolated in this module for future swap to ext-image-copy-capture-v1.
```

- [ ] **Step 5: Create src/export/mod.rs with error type and empty public API**

```rust
use std::path::Path;

use crate::types::FrameBuffer;

/// Errors that can occur during image export.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("PNG encoding failed: {0}")]
    Encode(String),

    #[error("failed to write file: {0}")]
    Io(#[from] std::io::Error),
}

/// Save a captured frame as a PNG file.
pub fn save_png(_frame: &FrameBuffer, _path: &Path) -> Result<(), ExportError> {
    todo!("implemented in Task 3")
}
```

- [ ] **Step 6: Create src/overlay/mod.rs as empty placeholder**

```rust
use crate::types::FrameBuffer;

/// Display the captured frame in a window. Closes on Escape.
pub fn run(_frame: FrameBuffer) -> anyhow::Result<()> {
    todo!("implemented in Task 4")
}
```

- [ ] **Step 7: Create src/main.rs**

```rust
mod capture;
mod export;
mod overlay;
mod types;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("cosmic-shot starting");

    // Pipeline will be wired in Task 5
    Ok(())
}
```

- [ ] **Step 8: Verify build and clippy**

Run: `cargo build 2>&1` (from project root)
Expected: Build succeeds (with dead_code warnings, which is fine for scaffolding)

Run: `cargo clippy -- -D warnings 2>&1`
Expected: May warn about unused imports/dead code — fix any clippy errors, allow dead_code temporarily with `#[allow(dead_code)]` on the stub functions only.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "feat: scaffold project with Cargo.toml and module structure"
```

---

## Task 2: Capture module — Wayland connection and wlr-screencopy frame grab

**Files:**
- Modify: `src/capture/mod.rs`
- Modify: `src/capture/screencopy.rs`

- [ ] **Step 1: Implement screencopy.rs with Dispatch impls**

Replace `src/capture/screencopy.rs` with:

```rust
//! Wayland Dispatch implementations for wlr-screencopy-unstable-v1.
//! Isolated in this module for future swap to ext-image-copy-capture-v1.

use wayland_client::protocol::{wl_buffer, wl_output};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use super::CaptureState;

/// Tracks the state of a single screencopy frame capture.
#[derive(Debug, Default)]
pub(crate) struct FrameState {
    /// Preferred shm format from the compositor.
    pub format: Option<wayland_client::protocol::wl_shm::Format>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    /// All buffer formats have been advertised.
    pub buffer_done: bool,
    /// Frame pixel data is ready to read.
    pub ready: bool,
    /// Capture failed.
    pub failed: bool,
}

impl Dispatch<ZwlrScreencopyManagerV1, ()> for CaptureState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrScreencopyManagerV1,
        _event: <ZwlrScreencopyManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Manager has no events.
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for CaptureState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use wayland_client::protocol::wl_shm::Format;

        match event {
            zwlr_screencopy_frame_v1::Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                // Accept only formats we can handle. Prefer the first supported one.
                if state.frame.format.is_none() {
                    if let WEnum::Value(fmt) = format {
                        if fmt == Format::Argb8888 || fmt == Format::Xrgb8888 {
                            state.frame.format = Some(fmt);
                            state.frame.width = width;
                            state.frame.height = height;
                            state.frame.stride = stride;
                        }
                    }
                }
            }
            zwlr_screencopy_frame_v1::Event::BufferDone => {
                state.frame.buffer_done = true;
            }
            zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                state.frame.ready = true;
            }
            zwlr_screencopy_frame_v1::Event::Failed => {
                state.frame.failed = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for CaptureState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_buffer::WlBuffer,
        _event: wl_buffer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // wl_buffer::Release — safe to ignore for single-shot capture.
    }
}

impl Dispatch<wl_output::WlOutput, ()> for CaptureState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_output::WlOutput,
        _event: wl_output::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // We only need the output proxy, not its events.
    }
}
```

- [ ] **Step 2: Implement capture/mod.rs with capture_output()**

Replace `src/capture/mod.rs` with:

```rust
mod screencopy;

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_output, wl_registry, wl_shm};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use smithay_client_toolkit::shm::raw::RawPool;
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::delegate_shm;

use crate::types::{FrameBuffer, PixelFormat};

use self::screencopy::FrameState;

/// Errors that can occur during screen capture.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("failed to connect to Wayland display: {0}")]
    Connection(String),

    #[error("Wayland global error: {0}")]
    Global(#[from] wayland_client::globals::GlobalError),

    #[error("compositor does not support wlr-screencopy-unstable-v1")]
    NoScreencopy,

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

    #[error("Wayland backend error: {0}")]
    Backend(#[from] wayland_client::backend::WaylandError),

    #[error("SCTK global error: {0}")]
    SctGlobal(#[from] smithay_client_toolkit::globals::GlobalError),
}

/// Internal state for the Wayland capture session.
pub(crate) struct CaptureState {
    pub shm: Shm,
    pub frame: FrameState,
}

impl ShmHandler for CaptureState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_shm!(CaptureState);

// Handle wl_registry events for globals we bind manually (wl_output, screencopy).
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for CaptureState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // SCTK's delegate_shm handles shm globals. We handle other globals
        // via GlobalList::bind() below, so no event processing needed here.
    }
}

/// Capture the first available output and return its pixel data.
pub fn capture_output() -> Result<FrameBuffer, CaptureError> {
    // 1. Connect to Wayland display.
    let conn = Connection::connect_to_env()
        .map_err(|e| CaptureError::Connection(e.to_string()))?;
    let (globals, mut event_queue) = registry_queue_init::<CaptureState>(&conn)?;
    let qh = event_queue.handle();

    // 2. Bind required globals.
    let shm = Shm::bind(&globals, &qh)?;

    let screencopy_manager: ZwlrScreencopyManagerV1 = globals
        .bind(&qh, 1..=3, ())
        .map_err(|_| CaptureError::NoScreencopy)?;

    // Bind the first wl_output.
    let output: wl_output::WlOutput = globals
        .bind(&qh, 1..=4, ())
        .map_err(|_| CaptureError::NoOutput)?;

    let mut state = CaptureState {
        shm,
        frame: FrameState::default(),
    };

    // 3. Request a frame capture.
    let frame_proxy = screencopy_manager.capture_output(0, &output, &qh, ());

    // 4. Dispatch until the compositor tells us what buffer format it wants.
    while !state.frame.buffer_done && !state.frame.failed {
        event_queue.blocking_dispatch(&mut state)?;
    }

    if state.frame.failed {
        return Err(CaptureError::FrameFailed);
    }

    let format = state.frame.format.ok_or(CaptureError::UnsupportedFormat)?;
    let width = state.frame.width;
    let height = state.frame.height;
    let stride = state.frame.stride;
    let pool_size = (stride * height) as usize;

    // 5. Create an shm buffer and tell the compositor to copy into it.
    let mut pool = RawPool::new(pool_size, &state.shm)
        .map_err(|e| CaptureError::ShmPool(e.to_string()))?;
    let wl_buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        stride as i32,
        format,
        (),
        &qh,
    );

    frame_proxy.copy(&wl_buffer);

    // 6. Wait for the frame to be ready.
    while !state.frame.ready && !state.frame.failed {
        event_queue.blocking_dispatch(&mut state)?;
    }

    if state.frame.failed {
        return Err(CaptureError::FrameFailed);
    }

    // 7. Read pixel data from the shared memory pool.
    let data = pool.mmap()[..pool_size].to_vec();

    let pixel_format = match format {
        wl_shm::Format::Argb8888 => PixelFormat::Argb8888,
        wl_shm::Format::Xrgb8888 => PixelFormat::Xrgb8888,
        // INVARIANT: We only accept Argb8888/Xrgb8888 in the Dispatch impl.
        _ => unreachable!(),
    };

    // 8. Clean up Wayland objects.
    wl_buffer.destroy();
    frame_proxy.destroy();
    screencopy_manager.destroy();
    output.release();

    tracing::info!(width, height, ?pixel_format, "frame captured");

    Ok(FrameBuffer {
        data,
        width,
        height,
        stride,
        format: pixel_format,
    })
}
```

- [ ] **Step 3: Verify build**

Run: `cargo build 2>&1`
Expected: Builds successfully. There will be dead_code warnings for `export` and `overlay` stubs — that's fine.

Run: `cargo clippy -- -D warnings 2>&1`
Expected: Passes (fix any issues before proceeding).

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(capture): Wayland connection and wlr-screencopy frame grab"
```

---

## Task 3: Export module — PNG encoding with pixel format conversion

**Files:**
- Modify: `src/export/mod.rs`

- [ ] **Step 1: Implement export/mod.rs**

Replace `src/export/mod.rs` with:

```rust
use std::path::Path;

use image::{ImageBuffer, Rgba};

use crate::types::FrameBuffer;

/// Errors that can occur during image export.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("PNG encoding failed: {0}")]
    Encode(String),

    #[error("failed to write file: {0}")]
    Io(#[from] std::io::Error),
}

/// Save a captured frame as a PNG file.
pub fn save_png(frame: &FrameBuffer, path: &Path) -> Result<(), ExportError> {
    let rgba = frame.to_rgba();

    let img: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(frame.width, frame.height, rgba)
            .ok_or_else(|| ExportError::Encode("pixel buffer size mismatch".to_string()))?;

    img.save(path).map_err(|e| ExportError::Encode(e.to_string()))?;

    tracing::info!(path = %path.display(), "saved PNG");
    Ok(())
}
```

- [ ] **Step 2: Verify build and clippy**

Run: `cargo build 2>&1`
Expected: Builds successfully.

Run: `cargo clippy -- -D warnings 2>&1`
Expected: Passes.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(export): PNG encoding and file write"
```

---

## Task 4: Overlay module — minimal iced window displaying the captured frame

**Files:**
- Modify: `src/overlay/mod.rs`

- [ ] **Step 1: Implement overlay/mod.rs**

Replace `src/overlay/mod.rs` with:

```rust
use iced::widget::{container, image};
use iced::{keyboard, window, Element, Length, Task as IcedTask, Theme};

use crate::types::FrameBuffer;

struct App {
    image_handle: image::Handle,
}

#[derive(Debug, Clone)]
enum Message {
    Close,
}

/// Display the captured frame in a window. Closes on Escape.
pub fn run(frame: FrameBuffer) -> anyhow::Result<()> {
    let width = frame.width as f32;
    let height = frame.height as f32;
    let rgba = frame.to_rgba();

    iced::application("cosmic-shot", App::update, App::view)
        .subscription(App::subscription)
        .window(window::Settings {
            size: iced::Size::new(width, height),
            decorations: false,
            ..Default::default()
        })
        .theme(|_| Theme::Dark)
        .run_with(move || {
            let handle = image::Handle::from_rgba(frame.width, frame.height, rgba);
            (App { image_handle: handle }, IcedTask::none())
        })
        .map_err(|e| anyhow::anyhow!("iced error: {e}"))?;

    Ok(())
}

impl App {
    fn update(&mut self, message: Message) -> IcedTask<Message> {
        match message {
            Message::Close => window::close(window::Id::MAIN),
        }
    }

    fn view(&self) -> Element<Message> {
        container(
            image::Image::new(self.image_handle.clone())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        keyboard::on_key_press(|key, _modifiers| match key {
            keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::Close),
            _ => None,
        })
    }
}
```

- [ ] **Step 2: Verify build and clippy**

Run: `cargo build 2>&1`
Expected: Builds successfully.

Run: `cargo clippy -- -D warnings 2>&1`
Expected: Passes.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(overlay): minimal iced window to display captured frame"
```

---

## Task 5: Wire pipeline in main.rs and verify end-to-end

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Wire main.rs**

Replace `src/main.rs` with:

```rust
mod capture;
mod export;
mod overlay;
mod types;

use std::path::Path;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("cosmic-shot starting");

    // Phase 1: Capture
    let frame = capture::capture_output()?;
    tracing::info!(
        width = frame.width,
        height = frame.height,
        "capture complete"
    );

    // Phase 2: Export (verification side effect)
    let output_path = Path::new("capture.png");
    export::save_png(&frame, output_path)?;

    // Phase 3: Display
    overlay::run(frame)?;

    Ok(())
}
```

- [ ] **Step 2: Verify build and clippy pass**

Run: `cargo clippy -- -D warnings 2>&1`
Expected: Passes with no warnings.

Run: `cargo build --release 2>&1`
Expected: Builds successfully.

- [ ] **Step 3: Run end-to-end on COSMIC**

Run: `cargo run --release 2>&1`
Expected:
1. Console shows "cosmic-shot starting", "frame captured", "saved PNG" log lines
2. `./capture.png` exists and shows the screen content
3. An iced window opens showing the captured frame
4. Pressing Escape closes the window
5. Program exits cleanly with code 0

- [ ] **Step 4: Verify the PNG**

Run: `file capture.png`
Expected: `capture.png: PNG image data, <width> x <height>, 8-bit/color RGBA, non-interlaced`

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: wire capture-export-overlay pipeline end-to-end"
```
