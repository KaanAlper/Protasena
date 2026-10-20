//! Editing-window input: pointer events to the engine's active mode, middle-button pan, wheel
//! zoom about the cursor, and View ▸ Zoom To Area's drag.

use egui::{PointerButton, Rect, Response, Ui};
use protasena_engine::{Button, Modifiers, PointerEvent, PointerKind};
use protasena_geom::Pt;
use serde_json::json;

use super::Xform;
use crate::App;

/// Wheel travel (points) that makes one zoom step.
const WHEEL_STEP: f32 = 40.0;
/// The engine's zoom limits (points per thou): no step is taken beyond them.
const ZOOM_LIMITS: (f64, f64) = (1e-4, 10.0);

/// Feeds this frame's pointer input in `rect` to the engine.
pub(super) fn handle(app: &mut App, ui: &Ui, resp: &Response, rect: Rect) {
    let view = app.session.view();
    let x = Xform { rect, center: view.center, zoom: view.zoom };
    let (pos, pressed, released, double, right, middle_down, delta, wheel, mods) = ui.input(|i| {
        let m = i.modifiers;
        (
            i.pointer.latest_pos(),
            i.pointer.button_pressed(PointerButton::Primary),
            i.pointer.button_released(PointerButton::Primary),
            i.pointer.button_double_clicked(PointerButton::Primary),
            i.pointer.button_pressed(PointerButton::Secondary),
            i.pointer.button_down(PointerButton::Middle),
            i.pointer.delta(),
            i.smooth_scroll_delta.y,
            Modifiers { shift: m.shift, ctrl: m.ctrl || m.command, alt: m.alt },
        )
    });
    let over = resp.hovered() || resp.contains_pointer();
    let inside = pos.is_some_and(|p| rect.contains(p)) && (over || app.canvas.pressed);
    if let Some(p) = pos.filter(|_| inside)
        && app.canvas.last_pointer != Some(p)
    {
        send(app, PointerEvent::moved(x.to_sheet(p)), false);
    }
    app.canvas.last_pointer = pos.filter(|p| rect.contains(*p));
    let Some(p) = pos else { return };
    let at = x.to_sheet(p);
    if middle_down && inside && delta != egui::Vec2::ZERO {
        let by = [-f64::from(delta.x) / x.zoom.max(1e-9), f64::from(delta.y) / x.zoom.max(1e-9)];
        let _ = app.run("pan", json!({ "by": by }));
    }
    if inside && wheel != 0.0 {
        wheel_zoom(app, at, wheel);
    }
    if pressed && inside && over {
        app.canvas.pressed = true;
        if let Some(None) = app.canvas.zoom_area {
            app.canvas.zoom_area = Some(Some(at));
            return;
        }
        send(app, PointerEvent { kind: PointerKind::Press, at, button: Button::Left, modifiers: mods }, true);
    }
    if released && app.canvas.pressed {
        app.canvas.pressed = false;
        if let Some(Some(from)) = app.canvas.zoom_area {
            app.canvas.zoom_area = None;
            finish_zoom_area(app, from, at);
            return;
        }
        send(app, PointerEvent { kind: PointerKind::Release, at, button: Button::Left, modifiers: mods }, true);
    }
    if double && inside {
        send(app, PointerEvent { kind: PointerKind::DoubleClick, at, button: Button::Left, modifiers: mods }, true);
    }
    if right && inside && over {
        app.canvas.zoom_area = None;
        send(app, PointerEvent { kind: PointerKind::Press, at, button: Button::Right, modifiers: mods }, true);
    }
}

/// Sends a pointer event; `report` shows its messages and dialogs (moves stay quiet).
fn send(app: &mut App, ev: PointerEvent, report: bool) {
    match app.session.pointer(ev) {
        Ok(fb) if report => app.feedback(fb),
        Ok(fb) => {
            if let Some(r) = fb.request {
                app.request(r);
            }
        }
        Err(e) if report => app.notify(e.to_string(), true),
        Err(_) => {}
    }
}

/// One zoom step per [`WHEEL_STEP`] of travel, keeping the sheet point under the cursor still.
fn wheel_zoom(app: &mut App, at: Pt, wheel: f32) {
    app.canvas.wheel += wheel;
    while app.canvas.wheel.abs() >= WHEEL_STEP {
        let zoom_in = app.canvas.wheel > 0.0;
        app.canvas.wheel -= WHEEL_STEP.copysign(app.canvas.wheel);
        let view = app.session.view();
        let step = app.session.settings().display.wheel_zoom;
        let factor = if zoom_in { step } else { 1.0 / step };
        if (zoom_in && view.zoom >= ZOOM_LIMITS.1) || (!zoom_in && view.zoom <= ZOOM_LIMITS.0) {
            app.canvas.wheel = 0.0;
            return;
        }
        let c = view.center;
        let center = Pt::new(at.x - (at.x - c.x) / factor, at.y - (at.y - c.y) / factor);
        let _ = app.run(if zoom_in { "zoom-in" } else { "zoom-out" }, json!({ "at": [center.x, center.y], "factor": step }));
    }
}

fn finish_zoom_area(app: &mut App, from: Pt, to: Pt) {
    if (from.x - to.x).abs() < 1.0 || (from.y - to.y).abs() < 1.0 {
        return;
    }
    if let Err(e) = app.run("zoom-area", json!({ "from": [from.x, from.y], "to": [to.x, to.y] })) {
        app.notify(e, true);
    }
}
