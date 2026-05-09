# M4 Clipboard + File Save — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the toolbar Copy/Save buttons to real export actions — crop the selection, copy to clipboard or save to disk as PNG, then exit the overlay.

**Architecture:** Three new modules: `config.rs` (load/save-dir config), `export/crop.rs` (crop selection from frame), and clipboard support in `export/mod.rs`. The overlay stores raw `FrameBuffer`s alongside `image::Handle`s so it can crop on export. Toolbar clicks detected via hit-testing in `update()` and emit `CopyRequested`/`SaveRequested` messages.

**Tech Stack:** `arboard` (clipboard), `serde` + `toml` (config), `dirs` (home dir), `chrono` (timestamps)

---

### Task 1: Add new dependencies to Cargo.toml

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add dependencies**

Add these lines to the `[dependencies]` section of `Cargo.toml`:

```toml
arboard = "3"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
dirs = "6"
chrono = { version = "0.4", default-features = false, features = ["clock"] }
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add arboard, serde, toml, dirs, chrono for M4"
```

---

### Task 2: Config module — TDD

**Files:**
- Create: `src/config.rs`
- Modify: `src/lib.rs` — add `pub mod config;`

- [ ] **Step 1: Add module declaration**

In `src/lib.rs`, add after the existing `pub mod` lines:

```rust
pub mod config;
```

- [ ] **Step 2: Create `src/config.rs` with tests first**

```rust
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Application configuration.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Directory where screenshots are saved.
    pub save_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_dir: "~/Pictures/cosmic-shot".to_string(),
        }
    }
}

impl Config {
    /// Load config from `~/.config/cosmic-shot/config.toml`.
    ///
    /// Returns defaults if the file does not exist or cannot be parsed.
    pub fn load() -> Self {
        let Some(config_dir) = dirs::config_dir() else {
            tracing::warn!("could not determine config directory, using defaults");
            return Self::default();
        };
        let path = config_dir.join("cosmic-shot").join("config.toml");
        Self::load_from(&path)
    }

    /// Load config from a specific path (for testing).
    pub fn load_from(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => config,
                Err(e) => {
                    tracing::warn!(%e, "failed to parse config, using defaults");
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    /// Resolve `save_dir` by expanding `~` to the user's home directory.
    pub fn resolved_save_dir(&self) -> PathBuf {
        expand_tilde(&self.save_dir)
    }
}

/// Expand a leading `~` or `~/` to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" || path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]); // skip "~/" or just "~"
        }
    }
    PathBuf::from(path)
}

/// Generate a timestamped screenshot filename.
///
/// Format: `screenshot-YYYY-MM-DD_HH-MM-SS.png`
pub fn screenshot_filename() -> String {
    let now = chrono::Local::now();
    now.format("screenshot-%Y-%m-%d_%H-%M-%S.png").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_save_dir() {
        let config = Config::default();
        assert_eq!(config.save_dir, "~/Pictures/cosmic-shot");
    }

    #[test]
    fn config_tilde_expansion() {
        let expanded = expand_tilde("~/Pictures/cosmic-shot");
        let home = dirs::home_dir().unwrap();
        assert_eq!(expanded, home.join("Pictures/cosmic-shot"));
    }

    #[test]
    fn config_tilde_expansion_bare_tilde() {
        let expanded = expand_tilde("~");
        let home = dirs::home_dir().unwrap();
        assert_eq!(expanded, home);
    }

    #[test]
    fn config_tilde_expansion_no_tilde() {
        let expanded = expand_tilde("/tmp/screenshots");
        assert_eq!(expanded, PathBuf::from("/tmp/screenshots"));
    }

    #[test]
    fn config_missing_file_uses_defaults() {
        let config = Config::load_from(Path::new("/nonexistent/path/config.toml"));
        assert_eq!(config.save_dir, "~/Pictures/cosmic-shot");
    }

    #[test]
    fn screenshot_filename_format() {
        let name = screenshot_filename();
        assert!(name.starts_with("screenshot-"));
        assert!(name.ends_with(".png"));
        // Format: screenshot-YYYY-MM-DD_HH-MM-SS.png = 35 chars
        assert_eq!(name.len(), 35);
    }
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test config::tests -- --nocapture`
Expected: all 6 tests pass.

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs src/lib.rs
git commit -m "feat: add config module with save_dir, tilde expansion, and filename generation"
```

---

### Task 3: Crop selection — TDD

**Files:**
- Create: `src/export/crop.rs`
- Modify: `src/export/mod.rs` — add `mod crop;` and re-export

- [ ] **Step 1: Create `src/export/crop.rs` with tests first**

```rust
use crate::types::FrameBuffer;

/// Cropped image in RGBA byte order.
#[derive(Debug)]
pub struct CroppedImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Crop a rectangular region from a `FrameBuffer`.
///
/// The region is specified in pixel coordinates and clamped to the frame's
/// bounds. The output is in RGBA8 byte order (same as `FrameBuffer::to_rgba`).
///
/// # Errors
///
/// Returns [`super::ExportError::Conversion`] if the frame data is malformed.
pub fn crop_selection(
    frame: &FrameBuffer,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<CroppedImage, super::ExportError> {
    let rgba = frame.to_rgba()?;

    // Clamp the crop region to the frame bounds.
    let x = x.min(frame.width);
    let y = y.min(frame.height);
    let w = width.min(frame.width.saturating_sub(x));
    let h = height.min(frame.height.saturating_sub(y));

    if w == 0 || h == 0 {
        return Ok(CroppedImage {
            rgba: Vec::new(),
            width: 0,
            height: 0,
        });
    }

    let src_stride = frame.width as usize * 4;
    let mut cropped = Vec::with_capacity(w as usize * h as usize * 4);

    for row in 0..h {
        let src_row_start = (y + row) as usize * src_stride + x as usize * 4;
        let src_row_end = src_row_start + w as usize * 4;
        cropped.extend_from_slice(&rgba[src_row_start..src_row_end]);
    }

    Ok(CroppedImage {
        rgba: cropped,
        width: w,
        height: h,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PixelFormat;

    /// Create a 4×4 Abgr8888 frame where pixel (x,y) has value [x, y, 0, 255].
    fn make_4x4_frame() -> FrameBuffer {
        let mut data = Vec::with_capacity(4 * 4 * 4);
        for y in 0..4u8 {
            for x in 0..4u8 {
                data.extend_from_slice(&[x, y, 0, 255]);
            }
        }
        FrameBuffer {
            data,
            width: 4,
            height: 4,
            stride: 16, // 4 pixels * 4 bytes
            format: PixelFormat::Abgr8888,
        }
    }

    #[test]
    fn crop_selection_extracts_correct_pixels() {
        let frame = make_4x4_frame();
        // Crop a 2×2 region starting at (1, 1).
        let cropped = crop_selection(&frame, 1, 1, 2, 2).unwrap();
        assert_eq!(cropped.width, 2);
        assert_eq!(cropped.height, 2);
        // Abgr8888: memory is already [R,G,B,A], so to_rgba() is identity.
        // Pixel (1,1) → [1, 1, 0, 255], pixel (2,1) → [2, 1, 0, 255]
        // Pixel (1,2) → [1, 2, 0, 255], pixel (2,2) → [2, 2, 0, 255]
        assert_eq!(
            cropped.rgba,
            vec![
                1, 1, 0, 255, 2, 1, 0, 255, // row y=1
                1, 2, 0, 255, 2, 2, 0, 255, // row y=2
            ]
        );
    }

    #[test]
    fn crop_selection_clamps_to_frame_bounds() {
        let frame = make_4x4_frame();
        // Request extends beyond frame: x=3, width=5 → clamped to width=1
        let cropped = crop_selection(&frame, 3, 0, 5, 2).unwrap();
        assert_eq!(cropped.width, 1);
        assert_eq!(cropped.height, 2);
        // Pixel (3,0) → [3, 0, 0, 255], pixel (3,1) → [3, 1, 0, 255]
        assert_eq!(cropped.rgba, vec![3, 0, 0, 255, 3, 1, 0, 255]);
    }

    #[test]
    fn crop_selection_fully_outside_returns_empty() {
        let frame = make_4x4_frame();
        // Start beyond frame bounds.
        let cropped = crop_selection(&frame, 10, 10, 5, 5).unwrap();
        assert_eq!(cropped.width, 0);
        assert_eq!(cropped.height, 0);
        assert!(cropped.rgba.is_empty());
    }
}
```

- [ ] **Step 2: Add module to `src/export/mod.rs`**

Add at the top of `src/export/mod.rs`, after the existing `use` statements:

```rust
mod crop;
pub use crop::{CroppedImage, crop_selection};
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test export::crop::tests -- --nocapture`
Expected: all 3 tests pass.

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add src/export/crop.rs src/export/mod.rs
git commit -m "feat: add crop_selection with clamping and full test coverage"
```

---

### Task 4: Clipboard and save helpers in export module

**Files:**
- Modify: `src/export/mod.rs`

- [ ] **Step 1: Add clipboard error variant to `ExportError`**

In `src/export/mod.rs`, add a new variant to `ExportError`:

```rust
    #[error("clipboard error: {0}")]
    Clipboard(String),
```

- [ ] **Step 2: Add `copy_to_clipboard` function**

Add after the existing `save_png` function in `src/export/mod.rs`:

```rust
/// Copy a cropped image to the system clipboard.
pub fn copy_to_clipboard(image: &CroppedImage) -> Result<(), ExportError> {
    use arboard::{Clipboard, ImageData};
    use std::borrow::Cow;

    let mut clipboard =
        Clipboard::new().map_err(|e| ExportError::Clipboard(e.to_string()))?;

    let img_data = ImageData {
        width: image.width as usize,
        height: image.height as usize,
        bytes: Cow::Borrowed(&image.rgba),
    };

    clipboard
        .set_image(img_data)
        .map_err(|e| ExportError::Clipboard(e.to_string()))?;

    tracing::info!("copied {}×{} image to clipboard", image.width, image.height);
    Ok(())
}
```

- [ ] **Step 3: Add `save_cropped_png` function**

Add after `copy_to_clipboard` in `src/export/mod.rs`:

```rust
/// Save a cropped image as a PNG file.
pub fn save_cropped_png(image: &CroppedImage, path: &Path) -> Result<(), ExportError> {
    let img: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(image.width, image.height, image.rgba.clone())
            .ok_or_else(|| ExportError::Encode("pixel buffer size mismatch".to_string()))?;

    img.save(path)
        .map_err(|e| ExportError::Encode(e.to_string()))?;

    tracing::info!(path = %path.display(), "saved cropped PNG");
    Ok(())
}
```

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add src/export/mod.rs
git commit -m "feat: add copy_to_clipboard and save_cropped_png export helpers"
```

---

### Task 5: Store raw FrameBuffers in overlay and add export messages

**Files:**
- Modify: `src/overlay/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add `raw_frames` field to `OverlayState`**

In `src/overlay/mod.rs`, change the `OverlayState` struct (line 15-26) to:

```rust
/// State shared across all layer-shell surfaces.
struct OverlayState {
    /// Raw captured frames in output order — used for cropping on export.
    raw_frames: Vec<FrameBuffer>,
    /// Frozen frames as image handles in output order — used for display.
    handles: Vec<image::Handle>,
    /// Windows assigned so far: window::Id → index into frames/handles.
    window_frame_idx: std::collections::HashMap<iced::window::Id, usize>,
    /// Global selection state — shared across all surfaces.
    selection: SelectionState,
    /// Current cursor position on the active window.
    cursor_pos: iced::Point,
    /// Which window owns the current selection (set on MousePressed).
    active_window: Option<iced::window::Id>,
}
```

- [ ] **Step 2: Add `CopyRequested` and `SaveRequested` to `Message`**

In the `Message` enum (line 35-43), add two new variants before the closing brace:

```rust
    /// User clicked Copy in the toolbar.
    CopyRequested,
    /// User clicked Save in the toolbar.
    SaveRequested,
```

- [ ] **Step 3: Update `run()` to store raw frames and build handles**

Change `run()` (currently at line 238) to accept frames, clone them for raw storage, and build handles:

Replace the existing frame conversion block (lines 243-252) with:

```rust
    // Keep raw frames for export cropping; build image handles for display.
    let raw_frames = frames;
    let handles: Vec<image::Handle> = raw_frames
        .iter()
        .map(|f| {
            // INVARIANT: data was read as pool.mmap()[..stride*height] in
            // capture_one_output, so data.len() == stride * height always holds.
            let rgba = f.to_rgba().expect("captured frame data is well-formed");
            image::Handle::from_rgba(f.width, f.height, rgba)
        })
        .collect();
```

- [ ] **Step 4: Update `run_with` closure to initialise `OverlayState` with new fields**

Replace the `OverlayState` construction in `run_with` (lines 337-343) with:

```rust
        OverlayState {
            raw_frames,
            handles,
            window_frame_idx: std::collections::HashMap::new(),
            selection: SelectionState::Idle,
            cursor_pos: iced::Point::ORIGIN,
            active_window: None,
        },
```

- [ ] **Step 5: Update all `state.frames` references to `state.handles`**

In `overlay_view()` (line 199-205), replace all references to `state.frames` with `state.handles`:

```rust
    let frame_idx = state.window_frame_idx.get(&window).copied().unwrap_or(0);
    let handle = state
        .handles
        .get(frame_idx)
        .or_else(|| state.handles.first())
        .cloned()
        .unwrap_or_else(|| state.handles[0].clone());
```

In `update` closure, the frame clamping (line 272) changes from `state.frames.len()` to `state.handles.len()`:

```rust
                        let clamped = next_idx.min(state.handles.len().saturating_sub(1));
```

- [ ] **Step 6: Add `mod config` to `main.rs`**

In `src/main.rs`, add after the existing module declarations:

```rust
mod config;
```

Also add `mod export;` since we're now using it from main.rs indirectly through overlay:

```rust
#[allow(dead_code)]
mod export;
```

- [ ] **Step 7: Run clippy and tests**

Run: `cargo clippy -- -D warnings && cargo test -- --include-ignored`
Expected: clean clippy, all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/overlay/mod.rs src/main.rs
git commit -m "feat: store raw FrameBuffers in overlay and add CopyRequested/SaveRequested messages"
```

---

### Task 6: Wire toolbar clicks and export actions

**Files:**
- Modify: `src/overlay/mod.rs`

- [ ] **Step 1: Add toolbar hit-test constants and import config/export**

At the top of `src/overlay/mod.rs`, add these imports:

```rust
use crate::config::{self, Config};
use crate::export::{self, crop_selection};
```

- [ ] **Step 2: Store toolbar bounds in `OverlayState`**

Add a field to `OverlayState`:

```rust
    /// Toolbar button bounds — computed during draw, checked during update.
    toolbar_copy_rect: Option<Rectangle>,
    toolbar_save_rect: Option<Rectangle>,
```

Initialise them as `None` in `run_with`.

- [ ] **Step 3: Compute and store toolbar button bounds during `draw()`**

In `SelectionCanvas::draw()`, where the toolbar labels are drawn (the `for (i, label)` loop, currently lines 158-169), replace with code that computes and stores per-button rectangles. Since `draw()` has `&self` (immutable), we can't write to state here. Instead, compute toolbar bounds in the `update` handler where we have `&mut state`.

Actually, the simpler approach: compute the toolbar geometry from the selection rect in `update()` when checking clicks, matching the same layout math used in `draw()`.

- [ ] **Step 4: Update toolbar rendering — white labels instead of grey**

In `SelectionCanvas::draw()`, change the toolbar label colour (line 165) from grey to white:

```rust
                            color: Color::WHITE,
```

- [ ] **Step 5: Handle toolbar clicks in `MousePressed`**

Replace the `Message::MousePressed` handler (lines 281-285) with click detection that checks if the click falls within a toolbar button:

```rust
                Message::MousePressed(id) => {
                    // If we have a selected rect with a visible toolbar, check
                    // if the click lands on a toolbar button.
                    if let SelectionState::Selected { rect } = &state.selection {
                        if state.active_window == Some(id) {
                            let toolbar_w = 120.0_f32;
                            let toolbar_h = 32.0_f32;
                            let toolbar_x = rect.x + (rect.width - toolbar_w) / 2.0;
                            // Match the draw() logic for toolbar Y position.
                            // We don't have bounds.height here, so assume below
                            // works (same heuristic as draw). We use a large
                            // default because layer-shell surfaces fill the output.
                            let toolbar_y = rect.y + rect.height + 8.0;

                            let copy_rect = Rectangle {
                                x: toolbar_x,
                                y: toolbar_y,
                                width: toolbar_w / 2.0,
                                height: toolbar_h,
                            };
                            let save_rect = Rectangle {
                                x: toolbar_x + toolbar_w / 2.0,
                                y: toolbar_y,
                                width: toolbar_w / 2.0,
                                height: toolbar_h,
                            };

                            let click = state.cursor_pos;
                            if copy_rect.contains(click) {
                                return IcedTask::done(Message::CopyRequested);
                            }
                            if save_rect.contains(click) {
                                return IcedTask::done(Message::SaveRequested);
                            }
                        }
                    }

                    // Default: start a new selection, cancelling any existing rect.
                    state.active_window = Some(id);
                    state.selection = SelectionState::Drawing { start: state.cursor_pos };
                    IcedTask::none()
                }
```

- [ ] **Step 6: Handle `CopyRequested` message**

Add to the `match message` in the `update` closure:

```rust
                Message::CopyRequested => {
                    if let SelectionState::Selected { rect } = &state.selection {
                        if let Some(window_id) = state.active_window {
                            let frame_idx = state.window_frame_idx
                                .get(&window_id)
                                .copied()
                                .unwrap_or(0);
                            if let Some(frame) = state.raw_frames.get(frame_idx) {
                                match crop_selection(
                                    frame,
                                    rect.x as u32,
                                    rect.y as u32,
                                    rect.width as u32,
                                    rect.height as u32,
                                ) {
                                    Ok(cropped) => {
                                        if let Err(e) = export::copy_to_clipboard(&cropped) {
                                            tracing::error!(%e, "clipboard copy failed");
                                        }
                                    }
                                    Err(e) => tracing::error!(%e, "crop failed"),
                                }
                            }
                        }
                    }
                    iced::exit()
                }
```

- [ ] **Step 7: Handle `SaveRequested` message**

Add to the `match message` in the `update` closure:

```rust
                Message::SaveRequested => {
                    if let SelectionState::Selected { rect } = &state.selection {
                        if let Some(window_id) = state.active_window {
                            let frame_idx = state.window_frame_idx
                                .get(&window_id)
                                .copied()
                                .unwrap_or(0);
                            if let Some(frame) = state.raw_frames.get(frame_idx) {
                                match crop_selection(
                                    frame,
                                    rect.x as u32,
                                    rect.y as u32,
                                    rect.width as u32,
                                    rect.height as u32,
                                ) {
                                    Ok(cropped) => {
                                        let cfg = Config::load();
                                        let dir = cfg.resolved_save_dir();
                                        if let Err(e) = std::fs::create_dir_all(&dir) {
                                            tracing::error!(%e, "failed to create save directory");
                                        } else {
                                            let path = dir.join(config::screenshot_filename());
                                            if let Err(e) = export::save_cropped_png(&cropped, &path) {
                                                tracing::error!(%e, "save failed");
                                            }
                                        }
                                    }
                                    Err(e) => tracing::error!(%e, "crop failed"),
                                }
                            }
                        }
                    }
                    iced::exit()
                }
```

- [ ] **Step 8: Run clippy and build**

Run: `cargo clippy -- -D warnings && cargo build --release`
Expected: clean clippy, successful release build.

- [ ] **Step 9: Run all tests**

Run: `cargo test -- --include-ignored`
Expected: all tests pass (existing + new config + crop tests).

- [ ] **Step 10: Commit**

```bash
git add src/overlay/mod.rs
git commit -m "feat: wire toolbar Copy/Save buttons to export actions with exit"
```

---

### Task 7: Clean up stale `#[allow]` attributes (review item)

**Files:**
- Modify: `src/overlay/selection.rs`
- Modify: `src/overlay/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Remove stale `#[allow(dead_code)]` from `SelectionState`**

In `src/overlay/selection.rs` line 6, remove:
```rust
#[allow(dead_code)]
```

- [ ] **Step 2: Remove stale `#[allow(dead_code)]` from `normalize_rect`**

In `src/overlay/selection.rs` line 19, remove:
```rust
#[allow(dead_code)]
```

- [ ] **Step 3: Remove `#[allow(unused_imports)]` from overlay re-export**

In `src/overlay/mod.rs` line 2, remove:
```rust
#[allow(unused_imports)]
```

- [ ] **Step 4: Run clippy to verify no new warnings**

Run: `cargo clippy -- -D warnings`
Expected: clean. If any dead-code warnings appear, fix them rather than re-adding allows.

- [ ] **Step 5: Commit**

```bash
git add src/overlay/selection.rs src/overlay/mod.rs src/main.rs
git commit -m "chore: remove stale #[allow(dead_code)] and #[allow(unused_imports)] attributes"
```

---

### Task 8: Manual end-to-end verification

- [ ] **Step 1: Run the app**

Run: `cargo run --release`
Expected: overlay appears on all monitors with frozen frames.

- [ ] **Step 2: Draw a selection and click Copy**

1. Click and drag to select a region.
2. Click "Copy" on the toolbar.
3. Open an image editor (e.g. GIMP) and paste.
Expected: the pasted image matches the selected region.

- [ ] **Step 3: Draw a selection and click Save**

1. Run the app again.
2. Click and drag to select a region.
3. Click "Save" on the toolbar.
4. Check `~/Pictures/cosmic-shot/` for a new `screenshot-*.png`.
Expected: file exists, dimensions match the selection, image content is correct.

- [ ] **Step 4: Test config override**

1. Create `~/.config/cosmic-shot/config.toml` with:
   ```toml
   save_dir = "/tmp/cosmic-shot-test"
   ```
2. Run the app, select, click Save.
Expected: PNG saved to `/tmp/cosmic-shot-test/screenshot-*.png`.

- [ ] **Step 5: Clean up test config**

Remove the test config file if desired.

---

### Task 9: Update design doc with implementation notes

**Files:**
- Modify: `docs/superpowers/specs/2026-05-09-m4-clipboard-save-design.md`

- [ ] **Step 1: Append implementation notes**

Add an `## Implementation Notes` section documenting any deviations from the design spec discovered during implementation.

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/specs/2026-05-09-m4-clipboard-save-design.md
git commit -m "docs: update M4 spec with implementation notes"
```
