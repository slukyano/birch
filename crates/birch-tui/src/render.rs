//! Painting: rows → terminal. Everything here is a pure function of the
//! view-model state; hit-testing mirrors the same geometry.

use birch_core::{FileStatus, Settings};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::flat_view::{FlatView, Row};
use crate::icons;

pub const INDENT_WIDTH: u16 = 2;
/// Width of the right-hand git badge column.
pub const BADGE_WIDTH: u16 = 2;

const SELECTION_BG: Color = Color::Rgb(0x2f, 0x3b, 0x54);
const CHEVRON_COLOR: Color = Color::Rgb(0x6d, 0x80, 0x86);
const SEPARATOR_COLOR: Color = Color::Rgb(0x6d, 0x80, 0x86);
const IGNORED_COLOR: Color = Color::Rgb(0x7a, 0x82, 0x8e);
/// Matched search characters render as IDEA-style boxes (ADR 0013).
const MATCH_BG: Color = Color::Rgb(0xb8, 0x86, 0x2d);
const MATCH_FG: Color = Color::Rgb(0x1a, 0x1b, 0x26);

pub fn status_color(status: FileStatus) -> Color {
    match status {
        FileStatus::Conflicted => Color::Rgb(0xe4, 0x67, 0x6b),
        FileStatus::Deleted => Color::Rgb(0xc7, 0x4e, 0x39),
        FileStatus::Renamed => Color::Rgb(0x73, 0xc9, 0x91),
        FileStatus::Modified => Color::Rgb(0xe2, 0xc0, 0x8d),
        FileStatus::Added => Color::Rgb(0x81, 0xb8, 0x8b),
        FileStatus::Untracked => Color::Rgb(0x73, 0xc9, 0x91),
    }
}

/// The tree gets everything above the one-line status bar.
pub fn tree_viewport_height(area: Rect) -> usize {
    area.height.saturating_sub(1) as usize
}

pub fn draw(
    frame: &mut Frame,
    rows: &[Row],
    view: &FlatView,
    settings: &Settings,
    bottom_line: &str,
) {
    let area = frame.area();
    if area.height == 0 || area.width == 0 {
        return;
    }
    let viewport = tree_viewport_height(area);
    let badge_width = BADGE_WIDTH.min(area.width);
    let tree_area = Rect {
        width: area.width - badge_width,
        height: viewport as u16,
        ..area
    };
    let badge_area = Rect {
        x: area.x + area.width - badge_width,
        width: badge_width,
        height: viewport as u16,
        ..area
    };
    let status_area = Rect {
        y: area.y + area.height.saturating_sub(1),
        height: 1,
        ..area
    };

    let selected = view.selection.as_deref();
    let mut lines = Vec::with_capacity(viewport);
    let mut badges = Vec::with_capacity(viewport);
    for row in rows.iter().skip(view.scroll).take(viewport) {
        let name_style = name_style(row);
        let mut spans = vec![Span::raw(
            " ".repeat((row.depth as u16 * INDENT_WIDTH) as usize),
        )];
        if row.kind.is_dir() && !row.missing {
            let chevron = if row.expanded {
                "\u{25be} "
            } else {
                "\u{25b8} "
            };
            spans.push(Span::styled(chevron, Style::default().fg(CHEVRON_COLOR)));
        } else {
            spans.push(Span::raw("  "));
        }
        if settings.icons {
            let (glyph, color) = icons::icon_for(&row.name, row.kind);
            let icon_color = if row.ignored { IGNORED_COLOR } else { color };
            spans.push(Span::styled(
                format!("{glyph} "),
                Style::default().fg(icon_color),
            ));
        }
        spans.extend(label_spans(row, name_style));
        if let Some(annotation) = &row.annotation {
            spans.push(Span::styled(
                format!("  {annotation}"),
                Style::default()
                    .fg(SEPARATOR_COLOR)
                    .add_modifier(Modifier::DIM),
            ));
        }
        let mut line = Line::from(spans);
        if selected == Some(row.path.as_path()) {
            line = line.style(Style::default().bg(SELECTION_BG));
        }
        lines.push(line);

        badges.push(match row.status {
            Some(status) if row.kind.is_dir() && !row.missing => Line::from(Span::styled(
                "\u{25cf}",
                Style::default().fg(status_color(status)),
            )),
            Some(status) => Line::from(Span::styled(
                status.badge().to_string(),
                Style::default().fg(status_color(status)),
            )),
            None => Line::default(),
        });
    }
    frame.render_widget(Paragraph::new(lines), tree_area);
    frame.render_widget(Paragraph::new(badges), badge_area);

    frame.render_widget(
        Paragraph::new(format!(" {bottom_line}"))
            .style(Style::default().add_modifier(Modifier::DIM)),
        status_area,
    );
}

/// Renders a row label with dim chain separators and lit match characters
/// (ADR 0013). A hit without char detail (path-mode in the tree) keeps the
/// whole-label bold from `name_style`.
fn label_spans(row: &Row, base: Style) -> Vec<Span<'static>> {
    let separator = Style::default().fg(SEPARATOR_COLOR);
    let lit = base.bg(MATCH_BG).fg(MATCH_FG).add_modifier(Modifier::BOLD);
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut run = String::new();
    let mut run_style: Option<Style> = None;
    for (i, ch) in row.name.chars().enumerate() {
        let style = if !row.chain.is_empty() && ch == '/' {
            separator
        } else if row.match_indices.binary_search(&(i as u32)).is_ok() {
            lit
        } else {
            base
        };
        if run_style != Some(style) {
            if let Some(prev) = run_style {
                spans.push(Span::styled(std::mem::take(&mut run), prev));
            }
            run_style = Some(style);
        }
        run.push(ch);
    }
    if let Some(prev) = run_style {
        spans.push(Span::styled(run, prev));
    }
    spans
}

fn name_style(row: &Row) -> Style {
    let base = base_name_style(row);
    match row.search {
        Some(true) => base.add_modifier(Modifier::BOLD),
        Some(false) => base.add_modifier(Modifier::DIM),
        None => base,
    }
}

fn base_name_style(row: &Row) -> Style {
    if row.missing {
        return Style::default()
            .fg(status_color(FileStatus::Deleted))
            .add_modifier(Modifier::CROSSED_OUT);
    }
    if row.ignored {
        return Style::default()
            .fg(IGNORED_COLOR)
            .add_modifier(Modifier::DIM);
    }
    match row.status {
        Some(status) => Style::default().fg(status_color(status)),
        None => Style::default(),
    }
}

/// Resolves a click at terminal coordinates to a row index and whether it hit
/// the chevron cell of a directory row. Returns `None` for clicks outside the
/// rows (the below-the-tree case gains meaning with the context menu, 0.5).
pub fn hit_test(
    rows: &[Row],
    view: &FlatView,
    area: Rect,
    column: u16,
    row_y: u16,
) -> Option<(usize, bool)> {
    let viewport = tree_viewport_height(area) as u16;
    if row_y < area.y || row_y >= area.y + viewport {
        return None;
    }
    let idx = view.scroll + (row_y - area.y) as usize;
    let row = rows.get(idx)?;
    let chevron_start = area.x + row.depth as u16 * INDENT_WIDTH;
    // Missing dirs render no chevron, so their chevron zone must not act
    // like one (it would activate on a single click).
    let on_chevron = row.kind.is_dir()
        && !row.missing
        && column >= chevron_start
        && column < chevron_start + INDENT_WIDTH;
    Some((idx, on_chevron))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use birch_core::NodeKind;

    use super::*;

    fn row(name: &str, kind: NodeKind, depth: usize) -> Row {
        Row {
            path: PathBuf::from("/r").join(name),
            name: name.into(),
            kind,
            depth,
            expanded: false,
            loaded: true,
            chain: Vec::new(),
            status: None,
            ignored: false,
            missing: false,
            search: None,
            match_indices: Vec::new(),
            annotation: None,
        }
    }

    #[test]
    fn hit_test_resolves_rows_and_chevrons() {
        let rows = vec![row("src", NodeKind::Dir, 0), row("deep", NodeKind::Dir, 1)];
        let view = FlatView::default();
        let area = Rect::new(0, 0, 40, 10);
        assert_eq!(hit_test(&rows, &view, area, 0, 0), Some((0, true)));
        assert_eq!(hit_test(&rows, &view, area, 5, 0), Some((0, false)));
        assert_eq!(hit_test(&rows, &view, area, 2, 1), Some((1, true)));
        assert_eq!(hit_test(&rows, &view, area, 0, 5), None);
    }

    #[test]
    fn hit_test_no_chevron_on_missing_dirs() {
        // Missing dirs render no chevron, so the zone must not report one.
        let mut r = row("gone", NodeKind::Dir, 0);
        r.missing = true;
        let rows = vec![r];
        let view = FlatView::default();
        let area = Rect::new(0, 0, 40, 10);
        assert_eq!(hit_test(&rows, &view, area, 0, 0), Some((0, false)));
    }

    #[test]
    fn hit_test_excludes_status_line() {
        let rows = vec![row("a", NodeKind::File, 0); 20];
        let view = FlatView::default();
        let area = Rect::new(0, 0, 40, 10);
        assert!(hit_test(&rows, &view, area, 0, 9).is_none());
        assert!(hit_test(&rows, &view, area, 0, 8).is_some());
    }

    #[test]
    fn hit_test_accounts_for_scroll() {
        let rows: Vec<Row> = (0..20)
            .map(|i| row(&format!("f{i}"), NodeKind::File, 0))
            .collect();
        let mut view = FlatView::default();
        view.scroll = 5;
        let area = Rect::new(0, 0, 40, 10);
        let (idx, _) = hit_test(&rows, &view, area, 4, 0).unwrap();
        assert_eq!(rows[idx].name, "f5");
        let (idx, _) = hit_test(&rows, &view, area, 4, 8).unwrap();
        assert_eq!(rows[idx].name, "f13");
    }

    #[test]
    fn label_spans_group_runs_and_dim_separators() {
        let mut r = row("a/b/cc", NodeKind::Dir, 0);
        r.chain = vec![PathBuf::from("/r/a"), PathBuf::from("/r/a/b")];
        r.match_indices = vec![4, 5]; // "cc"
        let spans = label_spans(&r, Style::default());
        let texts: Vec<&str> = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(texts, ["a", "/", "b", "/", "cc"]);
        // Separators are dim-styled, the matched run is lit.
        assert_ne!(spans[1].style, spans[0].style);
        assert_eq!(spans[1].style, spans[3].style);
        assert_eq!(spans[4].style.bg, Some(MATCH_BG));
        assert_eq!(spans[4].style.fg, Some(MATCH_FG));
        assert!(spans[4].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn name_style_precedence() {
        let mut r = row("a.rs", NodeKind::File, 0);
        assert_eq!(name_style(&r), Style::default());
        r.status = Some(FileStatus::Modified);
        assert_eq!(name_style(&r).fg, Some(status_color(FileStatus::Modified)));
        r.ignored = true;
        assert_eq!(name_style(&r).fg, Some(IGNORED_COLOR));
        r.missing = true;
        assert!(name_style(&r).add_modifier.contains(Modifier::CROSSED_OUT));
    }
}
