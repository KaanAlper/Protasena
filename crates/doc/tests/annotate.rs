//! Annotation: next free references, annotating new parts, renumbering everything.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::*;
use protasena_doc::*;

fn refs(p: &Project) -> Vec<String> {
    p.components().map(|(_, c)| c.reference.clone()).collect()
}

#[test]
fn next_reference_fills_the_lowest_gap() {
    let mut p = Project::new("t");
    assert_eq!(p.next_reference("R"), "R1");
    part(&mut p, "RES", "R1", (0.0, 0.0));
    part(&mut p, "RES", "R3", (0.0, 0.0));
    part(&mut p, "CAP", "C1", (0.0, 0.0));
    assert_eq!(p.next_reference("R"), "R2");
    assert_eq!(p.next_reference("C"), "C2");
    assert_eq!(p.next_reference("U"), "U1");
}

#[test]
fn annotate_numbers_only_unannotated_parts() {
    let mut p = Project::new("t");
    part(&mut p, "RES", "R?", (0.0, 0.0));
    part(&mut p, "RES", "R2", (0.0, 0.0));
    part(&mut p, "RES", "R?", (0.0, 0.0));
    part(&mut p, "CAP", "C", (0.0, 0.0));
    part(&mut p, "GROUND", "", (0.0, 0.0));
    let before = p.clone();
    let undo = p.apply(p.annotate()).unwrap();
    assert_eq!(refs(&p), ["R1", "R2", "R3", "C1", ""]);
    assert!(p.annotate().is_empty());
    p.apply(undo).unwrap();
    assert_eq!(p, before);
}

#[test]
fn reannotate_all_runs_top_to_bottom_then_left_to_right() {
    let mut p = Project::new("t");
    let low_left = part(&mut p, "RES", "R7", (0.0, -1000.0));
    let top_right = part(&mut p, "RES", "R?", (2000.0, 1000.0));
    let top_left = part(&mut p, "RES", "R2", (0.0, 1000.0));
    // Two gates of one op-amp keep sharing their reference.
    let gate_a = part(&mut p, "OPAMP", "U5", (0.0, 0.0));
    let gate_b = part(&mut p, "OPAMP", "U5", (3000.0, -3000.0));
    let other = part(&mut p, "OPAMP", "U1", (1000.0, 0.0));
    let undo = p.apply(p.reannotate_all()).unwrap();
    let r = |id| p.component(id).unwrap().reference.clone();
    assert_eq!([r(top_left), r(top_right), r(low_left)], ["R1", "R2", "R3"]);
    assert_eq!([r(gate_a), r(gate_b), r(other)], ["U1", "U1", "U2"]);
    assert!(p.reannotate_all().is_empty(), "renumbering twice changes nothing");
    p.apply(undo).unwrap();
    assert_eq!(refs(&p), ["R7", "R?", "R2", "U5", "U5", "U1"]);
}

#[test]
fn hostile_references_never_panic() {
    let mut p = Project::new("t");
    for r in ["R18446744073709551615", "R99999999999999999999999", "?", "??", "R??", "Ω1", "1R", " ", "\u{0}"] {
        part(&mut p, "RES", r, (f64::MAX, f64::MIN));
    }
    let _ = p.next_reference("R");
    let _ = p.apply(p.annotate()).unwrap();
    let _ = p.apply(p.reannotate_all()).unwrap();
    assert!(p.issues().is_empty());
}
