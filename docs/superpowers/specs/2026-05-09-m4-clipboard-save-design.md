# M4 Design: Clipboard + File Save

Wire the placeholder toolbar buttons (Copy / Save) to real export actions.
Introduce a config file for the save directory. Crop the selection from the
active output's frame before exporting.

## Config

- Path: `~/.config/cosmic-shot/config.toml`
- Created on first run if missing, populated with defaults
- M4 introduces one field:

```toml
# Directory where screenshots are saved
save_dir = "~/Pictures/cosmic-shot"
```

- `~` is expanded to `$HOME` (via `dirs::home_dir()`) at runtime
- The directory is created recursively on save if it does not exist
- Filename pattern: `screenshot-YYYY-MM-DD_HH-MM-SS.png` (local time)

### Dependencies

- `serde` + `serde` derive — config deserialization
- `toml` — TOML parsing
- `dirs` — portable `home_dir()` for `~` expansion
- `chrono` — local-time timestamp formatting

### Error policy

Config parse failures log a warning via `tracing::warn!` and fall back to
compiled-in defaults. A missing file is not an error — defaults are used and
the file is written on first save so the user has something to edit.

## Cropping

The selection rectangle lives in screen coordinates on the active window's
output. To export only the selected region:

1. Identify the `FrameBuffer` that belongs to `active_window`
   (via `window_frame_idx` → index into `frames: Vec<FrameBuffer>`).
2. Clamp the selection rectangle to the frame's pixel bounds.
3. Copy only the selected rows/columns into a new RGBA `Vec<u8>`.

This logic lives in `export/` as a new `crop_selection()` function.

### Signature

```rust
pub fn crop_selection(
    frame: &FrameBuffer,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<CroppedImage, ExportError>;

pub struct CroppedImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}
```

No allocation happens during the selection phase; cropping runs only when the
user triggers an export action.

## Clipboard (Copy)

- New dependency: `arboard`
- Crop the selection region (as above)
- Set the clipboard via `arboard::Clipboard::new()?.set_image(ImageData { ... })`
- `arboard::ImageData` accepts RGBA bytes directly — no PNG encoding needed

## Save

1. Load config (or defaults).
2. Expand `~` in `save_dir`.
3. Ensure directory exists (`fs::create_dir_all`).
4. Generate filename: `screenshot-YYYY-MM-DD_HH-MM-SS.png` (local time via `chrono`).
5. Crop selection.
6. Encode as PNG via `image` crate (reuse existing `save_png` path, adapted for `CroppedImage`).
7. Write to disk.

## Overlay Message Flow

```
User releases mouse → SelectionState::Selected
User clicks "Copy"  → Message::CopyRequested
User clicks "Save"  → Message::SaveRequested

CopyRequested:
  1. crop selection from active frame
  2. set clipboard (arboard)
  3. iced::exit()

SaveRequested:
  1. crop selection from active frame
  2. load config → resolve save_dir
  3. create dir if needed
  4. save_png(cropped, path)
  5. iced::exit()
```

## Toolbar Interaction

The M3 placeholder toolbar draws grey text labels ("Copy", "Save"). For M4:

- **Make them clickable:** detect whether a `MousePressed` event lands within a
  toolbar button's bounding box. If so, emit `Message::CopyRequested` or
  `Message::SaveRequested` instead of starting a new selection.
- **Active appearance:** change label colour from grey (`0.5, 0.5, 0.5`) to
  white when the toolbar is visible (i.e. `SelectionState::Selected`).
- No hover effect for M4 — keep it simple.

## Threading / Blocking

CLAUDE.md says "never block the Wayland event loop with synchronous I/O."
The iced event loop *is* the Wayland event loop, so in general we must avoid
blocking it.

For M4, both Copy and Save trigger an immediate `iced::exit()` after the
operation. The app is about to terminate, so a brief blocking call (clipboard
set ≈ negligible, PNG encode + write ≈ 10–50 ms) is acceptable. If profiling
shows a visible freeze on slow disks or huge selections, the encoding can be
moved to `tokio::task::spawn_blocking` — but not in M4.

## Error Handling

| Failure              | Behaviour                                    |
|----------------------|----------------------------------------------|
| Config parse error   | `tracing::warn!`, use defaults               |
| Missing save dir     | `create_dir_all`; if that fails → log error  |
| PNG encode failure   | `tracing::error!`, exit anyway               |
| Clipboard failure    | `tracing::error!`, exit anyway               |

The user must never be trapped in the overlay by an export error.

## Data Flow Change

The overlay currently converts `FrameBuffer`s to `image::Handle`s and discards
the raw data. M4 needs the raw pixel data for cropping, so the overlay must
store both:

- `frames: Vec<FrameBuffer>` — raw pixel data (for export)
- `handles: Vec<image::Handle>` — iced image handles (for display)

`FrameBuffer` is `Clone`, and we already hold the handles, so the additional
memory cost is one copy of the raw pixel data per output.

## New Dependencies

| Crate    | Purpose                        | Features     |
|----------|--------------------------------|--------------|
| `arboard` | Clipboard (image data)         | default      |
| `serde`  | Config deserialization          | `derive`     |
| `toml`   | TOML parsing                   | default      |
| `dirs`   | `home_dir()` for `~` expansion | default      |
| `chrono` | Local-time timestamp            | default      |

## Files Changed / Added

| File                       | Change                                                                                    |
|----------------------------|-------------------------------------------------------------------------------------------|
| `Cargo.toml`              | Add `arboard`, `serde`, `toml`, `dirs`, `chrono`                                         |
| `src/config.rs`           | **New** — `Config` struct, `load()`, defaults, `~` expansion, write-defaults-on-first-run |
| `src/export/mod.rs`       | Add `CroppedImage`, `crop_selection()`, `save_cropped_png()`; clipboard helper            |
| `src/overlay/mod.rs`      | Store raw `FrameBuffer`s; wire `CopyRequested`/`SaveRequested`; toolbar click detection   |
| `src/overlay/selection.rs`| No changes expected                                                                       |
| `src/main.rs`             | Pass raw frames to overlay alongside handles                                              |
| `src/lib.rs`              | `pub mod config;`                                                                         |

## Testing

| Test                                      | Location         | Asserts                                               |
|-------------------------------------------|------------------|-------------------------------------------------------|
| `crop_selection_extracts_correct_pixels`  | `export/mod.rs`  | Output pixels match expected sub-rect of input        |
| `crop_selection_clamps_to_frame_bounds`   | `export/mod.rs`  | Selection partially outside frame → clamped, no panic |
| `config_default_save_dir`                 | `config.rs`      | Default `save_dir` is `~/Pictures/cosmic-shot`        |
| `config_tilde_expansion`                  | `config.rs`      | `~` replaced with `$HOME`                             |
| `config_missing_file_uses_defaults`       | `config.rs`      | Non-existent path → defaults returned                 |
| `screenshot_filename_format`              | `config.rs`      | Filename matches `screenshot-YYYY-MM-DD_HH-MM-SS.png` |

Existing M1–M3 tests remain unaffected.
