//! Hostile `.psproj` input never panics: it loads or gives a [`LoadError`].
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::{sample_project, scratch};
use protasena_io::{LoadError, decode, encode, limits, load};

/// A tiny deterministic generator (xorshift) for fuzz-style inputs.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn hostile_garbage_never_panics() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for len in 0..400 {
        let bytes: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
        assert!(decode(&bytes).is_err());
    }
    for text in
        ["", " ", "{", "}", "null", "true", "{\"format\":", "\u{feff}{}", "{\"format\": \"protasena-project\", \"version\": 1, \"project\": null}"]
    {
        assert!(decode(text.as_bytes()).is_err(), "{text:?}");
    }
}

#[test]
fn hostile_mutations_of_a_real_file_never_panic() {
    let good = encode(&sample_project()).unwrap().into_bytes();
    let mut rng = Rng(42);
    for _ in 0..2000 {
        let mut bytes = good.clone();
        for _ in 0..1 + rng.next() % 4 {
            let at = (rng.next() as usize) % bytes.len();
            match rng.next() % 3 {
                0 => bytes[at] = rng.next() as u8,
                1 => {
                    bytes.remove(at);
                }
                _ => bytes.insert(at, b"{}[],:\"0e-9"[(rng.next() % 11) as usize]),
            }
        }
        let _ = decode(&bytes);
    }
}

#[test]
fn hostile_truncated_files_never_panic() {
    let good = encode(&sample_project()).unwrap().into_bytes();
    // The last byte is the final newline; without it the file is still complete.
    for cut in 0..good.len() - 1 {
        let truncated = &good[..cut];
        // A truncated file never loads (the closing brace is the last thing written).
        assert!(decode(truncated).is_err(), "cut at {cut}");
    }
}

#[test]
fn hostile_deep_nesting_is_refused_before_parsing() {
    for open in ["[", "{\"a\":"] {
        let text = format!("{{\"format\": \"protasena-project\", \"version\": 1, \"project\": {}", open.repeat(100_000));
        assert_eq!(decode(text.as_bytes()), Err(LoadError::TooDeep { limit: limits::MAX_DEPTH }));
    }
    // Brackets inside strings do not count.
    let text = format!("{{\"format\": \"protasena-project\", \"version\": 1, \"project\": {{\"name\": \"{}\\\"\"}}}}", "[{".repeat(1000));
    let (p, _) = decode(text.as_bytes()).unwrap();
    assert!(p.name.starts_with("[{[{") && p.name.ends_with('"'));
}

#[test]
fn hostile_huge_numbers_never_panic() {
    let base = encode(&sample_project()).unwrap();
    for (from, to) in [
        ("\"next_id\": ", "\"next_id\": 1e999, \"x\": "),
        ("\"next_id\": ", "\"next_id\": 18446744073709551616, \"x\": "),
        ("\"next_id\": ", "\"next_id\": -1, \"x\": "),
        ("[200.0, 0.0]", "[1e400, 0.0]"),
        ("[200.0, 0.0]", "[0.0, -1e309]"),
        ("\"angle\": 270", "\"angle\": 99999999999999999999"),
        ("\"angle\": 270", "\"angle\": 2147483640"),
        ("\"size\": 12.5", "\"size\": 1e308"),
    ] {
        assert!(base.contains(from), "{from}");
        let text = base.replacen(from, to, 1);
        let _ = decode(text.as_bytes());
    }
    // Ids at the top of the range load; the counter is then exhausted, which the model reports.
    let mut p = sample_project();
    p.sheets[0].junctions[0].id.0 = u64::MAX;
    p.next_id = u64::MAX;
    let (mut back, report) = decode(encode(&p).unwrap().as_bytes()).unwrap();
    assert_eq!(report.warnings.len(), 1);
    assert!(back.fresh_id::<protasena_doc::WireId>().is_err());
}

#[test]
fn hostile_item_counts_are_bounded() {
    // (The item limit is tested in the crate's unit tests: a file over it is very large.)
    let sheets = "{},".repeat(limits::MAX_SHEETS + 1);
    let text = format!("{{\"format\": \"protasena-project\", \"version\": 1, \"project\": {{\"sheets\": [{}]}}}}", sheets.trim_end_matches(','));
    assert!(matches!(decode(text.as_bytes()), Err(LoadError::TooMany { what: "sheets", .. })));
}

#[test]
fn hostile_oversized_files_are_refused_without_reading_them() {
    let dir = scratch("oversized");
    let path = dir.join("big.psproj");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(limits::MAX_FILE_BYTES + 1).unwrap();
    drop(file);
    assert!(matches!(load(&path), Err(LoadError::TooLarge { limit: limits::MAX_FILE_BYTES, .. })));
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn hostile_invalid_utf8_and_wrong_types_never_panic() {
    let base = encode(&sample_project()).unwrap();
    let mut bytes = base.clone().into_bytes();
    let at = base.find("divider").unwrap();
    bytes[at] = 0xff;
    assert!(matches!(decode(&bytes), Err(LoadError::Json(_))));
    for (from, to) in
        [("\"name\": \"divider\"", "\"name\": 5"), ("\"sheets\": [", "\"sheets\": {\"a\": ["), ("\"kind\": \"power\"", "\"kind\": \"nuclear\"")]
    {
        assert!(base.contains(from), "{from}");
        assert!(decode(base.replacen(from, to, 1).as_bytes()).is_err(), "{to}");
    }
}
