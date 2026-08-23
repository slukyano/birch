//! Moving files to the platform trash, and bringing them back.
//!
//! Split out from [`crate::ops`] because a delete that can be undone needs one
//! thing an ordinary delete does not: **a record of where the file went**.
//!
//! The `trash` crate does not provide it. On macOS it calls
//! `-[NSFileManager trashItemAtURL:resultingItemURL:error:]` with a null
//! out-parameter, discarding the resulting URL; and its `os_limited` module —
//! which holds `list` and `restore_all` — is compiled only for Windows and
//! Freedesktop systems. So each platform gets what it can actually offer:
//!
//! - **macOS:** birch makes the same `NSFileManager` call and *keeps* the
//!   resulting URL. The OS still performs the move, so this is not a
//!   hand-rolled trash ([ADR 0031](../../docs/adr/0031-delete-goes-to-trash.md)
//!   decision 4 stands); birch only stops throwing the answer away. Restoring
//!   is then a plain rename back.
//!
//!   Going straight to `NSFileManager` also settles the delete sound for good.
//!   The crate's default method drives Finder over `osascript`, which plays
//!   Finder's delete sound and needs Automation permission — a permission the
//!   user can deny, turning an ordinary delete into a trash failure. Neither
//!   applies here, and neither can be reintroduced by a change of default.
//! - **Elsewhere:** the crate's own record, restored by `os_limited::restore_all`,
//!   which understands `.trashinfo` files and per-mount trash directories.

use std::path::{Path, PathBuf};

use crate::ops::OpError;

/// One path that went to the trash, and what it takes to bring it back.
#[derive(Clone, Debug)]
pub struct Trashed {
    /// Where it was, and where a restore puts it back.
    pub from: PathBuf,
    pub ticket: Ticket,
}

/// What a restore needs, which is not the same thing on every platform.
#[derive(Clone, Debug)]
pub enum Ticket {
    /// Where the item now sits in the trash. Restoring is a rename back.
    InTrash(PathBuf),
    /// The platform trash's own record, which `restore_all` consumes.
    Record(Box<trash::TrashItem>),
    /// The delete happened, but nothing came back to identify it — an older
    /// platform path, or a redirected trash under test. Restoring says so
    /// rather than guessing at a file.
    Untracked,
}

/// Moves `path` to the trash, reporting what it takes to restore it.
pub fn to_trash(path: &Path) -> Result<Trashed, OpError> {
    let ticket = platform_trash(path)?;
    Ok(Trashed {
        from: path.to_path_buf(),
        ticket,
    })
}

/// Puts a trashed path back where it was. Refuses rather than overwrites: the
/// old location may have been filled since, by the user or anything else.
pub fn restore(trashed: &Trashed) -> Result<(), OpError> {
    if crate::ops::occupied(&trashed.from) {
        return Err(OpError::Exists(trashed.from.clone()));
    }
    match &trashed.ticket {
        Ticket::InTrash(current) => {
            if !crate::ops::occupied(current) {
                return Err(OpError::NoTrash(format!(
                    "{} is no longer in the trash",
                    name(&trashed.from)
                )));
            }
            if let Some(parent) = trashed.from.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::rename(current, &trashed.from)?;
            Ok(())
        }
        Ticket::Record(item) => restore_record(item),
        Ticket::Untracked => Err(OpError::NoTrash(format!(
            "{} is in the trash — birch cannot bring it back on this platform",
            name(&trashed.from)
        ))),
    }
}

fn name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

// ---- macOS: keep the resulting URL the crate discards ----

#[cfg(target_os = "macos")]
fn platform_trash(path: &Path) -> Result<Ticket, OpError> {
    use objc2_foundation::{NSFileManager, NSString, NSURL};

    let text = path
        .to_str()
        .ok_or_else(|| OpError::NoTrash("path is not valid UTF-8".into()))?;
    let manager = NSFileManager::defaultManager();
    let url = NSURL::fileURLWithPath(&NSString::from_str(text));
    let mut landed = None;
    manager
        .trashItemAtURL_resultingItemURL_error(&url, Some(&mut landed))
        .map_err(|e| OpError::NoTrash(e.localizedDescription().to_string()))?;
    // The OS is not obliged to fill it in. An empty answer means the delete
    // happened and the restore cannot be offered, which is worth saying — never
    // guessing at a file in the trash with a matching name.
    Ok(match landed {
        Some(url) => match url.path() {
            Some(path) => Ticket::InTrash(PathBuf::from(path.to_string())),
            None => Ticket::Untracked,
        },
        None => Ticket::Untracked,
    })
}

// ---- everywhere else: the crate's own record ----

#[cfg(not(target_os = "macos"))]
fn platform_trash(path: &Path) -> Result<Ticket, OpError> {
    trash::delete(path).map_err(|e| OpError::NoTrash(e.to_string()))?;
    // `list` is the only way back to the record the delete just wrote. The
    // newest entry whose original path matches is the one that was just made.
    let mine = trash::os_limited::list()
        .map_err(|e| OpError::NoTrash(e.to_string()))?
        .into_iter()
        .filter(|item| item.original_path() == path)
        .max_by_key(|item| item.time_deleted);
    Ok(match mine {
        Some(item) => Ticket::Record(Box::new(item)),
        None => Ticket::Untracked,
    })
}

#[cfg(target_os = "macos")]
fn restore_record(_item: &trash::TrashItem) -> Result<(), OpError> {
    // Unreachable: `platform_trash` never produces a record here, because
    // `os_limited` — which is what consumes one — is not compiled for macOS.
    Err(OpError::NoTrash("restoring is unavailable here".into()))
}

#[cfg(not(target_os = "macos"))]
fn restore_record(item: &trash::TrashItem) -> Result<(), OpError> {
    trash::os_limited::restore_all([item.clone()]).map_err(|e| OpError::NoTrash(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Uses the **real** trash, deliberately: the whole point is that the
    /// platform reports where it put the file. It restores what it trashed, so
    /// the trash is left exactly as it was found.
    #[test]
    fn a_trashed_file_reports_where_it_went_and_comes_back() {
        let dir = std::env::temp_dir().join(format!("birch-disposal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let victim = dir.join("victim.txt");
        std::fs::write(&victim, b"CONTENTS").unwrap();

        let trashed = to_trash(&victim).expect("the platform trash took it");
        assert!(!victim.exists(), "it left its directory");
        match &trashed.ticket {
            Ticket::InTrash(at) => assert!(at.exists(), "and is where the OS said: {at:?}"),
            Ticket::Record(_) => {}
            Ticket::Untracked => panic!("the platform reported nothing to restore from"),
        }

        restore(&trashed).expect("it came back");
        assert_eq!(
            std::fs::read(&victim).unwrap(),
            b"CONTENTS",
            "with its contents, at its old path"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_restore_refuses_rather_than_overwriting() {
        let dir = std::env::temp_dir().join(format!("birch-disposal-taken-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let victim = dir.join("victim.txt");
        std::fs::write(&victim, b"OLD").unwrap();

        let trashed = to_trash(&victim).unwrap();
        // Something else takes the name while the file sits in the trash.
        std::fs::write(&victim, b"SOMEONE ELSE").unwrap();
        assert!(
            matches!(restore(&trashed), Err(OpError::Exists(_))),
            "it refuses the collision"
        );
        assert_eq!(std::fs::read(&victim).unwrap(), b"SOMEONE ELSE");

        // Leave the trash as it was found.
        if let Ticket::InTrash(at) = &trashed.ticket {
            let _ = std::fs::remove_file(at);
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
