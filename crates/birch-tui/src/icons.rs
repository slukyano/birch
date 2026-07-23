//! Nerd Font icons per file type: a small hand-rolled map, no icon crate.
//! Glyphs are Nerd Fonts v3 codepoints; colors approximate the common
//! devicon palette.

use birch_core::NodeKind;
use ratatui::style::Color;

// One folder glyph regardless of expansion: the chevron already carries the
// open/closed state, and a flipping icon is churn in an ambient pane
// (JetBrains/Finder school; an open-folder variant can return as a style).
const DIR: &str = "\u{e5ff}"; //
const FILE: &str = "\u{f016}"; //
const DIR_COLOR: Color = Color::Rgb(0x7a, 0xa2, 0xf7);
const FILE_COLOR: Color = Color::Rgb(0x9a, 0xa5, 0xb1);

pub fn icon_for(name: &str, kind: NodeKind) -> (&'static str, Color) {
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
    use super::*;

    #[test]
    fn dirs_files_and_extensions() {
        assert_eq!(icon_for("src", NodeKind::Dir).0, DIR);
        assert_eq!(icon_for("main.rs", NodeKind::File).0, "\u{e7a8}");
        assert_eq!(icon_for("weird.xyz", NodeKind::File).0, FILE);
        assert_eq!(icon_for("Makefile", NodeKind::File).0, "\u{f489}");
        // extension matching is case-insensitive
        assert_eq!(icon_for("A.RS", NodeKind::File).0, "\u{e7a8}");
    }
}
