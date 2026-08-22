//! The four file operations: rename, create, and delete-to-trash.
//!
//! Functions over real paths that return the [`TreeDelta`]s they imply. No
//! view, no ratatui, no terminal — the op layer decides what happened on disk
//! and the tree reconciles.
//!
//! An operation does **not** need to suppress the watcher. A dirty directory is
//! re-listed into a `Snapshot`, which reconciles rather than appends
//! (ADR 0006), so applying a delta here and letting the snapshot follow
//! converges on the same state either way.

use std::path::{Path, PathBuf};
use std::{fmt, fs, io};

use crate::tree::{Entry, NodeKind, TreeDelta};

/// Why an operation could not be performed. Every variant is something a user
/// can act on, which is the point: a failed operation says what stopped it.
#[derive(Debug)]
pub enum OpError {
    /// The name is empty, or contains a path separator where one is not
    /// allowed.
    BadName(String),
    /// Something already exists at the destination. Nothing here overwrites.
    Exists(PathBuf),
    /// A directory would be moved inside itself.
    IntoOwnDescendant,
    /// The destination's parent does not exist.
    NoParent(PathBuf),
    /// The filesystem refused.
    Io(io::Error),
    /// The platform trash is unavailable — no XDG trash directory, an exotic
    /// filesystem, a network mount. Nothing is deleted and the failure is
    /// reported: birch has no delete that bypasses the trash (ADR 0031).
    NoTrash(String),
}

impl fmt::Display for OpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpError::BadName(name) => write!(f, "{name:?} is not a usable name"),
            OpError::Exists(path) => write!(f, "{} already exists", display(path)),
            OpError::IntoOwnDescendant => write!(f, "a folder cannot be moved inside itself"),
            OpError::NoParent(path) => write!(f, "{} does not exist", display(path)),
            OpError::Io(e) => write!(f, "{e}"),
            OpError::NoTrash(why) => write!(f, "no trash available: {why}"),
        }
    }
}

impl From<io::Error> for OpError {
    fn from(e: io::Error) -> Self {
        OpError::Io(e)
    }
}

fn display(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

pub type OpResult = Result<Vec<TreeDelta>, OpError>;

/// Renames `path` to `input`, relative to its parent. An input carrying
/// separators (`../other/name.ts`) performs a **move**, which is the 80% of
/// move without the complexity cliff; the destination's parent must already
/// exist, and nothing is ever overwritten.
pub fn rename(path: &Path, input: &str) -> OpResult {
    let parent = path.parent().ok_or(OpError::NoParent(path.into()))?;
    let target = resolve(parent, input)?;
    if target == path {
        return Ok(Vec::new());
    }
    move_to(path, &target)
}

/// Moves `path` into `directory`, keeping its name. The drag gesture's
/// operation (task 072); the same guards as a rename, since it is one.
pub fn move_into(path: &Path, directory: &Path) -> OpResult {
    let name = path.file_name().ok_or(OpError::NoParent(path.into()))?;
    move_to(path, &directory.join(name))
}

/// Moves `path` to exactly `target`. The primitive a rename and a drag both
/// come down to, and the one undo reverses a step with: it already knows both
/// ends, so it needs no name to resolve.
pub fn move_to(path: &Path, target: &Path) -> OpResult {
    if target == path {
        return Ok(Vec::new());
    }
    guard_destination(path, target)?;
    fs::rename(path, target)?;
    Ok(vec![
        TreeDelta::Removed { path: path.into() },
        added(target),
    ])
}

/// Creates a file or directory under `directory` from an inline input.
/// `foo/bar/baz.ts` creates the intermediate directories; a trailing slash
/// (`foo/`) creates a directory instead of a file. One binding, three
/// operations.
pub fn create(directory: &Path, input: &str, force_dir: bool) -> OpResult {
    // Read from the *trimmed* input, as `resolve` does: `sub/ ` with a trailing
    // space asks for a directory just as plainly as `sub/`. `force_dir` is what
    // the `New Folder` action sets, so the two routes land in one place.
    let trimmed = input.trim();
    let is_dir =
        force_dir || trimmed.ends_with('/') || trimmed.ends_with(std::path::MAIN_SEPARATOR);
    let target = resolve(directory, input)?;
    // A create makes something *here*. An input that climbs out would write
    // outside the tree, where no row could ever show it — refused rather than
    // performed invisibly.
    if !target.starts_with(directory) {
        return Err(OpError::BadName(input.trim().to_string()));
    }
    if occupied(&target) {
        return Err(OpError::Exists(target));
    }
    let parent = target.parent().ok_or(OpError::NoParent(target.clone()))?;
    fs::create_dir_all(parent)?;
    if is_dir {
        fs::create_dir(&target)?;
    } else {
        fs::File::create(&target)?;
    }
    // Everything from the first new component down is news to the tree, so the
    // shallowest created ancestor is what gets announced.
    let announce = shallowest_new(directory, &target);
    Ok(vec![added(&announce)])
}

/// What a multi-path removal actually did: the deltas for the paths that went,
/// and the failure that stopped the rest. Returning only the error would leave
/// the tree showing rows whose files are already gone.
pub struct Removal {
    pub deltas: Vec<TreeDelta>,
    pub failed: Option<OpError>,
}

/// Moves paths to the platform trash. Stops at the first failure — a partial
/// delete is reported, never silent.
pub fn trash(paths: &[PathBuf]) -> Removal {
    if let Some(directory) = REDIRECT.get() {
        return remove_each(paths, |path| move_aside(path, directory));
    }
    let context = trash_context();
    remove_each(paths, |path| {
        // A failure here ends the delete. birch has no way to remove a file
        // that does not go through the trash, so there is nothing to fall back
        // to and nothing to ask (ADR 0031).
        context
            .delete(path)
            .map_err(|e| OpError::NoTrash(e.to_string()))
    })
}

/// The trash to delete into.
///
/// On macOS the crate's default is `DeleteMethod::Finder`, which runs
/// `osascript -e 'tell application "Finder" to delete …'`. That costs two
/// things: Finder plays its delete sound on every call, and driving Finder
/// needs Automation permission — which the user can **deny**, and a denial
/// turns into a `NoTrash` failure, pushing birch into the "the trash could not
/// take it" failure for a reason that has nothing to do with the trash.
///
/// `NsFileManager` is silent, faster, and needs no permission. What it gives up
/// is Finder's **Put Back**, which is the only restore a macOS user has —
/// `trash`'s own `os_limited::restore_all` is not compiled for macOS at all.
/// A trashed file still sits in the trash and can be dragged out; it no longer
/// remembers where it came from. ADR 0031 records that trade.
fn trash_context() -> trash::TrashContext {
    #[allow(unused_mut)]
    let mut context = trash::TrashContext::default();
    #[cfg(target_os = "macos")]
    {
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        context.set_delete_method(DeleteMethod::NsFileManager);
    }
    context
}

/// Sends removals to `directory` instead of the platform trash.
///
/// **For tests only.** A suite that deletes into the developer's real trash
/// fills it a few files per run and — before the switch above — played a sound
/// each time. Set once per test binary; later calls are ignored.
pub fn redirect_trash_for_tests(directory: PathBuf) {
    let _ = REDIRECT.set(directory);
}

static REDIRECT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// The redirect's stand-in for trashing: move the path into `directory`,
/// suffixing on collision so a suite that trashes the same name twice does not
/// fail on the second.
fn move_aside(path: &Path, directory: &Path) -> Result<(), OpError> {
    fs::create_dir_all(directory)?;
    let name = display(path);
    let mut target = directory.join(&name);
    for n in 1.. {
        if !occupied(&target) {
            break;
        }
        target = directory.join(format!("{name}.{n}"));
    }
    fs::rename(path, &target)?;
    Ok(())
}

fn remove_each(paths: &[PathBuf], mut remove: impl FnMut(&Path) -> Result<(), OpError>) -> Removal {
    let mut deltas = Vec::with_capacity(paths.len());
    for path in paths {
        if let Err(e) = remove(path) {
            return Removal {
                deltas,
                failed: Some(e),
            };
        }
        deltas.push(TreeDelta::Removed { path: path.clone() });
    }
    Removal {
        deltas,
        failed: None,
    }
}

/// Resolves an inline input against a directory, refusing anything that is not
/// a usable name. Absolute inputs are refused: an inline edit renames a row,
/// and a row lives somewhere.
fn resolve(base: &Path, input: &str) -> Result<PathBuf, OpError> {
    let trimmed = input.trim();
    let cleaned = trimmed.trim_end_matches(['/', std::path::MAIN_SEPARATOR]);
    if cleaned.is_empty() || Path::new(cleaned).is_absolute() {
        return Err(OpError::BadName(trimmed.to_string()));
    }
    let joined = base.join(cleaned);
    normalize(&joined).ok_or_else(|| OpError::BadName(trimmed.to_string()))
}

/// Resolves `.` and `..` lexically. The path may not exist yet, so
/// `canonicalize` is not available.
fn normalize(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    (!out.as_os_str().is_empty()).then_some(out)
}

fn guard_destination(source: &Path, target: &Path) -> Result<(), OpError> {
    if target.starts_with(source) {
        return Err(OpError::IntoOwnDescendant);
    }
    // Not `exists()`: that follows symlinks, so a **dangling** link at the
    // destination would report absent and be silently replaced. Nothing here
    // overwrites, links included.
    if occupied(target) && !same_file(source, target) {
        return Err(OpError::Exists(target.into()));
    }
    match target.parent() {
        Some(parent) if parent.is_dir() => Ok(()),
        Some(parent) => Err(OpError::NoParent(parent.into())),
        None => Err(OpError::NoParent(target.into())),
    }
}

/// Whether anything at all occupies this path — a file, a directory, or a
/// symlink whose target is missing. `Path::exists` follows links and so calls a
/// dangling one absent.
fn occupied(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// Whether two paths name the same file on disk. On a case-insensitive
/// filesystem `readme.md` and `README.md` do, which is what makes a case-only
/// rename possible rather than a collision with itself.
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn added(path: &Path) -> TreeDelta {
    let kind = if path.is_dir() {
        NodeKind::Dir
    } else {
        NodeKind::File
    };
    TreeDelta::Added {
        parent: path.parent().unwrap_or(path).to_path_buf(),
        entries: vec![Entry {
            name: display(path),
            kind,
        }],
    }
}

/// The shallowest component of `target` that did not exist under `base` before
/// the create — which is the one the tree has not heard of.
fn shallowest_new(base: &Path, target: &Path) -> PathBuf {
    let Ok(relative) = target.strip_prefix(base) else {
        return target.to_path_buf();
    };
    let mut walk = base.to_path_buf();
    for component in relative.components() {
        walk.push(component);
        if walk == target {
            break;
        }
    }
    // The first component under `base` is the shallowest thing created, since
    // `create_dir_all` made every level below it in this call.
    match relative.components().next() {
        Some(first) => base.join(first),
        None => target.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dir(PathBuf);

    impl Dir {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "birch-ops-{}-{tag}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            // No test reaches the developer's real trash. Installed here rather
            // than per-test because every test that trashes builds a scratch
            // directory first, so future ones inherit it.
            redirect_trash_for_tests(
                std::env::temp_dir().join(format!("birch-test-trash-{}", std::process::id())),
            );
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Dir(path)
        }

        fn join(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }

        fn file(&self, name: &str) -> PathBuf {
            let path = self.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, b"x").unwrap();
            path
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_rename_moves_the_file_and_reports_both_ends() {
        let dir = Dir::new("rename");
        let path = dir.file("a.txt");
        let deltas = rename(&path, "b.txt").unwrap();
        assert!(!path.exists() && dir.join("b.txt").exists());
        assert!(matches!(deltas[0], TreeDelta::Removed { .. }));
        assert!(matches!(deltas[1], TreeDelta::Added { .. }));
    }

    #[test]
    fn renaming_to_a_path_performs_a_move() {
        let dir = Dir::new("rename-path");
        fs::create_dir(dir.join("sub")).unwrap();
        let path = dir.file("a.txt");
        rename(&path, "sub/a.txt").unwrap();
        assert!(!path.exists() && dir.join("sub/a.txt").exists());
    }

    #[test]
    fn nothing_overwrites_and_nothing_moves_into_itself() {
        let dir = Dir::new("guards");
        let a = dir.file("a.txt");
        dir.file("b.txt");
        assert!(matches!(rename(&a, "b.txt"), Err(OpError::Exists(_))));

        fs::create_dir_all(dir.join("outer/inner")).unwrap();
        assert!(matches!(
            move_into(&dir.join("outer"), &dir.join("outer/inner")),
            Err(OpError::IntoOwnDescendant)
        ));
        // A name that resolves to nothing is refused rather than guessed at.
        assert!(matches!(rename(&a, "   "), Err(OpError::BadName(_))));
        assert!(matches!(
            rename(&a, "/etc/passwd"),
            Err(OpError::BadName(_))
        ));
    }

    #[test]
    fn create_makes_files_directories_and_intermediates() {
        let dir = Dir::new("create");
        create(&dir.0, "a.txt", false).unwrap();
        assert!(dir.join("a.txt").is_file());

        create(&dir.0, "sub/", false).unwrap();
        assert!(dir.join("sub").is_dir());

        // Intermediates are made, and the tree hears about the shallowest one.
        let deltas = create(&dir.0, "deep/deeper/c.txt", false).unwrap();
        assert!(dir.join("deep/deeper/c.txt").is_file());
        match &deltas[0] {
            TreeDelta::Added { parent, entries } => {
                assert_eq!(parent, &dir.0);
                assert_eq!(entries[0].name, "deep");
                assert_eq!(entries[0].kind, NodeKind::Dir);
            }
            other => panic!("expected an Added, got {other:?}"),
        }

        assert!(matches!(
            create(&dir.0, "a.txt", false),
            Err(OpError::Exists(_))
        ));
    }

    #[test]
    fn a_move_into_a_directory_keeps_the_name() {
        let dir = Dir::new("move-into");
        let a = dir.file("a.txt");
        fs::create_dir(dir.join("sub")).unwrap();
        move_into(&a, &dir.join("sub")).unwrap();
        assert!(dir.join("sub/a.txt").is_file() && !a.exists());
    }

    /// `readme.md` → `README.md` is a routine rename, and on a case-insensitive
    /// filesystem the destination "exists" — as the same file.
    #[test]
    fn a_case_only_rename_is_not_a_collision_with_itself() {
        let dir = Dir::new("case");
        let path = dir.file("readme.md");
        rename(&path, "README.md").unwrap();
        let names: Vec<String> = fs::read_dir(&dir.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["README.md"]);
    }

    /// A dangling symlink is *something* at the destination. `exists()` follows
    /// links and calls it absent, which would silently unlink it.
    #[test]
    fn a_dangling_symlink_still_blocks_a_destination() {
        let dir = Dir::new("dangling");
        let a = dir.file("a.txt");
        #[cfg(unix)]
        std::os::unix::fs::symlink(dir.join("nowhere"), dir.join("link")).unwrap();
        assert!(matches!(rename(&a, "link"), Err(OpError::Exists(_))));
        assert!(
            fs::symlink_metadata(dir.join("link")).is_ok(),
            "still there"
        );
        assert!(a.exists(), "and the source was not moved");
    }

    #[test]
    fn a_create_cannot_climb_out_of_its_directory() {
        let dir = Dir::new("escape");
        fs::create_dir(dir.join("sub")).unwrap();
        assert!(matches!(
            create(&dir.join("sub"), "../escaped.txt", false),
            Err(OpError::BadName(_))
        ));
        assert!(!dir.join("escaped.txt").exists());
    }

    #[test]
    fn a_trailing_space_does_not_hide_a_trailing_slash() {
        let dir = Dir::new("trailing");
        create(&dir.0, "sub/ ", false).unwrap();
        assert!(dir.join("sub").is_dir(), "still a directory");
    }
}
