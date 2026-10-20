//! The window's side of the control channel: answering requests ([`handle`]), injecting real
//! input events one per frame ([`inject`]) and taking screenshots ([`Shots`]).

use std::sync::mpsc::Sender;

use protasena_geom::Pt;
use serde_json::{Value, json};

use crate::control::{self, ControlRequest, ControlResponse};
use crate::{App, GridStyle, LibraryStatus, UiState};

/// What a request needs from the frame loop.
pub(crate) enum Outcome {
    /// Answered.
    Done(Value),
    /// Take a screenshot to this path.
    Screenshot(String),
}

fn ok(v: Value) -> Outcome {
    Outcome::Done(json!({"ok": true, "result": v}))
}

fn err(e: impl std::fmt::Display) -> Outcome {
    Outcome::Done(json!({"ok": false, "error": e.to_string()}))
}

/// Answers one request.
pub(crate) fn handle(app: &mut App, ctx: &egui::Context, req: &ControlRequest) -> Outcome {
    let p = &req.params;
    let s = |k: &str| p.get(k).and_then(Value::as_str);
    let f = |k: &str| p.get(k).and_then(Value::as_f64).filter(|v| v.is_finite());
    let b = |k: &str| p.get(k).and_then(Value::as_bool).unwrap_or(false);
    match req.method.as_str() {
        "ping" => ok(json!("pong")),
        "control.methods" => ok(control::methods_json()),
        "engine.execute" => {
            let Some(id) = s("command") else { return err("missing `command`") };
            let params = p.get("params").cloned().filter(|v| !v.is_null()).unwrap_or_else(|| json!({}));
            let r = app.run(id, params);
            if r.is_ok() && matches!(id, "open" | "new") {
                let _ = app.run("zoom-all", json!({}));
            }
            match r {
                Ok(v) => ok(v),
                Err(e) => err(e),
            }
        }
        "engine.invoke" => {
            let Some(id) = s("command") else { return err("missing `command`") };
            app.invoke(id);
            ok(json!({ "dialog": app.dialog.as_ref().map(|d| d.title()) }))
        }
        "engine.commands" => ok(serde_json::to_value(app.session.commands()).unwrap_or_default()),
        "engine.state" => ok(state(app)),
        "ui.inspect" => ok(inspect(app, ctx)),
        "ui.elements" => ok(control::elements_json(ctx, p)),
        "ui.move" | "ui.click" => {
            let (Some(x), Some(y)) = (f("x"), f("y")) else { return err("missing x/y") };
            pointer_at(
                app,
                egui::pos2(x as f32, y as f32),
                if req.method == "ui.click" { s("button").or(Some("left")) } else { None },
                b("double"),
                b("shift"),
                b("ctrl"),
            );
            ok(Value::Null)
        }
        "ui.pointer" => {
            let (Some(x), Some(y)) = (f("x"), f("y")) else { return err("missing x/y") };
            let Some(xf) = app.canvas.xform else { return err("the editing window has not been drawn yet") };
            let pos = xf.to_screen(Pt::new(x, y));
            match s("action").unwrap_or("click") {
                "move" => pointer_at(app, pos, None, false, false, false),
                "double" => pointer_at(app, pos, Some("left"), true, false, false),
                "right" => pointer_at(app, pos, Some("right"), false, false, false),
                _ => pointer_at(app, pos, Some("left"), false, b("shift"), b("ctrl")),
            }
            ok(json!({ "screen": [pos.x, pos.y] }))
        }
        "ui.drag" => {
            let (Some(x), Some(y)) = (f("x"), f("y")) else { return err("missing x/y") };
            let to = p.get("to").and_then(Value::as_array);
            let (Some(tx), Some(ty)) = (to.and_then(|a| a.first()).and_then(Value::as_f64), to.and_then(|a| a.get(1)).and_then(Value::as_f64)) else {
                return err("missing to: [x, y]");
            };
            let button = button(s("button").unwrap_or("left"));
            let (from, to) = (egui::pos2(x as f32, y as f32), egui::pos2(tx as f32, ty as f32));
            let m = egui::Modifiers::default();
            app.synthetic.push(egui::Event::PointerMoved(from));
            app.synthetic.push(egui::Event::PointerButton { pos: from, button, pressed: true, modifiers: m });
            for i in 1..=8 {
                app.synthetic.push(egui::Event::PointerMoved(from + (to - from) * (i as f32 / 8.0)));
            }
            app.synthetic.push(egui::Event::PointerButton { pos: to, button, pressed: false, modifiers: m });
            ok(Value::Null)
        }
        "ui.key" => {
            let Some(key) = s("key").and_then(control::key_from) else { return err("unknown key") };
            let modifiers = egui::Modifiers { alt: b("alt"), shift: b("shift"), ctrl: b("ctrl"), command: b("ctrl"), mac_cmd: false };
            app.synthetic.push(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers });
            app.synthetic.push(egui::Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers });
            ok(Value::Null)
        }
        "ui.text" => {
            app.synthetic.push(egui::Event::Text(s("text").unwrap_or("").chars().take(10_000).collect()));
            ok(Value::Null)
        }
        "ui.set" => {
            let mut cur = serde_json::to_value(&app.ui).unwrap_or_else(|_| json!({}));
            if let (Some(o), Some(src)) = (cur.as_object_mut(), p.as_object()) {
                for (k, v) in src {
                    o.insert(k.clone(), v.clone());
                }
            }
            match serde_json::from_value::<UiState>(cur) {
                Ok(u) => {
                    app.ui = u;
                    ok(serde_json::to_value(&app.ui).unwrap_or_default())
                }
                Err(e) => err(e),
            }
        }
        "ui.resize" => {
            let (Some(w), Some(h)) = (f("width"), f("height")) else { return err("missing width/height") };
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w.clamp(320.0, 8192.0) as f32, h.clamp(240.0, 8192.0) as f32)));
            ok(Value::Null)
        }
        "ui.screenshot" => {
            let path = s("path").map_or_else(|| std::env::temp_dir().join("protasena-shot.png").display().to_string(), str::to_owned);
            Outcome::Screenshot(path)
        }
        "ui.dialog" => ok(json!({ "dialog": app.dialog.as_ref().map(|d| d.title()), "results": app.results.as_ref().map(|r| r.title()) })),
        "ui.dialog.close" => {
            app.dialog = None;
            app.results = None;
            ok(Value::Null)
        }
        "app.open" => {
            let Some(path) = s("path") else { return err("missing `path`") };
            match app.run("open", json!({ "path": path, "discard": true })) {
                Ok(v) => {
                    let _ = app.run("zoom-all", json!({}));
                    ok(v)
                }
                Err(e) => err(e),
            }
        }
        "app.quit" => {
            app.quit_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            ok(Value::Null)
        }
        other => err(format!("unknown method `{}`; `control.methods` lists them", other.chars().take(64).collect::<String>())),
    }
}

fn button(name: &str) -> egui::PointerButton {
    match name {
        "right" => egui::PointerButton::Secondary,
        "middle" => egui::PointerButton::Middle,
        _ => egui::PointerButton::Primary,
    }
}

/// Queues a move to `pos` and, with a button, a click (twice for a double click).
fn pointer_at(app: &mut App, pos: egui::Pos2, click: Option<&str>, double: bool, shift: bool, ctrl: bool) {
    app.synthetic.push(egui::Event::PointerMoved(pos));
    let Some(name) = click else { return };
    let button = button(name);
    let modifiers = egui::Modifiers { shift, ctrl, command: ctrl, ..Default::default() };
    for _ in 0..if double { 2 } else { 1 } {
        app.synthetic.push(egui::Event::PointerButton { pos, button, pressed: true, modifiers });
        app.synthetic.push(egui::Event::PointerButton { pos, button, pressed: false, modifiers });
    }
}

/// Feeds the next injected event (with the pointer position it needs) to egui.
pub(crate) fn inject(queue: &mut Vec<egui::Event>, raw: &mut egui::RawInput) {
    let Some(first) = queue.first().cloned() else { return };
    let n = match first {
        egui::Event::PointerMoved(_) | egui::Event::PointerButton { .. } => 1,
        _ => queue.iter().position(|e| matches!(e, egui::Event::Key { pressed: false, .. })).map_or(queue.len(), |i| i + 1),
    };
    match &first {
        egui::Event::PointerButton { pos, modifiers, .. } => {
            raw.events.push(egui::Event::PointerMoved(*pos));
            raw.events.push(egui::Event::ModifiersChanged(*modifiers));
        }
        egui::Event::PointerMoved(_) => raw.events.push(egui::Event::ModifiersChanged(egui::Modifiers::default())),
        _ => {}
    }
    raw.events.extend(queue.drain(..n.min(queue.len())));
}

fn state(app: &App) -> Value {
    let s = &app.session;
    let sheet = s.sheet();
    let count = |f: fn(&protasena_doc::Sheet) -> usize| sheet.map_or(0, f);
    json!({
        "project": s.project().name, "path": s.path().map(|p| p.display().to_string()), "dirty": s.is_dirty(),
        "sheet": sheet.map(|sh| sh.name.clone()), "sheets": s.project().sheets.len(),
        "components": count(|sh| sh.components.len()), "wires": count(|sh| sh.wires.len()),
        "junctions": count(|sh| sh.junctions.len()), "labels": count(|sh| sh.labels.len()),
        "terminals": count(|sh| sh.terminals.len()), "texts": count(|sh| sh.texts.len()), "buses": count(|sh| sh.buses.len()),
        "mode": s.mode(), "selection": s.selection().len(), "device": s.device().map(|d| d.name.clone()),
        "picked": s.picked().iter().map(|d| d.name.clone()).collect::<Vec<_>>(),
        "view": s.view(), "cursor": [s.cursor().x, s.cursor().y], "autorouter": s.autorouter(),
        "library": s.library().len(), "log": s.log().iter().rev().take(10).rev().collect::<Vec<_>>(),
    })
}

fn inspect(app: &App, ctx: &egui::Context) -> Value {
    let regions: Vec<Value> = control::widgets(ctx)
        .iter()
        .filter(|w| w.role == "region")
        .map(|w| json!({ "name": w.label, "rect": [w.rect.left(), w.rect.top(), w.rect.width(), w.rect.height()] }))
        .collect();
    let library = match &app.library {
        LibraryStatus::Builtin => json!({ "state": "builtin" }),
        LibraryStatus::Loading(d) => json!({ "state": "loading", "dir": d }),
        LibraryStatus::Loaded { dir, devices } => json!({ "state": "loaded", "dir": dir, "devices": devices }),
        LibraryStatus::Failed(e) => json!({ "state": "failed", "error": e }),
    };
    let r = ctx.content_rect();
    json!({
        "window": [r.width(), r.height()], "regions": regions, "theme": app.ui.theme,
        "grid_style": matches!(app.ui.grid_style, GridStyle::Lines).then_some("lines").unwrap_or("dots"),
        "dialog": app.dialog.as_ref().map(|d| d.title()), "results": app.results.as_ref().map(|r| r.title()),
        "toast": app.toast.as_ref().map(|t| t.text.clone()), "library": library, "title": app.window_title(),
        "canvas": app.canvas.xform.map(|x| json!({ "rect": [x.rect.left(), x.rect.top(), x.rect.width(), x.rect.height()], "center": [x.center.x, x.center.y], "zoom": x.zoom })),
    })
}

/// Screenshots waiting for a presented frame.
#[derive(Default)]
pub(crate) struct Shots {
    queued: Vec<(u64, u32)>,
    pending: Vec<(u64, String, Sender<ControlResponse>, f64)>,
    next: u64,
}

impl Shots {
    /// Queues a screenshot; it is taken a few frames later so injected input has settled.
    pub(crate) fn queue(&mut self, path: String, reply: Sender<ControlResponse>) {
        self.next = self.next.wrapping_add(1);
        self.queued.push((self.next, 0));
        self.pending.push((self.next, path, reply, 0.0));
    }

    /// Asks egui for the queued screenshots once three frames have passed.
    pub(crate) fn issue(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        for p in &mut self.pending {
            if p.3 == 0.0 {
                p.3 = now + 8.0;
            }
        }
        self.queued.retain_mut(|(token, frames)| {
            *frames += 1;
            if *frames >= 3 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(*token)));
                false
            } else {
                true
            }
        });
        if !self.queued.is_empty() || !self.pending.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }

    /// Saves the screenshots egui delivered.
    pub(crate) fn collect(&mut self, ctx: &egui::Context) {
        if self.pending.is_empty() {
            return;
        }
        let shots: Vec<(u64, std::sync::Arc<egui::ColorImage>)> = ctx.input(|i| {
            i.raw
                .events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Screenshot { user_data, image, .. } => {
                        Some((user_data.data.as_ref()?.downcast_ref::<u64>().copied()?, image.clone()))
                    }
                    _ => None,
                })
                .collect()
        });
        for (token, image) in shots {
            if let Some(i) = self.pending.iter().position(|p| p.0 == token) {
                let (_, path, reply, _) = self.pending.remove(i);
                let _ = reply.send(control::save_png(&image, &path));
            }
        }
        let now = ctx.input(|i| i.time);
        self.pending.retain(|(_, _, reply, deadline)| {
            if *deadline == 0.0 || now < *deadline {
                return true;
            }
            let _ = reply.send(json!({"ok": false, "error": "no frame was presented (window hidden or minimised?)"}));
            false
        });
    }
}
