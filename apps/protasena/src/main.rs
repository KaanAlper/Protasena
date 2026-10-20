//! `protasena`: Protasena schematic capture, the desktop application.
//!
//! ```text
//! protasena [PROJECT] [--library DIR] [--control PORT] [--theme light|dark]
//! ```
//! - `PROJECT`: a `.psproj` file, or a Proteus `.pdsprj` project (imported).
//! - `--library DIR` (else `PROTASENA_LIBRARY_DIR`): the parts pack (the directory with
//!   `index.json`), loaded in the background; without it only the built-in parts exist.
//! - `--control PORT` (else `PROTASENA_CONTROL_PORT`): the JSON-lines control channel on
//!   `127.0.0.1:PORT` (`{"id":1,"method":"engine.execute","params":{…}}`; methods in
//!   `protasena_ui_egui::control::METHODS`).
//! - `--theme` (else `PROTASENA_THEME`): the interface theme, light by default.
//!
//! The system settings (System menu) are read from `settings.json` in the configuration
//! directory (`PROTASENA_CONFIG_DIR`, else the platform's); the parts pack falls back to their
//! library folder.
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod control_server;

use std::path::PathBuf;
use std::process::ExitCode;

use protasena_engine::Session;
use protasena_ui_egui::theme::ThemePref;
use protasena_ui_egui::{App, Services};

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const USAGE: &str = "usage: protasena [PROJECT] [--library DIR] [--control PORT] [--theme light|dark]";

/// What the command line asked for.
#[derive(Debug, Default, PartialEq)]
struct Args {
    project: Option<PathBuf>,
    library: Option<PathBuf>,
    control: Option<u16>,
    theme: Option<ThemePref>,
}

/// Parses the arguments (after the program name). `Err` carries a message and the exit code.
fn parse(args: impl IntoIterator<Item = String>, env: impl Fn(&str) -> Option<String>) -> Result<Args, (String, u8)> {
    let mut a = Args {
        control: env("PROTASENA_CONTROL_PORT").and_then(|p| p.trim().parse().ok()),
        theme: env("PROTASENA_THEME").and_then(|t| theme(&t)),
        ..Args::default()
    };
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => return Err((format!("{NAME} {VERSION}: {}\n\n{USAGE}", env!("CARGO_PKG_DESCRIPTION")), 0)),
            "-V" | "--version" => return Err((format!("{NAME} {VERSION}"), 0)),
            "--control" => {
                let v = it.next().ok_or_else(|| ("--control needs a port".to_owned(), 2))?;
                a.control = Some(v.trim().parse().map_err(|_| (format!("--control: {v:?} is not a port"), 2))?);
            }
            "--library" => a.library = Some(PathBuf::from(it.next().ok_or_else(|| ("--library needs a directory".to_owned(), 2))?)),
            "--theme" => {
                let v = it.next().unwrap_or_default();
                a.theme = Some(theme(&v).ok_or_else(|| (format!("--theme: {v:?} is not light or dark"), 2))?);
            }
            s if s.starts_with('-') => return Err((format!("unknown option {s}\n{USAGE}"), 2)),
            _ if a.project.is_some() => return Err((format!("one project at a time\n{USAGE}"), 2)),
            _ => a.project = Some(PathBuf::from(arg)),
        }
    }
    Ok(a)
}

fn theme(s: &str) -> Option<ThemePref> {
    match s.trim().to_ascii_lowercase().as_str() {
        "light" => Some(ThemePref::Light),
        "dark" => Some(ThemePref::Dark),
        _ => None,
    }
}

/// A file chooser starting in `dir` when there is one.
fn chooser(dir: Option<&std::path::Path>) -> rfd::FileDialog {
    let d = rfd::FileDialog::new();
    match dir.filter(|d| d.is_dir()) {
        Some(dir) => d.set_directory(dir),
        None => d,
    }
}

fn services() -> Services {
    Services {
        pick_open: Some(Box::new(|dir: Option<&std::path::Path>| {
            chooser(dir).add_filter("Projects (psproj, pdsprj)", &["psproj", "pdsprj"]).add_filter("All files", &["*"]).pick_file()
        })),
        pick_save: Some(Box::new(|name: &str, exts: &[&str], dir: Option<&std::path::Path>| {
            chooser(dir).set_file_name(name).add_filter("Files", exts).save_file()
        })),
        open_path: Some(Box::new(open_in_viewer)),
    }
}

/// A session with the user's system settings (the settings file in the configuration
/// directory); problems reading it are warnings, never a failure.
fn session() -> (Session, Vec<String>) {
    let mut session = Session::new();
    let Some(path) = protasena_io::settings_path() else { return (session, Vec::new()) };
    let (settings, mut warnings) = protasena_io::load_settings(&path);
    warnings.extend(session.set_settings(settings, Some(path)));
    (session, warnings)
}

/// Opens a file with the system's default application (`xdg-open`, `open`, `explorer`). The
/// child is waited for on a thread of its own, so it never lingers as a zombie; a program that
/// cannot be started is reported, never a crash.
fn open_in_viewer(path: &std::path::Path) -> Result<(), String> {
    let program = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let mut child = std::process::Command::new(program).arg(path).spawn().map_err(|e| format!("{program}: {e}"))?;
    let _ = std::thread::Builder::new().name("viewer".into()).spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn main() -> ExitCode {
    let args = match parse(std::env::args().skip(1), |k| std::env::var(k).ok()) {
        Ok(a) => a,
        Err((msg, 0)) => {
            println!("{msg}");
            return ExitCode::SUCCESS;
        }
        Err((msg, code)) => {
            eprintln!("{NAME}: {msg}");
            return ExitCode::from(code);
        }
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Protasena")
            .with_inner_size([1600.0, 1000.0])
            .with_min_inner_size([960.0, 600.0])
            .with_app_id("io.github.kaanalper.protasena"),
        persist_window: false,
        ..Default::default()
    };
    let result = eframe::run_native(
        "Protasena",
        options,
        Box::new(move |cc| {
            let (session, warnings) = session();
            let settings_library = session.settings().library_dir.as_ref().map(PathBuf::from);
            let mut app = App::new(session, services());
            if let Some(t) = args.theme {
                app.ui.theme = t;
            }
            for w in &warnings {
                eprintln!("{NAME}: {w}");
            }
            if let Some(w) = warnings.first() {
                app.notify(w.clone(), true);
            }
            let pack = args.library.clone().or_else(protasena_io::library_dir).or(settings_library);
            let kicad: Vec<PathBuf> = app.session.settings().kicad_libraries.iter().map(PathBuf::from).collect();
            let cache = protasena_io::cache_dir();
            app.session.set_cache_dir(cache.clone());
            if pack.is_some() || !kicad.is_empty() {
                let (tx, rx) = std::sync::mpsc::channel();
                let ctx = cc.egui_ctx.clone();
                let p = pack.clone();
                let _ = std::thread::Builder::new().name("library".into()).spawn(move || {
                    let _ = tx.send(protasena_engine::load_libraries(p.as_deref(), &kicad, cache.as_deref()));
                    ctx.request_repaint();
                });
                app = app.with_library_loading(pack.map(|d| d.display().to_string()).unwrap_or_default(), rx);
            }
            if let Some(port) = args.control {
                let ctx = cc.egui_ctx.clone();
                app = app.with_control(control_server::start(port, move || ctx.request_repaint()));
            }
            if let Some(p) = &args.project
                && let Err(e) = app.run("open", serde_json::json!({ "path": p.display().to_string() }))
            {
                eprintln!("{NAME}: {e}");
                app.notify(e, true);
            }
            Ok(Box::new(Window(app)))
        }),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{NAME}: {e}");
            ExitCode::FAILURE
        }
    }
}

/// The eframe application around the window.
struct Window(App);

impl eframe::App for Window {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.0.logic(ctx);
        if std::mem::take(&mut self.0.quit_requested) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.0.ui(ui);
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.0.raw_input_hook(raw);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn args(v: &[&str]) -> Result<Args, (String, u8)> {
        parse(v.iter().map(|s| (*s).to_owned()), |_| None)
    }

    #[test]
    fn options_and_project() {
        let a = args(&["x.pdsprj", "--control", "47000", "--library", "/p", "--theme", "dark"]).unwrap();
        assert_eq!(a, Args { project: Some("x.pdsprj".into()), library: Some("/p".into()), control: Some(47000), theme: Some(ThemePref::Dark) });
        assert_eq!(args(&["--version"]).unwrap_err().1, 0);
        for bad in [&["--control"][..], &["--control", "port"], &["--control", "70000"], &["--frob"], &["a", "b"], &["--theme", "blue"]] {
            assert_eq!(args(bad).unwrap_err().1, 2, "{bad:?}");
        }
    }

    #[test]
    fn environment_gives_defaults() {
        let a = parse(Vec::new(), |k| match k {
            "PROTASENA_CONTROL_PORT" => Some("4100".into()),
            "PROTASENA_THEME" => Some("Dark".into()),
            _ => None,
        })
        .unwrap();
        assert_eq!((a.control, a.theme), (Some(4100), Some(ThemePref::Dark)));
    }
}
