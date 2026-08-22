//! Putting text on the system clipboard.
//!
//! **One transport per copy, chosen from the environment** — never two written
//! together, so a clipboard manager sees one event and nothing fights another
//! process for an X11 selection. The choice follows the shape Neovim uses:
//!
//! 1. A **remote session** (`$SSH_TTY`) takes OSC 52, because a native tool
//!    there writes a clipboard nobody will ever paste from.
//! 2. Otherwise the **first native tool available**, which matters more than a
//!    backstop: Apple Terminal does not support OSC 52 at all.
//! 3. With **no native tool**, OSC 52 — whose silence costs nothing there,
//!    since there is nothing to fall back to.
//!
//! OSC 52 is a one-way write with no reply, so a *reactive* chain — try it,
//! then fall back on failure — cannot be built: there is no failure to observe.
//!
//! The sequence is built here and written by the caller, which owns the
//! terminal; the choice is a pure function of facts a test can supply. Neither
//! needs a terminal or a clipboard to exercise.

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// A native clipboard command, in the order they are probed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeTool {
    PbCopy,
    WlCopy,
    XClip,
    XSel,
}

impl NativeTool {
    /// The program and the arguments it needs. `xclip` and `xsel` both default
    /// to the PRIMARY selection — the middle-click buffer — so both are told
    /// explicitly to write the one a paste reads.
    pub fn command(self) -> (&'static str, &'static [&'static str]) {
        match self {
            NativeTool::PbCopy => ("pbcopy", &[]),
            NativeTool::WlCopy => ("wl-copy", &[]),
            NativeTool::XClip => ("xclip", &["-selection", "clipboard"]),
            NativeTool::XSel => ("xsel", &["--clipboard", "--input"]),
        }
    }
}

/// How one copy will travel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transport {
    /// An escape sequence for the terminal to act on.
    Osc52,
    /// A subprocess to write to.
    Native(NativeTool),
}

/// Picks the transport. Pure: the caller supplies whether the session is remote
/// and which tools are usable, so every case is reachable from a test without
/// touching the process environment.
pub fn choose(ssh: bool, available: &[NativeTool]) -> Transport {
    match available.first() {
        Some(&tool) if !ssh => Transport::Native(tool),
        _ => Transport::Osc52,
    }
}

/// Whether this looks like a remote session. `$SSH_TTY` is the signal Neovim
/// keys the same decision on: it implies an interactive session on a tty
/// reached over ssh, which is exactly when a local clipboard is the wrong one.
pub fn is_remote() -> bool {
    std::env::var_os("SSH_TTY").is_some_and(|value| !value.is_empty())
}

/// The native tools usable here, most preferred first. A tool is offered only
/// when its display server is present, so `wl-copy` is not proposed without
/// Wayland and `xclip` not without X11.
pub fn available_tools() -> Vec<NativeTool> {
    let mut tools = Vec::new();
    if cfg!(target_os = "macos") && on_path("pbcopy") {
        tools.push(NativeTool::PbCopy);
    }
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && on_path("wl-copy") {
        tools.push(NativeTool::WlCopy);
    }
    if std::env::var_os("DISPLAY").is_some() {
        if on_path("xclip") {
            tools.push(NativeTool::XClip);
        }
        if on_path("xsel") {
            tools.push(NativeTool::XSel);
        }
    }
    tools
}

fn on_path(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| is_executable(&dir.join(program)))
}

#[cfg(unix)]
fn is_executable(path: &PathBuf) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &PathBuf) -> bool {
    path.is_file()
}

/// Performs the copy. Returns the escape sequence the **caller** must write to
/// the terminal, or `None` when a native tool took the text — the terminal
/// belongs to the render layer, and this module never reaches for it.
pub fn copy(text: &str, transport: Transport) -> io::Result<Option<String>> {
    match transport {
        Transport::Osc52 => Ok(Some(osc52(text))),
        Transport::Native(tool) => {
            spawn(text, tool)?;
            Ok(None)
        }
    }
}

/// `OSC 52 ; c ; <base64> BEL` — `c` is the clipboard a paste reads.
pub fn osc52(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))
}

/// Writes the text to a native tool's stdin. Its own output goes to `/dev/null`:
/// birch holds the alternate screen, and `xclip` complaining about a missing
/// `$DISPLAY` would scribble across the tree.
fn spawn(text: &str, tool: NativeTool) -> io::Result<()> {
    let (program, args) = tool.command();
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }
    drop(child.stdin.take());
    // `xclip` forks and stays resident to serve the X11 selection, so waiting
    // for it would hang; the write is what matters and it has landed.
    Ok(())
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let triple = u32::from(chunk[0]) << 16
            | u32::from(chunk.get(1).copied().unwrap_or(0)) << 8
            | u32::from(chunk.get(2).copied().unwrap_or(0));
        for i in 0..4 {
            out.push(if i <= chunk.len() {
                ALPHABET[(triple >> (18 - i * 6) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_remote_session_takes_osc52_even_with_a_tool_at_hand() {
        // The tool would write the remote machine's clipboard, which nobody
        // will ever paste from.
        assert_eq!(choose(true, &[NativeTool::PbCopy]), Transport::Osc52);
        assert_eq!(choose(true, &[]), Transport::Osc52);
    }

    #[test]
    fn a_local_session_prefers_the_first_native_tool() {
        assert_eq!(
            choose(false, &[NativeTool::WlCopy, NativeTool::XClip]),
            Transport::Native(NativeTool::WlCopy)
        );
        // ...and falls to OSC 52 only when there is nothing else, where its
        // silence costs nothing because there is no alternative.
        assert_eq!(choose(false, &[]), Transport::Osc52);
    }

    #[test]
    fn the_x11_tools_are_told_which_selection_to_write() {
        // Both default to PRIMARY, which a paste does not read.
        assert_eq!(
            NativeTool::XClip.command(),
            ("xclip", ["-selection", "clipboard"].as_slice())
        );
        assert_eq!(
            NativeTool::XSel.command(),
            ("xsel", ["--clipboard", "--input"].as_slice())
        );
    }

    #[test]
    fn base64_pads_every_remainder() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn the_sequence_carries_the_payload_verbatim() {
        assert_eq!(osc52("foo"), "\x1b]52;c;Zm9v\x07");
        // Non-ASCII and a newline inside a file name both survive, because the
        // payload is bytes rather than text by the time it is encoded.
        let awkward = "/r/wé\nird.txt";
        let encoded = osc52(awkward);
        assert!(encoded.starts_with("\x1b]52;c;") && encoded.ends_with('\x07'));
        assert_eq!(&encoded[7..encoded.len() - 1], base64(awkward.as_bytes()));
    }

    #[test]
    fn osc52_is_what_copy_hands_back_for_the_caller_to_write() {
        assert_eq!(
            copy("foo", Transport::Osc52).unwrap(),
            Some("\x1b]52;c;Zm9v\x07".into())
        );
    }
}
