//! birch-ctl: thin control-socket client (ADRs 0010/0011). Resolves the
//! instance socket, sends one NDJSON request, prints the response data.
//!
//! Exit codes: 0 ok, 1 the instance answered with an error or the transport
//! failed mid-request, 2 no instance found / unreachable.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use birch_core::protocol::{self, PathForm, Request, Response, SettingKey, Verb};
use clap::{Parser, Subcommand};

/// Control a running birch instance.
#[derive(Parser, Debug)]
#[command(name = "birch-ctl", version, about)]
struct Cli {
    /// Socket path (default: $BIRCH_SOCKET, else the nearest instance found
    /// by walking up from the current directory).
    #[arg(long, global = true, value_name = "path")]
    socket: Option<PathBuf>,

    #[command(subcommand)]
    verb: VerbCmd,
}

#[derive(Subcommand, Debug)]
enum VerbCmd {
    /// Expand to and select a path.
    Reveal { path: PathBuf },
    /// Print the current selection.
    GetPath {
        /// Print just the file name.
        #[arg(long, conflicts_with_all = ["rel", "abs"])]
        name: bool,
        /// Print the root-relative path (default).
        #[arg(long, conflicts_with = "abs")]
        rel: bool,
        /// Print the absolute path.
        #[arg(long)]
        abs: bool,
    },
    /// Print the tree root.
    GetRoot,
    /// Change a runtime setting.
    Set {
        #[arg(value_enum)]
        setting: SettingArg,
        /// on/off/true/false/1/0/toggle.
        value: String,
    },
    /// Re-root the tree.
    SetRoot { dir: PathBuf },
    /// Open the current selection.
    Open,
    /// Exit the instance.
    Quit,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum SettingArg {
    Hidden,
    Ignored,
    Noise,
    Icons,
    Compact,
    Git,
    FilesFirst,
}

impl From<SettingArg> for SettingKey {
    fn from(value: SettingArg) -> Self {
        match value {
            SettingArg::Hidden => SettingKey::Hidden,
            SettingArg::Ignored => SettingKey::Ignored,
            SettingArg::Noise => SettingKey::Noise,
            SettingArg::Icons => SettingKey::Icons,
            SettingArg::Compact => SettingKey::Compact,
            SettingArg::Git => SettingKey::Git,
            SettingArg::FilesFirst => SettingKey::FilesFirst,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let request = match build_request(&cli.verb) {
        Ok(request) => request,
        Err(message) => {
            eprintln!("birch-ctl: {message}");
            return ExitCode::from(1);
        }
    };

    let socket = cli
        .socket
        .or_else(|| std::env::var_os("BIRCH_SOCKET").map(PathBuf::from))
        .or_else(resolve_by_walking_up);
    let Some(socket) = socket else {
        eprintln!("birch-ctl: no running birch instance found for this directory");
        return ExitCode::from(2);
    };

    match roundtrip(&socket, &request) {
        Ok(response) if response.ok => {
            if let Some(data) = response.data {
                println!("{data}");
            }
            ExitCode::SUCCESS
        }
        Ok(response) => {
            eprintln!(
                "birch-ctl: {}",
                response.error.as_deref().unwrap_or("request failed")
            );
            ExitCode::from(1)
        }
        Err(e) => {
            let unreachable = matches!(
                e.kind(),
                std::io::ErrorKind::NotFound
                    | std::io::ErrorKind::ConnectionRefused
                    | std::io::ErrorKind::PermissionDenied
            );
            eprintln!("birch-ctl: cannot reach {}: {e}", socket.display());
            ExitCode::from(if unreachable { 2 } else { 1 })
        }
    }
}

fn build_request(verb: &VerbCmd) -> Result<Request, String> {
    let request = match verb {
        VerbCmd::Reveal { path } => {
            let mut request = Request::new(Verb::Reveal);
            request.path = Some(absolutize(path)?);
            request
        }
        VerbCmd::GetPath { name, rel: _, abs } => {
            let mut request = Request::new(Verb::GetPath);
            request.form = Some(if *name {
                PathForm::Name
            } else if *abs {
                PathForm::Abs
            } else {
                PathForm::Rel
            });
            request
        }
        VerbCmd::GetRoot => Request::new(Verb::GetRoot),
        VerbCmd::Set { setting, value } => {
            let mut request = Request::new(Verb::Set);
            request.setting = Some((*setting).into());
            request.value = Some(value.clone());
            request
        }
        VerbCmd::SetRoot { dir } => {
            let mut request = Request::new(Verb::SetRoot);
            request.path = Some(absolutize(dir)?);
            request
        }
        VerbCmd::Open => Request::new(Verb::Open),
        VerbCmd::Quit => Request::new(Verb::Quit),
    };
    Ok(request)
}

/// Paths are absolutized against the *client's* cwd — the server's cwd is
/// unrelated.
fn absolutize(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .map_err(|e| format!("cannot resolve the current directory: {e}"))
}

/// Default resolution: walk up from cwd; the first ancestor whose by-root
/// link exists names the nearest enclosing instance.
fn resolve_by_walking_up() -> Option<PathBuf> {
    let dir = protocol::socket_dir();
    let cwd = std::env::current_dir().ok()?.canonicalize().ok()?;
    cwd.ancestors()
        .map(|ancestor| protocol::by_root_link(&dir, ancestor))
        .find(|link| link.exists())
}

fn roundtrip(socket: &Path, request: &Request) -> std::io::Result<Response> {
    let mut stream = UnixStream::connect(socket)?;
    let mut line = serde_json::to_string(request)?;
    line.push('\n');
    stream.write_all(line.as_bytes())?;
    let mut reader = BufReader::new(stream);
    let mut reply = String::new();
    reader.read_line(&mut reply)?;
    serde_json::from_str(&reply).map_err(std::io::Error::other)
}
