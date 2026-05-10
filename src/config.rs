use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Application configuration.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Directory where screenshots are saved.
    pub save_dir: String,
    /// Human-readable keyboard shortcut shown in --print-shortcut output.
    /// Not used at runtime — documents which shortcut to register in COSMIC Settings.
    pub shortcut: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_dir: "~/Pictures/cosmic-shot".to_string(),
            shortcut: "Alt+Shift+S".to_string(),
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
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => {
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
        let name = screenshot_filename();
        assert!(name.starts_with("screenshot-"));
        assert!(name.ends_with(".png"));
        // Format: screenshot-YYYY-MM-DD_HH-MM-SS.png = 34 chars
        assert_eq!(name.len(), 34);
    }
}
