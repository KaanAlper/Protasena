//! Workspace tooling: `cargo xtask <command>`.
//!
//! Pure Rust (std + serde_json). `cargo` and `git` are invoked through `std::process::Command`.

mod assets;
mod layers;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "\
usage: cargo xtask <command>

commands:
  ci      fmt --check, clippy -D warnings, test, layers, assets (stops at the first failure)
  layers  enforce the crate dependency layering (AGENTS.md \"Architecture rules\")
  assets  check that every image/font/data file under crates/ and apps/ is in ATTRIBUTION.md
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = root().and_then(|root| match args.first().map(String::as_str) {
        Some("ci") => ci(&root),
        Some("layers") => layers::run(&root),
        Some("assets") => assets::run(&root),
        Some("-h" | "--help" | "help") | None => {
            print!("{USAGE}");
            Ok(())
        }
        Some(other) => Err(format!("unknown command `{other}`\n\n{USAGE}")),
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Workspace root (parent of the xtask crate).
fn root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().map(Path::to_path_buf).ok_or_else(|| "xtask has no parent directory".to_owned())
}

/// `cargo` (the one running us, if any) with `dir` as working directory.
pub fn cargo(dir: &Path) -> Command {
    let mut c = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    c.current_dir(dir);
    c
}

fn run(mut cmd: Command, what: &str) -> Result<(), String> {
    eprintln!("$ {what}");
    let status = cmd.status().map_err(|e| format!("{what}: failed to spawn: {e}"))?;
    if status.success() { Ok(()) } else { Err(format!("{what}: exited with {status}")) }
}

fn cargo_step(root: &Path, args: &[&str]) -> Result<(), String> {
    let mut c = cargo(root);
    c.args(args);
    run(c, &format!("cargo {}", args.join(" ")))
}

fn ci(root: &Path) -> Result<(), String> {
    type Step<'a> = (&'static str, Box<dyn Fn() -> Result<(), String> + 'a>);
    let steps: Vec<Step> = vec![
        ("fmt", Box::new(|| cargo_step(root, &["fmt", "--all", "--", "--check"]))),
        ("clippy", Box::new(|| cargo_step(root, &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]))),
        ("test", Box::new(|| cargo_step(root, &["test", "--workspace"]))),
        ("layers", Box::new(|| layers::run(root))),
        ("assets", Box::new(|| assets::run(root))),
    ];
    let mut done = Vec::new();
    for (name, step) in &steps {
        eprintln!("\n=== ci: {name} ===");
        if let Err(e) = step() {
            println!("\nCI summary:");
            for d in &done {
                println!("  ok    {d}");
            }
            println!("  FAIL  {name}: {e}");
            for (n, _) in steps.iter().skip(done.len() + 1) {
                println!("  skip  {n}");
            }
            return Err(format!("ci failed at `{name}`"));
        }
        done.push(*name);
    }
    println!("\nCI summary: all {} steps passed ({})", done.len(), done.join(", "));
    Ok(())
}
