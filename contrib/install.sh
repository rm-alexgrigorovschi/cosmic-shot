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
