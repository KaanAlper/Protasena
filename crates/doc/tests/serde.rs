//! `serde` support (feature `serde`): projects and ops survive a JSON round trip; hostile JSON is
//! an error, never a panic.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
#![cfg(feature = "serde")]

mod common;

use common::*;
use protasena_doc::*;
use protasena_geom::Vec2;

#[test]
fn projects_and_ops_round_trip_through_json() {
    let mut p = Project::new("t");
    part(&mut p, "RES", "R1", (100.0, 200.0));
    wire(&mut p, &[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0)]);
    let l = label(&mut p, (50.0, 0.0), "SDA");
    add(&mut p, Item::Label(l));
    let json = serde_json::to_string(&p).unwrap();
    assert_eq!(serde_json::from_str::<Project>(&json).unwrap(), p);
    let op = Op::batch(vec![
        Op::Move { ids: p.sheets[0].item_ids().collect(), by: Vec2::new(100.0, 0.0) },
        Op::SetSheet { id: first_sheet(&p), name: "A".into(), size: SheetSize::Custom { width: 1.0, height: 2.0 } },
    ]);
    let json = serde_json::to_value(&op).unwrap();
    assert_eq!(json["op"], "batch");
    assert_eq!(json["ops"][0]["ids"][0], serde_json::json!({"component": 2}));
    assert_eq!(serde_json::from_value::<Op>(json).unwrap(), op);
}

#[test]
fn hostile_json_never_panics() {
    let cases = [
        "",
        "null",
        "{}",
        r#"{"sheets":[{"wires":[{"id":1,"points":[[0,0]]}]}]}"#,
        r#"{"sheets":[{"components":[{"id":1,"device":{"name":"R"},"transform":{"at":[1e400,0]}}]}]}"#,
        r#"{"op":"move","ids":[{"wire":-1}],"by":[0,0]}"#,
        r#"{"op":"add","sheet":1,"item":{"text":{"id":1,"at":[0,0],"text":"x","orient":{"angle":-2147483648}}}}"#,
        r#"{"op":"batch","ops":[{"op":"batch","ops":[{"op":"nope"}]}]}"#,
    ];
    for json in cases {
        let _ = serde_json::from_str::<Project>(json);
        let _ = serde_json::from_str::<Op>(json);
    }
    // Structurally valid but inconsistent data is caught by `issues`.
    let p: Project = serde_json::from_str(r#"{"sheets":[{"id":1,"wires":[{"id":1,"points":[[0,0]]}]}],"next_id":0}"#).unwrap();
    assert!(p.issues().len() >= 2);
}
