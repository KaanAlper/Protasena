//! The chrome's painted geometry at display scales 1 and 1.34 and at a large and a compact
//! window: nothing overlaps and everything stays inside the window.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::*;

const SIZES: [(f32, f32); 2] = [(1600.0, 1000.0), (1024.0, 700.0)];
const SCALES: [f32; 2] = [1.0, 1.34];

#[test]
fn controls_of_one_bar_never_overlap() {
    let mut problems = Vec::new();
    for (w, ht) in SIZES {
        for scale in SCALES {
            let h = harness(w, ht, scale);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, ht)).expand(1.0);
            for role in ["menu", "tool", "mode", "status", "tab"] {
                let ws = with_role(&h, role);
                assert!(!ws.is_empty(), "{role} at {w}x{ht}@{scale}");
                for (i, a) in ws.iter().enumerate() {
                    if !screen.contains_rect(a.rect) {
                        problems.push(format!("{w}x{ht}@{scale}: {role} {:?} {:?} leaves the window", a.label, a.rect));
                    }
                    for b in ws.iter().skip(i + 1) {
                        if overlap(a.rect, b.rect) {
                            problems.push(format!("{w}x{ht}@{scale}: {role} {:?} {:?} overlaps {:?} {:?}", a.label, a.rect, b.label, b.rect));
                        }
                    }
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn areas_never_overlap() {
    let names = ["Menu bar", "Toolbars", "Tabs", "Editing window", "Status bar", "Overview"];
    for (w, ht) in SIZES {
        for scale in SCALES {
            let h = harness(w, ht, scale);
            let rects: Vec<(&str, egui::Rect)> = names.iter().map(|n| (*n, region(&h, n))).collect();
            for (i, (a, ra)) in rects.iter().enumerate() {
                for (b, rb) in rects.iter().skip(i + 1) {
                    assert!(!overlap(*ra, *rb), "{w}x{ht}@{scale}: {a} {ra:?} overlaps {b} {rb:?}");
                }
            }
            let modes = region(&h, "Mode toolbar");
            assert!(modes.right() <= region(&h, "Overview").left(), "{w}x{ht}@{scale}");
            assert!(region(&h, "Editing window").width() > w * 0.5, "the editing window keeps most of the width");
        }
    }
}
