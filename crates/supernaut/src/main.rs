//! The Supernaut binary: arg parsing, wiring, mode selection — per NORTH-STAR
//! §4.2 and its naming amendment (Supernaut app, havoc engine).

mod session;
mod session_backlog;
mod session_print;
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
    }
}

/// The no-subcommand path — prompt 3's documented acceptance, byte for byte:
/// print name/version, open (creating/migrating) the store, report, exit 0.
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

pub(crate) fn default_data_dir() -> Result<PathBuf, String> {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(xdg).join("supernaut"));
    }
    std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(|home| PathBuf::from(home).join(".local/share/supernaut"))
        .ok_or_else(|| "neither XDG_DATA_HOME nor HOME is set; pass --data-dir".to_owned())
}
