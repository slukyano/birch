//! Terminal event → semantic action mapping. Pure; the app resolves click
//! coordinates against the rendered rows.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};

/// Two complete clicks on the same row within this window activate — measured
/// release to release (ADR 0015, as amended by ADR 0025).
pub const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(450);

/// Infers double-clicks from single Down events (terminals never synthesize
/// them). Keyed on the row path, not the visual index, so a live-update
/// reshuffle between the two clicks cannot activate the wrong row.
#[derive(Default)]
pub struct ClickTimer {
    last: Option<(PathBuf, Instant)>,
}

impl ClickTimer {
    /// Records a completed click on `path` at `now` (its *release*); true when
    /// it completes a double-click. A completed double disarms — a
    /// triple-click starts a fresh select cycle.
    pub fn observe(&mut self, path: &Path, now: Instant) -> bool {
        let double = self
            .last
            .take()
            .is_some_and(|(p, t)| p == path && now.duration_since(t) <= DOUBLE_CLICK_WINDOW);
        if !double {
            self.last = Some((path.to_path_buf(), now));
        }
        double
    }

    /// Forgets the pending click. Chevron clicks disarm: chevron-then-name
    /// in quick succession is a select, not an open.
    pub fn disarm(&mut self) {
        self.last = None;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputAction {
    Up,
    Down,
    Right,
    Left,
    Enter,
    Quit,
    Esc,
    /// A printable character — always search input, never a hotkey
    /// (ADR 0008).
    Char(char),
    Backspace,
    ScrollUp,
    ScrollDown,
    /// Left button down: arms a click, acts on nothing (ADR 0025).
    Press {
        column: u16,
        row: u16,
    },
    /// Left button up: completes the armed click, or abandons it.
    Release {
        column: u16,
        row: u16,
    },
    Redraw,
}

pub fn map_event(event: &Event, mouse_enabled: bool) -> Option<InputAction> {
    match event {
        Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
            KeyCode::Up => Some(InputAction::Up),
            KeyCode::Down => Some(InputAction::Down),
            KeyCode::Right => Some(InputAction::Right),
            KeyCode::Left => Some(InputAction::Left),
            KeyCode::Enter => Some(InputAction::Enter),
            KeyCode::Esc => Some(InputAction::Esc),
            KeyCode::Backspace => Some(InputAction::Backspace),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(InputAction::Quit)
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                Some(InputAction::Char(c))
            }
            _ => None,
        },
        Event::Mouse(mouse) if mouse_enabled => match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => Some(InputAction::Press {
                column: mouse.column,
                row: mouse.row,
            }),
            // Any left-button release completes the armed press. birch enables
            // SGR encoding, where a release names its button; a terminal that
            // reports an ambiguous release still resolves correctly, because
            // at most one press is ever armed (ADR 0025).
            MouseEventKind::Up(MouseButton::Left) => Some(InputAction::Release {
                column: mouse.column,
                row: mouse.row,
            }),
            MouseEventKind::ScrollUp => Some(InputAction::ScrollUp),
            MouseEventKind::ScrollDown => Some(InputAction::ScrollDown),
            _ => None,
        },
        Event::Resize(..) => Some(InputAction::Redraw),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyEvent, MouseEvent};

    use super::*;

    #[test]
    fn keys_map_to_actions() {
        let ev = Event::Key(KeyEvent::from(KeyCode::Up));
        assert_eq!(map_event(&ev, true), Some(InputAction::Up));
        // q is a printable char, not a hotkey (ADR 0008).
        let ev = Event::Key(KeyEvent::from(KeyCode::Char('q')));
        assert_eq!(map_event(&ev, true), Some(InputAction::Char('q')));
        let ev = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(map_event(&ev, true), Some(InputAction::Quit));
        let ev = Event::Key(KeyEvent::from(KeyCode::Char(' ')));
        assert_eq!(map_event(&ev, true), Some(InputAction::Char(' ')));
        let ev = Event::Key(KeyEvent::from(KeyCode::Backspace));
        assert_eq!(map_event(&ev, true), Some(InputAction::Backspace));
        let ev = Event::Key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(map_event(&ev, true), Some(InputAction::Esc));
    }

    #[test]
    fn mouse_respects_toggle() {
        let ev = Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(map_event(&ev, true), Some(InputAction::ScrollDown));
        assert_eq!(map_event(&ev, false), None);
    }

    #[test]
    fn double_click_within_window_activates_then_disarms() {
        let mut timer = ClickTimer::default();
        let t0 = Instant::now();
        let a = Path::new("/r/a");
        assert!(!timer.observe(a, t0));
        assert!(timer.observe(a, t0 + Duration::from_millis(200)));
        // A completed double disarms: a triple-click starts a fresh cycle.
        assert!(!timer.observe(a, t0 + Duration::from_millis(300)));
    }

    #[test]
    fn late_or_moved_clicks_stay_single() {
        let mut timer = ClickTimer::default();
        let t0 = Instant::now();
        let a = Path::new("/r/a");
        let b = Path::new("/r/b");
        assert!(!timer.observe(a, t0));
        // Past the window: re-arms instead of activating.
        let late = t0 + DOUBLE_CLICK_WINDOW + Duration::from_millis(1);
        assert!(!timer.observe(a, late));
        // A different row resets the cycle — and arms the new row.
        assert!(!timer.observe(b, late + Duration::from_millis(50)));
        assert!(timer.observe(b, late + Duration::from_millis(100)));
    }

    #[test]
    fn disarm_forgets_the_pending_click() {
        let mut timer = ClickTimer::default();
        let t0 = Instant::now();
        let a = Path::new("/r/a");
        assert!(!timer.observe(a, t0));
        timer.disarm();
        assert!(!timer.observe(a, t0 + Duration::from_millis(10)));
    }
}
