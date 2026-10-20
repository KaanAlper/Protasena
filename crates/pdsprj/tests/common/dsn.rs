//! A writer for synthetic `ROOT.DSN` files: the observed structure, our own content.
//!
//! Records are appended to one buffer, so each builder returns the record's absolute offset,
//! which later records use as a link.

pub const UNIT: i32 = 2540; // file units per thou

/// Text styles our test designs declare (the reader checks fields against them).
pub const TEXT_STYLES: &[&str] = &["COMPONENT ID", "COMPONENT VALUE", "SUBCKT NAME", "PROPERTIES", "TERMINAL LABEL", "GENERATOR LABEL"];
pub const GRAPHIC_STYLES: &[&str] = &["COMPONENT", "WIRE", "BUS WIRE"];

pub fn th(x: i32, y: i32) -> (i32, i32) {
    (x * UNIT, y * UNIT)
}

/// A text field. Long texts (property blocks) use the continuation form.
pub fn field(text: &str, at: (i32, i32), style: &str, visible: bool, continued: bool) -> Vec<u8> {
    let short = if continued { "" } else { text };
    let mut b = vec![0xff, short.len() as u8];
    b.extend(short.as_bytes());
    b.extend(at.0.to_le_bytes());
    b.extend(at.1.to_le_bytes());
    let mut attrs = [0u8; 26];
    if continued {
        attrs[4] = 4;
    }
    attrs[10..14].copy_from_slice(&254_000u32.to_le_bytes());
    attrs[25] = u8::from(visible);
    b.extend(attrs);
    b.extend(b"Test Font\0");
    b.extend(style.as_bytes());
    b.extend([0u8; 5]);
    if continued {
        b.extend((text.len() as u32).to_le_bytes());
        b.extend(text.as_bytes());
    }
    b
}

pub struct Dsn {
    pub buf: Vec<u8>,
}

impl Dsn {
    /// Header and configuration with our styles, one symbol and the given device definitions.
    pub fn new(devices: &[(&str, &str)]) -> Self {
        let mut buf = b"ISIS SCHEMATIC FILE\x1a".to_vec();
        buf.resize(0x104, 0);
        buf[0x6f..0x77].copy_from_slice(b"01/02/03");
        buf[0x8b..0x93].copy_from_slice(b"04/05/06");
        buf[0xa7..0xa9].copy_from_slice(&901u16.to_le_bytes());
        buf[0xa9..0xab].copy_from_slice(&902u16.to_le_bytes());
        buf[0xad..0xb1].copy_from_slice(&1_000u32.to_le_bytes());
        buf[0xb1..0xb5].copy_from_slice(&2_000u32.to_le_bytes());
        buf.extend(b"DESIGN CONFIGURATION DATA\x1a");
        buf.extend(vec![0u8; 0x2be]);
        for list in [GRAPHIC_STYLES, TEXT_STYLES] {
            buf.extend((list.len() as u16).to_le_bytes());
            for name in list {
                pstr(&mut buf, name);
                buf.push(1);
                buf.extend(10u16.to_le_bytes());
                buf.extend([0u8; 8]);
            }
        }
        buf.extend([0xfe, 0xff]);
        entry(&mut buf, "$TESTSYM", &[7u8; 40]);
        buf.push(0);
        buf.extend([0xfe, 0xff]);
        for (name, text) in devices {
            let mut body = 12u32.to_le_bytes().to_vec();
            body.extend([0u8; 4]);
            body.extend((text.len() as u32 + 1).to_le_bytes());
            body.extend(text.as_bytes());
            body.push(0);
            entry(&mut buf, name, &body);
        }
        buf.push(0);
        buf.extend([0u8; 4]);
        Self { buf }
    }

    pub fn pos(&self) -> u32 {
        self.buf.len() as u32
    }

    /// Starts a circuit section with a 10 in × 7 in sheet.
    pub fn circuit(&mut self) {
        self.buf.extend(b"ISIS CIRCUIT FILE\x1a");
        self.buf.extend([0, 1, 0, 1, 0, 1, 0]);
        self.buf.extend((10_000 * UNIT).to_le_bytes());
        self.buf.extend((7_000 * UNIT).to_le_bytes());
        self.buf.extend([0, 0]);
        self.buf.extend(b"OBJECT DATA\0");
    }

    /// Ends the object list and adds a trailer.
    pub fn end(&mut self, trailer: &[u8]) {
        self.buf.push(0xff);
        self.buf.extend(trailer);
    }

    pub fn wire(&mut self, style: &str, points: &[(i32, i32)]) -> u32 {
        let at = self.pos();
        self.buf.push(0);
        self.buf.push(0x1d);
        self.buf.extend([0u8; 21]);
        self.buf.extend([0x02, 0x7f]);
        self.buf.extend(style.as_bytes());
        self.buf.extend([0, 0, 0]);
        self.buf.extend((points.len() as u16).to_le_bytes());
        for (x, y) in points {
            self.buf.extend(x.to_le_bytes());
            self.buf.extend(y.to_le_bytes());
        }
        at
    }

    pub fn node(&mut self, at: (i32, i32), links: [u32; 4]) -> u32 {
        let pos = self.pos();
        self.buf.push(0x01);
        self.xy(at);
        links.iter().for_each(|l| self.buf.extend(l.to_le_bytes()));
        pos
    }

    /// A component: reference, value, device-name and property fields, then the placement.
    pub fn component(&mut self, reference: &str, value: &str, device: &str, at: (i32, i32), props: &str, pins: &[u32]) -> u32 {
        let pos = self.pos();
        self.buf.push(0);
        self.buf.extend(field(reference, (at.0, at.1 + 200 * UNIT), "COMPONENT ID", true, false));
        self.buf.extend(field(value, (at.0, at.1 - 200 * UNIT), "COMPONENT VALUE", true, false));
        self.buf.extend(field(device, at, "SUBCKT NAME", false, false));
        self.buf.extend(field(props, at, "PROPERTIES", true, true));
        self.buf.extend((pins.len() as u16).to_le_bytes());
        pstr(&mut self.buf, device);
        self.xy(at);
        let mut placement = [0u8; 17];
        placement[4] = 3;
        self.buf.extend(placement);
        pins.iter().for_each(|l| self.buf.extend(l.to_le_bytes()));
        pos
    }

    pub fn terminal(&mut self, at: (i32, i32), symbol: &str, label: &str, link: u32) -> u32 {
        let pos = self.pos();
        self.buf.push(0x10);
        self.xy(at);
        self.buf.extend([0u8; 4]);
        pstr(&mut self.buf, symbol);
        self.buf.extend([0u8; 6]);
        self.buf.extend(field(label, at, "TERMINAL LABEL", true, false));
        self.buf.extend(link.to_le_bytes());
        pos
    }

    /// A generator (tag 0x38: with its settings blob) or probe (tag 0x30).
    pub fn generator(&mut self, tag: u8, at: (i32, i32), symbol: &str, label: &str, link: u32) -> u32 {
        let pos = self.pos();
        self.buf.push(tag);
        self.xy(at);
        self.buf.extend([0u8; 6]);
        pstr(&mut self.buf, "G0001");
        pstr(&mut self.buf, symbol);
        self.buf.extend(field(label, at, "GENERATOR LABEL", true, false));
        self.buf.extend(field("{LEVEL=1}\n", at, "PROPERTIES", true, true));
        self.buf.extend([0, 0]);
        self.buf.extend(link.to_le_bytes());
        self.buf.extend(0u32.to_le_bytes());
        if tag & 0x08 != 0 {
            self.buf.push(0);
            self.buf.extend(7u32.to_le_bytes());
            self.buf.extend([1, 2, 3]);
        }
        pos
    }

    /// A record of a kind the reader does not know: our own bytes.
    pub fn opaque(&mut self, tag: u8, len: usize) -> u32 {
        let pos = self.pos();
        self.buf.push(tag);
        self.buf.extend((0..len).map(|i| (i * 7 % 251) as u8));
        pos
    }

    fn xy(&mut self, at: (i32, i32)) {
        self.buf.extend(at.0.to_le_bytes());
        self.buf.extend(at.1.to_le_bytes());
    }
}

fn pstr(buf: &mut Vec<u8>, s: &str) {
    buf.push(s.len() as u8);
    buf.extend(s.as_bytes());
}

fn entry(buf: &mut Vec<u8>, name: &str, body: &[u8]) {
    pstr(buf, name);
    buf.extend(0x1234_5678u32.to_le_bytes());
    buf.extend((body.len() as u32 + 4).to_le_bytes());
    buf.extend(body);
}

/// Our test device definition.
pub const TESTRES: &str =
    "{*DEVICE}\n{PREFIX=X}\n{*PROPDEFS}\n{RATING=\"Test rating\",STRING}\n{*INDEX}\n{CAT=Test parts}\n{SUBCAT=Fixed}\n{*COMPONENT}\n{VALUE=1k}\n";

/// A design with one sheet: two components joined by wires at a node, a terminal, a generator,
/// a probe, an unknown record between them, and a second circuit with only a trailer.
pub fn sample_design() -> Vec<u8> {
    let mut d = Dsn::new(&[("TESTRES", TESTRES)]);
    d.circuit();
    let w1 = d.wire("WIRE", &[th(-100, 0), th(-100, 100), th(0, 100)]);
    let w2 = d.wire("WIRE", &[th(100, 100), th(0, 100)]);
    let w3 = d.wire("WIRE", &[th(0, 100), th(0, 300)]);
    d.node(th(0, 100), [w1, w2, w3, 0]);
    d.component("X1", "4k7", "TESTRES", th(-100, -100), "{RATING=0.5W}\n{CODE=AB}\n", &[w1, 0]);
    d.opaque(0x28, 300);
    d.component("X2", "10k", "TESTRES", th(100, -100), "", &[w2, 0]);
    d.terminal(th(0, 300), "$TERGROUND", "", w3);
    d.generator(0x38, th(300, 0), "$TESTGEN", "Vtest", w2);
    d.generator(0x30, th(400, 0), "$TESTPROBE", "P1", 0);
    d.wire("BUS WIRE", &[th(500, 0), th(500, 500)]);
    d.end(&[]);
    d.circuit();
    d.end(&[9u8; 50]);
    d.buf
}
