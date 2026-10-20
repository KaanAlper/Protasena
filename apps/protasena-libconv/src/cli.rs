//! Command line: arguments, help text and exit codes.

use std::path::PathBuf;

use protasena_libimport::{Map, Options, par};

/// Exit code: converted with zero failures.
pub const EXIT_OK: u8 = 0;
/// Exit code: the pack was written but some items failed (see the report).
pub const EXIT_FAILURES: u8 = 1;
/// Exit code: bad arguments or a problem that stopped the run.
pub const EXIT_USAGE: u8 = 2;

/// Usage text.
pub const USAGE: &str = "\
usage: protasena-libconv --symbols DIR --footprints DIR --map FILE --out DIR
                         [--report FILE] [--jobs N] [--upstream TEXT]

  --symbols DIR      directory with KiCad .kicad_sym symbol libraries
  --footprints DIR   directory with KiCad .pretty footprint libraries
  --map FILE         map.toml: categories, Proteus-style names, manufacturers
  --out DIR          pack directory to write (new, empty, or a previous pack)
  --report FILE      write the conversion report as JSON
  --jobs N           worker threads (default: all cores)
  --upstream TEXT    note recorded in the report, e.g. \"KiCad libraries 9.0.9.1\"
  -h, --help         this text
  -V, --version      version

Exit status: 0 = converted with no failures, 1 = written but with failures, 2 = error.";

/// What the command line asks for.
#[derive(Debug)]
pub enum Command {
    /// Print the usage text.
    Help,
    /// Print the version.
    Version,
    /// Convert.
    Convert {
        /// Conversion options.
        options: Box<Options>,
        /// Where to write the report.
        report: Option<PathBuf>,
    },
}

/// Parses the arguments (without the program name); reads and parses the map.
///
/// # Errors
/// A message for the user (missing or unknown flags, unreadable or invalid map).
pub fn parse(args: &[String]) -> Result<Command, String> {
    let (mut symbols, mut footprints, mut map, mut out, mut report) = (None, None, None, None, None);
    let mut jobs = par::default_jobs();
    let mut upstream = String::new();
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let mut value = || it.next().cloned().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "--symbols" => symbols = Some(PathBuf::from(value()?)),
            "--footprints" => footprints = Some(PathBuf::from(value()?)),
            "--map" => map = Some(PathBuf::from(value()?)),
            "--out" => out = Some(PathBuf::from(value()?)),
            "--report" => report = Some(PathBuf::from(value()?)),
            "--upstream" => upstream = value()?,
            "--jobs" => jobs = value()?.parse::<usize>().ok().filter(|&n| (1..=1024).contains(&n)).ok_or("--jobs needs a number from 1 to 1024")?,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let need = |v: Option<PathBuf>, flag: &str| v.ok_or_else(|| format!("{flag} is required"));
    let map_path = need(map, "--map")?;
    let text = std::fs::read_to_string(&map_path).map_err(|e| format!("{}: {e}", map_path.display()))?;
    let map = Map::parse(&text).map_err(|e| format!("{}: {e}", map_path.display()))?;
    let options = Options {
        symbols: need(symbols, "--symbols")?,
        footprints: need(footprints, "--footprints")?,
        map,
        out: need(out, "--out")?,
        jobs,
        upstream,
    };
    Ok(Command::Convert { options: Box::new(options), report })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn help_version_and_errors() {
        assert!(matches!(parse(&args(&["--help"])), Ok(Command::Help)));
        assert!(matches!(parse(&args(&["-V"])), Ok(Command::Version)));
        assert!(parse(&args(&["--bogus"])).unwrap_err().contains("unknown"));
        assert!(parse(&args(&["--out"])).unwrap_err().contains("needs a value"));
        assert!(parse(&args(&["--jobs", "0"])).unwrap_err().contains("--jobs"));
        assert!(parse(&args(&[])).unwrap_err().contains("--map"));
        assert!(parse(&args(&["--map", "/nonexistent/map.toml"])).is_err());
    }
}
