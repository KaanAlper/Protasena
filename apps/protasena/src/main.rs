//! `protasena`: Protasena schematic capture, the desktop application.
//!
//! A placeholder until M1 Task 9; it answers `--help` and `--version` and otherwise says that it is
//! not available yet.

use std::process::ExitCode;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("-V" | "--version") => {
            println!("{NAME} {VERSION}");
            ExitCode::SUCCESS
        }
        Some("-h" | "--help") => {
            println!("{NAME} {VERSION}: {ABOUT}\n\nusage: {NAME} [--help | --version]\n\nNot available yet (M1 Task 9).");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{NAME}: not available yet (M1 Task 9)");
            ExitCode::FAILURE
        }
    }
}
