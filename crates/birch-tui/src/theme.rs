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

/// Which glyph family supplies per-file icons. The devicon map (`icons.rs`)
/// is the base; the other families override a handful of common categories
/// (folder, generic file, markdown, config, …) with their own Nerd Font
/// glyphs and fall back to the devicon map for everything else.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IconSet {
    /// No icons at all (e.g. the `plain` theme).
    None,
    /// The built-in devicon map — the Nerd Font community default.
    Devicons,
    /// VS Code's codicon glyphs (outline, single-weight).
    Codicons,
    /// Material Design glyphs (outline variants where available).
    Material,
    /// GitHub's octicon glyphs.
    Octicons,
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
    /// Paint the whole pane this colour (`None` = inherit the terminal
    /// background, the default for every theme except full-canvas looks like
    /// the Commander retro's DOS blue).
    pub app_bg: Option<Color>,
    /// File-name foreground (`None` = terminal default).
    pub name_fg: Option<Color>,
    /// Directory-name foreground (`None` = terminal default).
    pub dir_fg: Option<Color>,
    /// Selection background (soft fill or full-row fill).
    pub selection_bg: Color,
    /// Selected-row foreground override — every span on the selected row takes
    /// this colour (the Commander black-on-cyan fg swap). `None` keeps each
    /// span's own colour.
    pub selection_fg: Option<Color>,
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
    /// Tint every icon this colour (the "hues are theme-owned" rule) instead
    /// of the devicon colours. `None` keeps per-filetype devicon hues.
    pub icon_tint: Option<Color>,
    /// Depth-fade floor for indent guides: `Some(floor)` fades the guide
    /// colour toward `floor` as depth increases (clamped, never dimmer).
    /// `None` keeps a uniform guide colour.
    pub guide_fade: Option<Color>,
    /// The collapsed-directory chevron glyph (e.g. `▸`, or a filled `▶`).
    pub chevron_collapsed: &'static str,
    /// The expanded-directory chevron glyph (e.g. `▾`, or a filled `▼`).
    pub chevron_expanded: &'static str,
}

impl Theme {
    /// Resolves a `ThemeId` to its `Theme`. Every catalog theme is a distinct
    /// value tuned for a black terminal background (ADR 0021). Themes
    /// differentiate via palette, guides, folder-icon, selection, and icon
    /// family (`IconSet`): the devicon map is the shared base, and the
    /// codicon/material/octicon families override the common categories.
    pub fn for_id(id: ThemeId) -> Theme {
        match id {
            ThemeId::Birch => Theme::birch(),
            ThemeId::Vscode => Theme::vscode(),
            ThemeId::Jetbrains => Theme::jetbrains(),
            ThemeId::Xcode => Theme::xcode(),
            ThemeId::Mocha => Theme::mocha(),
            ThemeId::Tokyonight => Theme::tokyonight(),
            ThemeId::Gruvbox => Theme::gruvbox(),
            ThemeId::Nord => Theme::nord(),
            ThemeId::Rosepine => Theme::rosepine(),
            ThemeId::Retro => Theme::retro(),
            ThemeId::Plain => Theme::plain(),
        }
    }

    /// The flagship: silver bark with a single gold stroke. A fully
    /// desaturated silver-green tree — bold bark-silver dirs, sage-tinted
    /// icons, moss chrome — where the only warm saturated mark is the gold
    /// selection bar. Indent guides fade with depth (the canopy recedes);
    /// their dashes on the silver field echo the lenticels of birch bark.
    fn birch() -> Theme {
        Theme {
            id: ThemeId::Birch,
            palette: Palette {
                app_bg: None,
                name_fg: Some(Color::Rgb(0xb0, 0xb4, 0xab)),
                // Bark silver, two value steps above files, bold.
                dir_fg: Some(Color::Rgb(0xce, 0xd3, 0xc6)),
                // Warm-neutral lift: findable at a glance, quiet all day.
                selection_bg: Color::Rgb(0x26, 0x23, 0x1c),
                selection_fg: None,
                // The one gold stroke — the single most saturated element.
                selection_accent: Color::Rgb(0xd9, 0xa6, 0x48),
                guide: Color::Rgb(0x45, 0x48, 0x3f),
                chevron: Color::Rgb(0x8a, 0x9a, 0x5b),
                separator: Color::Rgb(0x8a, 0x9a, 0x5b),
                ignored: Color::Rgb(0x5f, 0x66, 0x5c),
                match_bg: Color::Rgb(0xd9, 0xa6, 0x48),
                match_fg: Color::Rgb(0x1e, 0x1f, 0x1a),
                git: GitColors {
                    conflicted: Color::Rgb(0xc2, 0x50, 0x49),
                    deleted: Color::Rgb(0xa5, 0x50, 0x2e),
                    renamed: Color::Rgb(0x9c, 0xcf, 0xd8),
                    modified: Color::Rgb(0xd4, 0xa0, 0x17),
                    added: Color::Rgb(0x8a, 0x9a, 0x5b),
                    untracked: Color::Rgb(0x7c, 0x9a, 0x52),
                },
            },
            guides: GuideStyle::Indent,
            selection: SelectionStyle::SoftBarAccent,
            badges: BadgeStyle::Letter,
            icons: IconSet::Devicons,
            bold_dirs: true,
            folder_style: FolderStyle::Icon,
            // One sage for all icons — file type is the filename's job here.
            icon_tint: Some(Color::Rgb(0x7d, 0x8b, 0x6f)),
            // Depth fade: guides recede into the canopy, clamped at the floor.
            guide_fade: Some(Color::Rgb(0x33, 0x36, 0x2f)),
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
                app_bg: None,
                // Measured from VS Code Dark Modern: files and dirs share the
                // same text colour, and dir names are NOT bold.
                name_fg: Some(Color::Rgb(0xbf, 0xbf, 0xbf)),
                dir_fg: Some(Color::Rgb(0xbf, 0xbf, 0xbf)),
                // Dark Modern list.activeSelectionBackground (full-row).
                selection_bg: Color::Rgb(0x04, 0x39, 0x5e),
                selection_fg: None,
                selection_accent: Color::Rgb(0x00, 0x7a, 0xcc),
                guide: Color::Rgb(0x58, 0x58, 0x58),   // measured
                chevron: Color::Rgb(0x8c, 0x8c, 0x8c), // measured
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
            icons: IconSet::Codicons,
            bold_dirs: false, // real editors do not bold dir names
            folder_style: FolderStyle::Compact,
            icon_tint: None,
            guide_fade: None,
            chevron_collapsed: "\u{eab6}", // codicon chevron-right (VS Code)
            chevron_expanded: "\u{eab4}",  // codicon chevron-down
        }
    }

    /// JetBrains New UI-like: no indent guides (the New UI default), folder
    /// glyphs shown, a full-row New UI-blue selection over cool greys.
    fn jetbrains() -> Theme {
        Theme {
            id: ThemeId::Jetbrains,
            palette: Palette {
                app_bg: None,
                // Measured from IDEA New UI dark: cool greys, dirs not bold.
                name_fg: Some(Color::Rgb(0xbc, 0xbe, 0xc4)),
                dir_fg: Some(Color::Rgb(0xd1, 0xd3, 0xd9)),
                // New UI tree selection blue (full-row).
                selection_bg: Color::Rgb(0x2e, 0x43, 0x6e),
                selection_fg: None,
                selection_accent: Color::Rgb(0xd9, 0x97, 0x5a),
                guide: Color::Rgb(0x4b, 0x4b, 0x48),
                chevron: Color::Rgb(0xb5, 0xb8, 0xbe), // measured
                separator: Color::Rgb(0x80, 0x7a, 0x70),
                ignored: Color::Rgb(0x6f, 0x73, 0x7a),
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
            guides: GuideStyle::None,
            selection: SelectionStyle::FullRow,
            badges: BadgeStyle::Letter,
            icons: IconSet::Devicons,
            bold_dirs: false, // real editors do not bold dir names
            folder_style: FolderStyle::Icon,
            icon_tint: None,
            guide_fade: None,
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
                app_bg: None,
                name_fg: Some(Color::Rgb(0xe5, 0xea, 0xf0)),
                dir_fg: Some(Color::Rgb(0xff, 0xff, 0xff)),
                // macOS system-blue full-row selection.
                selection_bg: Color::Rgb(0x1e, 0x57, 0xc4),
                selection_fg: None,
                selection_accent: Color::Rgb(0x3f, 0x7a, 0xf6),
                guide: Color::Rgb(0x3a, 0x41, 0x4b),
                chevron: Color::Rgb(0xa7, 0xa6, 0xa7), // measured
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
            icons: IconSet::Material,
            bold_dirs: false, // real editors do not bold dir names
            folder_style: FolderStyle::Icon,
            icon_tint: None,
            guide_fade: None,
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
                app_bg: None,
                name_fg: None,
                dir_fg: Some(Color::Rgb(0x89, 0xb4, 0xfa)), // blue
                selection_bg: Color::Rgb(0x31, 0x32, 0x44), // surface0
                selection_fg: None,
                selection_accent: Color::Rgb(0xb4, 0xbe, 0xfe), // lavender
                guide: Color::Rgb(0x45, 0x47, 0x5a),            // surface1
                chevron: Color::Rgb(0x7f, 0x84, 0x9c),          // overlay1
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
            icons: IconSet::Devicons,
            bold_dirs: true,
            folder_style: FolderStyle::Icon,
            icon_tint: Some(Color::Rgb(0xfa, 0xb3, 0x87)), // peach
            guide_fade: None,
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
                app_bg: None,
                name_fg: None,
                dir_fg: Some(Color::Rgb(0x7a, 0xa2, 0xf7)), // blue
                selection_bg: Color::Rgb(0x29, 0x2e, 0x42), // bg_highlight
                selection_fg: None,
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
            icons: IconSet::Devicons,
            bold_dirs: true,
            folder_style: FolderStyle::Icon,
            icon_tint: None,
            guide_fade: None,
            chevron_collapsed: "\u{f460}", // octicon chevron-right (thin)
            chevron_expanded: "\u{f47c}",  // octicon chevron-down
        }
    }

    /// Gruvbox (dark): the warm retro scheme. Yellow bold dirs — the gruvbox
    /// signature, not blue — bg1 full-row selection, orange accent — values
    /// from the official gruvbox palette.
    fn gruvbox() -> Theme {
        Theme {
            id: ThemeId::Gruvbox,
            palette: Palette {
                app_bg: None,
                name_fg: Some(Color::Rgb(0xeb, 0xdb, 0xb2)), // fg1
                dir_fg: Some(Color::Rgb(0xfa, 0xbd, 0x2f)),  // bright yellow
                selection_bg: Color::Rgb(0x50, 0x49, 0x45),  // bg1
                selection_fg: None,
                selection_accent: Color::Rgb(0xfe, 0x80, 0x19), // bright orange
                guide: Color::Rgb(0x50, 0x49, 0x45),            // bg2
                chevron: Color::Rgb(0x92, 0x83, 0x74),          // gray
                separator: Color::Rgb(0x92, 0x83, 0x74),
                ignored: Color::Rgb(0x92, 0x83, 0x74),
                match_bg: Color::Rgb(0xfa, 0xbd, 0x2f), // bright yellow
                match_fg: Color::Rgb(0x28, 0x28, 0x28), // bg0
                git: GitColors {
                    conflicted: Color::Rgb(0xfb, 0x49, 0x34), // bright red
                    deleted: Color::Rgb(0xfb, 0x49, 0x34),
                    renamed: Color::Rgb(0x8e, 0xc0, 0x7c), // bright aqua
                    modified: Color::Rgb(0xfa, 0xbd, 0x2f), // bright yellow
                    added: Color::Rgb(0xb8, 0xbb, 0x26),   // bright green
                    untracked: Color::Rgb(0x8e, 0xc0, 0x7c), // bright aqua
                },
            },
            guides: GuideStyle::Indent,
            selection: SelectionStyle::FullRow,
            badges: BadgeStyle::Letter,
            icons: IconSet::Devicons,
            bold_dirs: true,
            folder_style: FolderStyle::Icon,
            icon_tint: None,
            guide_fade: None,
            chevron_collapsed: "\u{f460}", // octicon chevron-right (thin)
            chevron_expanded: "\u{f47c}",  // octicon chevron-down
        }
    }

    /// Nord: the arctic, calm blue-grey scheme. Frost-cyan bold dirs, nord2
    /// full-row selection — values from the official nord palette.
    fn nord() -> Theme {
        Theme {
            id: ThemeId::Nord,
            palette: Palette {
                app_bg: None,
                name_fg: Some(Color::Rgb(0xd8, 0xde, 0xe9)), // nord4 (snow storm)
                dir_fg: Some(Color::Rgb(0x88, 0xc0, 0xd0)),  // nord8 (frost cyan)
                selection_bg: Color::Rgb(0x43, 0x4c, 0x5e),  // nord2
                selection_fg: None,
                selection_accent: Color::Rgb(0x88, 0xc0, 0xd0), // nord8
                guide: Color::Rgb(0x4c, 0x56, 0x6a),            // nord3
                chevron: Color::Rgb(0x61, 0x6e, 0x88),          // nord3 bright (comments)
                separator: Color::Rgb(0x61, 0x6e, 0x88),
                ignored: Color::Rgb(0x61, 0x6e, 0x88),
                match_bg: Color::Rgb(0xeb, 0xcb, 0x8b), // nord13 (yellow)
                match_fg: Color::Rgb(0x2e, 0x34, 0x40), // nord0
                git: GitColors {
                    conflicted: Color::Rgb(0xbf, 0x61, 0x6a), // nord11 (red)
                    deleted: Color::Rgb(0xbf, 0x61, 0x6a),
                    renamed: Color::Rgb(0x8f, 0xbc, 0xbb), // nord7 (frost teal)
                    modified: Color::Rgb(0xeb, 0xcb, 0x8b), // nord13 (yellow)
                    added: Color::Rgb(0xa3, 0xbe, 0x8c),   // nord14 (green)
                    untracked: Color::Rgb(0xa3, 0xbe, 0x8c),
                },
            },
            guides: GuideStyle::Indent,
            selection: SelectionStyle::FullRow,
            badges: BadgeStyle::Letter,
            icons: IconSet::Devicons,
            bold_dirs: true,
            folder_style: FolderStyle::Icon,
            icon_tint: None,
            guide_fade: None,
            chevron_collapsed: "\u{f460}", // octicon chevron-right (thin)
            chevron_expanded: "\u{f47c}",  // octicon chevron-down
        }
    }

    /// Rosé Pine: the elegant dusk scheme. Iris-violet bold dirs, rose accent,
    /// highlight-med full-row selection — values from the official rosé pine
    /// palette.
    fn rosepine() -> Theme {
        Theme {
            id: ThemeId::Rosepine,
            palette: Palette {
                app_bg: None,
                name_fg: Some(Color::Rgb(0xe0, 0xde, 0xf4)), // text
                dir_fg: Some(Color::Rgb(0xc4, 0xa7, 0xe7)),  // iris
                selection_bg: Color::Rgb(0x40, 0x3d, 0x52),  // highlight med
                selection_fg: None,
                selection_accent: Color::Rgb(0xeb, 0xbc, 0xba), // rose
                guide: Color::Rgb(0x40, 0x3d, 0x52),            // highlight med
                chevron: Color::Rgb(0x90, 0x8c, 0xaa),          // subtle
                separator: Color::Rgb(0x90, 0x8c, 0xaa),
                ignored: Color::Rgb(0x6e, 0x6a, 0x86),  // muted
                match_bg: Color::Rgb(0xf6, 0xc1, 0x77), // gold
                match_fg: Color::Rgb(0x19, 0x17, 0x24), // base
                git: GitColors {
                    conflicted: Color::Rgb(0xeb, 0x6f, 0x92), // love
                    deleted: Color::Rgb(0xeb, 0x6f, 0x92),
                    renamed: Color::Rgb(0x9c, 0xcf, 0xd8), // foam
                    modified: Color::Rgb(0xf6, 0xc1, 0x77), // gold
                    added: Color::Rgb(0x31, 0x74, 0x8f),   // pine
                    untracked: Color::Rgb(0x9c, 0xcf, 0xd8), // foam
                },
            },
            guides: GuideStyle::Indent,
            selection: SelectionStyle::FullRow,
            badges: BadgeStyle::Letter,
            icons: IconSet::Devicons,
            bold_dirs: true,
            folder_style: FolderStyle::Icon,
            icon_tint: Some(Color::Rgb(0xeb, 0xbc, 0xba)), // rose
            guide_fade: None,
            chevron_collapsed: "\u{f460}", // octicon chevron-right (thin)
            chevron_expanded: "\u{f47c}",  // octicon chevron-down
        }
    }

    /// The Commander: canonical Norton/Midnight Commander DOS look. CGA blue
    /// canvas (#0000AA), lightgray files, white ALL-BOLD dirs, the black-on-
    /// cyan cursor bar, `+`/`-` tree marks, no icons — ASCII purity.
    fn retro() -> Theme {
        Theme {
            id: ThemeId::Retro,
            palette: Palette {
                // The DOS blue field — the one theme that paints its canvas.
                app_bg: Some(Color::Rgb(0x00, 0x00, 0xaa)),
                name_fg: Some(Color::Rgb(0xaa, 0xaa, 0xaa)),
                dir_fg: Some(Color::Rgb(0xff, 0xff, 0xff)),
                // Cursor bar: black on cyan (the NC selection).
                selection_bg: Color::Rgb(0x00, 0xaa, 0xaa),
                selection_fg: Some(Color::Rgb(0x00, 0x00, 0x00)),
                selection_accent: Color::Rgb(0xff, 0xff, 0x55),
                guide: Color::Rgb(0x55, 0x55, 0xff),
                chevron: Color::Rgb(0xff, 0xff, 0x55),
                separator: Color::Rgb(0x55, 0x55, 0xff),
                ignored: Color::Rgb(0x55, 0x55, 0xff),
                match_bg: Color::Rgb(0xff, 0xff, 0x55),
                match_fg: Color::Rgb(0x00, 0x00, 0xaa),
                git: GitColors {
                    conflicted: Color::Rgb(0xff, 0x55, 0x55),
                    deleted: Color::Rgb(0xff, 0x55, 0x55),
                    renamed: Color::Rgb(0x55, 0xff, 0xff),
                    modified: Color::Rgb(0xff, 0xff, 0x55),
                    added: Color::Rgb(0x55, 0xff, 0x55),
                    untracked: Color::Rgb(0x55, 0xff, 0x55),
                },
            },
            guides: GuideStyle::Connectors,
            selection: SelectionStyle::FullRow,
            badges: BadgeStyle::Letter,
            icons: IconSet::None,
            bold_dirs: true,
            folder_style: FolderStyle::Plain,
            icon_tint: None,
            guide_fade: None,
            chevron_collapsed: "+",
            chevron_expanded: "-",
        }
    }

    /// A stripped-back theme: basic ANSI colors, no icons, a plain full-row
    /// selection, classic connector guides.
    fn plain() -> Theme {
        Theme {
            id: ThemeId::Plain,
            palette: Palette {
                app_bg: None,
                name_fg: None,
                dir_fg: Some(Color::Blue),
                selection_bg: Color::DarkGray,
                selection_fg: None,
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
            icon_tint: None,
            guide_fade: None,
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
            ThemeId::Gruvbox,
            ThemeId::Nord,
            ThemeId::Rosepine,
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
        assert_eq!(birch.icons, IconSet::Devicons);

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

        // jetbrains: no guides (New UI default), folder glyphs, full-row
        // selection.
        let jb = Theme::for_id(ThemeId::Jetbrains);
        assert_eq!(jb.guides, GuideStyle::None);
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

        // Icon families: vscode draws codicons, xcode material glyphs; the
        // rest of the catalog stays on the devicon base map.
        assert_eq!(vscode.icons, IconSet::Codicons);
        assert_eq!(xcode.icons, IconSet::Material);
        assert_eq!(jb.icons, IconSet::Devicons);

        // mocha / tokyonight / gruvbox / nord / rosepine: palette themes —
        // colored bold dirs, surface-level full-row selection, indent guides,
        // thin octicon chevrons, devicon glyphs.
        for id in [
            ThemeId::Mocha,
            ThemeId::Tokyonight,
            ThemeId::Gruvbox,
            ThemeId::Nord,
            ThemeId::Rosepine,
        ] {
            let t = Theme::for_id(id);
            assert_eq!(t.guides, GuideStyle::Indent);
            assert_eq!(t.selection, SelectionStyle::FullRow);
            assert_eq!(t.badges, BadgeStyle::Letter);
            assert_eq!(t.folder_style, FolderStyle::Icon);
            assert_eq!(t.icons, IconSet::Devicons);
            assert!(t.bold_dirs);
            assert_eq!(t.chevron_collapsed, "\u{f460}"); // octicon thin chevron
            assert_eq!(t.chevron_expanded, "\u{f47c}");
            assert!(t.palette.dir_fg.is_some());
        }

        // The palette themes carry deliberately distinct dir colours (gruvbox
        // yellow, nord frost cyan, rosé pine iris — not blue-everywhere).
        let dir_colors: Vec<_> = [ThemeId::Gruvbox, ThemeId::Nord, ThemeId::Rosepine]
            .into_iter()
            .map(|id| Theme::for_id(id).palette.dir_fg)
            .collect();
        assert_eq!(dir_colors[0], Some(Color::Rgb(0xfa, 0xbd, 0x2f)));
        assert_eq!(dir_colors[1], Some(Color::Rgb(0x88, 0xc0, 0xd0)));
        assert_eq!(dir_colors[2], Some(Color::Rgb(0xc4, 0xa7, 0xe7)));

        // retro: the Commander — DOS blue canvas, black-on-cyan cursor bar,
        // +/- tree marks, no icons (ASCII purity).
        let retro = Theme::for_id(ThemeId::Retro);
        assert_eq!(retro.guides, GuideStyle::Connectors);
        assert_eq!(retro.selection, SelectionStyle::FullRow);
        assert_eq!(retro.icons, IconSet::None);
        assert!(retro.palette.app_bg.is_some());
        assert!(retro.palette.selection_fg.is_some());
        assert_eq!(retro.chevron_collapsed, "+");

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
