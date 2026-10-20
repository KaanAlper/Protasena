//! PDF 1.4: one page per sheet, the page the size of the area (times the scale), content
//! streams compressed with Flate, texts in the standard Helvetica font (not embedded, WinAnsi
//! encoding; characters outside it are written as `?`).

use std::fmt::Write as _;

use super::{Options, Page, colour, drawn, h_shift, line_width, num, polyline, text_lines};
use crate::prim::{Prim, Shape};

/// PDF points per thou.
const PT_PER_THOU: f64 = 0.072;

/// Bézier handle length of a quarter circle, relative to the radius.
const KAPPA: f64 = 0.552_284_75;

/// The PDF file of `pages`.
pub(super) fn write(pages: &[Page], o: Options) -> Vec<u8> {
    let mut doc = Doc::default();
    let n = pages.len();
    // Objects: 1 catalog, 2 page tree, 3 font, 4 info, then a page and its content per page.
    let page_ids: Vec<usize> = (0..n).map(|i| 5 + 2 * i).collect();
    doc.object(1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let kids: String = page_ids.iter().map(|id| format!("{id} 0 R ")).collect();
    doc.object(2, format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.trim_end()).into_bytes());
    doc.object(3, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec());
    let title = pages.first().map(|p| p.title.as_str()).unwrap_or_default();
    doc.object(4, format!("<< /Title ({}) /Producer (Protasena) >>", string(title)).into_bytes());
    for (page, id) in pages.iter().zip(&page_ids) {
        let k = PT_PER_THOU * o.scale;
        let (w, h) = (page.area.width() * k, page.area.height() * k);
        let stream = content(page, o, k);
        let packed = miniz_oxide::deflate::compress_to_vec_zlib(stream.as_bytes(), 6);
        doc.object(
            *id,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
                num(w),
                num(h),
                id + 1
            )
            .into_bytes(),
        );
        let mut body = format!("<< /Length {} /Filter /FlateDecode >>\nstream\n", packed.len()).into_bytes();
        body.extend_from_slice(&packed);
        body.extend_from_slice(b"\nendstream");
        doc.object(id + 1, body);
    }
    doc.finish(4 + 2 * n)
}

/// The drawing commands of one page, in thou after one `cm` that maps the area to the page.
fn content(page: &Page, o: Options, k: f64) -> String {
    let a = page.area;
    let mut out = String::new();
    let _ = writeln!(out, "1 1 1 rg 0 0 {} {} re f", num(a.width() * k), num(a.height() * k));
    let _ = writeln!(out, "{} 0 0 {} {} {} cm 1 J 1 j", num(k), num(k), num(-a.min.x * k), num(-a.min.y * k));
    for p in drawn(page) {
        prim(&mut out, p, o.mono);
    }
    out
}

fn prim(out: &mut String, p: &Prim, mono: bool) {
    let (r, g, b) = colour(p.ink, mono).unit();
    let _ = writeln!(out, "{} {} {} RG {} {} {} rg {} w", num(r), num(g), num(b), num(r), num(g), num(b), num(line_width(p)));
    if let Some(f) = p.fill {
        let (r, g, b) = colour(f, mono).unit();
        let _ = writeln!(out, "{} {} {} rg", num(r), num(g), num(b));
    }
    let paint = |closed: bool| match (closed, p.fill.is_some()) {
        (true, true) => "b",
        (true, false) => "s",
        (false, _) => "S",
    };
    match &p.shape {
        Shape::Circle { center, radius } => {
            let (cx, cy, r) = (center.x, center.y, radius.abs());
            let c = r * KAPPA;
            let _ = writeln!(out, "{} {} m", num(cx + r), num(cy));
            for (x1, y1, x2, y2, x, y) in [
                (cx + r, cy + c, cx + c, cy + r, cx, cy + r),
                (cx - c, cy + r, cx - r, cy + c, cx - r, cy),
                (cx - r, cy - c, cx - c, cy - r, cx, cy - r),
                (cx + c, cy - r, cx + r, cy - c, cx + r, cy),
            ] {
                let _ = writeln!(out, "{} {} {} {} {} {} c", num(x1), num(y1), num(x2), num(y2), num(x), num(y));
            }
            let _ = writeln!(out, "{}", paint(true));
        }
        Shape::Text { at, text, size, angle, h, v } => {
            let (r, g, b) = colour(p.ink, mono).unit();
            let (sin, cos) = angle.to_radians().sin_cos();
            for line in text_lines(*at, text, *size, *angle, *v) {
                let width = helvetica_width(&line.text) * size / 1000.0;
                let back = -width * h_shift(*h);
                let (x, y) = (line.at.x + back * cos, line.at.y + back * sin);
                let _ = writeln!(
                    out,
                    "BT {} {} {} rg /F1 {} Tf {} {} {} {} {} {} Tm ({}) Tj ET",
                    num(r),
                    num(g),
                    num(b),
                    num(*size),
                    num(cos),
                    num(sin),
                    num(-sin),
                    num(cos),
                    num(x),
                    num(y),
                    string(&line.text)
                );
            }
        }
        shape => {
            let Some((points, closed)) = polyline(shape) else { return };
            let (Some(first), true) = (points.first(), points.len() >= 2) else { return };
            let _ = write!(out, "{} {} m", num(first.x), num(first.y));
            for q in points.iter().skip(1) {
                let _ = write!(out, " {} {} l", num(q.x), num(q.y));
            }
            let _ = writeln!(out, " {}", paint(closed));
        }
    }
}

/// The WinAnsi byte of a character, `?` when it has none (Latin-1 above 0xA0 maps to itself).
fn win_ansi(c: char) -> u8 {
    match u32::from(c) {
        x @ 0x20..=0x7e | x @ 0xa0..=0xff => u8::try_from(x).unwrap_or(b'?'),
        _ => b'?',
    }
}

/// A PDF string's contents: WinAnsi bytes with `(`, `)` and `\` escaped, others above 0x7e as
/// octal escapes.
fn string(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars().take(super::MAX_TEXT_CHARS) {
        match win_ansi(c) {
            b'(' => out.push_str("\\("),
            b')' => out.push_str("\\)"),
            b'\\' => out.push_str("\\\\"),
            b @ 0x20..=0x7e => out.push(char::from(b)),
            b => {
                let _ = write!(out, "\\{b:03o}");
            }
        }
    }
    out
}

/// Advance widths of Helvetica for the printable ASCII characters 0x20–0x7e, in 1/1000 em.
const HELVETICA: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, // space – /
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, // 0 – ?
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, // @ – O
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, // P – _
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, // ` – o
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584, // p – ~
];

/// The width of a line of text in Helvetica, in 1/1000 of the font size.
fn helvetica_width(s: &str) -> f64 {
    s.chars()
        .take(super::MAX_TEXT_CHARS)
        .map(|c| {
            let code = usize::from(win_ansi(c));
            code.checked_sub(0x20).and_then(|i| HELVETICA.get(i)).copied().unwrap_or(556)
        })
        .map(f64::from)
        .sum()
}

/// A PDF file being built: numbered objects and their byte offsets.
#[derive(Default)]
struct Doc {
    bytes: Vec<u8>,
    offsets: Vec<(usize, usize)>,
}

impl Doc {
    fn object(&mut self, id: usize, body: Vec<u8>) {
        if self.bytes.is_empty() {
            self.bytes.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");
        }
        self.offsets.push((id, self.bytes.len()));
        self.bytes.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        self.bytes.extend_from_slice(&body);
        self.bytes.extend_from_slice(b"\nendobj\n");
    }

    fn finish(mut self, last: usize) -> Vec<u8> {
        self.offsets.sort_unstable();
        let xref = self.bytes.len();
        let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", last + 1);
        for id in 1..=last {
            let at = self.offsets.iter().find(|(i, _)| *i == id).map_or(0, |(_, o)| *o);
            let _ = writeln!(table, "{at:010} 00000 n ");
        }
        let _ = write!(table, "trailer\n<< /Size {} /Root 1 0 R /Info 4 0 R >>\nstartxref\n{xref}\n%%EOF\n", last + 1);
        self.bytes.extend_from_slice(table.as_bytes());
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_are_escaped_and_widths_known() {
        assert_eq!(string("a(b)\\c"), "a\\(b\\)\\\\c");
        assert_eq!(string("10µF Ω"), "10\\265F ?");
        assert_eq!(helvetica_width("AB"), 1334.0);
        assert_eq!(helvetica_width("~"), 584.0);
    }
}
