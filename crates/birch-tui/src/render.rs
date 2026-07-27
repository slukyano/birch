//! Painting: rows → terminal. Everything here is a pure function of the
//! view-model state; hit-testing mirrors the same geometry.

use birch_core::{FileStatus, Settings};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::flat_view::{FlatView, Row};
use crate::icons;
use crate::theme::{BadgeStyle, GuideStyle, SelectionStyle, Theme};

pub const INDENT_WIDTH: u16 = 2;
/// Width of the right-hand git badge column.
pub const BADGE_WIDTH: u16 = 2;

/// The tree gets everything above the one-line status bar.
pub fn tree_viewport_height(area: Rect) -> usize {
    area.height.saturating_sub(1) as usize
}

pub fn draw(
    frame: &mut Frame,
    rows: &[Row],
    view: &FlatView,
    settings: &Settings,
    theme: &Theme,
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
        let is_selected = selected == Some(row.path.as_path());
        lines.push(row_line(theme, settings, row, is_selected));
        badges.push(badge_line(theme, row));
    }
    frame.render_widget(Paragraph::new(lines), tree_area);
    frame.render_widget(Paragraph::new(badges), badge_area);

    frame.render_widget(
        Paragraph::new(format!(" {bottom_line}"))
            .style(Style::default().add_modifier(Modifier::DIM)),
        status_area,
    );
}

/// Builds the styled tree line for one row. Pure function of the row, the
/// active theme, and whether the row is selected — the same geometry
/// `hit_test` mirrors (guides and the accent bar occupy the existing indent
/// columns, never shifting the chevron).
fn row_line(theme: &Theme, settings: &Settings, row: &Row, selected: bool) -> Line<'static> {
    let name_style = name_style(theme, row);
    let mut spans = indent_spans(theme, row, selected);
    if row.kind.is_dir() && !row.missing {
        let chevron = if row.expanded {
            "\u{25be} "
        } else {
            "\u{25b8} "
        };
        spans.push(Span::styled(
            chevron,
            Style::default().fg(theme.palette.chevron),
        ));
    } else {
        spans.push(Span::raw("  "));
    }
    if settings.icons
        && let Some((glyph, color)) = icons::icon_for(theme, &row.name, row.kind)
    {
        let icon_color = if row.ignored {
            theme.palette.ignored
        } else {
            color
        };
        spans.push(Span::styled(
            format!("{glyph} "),
            Style::default().fg(icon_color),
        ));
    }
    spans.extend(label_spans(theme, row, name_style));
    if let Some(annotation) = &row.annotation {
        spans.push(Span::styled(
            format!("  {annotation}"),
            Style::default()
                .fg(theme.palette.separator)
                .add_modifier(Modifier::DIM),
        ));
    }
    let mut line = Line::from(spans);
    if selected {
        line = line.style(Style::default().bg(theme.palette.selection_bg));
    }
    line
}

/// The right-hand git badge for one row (directory rollups always use the `●`
/// dot; files follow the theme's `BadgeStyle`).
fn badge_line(theme: &Theme, row: &Row) -> Line<'static> {
    match row.status {
        Some(status) if row.kind.is_dir() && !row.missing => Line::from(Span::styled(
            "\u{25cf}",
            Style::default().fg(theme.palette.git.color(status)),
        )),
        Some(status) => {
            let text = match theme.badges {
                BadgeStyle::Letter => status.badge().to_string(),
                BadgeStyle::Symbol => "\u{25cf}".to_string(),
            };
            Line::from(Span::styled(
                text,
                Style::default().fg(theme.palette.git.color(status)),
            ))
        }
        None => Line::default(),
    }
}

/// The ancestor-indent columns (`INDENT_WIDTH` each). Renders the theme's
/// guide glyph, and — for a selected row under `SoftBarAccent` — turns the
/// outermost column into the left accent bar (`▏`), keeping the total width
/// identical so `hit_test` geometry is unchanged.
///
/// The outermost column (`level 0`) is never a guide: it is the root's spine,
/// which always runs full-height and is uninformative. It is the selection
/// accent bar (selected row, `SoftBarAccent`) or two blank spaces. Guides
/// (`Indent` lines and `Connectors`) begin at `level >= 1`.
///
/// A depth-0 (root) row has no indent columns, so its accent bar cannot be
/// drawn without clobbering the chevron; the soft background still marks it.
fn indent_spans(theme: &Theme, row: &Row, selected: bool) -> Vec<Span<'static>> {
    let depth = row.depth;
    let guide_style = Style::default().fg(theme.palette.guide);
    let accent = selected && theme.selection == SelectionStyle::SoftBarAccent;
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(depth);
    for level in 0..depth {
        // Level 0 is the root's spine: never a guide, just the accent bar or a
        // blank pad. This keeps the column at INDENT_WIDTH so hit_test is
        // unaffected.
        if level == 0 {
            if accent {
                spans.push(Span::styled(
                    "\u{258f}",
                    Style::default().fg(theme.palette.selection_accent),
                ));
                spans.push(Span::styled(" ", guide_style));
            } else {
                spans.push(Span::raw("  "));
            }
            continue;
        }
        let segment = match theme.guides {
            GuideStyle::None => "  ",
            GuideStyle::Indent => "\u{2502} ",
            GuideStyle::Connectors => connector_segment(row, level),
        };
        spans.push(Span::styled(segment, guide_style));
    }
    spans
}

/// The classic-connector glyph for column `level` (`>= 1`) of a row. The final
/// column (`level == depth - 1`) is the row's own connector — `└─` if it is the
/// last sibling, else `├─`. Interior columns draw a `│ ` continuation when the
/// ancestor at that depth still has a following sibling below, else blank. Each
/// segment is exactly `INDENT_WIDTH` (2) columns wide.
fn connector_segment(row: &Row, level: usize) -> &'static str {
    if level + 1 == row.depth {
        if row.last_sibling {
            "\u{2514}\u{2500}" // └─
        } else {
            "\u{251c}\u{2500}" // ├─
        }
    } else if row.guides.get(level).copied().unwrap_or(false) {
        "\u{2502} " // │
    } else {
        "  "
    }
}

/// Renders a row label with dim chain separators and lit match characters
/// (ADR 0013). A hit without char detail (path-mode in the tree) keeps the
/// whole-label bold from `name_style`.
fn label_spans(theme: &Theme, row: &Row, base: Style) -> Vec<Span<'static>> {
    let separator = Style::default().fg(theme.palette.separator);
    let lit = base
        .bg(theme.palette.match_bg)
        .fg(theme.palette.match_fg)
        .add_modifier(Modifier::BOLD);
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

fn name_style(theme: &Theme, row: &Row) -> Style {
    let base = base_name_style(theme, row);
    match row.search {
        Some(true) => base.add_modifier(Modifier::BOLD),
        Some(false) => base.add_modifier(Modifier::DIM),
        None => base,
    }
}

fn base_name_style(theme: &Theme, row: &Row) -> Style {
    if row.missing {
        return Style::default()
            .fg(theme.palette.git.color(FileStatus::Deleted))
            .add_modifier(Modifier::CROSSED_OUT);
    }
    if row.ignored {
        return Style::default()
            .fg(theme.palette.ignored)
            .add_modifier(Modifier::DIM);
    }
    let mut style = match row.status {
        Some(status) => Style::default().fg(theme.palette.git.color(status)),
        None => {
            let fg = if row.kind.is_dir() {
                theme.palette.dir_fg
            } else {
                theme.palette.name_fg
            };
            match fg {
                Some(color) => Style::default().fg(color),
                None => Style::default(),
            }
        }
    };
    if row.kind.is_dir() && theme.bold_dirs {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
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

    use birch_core::{NodeKind, ThemeId};

    use super::*;

    fn theme() -> Theme {
        Theme::for_id(ThemeId::Birch)
    }

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
            guides: Vec::new(),
            last_sibling: true,
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
        let theme = theme();
        let mut r = row("a/b/cc", NodeKind::Dir, 0);
        r.chain = vec![PathBuf::from("/r/a"), PathBuf::from("/r/a/b")];
        r.match_indices = vec![4, 5]; // "cc"
        let spans = label_spans(&theme, &r, Style::default());
        let texts: Vec<&str> = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(texts, ["a", "/", "b", "/", "cc"]);
        // Separators are dim-styled, the matched run is lit.
        assert_ne!(spans[1].style, spans[0].style);
        assert_eq!(spans[1].style, spans[3].style);
        assert_eq!(spans[4].style.bg, Some(theme.palette.match_bg));
        assert_eq!(spans[4].style.fg, Some(theme.palette.match_fg));
        assert!(spans[4].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn name_style_precedence() {
        let theme = theme();
        let mut r = row("a.rs", NodeKind::File, 0);
        // A plain file keeps the terminal default (Birch's name_fg is None).
        assert_eq!(name_style(&theme, &r), Style::default());
        r.status = Some(FileStatus::Modified);
        assert_eq!(
            name_style(&theme, &r).fg,
            Some(theme.palette.git.color(FileStatus::Modified))
        );
        r.ignored = true;
        assert_eq!(name_style(&theme, &r).fg, Some(theme.palette.ignored));
        r.missing = true;
        assert!(
            name_style(&theme, &r)
                .add_modifier
                .contains(Modifier::CROSSED_OUT)
        );
    }

    #[test]
    fn directories_are_bold_under_birch() {
        let theme = theme();
        let dir = row("src", NodeKind::Dir, 0);
        assert!(
            name_style(&theme, &dir)
                .add_modifier
                .contains(Modifier::BOLD)
        );
        let file = row("a.rs", NodeKind::File, 0);
        assert!(
            !name_style(&theme, &file)
                .add_modifier
                .contains(Modifier::BOLD)
        );
    }

    #[test]
    fn birch_paints_indent_guides_but_never_the_leftmost_column() {
        let theme = theme();
        // A depth-2 row (unselected): level 0 is the root spine (blank), level 1
        // a dim vertical guide — each two columns wide, so the chevron still
        // starts at depth * 2.
        let spans = indent_spans(&theme, &row("d", NodeKind::File, 2), false);
        let texts: Vec<&str> = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(texts, ["  ", "\u{2502} "]);
        // The level-1 guide carries the guide color.
        assert_eq!(spans[1].style.fg, Some(theme.palette.guide));
        let width: usize = texts.iter().map(|t| t.chars().count()).sum();
        assert_eq!(width, 2 * INDENT_WIDTH as usize);
    }

    #[test]
    fn birch_selection_paints_a_left_accent_bar() {
        let theme = theme();
        // Selected: the outermost indent column becomes the accent bar (▏),
        // width preserved so hit-test geometry is unchanged.
        let spans = indent_spans(&theme, &row("deep", NodeKind::File, 1), true);
        assert_eq!(spans[0].content.as_ref(), "\u{258f}");
        assert_eq!(spans[0].style.fg, Some(theme.palette.selection_accent));
        let width: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        assert_eq!(width, INDENT_WIDTH as usize);

        // A whole selected row also carries the soft selection background.
        let r = row("deep", NodeKind::File, 1);
        let line = row_line(&theme, &Settings::default(), &r, true);
        assert_eq!(line.style.bg, Some(theme.palette.selection_bg));
    }

    #[test]
    fn vscode_dirs_drop_the_folder_glyph_files_keep_icons() {
        let theme = Theme::for_id(ThemeId::Vscode);
        // A directory: the chevron stands in for the folder glyph, and there is
        // no icon gap (no folder glyph span at all).
        let dir = row("src", NodeKind::Dir, 1);
        let line = row_line(&theme, &Settings::default(), &dir, false);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains('\u{25b8}'), "chevron present"); // ▸
        assert!(!text.contains('\u{e5ff}'), "no folder glyph"); // DIR
        assert!(text.ends_with("src"));

        // A file still gets its type icon.
        let file = row("main.rs", NodeKind::File, 1);
        let fline = row_line(&theme, &Settings::default(), &file, false);
        let ftext: String = fline.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(ftext.contains('\u{e7a8}'), "rust icon present");
    }

    #[test]
    fn leftmost_column_is_never_a_guide() {
        // For every guide style, a depth-2 unselected row leaves level 0 blank
        // (the root spine is never drawn).
        for id in [
            ThemeId::Birch,
            ThemeId::Vscode,
            ThemeId::Retro,
            ThemeId::Plain,
        ] {
            let theme = Theme::for_id(id);
            let mut r = row("d", NodeKind::File, 2);
            r.guides = vec![true, true];
            let spans = indent_spans(&theme, &r, false);
            assert_eq!(spans[0].content.as_ref(), "  ", "theme {id:?}");
        }
    }

    #[test]
    fn connectors_draw_branch_last_and_continuation_glyphs() {
        let retro = Theme::for_id(ThemeId::Retro);
        // A depth-2 middle child (not last): level 0 blank, level 1 is its own
        // connector ├─.
        let mut middle = row("mid", NodeKind::File, 2);
        middle.guides = vec![true];
        middle.last_sibling = false;
        let texts: Vec<String> = indent_spans(&retro, &middle, false)
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert_eq!(texts, ["  ", "\u{251c}\u{2500}"]);

        // A depth-2 last child: its own connector is └─.
        let mut last = row("last", NodeKind::File, 2);
        last.guides = vec![true];
        last.last_sibling = true;
        let texts: Vec<String> = indent_spans(&retro, &last, false)
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert_eq!(texts, ["  ", "\u{2514}\u{2500}"]);

        // A depth-3 row under a non-last ancestor: level 0 blank, level 1 draws
        // the ancestor's │ continuation, level 2 is the row's own connector.
        let mut deep = row("deep", NodeKind::File, 3);
        deep.guides = vec![true, true]; // ancestor at depth 2 has a following sibling
        deep.last_sibling = true;
        let texts: Vec<String> = indent_spans(&retro, &deep, false)
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert_eq!(texts, ["  ", "\u{2502} ", "\u{2514}\u{2500}"]);

        // Same depth-3 row but the ancestor was the last sibling: continuation
        // column goes blank.
        deep.guides = vec![true, false];
        let texts: Vec<String> = indent_spans(&retro, &deep, false)
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert_eq!(texts, ["  ", "  ", "\u{2514}\u{2500}"]);
    }
}
