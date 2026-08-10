//! The Supernaut binary: arg parsing, wiring, mode selection — per NORTH-STAR
//! §4.2 and its naming amendment (Supernaut app, havoc engine).

use std::path::PathBuf;
use std::process::ExitCode;

use havoc_core::storage::Storage;

fn main() -> ExitCode {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let data_dir = match data_dir_from_args(std::env::args().skip(1)) {
        Ok(dir) => dir,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(error) = std::fs::create_dir_all(&data_dir) {
        eprintln!("cannot create data dir {}: {error}", data_dir.display());
        return ExitCode::FAILURE;
    }

    let db_path = data_dir.join("history.db");
    match Storage::open(&db_path) {
        Ok((storage, report)) => {
            let state = if report.applied() > 0 {
                format!(
                    "migrated v{} -> v{}",
                    report.from_version, report.to_version
                )
            } else {
                "up-to-date".to_owned()
            };
            let version = storage.schema_version().unwrap_or(-1);
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

/// `--data-dir <path>` overrides the default of `$XDG_DATA_HOME/supernaut`
/// (falling back to `~/.local/share/supernaut`, the XDG default). Hand-parsed:
/// the surface is one flag, and an argument-parsing dependency is not yet
/// justified (CLAUDE.md dependency policy).
fn data_dir_from_args(mut args: impl Iterator<Item = String>) -> Result<PathBuf, String> {
    match args.next().as_deref() {
        None => default_data_dir(),
        Some("--data-dir") => match args.next() {
            Some(dir) if args.next().is_none() => Ok(PathBuf::from(dir)),
            Some(_) => Err("unexpected extra arguments after --data-dir <path>".to_owned()),
            None => Err("--data-dir requires a path".to_owned()),
        },
        Some(other) => Err(format!(
            "unknown argument: {other}\nusage: supernaut [--data-dir <path>]"
        )),
    }
}

fn default_data_dir() -> Result<PathBuf, String> {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(xdg).join("supernaut"));
    }
    std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(|home| PathBuf::from(home).join(".local/share/supernaut"))
        .ok_or_else(|| "neither XDG_DATA_HOME nor HOME is set; pass --data-dir".to_owned())
}
