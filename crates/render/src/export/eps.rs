//! Encapsulated PostScript (level 2, one page): the bounding box in points, drawing in thou
//! after one `scale`, texts in Helvetica aligned with `stringwidth` (characters outside
//! printable ASCII are written as `?`).

use std::fmt::Write as _;

use super::{Options, Page, colour, drawn, h_shift, line_width, num, polyline, text_lines};
use crate::prim::{Prim, Shape};

/// PostScript points per thou.
const PT_PER_THOU: f64 = 0.072;

/// The EPS file of a page.
pub(super) fn write(page: &Page, o: Options) -> String {
    let a = page.area;
    let k = PT_PER_THOU * o.scale;
    let (w, h) = (a.width() * k, a.height() * k);
    let mut out = String::new();
    let _ = writeln!(out, "%!PS-Adobe-3.0 EPSF-3.0");
    let _ = writeln!(out, "%%BoundingBox: 0 0 {} {}", w.ceil(), h.ceil());
    let _ = writeln!(out, "%%HiResBoundingBox: 0 0 {} {}", num(w), num(h));
    let _ = writeln!(out, "%%Title: ({})", ps_string(&page.title));
    let _ = writeln!(out, "%%Creator: Protasena");
    let _ = writeln!(out, "%%LanguageLevel: 2");
    let _ = writeln!(out, "%%Pages: 1");
    let _ = writeln!(out, "%%EndComments");
    let _ = writeln!(out, "%%Page: 1 1");
    let _ = writeln!(out, "gsave 1 setgray 0 0 {} {} rectfill grestore", num(w), num(h));
    let _ = writeln!(out, "gsave {} {} scale {} {} translate 1 setlinecap 1 setlinejoin", num(k), num(k), num(-a.min.x), num(-a.min.y));
    for p in drawn(page) {
        prim(&mut out, p, o.mono);
    }
    out.push_str("grestore\nshowpage\n%%EOF\n");
    out
}

fn prim(out: &mut String, p: &Prim, mono: bool) {
    let (r, g, b) = colour(p.ink, mono).unit();
    let ink = format!("{} {} {} setrgbcolor", num(r), num(g), num(b));
    let fill = p.fill.map(|f| {
        let (r, g, b) = colour(f, mono).unit();
        format!("{} {} {} setrgbcolor", num(r), num(g), num(b))
    });
    let _ = writeln!(out, "{} setlinewidth", num(line_width(p)));
    let paint = |out: &mut String, closed: bool| {
        if closed {
            out.push_str(" closepath");
        }
        match (&fill, closed) {
            (Some(f), true) => {
                let _ = writeln!(out, " gsave {f} fill grestore {ink} stroke");
            }
            _ => {
                let _ = writeln!(out, " {ink} stroke");
            }
        }
    };
    match &p.shape {
        Shape::Circle { center, radius } => {
            let _ = write!(out, "newpath {} {} {} 0 360 arc", num(center.x), num(center.y), num(radius.abs()));
            paint(out, true);
        }
        Shape::Text { at, text, size, angle, h, v } => {
            let _ = writeln!(out, "{ink} /Helvetica findfont {} scalefont setfont", num(*size));
            for line in text_lines(*at, text, *size, *angle, *v) {
                let _ = writeln!(
                    out,
                    "gsave {} {} translate {} rotate 0 0 moveto ({}) dup stringwidth pop {} mul 0 rmoveto show grestore",
                    num(line.at.x),
                    num(line.at.y),
                    num(*angle),
                    ps_string(&line.text),
                    num(-h_shift(*h))
                );
            }
        }
        shape => {
            let Some((points, closed)) = polyline(shape) else { return };
            let (Some(first), true) = (points.first(), points.len() >= 2) else { return };
            let _ = write!(out, "newpath {} {} moveto", num(first.x), num(first.y));
            for q in points.iter().skip(1) {
                let _ = write!(out, " {} {} lineto", num(q.x), num(q.y));
            }
            paint(out, closed);
        }
    }
}

/// A PostScript string's contents: printable ASCII with `(`, `)` and `\` escaped, others `?`.
fn ps_string(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars().take(super::MAX_TEXT_CHARS) {
        match c {
            '(' | ')' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            ' '..='~' => out.push(c),
            _ => out.push('?'),
        }
    }
    out
}
