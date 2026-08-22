//! The inline editor: birch's only mode (ADR 0030).
//!
//! A rename draws **over** an existing row; a create draws a **phantom row**
//! under its target directory. Neither touches the tree — the text lives here
//! until `Enter`, so live updates that reshuffle rows carry the overlay along
//! and nothing arriving from the watcher can disturb what is being typed. An
//! edit abandoned, including by birch dying, leaves nothing behind.
//!
//! Pure logic: no ratatui, no terminal.

use std::path::PathBuf;

/// What the editor is anchored to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditKind {
    /// Renaming a row, drawn over it. Anchored to a real path, so a reshuffle
    /// moves the overlay with the row.
    Rename { path: PathBuf },
    /// Creating under a directory, drawn as a phantom row beneath it.
    Create { parent: PathBuf },
}

/// A text field open over the tree.
#[derive(Clone, Debug)]
pub struct Editor {
    kind: EditKind,
    buffer: String,
    /// Byte offset into `buffer`, always on a character boundary.
    cursor: usize,
}

impl Editor {
    /// Opens over a row, pre-filled with its current name. The cursor sits at
    /// the **extension boundary**, because renaming usually means changing the
    /// stem and typing over the extension is the common accident.
    pub fn rename(path: PathBuf, name: &str) -> Self {
        let cursor = stem_len(name);
        Self {
            kind: EditKind::Rename { path },
            buffer: name.to_string(),
            cursor,
        }
    }

    /// Opens a phantom row under `parent`, empty.
    pub fn create(parent: PathBuf) -> Self {
        Self {
            kind: EditKind::Create { parent },
            buffer: String::new(),
            cursor: 0,
        }
    }

    pub fn kind(&self) -> &EditKind {
        &self.kind
    }

    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    /// The cursor's position in **characters**, which is what a renderer needs
    /// to place a caret.
    pub fn cursor_chars(&self) -> usize {
        self.buffer[..self.cursor].chars().count()
    }

    pub fn insert(&mut self, c: char) {
        self.buffer.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Deletes the character before the cursor. Distinct from clearing the
    /// field: an empty buffer simply stays empty.
    pub fn backspace(&mut self) {
        let Some(previous) = self.buffer[..self.cursor].chars().next_back() else {
            return;
        };
        self.cursor -= previous.len_utf8();
        self.buffer.remove(self.cursor);
    }

    /// Deletes the character under the cursor.
    pub fn delete(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
        }
    }

    pub fn left(&mut self) {
        if let Some(previous) = self.buffer[..self.cursor].chars().next_back() {
            self.cursor -= previous.len_utf8();
        }
    }

    pub fn right(&mut self) {
        if let Some(next) = self.buffer[self.cursor..].chars().next() {
            self.cursor += next.len_utf8();
        }
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.buffer.len();
    }
}

/// Where a name's stem ends: before the final extension, or at the end when
/// there is none. A leading dot is part of the name (`.gitignore` has no
/// extension), which is why the search starts after the first character.
fn stem_len(name: &str) -> usize {
    match name.char_indices().nth(1) {
        Some((after_first, _)) => name[after_first..]
            .rfind('.')
            .map_or(name.len(), |at| after_first + at),
        None => name.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rename_starts_at_the_extension_boundary() {
        let editor = Editor::rename("/r/main.rs".into(), "main.rs");
        assert_eq!(editor.buffer(), "main.rs");
        assert_eq!(editor.cursor_chars(), 4, "before the dot");

        // A dotfile has no extension: the whole name is the stem.
        let editor = Editor::rename("/r/.gitignore".into(), ".gitignore");
        assert_eq!(editor.cursor_chars(), ".gitignore".chars().count());

        // Several dots: the last one starts the extension.
        let editor = Editor::rename("/r/a.tar.gz".into(), "a.tar.gz");
        assert_eq!(editor.cursor_chars(), 5);

        // No dot at all.
        let editor = Editor::rename("/r/README".into(), "README");
        assert_eq!(editor.cursor_chars(), 6);
    }

    #[test]
    fn a_create_starts_empty() {
        let editor = Editor::create("/r/src".into());
        assert_eq!(editor.buffer(), "");
        assert_eq!(editor.cursor_chars(), 0);
        assert_eq!(
            editor.kind(),
            &EditKind::Create {
                parent: "/r/src".into()
            }
        );
    }

    #[test]
    fn typing_and_deleting_track_the_cursor() {
        let mut editor = Editor::create("/r".into());
        for c in "abc".chars() {
            editor.insert(c);
        }
        assert_eq!((editor.buffer(), editor.cursor_chars()), ("abc", 3));

        editor.left();
        editor.insert('X');
        assert_eq!((editor.buffer(), editor.cursor_chars()), ("abXc", 3));

        editor.backspace();
        assert_eq!((editor.buffer(), editor.cursor_chars()), ("abc", 2));

        editor.delete();
        assert_eq!((editor.buffer(), editor.cursor_chars()), ("ab", 2));

        editor.home();
        assert_eq!(editor.cursor_chars(), 0);
        // Backspace at the start, and delete at the end, are no-ops rather
        // than errors.
        editor.backspace();
        assert_eq!(editor.buffer(), "ab");
        editor.end();
        editor.delete();
        assert_eq!(editor.buffer(), "ab");
    }

    #[test]
    fn multi_byte_characters_never_split() {
        let mut editor = Editor::rename("/r/wé.txt".into(), "wé.txt");
        // "wé" is the stem: two characters, three bytes.
        assert_eq!(editor.cursor_chars(), 2);
        editor.insert('ü');
        assert_eq!(editor.buffer(), "wéü.txt");
        assert_eq!(editor.cursor_chars(), 3);
        editor.backspace();
        assert_eq!(editor.buffer(), "wé.txt");
        editor.left();
        editor.delete();
        assert_eq!(editor.buffer(), "w.txt");
    }
}
