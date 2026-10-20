//! A minimal ZIP writer for test archives (stored or deflated members).

use protasena_pdsprj::crc32;

/// One member to write.
pub struct Entry {
    pub name: String,
    pub data: Vec<u8>,
    pub deflate: bool,
    /// General-purpose flags written to both headers.
    pub flags: u16,
    /// Method code override (None: 0 or 8 from `deflate`).
    pub method: Option<u16>,
    /// Declared uncompressed size override.
    pub declared_size: Option<u32>,
    /// CRC override.
    pub crc: Option<u32>,
}

impl Entry {
    pub fn new(name: &str, data: &[u8], deflate: bool) -> Self {
        Self { name: name.into(), data: data.to_vec(), deflate, flags: 0, method: None, declared_size: None, crc: None }
    }
}

/// Writes an archive of `entries` with a trailing archive comment.
pub fn zip(entries: &[Entry], comment: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for e in entries {
        let stored = if e.deflate { miniz_oxide::deflate::compress_to_vec(&e.data, 6) } else { e.data.clone() };
        let method = e.method.unwrap_or(if e.deflate { 8 } else { 0 });
        let crc = e.crc.unwrap_or_else(|| crc32(&e.data));
        let size = e.declared_size.unwrap_or(e.data.len() as u32);
        let offset = out.len() as u32;
        let common = |b: &mut Vec<u8>| {
            b.extend(20u16.to_le_bytes());
            b.extend(e.flags.to_le_bytes());
            b.extend(method.to_le_bytes());
            b.extend([0, 0, 0x21, 0]); // time, date
            b.extend(crc.to_le_bytes());
            b.extend((stored.len() as u32).to_le_bytes());
            b.extend(size.to_le_bytes());
            b.extend((e.name.len() as u16).to_le_bytes());
            b.extend(0u16.to_le_bytes());
        };
        out.extend(b"PK\x03\x04");
        common(&mut out);
        out.extend(e.name.as_bytes());
        out.extend(&stored);
        central.extend(b"PK\x01\x02");
        central.extend(0u16.to_le_bytes()); // made by
        common(&mut central);
        central.extend([0u8; 10]); // comment length, disk, internal and external attributes
        central.extend(offset.to_le_bytes());
        central.extend(e.name.as_bytes());
    }
    let cd_offset = out.len() as u32;
    out.extend(&central);
    out.extend(b"PK\x05\x06");
    out.extend([0u8; 4]);
    out.extend((entries.len() as u16).to_le_bytes());
    out.extend((entries.len() as u16).to_le_bytes());
    out.extend((central.len() as u32).to_le_bytes());
    out.extend(cd_offset.to_le_bytes());
    out.extend((comment.len() as u16).to_le_bytes());
    out.extend(comment);
    out
}
