//! birch-tui: the render layer — compaction, badges, widget, mouse, context
//! menu, inline edit. Paint-time transforms over the real tree from
//! `birch-core` (see `docs/design.md`).
//!
//! The view-model (`flat_view`) lives here per ADR 0003 but is pure logic
//! with no ratatui types, so it stays unit-testable without a terminal.

pub mod flat_view;
pub mod icons;
pub mod input;
pub mod render;

pub use flat_view::{FlatView, NavEffect, Row};
pub use input::InputAction;
