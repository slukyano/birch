//! Nerd Font icons per file type: a small hand-rolled map, no icon crate.
//! Glyphs are Nerd Fonts v3 codepoints; colors approximate the common
//! devicon palette. The devicon map is the base for every `IconSet`; the
//! codicon/material/octicon families override a handful of common categories
//! ([`Category`]) with their own glyphs and keep the devicon colors.

use birch_core::NodeKind;
use ratatui::style::Color;

use crate::theme::{FolderStyle, IconSet, Theme};

// One folder glyph regardless of expansion: the chevron already carries the
// open/closed state, and a flipping icon is churn in an ambient pane
// (JetBrains/Finder school; an open-folder variant can return as a style).
const DIR: &str = "\u{e5ff}"; //
const FILE: &str = "\u{f016}"; //
const DIR_COLOR: Color = Color::Rgb(0x7a, 0xa2, 0xf7);
const FILE_COLOR: Color = Color::Rgb(0x9a, 0xa5, 0xb1);

/// The categories a non-devicon `IconSet` restyles with family-specific
/// glyphs. Everything else falls back to the devicon map.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Category {
    Folder,
    GenericFile,
    Markdown,
    Json,
    Config,
    Image,
    License,
    Lock,
}

/// Resolves a row's icon glyph+color from the active theme's `IconSet` and
/// `FolderStyle`. `None` means "draw no icon": the theme has no icon set, the
/// `Plain` style suppresses all icons, or a directory under a non-`Icon` style
/// (its chevron stands in for the folder glyph).
pub fn icon_for(theme: &Theme, name: &str, kind: NodeKind) -> Option<(&'static str, Color)> {
    if theme.folder_style == FolderStyle::Plain || theme.icons == IconSet::None {
        return None;
    }
    // Only the `Icon` layout draws a folder glyph; `Compact` dirs show only
    // their chevron.
    if kind.is_dir() && theme.folder_style != FolderStyle::Icon {
        return None;
    }
    let (glyph, color) = devicon(name, kind);
    let glyph = category_of(name, kind, glyph)
        .and_then(|category| family_glyph(theme.icons, category))
        .unwrap_or(glyph);
    Some((glyph, color))
}

/// The family-specific glyph for a category, or `None` to keep the devicon
/// glyph. All codepoints verified against Nerd Fonts 3.4.0 `glyphnames.json`.
fn family_glyph(set: IconSet, category: Category) -> Option<&'static str> {
    match set {
        IconSet::None | IconSet::Devicons => None,
        IconSet::Codicons => Some(match category {
            Category::Folder => "\u{ea83}",      // codicon folder
            Category::GenericFile => "\u{ea7b}", // codicon file
            Category::Markdown => "\u{eb1d}",    // codicon markdown
            Category::Json => "\u{eb0f}",        // codicon json
            Category::Config => "\u{eaf8}",      // codicon gear
            Category::Image => "\u{eaea}",       // codicon file-media
            Category::License => "\u{eb12}",     // codicon law
            Category::Lock => "\u{ea75}",        // codicon lock
        }),
        IconSet::Material => Some(match category {
            Category::Folder => "\u{f0256}",      // md folder-outline
            Category::GenericFile => "\u{f0224}", // md file-outline
            Category::Markdown => "\u{f0354}",    // md language-markdown
            Category::Json => "\u{f0626}",        // md code-json
            Category::Config => "\u{f0493}",      // md cog
            Category::Image => "\u{f02e9}",       // md image
            Category::License => "\u{f05d1}",     // md scale-balance
            Category::Lock => "\u{f033e}",        // md lock
        }),
        IconSet::Octicons => match category {
            Category::Folder => Some("\u{f413}"),      // oct file-directory
            Category::GenericFile => Some("\u{f4a5}"), // oct file
            Category::Markdown => Some("\u{f48a}"),    // oct markdown
            Category::Json => None,                    // devicon fallback
            Category::Config => Some("\u{f423}"),      // oct gear
            Category::Image => Some("\u{f4e5}"),       // oct image
            Category::License => Some("\u{f495}"),     // oct law
            Category::Lock => Some("\u{f456}"),        // oct lock
        },
    }
}

/// Classifies a row into an override [`Category`], `None` when only the
/// devicon map applies. `devicon_glyph` detects the generic-file fallback.
fn category_of(name: &str, kind: NodeKind, devicon_glyph: &str) -> Option<Category> {
    if kind.is_dir() {
        return Some(Category::Folder);
    }
    if devicon_glyph == FILE {
        return Some(Category::GenericFile);
    }
    if matches!(
        name,
        "LICENSE" | "LICENSE.md" | "LICENSE.txt" | "COPYING" | "NOTICE"
    ) {
        return Some(Category::License);
    }
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("md" | "markdown") => Some(Category::Markdown),
        Some("json" | "jsonc") => Some(Category::Json),
        Some("toml" | "ini" | "cfg" | "conf" | "yml" | "yaml") => Some(Category::Config),
        Some("png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" | "bmp") => {
            Some(Category::Image)
        }
        Some("lock") => Some(Category::Lock),
        _ => None,
    }
}

fn devicon(name: &str, kind: NodeKind) -> (&'static str, Color) {
    if kind.is_dir() {
        return (DIR, DIR_COLOR);
    }
    if let Some(hit) = by_name(name) {
        return hit;
    }
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("rs") => ("\u{e7a8}", Color::Rgb(0xde, 0x78, 0x3c)),
        Some("py") => ("\u{e73c}", Color::Rgb(0xff, 0xd4, 0x3b)),
        Some("js" | "mjs" | "cjs") => ("\u{e74e}", Color::Rgb(0xf1, 0xe0, 0x5a)),
        Some("ts" | "mts") => ("\u{e628}", Color::Rgb(0x31, 0x78, 0xc6)),
        Some("jsx" | "tsx") => ("\u{e7ba}", Color::Rgb(0x61, 0xda, 0xfb)),
        Some("json" | "jsonc") => ("\u{e60b}", Color::Rgb(0xcb, 0xbb, 0x4a)),
        Some("toml" | "ini" | "cfg" | "conf") => ("\u{e615}", Color::Rgb(0x9a, 0xa5, 0xb1)),
        Some("yml" | "yaml") => ("\u{e615}", Color::Rgb(0xcb, 0x4b, 0x4b)),
        Some("md" | "markdown") => ("\u{e73e}", Color::Rgb(0x51, 0x9a, 0xba)),
        Some("html" | "htm") => ("\u{e736}", Color::Rgb(0xe4, 0x4d, 0x26)),
        Some("css" | "scss" | "less") => ("\u{e749}", Color::Rgb(0x56, 0x3d, 0x7c)),
        Some("sh" | "bash" | "zsh" | "fish") => ("\u{e795}", Color::Rgb(0x89, 0xe0, 0x51)),
        Some("go") => ("\u{e626}", Color::Rgb(0x00, 0xad, 0xd8)),
        Some("c" | "h") => ("\u{e61e}", Color::Rgb(0x55, 0x9d, 0xd3)),
        Some("cpp" | "cc" | "cxx" | "hpp") => ("\u{e61d}", Color::Rgb(0xf3, 0x4b, 0x7d)),
        Some("java") => ("\u{e738}", Color::Rgb(0xcc, 0x37, 0x2d)),
        Some("rb") => ("\u{e739}", Color::Rgb(0xcc, 0x34, 0x2d)),
        Some("php") => ("\u{e73d}", Color::Rgb(0x77, 0x7b, 0xb3)),
        Some("lock") => ("\u{f023}", Color::Rgb(0x9a, 0xa5, 0xb1)),
        Some("txt") => ("\u{f15c}", FILE_COLOR),
        Some("png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" | "bmp") => {
            ("\u{f1c5}", Color::Rgb(0xa0, 0x74, 0xc4))
        }
        Some("zip" | "tar" | "gz" | "xz" | "zst" | "bz2" | "7z") => {
            ("\u{f1c6}", Color::Rgb(0xaf, 0xb4, 0x2c))
        }
        Some("pdf") => ("\u{f1c1}", Color::Rgb(0xb3, 0x0b, 0x00)),
        _ => (FILE, FILE_COLOR),
    }
}

fn by_name(name: &str) -> Option<(&'static str, Color)> {
    match name {
        ".gitignore" | ".gitattributes" | ".gitmodules" => {
            Some(("\u{e702}", Color::Rgb(0xf1, 0x4e, 0x32)))
        }
        "Dockerfile" | "Containerfile" => Some(("\u{f308}", Color::Rgb(0x38, 0x4d, 0x54))),
        "Makefile" | "makefile" | "Justfile" | "justfile" => {
            Some(("\u{f489}", Color::Rgb(0x6d, 0x80, 0x86)))
        }
        "LICENSE" | "LICENSE.md" | "LICENSE.txt" | "COPYING" | "NOTICE" => {
            Some(("\u{f24e}", Color::Rgb(0xd4, 0xa9, 0x59)))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use birch_core::ThemeId;

    use super::*;

    #[test]
    fn dirs_files_and_extensions() {
        assert_eq!(devicon("src", NodeKind::Dir).0, DIR);
        assert_eq!(devicon("main.rs", NodeKind::File).0, "\u{e7a8}");
        assert_eq!(devicon("weird.xyz", NodeKind::File).0, FILE);
        assert_eq!(devicon("Makefile", NodeKind::File).0, "\u{f489}");
        // extension matching is case-insensitive
        assert_eq!(devicon("A.RS", NodeKind::File).0, "\u{e7a8}");
    }

    #[test]
    fn icon_set_gates_resolution() {
        let birch = Theme::for_id(ThemeId::Birch);
        assert_eq!(
            icon_for(&birch, "main.rs", NodeKind::File),
            Some(("\u{e7a8}", Color::Rgb(0xde, 0x78, 0x3c)))
        );
        // Birch shows the devicon folder glyph.
        assert_eq!(
            icon_for(&birch, "src", NodeKind::Dir),
            Some((DIR, DIR_COLOR))
        );

        // Plain has no icon set at all.
        let plain = Theme::for_id(ThemeId::Plain);
        assert_eq!(icon_for(&plain, "main.rs", NodeKind::File), None);
        assert_eq!(icon_for(&plain, "src", NodeKind::Dir), None);

        // vscode is FolderStyle::Compact: files keep their icon, dirs get none
        // (the chevron stands in for the folder glyph).
        let vscode = Theme::for_id(ThemeId::Vscode);
        assert_eq!(
            icon_for(&vscode, "main.rs", NodeKind::File),
            Some(("\u{e7a8}", Color::Rgb(0xde, 0x78, 0x3c)))
        );
        assert_eq!(icon_for(&vscode, "src", NodeKind::Dir), None);
    }

    #[test]
    fn icon_families_override_categories_and_keep_devicon_colors() {
        // vscode (Codicons): markdown gets the codicon glyph, devicon color.
        let vscode = Theme::for_id(ThemeId::Vscode);
        assert_eq!(
            icon_for(&vscode, "README.md", NodeKind::File),
            Some(("\u{eb1d}", Color::Rgb(0x51, 0x9a, 0xba)))
        );
        // Non-category files fall back to the devicon map unchanged.
        assert_eq!(
            icon_for(&vscode, "main.rs", NodeKind::File).map(|(g, _)| g),
            Some("\u{e7a8}")
        );
        // Generic files get the codicon file glyph.
        assert_eq!(
            icon_for(&vscode, "weird.xyz", NodeKind::File),
            Some(("\u{ea7b}", FILE_COLOR))
        );

        // xcode (Material): folder is the md outline glyph (supplementary-
        // plane codepoint), config files get the cog.
        let xcode = Theme::for_id(ThemeId::Xcode);
        assert_eq!(
            icon_for(&xcode, "src", NodeKind::Dir),
            Some(("\u{f0256}", DIR_COLOR))
        );
        assert_eq!(
            icon_for(&xcode, "Cargo.toml", NodeKind::File).map(|(g, _)| g),
            Some("\u{f0493}")
        );

        // birch (Devicons) keeps the base map everywhere.
        let birch = Theme::for_id(ThemeId::Birch);
        assert_eq!(
            icon_for(&birch, "src", NodeKind::Dir),
            Some((DIR, DIR_COLOR))
        );
        assert_eq!(
            icon_for(&birch, "README.md", NodeKind::File).map(|(g, _)| g),
            Some("\u{e73e}")
        );

        // Octicons: json has no octicon override — devicon fallback; the lock
        // and license categories do.
        assert_eq!(family_glyph(IconSet::Octicons, Category::Json), None);
        assert_eq!(
            family_glyph(IconSet::Octicons, Category::Lock),
            Some("\u{f456}")
        );
        assert_eq!(
            family_glyph(IconSet::Octicons, Category::License),
            Some("\u{f495}")
        );
    }
}
