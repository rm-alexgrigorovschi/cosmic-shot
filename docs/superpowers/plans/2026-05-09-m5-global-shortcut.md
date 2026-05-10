# M5 Global Shortcut Integration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make cosmic-shot launchable via a configurable keyboard shortcut in COSMIC DE by shipping a `.desktop` file, an install script, and a `--print-shortcut` CLI flag.

**Architecture:** Add a `shortcut` field to `Config` (documentation-only, not used at runtime). Parse `--print-shortcut` at the top of `main()` before the capture pipeline and exit early. Ship `contrib/cosmic-shot.desktop` and `contrib/install.sh` for one-command installation. No daemon, no auto-registration, no compositor file writes.

**Tech Stack:** Rust `std::env::args()` for CLI parsing; POSIX sh for install script; XDG `.desktop` format.

---

### Task 1: Add `shortcut` field to Config — TDD

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Write the failing tests**

Add these two tests inside the `#[cfg(test)] mod tests` block in `src/config.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
source "$HOME/.cargo/env" && cargo test config::tests::config_default_shortcut config::tests::config_shortcut_parsed_from_toml -- --nocapture
```

Expected: FAIL — `Config` has no `shortcut` field.

- [ ] **Step 3: Add `shortcut` field to `Config`**

In `src/config.rs`, change the `Config` struct and its `Default` impl:

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test config::tests -- --nocapture
```

Expected: all 8 tests pass (6 existing + 2 new).

- [ ] **Step 5: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add src/config.rs
git commit -m "feat: add shortcut field to Config with default Alt+Shift+S"
```

---

### Task 2: Add `--print-shortcut` flag to main — TDD

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Write the integration test**

In `tests/` (alongside existing integration tests), create `tests/cli.rs`:

```rust
use std::process::Command;

#[test]
fn print_shortcut_prints_two_lines_and_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_cosmic-shot"))
        .arg("--print-shortcut")
        .output()
        .expect("failed to run cosmic-shot");

    assert!(
        output.status.success(),
        "exit code was not 0: {:?}",
        output.status
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "expected exactly 2 lines, got: {:?}", lines);
    assert!(
        lines[0].starts_with("Shortcut:"),
        "first line should start with 'Shortcut:': {:?}",
        lines[0]
    );
    assert!(
        lines[1].starts_with("Command:"),
        "second line should start with 'Command:': {:?}",
        lines[1]
    );
    assert!(
        lines[1].contains("cosmic-shot"),
        "command line should contain 'cosmic-shot': {:?}",
        lines[1]
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test --test cli print_shortcut_prints_two_lines_and_exits_zero -- --nocapture
```

Expected: FAIL — cosmic-shot launches the full capture pipeline and hangs (no `--print-shortcut` handling yet). Kill with Ctrl+C after confirming it hangs.

- [ ] **Step 3: Add `--print-shortcut` handling to `main.rs`**

Replace the current content of `src/main.rs` with:

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
    if std::env::args().any(|a| a == "--print-shortcut") {
        let cfg = config::Config::load();
        println!("Shortcut: {}", cfg.shortcut);
        println!("Command:  cosmic-shot");
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("starting cosmic-shot");

    let frames = capture::capture_all_outputs()
        .context("failed to capture outputs")?;

    tracing::info!("captured {} output(s)", frames.len());

    overlay::run(frames).context("overlay error")?;

    Ok(())
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cargo test --test cli print_shortcut_prints_two_lines_and_exits_zero -- --nocapture
```

Expected: PASS. Output should be exactly:
```
Shortcut: Alt+Shift+S
Command:  cosmic-shot
```

- [ ] **Step 5: Run all tests**

```bash
cargo test -- --include-ignored
```

Expected: all existing tests still pass, plus the new integration test.

- [ ] **Step 6: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs tests/cli.rs
git commit -m "feat: add --print-shortcut flag that prints shortcut and command then exits"
```

---

### Task 3: Ship the `.desktop` file

**Files:**
- Create: `contrib/cosmic-shot.desktop`

- [ ] **Step 1: Create the `contrib/` directory and desktop file**

```bash
mkdir -p contrib
```

Create `contrib/cosmic-shot.desktop` with this exact content:

```ini
[Desktop Entry]
Name=cosmic-shot
Comment=Fast native screenshot tool for COSMIC DE
Exec=cosmic-shot
Type=Application
Categories=GNOME;COSMIC;Utility;
Keywords=screenshot;capture;screen;
Icon=camera-photo
Terminal=false
StartupNotify=false
NoDisplay=true
```

- [ ] **Step 2: Verify the desktop file is valid**

```bash
desktop-file-validate contrib/cosmic-shot.desktop
```

Expected: no output (valid). If `desktop-file-validate` is not installed, skip this step and note it in the commit message.

- [ ] **Step 3: Commit**

```bash
git add contrib/cosmic-shot.desktop
git commit -m "feat: add XDG desktop entry for cosmic-shot"
```

---

### Task 4: Ship the install script

**Files:**
- Create: `contrib/install.sh`

- [ ] **Step 1: Create `contrib/install.sh`**

Create `contrib/install.sh` with this exact content:

```sh
#!/bin/sh
# install.sh — install cosmic-shot binary and desktop file
# Usage:
#   ./contrib/install.sh          # user install (default)
#   ./contrib/install.sh --user   # user install
#   ./contrib/install.sh --system # system install (requires sudo)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BINARY="$REPO_ROOT/target/release/cosmic-shot"
DESKTOP="$SCRIPT_DIR/cosmic-shot.desktop"

# Verify the release binary exists.
if [ ! -f "$BINARY" ]; then
    echo "Error: release binary not found at $BINARY"
    echo "Run 'cargo build --release' first."
    exit 1
fi

# Parse install target.
TARGET="${1:---user}"

case "$TARGET" in
    --user)
        BIN_DIR="$HOME/.local/bin"
        DESKTOP_DIR="$HOME/.local/share/applications"
        ;;
    --system)
        BIN_DIR="/usr/local/bin"
        DESKTOP_DIR="/usr/share/applications"
        ;;
    *)
        echo "Usage: $0 [--user|--system]"
        exit 1
        ;;
esac

# Install binary.
mkdir -p "$BIN_DIR"
cp "$BINARY" "$BIN_DIR/cosmic-shot"
chmod +x "$BIN_DIR/cosmic-shot"
echo "Installed binary: $BIN_DIR/cosmic-shot"

# Install desktop file.
mkdir -p "$DESKTOP_DIR"
cp "$DESKTOP" "$DESKTOP_DIR/cosmic-shot.desktop"
echo "Installed desktop file: $DESKTOP_DIR/cosmic-shot.desktop"

# Refresh desktop database (non-fatal if not available).
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

# Read configured shortcut (non-fatal if cosmic-shot not yet on PATH).
SHORTCUT="Alt+Shift+S"
if command -v cosmic-shot >/dev/null 2>&1; then
    SHORTCUT="$(cosmic-shot --print-shortcut 2>/dev/null | grep '^Shortcut:' | sed 's/Shortcut: //')" || true
fi

echo ""
echo "cosmic-shot installed successfully."
echo ""
echo "To add a keyboard shortcut in COSMIC:"
echo "  1. Open Settings → Keyboard → Shortcuts → Custom Shortcuts"
echo "  2. Click '+'"
echo "  3. Name:     cosmic-shot"
echo "  4. Command:  cosmic-shot"
echo "  5. Shortcut: $SHORTCUT"
echo ""
echo "To change the shortcut, edit ~/.config/cosmic-shot/config.toml:"
echo "  shortcut = \"$SHORTCUT\""
echo ""
echo "Run 'cosmic-shot --print-shortcut' at any time to see your configured shortcut."
```

- [ ] **Step 2: Make it executable**

```bash
chmod +x contrib/install.sh
```

- [ ] **Step 3: Dry-run the user install to verify it works**

```bash
# Build release binary first if needed
source "$HOME/.cargo/env" && cargo build --release

# Run the install script (user mode — safe, no sudo)
./contrib/install.sh --user
```

Expected output (approximate):
```
Installed binary: /home/<user>/.local/bin/cosmic-shot
Installed desktop file: /home/<user>/.local/share/applications/cosmic-shot.desktop

cosmic-shot installed successfully.

To add a keyboard shortcut in COSMIC:
  1. Open Settings → Keyboard → Shortcuts → Custom Shortcuts
  2. Click '+'
  3. Name:     cosmic-shot
  4. Command:  cosmic-shot
  5. Shortcut: Alt+Shift+S
...
```

- [ ] **Step 4: Verify the installed binary works**

```bash
~/.local/bin/cosmic-shot --print-shortcut
```

Expected:
```
Shortcut: Alt+Shift+S
Command:  cosmic-shot
```

- [ ] **Step 5: Commit**

```bash
git add contrib/install.sh
git commit -m "feat: add install.sh for user and system installation with shortcut instructions"
```

---

### Task 5: Manual end-to-end verification

- [ ] **Step 1: Verify `--print-shortcut` with custom config**

Create a temporary config:

```bash
mkdir -p ~/.config/cosmic-shot
cat > ~/.config/cosmic-shot/config.toml << 'EOF'
save_dir = "~/Pictures/cosmic-shot"
shortcut = "Super+Shift+S"
EOF
```

Run:
```bash
./target/release/cosmic-shot --print-shortcut
```

Expected:
```
Shortcut: Super+Shift+S
Command:  cosmic-shot
```

- [ ] **Step 2: Restore default config**

```bash
cat > ~/.config/cosmic-shot/config.toml << 'EOF'
save_dir = "~/Pictures/cosmic-shot"
shortcut = "Alt+Shift+S"
EOF
```

- [ ] **Step 3: Register the shortcut in COSMIC Settings**

1. Open COSMIC Settings → Keyboard → Shortcuts → Custom Shortcuts
2. Click `+`
3. Name: `cosmic-shot`
4. Command: `cosmic-shot` (or full path `~/.local/bin/cosmic-shot`)
5. Shortcut: `Alt+Shift+S`
6. Press `Alt+Shift+S` — overlay should appear immediately

- [ ] **Step 4: Confirm existing functionality still works**

Run: `cargo test -- --include-ignored`

Expected: all tests pass.

---

### Task 6: Update design doc with implementation notes

**Files:**
- Modify: `docs/superpowers/specs/2026-05-09-m5-global-shortcut-design.md`

- [ ] **Step 1: Append implementation notes**

Add an `## Implementation Notes` section documenting any deviations from the spec discovered during implementation (e.g. whether `desktop-file-validate` was available, any shell portability issues in `install.sh`).

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/specs/2026-05-09-m5-global-shortcut-design.md
git commit -m "docs: update M5 spec with implementation notes"
```
