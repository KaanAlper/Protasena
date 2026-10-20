//! The control channel: programmatic control of the running window (agents, scripted checks).
//! Requests are JSON lines `{"id", "method", "params"}` read by the app's transport and handed
//! to the window as [`ControlRequest`]s; [`handle`] answers each one on the UI thread.
//! [`METHODS`] lists the methods. Parameters are hostile input: missing or malformed fields give
//! an error reply, never a panic.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use serde_json::{Value, json};

/// A reply: `{"ok": true, "result": …}` or `{"ok": false, "error": …}`.
pub type ControlResponse = Value;

/// One request, waiting for the window.
pub struct ControlRequest {
    /// The method.
    pub method: String,
    /// Its parameters.
    pub params: Value,
    /// Where the reply goes.
    pub reply: Sender<ControlResponse>,
    state: Arc<AtomicU8>,
}

const QUEUED: u8 = 0;
const STARTED: u8 = 1;
const CANCELLED: u8 = 2;

impl ControlRequest {
    /// A request and the transport's handle on its reply.
    pub fn new(method: impl Into<String>, params: Value) -> (Self, PendingReply) {
        let (tx, rx) = std::sync::mpsc::channel();
        let state = Arc::new(AtomicU8::new(QUEUED));
        (Self { method: method.into(), params, reply: tx, state: state.clone() }, PendingReply { rx, state })
    }

    /// Marks the request started; `false` when the transport already gave up on it (the
    /// window must then skip it: the client was told it did not run).
    pub fn begin(&self) -> bool {
        self.state.compare_exchange(QUEUED, STARTED, Ordering::AcqRel, Ordering::Acquire).is_ok()
    }
}

/// The transport's side of a [`ControlRequest`].
pub struct PendingReply {
    rx: Receiver<ControlResponse>,
    state: Arc<AtomicU8>,
}

impl PendingReply {
    /// The reply, or after `timeout` an error whose `state` says whether the request may still
    /// have run (`"not-run"`: safe to send again; `"may-have-run"`: inspect first).
    pub fn wait(self, timeout: Duration) -> ControlResponse {
        if let Ok(r) = self.rx.recv_timeout(timeout) {
            return r;
        }
        if self.state.compare_exchange(QUEUED, CANCELLED, Ordering::AcqRel, Ordering::Acquire).is_ok() {
            return json!({"ok": false, "error": "timeout", "state": "not-run"});
        }
        self.rx.try_recv().unwrap_or_else(|_| json!({"ok": false, "error": "timeout", "state": "may-have-run"}))
    }
}

/// The methods: name, parameters, what it does (`control.methods` replies with this list).
pub const METHODS: &[(&str, &str, &str)] = &[
    ("ping", "", "Replies \"pong\"."),
    ("control.methods", "", "This list."),
    ("engine.execute", "{command, params?}", "Runs a command's JSON form. Never opens a dialog."),
    ("engine.invoke", "{command}", "Runs a command as a menu pick would: it may open its dialog."),
    ("engine.commands", "", "Every command with menu path, shortcut, params doc and enabled state."),
    ("engine.state", "", "The session: project, sheet, item counts, mode, selection, devices, view."),
    ("ui.inspect", "", "Window state: areas and their rectangles, theme, dialog, library, view."),
    ("ui.elements", "{label?, role?}", "Controls drawn in the last frame: label, role, rectangle, enabled, selected."),
    ("ui.click", "{x, y, button?, shift?, ctrl?, double?}", "A real click in screen points."),
    ("ui.move", "{x, y}", "A real pointer move in screen points."),
    ("ui.drag", "{x, y, to: [x, y], button?}", "A real press, move and release in screen points."),
    ("ui.pointer", "{x, y, action?: click|move|double|right, shift?, ctrl?}", "Like ui.click/ui.move at a sheet point (thou)."),
    ("ui.key", "{key, ctrl?, shift?, alt?}", "A key press (egui key names: F6, G, Escape, PageUp…)."),
    ("ui.text", "{text}", "Text typed into the focused field."),
    ("ui.set", "{theme?: light|dark, grid_style?: dots|lines}", "Changes window settings."),
    ("ui.resize", "{width, height}", "Resizes the window."),
    ("ui.screenshot", "{path?}", "Saves a PNG of the window (needs a presented frame)."),
    ("ui.dialog", "", "The open dialog, if any."),
    ("ui.dialog.close", "", "Closes the open dialog like Cancel."),
    ("app.open", "{path}", "Opens a project (.psproj or .pdsprj), discarding unsaved changes."),
    ("app.quit", "", "Closes the window without asking."),
];

/// `control.methods`.
pub fn methods_json() -> Value {
    METHODS.iter().map(|(m, p, d)| json!({"method": m, "params": p, "description": d})).collect()
}

/// A control seen in the last frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Widget {
    /// egui id.
    pub id: egui::Id,
    /// Label (tooltip name for icon buttons).
    pub label: String,
    /// Role: `menu`, `menu-item`, `tool`, `mode`, `tab`, `list-item`, `field`, `button`, `region`.
    pub role: &'static str,
    /// Rectangle in screen points.
    pub rect: egui::Rect,
    /// Enabled.
    pub enabled: bool,
    /// Pressed, checked or chosen, for toggles and list rows.
    pub selected: Option<bool>,
}

fn widgets_id() -> egui::Id {
    egui::Id::new("protasena-observed-widgets")
}

/// Forgets last frame's controls (called at the start of each frame).
pub fn begin_frame(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<Vec<Widget>>(widgets_id()));
}

/// Records a control drawn this frame (observation only: focus and input are untouched).
pub fn record(ui: &egui::Ui, response: &egui::Response, label: &str, role: &'static str, selected: Option<bool>) {
    record_rect(ui.ctx(), response.id, response.rect, label, role, response.enabled(), selected);
}

/// Records an area or control by its rectangle.
pub fn record_rect(ctx: &egui::Context, id: egui::Id, rect: egui::Rect, label: &str, role: &'static str, enabled: bool, selected: Option<bool>) {
    if !rect.is_finite() || !rect.is_positive() {
        return;
    }
    let w = Widget { id, label: label.chars().take(256).collect(), role, rect, enabled, selected };
    ctx.data_mut(|d| {
        let list = d.get_temp_mut_or_default::<Vec<Widget>>(widgets_id());
        if let Some(old) = list.iter_mut().find(|o| o.id == w.id) {
            *old = w;
        } else if list.len() < 4096 {
            list.push(w);
        }
    });
}

/// The controls recorded in the last complete frame.
pub fn widgets(ctx: &egui::Context) -> Vec<Widget> {
    ctx.data_mut(|d| d.get_temp::<Vec<Widget>>(widgets_id()).unwrap_or_default())
}

/// `ui.elements`: recorded controls, filtered by `label` and `role`.
pub fn elements_json(ctx: &egui::Context, params: &Value) -> Value {
    let label = params.get("label").and_then(Value::as_str);
    let role = params.get("role").and_then(Value::as_str);
    widgets(ctx)
        .iter()
        .filter(|w| label.is_none_or(|l| w.label == l) && role.is_none_or(|r| w.role == r))
        .map(|w| {
            json!({
                "label": w.label, "role": w.role, "enabled": w.enabled, "selected": w.selected,
                "rect": [w.rect.left(), w.rect.top(), w.rect.width(), w.rect.height()],
            })
        })
        .collect()
}

/// An egui key from its name (`F6`, `G`, `Escape`, `PageUp`, `Delete`…).
pub fn key_from(name: &str) -> Option<egui::Key> {
    egui::Key::from_name(name).or(match name.to_ascii_lowercase().as_str() {
        "enter" | "return" => Some(egui::Key::Enter),
        "esc" | "escape" => Some(egui::Key::Escape),
        "delete" | "del" => Some(egui::Key::Delete),
        "page-up" | "pageup" => Some(egui::Key::PageUp),
        "page-down" | "pagedown" => Some(egui::Key::PageDown),
        "space" => Some(egui::Key::Space),
        _ => None,
    })
}

/// A PNG of a screenshot at `path`.
pub fn save_png(image: &egui::ColorImage, path: &str) -> Value {
    let [w, h] = image.size;
    let mut buf = Vec::with_capacity(w.saturating_mul(h).saturating_mul(4));
    for c in &image.pixels {
        buf.extend_from_slice(&c.to_array());
    }
    let (Ok(wu), Ok(hu)) = (u32::try_from(w), u32::try_from(h)) else { return json!({"ok": false, "error": "image too large"}) };
    let Some(img) = image::RgbaImage::from_raw(wu, hu, buf) else { return json!({"ok": false, "error": "bad image"}) };
    match img.save(path) {
        Ok(()) => json!({"ok": true, "result": {"path": path, "width": w, "height": h}}),
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_that_timed_out_before_it_started_is_skipped() {
        let (req, pending) = ControlRequest::new("ping", json!({}));
        assert_eq!(pending.wait(Duration::from_millis(1)), json!({"ok": false, "error": "timeout", "state": "not-run"}));
        assert!(!req.begin());
    }

    #[test]
    fn a_started_request_may_have_run() {
        let (req, pending) = ControlRequest::new("ping", json!({}));
        assert!(req.begin());
        assert_eq!(pending.wait(Duration::from_millis(1))["state"], "may-have-run");
    }

    #[test]
    fn keys_by_name() {
        assert_eq!(key_from("F6"), Some(egui::Key::F6));
        assert_eq!(key_from("g"), Some(egui::Key::G));
        assert_eq!(key_from("Page-Up"), Some(egui::Key::PageUp));
        assert_eq!(key_from("nope"), None);
    }

    #[test]
    fn method_names_are_unique() {
        let mut names: Vec<_> = METHODS.iter().map(|m| m.0).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), METHODS.len());
    }
}
