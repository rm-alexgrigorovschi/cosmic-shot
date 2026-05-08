# M1: wlr-screencopy Capture + Minimal iced Window — Design Spec

## Goal

Capture a screenshot of the first available Wayland output via `wlr-screencopy-unstable-v1`, display it in a fullscreen iced window, and write a PNG copy to the current directory for verification.

## Architecture

Two sequential phases, no async runtime:

1. **Capture phase** — Connect to the Wayland display using `wayland-client`. Enumerate globals, find `zwlr_screencopy_manager_v1` (fail loudly with a clear error message if the protocol is not advertised). Allocate a `wl_shm` buffer, request a frame copy of the first available output (no output selection in M1), run the Wayland event loop until `Ready` or `Failed`. Return a `FrameBuffer` struct containing pixel data, dimensions, and format.

2. **Display phase** — Hand the `FrameBuffer` to an iced application. Open a fullscreen/borderless window showing the captured image as a static texture. The window closes on Escape or window close event.

3. **PNG dump** — After capture, before opening the window, write `./capture.png` using the `image` crate. This is a verification side effect, not part of the core pipeline.

## Data Flow

```
main.rs
  → capture::capture_output() -> Result<FrameBuffer>
  → export::save_png(&frame, "./capture.png") -> Result<()>
  → overlay::run(frame) -> Result<()>  (iced app)
```

All three calls are sequential. No concurrency. No shared mutable state.

## Key Types

### `FrameBuffer`

```rust
pub struct FrameBuffer {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
}
```

### `PixelFormat`

```rust
pub enum PixelFormat {
    Argb8888,
    Xrgb8888,
}
```

These are the formats COSMIC's compositor (`cosmic-comp`) will typically provide via `wlr-screencopy`. We handle both; other formats produce a clear error.

### Error Types

- `CaptureError` — covers Wayland connection failures, missing protocol, buffer allocation errors, frame copy failures. Uses `thiserror`. Includes protocol context per CLAUDE.md.
- `ExportError` — covers PNG encoding failures, file I/O errors. Uses `thiserror`.
- `main.rs` uses `anyhow` to wrap both.

## Module Boundaries

### `capture/` — Wayland protocol code

- **Knows about:** `wayland-client`, `wayland-protocols-wlr`, `smithay-client-toolkit`, `wl_shm`
- **Knows nothing about:** iced, image encoding, file I/O
- **Public API:** `capture_output() -> Result<FrameBuffer, CaptureError>`
- **Internal modules:**
  - `screencopy.rs` — `wlr-screencopy-unstable-v1` protocol logic (isolated for future swap to `ext-image-copy-capture-v1`)
  - `shm.rs` — `wl_shm` buffer allocation and management

### `export/` — Encoding and disk I/O

- **Knows about:** `image` crate, file system
- **Knows nothing about:** Wayland protocols, iced
- **Public API:** `save_png(frame: &FrameBuffer, path: &Path) -> Result<(), ExportError>`
- **Internal modules:**
  - `encode.rs` — pixel format conversion (BGRx → RGBA) and PNG encoding

### `overlay/` — iced UI

- **Knows about:** `iced`
- **Knows nothing about:** Wayland protocols, file I/O
- **Public API:** `run(frame: FrameBuffer) -> Result<()>`
- **Receives a frame buffer, displays it. That's it for M1.**

## Dependencies (M1 only)

```toml
[dependencies]
anyhow = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
wayland-client = "0.31"
wayland-protocols-wlr = { version = "0.3", features = ["client"] }
smithay-client-toolkit = "0.19"
image = { version = "0.25", default-features = false, features = ["png"] }
iced = "0.13"
```

## Pixel Format Conversion

COSMIC's compositor provides frames in `Xrgb8888` or `Argb8888` (little-endian byte order). The byte layout in memory is `[B, G, R, X/A]` per pixel.

- For PNG export: convert to RGBA by swapping B↔R channels and setting alpha to 255 for Xrgb.
- For iced display: convert to RGBA for use with `iced::widget::image`.

The conversion function lives in `export/encode.rs` but is also used by `overlay/`. We'll put the shared `FrameBuffer` type and conversion in a top-level `types.rs` or keep conversion as a method on `FrameBuffer` itself.

**Decision:** `FrameBuffer` lives in `src/types.rs` with a `to_rgba(&self) -> Vec<u8>` method. Both `export/` and `overlay/` depend on this shared type. This is acceptable because `FrameBuffer` is a pure data type with no protocol or UI knowledge.

## Error Handling Strategy

- `capture/` returns `CaptureError` with variants for each failure mode
- `export/` returns `ExportError` with variants for encoding and I/O failures
- `overlay/` returns `anyhow::Error` (iced errors are opaque)
- `main.rs` wraps everything in `anyhow`, logs via `tracing`, and exits with a nonzero code on failure

## What M1 Does NOT Include

- No Tokio async runtime
- No config file (`~/.config/cosmic-shot/config.toml` deferred to M4)
- No clipboard support (M4)
- No selection rectangle or crop (M3)
- No global shortcut integration (M5)
- No proper output path logic (hardcoded `./capture.png`)
- No CLI argument parsing beyond the basics

## Runtime Requirements

- Wayland session (fails with clear error if `$WAYLAND_DISPLAY` is not set)
- Compositor must advertise `zwlr_screencopy_manager_v1` (fails loudly if missing)
- At least one active output

## Success Criteria

1. `cargo build --release` succeeds
2. `cargo clippy -- -D warnings` passes
3. Running `cosmic-shot` on COSMIC DE captures the screen and displays it in a window
4. `./capture.png` is a valid PNG matching the screen content
5. Pressing Escape closes the window cleanly
