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
    /// Classic `├─`/`└─`/`│` connectors, driven by the per-row
    /// following-sibling/last-child data on `Row` (`guides`, `last_sibling`).
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

/// How the glyph columns between the indent and the label are laid out. All
/// three keep the chevron as the first glyph column (at `depth * INDENT_WIDTH`),
/// so `hit_test` geometry is never affected.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FolderStyle {
    /// The editor default: two glyph columns. Dirs show `chevron folder-icon`,
    /// files show `·· file-icon` (the blank keeps icons aligned under dirs).
    Icon,
    /// One glyph column: the chevron sits where the icon would be. Dirs show
    /// only the chevron (no folder glyph), files show only their icon. Names
    /// align one column tighter than `Icon`.
    Compact,
    /// One glyph column, no icons at all: dirs show the chevron, files a blank.
    Plain,
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
    /// How the chevron/icon glyph columns between the indent and the label are
    /// laid out (`Icon`/`Compact`/`Plain`).
    pub folder_style: FolderStyle,
    /// The collapsed-directory chevron glyph (e.g. `▸`, or a filled `▶`).
    pub chevron_collapsed: &'static str,
    /// The expanded-directory chevron glyph (e.g. `▾`, or a filled `▼`).
    pub chevron_expanded: &'static str,
}

impl Theme {
    /// Resolves a `ThemeId` to its `Theme`. Every catalog theme is a distinct
    /// value tuned for a black terminal background (ADR 0021). Icon glyphs are
    /// shared (the Nerd Font map); themes differentiate via palette, guides,
    /// folder-icon, and selection. TODO(025): a per-theme icon *palette* /
    /// blocky retro glyph set would sharpen the emulation further.
    pub fn for_id(id: ThemeId) -> Theme {
        match id {
            ThemeId::Birch => Theme::birch(),
            ThemeId::Vscode => Theme::vscode(),
            ThemeId::Jetbrains => Theme::jetbrains(),
            ThemeId::Xcode => Theme::xcode(),
            ThemeId::Mocha => Theme::mocha(),
            ThemeId::Tokyonight => Theme::tokyonight(),
            ThemeId::Retro => Theme::retro(),
            ThemeId::Plain => Theme::plain(),
        }
    }

    /// The flagship theme (task 054): curated muted palette, dim indent guides,
    /// a soft selection with a birch-green left accent bar, bold directories.
    fn birch() -> Theme {
        Theme {
            id: ThemeId::Birch,
            palette: Palette {
                name_fg: None,
                // Blue + bold directories — the near-universal file-browser
                // signature (yazi, nvim-tree, LS_COLORS tradition).
                dir_fg: Some(Color::Rgb(0x7a, 0xa2, 0xf7)),
                // Softer / lower-contrast than the previous #2f3b54 full-row fill.
                selection_bg: Color::Rgb(0x28, 0x30, 0x40),
                // Birch green, echoing the logo.
                selection_accent: Color::Rgb(0x6f, 0x91, 0x52),
                guide: Color::Rgb(0x4c, 0x56, 0x6a),
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
            folder_style: FolderStyle::Icon,
            chevron_collapsed: "\u{f460}", // octicon chevron-right (thin)
            chevron_expanded: "\u{f47c}",  // octicon chevron-down
        }
    }

    /// VS Code-like: dim indent guides, a compact single glyph column (the
    /// chevron stands in for the folder glyph), a full-row VS Code active-
    /// selection blue, and the familiar VS Code blues/greys.
    fn vscode() -> Theme {
        Theme {
            id: ThemeId::Vscode,
            palette: Palette {
                name_fg: Some(Color::Rgb(0xd4, 0xd4, 0xd4)),
                dir_fg: Some(Color::Rgb(0xcf, 0xcf, 0xcf)),
                // VS Code list active-selection blue (full-row).
                selection_bg: Color::Rgb(0x09, 0x47, 0x71),
                selection_accent: Color::Rgb(0x00, 0x7a, 0xcc),
                guide: Color::Rgb(0x40, 0x40, 0x40),
                chevron: Color::Rgb(0x80, 0x80, 0x80),
                separator: Color::Rgb(0x6a, 0x73, 0x7d),
                ignored: Color::Rgb(0x6a, 0x6a, 0x6a),
                match_bg: Color::Rgb(0x0e, 0x63, 0x9c),
                match_fg: Color::Rgb(0xff, 0xff, 0xff),
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
            selection: SelectionStyle::FullRow,
            badges: BadgeStyle::Letter,
            icons: IconSet::NerdFont,
            bold_dirs: true,
            folder_style: FolderStyle::Compact,
            chevron_collapsed: "\u{eab6}", // codicon chevron-right (VS Code)
            chevron_expanded: "\u{eab4}",  // codicon chevron-down
        }
    }

    /// JetBrains/Darcula-like: dim indent guides, folder glyphs shown, a full-
    /// row IDEA-blue selection over a near-neutral warm palette.
    fn jetbrains() -> Theme {
        Theme {
            id: ThemeId::Jetbrains,
            palette: Palette {
                name_fg: Some(Color::Rgb(0xb8, 0xb4, 0xac)),
                dir_fg: Some(Color::Rgb(0xd0, 0xcb, 0xc0)),
                // IDEA full-row selection blue.
                selection_bg: Color::Rgb(0x21, 0x42, 0x83),
                selection_accent: Color::Rgb(0xd9, 0x97, 0x5a),
                guide: Color::Rgb(0x4b, 0x4b, 0x48),
                chevron: Color::Rgb(0x9a, 0x93, 0x86),
                separator: Color::Rgb(0x80, 0x7a, 0x70),
                ignored: Color::Rgb(0x6f, 0x6b, 0x63),
                match_bg: Color::Rgb(0x32, 0x59, 0x3d),
                match_fg: Color::Rgb(0xe8, 0xe4, 0xdc),
                git: GitColors {
                    conflicted: Color::Rgb(0xe0, 0x55, 0x55),
                    deleted: Color::Rgb(0xc7, 0x54, 0x50),
                    renamed: Color::Rgb(0xb3, 0xae, 0x60),
                    modified: Color::Rgb(0x68, 0x97, 0xbb),
                    added: Color::Rgb(0x6a, 0x87, 0x59),
                    untracked: Color::Rgb(0x6a, 0x87, 0x59),
                },
            },
            guides: GuideStyle::Indent,
            selection: SelectionStyle::FullRow,
            badges: BadgeStyle::Letter,
            icons: IconSet::NerdFont,
            bold_dirs: true,
            folder_style: FolderStyle::Icon,
            chevron_collapsed: "\u{eab6}", // thin chevron (IDEA New UI)
            chevron_expanded: "\u{eab4}",
        }
    }

    /// Xcode-like: no guides, folder glyphs shown, thin disclosure chevrons
    /// (the modern macOS sidebar look — filled triangles died with Big Sur),
    /// a full-row macOS-blue selection over a lighter, cooler palette.
    fn xcode() -> Theme {
        Theme {
            id: ThemeId::Xcode,
            palette: Palette {
                name_fg: Some(Color::Rgb(0xe5, 0xea, 0xf0)),
                dir_fg: Some(Color::Rgb(0xff, 0xff, 0xff)),
                // macOS system-blue full-row selection.
                selection_bg: Color::Rgb(0x1e, 0x57, 0xc4),
                selection_accent: Color::Rgb(0x3f, 0x7a, 0xf6),
                guide: Color::Rgb(0x3a, 0x41, 0x4b),
                chevron: Color::Rgb(0x98, 0xa0, 0xab),
                separator: Color::Rgb(0x8a, 0x92, 0x9c),
                ignored: Color::Rgb(0x6c, 0x75, 0x80),
                match_bg: Color::Rgb(0x3f, 0x6f, 0xb5),
                match_fg: Color::Rgb(0xff, 0xff, 0xff),
                git: GitColors {
                    conflicted: Color::Rgb(0xd0, 0x57, 0x4e),
                    deleted: Color::Rgb(0xd0, 0x57, 0x4e),
                    renamed: Color::Rgb(0x67, 0xb2, 0x6f),
                    modified: Color::Rgb(0x4a, 0x90, 0xd9),
                    added: Color::Rgb(0x67, 0xb2, 0x6f),
                    untracked: Color::Rgb(0x67, 0xb2, 0x6f),
                },
            },
            guides: GuideStyle::None,
            selection: SelectionStyle::FullRow,
            badges: BadgeStyle::Letter,
            icons: IconSet::NerdFont,
            bold_dirs: true,
            folder_style: FolderStyle::Icon,
            chevron_collapsed: "\u{eab6}", // thin chevron (modern macOS sidebar)
            chevron_expanded: "\u{eab4}",
        }
    }

    /// Catppuccin Mocha: the pastel-on-violet-grey community favourite. Blue
    /// bold dirs, surface0 full-row selection, lavender accent, overlay-level
    /// guides — values from the official mocha palette.
    fn mocha() -> Theme {
        Theme {
            id: ThemeId::Mocha,
            palette: Palette {
                name_fg: None,
                dir_fg: Some(Color::Rgb(0x89, 0xb4, 0xfa)), // blue
                selection_bg: Color::Rgb(0x31, 0x32, 0x44), // surface0
                selection_accent: Color::Rgb(0xb4, 0xbe, 0xfe), // lavender
                guide: Color::Rgb(0x45, 0x47, 0x5a),        // surface1
                chevron: Color::Rgb(0x7f, 0x84, 0x9c),      // overlay1
                separator: Color::Rgb(0x7f, 0x84, 0x9c),
                ignored: Color::Rgb(0x6c, 0x70, 0x86), // overlay0
                match_bg: Color::Rgb(0xf9, 0xe2, 0xaf), // yellow
                match_fg: Color::Rgb(0x11, 0x11, 0x1b), // crust
                git: GitColors {
                    conflicted: Color::Rgb(0xeb, 0xa0, 0xac), // maroon
                    deleted: Color::Rgb(0xf3, 0x8b, 0xa8),    // red
                    renamed: Color::Rgb(0x89, 0xb4, 0xfa),    // blue
                    modified: Color::Rgb(0xf9, 0xe2, 0xaf),   // yellow
                    added: Color::Rgb(0xa6, 0xe3, 0xa1),      // green
                    untracked: Color::Rgb(0x94, 0xe2, 0xd5),  // teal
                },
            },
            guides: GuideStyle::Indent,
            selection: SelectionStyle::FullRow,
            badges: BadgeStyle::Letter,
            icons: IconSet::NerdFont,
            bold_dirs: true,
            folder_style: FolderStyle::Icon,
            chevron_collapsed: "\u{f460}", // octicon chevron-right (thin)
            chevron_expanded: "\u{f47c}",  // octicon chevron-down
        }
    }

    /// Tokyo Night: the cool blue-violet scheme. Blue bold dirs, bg-highlight
    /// full-row selection, comment-colour muting — values from the official
    /// tokyonight palette.
    fn tokyonight() -> Theme {
        Theme {
            id: ThemeId::Tokyonight,
            palette: Palette {
                name_fg: None,
                dir_fg: Some(Color::Rgb(0x7a, 0xa2, 0xf7)), // blue
                selection_bg: Color::Rgb(0x29, 0x2e, 0x42), // bg_highlight
                selection_accent: Color::Rgb(0x7a, 0xa2, 0xf7),
                guide: Color::Rgb(0x3b, 0x42, 0x61),
                chevron: Color::Rgb(0x56, 0x5f, 0x89), // comment
                separator: Color::Rgb(0x56, 0x5f, 0x89),
                ignored: Color::Rgb(0x56, 0x5f, 0x89),
                match_bg: Color::Rgb(0xe0, 0xaf, 0x68), // orange-yellow
                match_fg: Color::Rgb(0x1a, 0x1b, 0x26), // bg
                git: GitColors {
                    conflicted: Color::Rgb(0xf7, 0x76, 0x8e), // red
                    deleted: Color::Rgb(0xdb, 0x4b, 0x4b),
                    renamed: Color::Rgb(0x7d, 0xcf, 0xff), // cyan
                    modified: Color::Rgb(0xe0, 0xaf, 0x68),
                    added: Color::Rgb(0x9e, 0xce, 0x6a), // green
                    untracked: Color::Rgb(0x73, 0xda, 0xca), // teal
                },
            },
            guides: GuideStyle::Indent,
            selection: SelectionStyle::FullRow,
            badges: BadgeStyle::Letter,
            icons: IconSet::NerdFont,
            bold_dirs: true,
            folder_style: FolderStyle::Icon,
            chevron_collapsed: "\u{f460}", // octicon chevron-right (thin)
            chevron_expanded: "\u{f47c}",  // octicon chevron-down
        }
    }

    /// A retro CRT look: classic `├─`/`└─` connectors, folder glyphs, filled
    /// disclosure triangles, and a high-contrast saturated amber/green palette
    /// with a full-row selection.
    fn retro() -> Theme {
        Theme {
            id: ThemeId::Retro,
            palette: Palette {
                name_fg: Some(Color::Rgb(0xff, 0xb4, 0x54)),
                dir_fg: Some(Color::Rgb(0x4e, 0xe4, 0x4e)),
                selection_bg: Color::Rgb(0x5f, 0x00, 0x5f),
                selection_accent: Color::Rgb(0xff, 0xff, 0x55),
                guide: Color::Rgb(0x3f, 0xbf, 0x3f),
                chevron: Color::Rgb(0xff, 0xb4, 0x54),
                separator: Color::Rgb(0xaf, 0x87, 0x5f),
                ignored: Color::Rgb(0x6f, 0x6f, 0x2f),
                match_bg: Color::Rgb(0xff, 0xff, 0x00),
                match_fg: Color::Rgb(0x00, 0x00, 0x00),
                git: GitColors {
                    conflicted: Color::Rgb(0xff, 0x55, 0x55),
                    deleted: Color::Rgb(0xff, 0x55, 0x55),
                    renamed: Color::Rgb(0x55, 0xff, 0x55),
                    modified: Color::Rgb(0xff, 0xff, 0x55),
                    added: Color::Rgb(0x55, 0xff, 0x55),
                    untracked: Color::Rgb(0x55, 0xff, 0xff),
                },
            },
            guides: GuideStyle::Connectors,
            selection: SelectionStyle::FullRow,
            badges: BadgeStyle::Letter,
            icons: IconSet::NerdFont,
            bold_dirs: true,
            folder_style: FolderStyle::Icon,
            chevron_collapsed: "\u{25b6}", // ▶ (filled)
            chevron_expanded: "\u{25bc}",  // ▼ (filled)
        }
    }

    /// A stripped-back theme: basic ANSI colors, no icons, a plain full-row
    /// selection, classic connector guides.
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
            guides: GuideStyle::Connectors,
            selection: SelectionStyle::FullRow,
            badges: BadgeStyle::Letter,
            icons: IconSet::None,
            bold_dirs: true,
            folder_style: FolderStyle::Plain,
            chevron_collapsed: "\u{25b8}", // ▸
            chevron_expanded: "\u{25be}",  // ▾
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
            ThemeId::Mocha,
            ThemeId::Tokyonight,
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
        assert_eq!(plain.guides, GuideStyle::Connectors);
        assert_eq!(plain.selection, SelectionStyle::FullRow);
        assert_eq!(plain.icons, IconSet::None);
    }

    #[test]
    fn catalog_themes_pick_distinct_points() {
        // vscode: compact single column (chevron stands in for the folder
        // glyph); dim indent guides; full-row selection.
        let vscode = Theme::for_id(ThemeId::Vscode);
        assert_eq!(vscode.folder_style, FolderStyle::Compact);
        assert_eq!(vscode.guides, GuideStyle::Indent);
        assert_eq!(vscode.selection, SelectionStyle::FullRow);

        // jetbrains: indent guides, folder glyphs, full-row selection.
        let jb = Theme::for_id(ThemeId::Jetbrains);
        assert_eq!(jb.guides, GuideStyle::Indent);
        assert_eq!(jb.folder_style, FolderStyle::Icon);
        assert_eq!(jb.selection, SelectionStyle::FullRow);

        // xcode: no guides, full-row selection, thin chevrons (modern macOS —
        // the filled triangle is the pre-Big Sur look).
        let xcode = Theme::for_id(ThemeId::Xcode);
        assert_eq!(xcode.guides, GuideStyle::None);
        assert_eq!(xcode.folder_style, FolderStyle::Icon);
        assert_eq!(xcode.selection, SelectionStyle::FullRow);
        assert_eq!(xcode.chevron_collapsed, "\u{eab6}"); // codicon thin chevron
        assert_eq!(xcode.chevron_expanded, "\u{eab4}");

        // vscode uses its own codicon twistie — the exact VS Code glyph.
        assert_eq!(vscode.chevron_collapsed, "\u{eab6}");

        // mocha / tokyonight: palette themes — blue bold dirs, surface-level
        // full-row selection, indent guides, thin octicon chevrons.
        for id in [ThemeId::Mocha, ThemeId::Tokyonight] {
            let t = Theme::for_id(id);
            assert_eq!(t.guides, GuideStyle::Indent);
            assert_eq!(t.selection, SelectionStyle::FullRow);
            assert_eq!(t.folder_style, FolderStyle::Icon);
            assert_eq!(t.chevron_collapsed, "\u{f460}"); // octicon thin chevron
            assert!(t.palette.dir_fg.is_some());
        }

        // retro: classic connectors, full-row selection, filled triangles (the
        // one deliberately legacy chevron in the catalog).
        let retro = Theme::for_id(ThemeId::Retro);
        assert_eq!(retro.guides, GuideStyle::Connectors);
        assert_eq!(retro.selection, SelectionStyle::FullRow);
        assert_eq!(retro.icons, IconSet::NerdFont);
        assert_eq!(retro.chevron_collapsed, "\u{25b6}"); // ▶ filled

        // plain: no icons at all (Plain folder style), width-safe ▸ chevron.
        let plain = Theme::for_id(ThemeId::Plain);
        assert_eq!(plain.folder_style, FolderStyle::Plain);
        assert_eq!(plain.chevron_collapsed, "\u{25b8}");

        // The catalog is not a single theme wearing different ids: at least the
        // guide/folder-style/chevron axes vary across the set.
        assert_ne!(vscode.folder_style, jb.folder_style);
        assert_ne!(jb.folder_style, plain.folder_style);
        assert_ne!(xcode.guides, retro.guides);
        assert_ne!(retro.chevron_collapsed, birch_default_chevron());
    }

    fn birch_default_chevron() -> &'static str {
        Theme::for_id(ThemeId::Birch).chevron_collapsed
    }
}
