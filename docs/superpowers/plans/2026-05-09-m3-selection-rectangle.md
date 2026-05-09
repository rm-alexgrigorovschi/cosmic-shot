# M3: Selection Rectangle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an interactive click-drag selection rectangle to the fullscreen overlay, with a dimmed background, dashed white border, size label, corner handles, and a placeholder toolbar that appears after mouse release.

**Architecture:** Extend `OverlayState` with a `SelectionState` machine; add `src/overlay/selection.rs` for pure logic (state transitions + rect normalisation); upgrade the overlay view to stack a `Canvas` widget on top of the frozen frame image; subscribe to mouse events via `iced::event::listen_with`; handle Escape context-sensitively (clear selection → Idle, then exit).

**Tech Stack:** `iced 0.13.1` with `canvas` feature, `iced::widget::{canvas, stack}`, `iced::event::listen_with`, `iced::mouse`, existing `iced_layershell` daemon pattern.

---

## File Map

| File | Change |
|---|---|
| `Cargo.toml` | Add `canvas` to iced features |
| `src/overlay/selection.rs` | New — `SelectionState`, `normalize_rect`, pure logic + unit tests |
| `src/overlay/mod.rs` | Extend — new messages, state fields, subscription, canvas view |

---

## Task 1: Add `canvas` feature to iced

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add canvas feature**

In `Cargo.toml`, change:
```toml
iced = { version = "0.13.1", features = ["image"] }
```
to:
```toml
iced = { version = "0.13.1", features = ["image", "canvas"] }
```

- [ ] **Step 2: Verify build**

```bash
source "$HOME/.cargo/env" && cargo build 2>&1
```

Expected: builds cleanly. `iced::widget::canvas::Canvas` and `iced::widget::stack` are now available.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add canvas feature to iced for M3 selection rectangle"
```

---

## Task 2: `SelectionState` and `normalize_rect` (TDD)

**Files:**
- Create: `src/overlay/selection.rs`
- Modify: `src/overlay/mod.rs` (add `mod selection;`)

- [ ] **Step 1: Write the failing tests first**

Create `src/overlay/selection.rs` with tests only:

```rust
use iced::Point;
use iced::Rectangle;

/// Which phase of the selection interaction we are in.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectionState {
    /// Waiting for the user to click. Crosshair cursor shown.
    Idle,
    /// User is holding the mouse button. Rect grows as cursor moves.
    Drawing { start: Point },
    /// User released the mouse. Toolbar is visible.
    Selected { rect: Rectangle },
}

impl Default for SelectionState {
    fn default() -> Self {
        SelectionState::Idle
    }
}

/// Produce a `Rectangle` with top-left origin from any two corner points.
/// Ensures width and height are at least 1.0.
pub fn normalize_rect(a: Point, b: Point) -> Rectangle {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::{Point, Rectangle};

    // --- normalize_rect ---

    #[test]
    fn normalize_rect_top_left_to_bottom_right() {
        let r = normalize_rect(Point::new(10.0, 20.0), Point::new(110.0, 80.0));
        assert_eq!(r.x, 10.0);
        assert_eq!(r.y, 20.0);
        assert_eq!(r.width, 100.0);
        assert_eq!(r.height, 60.0);
    }

    #[test]
    fn normalize_rect_bottom_right_to_top_left() {
        let r = normalize_rect(Point::new(110.0, 80.0), Point::new(10.0, 20.0));
        assert_eq!(r.x, 10.0);
        assert_eq!(r.y, 20.0);
        assert_eq!(r.width, 100.0);
        assert_eq!(r.height, 60.0);
    }

    #[test]
    fn normalize_rect_top_right_to_bottom_left() {
        let r = normalize_rect(Point::new(110.0, 20.0), Point::new(10.0, 80.0));
        assert_eq!(r.x, 10.0);
        assert_eq!(r.y, 20.0);
        assert_eq!(r.width, 100.0);
        assert_eq!(r.height, 60.0);
    }

    #[test]
    fn normalize_rect_bottom_left_to_top_right() {
        let r = normalize_rect(Point::new(10.0, 80.0), Point::new(110.0, 20.0));
        assert_eq!(r.x, 10.0);
        assert_eq!(r.y, 20.0);
        assert_eq!(r.width, 100.0);
        assert_eq!(r.height, 60.0);
    }

    #[test]
    fn normalize_rect_minimum_size_when_zero_distance() {
        let r = normalize_rect(Point::new(50.0, 50.0), Point::new(50.0, 50.0));
        assert_eq!(r.width, 1.0);
        assert_eq!(r.height, 1.0);
    }

    // --- SelectionState transitions (tested as pure logic) ---

    fn apply_press(state: SelectionState, cursor_pos: Point) -> SelectionState {
        match state {
            SelectionState::Idle => SelectionState::Drawing { start: cursor_pos },
            other => other,
        }
    }

    fn apply_release(state: SelectionState, cursor_pos: Point) -> SelectionState {
        match state {
            SelectionState::Drawing { start } => {
                SelectionState::Selected { rect: normalize_rect(start, cursor_pos) }
            }
            other => other,
        }
    }

    /// Returns None if still in same app, Some(()) if should close.
    fn apply_escape(state: SelectionState) -> (SelectionState, bool) {
        match state {
            SelectionState::Idle => (SelectionState::Idle, true),
            SelectionState::Drawing { .. } => (SelectionState::Idle, false),
            SelectionState::Selected { .. } => (SelectionState::Idle, false),
        }
    }

    #[test]
    fn selection_idle_to_drawing_on_press() {
        let s = apply_press(SelectionState::Idle, Point::new(100.0, 200.0));
        assert_eq!(s, SelectionState::Drawing { start: Point::new(100.0, 200.0) });
    }

    #[test]
    fn selection_drawing_to_selected_on_release() {
        let s = apply_release(
            SelectionState::Drawing { start: Point::new(10.0, 20.0) },
            Point::new(110.0, 80.0),
        );
        assert_eq!(
            s,
            SelectionState::Selected {
                rect: Rectangle { x: 10.0, y: 20.0, width: 100.0, height: 60.0 }
            }
        );
    }

    #[test]
    fn selection_escape_from_idle_signals_close() {
        let (_, should_close) = apply_escape(SelectionState::Idle);
        assert!(should_close);
    }

    #[test]
    fn selection_escape_from_drawing_resets_to_idle() {
        let (next, should_close) = apply_escape(
            SelectionState::Drawing { start: Point::new(0.0, 0.0) }
        );
        assert_eq!(next, SelectionState::Idle);
        assert!(!should_close);
    }

    #[test]
    fn selection_escape_from_selected_resets_to_idle() {
        let (next, should_close) = apply_escape(
            SelectionState::Selected {
                rect: Rectangle { x: 0.0, y: 0.0, width: 100.0, height: 100.0 }
            }
        );
        assert_eq!(next, SelectionState::Idle);
        assert!(!should_close);
    }
}
```

- [ ] **Step 2: Add `mod selection;` to `src/overlay/mod.rs`**

Add at the top of `src/overlay/mod.rs`, after existing `use` lines:
```rust
mod selection;
pub(crate) use selection::{SelectionState, normalize_rect};
```

- [ ] **Step 3: Run tests to verify they fail (TDD red)**

```bash
source "$HOME/.cargo/env" && cargo test --lib overlay::selection 2>&1
```

Expected: compile error on `todo!()` in `normalize_rect` — that's the TDD red state.

- [ ] **Step 4: Implement `normalize_rect`**

Replace `todo!()` in `src/overlay/selection.rs` with:

```rust
pub fn normalize_rect(a: Point, b: Point) -> Rectangle {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    let width = (a.x - b.x).abs().max(1.0);
    let height = (a.y - b.y).abs().max(1.0);
    Rectangle { x, y, width, height }
}
```

- [ ] **Step 5: Run tests to verify they pass (TDD green)**

```bash
cargo test --lib overlay::selection 2>&1
```

Expected: all 9 tests pass.

- [ ] **Step 6: Run clippy**

```bash
cargo clippy -- -D warnings 2>&1
```

Fix any warnings.

- [ ] **Step 7: Commit**

```bash
git add src/overlay/selection.rs src/overlay/mod.rs
git commit -m "feat: add SelectionState and normalize_rect with full unit tests"
```

---

## Task 3: Extend `OverlayState` and `Message`

**Files:**
- Modify: `src/overlay/mod.rs`

- [ ] **Step 1: Read current `src/overlay/mod.rs` in full**

Read the file before editing to understand exact current structure.

- [ ] **Step 2: Update `OverlayState` to include selection state**

Replace the `OverlayState` struct:

```rust
/// State shared across all layer-shell surfaces.
struct OverlayState {
    handle: image::Handle,
    selection: SelectionState,
    cursor_pos: iced::Point,
}
```

- [ ] **Step 3: Update `Message` enum**

Replace the `Message` enum:

```rust
/// Messages for the overlay daemon.
///
/// `#[to_layer_message(multi)]` generates `TryInto<LayershellCustomActionsWithId>`
/// (required by the `daemon` build pattern) plus multi-window layer-shell variants.
/// The catch-all arm in `update` handles those generated variants.
#[to_layer_message(multi)]
#[derive(Debug, Clone)]
enum Message {
    /// Escape key — exits if Idle, resets to Idle if Drawing or Selected.
    EscapePressed,
    /// Left mouse button pressed — captured via event subscription.
    MousePressed,
    /// Left mouse button released — captured via event subscription.
    MouseReleased,
    /// Cursor moved to a new position.
    CursorMoved(iced::Point),
}
```

- [ ] **Step 4: Update `update()` in the daemon closure**

Replace the update closure inside `run()`:

```rust
|state: &mut OverlayState, message: Message| -> IcedTask<Message> {
    match message {
        Message::CursorMoved(pos) => {
            state.cursor_pos = pos;
            IcedTask::none()
        }
        Message::MousePressed => {
            if matches!(state.selection, SelectionState::Idle) {
                state.selection = SelectionState::Drawing { start: state.cursor_pos };
            }
            IcedTask::none()
        }
        Message::MouseReleased => {
            if let SelectionState::Drawing { start } = state.selection {
                state.selection = SelectionState::Selected {
                    rect: normalize_rect(start, state.cursor_pos),
                };
            }
            IcedTask::none()
        }
        Message::EscapePressed => match state.selection {
            SelectionState::Idle => iced::exit(),
            _ => {
                state.selection = SelectionState::Idle;
                IcedTask::none()
            }
        },
        _ => IcedTask::none(),
    }
},
```

- [ ] **Step 5: Update `subscription()` to include mouse events**

Replace the subscription closure:

```rust
.subscription(|_state| {
    use iced::event::listen_with;
    use iced::{Event, mouse};

    iced::Subscription::batch([
        keyboard::on_key_press(|key, _mods| match key {
            keyboard::Key::Named(keyboard::key::Named::Escape) => {
                Some(Message::EscapePressed)
            }
            _ => None,
        }),
        listen_with(|event, _status, _id| match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                Some(Message::CursorMoved(position))
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                Some(Message::MousePressed)
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                Some(Message::MouseReleased)
            }
            _ => None,
        }),
    ])
})
```

- [ ] **Step 6: Update `run_with` initial state**

Replace:
```rust
.run_with(move || (OverlayState { handle }, IcedTask::none()))
```
With:
```rust
.run_with(move || (
    OverlayState {
        handle,
        selection: SelectionState::Idle,
        cursor_pos: iced::Point::ORIGIN,
    },
    IcedTask::none(),
))
```

- [ ] **Step 7: Build**

```bash
cargo build 2>&1
```

Expected: builds cleanly. The view (`overlay_view`) still shows just the image — canvas comes in Task 4.

- [ ] **Step 8: Run all tests**

```bash
cargo test --lib 2>&1
```

Expected: all pass (9 selection tests + 1 overlay empty-frames test + 5 types tests = 15 total).

- [ ] **Step 9: Run clippy**

```bash
cargo clippy -- -D warnings 2>&1
```

Fix any warnings.

- [ ] **Step 10: Commit**

```bash
git add src/overlay/mod.rs
git commit -m "feat: extend OverlayState and Message for selection input handling"
```

---

## Task 4: Canvas overlay (dim + rect + toolbar)

**Files:**
- Modify: `src/overlay/mod.rs`

This task adds the visual layer: a `Canvas` widget that draws the dim, the selection rectangle, and the placeholder toolbar.

- [ ] **Step 1: Add required imports to `src/overlay/mod.rs`**

Add to the top imports:
```rust
use iced::widget::{canvas, container, image, stack};
use iced::{Color, Point, Rectangle, Size};
use iced::widget::canvas::{Frame, Geometry, Path, Stroke, LineDash, Fill, Text};
use iced::mouse;
```

Remove the existing `use iced::widget::{container, image};` line and replace with the above.

- [ ] **Step 2: Add `SelectionCanvas` struct**

Add this struct and its `canvas::Program` impl BEFORE the `overlay_view` function:

```rust
/// Canvas program that draws the dim overlay, selection rectangle, and toolbar.
/// Holds a reference to the overlay state — no internal canvas state needed.
struct SelectionCanvas<'a> {
    state: &'a OverlayState,
}

impl<'a> canvas::Program<Message> for SelectionCanvas<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // 1. Dim the entire overlay.
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            Color { r: 0.0, g: 0.0, b: 0.0, a: 0.35 },
        );

        // 2. Draw selection rect if Drawing or Selected.
        let maybe_rect = match &self.state.selection {
            SelectionState::Drawing { start } => {
                Some(normalize_rect(*start, self.state.cursor_pos))
            }
            SelectionState::Selected { rect } => Some(*rect),
            SelectionState::Idle => None,
        };

        if let Some(rect) = maybe_rect {
            // 2a. Dashed white border.
            let border = Path::rectangle(Point::new(rect.x, rect.y), rect.size());
            frame.stroke(
                &border,
                Stroke {
                    style: iced::widget::canvas::stroke::Style::Solid(Color::WHITE),
                    width: 1.5,
                    line_dash: LineDash {
                        segments: &[6.0, 4.0],
                        offset: 0,
                    },
                    ..Stroke::default()
                },
            );

            // 2b. Corner handles (5×5 white squares).
            let handle_size = Size::new(5.0, 5.0);
            let corners = [
                Point::new(rect.x - 2.5, rect.y - 2.5),
                Point::new(rect.x + rect.width - 2.5, rect.y - 2.5),
                Point::new(rect.x - 2.5, rect.y + rect.height - 2.5),
                Point::new(rect.x + rect.width - 2.5, rect.y + rect.height - 2.5),
            ];
            for corner in corners {
                frame.fill_rectangle(corner, handle_size, Color::WHITE);
            }

            // 2c. Size label above the top-left corner.
            let label = format!("{} × {}", rect.width as u32, rect.height as u32);
            frame.fill_text(Text {
                content: label,
                position: Point::new(rect.x, rect.y - 18.0),
                color: Color::WHITE,
                size: iced::Pixels(12.0),
                ..Text::default()
            });

            // 3. Placeholder toolbar when Selected.
            if matches!(self.state.selection, SelectionState::Selected { .. }) {
                let toolbar_w = 120.0_f32;
                let toolbar_h = 32.0_f32;
                let toolbar_x = rect.x + (rect.width - toolbar_w) / 2.0;
                // Place 8px below; flip above if it would clip outside bounds.
                let toolbar_y = if rect.y + rect.height + 8.0 + toolbar_h < bounds.height {
                    rect.y + rect.height + 8.0
                } else {
                    rect.y - 8.0 - toolbar_h
                };

                // Toolbar background.
                frame.fill_rectangle(
                    Point::new(toolbar_x, toolbar_y),
                    Size::new(toolbar_w, toolbar_h),
                    Color { r: 0.15, g: 0.15, b: 0.15, a: 0.95 },
                );

                // Placeholder button labels (greyed out — actions wired in M4).
                for (i, label) in ["Copy", "Save"].iter().enumerate() {
                    frame.fill_text(Text {
                        content: label.to_string(),
                        position: Point::new(
                            toolbar_x + 15.0 + i as f32 * 55.0,
                            toolbar_y + 9.0,
                        ),
                        color: Color { r: 0.5, g: 0.5, b: 0.5, a: 1.0 },
                        size: iced::Pixels(13.0),
                        ..Text::default()
                    });
                }
            }
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &(),
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        match self.state.selection {
            SelectionState::Selected { .. } => mouse::Interaction::default(),
            _ => mouse::Interaction::Crosshair,
        }
    }
}
```

- [ ] **Step 3: Update `overlay_view` to use a stack with the canvas**

Replace the existing `overlay_view` function:

```rust
fn overlay_view(
    state: &OverlayState,
    _window: iced::window::Id,
) -> Element<'_, Message, iced::Theme, iced::Renderer> {
    let frozen = image::Image::new(state.handle.clone())
        .width(Length::Fill)
        .height(Length::Fill);

    let selection_canvas = canvas(SelectionCanvas { state })
        .width(Length::Fill)
        .height(Length::Fill);

    container(
        stack![frozen, selection_canvas]
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
```

- [ ] **Step 4: Build and fix any compile errors**

```bash
cargo build 2>&1
```

Common issues and fixes:
- `Text::default()` may not have a `font` field set — if `fill_text` errors, check `iced::widget::canvas::Text` fields in `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_widget-0.13.4/src/canvas/`.
- `stroke::Style` import path — if it errors, use `iced::widget::canvas::Fill` or check the actual path. Alternative: `Stroke::default().with_color(Color::WHITE)` if that method exists.
- `stack![]` macro — import is `iced::widget::stack` or just use `iced::widget::Stack::new(vec![...])`.

Fix any compile errors by reading the actual iced source in `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_widget-0.13.4/src/`.

- [ ] **Step 5: Run all tests**

```bash
cargo test --lib 2>&1
```

Expected: all 15 tests pass.

- [ ] **Step 6: Run clippy**

```bash
cargo clippy -- -D warnings 2>&1
```

Fix all warnings.

- [ ] **Step 7: Commit**

```bash
git add src/overlay/mod.rs
git commit -m "feat: add canvas selection overlay with dim, dashed rect, handles, and toolbar"
```

---

## Task 5: Manual end-to-end verification

- [ ] **Step 1: Run the binary on COSMIC**

```bash
source "$HOME/.cargo/env" && cargo run --release 2>&1
```

Verify:
- Overlay appears with ~35% dim over the frozen frame
- Cursor is a crosshair
- Click and drag draws a dashed white rectangle with corner handles
- Size label (e.g. `320 × 180`) appears above the top-left corner
- Releasing the mouse shows a small toolbar with greyed-out "Copy" and "Save"
- Escape while drawing → resets to Idle (rect disappears, crosshair returns)
- Escape while selected → resets to Idle
- Escape while Idle → closes the overlay

- [ ] **Step 2: Final checks**

```bash
cargo clippy -- -D warnings && cargo test --lib 2>&1
```

Expected: zero warnings, all 15 tests pass.

- [ ] **Step 3: Commit any fixups**

```bash
git add -A
git commit -m "fix: M3 manual verification fixups"
```

(Skip if nothing changed.)

---

## Task 6: Update design doc with deviations

- [ ] **Step 1: Note any API differences**

If any iced canvas API differed from the design (e.g. `Text` field names, `Stroke` style, `stack!` macro), add an "Implementation Notes" section to `docs/superpowers/specs/2026-05-09-m3-selection-rectangle-design.md`.

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/specs/2026-05-09-m3-selection-rectangle-design.md
git commit -m "docs: update M3 spec with implementation notes"
```

---

## Known Risks & Mitigations

| Risk | Mitigation |
|---|---|
| `canvas::Text` field names differ from plan | Read actual source at `iced_widget-0.13.4/src/canvas/text.rs` before using |
| `Stroke` style path — `canvas::stroke::Style` may not exist | Use `iced::widget::canvas::Stroke` and set `color` if style field differs |
| `stack![]` macro import — may need `use iced::widget::stack;` | Try `iced::widget::Stack::with_children(vec![...])` if macro fails |
| `frame.fill_rectangle` with `Color` vs `Fill` — signature may require `Fill` | Wrap: `Fill::from(Color { ... })` or just pass `Color` directly (it impls `Into<Fill>`) |
| Canvas dim doesn't visually "cut through" to frame below | The image is on the bottom layer of the stack; canvas draws on top — dim only affects the canvas layer, not the image. This is correct: the image shows through at full brightness everywhere, then the canvas paints a semi-transparent black rectangle over everything. The selection "clear" is actually a no-op since there's nothing to erase in the canvas — the image simply shows through where the canvas is transparent. Remove the "clear" fill_rectangle inside the selection rect — it's not needed. |
