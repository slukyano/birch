//! The control-socket wire protocol (ADR 0011) and socket addressing
//! (ADR 0010). Shared by the server (birch) and the client (birch-ctl) so
//! the shapes cannot drift. NDJSON, versioned, additive-only.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::persist;

pub const VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum Verb {
    Reveal,
    GetPath,
    GetRoot,
    Set,
    SetRoot,
    Open,
    Quit,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PathForm {
    Name,
    #[default]
    Rel,
    Abs,
}

/// Runtime-settable toggles (`birch-ctl set <setting> <value>`).
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum SettingKey {
    Hidden,
    Ignored,
    Noise,
    Icons,
    Compact,
    Git,
    FilesFirst,
}

/// One request line. Unknown fields are ignored on parse (serde default);
/// fields irrelevant to a verb are simply absent.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Request {
    pub v: u32,
    pub verb: Verb,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<PathForm>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting: Option<SettingKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

impl Request {
    pub fn new(verb: Verb) -> Self {
        Self {
            v: VERSION,
            verb,
            path: None,
            form: None,
            setting: None,
            value: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Response {
    pub v: u32,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Response {
    pub fn ok(data: Option<String>) -> Self {
        Self {
            v: VERSION,
            ok: true,
            data,
            error: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            v: VERSION,
            ok: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// `on`/`off`/`true`/`false`/`1`/`0`/`toggle` (ADR 0011).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingValue {
    On,
    Off,
    Toggle,
}

impl SettingValue {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "on" | "true" | "1" => Some(Self::On),
            "off" | "false" | "0" => Some(Self::Off),
            "toggle" => Some(Self::Toggle),
            _ => None,
        }
    }

    pub fn apply(self, current: bool) -> bool {
        match self {
            Self::On => true,
            Self::Off => false,
            Self::Toggle => !current,
        }
    }
}

// ---- addressing (ADR 0010) ----

/// The per-user socket dir: `$XDG_RUNTIME_DIR/birch`, else
/// `<tmp>/birch-<uid>`.
pub fn socket_dir() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from) {
        Some(runtime) if runtime.is_absolute() => runtime.join("birch"),
        _ => std::env::temp_dir().join(format!("birch-{}", effective_uid())),
    }
}

/// The effective uid (socket-dir naming and ownership checks).
pub fn effective_uid() -> u32 {
    // SAFETY: geteuid has no failure modes or preconditions. One syscall
    // wrapper is not worth the libc crate.
    unsafe { libc_geteuid() }
}

unsafe extern "C" {
    #[link_name = "geteuid"]
    fn libc_geteuid() -> u32;
}

pub fn instance_socket(dir: &Path, pid: u32) -> PathBuf {
    dir.join(format!("{pid}.sock"))
}

/// The most-recent-instance symlink for a root (same hash as persistence).
pub fn by_root_link(dir: &Path, root: &Path) -> PathBuf {
    dir.join("by-root")
        .join(format!("{}.sock", persist::root_hash(root)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_round_trip_and_tolerate_unknown_fields() {
        let mut req = Request::new(Verb::Reveal);
        req.path = Some("/r/src/main.rs".into());
        let line = serde_json::to_string(&req).unwrap();
        let back: Request = serde_json::from_str(&line).unwrap();
        assert_eq!(back.verb, Verb::Reveal);
        assert_eq!(back.path.as_deref(), Some(Path::new("/r/src/main.rs")));

        // Unknown fields (a future protocol addition) parse fine.
        let future = r#"{"v":9,"verb":"get-path","form":"abs","shiny":"ignored"}"#;
        let req: Request = serde_json::from_str(future).unwrap();
        assert_eq!(req.verb, Verb::GetPath);
        assert_eq!(req.form, Some(PathForm::Abs));

        let resp: Response =
            serde_json::from_str(r#"{"v":1,"ok":true,"data":"src","extra":1}"#).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.data.as_deref(), Some("src"));
    }

    #[test]
    fn verbs_and_settings_use_kebab_case() {
        assert_eq!(
            serde_json::to_string(&Verb::GetRoot).unwrap(),
            r#""get-root""#
        );
        assert_eq!(
            serde_json::to_string(&SettingKey::FilesFirst).unwrap(),
            r#""files-first""#
        );
    }

    #[test]
    fn setting_values_parse_and_apply() {
        assert_eq!(SettingValue::parse("on"), Some(SettingValue::On));
        assert_eq!(SettingValue::parse("0"), Some(SettingValue::Off));
        assert_eq!(SettingValue::parse("toggle"), Some(SettingValue::Toggle));
        assert_eq!(SettingValue::parse("maybe"), None);
        assert!(SettingValue::Toggle.apply(false));
        assert!(!SettingValue::Toggle.apply(true));
        assert!(SettingValue::On.apply(false));
    }

    #[test]
    fn addressing_paths() {
        let dir = Path::new("/run/user/1000/birch");
        assert_eq!(
            instance_socket(dir, 42),
            PathBuf::from("/run/user/1000/birch/42.sock")
        );
        let link = by_root_link(dir, Path::new("/some/project"));
        assert!(link.starts_with("/run/user/1000/birch/by-root"));
        assert!(link.extension().is_some_and(|e| e == "sock"));
        // Same root, same link (stable hash).
        assert_eq!(link, by_root_link(dir, Path::new("/some/project")));
    }
}
