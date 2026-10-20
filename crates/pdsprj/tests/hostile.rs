//! Hostile input never panics: random bytes, mutations, truncations and insertions of our own
//! synthetic files go through every reader.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::Rng;
use common::dsn::sample_design;
use protasena_pdsprj::{Container, Project, parse_design, parse_manifest, parse_property_block, parse_sections};

fn read_everything(bytes: Vec<u8>) {
    if let Ok(c) = Container::from_bytes(bytes) {
        for m in c.members() {
            if let Ok(data) = c.read(&m.name) {
                let _ = parse_design(&data);
                let _ = parse_manifest(&data);
            }
        }
        let _ = Project::from_container(&c);
    }
}

fn mutate(rng: &mut Rng, base: &[u8]) -> Vec<u8> {
    let mut b = base.to_vec();
    for _ in 0..=rng.below(8) {
        let at = rng.below(b.len());
        match rng.below(4) {
            0 => b[at] = rng.next() as u8,
            1 => b[at] = [0x00, 0xff, 0x7f, 0x80, 0x01][rng.below(5)],
            2 => b.truncate(at),
            _ => {
                let n = rng.below(16);
                let extra = rng.bytes(n);
                b.splice(at..at, extra);
            }
        }
        if b.is_empty() {
            break;
        }
    }
    b
}

#[test]
fn hostile_random_bytes_never_panic() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..2000 {
        let len = rng.below(600);
        let data = rng.bytes(len);
        read_everything(data.clone());
        let _ = parse_design(&data);
        let _ = parse_manifest(&data);
        let text = String::from_utf8_lossy(&data);
        let _ = parse_sections(&text);
        let _ = parse_property_block(&text);
    }
}

#[test]
fn hostile_mutated_designs_never_panic() {
    let base = sample_design();
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    for _ in 0..3000 {
        let _ = parse_design(&mutate(&mut rng, &base));
    }
    // Random bytes behind a valid header and configuration.
    let mut prefix = base.clone();
    let cut = prefix.windows(18).position(|w| w == b"ISIS CIRCUIT FILE\x1a").unwrap() + 18;
    prefix.truncate(cut);
    for _ in 0..1000 {
        let mut d = prefix.clone();
        let n = rng.below(400);
        d.extend(rng.bytes(n));
        let _ = parse_design(&d);
    }
}

#[test]
fn hostile_truncated_designs_never_panic() {
    let base = sample_design();
    for len in 0..base.len() {
        let _ = parse_design(&base[..len]);
    }
}

#[test]
fn hostile_mutated_archives_never_panic() {
    let base = common::archive(&sample_design());
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    for _ in 0..1500 {
        read_everything(mutate(&mut rng, &base));
    }
    // Stored members expose the design bytes to mutation without breaking the deflate stream.
    let stored = common::zip::zip(&[common::zip::Entry::new("ROOT.DSN", &sample_design(), false)], b"");
    for _ in 0..1500 {
        read_everything(mutate(&mut rng, &stored));
    }
}

#[test]
fn hostile_dense_anchor_bytes_resync_in_bounded_time() {
    // A circuit full of bytes that look like record starts makes the resync scan try many
    // candidates; it must finish and keep the bytes as unknown records.
    let mut d = common::dsn::Dsn::new(&[]);
    d.circuit();
    d.buf.extend([0x00, 0xff, 0x30, 0x08].iter().cycle().take(200_000).copied());
    let design = parse_design(&d.buf).unwrap();
    assert!(design.unknown_count() >= 1);
}
