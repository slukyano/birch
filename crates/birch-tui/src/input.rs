//! Terminal event → semantic action mapping. Pure; the app resolves click
//! coordinates against the rendered rows.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};

/// Two complete clicks on the same row within this window activate — measured
/// release to release (ADR 0015, as amended by ADR 0025).
pub const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(450);

/// Infers double-clicks from completed single clicks (terminals never
/// synthesize them). Keyed on the row path, not the visual index, so a
/// live-update reshuffle between the two clicks cannot activate the wrong row.
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
    /// Move the selection a page up or down; the viewport follows (task 081).
    PageUp,
    PageDown,
    /// Move the selection to the first or last reachable row.
    Home,
    End,
    /// `Shift` with a movement key: move the cursor and extend the selection
    /// from the anchor to where it lands (task 074).
    Extend(Movement),
    /// Open the context menu at the cursor row (task 092). The right button is
    /// the other way in, and the only one before this.
    OpenMenu,
    /// Rename the selection inline (`F2`).
    Rename,
    /// Create a file or directory inline (`Ctrl-N`); a trailing slash makes a
    /// directory, so one binding covers three operations.
    NewFile,
    /// Move the selection to the trash (`Delete`).
    Delete,
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
    /// Left button up: completes the armed click, or abandons it. The
    /// modifiers travel with the release, because that is where the click
    /// takes effect.
    Release {
        column: u16,
        row: u16,
        modifier: ClickModifier,
    },
    /// Right button down: arms the menu, opens nothing (ADR 0025 extended to
    /// the right button — there is one click model, not one per button).
    RightPress {
        column: u16,
        row: u16,
    },
    /// Right button up: opens the menu when it lands where the press did.
    RightRelease {
        column: u16,
        row: u16,
    },
    /// Pointer motion. Only reported while the context menu is open, which is
    /// the only time birch asks for motion tracking (task 090 would generalise
    /// it to the tree).
    Hover {
        column: u16,
        row: u16,
    },
    Redraw,
}

/// The cursor movements `Shift` can extend a selection along. `→` and `←` are
/// absent on purpose: they are structural (expand, split, collapse), and a
/// range has no meaning attached to them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movement {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}

/// What a modifier held during a click asks for (task 074). Whether a modifier
/// arrives at all is the terminal's decision — many keep `Shift` for their own
/// text selection while an application has the mouse — so birch handles what it
/// is given and never claims a modifier back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClickModifier {
    #[default]
    None,
    /// Extend the selection from the anchor to this row.
    Extend,
    /// Add this row to the selection, or take it out.
    Toggle,
}

/// `Shift` extends, `Ctrl` toggles, and `Shift` wins when both are held —
/// extending is the more specific ask. `Alt` is left alone: it is the modifier
/// terminals most often bind for themselves.
fn click_modifier(modifiers: KeyModifiers) -> ClickModifier {
    if modifiers.contains(KeyModifiers::SHIFT) {
        ClickModifier::Extend
    } else if modifiers.contains(KeyModifiers::CONTROL) {
        ClickModifier::Toggle
    } else {
        ClickModifier::None
    }
}

pub fn map_event(event: &Event, mouse_enabled: bool) -> Option<InputAction> {
    match event {
        Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                Some(InputAction::Extend(Movement::Up))
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                Some(InputAction::Extend(Movement::Down))
            }
            KeyCode::Up => Some(InputAction::Up),
            KeyCode::Down => Some(InputAction::Down),
            KeyCode::Right => Some(InputAction::Right),
            KeyCode::Left => Some(InputAction::Left),
            KeyCode::Enter => Some(InputAction::Enter),
            KeyCode::Esc => Some(InputAction::Esc),
            // Ctrl and Alt fall through to the plain action — `Ctrl-End` means
            // the end everywhere else, and a key that silently does nothing is
            // worse than one that does the obvious thing. `Shift` is excluded:
            // it extends a selection (task 074).
            KeyCode::PageUp | KeyCode::PageDown | KeyCode::Home | KeyCode::End => {
                let movement = match key.code {
                    KeyCode::PageUp => Movement::PageUp,
                    KeyCode::PageDown => Movement::PageDown,
                    KeyCode::Home => Movement::Home,
                    _ => Movement::End,
                };
                Some(if key.modifiers.contains(KeyModifiers::SHIFT) {
                    InputAction::Extend(movement)
                } else {
                    match movement {
                        Movement::PageUp => InputAction::PageUp,
                        Movement::PageDown => InputAction::PageDown,
                        Movement::Home => InputAction::Home,
                        _ => InputAction::End,
                    }
                })
            }
            // `Shift-F10` is the Windows and Linux convention for a context
            // menu, and function keys travel as CSI sequences that *do* carry
            // a modifier — unlike `Ctrl` with a letter, where `Shift` has no
            // representation at all (task 091). The `Menu` key is the same
            // convention on keyboards that have one; Apple keyboards do not,
            // which is why it cannot be the only binding.
            KeyCode::F(10) if key.modifiers.contains(KeyModifiers::SHIFT) => {
                Some(InputAction::OpenMenu)
            }
            KeyCode::Menu => Some(InputAction::OpenMenu),
            KeyCode::F(2) => Some(InputAction::Rename),
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(InputAction::NewFile)
            }
            // Forward delete. On an Apple keyboard the key labelled *Delete*
            // sends Backspace, which belongs to the search, so the gesture
            // there is `Fn`-`Delete`; binding Backspace to a destructive action
            // whose meaning depended on whether a search was open would be a
            // contextual hazard, and the menu is the discoverable path.
            KeyCode::Delete => Some(InputAction::Delete),
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
                modifier: click_modifier(mouse.modifiers),
            }),
            MouseEventKind::Down(MouseButton::Right) => Some(InputAction::RightPress {
                column: mouse.column,
                row: mouse.row,
            }),
            MouseEventKind::Up(MouseButton::Right) => Some(InputAction::RightRelease {
                column: mouse.column,
                row: mouse.row,
            }),
            MouseEventKind::Moved => Some(InputAction::Hover {
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
    fn the_navigation_keys_map_and_leave_shift_alone() {
        for (code, action) in [
            (KeyCode::PageUp, InputAction::PageUp),
            (KeyCode::PageDown, InputAction::PageDown),
            (KeyCode::Home, InputAction::Home),
            (KeyCode::End, InputAction::End),
        ] {
            assert_eq!(
                map_event(&Event::Key(KeyEvent::from(code)), true),
                Some(action)
            );
            // Ctrl and Alt mean the same thing as the bare key.
            for modifier in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
                assert_eq!(
                    map_event(&Event::Key(KeyEvent::new(code, modifier)), true),
                    Some(action),
                    "{code:?} with {modifier:?}"
                );
            }
        }
    }

    #[test]
    fn shift_extends_and_ctrl_toggles_on_a_release() {
        let at = |modifiers| {
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column: 4,
                row: 2,
                modifiers,
            })
        };
        let modifier_of = |ev| match map_event(&ev, true) {
            Some(InputAction::Release { modifier, .. }) => modifier,
            other => panic!("expected a release, got {other:?}"),
        };
        assert_eq!(modifier_of(at(KeyModifiers::NONE)), ClickModifier::None);
        assert_eq!(modifier_of(at(KeyModifiers::SHIFT)), ClickModifier::Extend);
        assert_eq!(
            modifier_of(at(KeyModifiers::CONTROL)),
            ClickModifier::Toggle
        );
        // Extending is the more specific ask, so it wins when both are held.
        assert_eq!(
            modifier_of(at(KeyModifiers::SHIFT | KeyModifiers::CONTROL)),
            ClickModifier::Extend
        );
        // Alt is left to the terminal.
        assert_eq!(modifier_of(at(KeyModifiers::ALT)), ClickModifier::None);
    }

    #[test]
    fn shift_turns_a_movement_key_into_an_extension() {
        for (code, movement) in [
            (KeyCode::Up, Movement::Up),
            (KeyCode::Down, Movement::Down),
            (KeyCode::PageUp, Movement::PageUp),
            (KeyCode::PageDown, Movement::PageDown),
            (KeyCode::Home, Movement::Home),
            (KeyCode::End, Movement::End),
        ] {
            assert_eq!(
                map_event(&Event::Key(KeyEvent::new(code, KeyModifiers::SHIFT)), true),
                Some(InputAction::Extend(movement)),
                "{code:?}"
            );
        }
        // The structural arrows have no range meaning, so Shift leaves them
        // exactly as they are.
        assert_eq!(
            map_event(
                &Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT)),
                true
            ),
            Some(InputAction::Right)
        );
    }

    #[test]
    fn the_context_menu_has_a_keyboard_trigger() {
        assert_eq!(
            map_event(
                &Event::Key(KeyEvent::new(KeyCode::F(10), KeyModifiers::SHIFT)),
                true
            ),
            Some(InputAction::OpenMenu)
        );
        assert_eq!(
            map_event(&Event::Key(KeyEvent::from(KeyCode::Menu)), true),
            Some(InputAction::OpenMenu)
        );
        // Plain F10 is not bound — the convention is the modified form, and an
        // unmodified function key is worth keeping free.
        assert_eq!(
            map_event(&Event::Key(KeyEvent::from(KeyCode::F(10))), true),
            None
        );
    }

    #[test]
    fn the_operations_have_the_keys_the_design_doc_names() {
        for (code, action) in [
            (KeyCode::F(2), InputAction::Rename),
            (KeyCode::Delete, InputAction::Delete),
        ] {
            assert_eq!(
                map_event(&Event::Key(KeyEvent::from(code)), true),
                Some(action)
            );
        }
        assert_eq!(
            map_event(
                &Event::Key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL)),
                true
            ),
            Some(InputAction::NewFile)
        );
        // A bare `n` is search input like every other printable (ADR 0008).
        assert_eq!(
            map_event(&Event::Key(KeyEvent::from(KeyCode::Char('n'))), true),
            Some(InputAction::Char('n'))
        );
        // Backspace stays the search's, on every platform.
        assert_eq!(
            map_event(&Event::Key(KeyEvent::from(KeyCode::Backspace)), true),
            Some(InputAction::Backspace)
        );
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
    fn the_left_button_maps_to_press_and_release() {
        let at = |kind| {
            Event::Mouse(MouseEvent {
                kind,
                column: 7,
                row: 3,
                modifiers: KeyModifiers::NONE,
            })
        };
        // The press arms and the release completes (ADR 0025); both carry the
        // coordinates, and the app decides what they mean.
        assert_eq!(
            map_event(&at(MouseEventKind::Down(MouseButton::Left)), true),
            Some(InputAction::Press { column: 7, row: 3 })
        );
        assert_eq!(
            map_event(&at(MouseEventKind::Up(MouseButton::Left)), true),
            Some(InputAction::Release {
                column: 7,
                row: 3,
                modifier: ClickModifier::None
            })
        );
        // The right button arms and completes the same way — one click model,
        // not one per button (task 071).
        assert_eq!(
            map_event(&at(MouseEventKind::Down(MouseButton::Right)), true),
            Some(InputAction::RightPress { column: 7, row: 3 })
        );
        assert_eq!(
            map_event(&at(MouseEventKind::Up(MouseButton::Right)), true),
            Some(InputAction::RightRelease { column: 7, row: 3 })
        );
        // With the mouse disabled, neither reaches the app.
        assert_eq!(
            map_event(&at(MouseEventKind::Down(MouseButton::Left)), false),
            None
        );
        assert_eq!(
            map_event(&at(MouseEventKind::Up(MouseButton::Left)), false),
            None
        );
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
