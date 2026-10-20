//! Library ▸ Pick Parts and the object selector's DEVICES list; placing components (Component
//! mode, with real-time annotation).

use protasena_doc::{Component, ComponentId, DeviceRef, FieldLabel, Item, Op, Properties};
use protasena_geom::{Pt, Transform, Vec2};
use protasena_library::{Device, Query};
use protasena_netlist::DeviceSource;
use serde_json::{Value, json};

use crate::args::Args;
use crate::bodies::body_bounds;
use crate::error::{EngineError, Result};
use crate::mode::Mode;
use crate::registry::CommandSpec;
use crate::request::Request;
use crate::session::Session;

/// Results returned by default, and at most.
const DEFAULT_RESULTS: u64 = 100;
const MAX_RESULTS: u64 = 10_000;

/// Gap between a part's body and its reference/value texts, in thou.
const LABEL_GAP: f64 = 50.0;

/// Half the gap between a reference and a value stacked beside a body, in thou.
const SIDE_SPLIT: f64 = 10.0;

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("pick-device", "Pick Parts", pick_device)
            .menu(&["Library", "Pick Parts"])
            .key("P")
            .params("{keywords?, category?, sub_category?, manufacturer?: string, limit?: n, pick?: device name, library?} — search, or add `pick` to the DEVICES list")
            .dialog(|_| Some(Request::PickDevices)),
        CommandSpec::new("select-device", "Select Device", select_device)
            .params("{device: name, library?: string} — chooses the device to place (adds it to the DEVICES list)")
            .machine(Mode::Component),
        CommandSpec::new("place-component", "Place Component", place_component)
            .params("{at: [x, y], device?: name (default: the selected device), library?, orient?: angle|{angle, mirror}, reference?, value?, unit?: n}")
            .machine(Mode::Component),
    ]
}

/// The device named by `device`/`library`, or the selected one.
fn device_ref(s: &Session, a: &Args) -> Result<DeviceRef> {
    match a.str("device")? {
        Some(name) => Ok(DeviceRef::new(a.str("library")?.unwrap_or(""), name)),
        None => s.device.clone().ok_or_else(|| EngineError::Other("no device selected: pick one first (P)".into())),
    }
}

fn lookup<'a>(s: &'a Session, a: &Args, r: &DeviceRef) -> Result<&'a Device> {
    DeviceSource::device(&s.library, r).ok_or_else(|| a.bad(format!("the library has no device {:?}", r.name)))
}

/// Puts a device in the DEVICES list and makes it the one to place.
fn choose(s: &mut Session, d: &Device) -> DeviceRef {
    let r = DeviceRef::new(&d.library, &d.name);
    if !s.picked.contains(&r) {
        s.picked.push(r.clone());
    }
    s.device = Some(r.clone());
    s.mode = Mode::Component;
    s.machine = crate::interact::Machine::Idle;
    s.touch();
    r
}

fn device_json(d: &Device) -> Value {
    json!({
        "name": d.name, "library": d.library, "description": d.description, "category": d.category,
        "sub_category": d.sub_category, "manufacturer": d.manufacturer,
    })
}

fn pick_device(s: &mut Session, a: &Args) -> Result<Value> {
    if let Some(name) = a.str("pick")? {
        let r = DeviceRef::new(a.str("library")?.unwrap_or(""), name);
        let d = lookup(s, a, &r)?.clone();
        let chosen = choose(s, &d);
        return Ok(json!({ "picked": chosen.name, "library": chosen.library }));
    }
    let mut q = Query::keywords(a.str("keywords")?.unwrap_or(""));
    if let Some(c) = a.str("category")? {
        q = q.category(c);
    }
    if let Some(c) = a.str("sub_category")? {
        q = q.sub_category(c);
    }
    if let Some(m) = a.str("manufacturer")? {
        q = q.manufacturer(m);
    }
    let limit = usize::try_from(a.u64("limit")?.unwrap_or(DEFAULT_RESULTS).min(MAX_RESULTS)).unwrap_or(usize::MAX);
    let hits = s.library.search(&q);
    let results: Vec<Value> = hits.iter().take(limit).map(|d| device_json(d)).collect();
    Ok(json!({ "count": hits.len(), "results": results }))
}

fn select_device(s: &mut Session, a: &Args) -> Result<Value> {
    let name = a.str_req("device")?;
    let r = DeviceRef::new(a.str("library")?.unwrap_or(""), name);
    let d = lookup(s, a, &r)?.clone();
    let chosen = choose(s, &d);
    Ok(json!({ "device": chosen.name, "library": chosen.library }))
}

/// A new component of `d` at `at` with Proteus's defaults: VALUE and PACKAGE from the device,
/// the other default properties, and the next free reference of its prefix.
pub(crate) fn new_component(s: &Session, d: &Device, id: ComponentId, at: Pt, transform: Transform) -> Component {
    let defaults = d.default_properties();
    let get = |name: &str| defaults.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.clone());
    let value = get("VALUE").unwrap_or_else(|| d.name.clone());
    let package = get("PACKAGE").or_else(|| d.default_package.clone());
    let others = defaults.iter().filter(|(k, _)| !k.eq_ignore_ascii_case("VALUE") && !k.eq_ignore_ascii_case("PACKAGE")).cloned();
    let reference = if d.prefix.is_empty() { String::new() } else { s.project.next_reference(&d.prefix) };
    let mut c = Component::new(id, DeviceRef::new(&d.library, &d.name), &reference, &value).placed(transform);
    c.package = package;
    c.properties = Properties::from_pairs(others);
    let (reference_at, value_at) = label_anchors(s, d, &c);
    let origin = at.to_vec();
    c.reference_label = FieldLabel { offset: reference_at.to_vec() - origin, visible: !reference.is_empty() };
    c.value_label = FieldLabel { offset: value_at.to_vec() - origin, visible: true };
    c
}

/// Where a new part's reference and value texts go (sheet points): above and below the body
/// when no pin leaves it upwards, else beside it on the right (reference over value) when no
/// pin leaves to the right; a power symbol's net name sits on the far side from its pin.
/// Proteus's own text placement is still to compare.
fn label_anchors(s: &Session, d: &Device, c: &Component) -> (Pt, Pt) {
    let body = body_bounds(&s.library, c);
    let mid = body.center();
    let above = (Pt::new(mid.x, body.max.y + LABEL_GAP), Pt::new(mid.x, body.min.y - LABEL_GAP));
    let pins = crate::bodies::visible_pins(&s.library, c);
    let leaves = |v: Vec2| pins.iter().any(|p| crate::bodies::outward(p).dot(v) > 0.5);
    if d.symbol.power {
        let pin_below = pins.iter().all(|p| p.at.y <= mid.y);
        let net = if pin_below { above.0 } else { above.1 };
        return (above.0, net);
    }
    if !leaves(Vec2::new(0.0, 1.0)) {
        return above;
    }
    if !leaves(Vec2::new(1.0, 0.0)) {
        let x = body.max.x + LABEL_GAP;
        return (Pt::new(x, mid.y + SIDE_SPLIT), Pt::new(x, mid.y - SIDE_SPLIT));
    }
    above
}

fn place_component(s: &mut Session, a: &Args) -> Result<Value> {
    let at = a.point_req("at")?;
    let r = device_ref(s, a)?;
    let d = lookup(s, a, &r)?.clone();
    let orient = a.orient("orient")?.unwrap_or(s.placement);
    let id: ComponentId = s.fresh()?;
    let mut c = new_component(s, &d, id, at, Transform::new(orient, at.to_vec()));
    if let Some(reference) = a.str("reference")? {
        c.reference = reference.to_owned();
    }
    if let Some(value) = a.str("value")? {
        c.value = value.to_owned();
    }
    if let Some(unit) = a.u64("unit")? {
        c.unit = u32::try_from(unit).map_err(|_| a.bad("`unit` is too large"))?;
    }
    let reference = c.reference.clone();
    let sheet = s.sheet;
    s.commit("place-component", vec![Op::Add { sheet, item: Item::Component(c), index: None }])?;
    let r = DeviceRef::new(&d.library, &d.name);
    if !s.picked.contains(&r) {
        s.picked.push(r);
    }
    Ok(json!({ "id": id.0, "reference": reference }))
}
