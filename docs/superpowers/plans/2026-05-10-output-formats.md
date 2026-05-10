# Output Formats Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add JPEG and WebP output format support alongside PNG, configurable via `config.toml` with a single quality setting.

**Architecture:** Add an `OutputFormat` enum to `config.rs`, parameterize the export functions to accept format and quality, and wire the config into the overlay's save handler. Clipboard always uses PNG.

**Tech Stack:** `image` crate (adding `jpeg` + `webp` features), `serde` for config deserialization

---

### Task 1: Add `OutputFormat` enum and config fields

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Write tests for `OutputFormat` deserialization and defaults**

Add these tests to the existing `#[cfg(test)] mod tests` block in `src/config.rs`:

```rust
#[test]
fn output_format_file_extension() {
    assert_eq!(OutputFormat::Png.file_extension(), "png");
    assert_eq!(OutputFormat::Jpeg.file_extension(), "jpg");
    assert_eq!(OutputFormat::WebP.file_extension(), "webp");
}

#[test]
fn output_format_deserialize_valid() {
    let config: Config = toml::from_str(r#"format = "jpeg""#).unwrap();
    assert_eq!(config.format, OutputFormat::Jpeg);

    let config: Config = toml::from_str(r#"format = "webp""#).unwrap();
    assert_eq!(config.format, OutputFormat::WebP);

    let config: Config = toml::from_str(r#"format = "png""#).unwrap();
    assert_eq!(config.format, OutputFormat::Png);
}

#[test]
fn config_default_format_and_quality() {
    let config = Config::default();
    assert_eq!(config.format, OutputFormat::Png);
    assert_eq!(config.quality, 85);
}

#[test]
fn config_quality_from_toml() {
    let config: Config = toml::from_str(r#"quality = 50"#).unwrap();
    assert_eq!(config.quality, 50);
}

#[test]
fn screenshot_filename_with_format() {
    let name = screenshot_filename(&OutputFormat::Jpeg);
    assert!(name.starts_with("screenshot-"));
    assert!(name.ends_with(".jpg"));

    let name = screenshot_filename(&OutputFormat::WebP);
    assert!(name.ends_with(".webp"));

    let name = screenshot_filename(&OutputFormat::Png);
    assert!(name.ends_with(".png"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib config`
Expected: compilation errors — `OutputFormat` doesn't exist yet, `screenshot_filename` doesn't take a parameter.

- [ ] **Step 3: Implement `OutputFormat` enum**

Add this above the `Config` struct in `src/config.rs`:

```rust
/// Supported output image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Png,
    Jpeg,
    #[serde(alias = "webp")]
    WebP,
}

impl OutputFormat {
    /// File extension for this format (without the dot).
    pub fn file_extension(&self) -> &str {
        match self {
            OutputFormat::Png => "png",
            OutputFormat::Jpeg => "jpg",
            OutputFormat::WebP => "webp",
        }
    }

    /// Corresponding `image::ImageFormat`.
    pub fn image_format(&self) -> image::ImageFormat {
        match self {
            OutputFormat::Png => image::ImageFormat::Png,
            OutputFormat::Jpeg => image::ImageFormat::Jpeg,
            OutputFormat::WebP => image::ImageFormat::WebP,
        }
    }
}
```

- [ ] **Step 4: Add `format` and `quality` fields to `Config`**

Update the `Config` struct:

```rust
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Directory where screenshots are saved.
    pub save_dir: String,
    /// Human-readable keyboard shortcut shown in --print-shortcut output.
    pub shortcut: String,
    /// Output image format.
    pub format: OutputFormat,
    /// Quality for lossy formats (1-100). Ignored for PNG.
    pub quality: u8,
}
```

Update `Default for Config`:

```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            save_dir: "~/Pictures/cosmic-shot".to_string(),
            shortcut: "Alt+Shift+S".to_string(),
            format: OutputFormat::Png,
            quality: 85,
        }
    }
}
```

- [ ] **Step 5: Update `screenshot_filename()` to accept format**

Change the function signature and body:

```rust
/// Generate a timestamped screenshot filename.
///
/// Format: `screenshot-YYYY-MM-DD_HH-MM-SS.{ext}`
pub fn screenshot_filename(format: &OutputFormat) -> String {
    let now = chrono::Local::now();
    now.format(&format!("screenshot-%Y-%m-%d_%H-%M-%S.{}", format.file_extension()))
        .to_string()
}
```

- [ ] **Step 6: Update the old `screenshot_filename_format` test**

The existing test at line 141 checks for `.png` extension and length 34. Update it:

```rust
#[test]
fn screenshot_filename_format() {
    let name = screenshot_filename(&OutputFormat::Png);
    assert!(name.starts_with("screenshot-"));
    assert!(name.ends_with(".png"));
    // Format: screenshot-YYYY-MM-DD_HH-MM-SS.png = 34 chars
    assert_eq!(name.len(), 34);
}
```

- [ ] **Step 7: Add `image` to config.rs imports**

Add at the top of `src/config.rs` (needed for `image::ImageFormat` in `OutputFormat::image_format`):

The `image` crate is already a dependency. No import needed at the module level — the method body uses the fully qualified path `image::ImageFormat`. Verify this compiles.

- [ ] **Step 8: Run tests**

Run: `cargo test --lib config`
Expected: all config tests pass.

- [ ] **Step 9: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings.

- [ ] **Step 10: Commit**

```bash
git add src/config.rs
git commit -m "feat: add OutputFormat enum and format/quality config fields"
```

---

### Task 2: Add JPEG/WebP features to image crate and update export

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/export/mod.rs`

- [ ] **Step 1: Add `jpeg` and `webp` features to image crate**

In `Cargo.toml`, change the `image` dependency:

```toml
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "webp"] }
```

- [ ] **Step 2: Write a test for JPEG alpha-drop**

Add a test module at the bottom of `src/export/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OutputFormat;
    use std::path::PathBuf;

    /// Create a tiny 2×2 RGBA test image.
    fn make_2x2_cropped() -> CroppedImage {
        CroppedImage {
            rgba: vec![
                255, 0, 0, 255,   0, 255, 0, 255,    // row 0: red, green
                0, 0, 255, 255,   255, 255, 0, 128,   // row 1: blue, yellow (semi-transparent)
            ],
            width: 2,
            height: 2,
        }
    }

    #[test]
    fn save_cropped_png() {
        let img = make_2x2_cropped();
        let dir = std::env::temp_dir().join("cosmic-shot-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.png");
        save_cropped(&img, &path, OutputFormat::Png, 85).unwrap();
        assert!(path.exists());
        // Verify it's a valid PNG by reading it back.
        let decoded = image::open(&path).unwrap();
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn save_cropped_jpeg() {
        let img = make_2x2_cropped();
        let dir = std::env::temp_dir().join("cosmic-shot-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.jpg");
        save_cropped(&img, &path, OutputFormat::Jpeg, 85).unwrap();
        assert!(path.exists());
        let decoded = image::open(&path).unwrap();
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn save_cropped_webp() {
        let img = make_2x2_cropped();
        let dir = std::env::temp_dir().join("cosmic-shot-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.webp");
        save_cropped(&img, &path, OutputFormat::WebP, 90).unwrap();
        assert!(path.exists());
        let decoded = image::open(&path).unwrap();
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
        std::fs::remove_file(&path).ok();
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --lib export`
Expected: compilation error — `save_cropped` doesn't exist yet.

- [ ] **Step 4: Implement `save_cropped()`**

Replace the existing `save_cropped_png()` function in `src/export/mod.rs` with:

```rust
use crate::config::OutputFormat;

/// Save a cropped image to a file in the specified format.
pub fn save_cropped(
    image: &CroppedImage,
    path: &Path,
    format: OutputFormat,
    quality: u8,
) -> Result<(), ExportError> {
    let quality = quality.clamp(1, 100);

    match format {
        OutputFormat::Png => {
            let img: ImageBuffer<Rgba<u8>, _> =
                ImageBuffer::from_raw(image.width, image.height, image.rgba.clone())
                    .ok_or_else(|| ExportError::Encode("pixel buffer size mismatch".into()))?;
            img.save(path)
                .map_err(|e| ExportError::Encode(e.to_string()))?;
        }
        OutputFormat::Jpeg => {
            // JPEG doesn't support alpha — convert RGBA to RGB.
            let rgb_data: Vec<u8> = image
                .rgba
                .chunks_exact(4)
                .flat_map(|px| [px[0], px[1], px[2]])
                .collect();
            let img: ImageBuffer<image::Rgb<u8>, _> =
                ImageBuffer::from_raw(image.width, image.height, rgb_data)
                    .ok_or_else(|| ExportError::Encode("pixel buffer size mismatch".into()))?;
            let mut file = std::fs::File::create(path)?;
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, quality);
            img.write_with_encoder(encoder)
                .map_err(|e| ExportError::Encode(e.to_string()))?;
        }
        OutputFormat::WebP => {
            let img: ImageBuffer<Rgba<u8>, _> =
                ImageBuffer::from_raw(image.width, image.height, image.rgba.clone())
                    .ok_or_else(|| ExportError::Encode("pixel buffer size mismatch".into()))?;
            let mut file = std::fs::File::create(path)?;
            let encoder = image::codecs::webp::WebPEncoder::new_with_quality(
                &mut file,
                image::codecs::webp::WebPQuality::lossy(quality),
            );
            img.write_with_encoder(encoder)
                .map_err(|e| ExportError::Encode(e.to_string()))?;
        }
    }

    tracing::info!(path = %path.display(), format = ?format, "saved cropped image");
    Ok(())
}
```

- [ ] **Step 5: Update `ExportError` encode variant description**

Change the error description from PNG-specific to generic:

```rust
#[error("image encoding failed: {0}")]
Encode(String),
```

- [ ] **Step 6: Run tests**

Run: `cargo test --lib export`
Expected: all 3 new tests + 3 existing crop tests pass.

- [ ] **Step 7: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml src/export/mod.rs
git commit -m "feat: add JPEG/WebP export support with quality parameter"
```

---

### Task 3: Wire config format into overlay save handler

**Files:**
- Modify: `src/overlay/mod.rs`

- [ ] **Step 1: Update `SaveRequested` handler**

In `src/overlay/mod.rs`, find the `Message::SaveRequested` match arm (around line 429). Change:

```rust
let path = dir.join(config::screenshot_filename());
if let Err(e) = export::save_cropped_png(&cropped, &path) {
```

to:

```rust
let path = dir.join(config::screenshot_filename(&cfg.format));
if let Err(e) = export::save_cropped(&cropped, &path, cfg.format, cfg.quality) {
```

Note: `cfg` is already defined on line 446 as `let cfg = Config::load();`, so `cfg.format` and `cfg.quality` are available.

- [ ] **Step 2: Verify compilation**

Run: `cargo build`
Expected: compiles without errors.

- [ ] **Step 3: Run all tests**

Run: `cargo test`
Expected: all tests pass (config, export, crop, overlay, integration).

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
git add src/overlay/mod.rs
git commit -m "feat: wire output format config into save handler"
```

---

### Task 4: Verify full pipeline

- [ ] **Step 1: Run clippy**

```bash
cargo clippy -- -D warnings
```
Expected: no warnings.

- [ ] **Step 2: Run all tests**

```bash
cargo test
```
Expected: all tests pass.

- [ ] **Step 3: Build release**

```bash
cargo build --release
```
Expected: compiles successfully.

- [ ] **Step 4: Verify .deb still builds**

```bash
cargo deb --no-build
```
Expected: `.deb` created without errors.
