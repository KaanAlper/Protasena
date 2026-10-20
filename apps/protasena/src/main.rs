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

fn services() -> Services {
    Services {
        pick_open: Some(Box::new(|| {
            rfd::FileDialog::new().add_filter("Projects (psproj, pdsprj)", &["psproj", "pdsprj"]).add_filter("All files", &["*"]).pick_file()
        })),
        pick_save: Some(Box::new(|name: &str, exts: &[&str]| rfd::FileDialog::new().set_file_name(name).add_filter("Files", exts).save_file())),
    }
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
            let mut app = App::new(Session::new(), services());
            if let Some(t) = args.theme {
                app.ui.theme = t;
            }
            if let Some(dir) = args.library.clone().or_else(protasena_io::library_dir) {
                let (tx, rx) = std::sync::mpsc::channel();
                let ctx = cc.egui_ctx.clone();
                let d = dir.clone();
                let _ = std::thread::Builder::new().name("library".into()).spawn(move || {
                    let _ = tx.send(protasena_io::load_library_from(Some(&d)).map_err(|e| e.to_string()));
                    ctx.request_repaint();
                });
                app = app.with_library_loading(dir.display().to_string(), rx);
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
