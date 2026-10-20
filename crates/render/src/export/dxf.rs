//! DXF (AutoCAD R12, ASCII), coordinates in thou (`$INSUNITS` 9, mils): one layer per kind of
//! ink, lines and outlines as `POLYLINE`s, circles as `CIRCLE`s, texts as `TEXT` with their
//! alignment. Fills are not written (outlines only).

use std::fmt::Write as _;

use super::{Page, drawn, num, polyline, text_lines};
use crate::prim::{HAlign, Ink, Prim, Shape};

/// Cap height of a text relative to its height.
const CAP: f64 = 0.7;

/// Layers: name and AutoCAD colour index, for each ink.
fn layer(ink: Ink) -> (&'static str, u8) {
    match ink {
        Ink::Body | Ink::BodyFill | Ink::BodyText | Ink::Pin | Ink::Missing | Ink::Graphic | Ink::Subcircuit | Ink::Port => ("BODY", 1),
        Ink::PinName | Ink::PinNumber => ("PINTEXT", 5),
        Ink::Reference | Ink::Value | Ink::Text => ("TEXT", 7),
        Ink::Wire | Ink::Junction => ("WIRES", 3),
        Ink::Bus => ("BUSES", 5),
        Ink::Label => ("LABELS", 4),
        Ink::Terminal | Ink::TerminalText => ("TERMINALS", 6),
        Ink::SheetBorder => ("BORDER", 8),
        Ink::Pad | Ink::Silk => ("PCB", 2),
    }
}

const LAYERS: [Ink; 9] = [Ink::Body, Ink::PinName, Ink::Text, Ink::Wire, Ink::Bus, Ink::Label, Ink::Terminal, Ink::SheetBorder, Ink::Pad];

/// The DXF file of a page.
pub(super) fn write(page: &Page) -> String {
    let mut out = String::new();
    let a = page.area;
    group(&mut out, 0, "SECTION");
    group(&mut out, 2, "HEADER");
    group(&mut out, 9, "$ACADVER");
    group(&mut out, 1, "AC1009");
    group(&mut out, 9, "$INSUNITS");
    group(&mut out, 70, "9");
    group(&mut out, 9, "$EXTMIN");
    point(&mut out, 10, a.min.x, a.min.y);
    group(&mut out, 9, "$EXTMAX");
    point(&mut out, 10, a.max.x, a.max.y);
    group(&mut out, 0, "ENDSEC");
    group(&mut out, 0, "SECTION");
    group(&mut out, 2, "TABLES");
    group(&mut out, 0, "TABLE");
    group(&mut out, 2, "LAYER");
    group(&mut out, 70, &LAYERS.len().to_string());
    for ink in LAYERS {
        let (name, colour) = layer(ink);
        group(&mut out, 0, "LAYER");
        group(&mut out, 2, name);
        group(&mut out, 70, "0");
        group(&mut out, 62, &colour.to_string());
        group(&mut out, 6, "CONTINUOUS");
    }
    group(&mut out, 0, "ENDTAB");
    group(&mut out, 0, "ENDSEC");
    group(&mut out, 0, "SECTION");
    group(&mut out, 2, "ENTITIES");
    for p in drawn(page) {
        entity(&mut out, p);
    }
    group(&mut out, 0, "ENDSEC");
    group(&mut out, 0, "EOF");
    out
}

fn entity(out: &mut String, p: &Prim) {
    let (name, _) = layer(p.ink);
    match &p.shape {
        Shape::Circle { center, radius } => {
            group(out, 0, "CIRCLE");
            group(out, 8, name);
            point(out, 10, center.x, center.y);
            group(out, 40, &num(radius.abs()));
        }
        Shape::Text { at, text, size, angle, h, v } => {
            let justify = match h {
                HAlign::Left => 0,
                HAlign::Center => 1,
                HAlign::Right => 2,
            };
            for line in text_lines(*at, text, *size, *angle, *v) {
                if line.text.trim().is_empty() {
                    continue;
                }
                group(out, 0, "TEXT");
                group(out, 8, name);
                point(out, 10, line.at.x, line.at.y);
                group(out, 40, &num(size * CAP));
                group(out, 1, &clean(&line.text));
                group(out, 50, &num(*angle));
                if justify != 0 {
                    group(out, 72, &justify.to_string());
                    point(out, 11, line.at.x, line.at.y);
                }
            }
        }
        shape => {
            let Some((points, closed)) = polyline(shape) else { return };
            if points.len() < 2 {
                return;
            }
            group(out, 0, "POLYLINE");
            group(out, 8, name);
            group(out, 66, "1");
            point(out, 10, 0.0, 0.0);
            group(out, 70, if closed { "1" } else { "0" });
            for q in &points {
                group(out, 0, "VERTEX");
                group(out, 8, name);
                point(out, 10, q.x, q.y);
            }
            group(out, 0, "SEQEND");
            group(out, 8, name);
        }
    }
}

fn group(out: &mut String, code: u16, value: &str) {
    let _ = writeln!(out, "{code:>3}\n{value}");
}

fn point(out: &mut String, code: u16, x: f64, y: f64) {
    group(out, code, &num(x));
    group(out, code + 10, &num(y));
    group(out, code + 20, "0");
}

/// A text value on one line: control characters dropped, non-ASCII as `?` (R12 files are
/// read in the reader's code page).
fn clean(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).map(|c| if c.is_ascii() { c } else { '?' }).collect()
}
