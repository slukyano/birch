//! The theme abstraction (ADR 0021): the whole paint layer's visual
//! vocabulary — colors, glyphs, guide/selection/badge styles — resolved from
//! a `birch_core::ThemeId`. Everything ratatui about "what the tree looks
//! like" lives here, so `birch-core` stays ratatui-free and only *names* the
//! theme.

use birch_core::{FileStatus, ThemeId};
use ratatui::style::Color;

/// How ancestor indentation is drawn. All variants render inside the existing
/// indent columns (`INDENT_WIDTH` per level), so `hit_test` geometry is never
/// affected.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GuideStyle {
    /// Plain spaces — an indented list, no vertical lines.
    None,
    /// A dim `│` in each ancestor indent column (VS Code-style).
    Indent,
    /// Classic `├──`/`└──`/`│` connectors. Not yet realized — needs
    /// sibling/last-child data from `flat_view`; rendered as `Indent` guides
    /// until then. TODO(025): real connectors.
    Connectors,
}

/// How the selected row is highlighted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelectionStyle {
    /// A single background fill across the whole row.
    FullRow,
    /// A soft background plus a left accent bar (`▏`) in the accent color.
    SoftBarAccent,
}

/// How a git status badge is drawn for a file row (directory rollups always
/// use the `●` dot).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BadgeStyle {
    /// A status letter (`M`/`A`/`D`/…).
    Letter,
    /// A `●` dot in the status color.
    Symbol,
}

/// Which glyph map supplies per-file icons.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IconSet {
    /// No icons at all (e.g. the `plain` theme).
    None,
    /// The built-in Nerd Font map (`icons.rs`).
    NerdFont,
}

/// Per-`FileStatus` git colors.
#[derive(Clone, Copy, Debug)]
pub struct GitColors {
    pub conflicted: Color,
    pub deleted: Color,
    pub renamed: Color,
    pub modified: Color,
    pub added: Color,
    pub untracked: Color,
}

impl GitColors {
    pub fn color(&self, status: FileStatus) -> Color {
        match status {
            FileStatus::Conflicted => self.conflicted,
            FileStatus::Deleted => self.deleted,
            FileStatus::Renamed => self.renamed,
            FileStatus::Modified => self.modified,
            FileStatus::Added => self.added,
            FileStatus::Untracked => self.untracked,
        }
    }
}

/// Every color the render layer needs. `name_fg`/`dir_fg` are `Option`: `None`
/// leaves the terminal's default foreground (so a theme can be palette-neutral).
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// File-name foreground (`None` = terminal default).
    pub name_fg: Option<Color>,
    /// Directory-name foreground (`None` = terminal default).
    pub dir_fg: Option<Color>,
    /// Selection background (soft fill or full-row fill).
    pub selection_bg: Color,
    /// The left accent-bar color for `SoftBarAccent`.
    pub selection_accent: Color,
    /// Indent-guide line color.
    pub guide: Color,
    pub chevron: Color,
    pub separator: Color,
    pub ignored: Color,
    /// Search-match box (IDEA-style, ADR 0013).
    pub match_bg: Color,
    pub match_fg: Color,
    pub git: GitColors,
}

/// The resolved visual vocabulary for one theme.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub id: ThemeId,
    pub palette: Palette,
    pub guides: GuideStyle,
    pub selection: SelectionStyle,
    pub badges: BadgeStyle,
    pub icons: IconSet,
    /// Directory names are rendered bold.
    pub bold_dirs: bool,
    /// Show a folder glyph for directories; when false the chevron stands in
    /// its place (VS Code-style), even with icons on.
    pub folder_icon: bool,
}

impl Theme {
    /// Resolves a `ThemeId` to its `Theme`. Only `Birch` and `Plain` are fully
    /// realized here; the emulation themes are placeholders mapped onto
    /// `Birch` for now. TODO(025): distinct `vscode`/`jetbrains`/`xcode`/`retro`
    /// themes.
    pub fn for_id(id: ThemeId) -> Theme {
        match id {
            ThemeId::Birch => Theme::birch(),
            ThemeId::Plain => Theme::plain(),
            // TODO(025): distinct theme — placeholder mapped onto Birch.
            ThemeId::Vscode | ThemeId::Jetbrains | ThemeId::Xcode | ThemeId::Retro => {
                let mut theme = Theme::birch();
                theme.id = id;
                theme
            }
        }
    }

    /// The flagship theme (task 054): curated muted palette, dim indent guides,
    /// a soft selection with a birch-green left accent bar, bold directories.
    fn birch() -> Theme {
        Theme {
            id: ThemeId::Birch,
            palette: Palette {
                name_fg: None,
                dir_fg: None,
                // Softer / lower-contrast than the previous #2f3b54 full-row fill.
                selection_bg: Color::Rgb(0x28, 0x30, 0x40),
                // Birch green, echoing the logo.
                selection_accent: Color::Rgb(0x6f, 0x91, 0x52),
                guide: Color::Rgb(0x3a, 0x42, 0x50),
                chevron: Color::Rgb(0x6d, 0x80, 0x86),
                separator: Color::Rgb(0x6d, 0x80, 0x86),
                ignored: Color::Rgb(0x7a, 0x82, 0x8e),
                match_bg: Color::Rgb(0xb8, 0x86, 0x2d),
                match_fg: Color::Rgb(0x1a, 0x1b, 0x26),
                git: GitColors {
                    conflicted: Color::Rgb(0xe4, 0x67, 0x6b),
                    deleted: Color::Rgb(0xc7, 0x4e, 0x39),
                    renamed: Color::Rgb(0x73, 0xc9, 0x91),
                    modified: Color::Rgb(0xe2, 0xc0, 0x8d),
                    added: Color::Rgb(0x81, 0xb8, 0x8b),
                    untracked: Color::Rgb(0x73, 0xc9, 0x91),
                },
            },
            guides: GuideStyle::Indent,
            selection: SelectionStyle::SoftBarAccent,
            badges: BadgeStyle::Letter,
            icons: IconSet::NerdFont,
            bold_dirs: true,
            folder_icon: true,
        }
    }

    /// A stripped-back theme: basic ANSI colors, no icons, a plain full-row
    /// selection, no guides.
    fn plain() -> Theme {
        Theme {
            id: ThemeId::Plain,
            palette: Palette {
                name_fg: None,
                dir_fg: Some(Color::Blue),
                selection_bg: Color::DarkGray,
                selection_accent: Color::Gray,
                guide: Color::DarkGray,
                chevron: Color::Gray,
                separator: Color::DarkGray,
                ignored: Color::DarkGray,
                match_bg: Color::Yellow,
                match_fg: Color::Black,
                git: GitColors {
                    conflicted: Color::Red,
                    deleted: Color::Red,
                    renamed: Color::Green,
                    modified: Color::Yellow,
                    added: Color::Green,
                    untracked: Color::Green,
                },
            },
            // GuideStyle::Connectors is defined but not yet realized; the plain
            // theme stays genuinely plain with no guides. TODO(025).
            guides: GuideStyle::None,
            selection: SelectionStyle::FullRow,
            badges: BadgeStyle::Letter,
            icons: IconSet::None,
            bold_dirs: true,
            folder_icon: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_id_resolves_every_theme_and_keeps_the_id() {
        for id in [
            ThemeId::Birch,
            ThemeId::Vscode,
            ThemeId::Jetbrains,
            ThemeId::Xcode,
            ThemeId::Retro,
            ThemeId::Plain,
        ] {
            assert_eq!(Theme::for_id(id).id, id);
        }
    }

    #[test]
    fn birch_and_plain_differ_on_the_key_axes() {
        let birch = Theme::for_id(ThemeId::Birch);
        assert_eq!(birch.guides, GuideStyle::Indent);
        assert_eq!(birch.selection, SelectionStyle::SoftBarAccent);
        assert_eq!(birch.icons, IconSet::NerdFont);

        let plain = Theme::for_id(ThemeId::Plain);
        assert_eq!(plain.guides, GuideStyle::None);
        assert_eq!(plain.selection, SelectionStyle::FullRow);
        assert_eq!(plain.icons, IconSet::None);
    }
}
