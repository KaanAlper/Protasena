//! The object records of a circuit and their strict readers.
//!
//! Every record starts with a tag byte. Links are absolute offsets of other records in
//! `ROOT.DSN` (0 = none). Layouts (after the tag):
//! ```text
//! wire       tag 0x00: style block (24 bytes: u8 flags, 21 bytes, 0x02 0x7F), cstr style,
//!            u16 0, u16 n, n × (i32 x, i32 y)
//! component  tag 0x00 or 0x08, followed by text fields: fields, u16 pins, pstr device,
//!            i32 x, i32 y, 17 bytes (u32 ordinal at +4), pins × u32 link
//! node       tag 0x01: i32 x, i32 y, 4 × u32 link
//! terminal   tag 0x10: i32 x, i32 y, u32, pstr symbol, 6 bytes, fields, u32 link
//! generator  tag 0x30..=0x3F (generators and probes): i32 x, i32 y, u32, u16, pstr name,
//!            pstr symbol, fields, u8, u8, 2 × u32 link,
//!            when tag & 0x08: u8, u32 n, n-4 bytes (n counts itself)
//! ```

use crate::cursor::{Cursor, is_name};
use crate::dsn::field::{Coord, TextField, read_fields};
use crate::dsn::text::parse_property_block;
use crate::limits::{MAX_PINS, MAX_TEXT_BYTES};

/// Text style of a component's reference field.
pub(crate) const REFERENCE_STYLE: &str = "COMPONENT ID";
/// Text style of a component's value field.
pub(crate) const VALUE_STYLE: &str = "COMPONENT VALUE";

const NAME_MAX: usize = 255;
const STYLE_BLOCK_LEN: usize = 24;
const STYLE_BLOCK_END: [u8; 2] = [0x02, 0x7f];
const PLACEMENT_LEN: usize = 17;
const TERMINAL_GAP: usize = 6;

/// A wire or bus polyline (the style name tells them apart).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wire {
    /// Offset of the record in `ROOT.DSN`.
    pub offset: u32,
    /// Graphic style name (`WIRE`, `BUS WIRE`).
    pub style: String,
    /// Vertices.
    pub points: Vec<Coord>,
}

/// A connection node where wires and pins meet (up to four links).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Offset of the record.
    pub offset: u32,
    /// Position.
    pub at: Coord,
    /// The records it joins (0 = unused slot).
    pub links: [u32; 4],
}

impl Node {
    /// Number of used links.
    pub fn degree(&self) -> usize {
        self.links.iter().filter(|&&l| l != 0).count()
    }
}

/// A placed component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// Offset of the record.
    pub offset: u32,
    /// The tag byte (0x00 or 0x08; the meaning of 0x08 is not known).
    pub tag: u8,
    /// Device name.
    pub device: String,
    /// Origin.
    pub at: Coord,
    /// A per-component number (meaning unconfirmed).
    pub ordinal: u32,
    /// Text fields (reference, value, device, property block, …).
    pub fields: Vec<TextField>,
    /// One link per pin (0 = unconnected).
    pub pins: Vec<u32>,
}

impl Component {
    /// The field with this text style.
    pub fn field(&self, style: &str) -> Option<&TextField> {
        self.fields.iter().find(|f| f.style == style)
    }

    /// The reference field.
    pub fn reference_field(&self) -> Option<&TextField> {
        self.field(REFERENCE_STYLE)
    }

    /// The value field.
    pub fn value_field(&self) -> Option<&TextField> {
        self.field(VALUE_STYLE)
    }

    /// The reference (`R1`); empty when the field is absent.
    pub fn reference(&self) -> &str {
        self.reference_field().map_or("", |f| f.text.as_str())
    }

    /// The value (`10k`); empty when the field is absent.
    pub fn value(&self) -> &str {
        self.value_field().map_or("", |f| f.text.as_str())
    }

    /// The `{KEY=VALUE}` pairs of every field that holds a property block, in order.
    pub fn properties(&self) -> Vec<(String, String)> {
        self.fields.iter().filter(|f| f.text.starts_with('{')).flat_map(|f| parse_property_block(&f.text)).collect()
    }
}

/// A terminal (power, ground, port, …), identified by its symbol name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terminal {
    /// Offset of the record.
    pub offset: u32,
    /// Connection point.
    pub at: Coord,
    /// Symbol name.
    pub symbol: String,
    /// Text fields (the label first).
    pub fields: Vec<TextField>,
    /// The record it connects to.
    pub link: u32,
}

impl Terminal {
    /// The label text (the net name; may be empty).
    pub fn label(&self) -> &str {
        self.fields.first().map_or("", |f| f.text.as_str())
    }
}

/// A generator or probe (simulation objects).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generator {
    /// Offset of the record.
    pub offset: u32,
    /// The tag byte (0x30..=0x3F).
    pub tag: u8,
    /// Position.
    pub at: Coord,
    /// Its internal name.
    pub name: String,
    /// Symbol name.
    pub symbol: String,
    /// Text fields (the label first).
    pub fields: Vec<TextField>,
    /// The records it connects to (0 = none).
    pub links: [u32; 2],
}

/// One object record of a circuit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    /// A wire or bus.
    Wire(Wire),
    /// A connection node.
    Node(Node),
    /// A component.
    Component(Component),
    /// A terminal.
    Terminal(Terminal),
    /// A generator or probe.
    Generator(Generator),
    /// A record the reader does not decode: its tag and size, not its bytes.
    Unknown {
        /// Offset of the record.
        offset: u32,
        /// Its first byte.
        tag: u8,
        /// Its size up to the next decoded record.
        bytes_len: usize,
    },
}

/// What a record reader needs: the circuit's data (records never reach past it), the size of
/// the whole file (links must point inside it) and the known text styles.
pub(crate) struct Ctx<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) file_len: usize,
    pub(crate) styles: &'a [String],
}

impl Ctx<'_> {
    fn link(&self, c: &mut Cursor<'_>) -> Option<u32> {
        let v = c.u32()?;
        (v == 0 || usize::try_from(v).is_ok_and(|v| v < self.file_len)).then_some(v)
    }
}

/// Whether a record that the resynchronising scan may start at begins at `pos`.
pub(crate) fn is_anchor(data: &[u8], pos: usize) -> bool {
    matches!(record_kind(data, pos), Some(Kind::Wire | Kind::Component | Kind::Generator))
}

#[derive(Clone, Copy)]
enum Kind {
    Wire,
    Component,
    Node,
    Terminal,
    Generator,
}

fn record_kind(data: &[u8], pos: usize) -> Option<Kind> {
    let tag = *data.get(pos)?;
    let next = data.get(pos.checked_add(1)?).copied();
    let block_end = pos.checked_add(1 + STYLE_BLOCK_LEN)?;
    let style_end = data.get(block_end.checked_sub(2)?..block_end);
    match tag {
        0x00 | 0x08 if next == Some(0xff) => Some(Kind::Component),
        0x00 if style_end == Some(STYLE_BLOCK_END.as_slice()) => Some(Kind::Wire),
        0x01 => Some(Kind::Node),
        0x10 => Some(Kind::Terminal),
        0x30..=0x3f => Some(Kind::Generator),
        _ => None,
    }
}

/// Reads the record at `pos` and returns it with the position after it, or `None` when no
/// known record parses there.
pub(crate) fn read_record(ctx: &Ctx<'_>, pos: usize) -> Option<(Record, usize)> {
    let kind = record_kind(ctx.data, pos)?;
    let tag = *ctx.data.get(pos)?;
    let offset = u32::try_from(pos).ok()?;
    let mut c = Cursor::new(ctx.data, pos.checked_add(1)?);
    let record = match kind {
        Kind::Wire => Record::Wire(wire(&mut c, offset)?),
        Kind::Component => Record::Component(component(ctx, &mut c, offset, tag)?),
        Kind::Node => {
            let at = Coord::read(&mut c)?;
            let links = [ctx.link(&mut c)?, ctx.link(&mut c)?, ctx.link(&mut c)?, ctx.link(&mut c)?];
            Record::Node(Node { offset, at, links })
        }
        Kind::Terminal => Record::Terminal(terminal(ctx, &mut c, offset)?),
        Kind::Generator => Record::Generator(generator(ctx, &mut c, offset, tag)?),
    };
    Some((record, c.pos()))
}

fn wire(c: &mut Cursor<'_>, offset: u32) -> Option<Wire> {
    c.skip(STYLE_BLOCK_LEN)?;
    let style = c.cstr(NAME_MAX)?;
    let _ = c.u16()?;
    let count = usize::from(c.u16()?);
    if !is_name(&style, NAME_MAX) || count == 0 {
        return None;
    }
    let points = (0..count).map(|_| Coord::read(c)).collect::<Option<Vec<_>>>()?;
    Some(Wire { offset, style, points })
}

fn component(ctx: &Ctx<'_>, c: &mut Cursor<'_>, offset: u32, tag: u8) -> Option<Component> {
    let fields = read_fields(c, ctx.styles)?;
    if fields.first()?.style != REFERENCE_STYLE {
        return None;
    }
    let pins = usize::from(c.u16()?);
    let device = c.pstr()?;
    if pins > MAX_PINS || !is_name(&device, NAME_MAX) {
        return None;
    }
    let at = Coord::read(c)?;
    let placement = c.bytes(PLACEMENT_LEN)?;
    let ordinal = Cursor::new(placement, 4).u32()?;
    let pins = (0..pins).map(|_| ctx.link(c)).collect::<Option<Vec<_>>>()?;
    Some(Component { offset, tag, device, at, ordinal, fields, pins })
}

fn terminal(ctx: &Ctx<'_>, c: &mut Cursor<'_>, offset: u32) -> Option<Terminal> {
    let at = Coord::read(c)?;
    c.skip(4)?;
    let symbol = c.pstr()?;
    c.skip(TERMINAL_GAP)?;
    let fields = read_fields(c, ctx.styles)?;
    let link = ctx.link(c)?;
    (is_name(&symbol, NAME_MAX) && !fields.is_empty()).then_some(Terminal { offset, at, symbol, fields, link })
}

fn generator(ctx: &Ctx<'_>, c: &mut Cursor<'_>, offset: u32, tag: u8) -> Option<Generator> {
    let at = Coord::read(c)?;
    c.skip(6)?;
    let name = c.pstr()?;
    let symbol = c.pstr()?;
    if !is_name(&symbol, NAME_MAX) || name.bytes().any(|b| !(0x20..0x7f).contains(&b)) {
        return None;
    }
    let fields = read_fields(c, ctx.styles)?;
    if fields.is_empty() {
        return None;
    }
    c.skip(2)?;
    let links = [ctx.link(c)?, ctx.link(c)?];
    if tag & 0x08 != 0 {
        c.skip(1)?;
        let len = usize::try_from(c.u32()?).ok()?;
        if len > MAX_TEXT_BYTES {
            return None;
        }
        c.skip(len.checked_sub(4)?)?;
    }
    Some(Generator { offset, tag, at, name, symbol, fields, links })
}
