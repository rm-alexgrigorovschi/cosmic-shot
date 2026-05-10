use std::path::{Path, PathBuf};

use serde::Deserialize;

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
    #[allow(dead_code)] // Used in export module (next task)
    pub fn image_format(&self) -> image::ImageFormat {
        match self {
            OutputFormat::Png => image::ImageFormat::Png,
            OutputFormat::Jpeg => image::ImageFormat::Jpeg,
            OutputFormat::WebP => image::ImageFormat::WebP,
        }
    }
}

/// Application configuration.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Directory where screenshots are saved.
    pub save_dir: String,
    /// Human-readable keyboard shortcut shown in --print-shortcut output.
    /// Not used at runtime — documents which shortcut to register in COSMIC Settings.
    pub shortcut: String,
    /// Output image format.
    pub format: OutputFormat,
    /// Quality for lossy formats (1-100). Ignored for PNG.
    pub quality: u8,
    /// Seconds to wait before capturing. Clamped to 0–60.
    pub delay_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_dir: "~/Pictures/cosmic-shot".to_string(),
            shortcut: "Alt+Shift+S".to_string(),
            format: OutputFormat::Png,
            quality: 85,
            delay_secs: 0,
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
        tracing::debug!(path = %path.display(), "loading config");
        Self::load_from(&path)
    }

    /// Load config from a specific path (for testing).
    pub fn load_from(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(mut config) => {
                    if config.delay_secs > 60 {
                        tracing::warn!(
                            delay_secs = config.delay_secs,
                            "delay_secs exceeds maximum of 60, clamping"
                        );
                        config.delay_secs = 60;
                    }
                    tracing::info!(path = %path.display(), "config loaded");
                    config
                }
                Err(e) => {
                    tracing::warn!(%e, path = %path.display(), "failed to parse config, using defaults");
                    Self::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => {
                tracing::warn!(%e, "failed to read config, using defaults");
                Self::default()
            }
        }
    }

    /// Resolve `save_dir` by expanding `~` to the user's home directory.
    pub fn resolved_save_dir(&self) -> PathBuf {
        expand_tilde(&self.save_dir)
    }
}

/// Expand a leading `~` or `~/` to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    } else if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(path)
}

/// Generate a timestamped screenshot filename.
///
/// Format: `screenshot-YYYY-MM-DD_HH-MM-SS.{ext}`
pub fn screenshot_filename(format: &OutputFormat) -> String {
    let now = chrono::Local::now();
    now.format(&format!("screenshot-%Y-%m-%d_%H-%M-%S.{}", format.file_extension()))
        .to_string()
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
    fn config_default_shortcut() {
        let config = Config::default();
        assert_eq!(config.shortcut, "Alt+Shift+S");
    }

    #[test]
    fn config_shortcut_parsed_from_toml() {
        let toml = r#"
            save_dir = "~/Pictures"
            shortcut = "Super+Shift+S"
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.shortcut, "Super+Shift+S");
    }

    #[test]
    fn screenshot_filename_format() {
        let name = screenshot_filename(&OutputFormat::Png);
        assert!(name.starts_with("screenshot-"));
        assert!(name.ends_with(".png"));
        // Format: screenshot-YYYY-MM-DD_HH-MM-SS.png = 34 chars
        assert_eq!(name.len(), 34);
    }

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
    fn config_default_delay_secs() {
        let config = Config::default();
        assert_eq!(config.delay_secs, 0);
    }

    #[test]
    fn config_delay_secs_from_toml() {
        let config: Config = toml::from_str(r#"delay_secs = 5"#).unwrap();
        assert_eq!(config.delay_secs, 5);
    }

    #[test]
    fn config_delay_secs_clamped_to_60() {
        let dir = std::env::temp_dir().join("cosmic-shot-test-config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "delay_secs = 120").unwrap();
        let config = Config::load_from(&path);
        assert_eq!(config.delay_secs, 60);
        std::fs::remove_file(&path).ok();
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
}
