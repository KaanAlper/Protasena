//! JSON forms of the placing, wiring and design commands: the resulting project state.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::*;
use protasena_doc::{ItemId, TerminalKind};
use protasena_engine::{EngineError, Mode, Session};
use protasena_geom::Orient;
use serde_json::json;

#[test]
fn pick_device_searches_and_picks() {
    let mut s = Session::new();
    let r = exec(&mut s, "pick-device", json!({ "keywords": "capacitor" }));
    let names: Vec<&str> = r["results"].as_array().unwrap().iter().map(|d| d["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"CAP") && names.contains(&"CAP-ELEC"), "{names:?}");
    assert_eq!(r["count"], json!(names.len()));
    let r = exec(&mut s, "pick-device", json!({ "category": "Capacitors", "sub_category": "Electrolytic" }));
    assert_eq!(r["results"][0]["name"], "CAP-ELEC");
    let r = exec(&mut s, "pick-device", json!({ "limit": 1 }));
    assert_eq!(r["results"].as_array().unwrap().len(), 1);
    assert!(r["count"].as_u64().unwrap() > 1);
    exec(&mut s, "pick-device", json!({ "pick": "res" }));
    assert_eq!(s.device().unwrap().name, "RES");
    assert_eq!(s.picked().len(), 1);
    assert_eq!(s.mode(), Mode::Component);
    assert!(matches!(s.execute("pick-device", &json!({ "pick": "NOPE" })), Err(EngineError::BadParams { .. })));
    exec(&mut s, "select-device", json!({ "device": "CAP" }));
    assert_eq!(s.device().unwrap().name, "CAP");
    assert_eq!(s.picked().len(), 2);
}

#[test]
fn place_component_annotates_and_takes_device_defaults() {
    let mut s = with_res();
    let a = exec(&mut s, "place-component", json!({ "at": [0, 0] }));
    let b = exec(&mut s, "place-component", json!({ "at": [1000, 0], "orient": 90 }));
    assert_eq!((a["reference"].as_str(), b["reference"].as_str()), (Some("R1"), Some("R2")));
    let c = &sheet(&s).components[1];
    assert_eq!((c.value.as_str(), c.package.as_deref()), ("10k", Some("RES40")));
    assert_eq!(c.transform.orient, Orient::rotation(1));
    assert_eq!(c.origin(), p(1000.0, 0.0));
    let g = exec(&mut s, "place-component", json!({ "at": [0, -500], "device": "GROUND" }));
    assert_eq!(g["reference"], "");
    let custom = exec(&mut s, "place-component", json!({ "at": [0, 900], "device": "CAP", "reference": "C7", "value": "22n" }));
    assert_eq!(custom["reference"], "C7");
    assert_eq!(sheet(&s).components[3].value, "22n");
    assert!(s.execute("place-component", &json!({ "at": [0, 0], "device": "NOPE" })).is_err());
    assert!(Session::new().execute("place-component", &json!({ "at": [0, 0] })).is_err(), "no device selected");
}

#[test]
fn wire_forms() {
    let mut s = with_res();
    exec(&mut s, "place-component", json!({ "at": [0, 0] }));
    exec(&mut s, "place-component", json!({ "at": [1000, 500] }));
    let routed = exec(&mut s, "wire", json!({ "from": { "component": "R1", "pin": "2" }, "to": { "component": "R2", "pin": "1" } }));
    assert_eq!(routed["points"], json!([[200.0, 0.0], [500.0, 0.0], [500.0, 500.0], [800.0, 500.0]]));
    let straight = exec(&mut s, "wire", json!({ "from": [0, 1000], "to": [300, 1300], "route": false }));
    assert_eq!(straight["points"], json!([[0.0, 1000.0], [300.0, 1300.0]]));
    let exact = exec(&mut s, "wire", json!({ "points": [[0, 2000], [0, 2000], [500, 2000], [500, 2500]] }));
    assert_eq!(exact["points"], json!([[0.0, 2000.0], [500.0, 2000.0], [500.0, 2500.0]]));
    let via = exec(&mut s, "wire", json!({ "from": [0, 3000], "via": [[300, 3300]], "to": [600, 3300], "route": false }));
    assert_eq!(via["points"], json!([[0.0, 3000.0], [300.0, 3300.0], [600.0, 3300.0]]));
    assert_eq!(nets(&s).len(), 1);
    assert!(s.execute("wire", &json!({ "points": [[0, 0]] })).is_err());
    assert!(s.execute("wire", &json!({ "from": { "component": "R1", "pin": "9" }, "to": [0, 0] })).is_err());
    assert!(s.execute("wire", &json!({ "from": { "component": "R9", "pin": "1" }, "to": [0, 0] })).is_err());
}

#[test]
fn wire_ending_on_a_wire_gets_a_junction() {
    let mut s = Session::new();
    exec(&mut s, "wire", json!({ "points": [[0, 0], [1000, 0]] }));
    exec(&mut s, "wire", json!({ "points": [[500, 0], [500, 500]] }));
    assert_eq!(sheet(&s).junctions.len(), 1);
    assert_eq!(sheet(&s).junctions[0].at, p(500.0, 0.0));
    exec(&mut s, "undo", json!({}));
    assert!(sheet(&s).junctions.is_empty(), "the dot is part of the wire's undo step");
}

#[test]
fn junction_label_terminal_bus_text() {
    let mut s = Session::new();
    exec(&mut s, "wire", json!({ "points": [[0, 0], [1000, 0]] }));
    exec(&mut s, "junction", json!({ "at": [300, 0] }));
    let l = exec(&mut s, "wire-label", json!({ "at": [400, 0.5], "text": " CLK " }));
    let label = &sheet(&s).labels[0];
    assert_eq!((label.at, label.text.as_str()), (p(400.0, 0.0), "CLK"));
    assert_eq!(l["id"], json!(label.id.0));
    assert!(s.execute("wire-label", &json!({ "at": [400, 300], "text": "X" })).is_err(), "not on a wire");
    assert!(s.execute("wire-label", &json!({ "at": [400, 0], "text": "  " })).is_err());
    exec(&mut s, "terminal", json!({ "at": [1000, 0], "kind": "power", "name": "VDD" }));
    exec(&mut s, "select-terminal", json!({ "kind": "input" }));
    assert_eq!((s.terminal_kind(), s.mode()), (TerminalKind::Input, Mode::Terminals));
    exec(&mut s, "terminal", json!({ "at": [0, 0] }));
    let kinds: Vec<_> = sheet(&s).terminals.iter().map(|t| t.kind).collect();
    assert_eq!(kinds, [TerminalKind::Power, TerminalKind::Input]);
    assert!(s.execute("terminal", &json!({ "at": [0, 0], "kind": "weird" })).is_err());
    exec(&mut s, "bus", json!({ "points": [[0, 500], [1000, 500]] }));
    assert!(s.execute("bus", &json!({ "points": [[0, 500], [0, 500]] })).is_err());
    exec(&mut s, "text", json!({ "at": [0, 900], "text": "Hello", "size": 120 }));
    assert_eq!(sheet(&s).texts[0].size, 120.0);
    let t = sheet(&s).terminals[1].id.0;
    exec(&mut s, "edit-item", json!({ "id": t, "name": "IN1", "kind": "output" }));
    assert_eq!((sheet(&s).terminals[1].name.as_str(), sheet(&s).terminals[1].kind), ("IN1", TerminalKind::Output));
    let lid = sheet(&s).labels[0].id.0;
    exec(&mut s, "edit-item", json!({ "id": lid, "text": "CLK2" }));
    assert_eq!(sheet(&s).labels[0].text, "CLK2");
    let w = sheet(&s).wires[0].id.0;
    assert!(s.execute("edit-item", &json!({ "id": w })).is_err());
}

#[test]
fn edit_component_sets_fields_and_properties() {
    let mut s = with_res();
    exec(&mut s, "place-component", json!({ "at": [0, 0] }));
    exec(
        &mut s,
        "edit-component",
        json!({ "component": "R1", "reference": "R10", "value": "4k7", "package": null, "properties": { "TOLERANCE": "1%" } }),
    );
    let c = &sheet(&s).components[0];
    assert_eq!((c.reference.as_str(), c.value.as_str(), c.package.as_deref()), ("R10", "4k7", None));
    assert_eq!(c.properties.get("tolerance"), Some("1%"));
    let id = c.id;
    exec(&mut s, "select", json!({ "ids": [id.0] }));
    exec(&mut s, "edit-component", json!({ "properties": { "TOLERANCE": null }, "show_value": false }));
    let c = &sheet(&s).components[0];
    assert!(c.properties.get("TOLERANCE").is_none() && !c.value_label.visible);
    assert!(s.execute("edit-component", &json!({ "component": "R1" })).is_err(), "R1 no longer exists");
    assert!(s.execute("edit-component", &json!({ "component": "R10", "properties": { "A": 5 } })).is_err());
}

#[test]
fn annotate_incremental_and_total() {
    let mut s = with_res();
    for (x, r) in [(0, "R?"), (1000, "R5"), (2000, "R?")] {
        exec(&mut s, "place-component", json!({ "at": [x, 0], "reference": r }));
    }
    exec(&mut s, "annotate", json!({}));
    let refs = |s: &Session| sheet(s).components.iter().map(|c| c.reference.clone()).collect::<Vec<_>>();
    assert_eq!(refs(&s), ["R1", "R5", "R2"]);
    exec(&mut s, "annotate", json!({ "mode": "total" }));
    assert_eq!(refs(&s), ["R1", "R2", "R3"]);
    assert!(s.execute("annotate", &json!({ "mode": "sideways" })).is_err());
}

#[test]
fn erc_reports_unconnected_pins() {
    let mut s = divider();
    let r = exec(&mut s, "erc", json!({}));
    assert_eq!(r["ok"], false);
    let unconnected: Vec<String> =
        r["unconnected"].as_array().unwrap().iter().map(|n| format!("{}.{}", n["reference"].as_str().unwrap(), n["pin"].as_str().unwrap())).collect();
    assert_eq!(unconnected, ["R2.2"]);
    assert!(s.log().last().unwrap().starts_with("Electrical Rules Check"));
}

#[test]
fn design_and_sheets() {
    let mut s = Session::new();
    exec(&mut s, "design-properties", json!({ "title": "Amp", "author": "Me", "name": "amp" }));
    assert_eq!((s.project().design.title.as_str(), s.project().name.as_str()), ("Amp", "amp"));
    exec(&mut s, "sheet-properties", json!({ "name": "Main", "size": "A3" }));
    assert_eq!((sheet(&s).name.as_str(), sheet(&s).size), ("Main", protasena_doc::SheetSize::A3));
    exec(&mut s, "sheet-properties", json!({ "size": { "custom": { "width": 5000, "height": 4000 } } }));
    assert!(s.execute("sheet-properties", &json!({ "size": { "custom": { "width": -1, "height": 4000 } } })).is_err());
    let first = s.sheet_id();
    let n = exec(&mut s, "new-sheet", json!({}));
    assert_eq!(n["name"], "Root sheet 1", "the first free default name");
    assert_eq!(s.project().sheets.len(), 2);
    assert_ne!(s.sheet_id(), first);
    exec(&mut s, "previous-sheet", json!({}));
    assert_eq!(s.sheet_id(), first);
    exec(&mut s, "previous-sheet", json!({}));
    assert_eq!(s.sheet_id(), first, "stays on the first sheet");
    exec(&mut s, "next-sheet", json!({}));
    exec(&mut s, "goto-sheet", json!({ "sheet": "Main" }));
    assert_eq!(s.sheet_id(), first);
    exec(&mut s, "goto-sheet", json!({ "sheet": { "index": 1 } }));
    exec(&mut s, "remove-sheet", json!({}));
    assert_eq!((s.project().sheets.len(), s.sheet_id()), (1, first));
    assert!(s.execute("remove-sheet", &json!({})).is_err(), "the last sheet stays");
    exec(&mut s, "undo", json!({}));
    assert_eq!(s.project().sheets.len(), 2);
}

#[test]
fn delete_keeps_attached_wires() {
    let mut s = divider();
    let r1 = sheet(&s).components[0].id;
    let wires = sheet(&s).wires.len();
    exec(&mut s, "delete", json!({ "ids": [r1.0] }));
    assert_eq!(sheet(&s).components.len(), 1);
    assert_eq!(sheet(&s).wires.len(), wires);
    assert!(s.execute("delete", &json!({ "ids": [] })).is_err());
    assert!(s.execute("delete", &json!({ "ids": [{ "component": r1.0 }] })).is_err(), "already gone");
    let _ = ItemId::Component(r1);
}

#[test]
fn a_new_library_keeps_the_design_and_the_devices_list() {
    let mut s = Session::new();
    exec(&mut s, "pick-device", json!({ "pick": "RES" }));
    exec(&mut s, "place-component", json!({ "at": [0, 0] }));
    let before = s.project().clone();
    let rev = s.revision();
    s.set_library(protasena_library::Library::default());
    assert_eq!(s.project(), &before);
    assert_eq!(s.picked().len(), 1);
    assert!(s.library().is_empty());
    assert!(s.revision() > rev, "the window redraws");
    assert!(s.execute("place-component", &json!({ "at": [500, 0] })).is_err(), "the device is gone with the old library");
}

#[test]
fn reference_and_value_go_where_no_pin_leaves_the_body() {
    let mut s = Session::new();
    let place = |s: &mut Session, device: &str, at: [i32; 2], orient: i32| {
        let r = exec(s, "place-component", json!({ "device": device, "at": at, "orient": orient }));
        let id = protasena_doc::ComponentId(r["id"].as_u64().unwrap());
        s.project().component(id).unwrap().clone()
    };
    let flat = place(&mut s, "RES", [0, 0], 0);
    assert!(flat.reference_label.offset.y > 0.0 && flat.reference_label.offset.x.abs() < 1e-9, "above a horizontal resistor");
    assert!(flat.value_label.offset.y < 0.0, "value below it");
    let upright = place(&mut s, "RES", [2000, 0], 90);
    assert!(
        upright.reference_label.offset.x > 0.0 && upright.value_label.offset.x > 0.0,
        "beside an upright resistor: {:?}",
        upright.reference_label
    );
    assert!(upright.reference_label.offset.y > upright.value_label.offset.y, "reference over value");
    let q = place(&mut s, "NPN", [4000, 0], 0);
    assert!(q.reference_label.offset.x > 0.0, "a transistor's pins leave up, down and left: texts on the right");
    let vcc = place(&mut s, "POWER", [6000, 0], 0);
    assert!(vcc.value_label.offset.y > 0.0, "a supply's net name sits above its bar");
    let gnd = place(&mut s, "GROUND", [8000, 0], 0);
    assert!(gnd.value_label.offset.y < 0.0, "a ground's below");
}
