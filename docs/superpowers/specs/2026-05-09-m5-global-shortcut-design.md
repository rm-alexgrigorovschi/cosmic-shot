# M5 Design: Global Shortcut Integration

Make cosmic-shot launchable via a keyboard shortcut in COSMIC DE. No daemon,
no auto-registration, no compositor file writes. Ship a `.desktop` file and
an install script; print clear instructions so the user can register the
shortcut manually in COSMIC Settings.

## Scope

1. XDG `.desktop` file so the desktop environment knows about cosmic-shot
2. `contrib/install.sh` for one-command installation to user or system paths
3. `shortcut` field in `config.toml` so the user can change the key binding
4. `--print-shortcut` CLI flag to print the configured shortcut on demand

Out of scope: auto-writing COSMIC compositor config, daemon mode, DBus
integration, Flatpak.

## Desktop File

Installed to `~/.local/share/applications/` (user) or `/usr/share/applications/`
(system). Ships in the repo at `contrib/cosmic-shot.desktop`.

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

`NoDisplay=true` hides it from the app launcher — it's a background tool
invoked via shortcut, not searched for by name.

`StartupNotify=false` because the overlay appears and disappears in under a
second; a startup spinner would be jarring.

## Config: `shortcut` Field

Add to `~/.config/cosmic-shot/config.toml`:

```toml
# Directory where screenshots are saved
save_dir = "~/Pictures/cosmic-shot"

# Keyboard shortcut shown in --print-shortcut instructions.
# Format: human-readable, e.g. "Alt+Shift+S", "Super+Shift+S", "Print"
# Change this if you register a different shortcut in COSMIC Settings.
shortcut = "Alt+Shift+S"
```

Default value: `"Alt+Shift+S"` — matches the Flameshot shortcut convention
already present on this machine, making migration natural.

cosmic-shot does not read this field at runtime for any functional purpose.
It is stored solely so `--print-shortcut` and `install.sh` can show the user
the correct shortcut to type into COSMIC Settings.

## `--print-shortcut` Subcommand

```
$ cosmic-shot --print-shortcut
Shortcut: Alt+Shift+S
Command:  cosmic-shot
```

Reads `Config::load()`, prints `config.shortcut` and the literal string
`"cosmic-shot"`. Exits with code 0. Does not launch the capture pipeline.

Implementation: parse `--print-shortcut` in `main.rs` before calling
`capture_all_outputs()`; print and return `Ok(())`.

## Install Script

`contrib/install.sh` — POSIX sh, no external dependencies beyond `cp`/`mkdir`.

### Usage

```sh
./contrib/install.sh          # installs to ~/.local/bin + ~/.local/share
./contrib/install.sh --user   # same as default
./contrib/install.sh --system # installs to /usr/local/bin + /usr/share (needs sudo)
```

### User install (`--user`, default)

```sh
mkdir -p ~/.local/bin
cp target/release/cosmic-shot ~/.local/bin/cosmic-shot
chmod +x ~/.local/bin/cosmic-shot

mkdir -p ~/.local/share/applications
cp contrib/cosmic-shot.desktop ~/.local/share/applications/
update-desktop-database ~/.local/share/applications/ 2>/dev/null || true
```

### System install (`--system`)

```sh
cp target/release/cosmic-shot /usr/local/bin/cosmic-shot
chmod +x /usr/local/bin/cosmic-shot

cp contrib/cosmic-shot.desktop /usr/share/applications/
update-desktop-database /usr/share/applications/ 2>/dev/null || true
```

### Instructions printed after both installs

```
cosmic-shot installed successfully.

To add a keyboard shortcut in COSMIC:
  1. Open Settings → Keyboard → Shortcuts → Custom Shortcuts
  2. Click "+"
  3. Name:     cosmic-shot
  4. Command:  cosmic-shot
  5. Shortcut: <YOUR_SHORTCUT>   ← run 'cosmic-shot --print-shortcut' to see this

To change the shortcut, edit ~/.config/cosmic-shot/config.toml:
  shortcut = "Alt+Shift+S"
```

`<YOUR_SHORTCUT>` is filled at runtime by calling `cosmic-shot --print-shortcut`
inside the script, or by reading `config.toml` directly with `grep`.

## Config Struct Change

In `src/config.rs`, add one field to `Config`:

```rust
/// Human-readable keyboard shortcut shown in --print-shortcut output.
/// Not used at runtime — for documentation purposes only.
pub shortcut: String,
```

`Default` impl returns `"Alt+Shift+S".to_string()`.

Serde `#[serde(default)]` already on the struct means existing config files
without `shortcut` get the default automatically.

## CLI Argument Parsing

No external arg-parsing crate needed. `std::env::args()` is sufficient for
one flag:

```rust
if std::env::args().any(|a| a == "--print-shortcut") {
    let cfg = Config::load();
    println!("Shortcut: {}", cfg.shortcut);
    println!("Command:  cosmic-shot");
    return Ok(());
}
```

Placed at the top of `main()`, before the capture pipeline.

## Files

| File                          | Change                                             |
|-------------------------------|----------------------------------------------------|
| `contrib/cosmic-shot.desktop` | New — XDG desktop entry                            |
| `contrib/install.sh`          | New — install script (user + system targets)       |
| `src/config.rs`               | Add `shortcut: String` field, default `"Alt+Shift+S"` |
| `src/main.rs`                 | Parse `--print-shortcut`, print and exit            |

## Testing

| Test                              | Location    | Asserts                                                  |
|-----------------------------------|-------------|----------------------------------------------------------|
| `config_default_shortcut`         | `config.rs` | `Config::default().shortcut == "Alt+Shift+S"`            |
| `config_shortcut_parsed_from_toml`| `config.rs` | Custom `shortcut` value survives round-trip through toml |
| `print_shortcut_flag_exits_cleanly` | `main.rs` integration test | `--print-shortcut` prints two lines and exits 0 |

Install script correctness is verified manually (copies correct files, prints
instructions). No bats test suite for M5 — manual verification in Task 8.

## Implementation Notes

### Deviations from design

**Desktop file categories fixed by desktop-file-validate**
The spec listed `Categories=GNOME;COSMIC;Utility;` but `GNOME` and `COSMIC` are unregistered categories that fail XDG validation. Fixed to `Categories=Utility;X-COSMIC;` per the XDG spec requirement that unregistered categories use the `X-` prefix.

**install.sh: empty shortcut guard added**
After code review, added `SHORTCUT="${SHORTCUT:-Alt+Shift+S}"` after the `--print-shortcut` subprocess call to guard against an empty string if the output format ever changes.

**Frame assignment bug fixed during M5 verification**
During end-to-end testing, a pre-existing M2 bug was discovered: each overlay surface showed frame 0 (main screen capture) on all screens until the cursor moved there, because frame indices were assigned lazily on first `CursorMoved`. Fixed by moving assignment to the window creation callback (`|state, id|`), which fires immediately when each layer-shell surface is created. Each screen now shows its own correct frozen frame from the moment the overlay appears.

### What was verified

- `cosmic-shot --print-shortcut` prints correct shortcut and exits 0
- `./contrib/install.sh --user` installs binary + desktop file, prints COSMIC shortcut registration instructions
- `Alt+Shift+S` shortcut registered in COSMIC Settings → Keyboard → Custom Shortcuts
- Pressing `Alt+Shift+S` launches the overlay immediately from any application
- Both screens show their own frozen/dimmed frame (correct by design)
- Selection, Copy, and Save all work correctly when launched via shortcut
