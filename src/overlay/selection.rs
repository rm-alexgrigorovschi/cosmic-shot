use iced::Point;
use iced::Rectangle;

/// Which phase of the selection interaction we are in.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SelectionState {
    /// Waiting for the user to click. Crosshair cursor shown.
    #[default]
    Idle,
    /// User is holding the mouse button. Rect grows as cursor moves.
    Drawing { start: Point },
    /// User released the mouse. Toolbar is visible.
    Selected { rect: Rectangle },
}

/// Produce a `Rectangle` with top-left origin from any two corner points.
/// Ensures width and height are at least 1.0.
pub fn normalize_rect(a: Point, b: Point) -> Rectangle {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    let width = (a.x - b.x).abs().max(1.0);
    let height = (a.y - b.y).abs().max(1.0);
    Rectangle { x, y, width, height }
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

    /// Returns (next_state, should_close).
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
