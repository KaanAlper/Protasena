//! The Bill of Materials command: its JSON form, CSV and HTML text and files.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::*;
use protasena_engine::{Outcome, Request, Session};
use serde_json::json;

fn three() -> Session {
    let mut s = with_res();
    for (x, v) in [(0, "10k"), (1000, "10k"), (2000, "1k")] {
        exec(&mut s, "place-component", json!({ "at": [x, 0], "value": v }));
    }
    exec(&mut s, "place-component", json!({ "device": "GROUND", "at": [0, -1000] }));
    s
}

#[test]
fn the_report_groups_parts_and_writes_csv_and_html() {
    let mut s = three();
    let steps = s.undo_steps().len();
    let v = exec(&mut s, "bill-of-materials", json!({}));
    assert_eq!(v["total"], json!(3));
    assert_eq!(v["lines"][0]["references"], json!(["R1", "R2"]));
    assert_eq!(v["lines"][0]["quantity"], json!(2));
    assert!(v.get("text").is_none());
    let csv = exec(&mut s, "bill-of-materials", json!({ "format": "csv" }));
    assert!(csv["text"].as_str().unwrap().starts_with("Quantity,References,Value,Device,Package,Description\r\n2,\"R1, R2\",10k,RES"));
    let dir = scratch("bom");
    for (file, needle) in [("parts.csv", "1,R3,1k"), ("parts.html", "<td>R3</td>"), ("parts.json", "\"references\"")] {
        let path = dir.join(file);
        let v = exec(&mut s, "bill-of-materials", json!({ "path": path.display().to_string() }));
        assert_eq!(v["path"], json!(path.display().to_string()));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains(needle), "{file}: {text}");
    }
    assert_eq!(s.undo_steps().len(), steps, "the report changes nothing");
    for bad in [
        json!({ "format": "xls" }),
        json!({ "path": "" }),
        json!({ "path": dir.display().to_string() }),
        json!({ "path": dir.join("x.pdf").display().to_string() }),
    ] {
        assert!(s.execute("bill-of-materials", &bad).is_err(), "{bad}");
    }
}

#[test]
fn the_toolbar_asks_for_the_window() {
    let mut s = three();
    assert_eq!(s.invoke("bill-of-materials").unwrap(), Outcome::Dialog(Request::BillOfMaterials));
}
