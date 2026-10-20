//! File ▸ Export Graphics and Print Design: the files written, their defaults and refusals.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::*;
use protasena_engine::{EngineError, Outcome, Request, Session};
use serde_json::json;

fn path(dir: &std::path::Path, name: &str) -> String {
    dir.join(name).display().to_string()
}

#[test]
fn each_format_is_written_and_the_design_is_untouched() {
    let dir = scratch("export-formats");
    let mut s = divider();
    let (before, steps, dirty) = (s.project().clone(), s.undo_steps().len(), s.is_dirty());
    for (cmd, file, magic) in [
        ("export-svg", "a.svg", &b"<?xml"[..]),
        ("export-pdf", "a.pdf", b"%PDF-1.4"),
        ("export-eps", "a.eps", b"%!PS-Adobe-3.0 EPSF"),
        ("export-dxf", "a.dxf", b"  0\nSECTION"),
        ("export-bitmap", "a.png", b"\x89PNG"),
    ] {
        let r = exec(&mut s, cmd, json!({ "path": path(&dir, file), "dpi": 50 }));
        assert_eq!(r["pages"], 1, "{cmd}");
        let bytes = std::fs::read(dir.join(file)).unwrap();
        assert!(bytes.starts_with(magic), "{cmd}");
        assert_eq!(r["bytes"], bytes.len(), "{cmd}");
    }
    let r = exec(&mut s, "export-bitmap", json!({ "path": path(&dir, "a.bmp"), "format": "bmp", "dpi": 20 }));
    assert_eq!(r["format"], "bmp");
    assert!(std::fs::read(dir.join("a.bmp")).unwrap().starts_with(b"BM"));
    let svg = std::fs::read_to_string(dir.join("a.svg")).unwrap();
    assert!(svg.contains(">R1</text>") && svg.contains(">R2</text>"));
    assert_eq!((s.project(), s.undo_steps().len(), s.is_dirty()), (&before, steps, dirty), "exporting changes nothing");
}

#[test]
fn pdf_takes_every_sheet_and_paths_default_to_the_project_file() {
    let dir = scratch("export-pdf");
    let mut s = divider();
    exec(&mut s, "new-sheet", json!({}));
    exec(&mut s, "save-as", json!({ "path": path(&dir, "demo.psproj") }));
    let r = exec(&mut s, "export-pdf", json!({ "all_sheets": true }));
    assert_eq!(r["path"], path(&dir, "demo.pdf"));
    assert_eq!(r["pages"], 2);
    let pdf = std::fs::read(dir.join("demo.pdf")).unwrap();
    assert!(String::from_utf8_lossy(&pdf).contains("/Count 2"));
    let r = exec(&mut s, "export-graphics", json!({ "format": "svg", "sheet": { "index": 0 } }));
    assert_eq!(r["path"], path(&dir, "demo.svg"));
    assert!(std::fs::read_to_string(dir.join("demo.svg")).unwrap().contains(">R1</text>"), "the first sheet, not the current one");
}

#[test]
fn print_design_prints_every_sheet_to_pdf_and_hands_back_view() {
    let dir = scratch("print");
    let mut s = divider();
    exec(&mut s, "new-sheet", json!({}));
    let r = exec(&mut s, "print-design", json!({ "path": path(&dir, "p.pdf"), "view": true }));
    assert_eq!((r["pages"].as_u64(), r["view"].as_bool(), r["format"].as_str()), (Some(2), Some(true), Some("pdf")));
    let r = exec(&mut s, "print-design", json!({ "path": path(&dir, "q.pdf"), "sheet": { "index": 1 }, "mono": true }));
    assert_eq!((r["pages"].as_u64(), r["view"].as_bool()), (Some(1), Some(false)));
    assert!(std::fs::read(dir.join("q.pdf")).unwrap().starts_with(b"%PDF"));
}

#[test]
fn menu_picks_ask_for_the_export_dialog() {
    let mut s = Session::new();
    for (cmd, format) in [("export-bitmap", "png"), ("export-dxf", "dxf"), ("export-eps", "eps"), ("export-pdf", "pdf"), ("export-svg", "svg")] {
        assert_eq!(s.invoke(cmd).unwrap(), Outcome::Dialog(Request::ExportGraphics { format }), "{cmd}");
    }
    assert_eq!(s.invoke("print-design").unwrap(), Outcome::Dialog(Request::PrintDesign));
}

#[test]
fn bad_exports_are_refused_and_write_nothing() {
    let dir = scratch("export-refused");
    let mut s = divider();
    let bad = |s: &mut Session, cmd: &str, p: serde_json::Value| {
        let r = s.execute(cmd, &p);
        assert!(matches!(r, Err(EngineError::BadParams { .. })), "{cmd} {p}: {r:?}");
    };
    let f = path(&dir, "x.svg");
    bad(&mut s, "export-svg", json!({}));
    bad(&mut s, "export-graphics", json!({ "path": f }));
    bad(&mut s, "export-graphics", json!({ "path": f, "format": "wmf" }));
    bad(&mut s, "export-dxf", json!({ "path": f, "format": "png" }));
    bad(&mut s, "export-svg", json!({ "path": f, "all_sheets": true }));
    bad(&mut s, "export-bitmap", json!({ "path": f, "dpi": 5 }));
    bad(&mut s, "export-bitmap", json!({ "path": f, "dpi": 1e9 }));
    bad(&mut s, "export-svg", json!({ "path": f, "scale": 0 }));
    bad(&mut s, "export-svg", json!({ "path": f, "sheet": "nope" }));
    bad(&mut s, "export-svg", json!({ "path": "" }));
    assert!(!dir.join("x.svg").exists());
    let r = s.execute("export-bitmap", &json!({ "path": path(&dir, "big.png"), "dpi": 2400 }));
    assert!(matches!(&r, Err(EngineError::Other(m)) if m.contains("pixels")), "{r:?}");
    let r = s.execute("export-svg", &json!({ "path": dir.display().to_string() }));
    assert!(matches!(r, Err(EngineError::Export(_))), "a directory is in the way: {r:?}");
    assert!(!dir.join("big.png").exists());
}
