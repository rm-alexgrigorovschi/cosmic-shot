# M6: cargo-deb Packaging & CI Release

## Overview

Package cosmic-shot for distribution via `.deb` and tarball, with a GitHub Actions
workflow that builds and publishes release artifacts on tag push. Update the
existing `install.sh` to download binaries from GitHub Releases.

## Deliverables

1. `[package.metadata.deb]` in `Cargo.toml`
2. `.github/workflows/release.yml`
3. Updated `contrib/install.sh` (remote download mode)

---

## 1. cargo-deb Configuration

Add `[package.metadata.deb]` to `Cargo.toml`:

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

- **Section:** `x11` — standard for screenshot/display tools, including Wayland ones.
- **Depends:** `libwayland-client0` (Wayland runtime lib) and `wl-clipboard` (provides
  `wl-copy`, used for clipboard export). These are the only runtime dependencies;
  all other deps are statically linked by Rust.
- **Assets:** the binary and the `.desktop` file. No man page, no systemd units,
  no postinst/prerm scripts.
- **Maintainer:** pulled from `Cargo.toml` `authors` field (must be added if missing).

### What cargo-deb produces

`cargo deb --no-build` (after `cargo build --release`) generates:
`target/debian/cosmic-shot_0.1.0-1_amd64.deb`

The `.deb` installs:
- `/usr/bin/cosmic-shot`
- `/usr/share/applications/cosmic-shot.desktop`

## 2. GitHub Actions Release Workflow

Single file: `.github/workflows/release.yml`

### Trigger

```yaml
on:
  push:
    tags: ['v*']
```

Tag-push only. No PR checks, no main-branch builds (can add later).

### Runner

`ubuntu-latest` (x86_64 only).

### Steps

1. **Checkout** — `actions/checkout@v4`
2. **Install system deps** — `apt-get install` for Wayland/build headers:
   `libwayland-dev`, `libxkbcommon-dev`, `libvulkan-dev`, `pkg-config`,
   and any other headers the build requires (determined during implementation).
3. **Install Rust toolchain** — `dtolnay/rust-toolchain@stable`
4. **Clippy gate** — `cargo clippy -- -D warnings`
5. **Test gate** — `cargo test`
6. **Release build** — `cargo build --release`
7. **Build .deb** — `cargo install cargo-deb && cargo deb --no-build`
8. **Create tarball** — `cosmic-shot-${VERSION}-x86_64-linux.tar.gz` containing:
   - `cosmic-shot` (binary)
   - `contrib/cosmic-shot.desktop`
9. **Create GitHub Release** — `softprops/action-gh-release@v2` with:
   - The `.deb` file
   - The `.tar.gz` tarball
   - Auto-generated release notes from tag

### Version Validation

The tag name (e.g. `v0.1.0`) is stripped of the `v` prefix and compared to the
version in `Cargo.toml`. The workflow fails if they don't match, preventing
accidental version mismatches.

## 3. Updated install.sh

Rewrite `contrib/install.sh` to support remote download from GitHub Releases.

### Modes

- **Remote mode (default):** downloads the latest release tarball from GitHub.
- **Local mode (`--local`):** copies from a local build (current behavior preserved).

### Flags

| Flag | Description |
|------|-------------|
| `--user` | Install to `~/.local/bin` + `~/.local/share/applications` (default) |
| `--system` | Install to `/usr/local/bin` + `/usr/share/applications` (needs sudo) |
| `--local` | Use local build instead of downloading from GitHub |

### Remote Download Flow

1. Query GitHub API: `https://api.github.com/repos/OWNER/REPO/releases/latest`
2. Parse the tarball URL from the response (grep/sed, no `jq` dependency)
3. Download tarball via `curl`
4. Extract to temp directory
5. Copy binary and `.desktop` file to target locations
6. Verify `~/.local/bin` is in `$PATH`; warn if not
7. Print shortcut setup instructions

### Script Requirements

- POSIX shell (`#!/bin/sh`), no bashisms
- Dependencies: `curl`, `tar` (ubiquitous on Linux)
- Repo coordinates hardcoded as variables at top: `OWNER` / `REPO` (set during implementation to match the actual GitHub repository)
- Graceful error handling: network failures, missing tools, permission errors

## Out of Scope

- APT repository via GitHub Pages (deferred per SPEC.md)
- aarch64 cross-compilation
- PR/main-branch CI checks
- Man page
- Flatpak/Snap/Nix packaging
- Automated changelog generation

## Dependencies

No new Rust crate dependencies. `cargo-deb` is a build-time tool installed in CI.

## Testing

- `cargo deb --no-build` tested locally before merging
- Tag a test release (`v0.0.0-test`) to validate the full CI pipeline
- `install.sh --local` mode tested locally
- Remote mode tested after first real release
