//! Protasena user interface (layer L5): Proteus 8 Schematic Capture's window layout drawn with
//! egui in our own theme, and the JSON-lines control channel.
//!
//! Top to bottom, as in Proteus: the menu bar ([`menus`]), the toolbar band with the Global,
//! View, Edit and Design toolbars ([`toolbars`]), the tab strip; on the left the Mode and
//! Orientation toolbars ([`modebar`]) and the pane with the overview window above the object
//! selector ([`sidepane`]); the editing window ([`canvas`]) in the middle; the status bar
//! ([`statusbar`]) at the bottom. Dialogs are in [`dialogs`].
//!
//! The window is thin: it reads the engine's [`Session`] and acts only through its commands
//! ([`Session::execute`] for values entered in a dialog, [`Session::invoke`] for menu picks,
//! toolbar buttons and keys, [`Session::pointer`] for the editing window). Colours come from
//! [`theme::Tokens`], strings from [`i18n`].
#![warn(missing_docs)]

mod actions;
pub mod canvas;
pub mod control;
pub mod dialogs;
pub mod i18n;
pub mod icons;
mod keys;
pub mod menus;
pub mod modebar;
mod remote;
pub mod sidepane;
pub mod statusbar;
pub mod theme;
pub mod toolbars;
pub mod unsaved;
pub mod widgets;

use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use protasena_engine::Session;
use protasena_library::Library;
use serde::{Deserialize, Serialize};

pub use control::{ControlRequest, ControlResponse, PendingReply};
pub use keys::shortcut_name;

/// File choosers the app provides (the native dialogs); without them the window asks for a
/// path in its own small dialog.
#[derive(Default)]
pub struct Services {
    /// Choose a project to open.
    pub pick_open: Option<Box<dyn Fn() -> Option<PathBuf>>>,
    /// Choose where to save: suggested file name and the extensions offered.
    pub pick_save: Option<Box<dyn Fn(&str, &[&str]) -> Option<PathBuf>>>,
    /// Open a file in the system's viewer (Print Design's PDF); `Err` says why it failed.
    pub open_path: Option<Box<dyn Fn(&std::path::Path) -> Result<(), String>>>,
}

/// How the grid is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GridStyle {
    /// Dots at the grid points.
    #[default]
    Dots,
    /// Lines.
    Lines,
}

/// Window settings (not part of the design).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiState {
    /// Light or dark.
    pub theme: theme::ThemePref,
    /// Dots or lines.
    pub grid_style: GridStyle,
}

/// Where the parts library stands.
#[derive(Clone, Debug, PartialEq)]
pub enum LibraryStatus {
    /// Only the built-in parts.
    Builtin,
    /// The pack is loading in the background.
    Loading(String),
    /// The pack is loaded.
    Loaded {
        /// Where from.
        dir: String,
        /// Devices in the library.
        devices: usize,
    },
    /// The pack could not be loaded (the built-in parts stay).
    Failed(String),
}

/// A message shown at the bottom of the editing window for a moment.
#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    /// The text.
    pub text: String,
    /// Whether it reports a refusal or error.
    pub error: bool,
    /// When it goes, in seconds of egui time.
    pub until: f64,
}

/// The application window.
pub struct App {
    /// The editing session.
    pub session: Session,
    /// Window settings.
    pub ui: UiState,
    /// File choosers.
    pub services: Services,
    /// The open dialog.
    pub dialog: Option<dialogs::Dialog>,
    /// A result window (ERC, netlist, messages, about).
    pub results: Option<dialogs::results::Results>,
    /// The transient message.
    pub toast: Option<Toast>,
    /// Editing-window state.
    pub canvas: canvas::CanvasState,
    /// The parts library.
    pub library: LibraryStatus,
    /// Events injected by the control channel, fed to egui one by one.
    pub synthetic: Vec<egui::Event>,
    /// Set once the window may close (File ▸ Exit Application, the close button, `app.quit`,
    /// after the unsaved-changes question when there was one); the app shell closes it.
    pub quit_requested: bool,
    /// The window may close without asking again.
    close_confirmed: bool,
    /// What waits for a save to succeed (the unsaved-changes question's Save).
    after_save: Option<unsaved::Then>,
    control_rx: Option<Receiver<ControlRequest>>,
    library_rx: Option<Receiver<Result<Library, String>>>,
    shots: remote::Shots,
    shown_theme: Option<egui::Theme>,
    fonts: bool,
    title: String,
    now: f64,
}

impl App {
    /// A window on `session`.
    pub fn new(session: Session, services: Services) -> Self {
        Self {
            session,
            ui: UiState::default(),
            services,
            dialog: None,
            results: None,
            toast: None,
            canvas: canvas::CanvasState::default(),
            library: LibraryStatus::Builtin,
            synthetic: Vec::new(),
            quit_requested: false,
            close_confirmed: false,
            after_save: None,
            control_rx: None,
            library_rx: None,
            shots: remote::Shots::default(),
            shown_theme: None,
            fonts: false,
            title: String::new(),
            now: 0.0,
        }
    }

    /// Answers control requests arriving on `rx`.
    pub fn with_control(mut self, rx: Receiver<ControlRequest>) -> Self {
        self.control_rx = Some(rx);
        self
    }

    /// Answers control requests arriving on `rx` (for an app already built).
    pub fn set_control(&mut self, rx: Receiver<ControlRequest>) {
        self.control_rx = Some(rx);
    }

    /// Takes the parts library from `rx` once it has loaded (from `dir`).
    pub fn with_library_loading(mut self, dir: String, rx: Receiver<Result<Library, String>>) -> Self {
        self.library = LibraryStatus::Loading(dir);
        self.library_rx = Some(rx);
        self
    }

    /// Whether the system fonts should be looked for (off in tests, for stable layouts).
    pub fn use_system_fonts(&mut self, on: bool) {
        self.fonts = !on;
    }

    /// Per-frame work before drawing: theme, fonts, title, library, control requests, keys.
    pub fn logic(&mut self, ctx: &egui::Context) {
        self.now = ctx.input(|i| i.time);
        let shown = self.ui.theme.theme();
        theme::set_active(shown);
        if self.shown_theme != Some(shown) {
            theme::apply(ctx, shown);
            self.shown_theme = Some(shown);
        }
        if !self.fonts {
            theme::install_fonts(ctx);
            self.fonts = true;
        }
        self.poll_library();
        if ctx.input(|i| i.viewport().close_requested()) {
            self.close_requested(ctx);
        }
        let title = self.window_title();
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        self.drain_control(ctx);
        self.shots.collect(ctx);
        self.shots.issue(ctx);
        if !self.synthetic.is_empty() {
            ctx.request_repaint();
        }
        keys::handle(self, ctx);
        if self.toast.as_ref().is_some_and(|t| t.until < self.now) {
            self.toast = None;
        }
    }

    /// Feeds one injected event per frame to egui (called from the app's raw input hook).
    pub fn raw_input_hook(&mut self, raw: &mut egui::RawInput) {
        remote::inject(&mut self.synthetic, raw);
    }

    /// The window title: `<project> - Protasena - Schematic Capture`.
    pub fn window_title(&self) -> String {
        format!("{} - Protasena - {}", self.session.project().name, tl!("Schematic Capture"))
    }

    /// Lays out the whole window.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        control::begin_frame(ui.ctx());
        menus::menu_bar(self, ui);
        toolbars::show(self, ui);
        toolbars::tab_strip(ui);
        statusbar::show(self, ui);
        modebar::show(self, ui);
        sidepane::show(self, ui);
        canvas::show(self, ui);
        dialogs::show(self, ui.ctx());
        dialogs::results::show(self, ui.ctx());
    }

    fn poll_library(&mut self) {
        let Some(rx) = &self.library_rx else { return };
        let Ok(result) = rx.try_recv() else { return };
        let dir = match &self.library {
            LibraryStatus::Loading(d) => d.clone(),
            _ => String::new(),
        };
        match result {
            Ok(lib) => {
                let devices = lib.len();
                self.session.set_library(lib);
                self.library = LibraryStatus::Loaded { dir, devices };
            }
            Err(e) => {
                self.notify(format!("{}: {e}", tl!("The parts library could not be loaded")), true);
                self.library = LibraryStatus::Failed(e);
            }
        }
        self.library_rx = None;
    }

    fn drain_control(&mut self, ctx: &egui::Context) {
        let Some(rx) = self.control_rx.take() else { return };
        while let Ok(req) = rx.try_recv() {
            if !req.begin() {
                continue;
            }
            match remote::handle(self, ctx, &req) {
                remote::Outcome::Done(v) => {
                    let _ = req.reply.send(v);
                }
                remote::Outcome::Screenshot(path) => self.shots.queue(path, req.reply.clone()),
            }
            ctx.request_repaint();
        }
        self.control_rx = Some(rx);
    }

    /// Shows a message at the bottom of the editing window.
    pub fn notify(&mut self, text: impl Into<String>, error: bool) {
        self.toast = Some(Toast { text: text.into(), error, until: self.now + if error { 6.0 } else { 4.0 } });
    }
}
