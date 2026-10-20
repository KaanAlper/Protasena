//! File ▸ Export Graphics from the window: the menu items, their dialogs and
//! the files they write.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use std::path::PathBuf;

use common::*;
use egui_kittest::kittest::Queryable;
use protasena_ui_egui::dialogs::Dialog;
use serde_json::json;

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("protasena-ui").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_export_items_are_enabled_except_the_ones_not_built() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    h.get_by_label("File").click();
    h.run_steps(3);
    assert!(!find(&h, "Print Setup", "menu-item").enabled);
    let sub = find(&h, "Export Graphics", "menu").rect.center();
    h.event(egui::Event::PointerMoved(sub));
    h.run_steps(4);
    for item in ["Export Bitmap", "Export DXF File", "Export Encapsulated Postscript", "Export Adobe PDF File", "Export SVG File"] {
        assert!(find(&h, item, "menu-item").enabled, "{item}");
    }
    assert!(!find(&h, "Export Metafile", "menu-item").enabled);
    assert!(!find(&h, "Export Vector File", "menu-item").enabled);
    h.get_by_label("Export SVG File").click();
    h.run_steps(3);
    assert!(matches!(h.state().dialog, Some(Dialog::Export(_))), "the item opens its dialog");
}

#[test]
fn export_svg_writes_the_sheet_from_its_dialog() {
    let dir = scratch("export-svg");
    let mut h = harness(1600.0, 1000.0, 1.0);
    h.state_mut().run("place-component", json!({ "device": "RES", "at": [0, 0] })).unwrap();
    h.state_mut().invoke("export-svg");
    h.run_steps(2);
    let Some(Dialog::Export(d)) = &mut h.state_mut().dialog else { panic!("Export SVG File's dialog") };
    assert_eq!(d.out.path, "UNTITLED.svg");
    d.out.path = dir.join("t.svg").display().to_string();
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none(), "{:?}", h.state().toast);
    assert!(std::fs::read_to_string(dir.join("t.svg")).unwrap().contains(">R1</text>"));
}

#[test]
fn export_bitmap_offers_png_and_bmp_and_refuses_bad_numbers() {
    let dir = scratch("export-bitmap");
    let mut h = harness(1600.0, 1000.0, 1.0);
    h.state_mut().invoke("export-bitmap");
    h.run_steps(2);
    h.get_by_label("BMP").click();
    h.run_steps(2);
    let Some(Dialog::Export(d)) = &mut h.state_mut().dialog else { panic!("Export Bitmap's dialog") };
    assert_eq!((d.format, d.out.path.as_str()), ("bmp", "UNTITLED.bmp"));
    d.dpi = "many".into();
    d.out.path = dir.join("t.bmp").display().to_string();
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    let Some(Dialog::Export(d)) = &mut h.state_mut().dialog else { panic!("a bad number keeps the dialog open") };
    assert!(d.out.error.is_some());
    d.dpi = "20".into();
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
    assert!(std::fs::read(dir.join("t.bmp")).unwrap().starts_with(b"BM"));
}

#[test]
fn browse_without_a_file_chooser_says_so() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    h.state_mut().invoke("export-pdf");
    h.run_steps(2);
    h.get_by_label("Browse...").click();
    h.run_steps(3);
    let Some(Dialog::Export(d)) = &h.state().dialog else { panic!("still open") };
    assert!(d.out.error.as_deref().is_some_and(|e| e.contains("file chooser")));
}
