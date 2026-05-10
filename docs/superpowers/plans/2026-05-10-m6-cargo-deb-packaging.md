# M6: cargo-deb Packaging & CI Release — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Package cosmic-shot as a `.deb` and tarball, with a GitHub Actions workflow that builds and publishes release artifacts on tag push, and an updated `install.sh` that downloads binaries from GitHub Releases.

**Architecture:** Three independent deliverables — `cargo-deb` metadata in `Cargo.toml`, a GitHub Actions release workflow, and an updated `install.sh`. No new Rust code; this is all config/scripting.

**Tech Stack:** `cargo-deb`, GitHub Actions, POSIX shell, `softprops/action-gh-release@v2`, `dtolnay/rust-toolchain`

---

### Task 1: Add cargo-deb metadata to Cargo.toml

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add `authors` field to `[package]`**

Add an `authors` field so `cargo-deb` can populate the Maintainer field in the `.deb`:

```toml
# In [package], after the `license = "MIT"` line, add:
authors = ["Alex G <alex@example.com>"]
```

(Use your real name/email.)

- [ ] **Step 2: Add `[package.metadata.deb]` section**

Append this to the end of `Cargo.toml`:

```toml
[package.metadata.deb]
section = "x11"
priority = "optional"
depends = "libwayland-client0, wl-clipboard"
assets = [
    ["target/release/cosmic-shot", "/usr/bin/cosmic-shot", "755"],
    ["contrib/cosmic-shot.desktop", "/usr/share/applications/cosmic-shot.desktop", "644"],
]
```

- [ ] **Step 3: Install cargo-deb and test locally**

Run:
```bash
cargo install cargo-deb
cargo build --release
cargo deb --no-build
```

Expected: a file at `target/debian/cosmic-shot_0.1.0-1_amd64.deb` is created without errors.

- [ ] **Step 4: Inspect the .deb**

Run:
```bash
dpkg-deb --info target/debian/cosmic-shot_0.1.0-1_amd64.deb
dpkg-deb --contents target/debian/cosmic-shot_0.1.0-1_amd64.deb
```

Expected: `--info` shows Section `x11`, Depends includes `libwayland-client0, wl-clipboard`. `--contents` shows `/usr/bin/cosmic-shot` and `/usr/share/applications/cosmic-shot.desktop`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml
git commit -m "feat(packaging): add cargo-deb metadata for .deb generation"
```

---

### Task 2: Create GitHub Actions release workflow

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Create the workflows directory**

```bash
mkdir -p .github/workflows
```

- [ ] **Step 2: Write the release workflow**

Create `.github/workflows/release.yml` with:

```yaml
name: Release

on:
  push:
    tags: ['v*']

permissions:
  contents: write

env:
  CARGO_TERM_COLOR: always

jobs:
  release:
    name: Build and Release
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Extract version from tag
        id: version
        run: |
          TAG="${GITHUB_REF#refs/tags/v}"
          echo "version=$TAG" >> "$GITHUB_OUTPUT"

      - name: Validate version matches Cargo.toml
        run: |
          CARGO_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
          TAG_VERSION="${{ steps.version.outputs.version }}"
          if [ "$CARGO_VERSION" != "$TAG_VERSION" ]; then
            echo "ERROR: Tag version ($TAG_VERSION) does not match Cargo.toml version ($CARGO_VERSION)"
            exit 1
          fi

      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libwayland-dev \
            libxkbcommon-dev \
            libvulkan-dev \
            pkg-config

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Test
        run: cargo test

      - name: Build release binary
        run: cargo build --release

      - name: Build .deb package
        run: |
          cargo install cargo-deb
          cargo deb --no-build

      - name: Create tarball
        run: |
          VERSION="${{ steps.version.outputs.version }}"
          TARBALL="cosmic-shot-${VERSION}-x86_64-linux.tar.gz"
          mkdir -p staging
          cp target/release/cosmic-shot staging/
          cp contrib/cosmic-shot.desktop staging/
          tar -czf "$TARBALL" -C staging .
          echo "TARBALL=$TARBALL" >> "$GITHUB_ENV"

      - name: Find .deb file
        run: |
          DEB=$(ls target/debian/*.deb)
          echo "DEB=$DEB" >> "$GITHUB_ENV"

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          generate_release_notes: true
          files: |
            ${{ env.DEB }}
            ${{ env.TARBALL }}
```

- [ ] **Step 3: Validate the YAML syntax**

Run:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"
```

If `pyyaml` isn't installed, use:
```bash
python3 -c "import json; print('YAML file exists')" && cat .github/workflows/release.yml | head -5
```

Expected: no syntax errors.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add GitHub Actions release workflow for tag-triggered builds"
```

---

### Task 3: Rewrite install.sh with remote download support

**Files:**
- Modify: `contrib/install.sh`

- [ ] **Step 1: Rewrite install.sh**

Replace the contents of `contrib/install.sh` with:

```sh
#!/bin/sh
# install.sh — install cosmic-shot binary and desktop file
#
# Usage:
#   curl -sSL https://raw.githubusercontent.com/OWNER/cosmic-shot/main/contrib/install.sh | sh
#   ./contrib/install.sh              # remote install, user mode (default)
#   ./contrib/install.sh --user       # remote install, user mode
#   ./contrib/install.sh --system     # remote install, system mode (requires sudo)
#   ./contrib/install.sh --local      # local install from build tree

set -e

# ── Configuration ──────────────────────────────────────────────────────
OWNER="OWNER"
REPO="cosmic-shot"
GITHUB_API="https://api.github.com/repos/${OWNER}/${REPO}/releases/latest"
# ───────────────────────────────────────────────────────────────────────

MODE="remote"
TARGET="user"

# ── Parse arguments ────────────────────────────────────────────────────
for arg in "$@"; do
    case "$arg" in
        --user)   TARGET="user" ;;
        --system) TARGET="system" ;;
        --local)  MODE="local" ;;
        --help|-h)
            echo "Usage: $0 [--user|--system] [--local]"
            echo ""
            echo "  --user    Install to ~/.local/bin (default)"
            echo "  --system  Install to /usr/local/bin (requires sudo)"
            echo "  --local   Use local build instead of downloading from GitHub"
            exit 0
            ;;
        *)
            echo "Unknown option: $arg"
            echo "Usage: $0 [--user|--system] [--local]"
            exit 1
            ;;
    esac
done

# ── Set install directories ───────────────────────────────────────────
case "$TARGET" in
    user)
        BIN_DIR="$HOME/.local/bin"
        DESKTOP_DIR="$HOME/.local/share/applications"
        ;;
    system)
        BIN_DIR="/usr/local/bin"
        DESKTOP_DIR="/usr/share/applications"
        ;;
esac

# ── Acquire binary and desktop file ───────────────────────────────────
if [ "$MODE" = "local" ]; then
    # Local mode: use build tree.
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
    BINARY="$REPO_ROOT/target/release/cosmic-shot"
    DESKTOP="$SCRIPT_DIR/cosmic-shot.desktop"

    if [ ! -f "$BINARY" ]; then
        echo "Error: release binary not found at $BINARY"
        echo "Run 'cargo build --release' first."
        exit 1
    fi
else
    # Remote mode: download from GitHub Releases.
    if ! command -v curl >/dev/null 2>&1; then
        echo "Error: curl is required but not installed."
        exit 1
    fi

    echo "Fetching latest release from GitHub..."
    RELEASE_JSON=$(curl -sSL "$GITHUB_API")

    # Extract tarball URL (x86_64-linux.tar.gz) without jq.
    TARBALL_URL=$(echo "$RELEASE_JSON" | grep -o '"browser_download_url": *"[^"]*x86_64-linux\.tar\.gz"' | head -1 | sed 's/.*"\(https[^"]*\)"/\1/')

    if [ -z "$TARBALL_URL" ]; then
        echo "Error: could not find x86_64-linux tarball in latest release."
        echo "Check https://github.com/${OWNER}/${REPO}/releases for available downloads."
        exit 1
    fi

    TAG_NAME=$(echo "$RELEASE_JSON" | grep -o '"tag_name": *"[^"]*"' | head -1 | sed 's/.*"\([^"]*\)"/\1/')
    echo "Downloading ${REPO} ${TAG_NAME}..."

    TMPDIR=$(mktemp -d)
    trap 'rm -rf "$TMPDIR"' EXIT

    curl -sSL "$TARBALL_URL" | tar -xz -C "$TMPDIR"

    BINARY="$TMPDIR/cosmic-shot"
    DESKTOP="$TMPDIR/cosmic-shot.desktop"

    if [ ! -f "$BINARY" ]; then
        echo "Error: binary not found in downloaded tarball."
        exit 1
    fi
fi

# ── Install ────────────────────────────────────────────────────────────
mkdir -p "$BIN_DIR"
cp "$BINARY" "$BIN_DIR/cosmic-shot"
chmod +x "$BIN_DIR/cosmic-shot"
echo "Installed binary: $BIN_DIR/cosmic-shot"

mkdir -p "$DESKTOP_DIR"
cp "$DESKTOP" "$DESKTOP_DIR/cosmic-shot.desktop"
echo "Installed desktop file: $DESKTOP_DIR/cosmic-shot.desktop"

# Refresh desktop database (non-fatal).
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

# ── PATH check ─────────────────────────────────────────────────────────
if [ "$TARGET" = "user" ]; then
    case ":$PATH:" in
        *":$BIN_DIR:"*) ;;
        *)
            echo ""
            echo "WARNING: $BIN_DIR is not in your PATH."
            echo "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
            echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
            ;;
    esac
fi

# ── Shortcut instructions ─────────────────────────────────────────────
SHORTCUT="Alt+Shift+S"
if command -v cosmic-shot >/dev/null 2>&1; then
    SHORTCUT="$(cosmic-shot --print-shortcut 2>/dev/null | grep '^Shortcut:' | sed 's/Shortcut: //')" || true
    SHORTCUT="${SHORTCUT:-Alt+Shift+S}"
fi

echo ""
echo "cosmic-shot installed successfully."
echo ""
echo "To add a keyboard shortcut in COSMIC:"
echo "  1. Open Settings -> Keyboard -> Shortcuts -> Custom Shortcuts"
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

- [ ] **Step 2: Test local mode**

Run:
```bash
chmod +x contrib/install.sh
./contrib/install.sh --local --user
```

Expected: same behavior as the old script — installs binary and desktop file to `~/.local/bin` and `~/.local/share/applications`.

- [ ] **Step 3: Test --help**

Run:
```bash
./contrib/install.sh --help
```

Expected: prints usage information and exits 0.

- [ ] **Step 4: Test error on unknown flag**

Run:
```bash
./contrib/install.sh --bogus 2>&1; echo "exit: $?"
```

Expected: prints "Unknown option" and exits 1.

- [ ] **Step 5: Commit**

```bash
git add contrib/install.sh
git commit -m "feat(install): add remote download mode to install.sh"
```

---

### Task 4: Set git remote and update OWNER placeholder

**Files:**
- Modify: `contrib/install.sh` (OWNER variable)

- [ ] **Step 1: Add the GitHub remote**

```bash
git remote add origin git@github.com:OWNER/cosmic-shot.git
```

(Replace `OWNER` with the actual GitHub username or org.)

- [ ] **Step 2: Update OWNER in install.sh**

Edit `contrib/install.sh` and replace:
```sh
OWNER="OWNER"
```
with the actual GitHub username/org.

- [ ] **Step 3: Commit**

```bash
git add contrib/install.sh
git commit -m "chore: set GitHub remote and update install.sh repo owner"
```

---

### Task 5: Verify full pipeline locally

- [ ] **Step 1: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: no warnings.

- [ ] **Step 2: Run tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 3: Build release and generate .deb**

```bash
cargo build --release
cargo deb --no-build
```

Expected: `.deb` file created at `target/debian/cosmic-shot_0.1.0-1_amd64.deb`.

- [ ] **Step 4: Inspect .deb contents**

```bash
dpkg-deb --contents target/debian/cosmic-shot_0.1.0-1_amd64.deb
```

Expected: lists `/usr/bin/cosmic-shot` and `/usr/share/applications/cosmic-shot.desktop`.

- [ ] **Step 5: Test local install from .deb (optional, if on Debian/Ubuntu)**

```bash
sudo dpkg -i target/debian/cosmic-shot_0.1.0-1_amd64.deb
which cosmic-shot
cosmic-shot --print-shortcut
sudo dpkg -r cosmic-shot
```

Expected: installs, runs, and removes cleanly.

---

### Task 6: Push and test release pipeline

- [ ] **Step 1: Push main branch**

```bash
git push -u origin main
```

- [ ] **Step 2: Tag a test release**

```bash
git tag v0.1.0
git push origin v0.1.0
```

- [ ] **Step 3: Monitor GitHub Actions**

Go to `https://github.com/OWNER/cosmic-shot/actions` and watch the Release workflow. Expected: all steps pass, GitHub Release created with `.deb` and `.tar.gz` attached.

- [ ] **Step 4: Test remote install.sh**

After the release is published:
```bash
curl -sSL https://raw.githubusercontent.com/OWNER/cosmic-shot/main/contrib/install.sh | sh
```

Expected: downloads latest tarball, installs binary and desktop file to `~/.local/bin`.

- [ ] **Step 5: Verify installed version**

```bash
cosmic-shot --print-shortcut
```

Expected: prints the configured shortcut, confirming the binary works.
