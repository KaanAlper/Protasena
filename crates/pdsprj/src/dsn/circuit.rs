//! Circuit sections: a header (sheet size) and the object records.
//!
//! ```text
//! banner "ISIS CIRCUIT FILE" 0x1A
//! 7 bytes, u32 sheet width, u32 sheet height, u16, "OBJECT DATA" NUL
//! records…, 0xFF (end of objects), trailer up to the next section (not decoded)
//! ```
//! Records are not length-prefixed. After a record the reader does not know, it scans forward
//! for the next position where a known record parses and is followed by another one (or by the
//! end of the objects); the bytes in between become one [`Record::Unknown`].

use crate::cursor::Cursor;
use crate::dsn::records::{Ctx, Record, is_anchor, read_record};
use crate::limits::{MAX_RECORDS, MAX_RESYNC_TRIES};

/// Banner of a circuit section.
pub(crate) const CIRCUIT_BANNER: &[u8] = b"ISIS CIRCUIT FILE\x1a";

const OBJECTS_MARK: &[u8] = b"OBJECT DATA\0";
const SIZE_AT: usize = 7;
const END_OF_OBJECTS: u8 = 0xff;

/// One circuit section (the first is the root sheet).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Circuit {
    /// Offset of the section banner in `ROOT.DSN`.
    pub offset: u32,
    /// Sheet width in file units (0 when the header could not be read).
    pub sheet_width: u32,
    /// Sheet height in file units.
    pub sheet_height: u32,
    /// The records in file order.
    pub records: Vec<Record>,
    /// Whether the records ended with the end-of-objects mark.
    pub ended: bool,
    /// Bytes after the end-of-objects mark (not decoded; simulation settings live there).
    pub trailer_len: usize,
}

impl Circuit {
    /// Number of [`Record::Unknown`] records.
    pub fn unknown_count(&self) -> usize {
        self.records.iter().filter(|r| matches!(r, Record::Unknown { .. })).count()
    }

    /// The record at a file offset (what links point to).
    pub fn record_at(&self, offset: u32) -> Option<&Record> {
        self.records.iter().find(|r| record_offset(r) == offset)
    }
}

/// The offset of any record.
pub fn record_offset(record: &Record) -> u32 {
    match record {
        Record::Wire(w) => w.offset,
        Record::Node(n) => n.offset,
        Record::Component(c) => c.offset,
        Record::Terminal(t) => t.offset,
        Record::Generator(g) => g.offset,
        Record::Unknown { offset, .. } => *offset,
    }
}

/// Parses the circuit section in `file[start..end]` (`start` is its banner).
pub(crate) fn parse_circuit(file: &[u8], start: usize, end: usize, styles: &[String]) -> Circuit {
    let offset = u32::try_from(start).unwrap_or(u32::MAX);
    let data = file.get(..end).unwrap_or(file);
    let head = start.saturating_add(CIRCUIT_BANNER.len());
    let mut c = Cursor::new(data, head.saturating_add(SIZE_AT));
    let size = c.u32().zip(c.u32());
    let objects = head.saturating_add(SIZE_AT + 10);
    let marked = data.get(objects..).is_some_and(|d| d.starts_with(OBJECTS_MARK));
    let (Some((sheet_width, sheet_height)), true) = (size, marked) else {
        let records = unknown(data, head, end).into_iter().collect();
        return Circuit { offset, records, ..Circuit::default() };
    };
    let ctx = Ctx { data, file_len: file.len(), styles };
    let (records, ended, trailer_len) = read_objects(&ctx, objects.saturating_add(OBJECTS_MARK.len()), end);
    Circuit { offset, sheet_width, sheet_height, records, ended, trailer_len }
}

fn unknown(data: &[u8], from: usize, to: usize) -> Option<Record> {
    let tag = *data.get(from)?;
    let bytes_len = to.checked_sub(from).filter(|&n| n > 0)?;
    Some(Record::Unknown { offset: u32::try_from(from).ok()?, tag, bytes_len })
}

fn read_objects(ctx: &Ctx<'_>, start: usize, end: usize) -> (Vec<Record>, bool, usize) {
    let mut records = Vec::new();
    let mut pos = start;
    let mut tries = 0usize;
    while pos < end {
        if ctx.data.get(pos) == Some(&END_OF_OBJECTS) {
            return (records, true, end.saturating_sub(pos.saturating_add(1)));
        }
        if records.len() >= MAX_RECORDS {
            records.extend(unknown(ctx.data, pos, end));
            break;
        }
        if let Some((record, next)) = read_record(ctx, pos) {
            records.push(record);
            pos = next;
            continue;
        }
        let next = resync(ctx, pos, end, &mut tries);
        records.extend(unknown(ctx.data, pos, next));
        pos = next;
    }
    (records, false, 0)
}

/// The first position after `pos` where a known record parses and is followed by the end of
/// the objects or by another known record; `end` when there is none (or the tries run out).
fn resync(ctx: &Ctx<'_>, pos: usize, end: usize, tries: &mut usize) -> usize {
    let mut q = pos.saturating_add(1);
    while q < end && *tries < MAX_RESYNC_TRIES {
        if is_anchor(ctx.data, q) {
            *tries = tries.saturating_add(1);
            if let Some((_, after)) = read_record(ctx, q) {
                let confirmed = after >= end || ctx.data.get(after) == Some(&END_OF_OBJECTS) || read_record(ctx, after).is_some();
                if confirmed {
                    return q;
                }
            }
        }
        q = q.saturating_add(1);
    }
    end
}
