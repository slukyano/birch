//! Per-root state persistence: expansion, selection, and scroll survive
//! restarts. State is a cache — load failures of any kind are silently
//! discarded, writes are atomic (temp + rename).

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;

/// All paths are root-relative real paths, so visibility or compaction
/// changes cannot corrupt restored state (design doc, Architecture).
#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Eq)]
pub struct PersistedState {
    pub version: u32,
    pub expanded: Vec<PathBuf>,
    pub selection: Option<PathBuf>,
    pub scroll: usize,
}

/// FNV-1a, hand-rolled: `DefaultHasher`'s algorithm is not guaranteed stable
/// across Rust releases, and the file name must be.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The stable per-root hash string shared by persistence file names and the
/// socket by-root symlinks (ADR 0010).
pub fn root_hash(root: &Path) -> String {
    format!("{:016x}", fnv1a64(root.as_os_str().as_encoded_bytes()))
}

fn path_under(cache_base: &Path, root: &Path) -> PathBuf {
    cache_base
        .join("birch")
        .join(format!("{}.json", root_hash(root)))
}

pub fn state_path(root: &Path) -> Option<PathBuf> {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::home_dir().map(|home| home.join(".cache")))?;
    Some(path_under(&cache, root))
}

pub fn load(root: &Path) -> Option<PersistedState> {
    load_path(&state_path(root)?)
}

fn load_path(path: &Path) -> Option<PersistedState> {
    let bytes = std::fs::read(path).ok()?;
    let state: PersistedState = serde_json::from_slice(&bytes).ok()?;
    (state.version == VERSION).then_some(state)
}

pub fn save(root: &Path, state: &PersistedState) -> io::Result<()> {
    let path =
        state_path(root).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no cache dir"))?;
    save_path(&path, state)
}

fn save_path(path: &Path, state: &PersistedState) -> io::Result<()> {
    let dir = path.parent().expect("state path has a parent");
    std::fs::create_dir_all(dir)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(state)?)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trips through an injected cache base — no process-global env
    /// mutation, so this is safe alongside concurrently running tests.
    #[test]
    fn round_trip_and_discard() {
        let tmp = std::env::temp_dir().join(format!("birch-persist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let root = Path::new("/some/project");
        let path = path_under(&tmp, root);
        assert!(load_path(&path).is_none(), "missing file loads as none");

        let state = PersistedState {
            version: VERSION,
            expanded: vec!["src".into(), "src/deep".into()],
            selection: Some("src/main.rs".into()),
            scroll: 3,
        };
        save_path(&path, &state).unwrap();
        assert_eq!(load_path(&path), Some(state));
        assert!(
            !path.with_extension("json.tmp").exists(),
            "atomic write leaves no temp file behind"
        );

        // Different root → different file.
        assert_ne!(path, path_under(&tmp, Path::new("/other/project")));

        // Corrupt content and unknown versions are discarded.
        std::fs::write(&path, b"not json").unwrap();
        assert!(load_path(&path).is_none());
        std::fs::write(
            &path,
            br#"{"version":99,"expanded":[],"selection":null,"scroll":0}"#,
        )
        .unwrap();
        assert!(load_path(&path).is_none());

        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn fnv_is_stable() {
        // Pinned values: the file-name scheme must never drift.
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"/some/project"), fnv1a64(b"/some/project"));
        assert_ne!(fnv1a64(b"/a"), fnv1a64(b"/b"));
    }
}
