//! The Supernaut binary: arg parsing, wiring, mode selection — per NORTH-STAR
//! §4.2 and its naming amendment (Supernaut app, havoc engine).

mod credentials;
mod session;
mod session_backlog;
mod session_print;
mod session_wait;
mod wiring;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use havoc_core::storage::Storage;

#[derive(Parser, Debug)]
#[command(name = "supernaut", version, disable_help_subcommand = true)]
struct Cli {
    /// Where history lives. Defaults to $XDG_DATA_HOME/supernaut.
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Debug session: drive the havoc engine over the typed boundary.
    Session(session::SessionArgs),
    /// Manage the SASL passwords in the OS keyring.
    Credential {
        #[command(subcommand)]
        action: CredentialAction,
    },
}

/// `set` and nothing else: a `get` would print what NORTH-STAR §5.8 forbids
/// printing, and the session's own startup error is already the diagnostic that
/// says whether an entry is there. `rm` arrives when somebody needs it.
#[derive(clap::Subcommand, Debug)]
enum CredentialAction {
    /// Store a configured network's SASL password, read from **stdin**. Never
    /// from an argument: `ps` is world-readable.
    Set {
        /// The network name, as the config file spells it. The account name is
        /// read from that network's `sasl_account` key.
        network: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        None => open_store_and_report(cli.data_dir),
        Some(Command::Session(mut args)) => {
            if args.data_dir.is_none() {
                args.data_dir = cli.data_dir;
            }
            let runtime = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(error) => {
                    eprintln!("runtime: {error}");
                    return ExitCode::FAILURE;
                }
            };
            match runtime.block_on(session::run(args)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(message) => {
                    eprintln!("{message}");
                    ExitCode::FAILURE
                }
            }
        }
        // No tokio runtime: talking to the keychain is a blocking syscall and
        // this subcommand dials nothing.
        Some(Command::Credential {
            action: CredentialAction::Set { network },
        }) => match credentials::set(&network) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                ExitCode::FAILURE
            }
        },
    }
}

/// The no-subcommand path — prompt 3's documented acceptance, byte for byte:
/// print name/version, open (creating/migrating) the store, report, exit 0.
///
/// It reads **no config file**, and that is the whole of NORTH-STAR §3.1's
/// works-before-configuration property that stage 1 can honestly claim: config
/// is mandatory for `session` only. The product's answer is stage 2's first-run,
/// which may be neither a flags fallback nor a silent config write.
fn open_store_and_report(data_dir: Option<PathBuf>) -> ExitCode {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let data_dir = match data_dir {
        Some(dir) => dir,
        None => match default_data_dir() {
            Ok(dir) => dir,
            Err(message) => {
                eprintln!("{message}");
                return ExitCode::FAILURE;
            }
        },
    };

    if let Err(error) = std::fs::create_dir_all(&data_dir) {
        eprintln!("cannot create data dir {}: {error}", data_dir.display());
        return ExitCode::FAILURE;
    }

    let db_path = data_dir.join("history.db");
    match Storage::open(&db_path, false) {
        Ok((storage, report)) => {
            let state = if report.applied() > 0 {
                format!(
                    "migrated v{} -> v{}",
                    report.from_version, report.to_version
                )
            } else {
                "up-to-date".to_owned()
            };
            let version = storage.client().schema_version().unwrap_or(-1);
            println!(
                "history: {} (schema v{version}, {state})",
                db_path.display()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot open history at {}: {error}", db_path.display());
            ExitCode::FAILURE
        }
    }
}

/// Where the network seed file lives: `SUPERNAUT_CONFIG_DIR`, then
/// `XDG_CONFIG_HOME/supernaut`, then `$HOME/.config/supernaut`. Locating files
/// is the binary's job; parsing them is havoc-core's (`config::parse` does no
/// file I/O at all). One knob per location, deliberately — there is no
/// `--config` flag, because two ways to say where the file is means two things
/// to check when it is not found.
pub(crate) fn default_config_path() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("SUPERNAUT_CONFIG_DIR").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(dir).join("config.toml"));
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(xdg).join("supernaut").join("config.toml"));
    }
    std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(|home| PathBuf::from(home).join(".config/supernaut/config.toml"))
        .ok_or_else(|| {
            "cannot locate config.toml: none of SUPERNAUT_CONFIG_DIR, XDG_CONFIG_HOME or HOME is set"
                .to_owned()
        })
}

/// Locate, read and parse the config file: the one sentence both `session` and
/// `credential set` say, so "cannot read config" reads identically whichever
/// subcommand a person typed. `tls_ca` resolves against the file's own
/// directory, which is why the base dir travels into the parser.
pub(crate) fn load_config() -> Result<havoc_core::config::Config, String> {
    let config_path = default_config_path()?;
    let text = std::fs::read_to_string(&config_path).map_err(|e| {
        format!(
            "cannot read config {}: {e} — write the file, or point SUPERNAUT_CONFIG_DIR \
             at the directory holding it",
            config_path.display()
        )
    })?;
    let base_dir = config_path.parent().unwrap_or(std::path::Path::new("."));
    havoc_core::config::parse(&text, base_dir)
}

pub(crate) fn default_data_dir() -> Result<PathBuf, String> {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(xdg).join("supernaut"));
    }
    std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(|home| PathBuf::from(home).join(".local/share/supernaut"))
        .ok_or_else(|| "neither XDG_DATA_HOME nor HOME is set; pass --data-dir".to_owned())
}
