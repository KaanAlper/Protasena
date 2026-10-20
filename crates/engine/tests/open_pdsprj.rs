//! Opening a Proteus `.pdsprj` project (a synthetic file in the observed structure, our own
//! content): File ▸ Open Project imports it, and Save never writes over it.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;
#[path = "../../pdsprj/tests/common/mod.rs"]
mod pds;

use common::scratch;
use protasena_engine::Session;
use serde_json::json;

#[test]
fn open_imports_a_pdsprj_and_save_asks_for_a_new_file() {
    let dir = scratch("open-pdsprj");
    let path = dir.join("demo.pdsprj");
    let bytes = pds::archive(&pds::dsn::sample_design());
    std::fs::write(&path, &bytes).unwrap();
    let mut s = Session::new();
    let r = s.execute("open", &json!({ "path": path.display().to_string() })).unwrap();
    assert_eq!(r["imported"], json!(true));
    let sheet = s.sheet().unwrap();
    assert_eq!((sheet.components.len(), sheet.wires.len()), (2, 3));
    assert!(s.path().is_none(), "the Proteus file is never the save target");
    assert!(s.execute("save", &json!({})).is_err(), "save needs a new path");
    assert_eq!(std::fs::read(&path).unwrap(), bytes, "the .pdsprj is left as it was");
    assert!(s.log().iter().any(|l| l.contains("Imported")), "{:?}", s.log());
}

#[test]
fn a_broken_pdsprj_is_refused_and_the_project_kept() {
    let dir = scratch("open-pdsprj-broken");
    let path = dir.join("broken.PDSPRJ");
    std::fs::write(&path, b"PK\x03\x04 not really").unwrap();
    let mut s = Session::new();
    s.execute("junction", &json!({ "at": [0, 0] })).unwrap();
    s.execute("save-as", &json!({ "path": dir.join("kept.psproj").display().to_string() })).unwrap();
    assert!(s.execute("open", &json!({ "path": path.display().to_string() })).is_err());
    assert_eq!(s.sheet().unwrap().junctions.len(), 1);
}
