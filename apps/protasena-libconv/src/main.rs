//! `protasena-libconv`: KiCad libraries → Protasena library pack. See the library crate docs
//! and `--help`.

use std::process::ExitCode;

use protasena_libconv::cli::{self, Command, EXIT_FAILURES, EXIT_OK, EXIT_USAGE};

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (options, report_path) = match cli::parse(&args) {
        Ok(Command::Help) => {
            println!("{NAME} {VERSION}: {ABOUT}\n\n{}", cli::USAGE);
            return ExitCode::from(EXIT_OK);
        }
        Ok(Command::Version) => {
            println!("{NAME} {VERSION}");
            return ExitCode::from(EXIT_OK);
        }
        Ok(Command::Convert { options, report }) => (options, report),
        Err(e) => {
            eprintln!("{NAME}: {e}\n\n{}", cli::USAGE);
            return ExitCode::from(EXIT_USAGE);
        }
    };
    let report = match protasena_libconv::run(&options) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{NAME}: {e}");
            return ExitCode::from(EXIT_USAGE);
        }
    };
    let c = &report.counts;
    println!(
        "{} symbols in {} libraries → {} devices; {} footprints in {} libraries → {} packages; {} failures",
        c.symbols,
        c.symbol_libraries,
        c.devices,
        c.footprints,
        c.footprint_libraries,
        c.packages,
        report.failures.len()
    );
    for f in report.failures.iter().take(20) {
        eprintln!("  FAILED {}: {}", f.what, f.error);
    }
    if let Some(path) = report_path {
        let written = report.to_json().map_err(|e| e.to_string()).and_then(|t| std::fs::write(&path, t + "\n").map_err(|e| e.to_string()));
        if let Err(e) = written {
            eprintln!("{NAME}: {}: {e}", path.display());
            return ExitCode::from(EXIT_USAGE);
        }
    }
    ExitCode::from(if report.failures.is_empty() { EXIT_OK } else { EXIT_FAILURES })
}
