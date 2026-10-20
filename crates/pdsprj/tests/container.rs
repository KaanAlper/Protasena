//! The ZIP container: members, extraction and every refusal.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::zip::{Entry, zip};
use protasena_pdsprj::{Container, Error, Method, limits};

fn one(entry: Entry) -> Container {
    Container::from_bytes(zip(&[entry], b"")).unwrap()
}

#[test]
fn lists_and_reads_stored_and_deflated_members() {
    let text = b"hello hello hello hello".repeat(20);
    let c = Container::from_bytes(zip(&[Entry::new("A.TXT", &text, true), Entry::new("DIR/B.BIN", &[1, 2, 3], false)], b"a comment")).unwrap();
    let m: Vec<_> = c.members().iter().map(|m| (m.name.as_str(), m.method, m.size)).collect();
    assert_eq!(m, [("A.TXT", Method::Deflated, text.len() as u64), ("DIR/B.BIN", Method::Stored, 3)]);
    assert!(c.members()[0].compressed_size < text.len() as u64);
    assert_eq!(c.read("a.txt").unwrap(), text);
    assert_eq!(c.read("DIR/B.BIN").unwrap(), [1, 2, 3]);
    assert!(matches!(c.read("C"), Err(Error::MissingMember(_))));
    let empty = Container::from_bytes(zip(&[], b"")).unwrap();
    assert!(empty.members().is_empty());
}

#[test]
fn refuses_damaged_or_unsupported_members() {
    let crc = Entry { crc: Some(1), ..Entry::new("X", b"data", true) };
    assert!(matches!(one(crc).read("X"), Err(Error::Corrupt { .. })));
    let encrypted = Entry { flags: 1, ..Entry::new("X", b"data", false) };
    assert!(matches!(one(encrypted).read("X"), Err(Error::Unsupported(_))));
    let method = Entry { method: Some(12), ..Entry::new("X", b"data", false) };
    assert!(matches!(one(method).read("X"), Err(Error::Unsupported(_))));
    let short = Entry { declared_size: Some(3), ..Entry::new("X", b"data", false) };
    assert!(matches!(one(short).read("X"), Err(Error::Corrupt { .. })));
    let long = Entry { declared_size: Some(5000), ..Entry::new("X", &[0u8; 4000], true) };
    assert!(matches!(one(long).read("X"), Err(Error::Corrupt { .. })));
    let truncated_inflate = Entry { declared_size: Some(10), ..Entry::new("X", &[7u8; 4000], true) };
    assert!(matches!(one(truncated_inflate).read("X"), Err(Error::Corrupt { .. })));
}

#[test]
fn refuses_zip_bombs_and_oversized_members() {
    let bomb = Entry { declared_size: Some(64 * 1024 * 1024), ..Entry::new("X", &[0u8; 100], true) };
    assert!(matches!(one(bomb).read("X"), Err(Error::SuspiciousRatio { .. })));
    let huge = Entry { declared_size: Some(u32::MAX - 1), ..Entry::new("X", &[0u8; 100], false) };
    assert!(matches!(one(huge).read("X"), Err(Error::TooLarge { .. })));
    // Highly compressible but small members are fine.
    assert_eq!(one(Entry::new("Z", &vec![0u8; 512 * 1024], true)).read("Z").unwrap().len(), 512 * 1024);
}

#[test]
fn refuses_bad_directories() {
    assert!(matches!(Container::from_bytes(Vec::new()), Err(Error::NotZip(_))));
    assert!(matches!(Container::from_bytes(b"PK\x05\x06".to_vec()), Err(Error::NotZip(_))));
    let entries: Vec<_> = (0..=limits::MAX_MEMBERS).map(|i| Entry::new(&format!("M{i}"), b"x", false)).collect();
    assert!(matches!(Container::from_bytes(zip(&entries, b"")), Err(Error::TooManyMembers { .. })));
    let dup = zip(&[Entry::new("A", b"1", false), Entry::new("a", b"2", false)], b"");
    assert!(matches!(Container::from_bytes(dup), Err(Error::DuplicateMember(_))));
    // A directory offset past the data.
    let mut bad = zip(&[Entry::new("A", b"1", false)], b"");
    let n = bad.len();
    bad[n - 6..n - 2].copy_from_slice(&0x7fff_0000u32.to_le_bytes());
    assert!(matches!(Container::from_bytes(bad), Err(Error::NotZip(_))));
    // ZIP64 and multi-disk markers.
    let mut z64 = zip(&[], b"");
    z64[10..12].copy_from_slice(&0xffffu16.to_le_bytes());
    assert!(matches!(Container::from_bytes(z64), Err(Error::Unsupported(_))));
    let mut disk = zip(&[], b"");
    disk[4] = 1;
    assert!(matches!(Container::from_bytes(disk), Err(Error::Unsupported(_))));
}

#[test]
fn member_data_must_lie_inside_the_archive() {
    let mut bytes = zip(&[Entry::new("A", b"abc", false)], b"");
    bytes[0] = b'X'; // break the local header signature
    let c = Container::from_bytes(bytes).unwrap();
    assert!(matches!(c.read("A"), Err(Error::Corrupt { .. })));
}

#[test]
fn archive_size_is_limited() {
    let too_big = vec![0u8; limits::MAX_ARCHIVE_BYTES as usize + 1];
    assert!(matches!(Container::from_bytes(too_big), Err(Error::TooLarge { .. })));
}
