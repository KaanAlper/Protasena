//! Coordinates and the text fields that records carry (reference, value, labels, property
//! blocks).
//!
//! A text field:
//! ```text
//! u8 0xFF, pstr text, i32 x, i32 y,
//! 26 bytes of attributes (u16 flags at +4, u32 character height at +10, last byte: shown),
//! cstr font, cstr text style, u32 (0),
//! when flags & 4: u32 total length, then total - len(text) more bytes of text
//! ```

use crate::cursor::{Cursor, is_name, latin1};
use crate::limits::{MAX_FIELDS, MAX_TEXT_BYTES};

/// File units per thou (1/1000 in): coordinates are in 10 nm steps.
pub const UNITS_PER_THOU: i32 = 2540;

const FIELD_MARK: u8 = 0xff;
const ATTR_LEN: usize = 26;
const FLAG_CONTINUED: u16 = 4;
const STYLE_MAX: usize = 64;

/// A point in file units (see [`UNITS_PER_THOU`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Coord {
    /// x.
    pub x: i32,
    /// y.
    pub y: i32,
}

impl Coord {
    /// The point in thou.
    pub fn to_thou(self) -> (f64, f64) {
        let k = f64::from(UNITS_PER_THOU);
        (f64::from(self.x) / k, f64::from(self.y) / k)
    }

    pub(crate) fn read(c: &mut Cursor<'_>) -> Option<Self> {
        Some(Self { x: c.i32()?, y: c.i32()? })
    }
}

/// A text field of a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextField {
    /// The text (with its continuation, for property blocks).
    pub text: String,
    /// Anchor on the sheet.
    pub at: Coord,
    /// Text style name (one of the configuration's text styles).
    pub style: String,
    /// Font name.
    pub font: String,
    /// Character height in file units.
    pub height: u32,
    /// Attribute flags (bit 2: the text continues after the field).
    pub flags: u16,
    /// Whether the text is shown (the last attribute byte; inferred, not yet confirmed).
    pub visible: bool,
}

/// Reads the text fields at the cursor (while the next byte is the field mark). `styles` are
/// the configuration's text styles; when it is non-empty, every field must use one of them.
pub(crate) fn read_fields(c: &mut Cursor<'_>, styles: &[String]) -> Option<Vec<TextField>> {
    let mut fields = Vec::new();
    while c.peek(0) == Some(FIELD_MARK) {
        if fields.len() >= MAX_FIELDS {
            return None;
        }
        fields.push(read_field(c, styles)?);
    }
    Some(fields)
}

fn read_field(c: &mut Cursor<'_>, styles: &[String]) -> Option<TextField> {
    if c.u8()? != FIELD_MARK {
        return None;
    }
    let mut text = c.pstr()?;
    let at = Coord::read(c)?;
    let attrs: [u8; ATTR_LEN] = c.array()?;
    let mut a = Cursor::new(&attrs, 4);
    let flags = a.u16()?;
    let height = Cursor::new(&attrs, 10).u32()?;
    let visible = attrs.last().is_some_and(|&b| b != 0);
    let font = c.cstr(STYLE_MAX)?;
    let style = c.cstr(STYLE_MAX)?;
    let known = styles.is_empty() || styles.contains(&style);
    if !is_name(&font, STYLE_MAX) || !is_name(&style, STYLE_MAX) || !known {
        return None;
    }
    c.skip(4)?;
    if flags & FLAG_CONTINUED != 0 {
        let total = usize::try_from(c.u32()?).ok()?;
        if total > MAX_TEXT_BYTES {
            return None;
        }
        let rest = c.bytes(total.checked_sub(text.len())?)?;
        text.push_str(&latin1(rest));
    }
    Some(TextField { text, at, style, font, height, flags, visible })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(text: &str, style: &str, flags: u16, more: &[u8]) -> Vec<u8> {
        let mut b = vec![FIELD_MARK, text.len() as u8];
        b.extend(text.as_bytes());
        b.extend(2540i32.to_le_bytes());
        b.extend((-5080i32).to_le_bytes());
        let mut attrs = [0u8; ATTR_LEN];
        attrs[4..6].copy_from_slice(&flags.to_le_bytes());
        attrs[10..14].copy_from_slice(&254_000u32.to_le_bytes());
        attrs[25] = 1;
        b.extend(attrs);
        b.extend(b"Our Font\0");
        b.extend(style.as_bytes());
        b.extend([0, 0, 0, 0, 0]);
        b.extend(more);
        b
    }

    #[test]
    fn reads_fields_and_continuations() {
        let mut data = field("X1", "LABEL A", 0, &[]);
        data.extend(field("{K=", "BLOCK", FLAG_CONTINUED, &[5, 0, 0, 0, b'1', b'}']));
        let styles = vec!["LABEL A".to_owned(), "BLOCK".to_owned()];
        let fields = read_fields(&mut Cursor::new(&data, 0), &styles).unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!((fields[0].text.as_str(), fields[0].at, fields[0].height), ("X1", Coord { x: 2540, y: -5080 }, 254_000));
        assert!(fields[0].visible);
        assert_eq!(fields[1].text, "{K=1}");
        assert_eq!(fields[0].at.to_thou(), (1.0, -2.0));
        // An unknown style is rejected when the styles are known, accepted otherwise.
        assert!(read_fields(&mut Cursor::new(&data, 0), &["OTHER".to_owned()]).is_none());
        assert!(read_fields(&mut Cursor::new(&data, 0), &[]).is_some());
        // A continuation shorter than the text is rejected.
        let bad = field("long text", "BLOCK", FLAG_CONTINUED, &[2, 0, 0, 0]);
        assert!(read_fields(&mut Cursor::new(&bad, 0), &[]).is_none());
    }
}
