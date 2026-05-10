# Delay Capture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a configurable pre-capture delay so users can capture tooltips and menus that disappear on click. Delay is set via `config.toml` (`delay_secs`) and/or `--delay N` CLI flag (CLI wins).

**Architecture:** All delay logic lives in `main.rs` between tracing init and `capture_all_outputs()`. Config gets a `delay_secs` field. The capture module is untouched.

**Tech Stack:** `tokio::time::sleep` (add `time` feature to Tokio), manual CLI parsing (same pattern as `--print-shortcut`)

---

### Task 1: Add `delay_secs` to Config

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Write failing tests**

Add these tests to the `#[cfg(test)] mod tests` block in `src/config.rs`:

```rust
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
    let config: Config = toml::from_str(r#"delay_secs = 120"#).unwrap();
    assert_eq!(config.delay_secs, 60);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib config`
Expected: compilation error — `delay_secs` field doesn't exist yet.

- [ ] **Step 3: Add `delay_secs` field to `Config` struct**

Add the field after `quality`:

```rust
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub save_dir: String,
    pub shortcut: String,
    pub format: OutputFormat,
    pub quality: u8,
    /// Seconds to wait before capturing. Clamped to 0–60.
    pub delay_secs: u64,
}
```

- [ ] **Step 4: Add `delay_secs` to `Default for Config`**

```rust
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
```

- [ ] **Step 5: Implement clamping at load time**

The clamping happens after deserialization. The cleanest way is a custom `Deserialize` shim for the field, but a simpler approach is to clamp in `load_from()` after parsing. Update the success branch in `load_from()`:

```rust
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
```

Note: the clamping test uses `toml::from_str` directly which bypasses `load_from()`. To make the test pass, also apply the clamp in a `fn clamp(&mut self)` method called from both `load_from()` and a `Deserialize` post-step, OR simply test via `load_from()` with a temp file. Use `load_from()` for the clamping test:

```rust
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
```

- [ ] **Step 6: Run tests**

Run: `cargo test --lib config`
Expected: all config tests pass including the 3 new ones.

- [ ] **Step 7: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings.

- [ ] **Step 8: Commit**

```bash
git add src/config.rs
git commit -m "feat: add delay_secs config field with 60s cap"
```

---

### Task 2: Add `--delay N` CLI flag and countdown in main.rs

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/main.rs`

- [ ] **Step 1: Add `time` feature to Tokio in Cargo.toml**

Change:
```toml
tokio = { version = "1", features = ["rt", "macros"] }
```
to:
```toml
tokio = { version = "1", features = ["rt", "macros", "time"] }
```

- [ ] **Step 2: Rewrite main.rs**

Replace the entire contents of `src/main.rs` with:

```rust
use anyhow::Context;
use tracing_subscriber::EnvFilter;

// These items are used via lib.rs; suppress dead_code for the binary target.
#[allow(dead_code)]
mod capture;
mod config;
mod export;
mod overlay;
mod types;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // Handle CLI flags before initialising tracing or touching Wayland.
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--print-shortcut") {
        let cfg = config::Config::load();
        println!("Shortcut: {}", cfg.shortcut);
        println!("Command:  cosmic-shot");
        return Ok(());
    }

    // Parse --delay N
    let cli_delay: Option<u64> = parse_delay_flag(&args)?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("starting cosmic-shot");

    let cfg = config::Config::load();

    // Resolve delay: CLI flag takes precedence over config.
    let delay_secs = cli_delay.unwrap_or(cfg.delay_secs);

    if delay_secs > 0 {
        run_countdown(delay_secs).await;
    }

    let frames = capture::capture_all_outputs()
        .context("failed to capture outputs")?;

    tracing::info!("captured {} output(s)", frames.len());

    overlay::run(frames).context("overlay error")?;

    Ok(())
}

/// Parse `--delay N` from the argument list.
///
/// Returns `Ok(Some(n))` if `--delay N` is present and valid,
/// `Ok(None)` if `--delay` is absent,
/// `Err` if `--delay` is present but the value is missing or not a number.
fn parse_delay_flag(args: &[String]) -> anyhow::Result<Option<u64>> {
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "--delay" {
            let val = iter
                .next()
                .ok_or_else(|| anyhow::anyhow!("--delay requires a value (e.g. --delay 3)"))?;
            let secs: u64 = val.parse().map_err(|_| {
                anyhow::anyhow!("--delay value must be a non-negative integer, got {:?}", val)
            })?;
            let secs = secs.min(60);
            return Ok(Some(secs));
        }
    }
    Ok(None)
}

/// Print a countdown to stdout and sleep until capture time.
async fn run_countdown(delay_secs: u64) {
    for remaining in (1..=delay_secs).rev() {
        println!("Capturing in {}...", remaining);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build`
Expected: compiles without errors.

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/main.rs
git commit -m "feat: add --delay N flag and countdown for pre-capture delay"
```

---

### Task 3: Add CLI integration tests

**Files:**
- Modify: `tests/cli.rs`

- [ ] **Step 1: Add tests for --delay flag**

Append to `tests/cli.rs`:

```rust
#[test]
fn delay_flag_missing_value_exits_nonzero() {
    let output = Command::new(env!("CARGO_BIN_EXE_cosmic-shot"))
        .arg("--delay")
        .output()
        .expect("failed to run cosmic-shot");

    assert!(
        !output.status.success(),
        "expected non-zero exit when --delay has no value"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("--delay requires a value") || stderr.contains("--delay"),
        "stderr should mention --delay: {:?}",
        stderr
    );
}

#[test]
fn delay_flag_non_integer_exits_nonzero() {
    let output = Command::new(env!("CARGO_BIN_EXE_cosmic-shot"))
        .args(["--delay", "abc"])
        .output()
        .expect("failed to run cosmic-shot");

    assert!(
        !output.status.success(),
        "expected non-zero exit when --delay value is not an integer"
    );
}

#[test]
fn delay_flag_zero_does_not_hang() {
    // --delay 0 should parse successfully and exit quickly (it will try to
    // connect to Wayland and fail, but should not hang on the countdown).
    // We just verify it doesn't panic on argument parsing by checking the
    // process exits within a reasonable time.
    use std::time::{Duration, Instant};
    let start = Instant::now();
    let _output = Command::new(env!("CARGO_BIN_EXE_cosmic-shot"))
        .args(["--delay", "0"])
        .output()
        .expect("failed to run cosmic-shot");
    // Should exit quickly — well under 5 seconds (no countdown, may fail on Wayland)
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "process took too long with --delay 0"
    );
}
```

- [ ] **Step 2: Run the CLI tests**

Run: `cargo test --test cli`
Expected: all 4 CLI tests pass (including existing `print_shortcut_prints_two_lines_and_exits_zero`).

Note: the `delay_flag_zero_does_not_hang` test may see a non-zero exit (Wayland not available in CI) but should complete quickly.

- [ ] **Step 3: Run all tests**

Run: `cargo test`
Expected: all tests pass.

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
git add tests/cli.rs
git commit -m "test: add CLI integration tests for --delay flag"
```

---

### Task 4: Verify full pipeline

- [ ] **Step 1: Clippy**

```bash
cargo clippy -- -D warnings
```
Expected: no warnings.

- [ ] **Step 2: All tests**

```bash
cargo test
```
Expected: all tests pass.

- [ ] **Step 3: Release build**

```bash
cargo build --release
```
Expected: compiles successfully.

- [ ] **Step 4: Manual smoke test**

```bash
./target/release/cosmic-shot --delay abc
```
Expected: exits with error mentioning `--delay`.

```bash
./target/release/cosmic-shot --delay
```
Expected: exits with error mentioning `--delay requires a value`.
