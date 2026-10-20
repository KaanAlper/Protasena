//! Tool ▸ Search & Tag and Property Assignment Tool: what they tag and change, undo, refusals.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::*;
use protasena_doc::ItemId;
use protasena_engine::{EngineError, Outcome, Request, Session};
use serde_json::{Value, json};

/// R1 10k, R2 4k7, R3 100R (TOLERANCE 1%), C1 100n.
fn parts() -> Session {
    let mut s = with_res();
    for (x, v) in [(0, "10k"), (1000, "4k7"), (2000, "100R")] {
        exec(&mut s, "place-component", json!({ "at": [x, 0], "value": v }));
    }
    exec(&mut s, "edit-component", json!({ "component": "R3", "properties": { "TOLERANCE": "1%" } }));
    exec(&mut s, "place-component", json!({ "at": [0, 1000], "device": "CAP", "value": "100n" }));
    s
}

fn tagged(s: &Session) -> Vec<String> {
    s.selection()
        .iter()
        .map(|id| match id {
            ItemId::Component(c) => s.project().component(*c).unwrap().reference.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

fn tag(s: &mut Session, p: Value) -> Vec<String> {
    exec(s, "search-tag", p);
    tagged(s)
}

#[test]
fn search_and_tag_matches_by_property_and_operator() {
    let mut s = parts();
    assert_eq!(tag(&mut s, json!({ "property": "REF", "value": "r", "op": "begins" })), ["R1", "R2", "R3"]);
    assert_eq!(tag(&mut s, json!({ "property": "ref", "value": "r", "op": "begins", "case": true })), Vec::<String>::new());
    assert_eq!(tag(&mut s, json!({ "property": "VALUE", "value": "10k" })), ["R1"]);
    assert_eq!(tag(&mut s, json!({ "property": "VALUE", "value": "5k", "op": "less" })), ["R2", "R3", "C1"]);
    assert_eq!(tag(&mut s, json!({ "property": "VALUE", "value": "1k", "op": "greater" })), ["R1", "R2"]);
    assert_eq!(tag(&mut s, json!({ "property": "DEVICE", "value": "CAP" })), ["C1"]);
    assert_eq!(tag(&mut s, json!({ "property": "tolerance", "value": "1%" })), ["R3"]);
    assert_eq!(tag(&mut s, json!({ "property": "TOLERANCE", "value": "", "op": "not-equals" })), ["R3"]);
    assert_eq!(tag(&mut s, json!({ "property": "VALUE", "value": "k", "op": "not-contains" })), ["R3", "C1"]);
    assert_eq!(tag(&mut s, json!({ "property": "VALUE", "value": "7", "op": "ends" })), ["R2"]);
}

#[test]
fn searches_start_anew_narrow_or_widen_the_tagged_set() {
    let mut s = parts();
    assert_eq!(tag(&mut s, json!({ "property": "DEVICE", "value": "RES" })), ["R1", "R2", "R3"]);
    assert_eq!(tag(&mut s, json!({ "property": "VALUE", "value": "k", "op": "contains", "mode": "and" })), ["R1", "R2"]);
    assert_eq!(tag(&mut s, json!({ "property": "REF", "value": "C1", "mode": "or" })), ["R1", "R2", "C1"]);
    assert_eq!(tag(&mut s, json!({ "property": "REF", "value": "R1", "mode": "or" })), ["R1", "R2", "C1"], "no duplicates");
    let steps = s.undo_steps().len();
    tag(&mut s, json!({ "value": "nothing" }));
    assert!(s.selection().is_empty());
    assert_eq!(s.undo_steps().len(), steps, "tagging is not a design change");
}

#[test]
fn property_assignment_sets_counts_renames_and_removes_in_one_undo_step() {
    let mut s = parts();
    tag(&mut s, json!({ "property": "DEVICE", "value": "RES" }));
    let before = design(s.project());
    let r = exec(&mut s, "property-assign", json!({ "string": "POWER=0.25W" }));
    assert_eq!(r["changed"], 3);
    let after = design(s.project());
    assert!(s.project().components().filter(|(_, c)| c.device.name == "RES").all(|(_, c)| c.properties.get("power") == Some("0.25W")));
    exec(&mut s, "undo", json!({}));
    assert_eq!(design(s.project()), before, "one undo step");
    exec(&mut s, "redo", json!({}));
    assert_eq!(design(s.project()), after);
    exec(&mut s, "property-assign", json!({ "string": "REF=RX#", "count": 10, "increment": 5 }));
    let refs: Vec<String> = s.project().components().map(|(_, c)| c.reference.clone()).collect();
    assert_eq!(refs, ["RX10", "RX15", "RX20", "C1"]);
    exec(&mut s, "property-assign", json!({ "string": "POWER=WATTS", "action": "rename", "apply_to": "all" }));
    let c = s.project().components().next().unwrap().1;
    assert_eq!((c.properties.get("POWER"), c.properties.get("WATTS")), (None, Some("0.25W")));
    let r = exec(&mut s, "property-assign", json!({ "string": "WATTS", "action": "remove", "apply_to": "sheet" }));
    assert_eq!(r["changed"], 3);
    exec(&mut s, "property-assign", json!({ "string": "VALUE=1k", "apply_to": "sheet" }));
    assert!(s.project().components().all(|(_, c)| c.value == "1k"));
    let steps = s.undo_steps().len();
    let r = exec(&mut s, "property-assign", json!({ "string": "VALUE=1k", "apply_to": "sheet" }));
    assert_eq!((r["changed"].as_u64(), s.undo_steps().len()), (Some(0), steps), "nothing changed, no undo step");
    exec(&mut s, "property-assign", json!({ "string": "PACKAGE", "action": "remove", "apply_to": "all" }));
    assert!(s.project().components().all(|(_, c)| c.package.is_none()));
}

#[test]
fn bad_assignments_are_refused_and_change_nothing() {
    let mut s = parts();
    let before = s.project().clone();
    let r = s.execute("property-assign", &json!({ "string": "X=1" }));
    assert!(matches!(&r, Err(EngineError::Other(m)) if m.contains("tagged")), "{r:?}");
    for p in [
        json!({}),
        json!({ "string": "=1", "apply_to": "all" }),
        json!({ "string": "X", "apply_to": "all" }),
        json!({ "string": "X=1", "action": "remove", "apply_to": "all" }),
        json!({ "string": "X=1", "action": "explode", "apply_to": "all" }),
        json!({ "string": "X=1", "apply_to": "everywhere" }),
        json!({ "string": "DEVICE=CAP", "apply_to": "all" }),
        json!({ "string": "REF=X", "action": "rename", "apply_to": "all" }),
        json!({ "string": "VALUE", "action": "remove", "apply_to": "all" }),
        json!({ "string": "A\u{1}=1", "apply_to": "all" }),
        json!({ "string": format!("{}=1", "N".repeat(300)), "apply_to": "all" }),
        json!({ "string": "X=#", "apply_to": "all", "count": 1e12 }),
        json!({ "string": "X=#", "apply_to": "all", "increment": "one" }),
    ] {
        let r = s.execute("property-assign", &p);
        assert!(matches!(r, Err(EngineError::BadParams { .. })), "{p}: {r:?}");
    }
    for p in [json!({ "property": "" }), json!({ "op": "sideways" }), json!({ "op": "not-less" }), json!({ "mode": "xor" })] {
        assert!(matches!(s.execute("search-tag", &p), Err(EngineError::BadParams { .. })), "{p}");
    }
    assert_eq!(s.project(), &before);
}

#[test]
fn the_tools_have_proteus_s_keys_and_dialogs() {
    let mut s = parts();
    assert_eq!(s.invoke("search-tag").unwrap(), Outcome::Dialog(Request::SearchTag));
    assert_eq!(s.invoke("property-assign").unwrap(), Outcome::Dialog(Request::PropertyAssign));
    assert_eq!(protasena_engine::command_for_key("T").map(|c| c.id), Some("search-tag"));
    assert_eq!(protasena_engine::command_for_key("A").map(|c| c.id), Some("property-assign"));
}
