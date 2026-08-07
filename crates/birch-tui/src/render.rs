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
use crate::theme::{BadgeStyle, FolderStyle, GuideStyle, SelectionStyle, Theme};

pub const INDENT_WIDTH: u16 = 2;
/// Width of the right-hand git badge column.
pub const BADGE_WIDTH: u16 = 2;

/// Width of the scrollbar column, when one is shown.
pub const SCROLLBAR_WIDTH: u16 = 1;

/// Where the scrollbar's thumb sits, in rows from the top of the viewport.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Thumb {
    pub start: usize,
    pub len: usize,
}

/// The thumb for `scroll` over `rows` rows in a `viewport`-row pane, or `None`
/// when everything fits — and equally when the pane is too short for the thumb
/// to move, since a bar pinned at full height carries no information either.
///
/// The thumb touches the top only at the very top and the bottom only at the
/// very bottom: "am I actually at the end?" is the question a scrollbar exists
/// to answer, so rounding must never claim an extreme that is not real. The
/// single exception is documented at the `travel == 1` branch below.
pub fn thumb(rows: usize, viewport: usize, scroll: usize) -> Option<Thumb> {
    if viewport == 0 || rows <= viewport {
        return None;
    }
    let max_scroll = rows - viewport;
    let scroll = scroll.min(max_scroll);
    // Proportional, but never invisible and never the whole track.
    let len = ((viewport * viewport) / rows).clamp(1, viewport.saturating_sub(1).max(1));
    let travel = viewport - len;
    if travel == 0 {
        // A one-row track: the thumb would fill it at every scroll, claiming
        // both ends at once. Show nothing rather than something false.
        return None;
    }
    let start = if scroll == 0 {
        0
    } else if scroll == max_scroll {
        travel
    } else if travel >= 2 {
        // Strictly between the ends, so neither extreme is ever claimed early.
        ((scroll * travel) / max_scroll).clamp(1, travel - 1)
    } else {
        // A track with one free slot cannot show three states. The bottom is
        // the end that matters — "am I at the end?" — so it stays exact, and
        // the top slot is shared with the rows just below it.
        0
    };
    Some(Thumb { start, len })
}

/// Whether a scrollbar column is reserved for this frame.
fn scrollbar_shown(settings: &Settings, rows: usize, viewport: usize, width: u16) -> bool {
    settings.scrollbar
        // The furniture must not eat the last column of the names.
        && width > BADGE_WIDTH + 1 + SCROLLBAR_WIDTH
        && thumb(rows, viewport, 0).is_some()
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
    theme: &Theme,
    bottom_line: &str,
) {
    let area = frame.area();
    if area.height == 0 || area.width == 0 {
        return;
    }
    let viewport = tree_viewport_height(area);
    let badge_width = BADGE_WIDTH.min(area.width);
    // The scrollbar takes the far-right column, pushing the badges left, and
    // only while it is shown — a blank reserved column is real estate paid for
    // nothing in a narrow pane.
    let bar_width = if scrollbar_shown(settings, rows.len(), viewport, area.width) {
        SCROLLBAR_WIDTH
    } else {
        0
    };
    // One gutter column before the badges so truncated text never touches
    // a status indicator.
    let tree_area = Rect {
        width: area.width - bar_width - badge_width - 1.min(area.width - bar_width - badge_width),
        height: viewport as u16,
        ..area
    };
    let badge_area = Rect {
        x: area.x + area.width - bar_width - badge_width,
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
    let mut selected_visible: Option<u16> = None;
    for (i, row) in rows.iter().skip(view.scroll).take(viewport).enumerate() {
        let is_selected = selected == Some(row.path.as_path());
        if is_selected {
            selected_visible = Some(i as u16);
        }
        lines.push(row_line(theme, settings, row, is_selected));
        badges.push(badge_line(theme, row));
    }
    // A theme with an app_bg (the Commander's DOS blue) paints its whole
    // canvas; the area is filled first so the badge gutter matches, and every
    // paragraph carries the fill so empty cells match.
    let canvas = match theme.palette.app_bg {
        Some(bg) => Style::default().bg(bg),
        None => Style::default(),
    };
    if theme.palette.app_bg.is_some() {
        frame.buffer_mut().set_style(area, canvas);
    }
    frame.render_widget(Paragraph::new(lines).style(canvas), tree_area);
    frame.render_widget(Paragraph::new(badges).style(canvas), badge_area);
    if bar_width > 0
        && let Some(Thumb { start, len }) = thumb(rows.len(), viewport, view.scroll)
    {
        let x = area.x + area.width - SCROLLBAR_WIDTH;
        let track = Style::default().fg(theme.palette.guide);
        let grip = Style::default().fg(theme.palette.selection_accent);
        let bar: Vec<Line> = (0..viewport)
            .map(|i| {
                let on_thumb = i >= start && i < start + len;
                let (glyph, style) = if on_thumb {
                    ("\u{2588}", grip)
                } else {
                    ("\u{2502}", track)
                };
                Line::from(Span::styled(glyph, style))
            })
            .collect();
        frame.render_widget(
            Paragraph::new(bar).style(canvas),
            Rect {
                x,
                width: SCROLLBAR_WIDTH,
                height: viewport as u16,
                ..area
            },
        );
    }

    frame.render_widget(
        Paragraph::new(format!(" {bottom_line}")).style(canvas.add_modifier(Modifier::DIM)),
        status_area,
    );

    // The selection wash runs edge to edge (badge gutter included), so the
    // accent bar and the wash read as one gesture, not a floating chip. Cells
    // that carry a deliberate background of their own — the lit characters of
    // a search match — keep it: washing over them would leave their dark
    // foreground on a dark row, i.e. invisible text on the very row the
    // selection sits on.
    if let Some(i) = selected_visible {
        let y = tree_area.y + i;
        let wash = theme.palette.selection_bg;
        let buffer = frame.buffer_mut();
        // Up to, but not including, the scrollbar: the bar reads as furniture
        // outside the row, and in themes where the guide colour equals the
        // wash the track would vanish on the selected row.
        for x in area.left()..area.right() - bar_width {
            let cell = &mut buffer[(x, y)];
            let painted = cell.bg != Color::Reset && Some(cell.bg) != theme.palette.app_bg;
            if !painted {
                cell.set_bg(wash);
            }
        }
    }
}

/// Builds the styled tree line for one row. Pure function of the row, the
/// active theme, and whether the row is selected — the same geometry
/// `hit_test` mirrors (guides and the accent bar occupy the existing indent
/// columns, never shifting the chevron).
fn row_line(theme: &Theme, settings: &Settings, row: &Row, selected: bool) -> Line<'static> {
    let name_style = name_style(theme, row);
    let mut spans = indent_spans(theme, row, selected);
    spans.extend(glyph_column_spans(theme, settings, row));
    spans.extend(label_spans(theme, row, name_style));
    if let Some(annotation) = &row.annotation {
        spans.push(Span::styled(
            format!("  {annotation}"),
            Style::default()
                .fg(theme.palette.separator)
                .add_modifier(Modifier::DIM),
        ));
    }
    if selected && let Some(fg) = theme.palette.selection_fg {
        for span in &mut spans {
            span.style = span.style.fg(fg);
        }
    }
    let mut line = Line::from(spans);
    if selected {
        line = line.style(Style::default().bg(theme.palette.selection_bg));
    }
    line
}

/// The glyph columns between the indent and the label, per `FolderStyle`. The
/// chevron (for a non-missing dir) is ALWAYS the first glyph column, sitting at
/// `depth * INDENT_WIDTH`, so `hit_test`'s chevron zone is identical across all
/// three styles. Let `C` = the theme's chevron (2 cols), `I` = an icon (2 cols),
/// `··` = two blanks:
///
/// - `Icon`    — dir: `C I`, file: `·· I` (two glyph columns; blank keeps icons
///   aligned). With icons off it collapses to the `Plain` layout.
/// - `Compact` — dir: `C`, file: `I` (one glyph column). With icons off files
///   render `··`.
/// - `Plain`   — dir: `C`, file: `··` (one glyph column, never any icon).
///
/// A missing dir (no chevron) renders `··` in that first column in every style.
fn glyph_column_spans(theme: &Theme, settings: &Settings, row: &Row) -> Vec<Span<'static>> {
    let is_dir = row.kind.is_dir();
    let has_chevron = is_dir && !row.missing;
    let chevron = || {
        let glyph = if row.expanded {
            theme.chevron_expanded
        } else {
            theme.chevron_collapsed
        };
        Span::styled(
            format!("{glyph} "),
            Style::default().fg(theme.palette.chevron),
        )
    };
    let icon = || {
        if !settings.icons {
            return None;
        }
        icons::icon_for(theme, &row.name, row.kind).map(|(glyph, color)| {
            let icon_color = if row.ignored {
                theme.palette.ignored
            } else {
                // Hues are theme-owned: a tint overrides devicon colours.
                theme.icon_tint.unwrap_or(color)
            };
            Span::styled(format!("{glyph} "), Style::default().fg(icon_color))
        })
    };
    let blank = || Span::raw("  ");
    let first = || if has_chevron { chevron() } else { blank() };

    let mut spans: Vec<Span<'static>> = Vec::with_capacity(2);
    match theme.folder_style {
        // Two glyph columns: the chevron column, then the icon column. Files
        // blank the chevron column so icons align under directories. When icons
        // are suppressed the icon column simply vanishes (Plain-like layout).
        FolderStyle::Icon => {
            spans.push(first());
            if let Some(glyph) = icon() {
                spans.push(glyph);
            }
        }
        // One glyph column: dirs show their chevron (no folder glyph), files
        // show their icon (or a blank when icons are off).
        FolderStyle::Compact => {
            if is_dir {
                spans.push(first());
            } else {
                spans.push(icon().unwrap_or_else(blank));
            }
        }
        // One glyph column, no icons ever.
        FolderStyle::Plain => spans.push(first()),
    }
    spans
}

/// The right-hand git badge for one row (directory rollups always use the `●`
/// dot; files follow the theme's `BadgeStyle`).
fn badge_line(theme: &Theme, row: &Row) -> Line<'static> {
    match row.status {
        Some(status) if row.kind.is_dir() && !row.missing => Line::from(Span::styled(
            theme.badge_dot,
            Style::default().fg(theme.palette.git.color(status)),
        )),
        Some(status) => {
            let text = match theme.badges {
                BadgeStyle::Letter => status.badge().to_string(),
                BadgeStyle::Symbol => theme.badge_dot.to_string(),
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
    // Depth fade (guide_fade = floor): lerp guide -> floor over levels 1..=4,
    // clamped at the floor so deep trees never lose their guides.
    let level_style = |level: usize| -> Style {
        let Some(floor) = theme.guide_fade else {
            return guide_style;
        };
        let (Color::Rgb(r0, g0, b0), Color::Rgb(r1, g1, b1)) = (theme.palette.guide, floor) else {
            return guide_style;
        };
        let t = [0u16, 45, 75, 100][(level.saturating_sub(1)).min(3)];
        let lerp = |a: u8, b: u8| -> u8 {
            (u16::from(a) + (u16::from(b).saturating_sub(u16::from(a)) * t / 100).min(255)) as u8
        };
        // Components move toward the floor in either direction.
        let mix = |a: u8, b: u8| -> u8 {
            if b >= a {
                lerp(a, b)
            } else {
                (u16::from(a) - (u16::from(a) - u16::from(b)) * t / 100) as u8
            }
        };
        Style::default().fg(Color::Rgb(mix(r0, r1), mix(g0, g1), mix(b0, b1)))
    };
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(depth);
    for level in 0..depth {
        // Level 0 is the root's spine: never a guide, just the accent bar or a
        // blank pad. This keeps the column at INDENT_WIDTH so hit_test is
        // unaffected.
        if level == 0 {
            if accent {
                spans.push(Span::styled(
                    "\u{258f}",
                    Style::default()
                        .fg(theme.palette.selection_accent)
                        .add_modifier(Modifier::BOLD),
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
        spans.push(Span::styled(segment, level_style(level)));
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

/// A match is emphasized, a dim row is muted, and a row that is live without
/// matching (a directory under a glob filter) renders as usual (ADR 0023).
/// A dim row also loses `bold_dirs`: a bold-and-dim directory still reads as
/// prominent, which is the opposite of what "inert" should look like.
fn name_style(theme: &Theme, row: &Row) -> Style {
    let base = base_name_style(theme, row);
    if row.matched {
        base.add_modifier(Modifier::BOLD)
    } else if !row.live {
        base.remove_modifier(Modifier::BOLD)
            .add_modifier(Modifier::DIM)
    } else {
        base
    }
}

fn base_name_style(theme: &Theme, row: &Row) -> Style {
    if row.missing {
        return Style::default()
            .fg(theme.palette.git.color(FileStatus::Deleted))
            .add_modifier(Modifier::CROSSED_OUT);
    }
    // Ignored rows mute via the fg colour step alone — no `DIM` attribute,
    // which renders inconsistently across terminals and double-dims the colour.
    if row.ignored {
        return Style::default().fg(theme.palette.ignored);
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
    settings: &Settings,
    area: Rect,
    column: u16,
    row_y: u16,
) -> Option<(usize, bool)> {
    let viewport = tree_viewport_height(area) as u16;
    if row_y < area.y || row_y >= area.y + viewport {
        return None;
    }
    // While the scrollbar is shown its column is inert: a pure indicator must
    // not double as a selection surface, and this reserves the gesture for a
    // future drag-to-scroll.
    if scrollbar_shown(settings, rows.len(), viewport as usize, area.width)
        && column >= area.x + area.width - SCROLLBAR_WIDTH
    {
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
            live: true,
            matched: false,
            pickable: true,
            match_indices: Vec::new(),
            annotation: None,
            guides: Vec::new(),
            last_sibling: true,
        }
    }

    /// Settings with the scrollbar off, so these cases test row geometry
    /// alone; the bar's own column has its own tests.
    fn no_bar() -> Settings {
        Settings {
            scrollbar: false,
            ..Settings::default()
        }
    }

    #[test]
    fn hit_test_resolves_rows_and_chevrons() {
        let rows = vec![row("src", NodeKind::Dir, 0), row("deep", NodeKind::Dir, 1)];
        let view = FlatView::default();
        let area = Rect::new(0, 0, 40, 10);
        assert_eq!(
            hit_test(&rows, &view, &no_bar(), area, 0, 0),
            Some((0, true))
        );
        assert_eq!(
            hit_test(&rows, &view, &no_bar(), area, 5, 0),
            Some((0, false))
        );
        assert_eq!(
            hit_test(&rows, &view, &no_bar(), area, 2, 1),
            Some((1, true))
        );
        assert_eq!(hit_test(&rows, &view, &no_bar(), area, 0, 5), None);
    }

    #[test]
    fn hit_test_no_chevron_on_missing_dirs() {
        // Missing dirs render no chevron, so the zone must not report one.
        let mut r = row("gone", NodeKind::Dir, 0);
        r.missing = true;
        let rows = vec![r];
        let view = FlatView::default();
        let area = Rect::new(0, 0, 40, 10);
        assert_eq!(
            hit_test(&rows, &view, &no_bar(), area, 0, 0),
            Some((0, false))
        );
    }

    #[test]
    fn hit_test_excludes_status_line() {
        let rows = vec![row("a", NodeKind::File, 0); 20];
        let view = FlatView::default();
        let area = Rect::new(0, 0, 40, 10);
        assert!(hit_test(&rows, &view, &no_bar(), area, 0, 9).is_none());
        assert!(hit_test(&rows, &view, &no_bar(), area, 0, 8).is_some());
    }

    #[test]
    fn hit_test_accounts_for_scroll() {
        let rows: Vec<Row> = (0..20)
            .map(|i| row(&format!("f{i}"), NodeKind::File, 0))
            .collect();
        let mut view = FlatView::default();
        view.scroll = 5;
        let area = Rect::new(0, 0, 40, 10);
        let (idx, _) = hit_test(&rows, &view, &no_bar(), area, 4, 0).unwrap();
        assert_eq!(rows[idx].name, "f5");
        let (idx, _) = hit_test(&rows, &view, &no_bar(), area, 4, 8).unwrap();
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
    fn the_selection_wash_does_not_swallow_the_match_highlight() {
        // The wash is painted over the whole row after the text, so it must
        // skip cells that carry a background of their own. Otherwise the lit
        // characters keep their dark foreground on a dark row and vanish —
        // and the selected row is exactly where the current match sits.
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let theme = theme();
        let mut r = row("main.rs", NodeKind::File, 1);
        r.match_indices = vec![0, 1]; // "ma"
        r.matched = true;
        let rows = vec![r];
        let mut view = FlatView::default();
        view.selection = Some(PathBuf::from("/r").join("main.rs"));

        let mut terminal = Terminal::new(TestBackend::new(40, 3)).unwrap();
        terminal
            .draw(|frame| draw(frame, &rows, &view, &Settings::default(), &theme, ""))
            .unwrap();

        let buffer = terminal.backend().buffer();
        let lit = (0..40)
            .map(|x| buffer[(x, 0)].clone())
            .find(|cell| cell.symbol() == "m")
            .expect("the label is painted");
        assert_eq!(
            lit.bg, theme.palette.match_bg,
            "a matched character keeps its highlight under the selection wash"
        );
        // And the rest of the row still washes, edge to edge.
        assert_eq!(buffer[(39, 0)].bg, theme.palette.selection_bg);
    }

    #[test]
    fn name_style_precedence() {
        let theme = theme();
        let mut r = row("a.rs", NodeKind::File, 0);
        // A plain file takes the theme's name_fg (the flagship sets one).
        assert_eq!(name_style(&theme, &r).fg, theme.palette.name_fg);
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

    /// The glyph texts a row renders between the indent and the label, joined.
    fn glyph_text(theme: &Theme, row: &Row) -> String {
        glyph_column_spans(theme, &Settings::default(), row)
            .iter()
            .map(|s| s.content.to_string())
            .collect()
    }

    #[test]
    fn icon_style_shows_chevron_then_folder_glyph_files_pad_then_icon() {
        // birch is FolderStyle::Icon: dir = chevron + folder glyph; file = two
        // blanks (chevron column) + file icon, so icons align under dirs.
        let theme = theme();
        let dir = row("src", NodeKind::Dir, 1);
        assert_eq!(glyph_text(&theme, &dir), "\u{f460} \u{e5ff} "); // thin chevron + DIR glyph

        let file = row("main.rs", NodeKind::File, 1);
        assert_eq!(glyph_text(&theme, &file), "  \u{e7a8} "); // ·· + rust icon
    }

    #[test]
    fn compact_style_uses_one_glyph_column() {
        // vscode is FolderStyle::Compact: dir = chevron only (no folder glyph);
        // file = its icon only. Names sit one column tighter than Icon.
        let theme = Theme::for_id(ThemeId::Vscode);
        let dir = row("src", NodeKind::Dir, 1);
        let dtext = glyph_text(&theme, &dir);
        assert_eq!(dtext, "\u{eab6} "); // codicon chevron and nothing else
        assert!(!dtext.contains('\u{e5ff}'), "no folder glyph in compact");

        let file = row("main.rs", NodeKind::File, 1);
        assert_eq!(glyph_text(&theme, &file), "\u{e7a8} "); // rust icon only
    }

    #[test]
    fn plain_style_draws_no_icons() {
        // plain is FolderStyle::Plain: dir = chevron; file = two blanks; never
        // an icon.
        let theme = Theme::for_id(ThemeId::Plain);
        let dir = row("src", NodeKind::Dir, 1);
        assert_eq!(glyph_text(&theme, &dir), "\u{25b8} ");

        let file = row("main.rs", NodeKind::File, 1);
        assert_eq!(glyph_text(&theme, &file), "  ");
    }

    #[test]
    fn icon_style_without_icons_setting_collapses_to_plain_layout() {
        // Icons off: the icon column vanishes (dir = chevron, file = ··).
        let theme = theme();
        let settings = Settings {
            icons: false,
            ..Settings::default()
        };
        let dir = row("src", NodeKind::Dir, 1);
        let dtext: String = glyph_column_spans(&theme, &settings, &dir)
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert_eq!(dtext, "\u{f460} ");
        let file = row("main.rs", NodeKind::File, 1);
        let ftext: String = glyph_column_spans(&theme, &settings, &file)
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert_eq!(ftext, "  ");
    }

    #[test]
    fn xcode_uses_thin_disclosure_chevrons() {
        // Modern macOS sidebars use thin chevrons (Big Sur+), not the old
        // filled triangles; retro keeps the filled pair.
        let theme = Theme::for_id(ThemeId::Xcode);
        let mut collapsed = row("src", NodeKind::Dir, 0);
        collapsed.expanded = false;
        // xcode draws Material icons: the md folder-outline glyph.
        assert_eq!(glyph_text(&theme, &collapsed), "\u{eab6} \u{f0256} ");
        let mut expanded = row("src", NodeKind::Dir, 0);
        expanded.expanded = true;
        assert!(glyph_text(&theme, &expanded).starts_with('\u{eab4}'));
    }

    #[test]
    fn missing_dir_pads_the_first_glyph_column_in_every_style() {
        for id in [ThemeId::Birch, ThemeId::Vscode, ThemeId::Plain] {
            let theme = Theme::for_id(id);
            let mut r = row("gone", NodeKind::Dir, 1);
            r.missing = true;
            // No chevron: the first glyph column is two blanks regardless.
            assert!(
                glyph_text(&theme, &r).starts_with("  "),
                "theme {id:?} missing dir must pad the chevron column"
            );
        }
    }

    #[test]
    fn dir_chevron_resolves_at_depth_offset_across_every_folder_style() {
        // hit_test is theme-independent geometry, but confirm each catalog theme
        // keeps the chevron as the first glyph column at depth * INDENT_WIDTH so
        // the chevron zone lines up with what is painted.
        let view = FlatView::default();
        let area = Rect::new(0, 0, 40, 10);
        for id in [
            ThemeId::Birch,
            ThemeId::Vscode,
            ThemeId::Jetbrains,
            ThemeId::Xcode,
            ThemeId::Retro,
            ThemeId::Plain,
        ] {
            let theme = Theme::for_id(id);
            let dir = row("src", NodeKind::Dir, 2);
            // The rendered glyph columns begin with the chevron glyph.
            assert!(
                glyph_text(&theme, &dir).starts_with(theme.chevron_collapsed),
                "theme {id:?} must paint the chevron first"
            );
            // And the chevron zone sits at depth * INDENT_WIDTH.
            let start = 2 * INDENT_WIDTH; // depth 2
            let rows = vec![dir];
            assert_eq!(
                hit_test(&rows, &view, &no_bar(), area, start, 0),
                Some((0, true)),
                "theme {id:?} chevron hit at depth offset"
            );
            assert_eq!(
                hit_test(&rows, &view, &no_bar(), area, start.saturating_sub(1), 0),
                Some((0, false)),
                "theme {id:?} just left of the chevron is not the chevron"
            );
        }
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

    // ---- 068: the scrollbar ----

    #[test]
    fn no_thumb_when_everything_fits() {
        assert_eq!(thumb(10, 10, 0), None);
        assert_eq!(thumb(3, 10, 0), None);
        // A zero-height viewport has no track to draw on.
        assert_eq!(thumb(100, 0, 0), None);
    }

    #[test]
    fn the_thumb_touches_an_end_only_at_that_end() {
        let viewport = 10;
        let rows = 100;
        let max_scroll = rows - viewport;
        let top = thumb(rows, viewport, 0).unwrap();
        assert_eq!(top.start, 0, "top only at the top");
        let bottom = thumb(rows, viewport, max_scroll).unwrap();
        assert_eq!(
            bottom.start + bottom.len,
            viewport,
            "bottom only at the bottom"
        );
        // One row from either end must not claim the extreme.
        let near_top = thumb(rows, viewport, 1).unwrap();
        assert!(near_top.start > 0, "one row down is not the top");
        let near_bottom = thumb(rows, viewport, max_scroll - 1).unwrap();
        assert!(
            near_bottom.start + near_bottom.len < viewport,
            "one row up is not the bottom"
        );
    }

    #[test]
    fn the_thumb_is_never_invisible_and_never_the_whole_track() {
        // A huge tree in a small pane still shows something to grab.
        let t = thumb(1_000_000, 5, 0).unwrap();
        assert_eq!(t.len, 1);
        // A tree barely taller than the pane leaves room to move.
        let t = thumb(11, 10, 0).unwrap();
        assert!(t.len < 10, "a full-height thumb reports nothing");
        assert!(t.len >= 1);
    }

    #[test]
    fn the_thumb_stays_inside_the_track_at_every_scroll() {
        for rows in [11usize, 40, 999, 10_000] {
            for viewport in [1usize, 2, 7, 45] {
                if rows <= viewport {
                    continue;
                }
                for scroll in [0, 1, 3, rows - viewport - 1, rows - viewport, rows] {
                    let Some(t) = thumb(rows, viewport, scroll) else {
                        // The only track with no room to move is a one-row one.
                        assert_eq!(viewport, 1, "rows={rows} viewport={viewport}");
                        continue;
                    };
                    assert!(
                        t.start + t.len <= viewport,
                        "rows={rows} viewport={viewport} scroll={scroll} overflowed"
                    );
                    assert!(t.len >= 1);
                }
            }
        }
    }

    #[test]
    fn a_track_with_no_travel_shows_nothing() {
        // A one-row track would hold a full-height thumb at every scroll,
        // claiming the top and the bottom at once. Better to show no bar.
        assert_eq!(thumb(100, 1, 0), None);
        assert_eq!(thumb(100, 1, 99), None);
    }

    #[test]
    fn a_single_slot_track_keeps_the_bottom_exact() {
        // Two rows of track leave one free slot: the bottom must still mean
        // the bottom, even though the top slot is shared.
        let (rows, viewport) = (100, 2);
        assert_eq!(thumb(rows, viewport, 0).unwrap().start, 0);
        assert_eq!(thumb(rows, viewport, 50).unwrap().start, 0);
        let bottom = thumb(rows, viewport, rows - viewport).unwrap();
        assert_eq!(bottom.start + bottom.len, viewport);
    }

    #[test]
    fn the_layout_holds_in_a_pane_too_narrow_for_the_furniture() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;
        let theme = theme();
        let rows: Vec<Row> = (0..40)
            .map(|i| row(&format!("f{i}"), NodeKind::File, 0))
            .collect();
        let view = FlatView::default();
        // Every pane from one cell wide upward must draw without panicking on
        // the u16 arithmetic that reserves the bar and badge columns.
        for width in 1..=12u16 {
            for height in 1..=4u16 {
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal
                    .draw(|frame| draw(frame, &rows, &view, &Settings::default(), &theme, ""))
                    .unwrap();
            }
        }
    }

    #[test]
    fn the_selection_wash_stops_before_the_scrollbar() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;
        // In themes where the guide colour equals the wash, washing over the
        // bar would erase the track on the selected row.
        let theme = theme();
        let rows: Vec<Row> = (0..40)
            .map(|i| row(&format!("f{i}"), NodeKind::File, 0))
            .collect();
        let mut view = FlatView::default();
        view.selection = Some(rows[0].path.clone());
        let (w, h) = (30u16, 4u16);
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal
            .draw(|frame| draw(frame, &rows, &view, &Settings::default(), &theme, ""))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let bar_cell = &buffer[(w - 1, 0)];
        assert_ne!(
            bar_cell.bg, theme.palette.selection_bg,
            "the wash reached the scrollbar column"
        );
    }

    #[test]
    fn the_bar_column_is_inert_while_shown() {
        let rows: Vec<Row> = (0..50)
            .map(|i| row(&format!("f{i}"), NodeKind::File, 0))
            .collect();
        let view = FlatView::default();
        let area = Rect::new(0, 0, 40, 10);
        let on = Settings::default();
        // The far-right column belongs to the bar, not to the row behind it.
        assert_eq!(hit_test(&rows, &view, &on, area, 39, 0), None);
        assert!(hit_test(&rows, &view, &on, area, 38, 0).is_some());
        // With the bar off, the same column selects again.
        assert!(hit_test(&rows, &view, &no_bar(), area, 39, 0).is_some());
    }

    #[test]
    fn the_bar_column_stays_live_when_the_rows_fit() {
        let rows = vec![row("only", NodeKind::File, 0)];
        let view = FlatView::default();
        let area = Rect::new(0, 0, 40, 10);
        assert!(hit_test(&rows, &view, &Settings::default(), area, 39, 0).is_some());
    }
}
