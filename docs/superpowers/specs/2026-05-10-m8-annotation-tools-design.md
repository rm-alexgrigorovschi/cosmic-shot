# M8: Annotation Tools Design

## Overview

Add annotation tools to cosmic-shot — arrow, rectangle, text, and highlight —
plus an eraser. Annotations appear in a merged toolbar below the selection
rectangle. All annotations are vector shapes stored in `OverlayState`, rendered
on the iced canvas, and burned onto the pixel data at export time.

## Tools

| Tool      | Key | Interaction                                              |
|-----------|-----|----------------------------------------------------------|
| Arrow     | `A` | Click-drag from start to end; arrowhead at endpoint      |
| Rectangle | `R` | Click-drag to draw an outlined rectangle (2px border)    |
| Text      | `T` | Click to place cursor, type to enter text, Enter commits |
| Highlight | `H` | Click-drag to draw a semi-transparent filled rectangle   |
| Eraser    | `E` | Click on an annotation to remove it                      |

### Global shortcuts

| Key         | Action                                    |
|-------------|-------------------------------------------|
| `Backspace` | Remove the last annotation (any tool)     |
| `Ctrl+C`    | Copy to clipboard                         |
| `Ctrl+S`    | Save to file                              |
| `Escape`    | Deselect tool → reset selection → close   |

`Ctrl+C` and `Ctrl+S` work inside the iced overlay when launched via keyboard
shortcut. When running from a terminal, `Ctrl+C` sends SIGINT (kills the
process) — acceptable since that's expected terminal behavior.

## Data Model

### Annotation enum

```rust
enum Annotation {
    Arrow     { start: Point, end: Point },
    Rect      { rect: Rectangle },
    Text      { pos: Point, content: String },
    Highlight { rect: Rectangle },
}
```

### AnnotationTool enum

```rust
enum AnnotationTool {
    None,
    Arrow,
    Rect,
    Text,
    Highlight,
    Eraser,
}
```

### PendingAnnotation

In-progress annotation while the user is dragging:

```rust
enum PendingAnnotation {
    Arrow     { start: Point },
    Rect      { start: Point },
    Highlight { start: Point },
    Text      { pos: Point },
}
```

### OverlayState additions

```rust
// Added to OverlayState:
annotations: Vec<Annotation>,
active_tool: AnnotationTool,
pending: Option<PendingAnnotation>,
text_input: String,
```

## Config

New field in `~/.config/cosmic-shot/config.toml`:

```toml
annotation_color = "#ff0000"   # default: red
```

Parsed as `[u8; 4]` RGBA (alpha always 255, except highlight which uses ~80
for semi-transparency). Invalid hex values log a warning and fall back to red.

## Toolbar

### Layout

Single merged row below the selection rectangle (or above if near bottom edge,
same flip logic as the current Copy/Save toolbar):

```
[ → Arrow ][ □ Rect ][ T Text ][ ▬ Hi ][ ✕ Erase ] │ [ ⎘ Copy ][ 💾 Save ]
```

Active tool is visually highlighted (brighter background or underline).

### Hit-testing

The current toolbar draws in `SelectionCanvas::draw()` and hit-tests in
`MousePressed` with duplicated geometry. This design extracts toolbar logic
into `overlay/toolbar.rs` with a `ToolbarLayout` struct that computes button
rects once and is used by both `draw()` and click handling.

## Interaction Flow

### Arrow / Rectangle / Highlight

1. User presses tool key or clicks toolbar button → `active_tool` changes
2. User clicks in the selection area → `PendingAnnotation::Arrow { start }`
3. User drags → preview drawn on canvas (dashed line / dashed rect)
4. User releases → `Annotation` committed to `annotations` vec
5. Tool remains active for next annotation

### Text

1. User presses `T` or clicks text toolbar button → `active_tool = Text`
2. User clicks in selection area → `PendingAnnotation::Text { pos }`
3. Keyboard events captured → characters accumulate in `text_input`
4. Canvas shows live text preview at the click position
5. `Enter` or click elsewhere → `Annotation::Text` committed, `text_input` cleared
6. `Escape` while typing → cancel text placement (don't commit)

### Eraser

1. User presses `E` or clicks eraser button → `active_tool = Eraser`
2. User clicks near an annotation → removed from `annotations` vec
3. Hit detection: check if click point is within a bounding box of each
   annotation (with ~5px tolerance for lines/arrows)

### Backspace

Removes the last item from `annotations` regardless of active tool. If
`annotations` is empty, no-op.

### Escape (layered)

1. If text is being placed (`pending` is `Text`) → cancel text input
2. If a tool is active → deactivate tool (`active_tool = None`)
3. If selection exists → reset to `Idle`
4. If already `Idle` → exit app

## Export: Burning Annotations

When Copy or Save is triggered:

1. `crop_selection()` produces a `CroppedImage` (existing)
2. `burn_annotations()` renders annotations onto the cropped pixel data
3. Result passed to `copy_to_clipboard()` or `save_cropped()`

### burn_annotations()

```rust
pub fn burn_annotations(
    image: &mut CroppedImage,
    annotations: &[Annotation],
    selection_rect: Rectangle,
    scale: f32,
    color: [u8; 4],
)
```

Each annotation's coordinates are relative to the overlay. Subtract
`selection_rect.{x,y}` then multiply by `scale` to get pixel coordinates
in the cropped image.

Rendering per type:
- **Arrow:** draw line (Bresenham) + filled triangle arrowhead
- **Rect:** draw 2px outline rectangle
- **Highlight:** alpha-blend a semi-transparent filled rectangle
- **Text:** render using `imageproc` + `ab_glyph` with an embedded font

### Text rendering dependency

Add `imageproc` and `ab_glyph` crates for pixel-level text rendering at
export time. These are used only in `burn_annotations()`, not in the iced
canvas (iced handles live preview text rendering with `canvas::Text`).

## File Structure

| File                        | Responsibility                                          |
|-----------------------------|---------------------------------------------------------|
| `src/overlay/mod.rs`       | Overlay state, message routing, tool key handling       |
| `src/overlay/selection.rs` | Selection state machine (unchanged)                     |
| `src/overlay/annotation.rs`| `Annotation`, `AnnotationTool`, `PendingAnnotation`, draw helpers |
| `src/overlay/toolbar.rs`   | `ToolbarLayout`, button rects, hit-test, draw           |
| `src/export/burn.rs`       | `burn_annotations()` — render annotations onto pixels   |
| `src/export/mod.rs`        | Existing exports + re-export burn module                |
| `src/config.rs`            | Add `annotation_color` field                            |

## New Dependencies

```toml
imageproc = "0.25"
ab_glyph = "0.2"
```

Plus an embedded font file (e.g. DejaVu Sans or Liberation Sans, OFL/Apache
licensed) included as `const FONT_BYTES: &[u8] = include_bytes!(...)`.

## Out of Scope

- Freehand pen tool
- Color picker in toolbar
- Annotation resize/move after placement
- Multiple annotation colors per screenshot
- Font size configuration (fixed at 16px logical, scaled by HiDPI factor at export)
- Redo (only undo via Backspace)

## Testing

- Unit tests for `Annotation` bounding box calculations
- Unit tests for `ToolbarLayout` button rect computation
- Unit tests for `burn_annotations()` with a small test image:
  verify arrow draws pixels, rect draws border, highlight blends alpha
- Unit test for annotation coordinate transform (logical → physical)
- Integration test: `Backspace` removes last annotation
- Manual: place all 4 annotation types, erase one, copy, paste into viewer
