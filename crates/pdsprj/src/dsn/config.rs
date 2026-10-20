//! The design configuration section: style tables and the embedded library.
//!
//! Layout (little-endian):
//! ```text
//! banner "DESIGN CONFIGURATION DATA" 0x1A
//! preamble          fixed size: fonts, colours, sheet settings (not decoded)
//! graphic styles    u16 count, count × style
//! text styles       u16 count, count × style
//! style             pstr name, u8 flags, u16 n, n-2 bytes (n counts itself)
//! library lists     while u16 0xFFFE: entries until an empty name
//! entry             pstr name, u32 stamp, u32 n, n-4 bytes (n counts itself)
//! u32               trailer (not decoded)
//! ```
//! An entry whose body ends in a definition text (see [`DeviceDef`]) is an embedded device
//! definition; any other entry is an embedded symbol, kept by name and size only.

use crate::cursor::{Cursor, is_name};
use crate::dsn::text::{TextSection, parse_sections};
use crate::limits::{MAX_LIBRARY_ENTRIES, MAX_STYLES, MAX_TEXT_BYTES};

/// Banner of the configuration section.
pub(crate) const CONFIG_BANNER: &[u8] = b"DESIGN CONFIGURATION DATA\x1a";

/// Bytes between the banner and the graphic styles (constant in every file seen).
const PREAMBLE_LEN: usize = 0x2be;

const LIST_MARKER: u16 = 0xfffe;
const NAME_MAX: usize = 255;

/// An embedded library symbol: name and size only (its graphics are not decoded).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryEntry {
    /// Symbol name.
    pub name: String,
    /// Size of its data in bytes.
    pub bytes_len: usize,
}

/// An embedded device definition. Its binary part (before the text) is not decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceDef {
    /// Device name.
    pub name: String,
    /// The definition text, by section (`DEVICE`, `PROPDEFS`, `INDEX`, `COMPONENT`, …).
    pub sections: Vec<TextSection>,
}

impl DeviceDef {
    /// The first section with this name (compared without case).
    pub fn section(&self, name: &str) -> Option<&TextSection> {
        self.sections.iter().find(|s| s.name.eq_ignore_ascii_case(name))
    }
}

/// What the configuration section holds that the reader decodes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    /// Names of the graphic styles.
    pub graphic_styles: Vec<String>,
    /// Names of the text styles.
    pub text_styles: Vec<String>,
    /// Embedded symbols.
    pub symbols: Vec<LibraryEntry>,
    /// Embedded device definitions.
    pub devices: Vec<DeviceDef>,
}

/// Parses the configuration section starting at `start` (its banner). Returns the section and
/// where it ends, or `None` if it does not have the expected layout.
pub(crate) fn parse_config(data: &[u8], start: usize) -> Option<(Config, usize)> {
    if !data.get(start..)?.starts_with(CONFIG_BANNER) {
        return None;
    }
    let mut c = Cursor::new(data, start.checked_add(CONFIG_BANNER.len())?.checked_add(PREAMBLE_LEN)?);
    let graphic_styles = styles(&mut c)?;
    let text_styles = styles(&mut c)?;
    let mut config = Config { graphic_styles, text_styles, ..Config::default() };
    let mut entries = 0usize;
    while Cursor::new(data, c.pos()).u16() == Some(LIST_MARKER) {
        c.skip(2)?;
        loop {
            let name = c.pstr()?;
            if name.is_empty() {
                break;
            }
            entries = entries.checked_add(1)?;
            if entries > MAX_LIBRARY_ENTRIES || !is_name(&name, NAME_MAX) {
                return None;
            }
            let _stamp = c.u32()?;
            let len = usize::try_from(c.u32()?).ok()?;
            let body = c.bytes(len.checked_sub(4)?)?;
            match definition_text(body) {
                Some(text) => config.devices.push(DeviceDef { name, sections: parse_sections(&text) }),
                None => config.symbols.push(LibraryEntry { name, bytes_len: body.len() }),
            }
        }
    }
    c.skip(4)?; // trailer
    Some((config, c.pos()))
}

fn styles(c: &mut Cursor<'_>) -> Option<Vec<String>> {
    let count = usize::from(c.u16()?);
    if count > MAX_STYLES {
        return None;
    }
    (0..count)
        .map(|_| {
            let name = c.pstr()?;
            let _flags = c.u8()?;
            let len = usize::from(c.u16()?);
            c.skip(len.checked_sub(2)?)?;
            is_name(&name, NAME_MAX).then_some(name)
        })
        .collect()
}

/// The definition text at the end of an entry body: the body starts with the text's offset
/// (counted from the body start), the text's length (with its final NUL) precedes it, and the
/// text runs to the end of the body.
fn definition_text(body: &[u8]) -> Option<String> {
    let offset = usize::try_from(Cursor::new(body, 0).u32()?).ok()?;
    let len = usize::try_from(Cursor::new(body, offset.checked_sub(4)?).u32()?).ok()?;
    if offset < 8 || len == 0 || len > MAX_TEXT_BYTES || offset.checked_add(len)? != body.len() {
        return None;
    }
    let text = body.get(offset..offset.checked_add(len)?.checked_sub(1)?)?;
    if !text.starts_with(b"{*") || body.last() != Some(&0) {
        return None;
    }
    Some(crate::cursor::latin1(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body_with_text(text: &[u8], gap: usize) -> Vec<u8> {
        let offset = 8 + gap;
        let mut b = (offset as u32).to_le_bytes().to_vec();
        b.extend(vec![0; gap]);
        b.extend(((text.len() + 1) as u32).to_le_bytes());
        b.extend(text);
        b.push(0);
        b
    }

    #[test]
    fn definition_text_needs_the_exact_layout() {
        let body = body_with_text(b"{*DEVICE}\n{PREFIX=X}\n", 3);
        assert_eq!(definition_text(&body).as_deref(), Some("{*DEVICE}\n{PREFIX=X}\n"));
        let mut longer = body.clone();
        longer.push(0);
        assert_eq!(definition_text(&longer), None);
        assert_eq!(definition_text(&body_with_text(b"plain", 0)), None);
        assert_eq!(definition_text(&[0xff; 16]), None);
        assert_eq!(definition_text(&[]), None);
    }
}
