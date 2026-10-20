//! View menu (redraw, grid, false origin, X-cursor, snap, zoom and pan), Tool ▸ Wire
//! Autorouter and the Mode toolbar. View state is kept for the UI and is not undoable.

use protasena_geom::{Pt, Rect};
use serde_json::{Value, json};

use crate::args::Args;
use crate::error::Result;
use crate::interact::Machine;
use crate::mode::Mode;
use crate::registry::CommandSpec;
use crate::session::Session;
use crate::view::{Snap, ZOOM_STEP};

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("redraw", "Redraw Display", redraw).menu(&["View", "Redraw Display"]).key("R").params("{}"),
        CommandSpec::new("toggle-grid", "Toggle Grid", toggle_grid).menu(&["View", "Toggle Grid"]).key("G").params("{on?: bool}"),
        CommandSpec::new("toggle-false-origin", "Toggle False Origin", toggle_false_origin)
            .menu(&["View", "Toggle False Origin"])
            .key("O")
            .params("{at?: [x, y]} — sets the false origin at `at` (default: the cursor), or clears it"),
        CommandSpec::new("toggle-x-cursor", "Toggle X-Cursor", toggle_x_cursor).menu(&["View", "Toggle X-Cursor"]).key("X").params("{on?: bool}"),
        CommandSpec::new("snap-10th", "Snap 10th", |s, _| set_snap(s, Snap::Th10)).menu(&["View", "Snap 10th"]).key("Ctrl+F1").params("{}"),
        CommandSpec::new("snap-50th", "Snap 50th", |s, _| set_snap(s, Snap::Th50)).menu(&["View", "Snap 50th"]).key("F2").params("{}"),
        CommandSpec::new("snap-100th", "Snap 0.1in", |s, _| set_snap(s, Snap::In01)).menu(&["View", "Snap 0.1in"]).key("F3").params("{}"),
        CommandSpec::new("snap-500th", "Snap 0.5in", |s, _| set_snap(s, Snap::In05)).menu(&["View", "Snap 0.5in"]).key("F4").params("{}"),
        CommandSpec::new("center-at-cursor", "Center At Cursor", center).menu(&["View", "Center At Cursor"]).key("F5").params("{at?: [x, y]}"),
        CommandSpec::new("zoom-in", "Zoom In", |s, a| zoom(s, a, ZOOM_STEP)).menu(&["View", "Zoom In"]).key("F6").params("{at?: [x, y]}"),
        CommandSpec::new("zoom-out", "Zoom Out", |s, a| zoom(s, a, 1.0 / ZOOM_STEP)).menu(&["View", "Zoom Out"]).key("F7").params("{at?: [x, y]}"),
        CommandSpec::new("zoom-all", "Zoom To View Entire Sheet", zoom_all).menu(&["View", "Zoom To View Entire Sheet"]).key("F8").params("{}"),
        CommandSpec::new("zoom-area", "Zoom To Area", zoom_area).menu(&["View", "Zoom To Area"]).params("{from: [x, y], to: [x, y]}"),
        CommandSpec::new("pan", "Pan", pan).params("{by?: [dx, dy], center?: [x, y]}"),
        CommandSpec::new("set-viewport", "Viewport", set_viewport).params("{width: px, height: px} — the editing window's size"),
        CommandSpec::new("toggle-autorouter", "Wire Autorouter", toggle_autorouter).menu(&["Tool", "Wire Autorouter"]).key("W").params("{on?: bool}"),
        CommandSpec::new("set-mode", "Mode", set_mode).params("{mode: \"selection\"|\"component\"|…} — the Mode toolbar"),
    ]
}

fn view_json(s: &Session) -> Result<Value> {
    Ok(serde_json::to_value(&s.view).unwrap_or(Value::Null))
}

fn redraw(s: &mut Session, _: &Args) -> Result<Value> {
    s.view.redraws = s.view.redraws.wrapping_add(1);
    s.touch();
    view_json(s)
}

fn toggle_grid(s: &mut Session, a: &Args) -> Result<Value> {
    s.view.grid = a.bool("on")?.unwrap_or(!s.view.grid);
    s.touch();
    view_json(s)
}

fn toggle_false_origin(s: &mut Session, a: &Args) -> Result<Value> {
    s.view.false_origin = match (a.point("at")?, s.view.false_origin) {
        (Some(p), _) => Some(s.snap(p)),
        (None, Some(_)) => None,
        (None, None) => Some(s.cursor),
    };
    s.touch();
    view_json(s)
}

fn toggle_x_cursor(s: &mut Session, a: &Args) -> Result<Value> {
    s.view.x_cursor = a.bool("on")?.unwrap_or(!s.view.x_cursor);
    s.touch();
    view_json(s)
}

fn set_snap(s: &mut Session, snap: Snap) -> Result<Value> {
    s.view.snap = snap;
    s.cursor = s.snap(s.cursor);
    s.touch();
    view_json(s)
}

fn center(s: &mut Session, a: &Args) -> Result<Value> {
    s.view.center = a.point("at")?.unwrap_or(s.cursor);
    s.touch();
    view_json(s)
}

fn zoom(s: &mut Session, a: &Args, factor: f64) -> Result<Value> {
    if let Some(p) = a.point("at")? {
        s.view.center = p;
    }
    s.view.set_zoom(s.view.zoom * factor);
    s.touch();
    view_json(s)
}

/// The sheet's rectangle: centred on the origin.
pub(crate) fn sheet_rect(s: &Session) -> Rect {
    let (w, h) = s.sheet().map_or((11_693.0, 8_268.0), |sh| sh.size.dimensions());
    Rect::new(Pt::new(-w / 2.0, -h / 2.0), Pt::new(w / 2.0, h / 2.0))
}

fn zoom_all(s: &mut Session, _: &Args) -> Result<Value> {
    let r = sheet_rect(s);
    s.view.fit(r);
    s.touch();
    view_json(s)
}

fn zoom_area(s: &mut Session, a: &Args) -> Result<Value> {
    let r = Rect::new(a.point_req("from")?, a.point_req("to")?);
    s.view.fit(r);
    s.touch();
    view_json(s)
}

fn pan(s: &mut Session, a: &Args) -> Result<Value> {
    if let Some(c) = a.point("center")? {
        s.view.center = c;
    }
    if let Some(by) = a.vec("by")? {
        s.view.center += by;
    }
    s.touch();
    view_json(s)
}

fn set_viewport(s: &mut Session, a: &Args) -> Result<Value> {
    let size = |key: &str| -> Result<f64> {
        let v = a.f64(key)?.ok_or_else(|| a.bad(format!("`{key}` (pixels) is required")))?;
        if v >= 1.0 { Ok(v) } else { Err(a.bad(format!("`{key}` must be at least 1"))) }
    };
    s.view.viewport = (size("width")?, size("height")?);
    s.touch();
    view_json(s)
}

fn toggle_autorouter(s: &mut Session, a: &Args) -> Result<Value> {
    s.autorouter = a.bool("on")?.unwrap_or(!s.autorouter);
    s.touch();
    Ok(json!({ "autorouter": s.autorouter }))
}

fn set_mode(s: &mut Session, a: &Args) -> Result<Value> {
    let id = a.str_req("mode")?;
    let mode = Mode::from_id(id).ok_or_else(|| a.bad(format!("unknown mode {id:?}")))?;
    s.mode = mode;
    s.machine = Machine::Idle;
    s.touch();
    Ok(json!({ "mode": mode.id(), "built": mode.is_built() }))
}
