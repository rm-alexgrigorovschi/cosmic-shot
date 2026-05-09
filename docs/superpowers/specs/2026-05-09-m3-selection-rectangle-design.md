# M3: Selection Rectangle — Design Spec

Date: 2026-05-09

## Goal

Add an interactive selection rectangle to the fullscreen overlay:
- Click-drag to draw a selection on the frozen frame
- Dashed white border, corner handles, size label; whole overlay slightly dimmed (~35% alpha)
- Mouse release shows a floating placeholder toolbar (no actions yet — wired in M4)
- Escape clears selection → resets to Idle; Escape from Idle closes the overlay

## Decisions

| Question | Decision |
|---|---|
| Interaction model | Click-drag to draw |
| Visual style | Slight dim (35% black) over whole overlay; dashed white border + corner handles + size label on rect |
| Confirmation | Mouse release → floating toolbar near selection |
| Toolbar actions | Placeholder only (greyed-out "Copy" and "Save" labels) — M4 wires real export |
| Implementation | iced event subscription + iced `canvas` widget |

## Architecture

### State Machine

```
Idle            — crosshair cursor, waiting for click
Drawing(start)  — mouse held, rect growing as cursor moves  
Selected(rect)  — mouse released, toolbar visible
```

Transitions:

| State    | Message          | Next State                            |
|---|---|---|
| `Idle`     | `MousePressed`     | `Drawing { start: cursor_pos }`          |
| `Drawing`  | `CursorMoved(p)`   | `Drawing { start }` + update cursor_pos |
| `Drawing`  | `MouseReleased(p)` | `Selected { rect: normalize(start, p) }` |
| `Selected` | `EscapePressed`    | `Idle`                                  |
| `Drawing`  | `EscapePressed`    | `Idle`                                  |
| `Idle`     | `EscapePressed`    | emits `Close` → `iced::exit()`          |

`normalize(a, b)` produces a `Rectangle` with origin always at top-left regardless of drag direction.

### Module Changes

**`src/overlay/mod.rs`** — extended (not replaced):
- `OverlayState` gains `selection: SelectionState` and `cursor_pos: Point`
- `Message` gains `CursorMoved(Point)`, `MousePressed`, `MouseReleased`, `EscapePressed`, `ResetSelection`
- `subscription()` adds `iced::event::listen_with` for mouse events alongside existing keyboard handler
- `overlay_view()` upgraded to stack `image::Image` + `Canvas<SelectionCanvas>`
- New private type `SelectionCanvas` implements `iced::widget::canvas::Program`

**`src/overlay/selection.rs`** — new file:
- `SelectionState` enum
- `normalize_rect(a: Point, b: Point) -> Rectangle`
- Pure logic, no iced runtime dependency — fully unit-testable

**`Cargo.toml`**:
- Add `canvas` to iced features: `features = ["image", "canvas"]`

### View Stack

```
container (fill)
  └─ stack![
       image::Image(frozen frame),     // bottom layer
       canvas(SelectionCanvas),        // top layer: dim + rect + toolbar
     ]
```

### Canvas Drawing

In `SelectionCanvas::draw()`:

1. **Dim layer** — `frame.fill_rectangle(frame.center(), frame.size(), Color { a: 0.35, ..Color::BLACK })`
2. **Selection rect** (when `Drawing` or `Selected`):
   - Dashed white stroke border (`LineDash` with segment + gap)
   - Four 5×5 white filled corner handle squares
   - Size label via `frame.fill_text` above top-left corner: `"{w} × {h}"`
3. **Toolbar** (when `Selected`):
   - Toolbar appears 8px below the selection rect; if that would clip outside the overlay bounds, it appears 8px above instead
   - Two greyed-out placeholder buttons: "Copy" and "Save"
   - No click handlers in M3

### Cursor

`canvas::Program::mouse_interaction()` returns:
- `mouse::Interaction::Crosshair` — `Idle` and `Drawing`
- `mouse::Interaction::Default` — `Selected`

## Dependencies

- Add `canvas` feature to `iced` in `Cargo.toml` — no new crates needed

## Error Handling

- Zero-size drag (click without moving): `normalize` clamps to minimum 1×1 rect
- No output frames: early `Ok(())` return unchanged from M2

## Testing Strategy (TDD)

### Unit tests — `src/overlay/selection.rs`

| Test | What it checks |
|---|---|
| `selection_idle_to_drawing` | `MousePressed` in `Idle` → `Drawing { start }` |
| `selection_drawing_to_selected` | `MouseReleased` in `Drawing` → `Selected { rect }` |
| `selection_escape_from_selected` | `EscapePressed` in `Selected` → `Idle` |
| `selection_escape_from_idle` | `EscapePressed` in `Idle` → signals close |
| `normalize_rect_top_left_drag` | drag from top-left to bottom-right → correct rect |
| `normalize_rect_bottom_right_drag` | drag from bottom-right to top-left → correct rect |
| `normalize_rect_other_diagonals` | top-right→bottom-left and bottom-left→top-right |
| `normalize_rect_minimum_size` | zero-distance drag → 1×1 rect |

### What is NOT tested

- Canvas pixel output (visual correctness)
- Toolbar button layout (no click handlers yet)
- Cursor shape (iced runtime behaviour)

## Out of Scope for M3

- Toolbar button actions (M4)
- Clipboard copy (M4)
- File save (M4)
- Per-output frame mapping (deferred from M2)
- Resize handles after selection (drag corners to adjust — M3+ if needed)

## Implementation Notes

### Deviations from design

**Per-window vs shared selection:** The design spec said one global `SelectionState`. During implementation we went through two iterations:
1. First attempt: per-window independent selection (`HashMap<window::Id, PerWindowState>`) — caused rectangle to appear on all screens simultaneously. Wrong.
2. Second attempt: per-window independent but gated — caused independent rectangles on each screen. Not the desired UX.
3. Final approach: single global `selection` + `cursor_pos` + `active_window: Option<window::Id>`. The window that receives `MousePressed` becomes the active owner; canvas only draws the rect on the active surface. This matches the "one rect across all screens" UX.

**Per-output frame mapping (M2 debt resolved in M3):** Each surface now shows its own frozen frame. `OverlayState` holds `frames: Vec<image::Handle>` and `window_frame_idx: HashMap<window::Id, usize>`. Frame indices are assigned in first-seen order on first `CursorMoved` per window.

**Click-to-redraw:** `MousePressed` always starts a new `Drawing` state regardless of current state — no need to Escape first to reset.

**Size label:** Fixed-width pill (90px) anchored to top-left corner of selection, bold 13px black text centred inside white background. `horizontal_alignment: Center` + `vertical_alignment: Center` used for correct centering.

**`canvas::Text` font weight:** `iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }` works in iced 0.13.1.
