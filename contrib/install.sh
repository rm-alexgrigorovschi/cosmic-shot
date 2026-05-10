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
