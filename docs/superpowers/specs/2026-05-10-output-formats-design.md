# Output Format Options Design

## Overview

Add JPEG and WebP output support alongside the existing PNG format. Format and
quality are configured via `config.toml`. Clipboard always uses PNG for
compatibility.

## Config

Two new fields in `~/.config/cosmic-shot/config.toml`:

```toml
format = "png"    # "png", "jpeg", or "webp"
quality = 85      # 1-100, applies to jpeg/webp only (png is always lossless)
```

Both optional. Defaults: `format = "png"`, `quality = 85`.

Invalid values (e.g. `format = "bmp"`) log a warning via `tracing::warn!` and
fall back to defaults. Quality is clamped to the 1-100 range (values outside
are silently clamped, not rejected).

### OutputFormat enum

A new `OutputFormat` enum in `config.rs`:

```rust
enum OutputFormat {
    Png,
    Jpeg,
    WebP,
}
```

Methods:
- `file_extension(&self) -> &str` — returns `"png"`, `"jpg"`, or `"webp"`
- `image_format(&self) -> image::ImageFormat` — returns the corresponding `ImageFormat` variant
- `mime_type(&self) -> &str` — returns `"image/png"`, `"image/jpeg"`, or `"image/webp"`

Deserialized from the config string via a custom `serde::Deserialize` impl (or
`#[serde(rename_all = "lowercase")]` with manual variants).

## Export Changes

### save_cropped()

Rename `save_cropped_png()` to `save_cropped()`. New signature:

```rust
pub fn save_cropped(
    img: &CroppedImage,
    path: &Path,
    format: OutputFormat,
    quality: u8,
) -> Result<(), ExportError>
```

Behavior by format:
- **PNG:** encode via `ImageFormat::Png`. Quality parameter ignored.
- **JPEG:** convert RGBA to RGB first (drop alpha channel), then encode with
  the quality parameter. JPEG does not support transparency.
- **WebP:** encode RGBA with the quality parameter.

### copy_to_clipboard()

No changes. Always encodes as PNG and pipes to `wl-copy --type image/png`.
This ensures maximum compatibility with paste targets.

### screenshot_filename()

Parameterize the file extension:

```rust
pub fn screenshot_filename(format: &OutputFormat) -> String {
    let now = chrono::Local::now();
    now.format(&format!("screenshot-%Y-%m-%d_%H-%M-%S.{}", format.file_extension()))
        .to_string()
}
```

## Overlay Integration

In `overlay/mod.rs`:

- `SaveRequested` handler: read `config.format` and `config.quality`, pass to
  `save_cropped()` and `screenshot_filename()`.
- `CopyRequested` handler: unchanged (still uses `copy_to_clipboard()` which is
  PNG-only).

No UI changes. The toolbar buttons work identically.

## Cargo.toml

Add `"jpeg"` and `"webp"` to the `image` crate features:

```toml
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "webp"] }
```

## Files Modified

| File | Change |
|------|--------|
| `Cargo.toml` | Add `jpeg`, `webp` features to `image` crate |
| `src/config.rs` | Add `OutputFormat` enum, `format`/`quality` fields to `Config`, update `screenshot_filename()` |
| `src/export/mod.rs` | Rename `save_cropped_png` → `save_cropped`, add format/quality params, RGBA→RGB for JPEG |
| `src/overlay/mod.rs` | Pass format/quality from config to export in `SaveRequested` handler |

## Out of Scope

- Format selection UI (all config via `config.toml`)
- Clipboard format matching (always PNG)
- GIF, BMP, TIFF, or other formats
- Per-screenshot format override via CLI flags

## Testing

- Unit tests for `OutputFormat` enum: `file_extension()`, `image_format()`, deserialization
- Unit tests for `screenshot_filename()` with each format
- Config parsing tests: valid formats, invalid formats (fallback), missing fields (defaults)
- Integration: `save_cropped()` with each format on a test image, verify file is valid
- JPEG alpha-drop test: verify RGBA input produces valid RGB JPEG output
