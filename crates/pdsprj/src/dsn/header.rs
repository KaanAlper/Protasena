//! The fixed-size header block at the start of `ROOT.DSN`.

use crate::cursor::Cursor;

/// Banner that starts `ROOT.DSN` (each section banner ends with 0x1A).
pub(crate) const SCHEMATIC_BANNER: &[u8] = b"ISIS SCHEMATIC FILE\x1a";

/// Size of the header block; the configuration section follows it.
pub(crate) const HEADER_LEN: usize = 0x104;

const CREATED_DATE_AT: usize = 0x6f;
const MODIFIED_DATE_AT: usize = 0x8b;
const DATE_MAX: usize = 27;
const VERSION_AT: usize = 0xa7;
const TIMES_AT: usize = 0xad;

/// What the header holds that the reader decodes. Its design-property text fields are not
/// decoded yet (their order is unknown).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    /// Creation date as the program wrote it (`DD/MM/YY`).
    pub created_date: String,
    /// Last modification date (`DD/MM/YY`).
    pub modified_date: String,
    /// Program release that saved the file (the release number × 100).
    pub release: u16,
    /// File format version.
    pub file_version: u16,
    /// Creation time, Unix seconds.
    pub created: u32,
    /// Last modification time, Unix seconds.
    pub modified: u32,
}

/// Parses the header, or `None` when the data does not start with the schematic banner or is
/// shorter than the header block.
pub(crate) fn parse_header(data: &[u8]) -> Option<Header> {
    if !data.starts_with(SCHEMATIC_BANNER) || data.len() < HEADER_LEN {
        return None;
    }
    let date = |at| Cursor::new(data, at).cstr(DATE_MAX).unwrap_or_default();
    let mut v = Cursor::new(data, VERSION_AT);
    let mut t = Cursor::new(data, TIMES_AT);
    Some(Header {
        created_date: date(CREATED_DATE_AT),
        modified_date: date(MODIFIED_DATE_AT),
        release: v.u16()?,
        file_version: v.u16()?,
        created: t.u32()?,
        modified: t.u32()?,
    })
}
