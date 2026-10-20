//! Pick Devices search, lookup and the category tree.
// Test code: helpers outside `#[test]` functions may panic too (clippy.toml covers only those).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::{two_pad, two_pin};
use protasena_library::{Category, Device, Library, Query};

fn names(hits: &[&Device]) -> Vec<String> {
    hits.iter().map(|d| d.name.clone()).collect()
}

fn sample() -> Library {
    let mk = |name: &str, desc: &str, cat: &str, sub: &str, mfr: &str, kw: &[&str]| Device {
        description: desc.into(),
        category: cat.into(),
        sub_category: sub.into(),
        manufacturer: mfr.into(),
        keywords: kw.iter().map(|k| (*k).to_owned()).collect(),
        ..two_pin(name, "LIB", "P2")
    };
    let devices = vec![
        mk("BC547", "NPN small-signal transistor", "Transistors", "Bipolar", "Acme", &["amplifier"]),
        mk("2N3906", "PNP small-signal transistor", "Transistors", "Bipolar", "Other", &[]),
        mk("IRF540", "N-channel power MOSFET", "Transistors", "MOSFET", "Acme", &["switch"]),
        mk("LM358", "Dual operational amplifier", "Operational Amplifiers", "Dual", "Acme", &["opamp"]),
        mk("Ünit", "Ärger test part", "Tests", "", "", &[]),
    ];
    Library::from_parts(devices, vec![two_pad("P2", "LIB")]).unwrap()
}

#[test]
fn every_keyword_must_match_name_description_or_keywords() {
    let lib = sample();
    assert_eq!(names(&lib.search(&Query::keywords("transistor"))), ["2N3906", "BC547"]);
    assert_eq!(names(&lib.search(&Query::keywords("small npn"))), ["BC547"]);
    assert_eq!(names(&lib.search(&Query::keywords("amplifier"))), ["BC547", "LM358"]);
    assert_eq!(names(&lib.search(&Query::keywords("bc5"))), ["BC547"]);
    assert!(lib.search(&Query::keywords("npn mosfet")).is_empty());
}

#[test]
fn matching_ignores_case_including_non_ascii() {
    let lib = sample();
    assert_eq!(names(&lib.search(&Query::keywords("MOSFET n-CHANNEL"))), ["IRF540"]);
    assert_eq!(names(&lib.search(&Query::keywords("ärger ÜNIT"))), ["Ünit"]);
    assert_eq!(lib.get("lm358").map(|d| d.name.as_str()), Some("LM358"));
    assert_eq!(lib.get("üNIT").map(|d| d.name.as_str()), Some("Ünit"));
    assert!(lib.get("LM35").is_none());
}

#[test]
fn filters_narrow_by_whole_category_sub_category_and_manufacturer() {
    let lib = sample();
    assert_eq!(names(&lib.search(&Query::default().category("transistors"))), ["2N3906", "BC547", "IRF540"]);
    assert_eq!(names(&lib.search(&Query::default().category("Transistors").sub_category("bipolar"))), ["2N3906", "BC547"]);
    assert_eq!(names(&lib.search(&Query::keywords("transistor").manufacturer("ACME"))), ["BC547"]);
    assert!(lib.search(&Query::default().category("Transistor")).is_empty(), "whole values only");
    assert_eq!(lib.search(&Query::keywords("   ")).len(), lib.len(), "blank keywords match all");
}

#[test]
fn categories_tree_counts_devices() {
    let lib = sample();
    let tree = lib.categories();
    assert_eq!(
        tree,
        [
            Category { name: "Operational Amplifiers".into(), count: 1, sub_categories: vec![("Dual".into(), 1)] },
            Category { name: "Tests".into(), count: 1, sub_categories: vec![] },
            Category { name: "Transistors".into(), count: 3, sub_categories: vec![("Bipolar".into(), 2), ("MOSFET".into(), 1)] },
        ]
    );
}

#[test]
fn builtin_search_finds_the_generic_parts() {
    let lib = Library::builtin();
    assert_eq!(names(&lib.search(&Query::keywords("capacitor"))), ["CAP", "CAP-ELEC"]);
    assert_eq!(names(&lib.search(&Query::keywords("res"))), ["RES"]);
    assert_eq!(names(&lib.search(&Query::default().category("Power"))), ["GROUND", "POWER"]);
    assert_eq!(names(&lib.search(&Query::keywords("transistor").sub_category("Bipolar"))), ["NPN", "PNP"]);
    assert!(lib.categories().iter().all(|c| c.count > 0));
}
