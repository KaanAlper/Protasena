//! Pin alternate functions (KiCad's `alternate`): kept in the device model, written by pack
//! version 2, and version 1 packs (which have none) still load.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::{Scratch, two_pad, two_pin};
use protasena_library::{Device, Library, MIN_VERSION, Pin, PinAlternate, PinKind, VERSION, limits};

fn with_alternates() -> Device {
    let mut d = two_pin("MCU", "LIB", "P2");
    let mut symbol = (*d.symbol).clone();
    symbol.units[0].pins[0].alternates =
        vec![PinAlternate::new("SDA", PinKind::Bidirectional), PinAlternate::new("TX", PinKind::Output), PinAlternate::new("AIN0", PinKind::Input)];
    d.symbol = symbol.into();
    d
}

#[test]
fn alternates_round_trip_and_are_left_out_when_empty() {
    let d = with_alternates();
    let text = serde_json::to_string(&d).unwrap();
    assert!(
        text.contains(r#""alternates":[{"name":"SDA","kind":"bidirectional"},{"name":"TX","kind":"output"},{"name":"AIN0","kind":"input"}]"#),
        "{text}"
    );
    assert_eq!(serde_json::from_str::<Device>(&text).unwrap(), d);
    let plain = serde_json::to_string(&d.symbol.units[0].pins[1]).unwrap();
    assert!(!plain.contains("alternates"), "{plain}");
    let pin: Pin = serde_json::from_str(r#"{"number":"1","at":[0,0],"orient":"left"}"#).unwrap();
    assert!(pin.alternates.is_empty());
}

#[test]
fn packs_are_written_as_version_2_and_version_1_still_loads() {
    assert_eq!((MIN_VERSION, VERSION), (1, 2));
    let dir = Scratch::new("alternates");
    let lib = Library::from_parts(vec![with_alternates()], vec![two_pad("P2", "LIB")]).unwrap();
    lib.save_dir(dir.path()).unwrap();
    let index = std::fs::read_to_string(dir.path().join("index.json")).unwrap();
    assert!(index.contains("\"version\": 2"), "{index}");
    let back = Library::load_dir(dir.path()).unwrap();
    assert_eq!(back.get("MCU").unwrap().symbol.units[0].pins[0].alternates.len(), 3);
    // The same pack marked version 1 (as packs written before alternates were kept).
    for e in std::fs::read_dir(dir.path()).unwrap() {
        let p = e.unwrap().path();
        let text = std::fs::read_to_string(&p).unwrap().replace("\"version\": 2", "\"version\": 1");
        std::fs::write(&p, text).unwrap();
    }
    assert!(Library::load_dir(dir.path()).is_ok(), "version 1 loads");
    for e in std::fs::read_dir(dir.path()).unwrap() {
        let p = e.unwrap().path();
        let text = std::fs::read_to_string(&p).unwrap().replace("\"version\": 1", "\"version\": 3");
        std::fs::write(&p, text).unwrap();
    }
    let e = Library::load_dir(dir.path()).unwrap_err().to_string();
    assert!(e.contains("version 3 is not supported") && e.contains("versions 1 to 2"), "{e}");
}

#[test]
fn hostile_alternate_lists_are_refused() {
    let many = vec![r#"{"name":"A","kind":"input"}"#; limits::MAX_ALTERNATES + 1].join(",");
    let text = format!(r#"{{"number":"1","at":[0,0],"orient":"left","alternates":[{many}]}}"#);
    let e = serde_json::from_str::<Pin>(&text).unwrap_err().to_string();
    assert!(e.contains("more than"), "{e}");
    let mut d = with_alternates();
    let mut symbol = (*d.symbol).clone();
    symbol.units[0].pins[0].alternates.push(PinAlternate::new("", PinKind::Input));
    d.symbol = symbol.into();
    let issues = Library::from_parts(vec![d], vec![two_pad("P2", "LIB")]).unwrap_err().to_string();
    assert!(issues.contains("pin alternate"), "{issues}");
}
