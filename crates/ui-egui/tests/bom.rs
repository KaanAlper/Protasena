//! The Global Toolbar's Bill of Materials: the report window and its CSV and HTML exports.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::*;
use egui_kittest::kittest::Queryable;
use protasena_ui_egui::dialogs::results::Results;
use protasena_ui_egui::dialogs::{Dialog, PathPurpose};
use serde_json::json;

#[test]
fn the_toolbar_button_shows_the_report_and_exports_it() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    for (x, v) in [(0, "10k"), (1000, "10k"), (2000, "1k")] {
        h.state_mut().run("place-component", json!({ "device": "RES", "at": [x, 0], "value": v })).unwrap();
    }
    h.run_steps(2);
    let b = find(&h, "Bill of Materials", "tool");
    assert!(b.enabled);
    h.get_by_label("Bill of Materials").click();
    h.run_steps(3);
    let Some(Results::Bom(v)) = &h.state().results else { panic!("the report window opens: {:?}", h.state().results) };
    assert_eq!(v["total"], json!(3));
    assert_eq!(v["lines"][0]["references"], json!(["R1", "R2"]));
    h.get_by_label("Export CSV...").click();
    h.run_steps(3);
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("protasena-ui-bom");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("parts.csv");
    let Some(Dialog::Path(d)) = &mut h.state_mut().dialog else { panic!("without a chooser the path is asked for") };
    assert_eq!(d.purpose, PathPurpose::Bom("csv"));
    assert_eq!(d.path, "UNTITLED-bom.csv");
    d.path = path.display().to_string();
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
    assert!(std::fs::read_to_string(&path).unwrap().contains("2,\"R1, R2\",10k,RES"));
    assert!(h.state().toast.as_ref().is_some_and(|t| t.text.contains("parts.csv")));
}
