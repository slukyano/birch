//! The context menu: a projection of the action layer (ADR 0029), plus the
//! geometry that keeps it readable in a pane narrower and shorter than it is.
//!
//! The menu holds ordering, a highlight, and a scroll offset. It knows nothing
//! about what any action does — that lives in `birch_core::action`.

use birch_core::action::{Action, ActionContext, Label, MenuRow, menu_rows};

/// Columns the frame costs on each side: the border, and a space inside it.
/// The border is what separates the menu from the tree beneath, so no theme
/// colour has to be invented for a menu background.
const FRAME: u16 = 2;
/// Rows the frame costs: the top and bottom border.
const FRAME_ROWS: u16 = 2;
/// Columns between a label and its accelerator.
const GAP: u16 = 2;
/// The narrowest menu worth drawing: the frame, and a character between.
const MIN_WIDTH: u16 = FRAME * 2 + 1;

/// Where the menu sits and how wide it is, resolved against the pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuLayout {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    /// False when the pane is too narrow to hold labels *and* accelerators;
    /// the accelerators are dropped first, being redundant hints.
    pub accelerators: bool,
}

impl MenuLayout {
    /// Columns available to a row's text, inside the frame.
    pub fn content_width(self) -> usize {
        self.width.saturating_sub(FRAME * 2) as usize
    }

    /// Rows available to entries, inside the top and bottom border.
    pub fn visible_rows(self) -> usize {
        self.height.saturating_sub(FRAME_ROWS) as usize
    }
}

pub struct Menu {
    rows: Vec<MenuRow>,
    /// The label each row shows, resolved when the menu opened. `Open` names
    /// the program it will run — or says which way it will toggle a directory —
    /// so a label is no longer a constant of its action (task 034); a
    /// separator's is empty.
    labels: Vec<Label>,
    /// Index into `rows`. Always an entry, never a separator, whenever there
    /// is any entry at all.
    highlighted: usize,
    /// First visible row, for a menu taller than the pane.
    scroll: usize,
    /// Where the click that opened it landed.
    origin: (u16, u16),
}

impl Menu {
    /// Builds the menu for a selection. `None` when no action is available —
    /// an empty menu is not a menu, and a surface that opens onto nothing is
    /// worse than one that does not open.
    pub fn open(context: ActionContext, column: u16, row: u16) -> Option<Self> {
        let rows = menu_rows(context);
        let highlighted = rows.iter().position(is_entry)?;
        let labels = rows
            .iter()
            .map(|row| match row {
                MenuRow::Entry(action) => action.label_in(context),
                MenuRow::Separator => Label::default(),
            })
            .collect();
        Some(Self {
            rows,
            labels,
            highlighted,
            scroll: 0,
            origin: (column, row),
        })
    }

    pub fn rows(&self) -> &[MenuRow] {
        &self.rows
    }

    /// The label a row shows, resolved when the menu opened.
    pub fn label(&self, index: usize) -> &Label {
        static EMPTY: std::sync::OnceLock<Label> = std::sync::OnceLock::new();
        self.labels
            .get(index)
            .unwrap_or_else(|| EMPTY.get_or_init(Label::default))
    }

    pub fn highlighted(&self) -> usize {
        self.highlighted
    }

    pub fn scroll(&self) -> usize {
        self.scroll
    }

    /// The action under the highlight.
    pub fn selected_action(&self) -> Option<Action> {
        match self.rows.get(self.highlighted) {
            Some(MenuRow::Entry(action)) => Some(*action),
            _ => None,
        }
    }

    /// Moves the highlight to the next or previous **entry**, stepping over
    /// separators and stopping at the ends. A menu is short; wrapping it buys
    /// nothing and costs predictability.
    pub fn step(&mut self, forward: bool) {
        let next = if forward {
            self.rows[self.highlighted + 1..]
                .iter()
                .position(is_entry)
                .map(|offset| self.highlighted + 1 + offset)
        } else {
            self.rows[..self.highlighted].iter().rposition(is_entry)
        };
        if let Some(next) = next {
            self.highlighted = next;
        }
    }

    /// `Home` / `End` inside the menu — the same keys they are in the tree.
    pub fn go_to_edge(&mut self, first: bool) {
        let target = if first {
            self.rows.iter().position(is_entry)
        } else {
            self.rows.iter().rposition(is_entry)
        };
        if let Some(target) = target {
            self.highlighted = target;
        }
    }

    /// The action at a row index, or `None` for a separator or a miss. Callers
    /// that act on a *clicked* row must ask this rather than moving the
    /// highlight and reading it back: `highlight` ignores separators, so the
    /// highlight would still be standing on the previous entry.
    pub fn action_at(&self, index: usize) -> Option<Action> {
        match self.rows.get(index) {
            Some(MenuRow::Entry(action)) => Some(*action),
            _ => None,
        }
    }

    /// Points the highlight at a row, ignoring separators so hovering the rule
    /// between two groups does not move it.
    pub fn highlight(&mut self, index: usize) {
        if self.rows.get(index).is_some_and(is_entry) {
            self.highlighted = index;
        }
    }

    /// Resolves position and size against the pane, and scrolls the highlight
    /// into view. Called before every draw and before every hit test, so the
    /// two can never disagree about where the menu is.
    pub fn layout(&mut self, area_x: u16, area_y: u16, width: u16, height: u16) -> MenuLayout {
        let label_width = self
            .labels
            .iter()
            .map(|label| label.width() as u16)
            .max()
            .unwrap_or(0);
        let accel_width = self
            .rows
            .iter()
            .filter_map(|row| match row {
                MenuRow::Entry(action) => action.accelerator(),
                MenuRow::Separator => None,
            })
            .map(|accel| accel.chars().count() as u16)
            .max()
            .unwrap_or(0);

        let full = FRAME * 2 + label_width + GAP + accel_width;
        let bare = FRAME * 2 + label_width;
        // Accelerators go first when the pane cannot hold them; the labels
        // ellipsise only after that.
        let (mut menu_width, accelerators) = if full <= width {
            (full, accel_width > 0)
        } else {
            (bare.min(width), false)
        };
        menu_width = menu_width.clamp(MIN_WIDTH.min(width), width.max(1));

        let menu_height = (self.rows.len() as u16 + FRAME_ROWS).min(height);
        // Shift left to fit rather than flipping: in a narrow pane a flip
        // throws the menu across the whole width, away from the pointer.
        let x = self
            .origin
            .0
            .min(area_x + width.saturating_sub(menu_width))
            .max(area_x);
        // Grow downward, flipping to grow upward when the bottom is in the way.
        let y = if self.origin.1 + menu_height <= area_y + height {
            self.origin.1
        } else {
            self.origin
                .1
                .saturating_sub(menu_height)
                .max(area_y)
                .min(area_y + height.saturating_sub(menu_height))
        };

        let visible = menu_height.saturating_sub(FRAME_ROWS) as usize;
        if visible > 0 {
            self.scroll = self.scroll.min(self.rows.len().saturating_sub(visible));
            if self.highlighted < self.scroll {
                self.scroll = self.highlighted;
            } else if self.highlighted >= self.scroll + visible {
                self.scroll = self.highlighted + 1 - visible;
            }
        }

        MenuLayout {
            x,
            y,
            width: menu_width,
            height: menu_height,
            accelerators,
        }
    }

    /// The row index under a point, or `None` when the point is outside the
    /// menu. Callers must have laid the menu out first.
    pub fn hit(&self, layout: MenuLayout, column: u16, row: u16) -> Option<usize> {
        let inside = column >= layout.x
            && column < layout.x + layout.width
            && row > layout.y
            && row + 1 < layout.y + layout.height;
        if !inside {
            return None;
        }
        let index = self.scroll + (row - layout.y - 1) as usize;
        (index < self.rows.len()).then_some(index)
    }

    /// Whether a point is anywhere on the menu, border included. A click
    /// outside closes it; a click on the frame is consumed and does nothing.
    pub fn covers(&self, layout: MenuLayout, column: u16, row: u16) -> bool {
        column >= layout.x
            && column < layout.x + layout.width
            && row >= layout.y
            && row < layout.y + layout.height
    }
}

fn is_entry(row: &MenuRow) -> bool {
    matches!(row, MenuRow::Entry(_))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use birch_core::action::ActionTarget;

    use super::*;

    fn dir_context(targets: &[ActionTarget]) -> ActionContext<'_> {
        ActionContext {
            targets,
            picker: false,
            open_is_platform: false,
            open_program: "nvim",
            system_program: "open",
            undo: None,
            cursor: targets.first(),
        }
    }

    fn dir() -> Vec<ActionTarget> {
        vec![ActionTarget {
            path: PathBuf::from("/r/src"),
            is_dir: true,
            expanded: false,
            missing: false,
        }]
    }

    fn menu_at(column: u16, row: u16) -> Menu {
        let targets = dir();
        Menu::open(dir_context(&targets), column, row).expect("a directory has actions")
    }

    #[test]
    fn a_menu_with_no_available_action_does_not_open() {
        let missing = [ActionTarget {
            path: PathBuf::from("/r/gone"),
            is_dir: false,
            expanded: false,
            missing: true,
        }];
        assert!(Menu::open(dir_context(&missing), 0, 0).is_none());
    }

    #[test]
    fn stepping_skips_the_separators_and_stops_at_the_ends() {
        let mut menu = menu_at(0, 0);
        // The entries in order, with a rule between each group.
        let entries: Vec<Action> = menu
            .rows()
            .iter()
            .filter_map(|row| match row {
                MenuRow::Entry(action) => Some(*action),
                MenuRow::Separator => None,
            })
            .collect();
        assert!(
            entries.len() >= 3,
            "the fixture needs several groups to cross"
        );

        assert_eq!(menu.selected_action(), Some(entries[0]));
        for expected in &entries[1..] {
            menu.step(true);
            assert_eq!(menu.selected_action(), Some(*expected));
        }
        // At the last entry, stepping further stops rather than wrapping.
        menu.step(true);
        assert_eq!(menu.selected_action(), entries.last().copied());
        for expected in entries.iter().rev().skip(1) {
            menu.step(false);
            assert_eq!(menu.selected_action(), Some(*expected));
        }
        menu.step(false);
        assert_eq!(menu.selected_action(), Some(entries[0]));

        menu.go_to_edge(false);
        assert_eq!(menu.selected_action(), entries.last().copied());
        menu.go_to_edge(true);
        assert_eq!(menu.selected_action(), Some(entries[0]));
    }

    #[test]
    fn the_highlight_never_lands_on_a_separator() {
        let mut menu = menu_at(0, 0);
        let rule = menu
            .rows()
            .iter()
            .position(|row| matches!(row, MenuRow::Separator))
            .expect("the menu has groups");
        let standing = menu.selected_action();
        menu.highlight(rule);
        assert_eq!(
            menu.selected_action(),
            standing,
            "a rule does not take the highlight"
        );
        let after = menu.action_at(rule + 1).expect("an entry follows the rule");
        menu.highlight(rule + 1);
        assert_eq!(menu.selected_action(), Some(after));
    }

    #[test]
    fn it_shifts_left_at_the_right_edge_rather_than_flipping() {
        let mut menu = menu_at(38, 1);
        let layout = menu.layout(0, 0, 40, 20);
        assert_eq!(
            layout.x + layout.width,
            40,
            "the menu ends flush with the pane instead of overflowing"
        );
        assert!(layout.x < 38, "and so starts left of the click");
        // With room, it starts exactly at the click column.
        let mut menu = menu_at(3, 1);
        assert_eq!(menu.layout(0, 0, 40, 20).x, 3);
    }

    #[test]
    fn it_flips_upward_at_the_bottom() {
        let mut menu = menu_at(0, 19);
        let layout = menu.layout(0, 0, 40, 20);
        assert!(
            layout.y + layout.height <= 20,
            "the menu stays inside the pane"
        );
        assert!(layout.y < 19, "growing upward from the click");
    }

    #[test]
    fn a_narrow_pane_drops_the_accelerators_first() {
        let mut menu = menu_at(0, 0);
        let roomy = menu.layout(0, 0, 40, 20);
        assert!(roomy.accelerators);
        // "Reveal Root Here" is 16 columns; 20 holds it inside the frame but
        // not the accelerator column beside it.
        let tight = menu.layout(0, 0, 20, 20);
        assert!(!tight.accelerators);
        assert_eq!(tight.width, 20);
        assert_eq!(tight.content_width(), 16);
    }

    #[test]
    fn a_short_pane_clamps_and_scrolls_the_highlight_into_view() {
        let mut menu = menu_at(0, 0);
        let layout = menu.layout(0, 0, 40, 4);
        assert_eq!(layout.height, 4, "clamped to the pane");
        assert_eq!(layout.visible_rows(), 2, "two rows inside the frame");
        assert_eq!(menu.scroll(), 0);
        // The last entry does not fit in two rows from the top.
        let last = menu.rows().len() - 1;
        menu.go_to_edge(false);
        menu.layout(0, 0, 40, 4);
        assert_eq!(
            menu.scroll(),
            last + 1 - layout.visible_rows(),
            "the highlight is scrolled into view"
        );
    }

    /// A click on the rule between two groups must do nothing. `highlight`
    /// ignores separators by design, so a caller that moves the highlight and
    /// reads it back would fire whatever entry was standing there instead.
    #[test]
    fn a_separator_has_no_action_to_invoke() {
        let menu = menu_at(0, 0);
        assert_eq!(menu.action_at(0), Some(Action::Open));
        assert_eq!(menu.action_at(99), None, "past the end");
        // Whatever the menu holds, a rule never carries an action and every
        // entry does.
        for (index, row) in menu.rows().iter().enumerate() {
            assert_eq!(
                menu.action_at(index).is_none(),
                matches!(row, MenuRow::Separator),
                "row {index}"
            );
        }
    }

    #[test]
    fn a_hit_on_a_scrolled_menu_accounts_for_the_offset() {
        let mut menu = menu_at(0, 0);
        menu.go_to_edge(false);
        let layout = menu.layout(0, 0, 40, 4);
        assert!(menu.scroll() > 0, "the last entry forced a scroll");
        // The visible window now starts at the scroll offset, and a hit maps
        // through it rather than from the top of the menu.
        let top = menu.scroll();
        assert_eq!(menu.hit(layout, layout.x, layout.y + 1), Some(top));
        assert_eq!(menu.hit(layout, layout.x, layout.y + 2), Some(top + 1));
    }

    #[test]
    fn a_hit_resolves_to_a_row_and_misses_outside() {
        let mut menu = menu_at(4, 2);
        let layout = menu.layout(0, 0, 40, 20);
        // Row y is the top border, so the first entry is one row below it.
        assert_eq!(menu.hit(layout, layout.x, layout.y), None);
        assert_eq!(menu.hit(layout, layout.x, layout.y + 1), Some(0));
        assert_eq!(menu.hit(layout, layout.x, layout.y + 3), Some(2));
        assert_eq!(
            menu.hit(layout, layout.x, layout.y + layout.height - 1),
            None,
            "the bottom border is not an entry"
        );
        assert_eq!(
            menu.hit(layout, layout.x + layout.width, layout.y + 1),
            None
        );
        // ...but the frame is still part of the menu, so a click there is
        // consumed rather than reaching the tree.
        assert!(menu.covers(layout, layout.x, layout.y));
        assert!(!menu.covers(layout, layout.x + layout.width, layout.y));
    }
}
