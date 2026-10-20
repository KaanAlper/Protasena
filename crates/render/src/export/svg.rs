//! SVG: one `<svg>` sized in inches with a view box in thou (y down, so sheet y is flipped),
//! lines and outlines as `<path>`, circles as `<circle>`, texts as `<text>` in Helvetica.

use std::fmt::Write as _;

use protasena_geom::Pt;

use super::{Options, Page, Rgb, colour, drawn, line_width, num, polyline, text_lines};
use crate::prim::{HAlign, Prim, Shape};

/// The SVG document of a page.
pub(super) fn write(page: &Page, o: Options) -> String {
    let a = page.area;
    let (w, h) = (a.width(), a.height());
    let mut out = String::new();
    let _ = writeln!(out, r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}in" height="{}in" viewBox="0 0 {} {}">"#,
        num(w * o.scale / 1000.0),
        num(h * o.scale / 1000.0),
        num(w),
        num(h)
    );
    let _ = writeln!(out, "<title>{}</title>", escape(&page.title));
    let _ = writeln!(out, r#"<rect x="0" y="0" width="{}" height="{}" fill="{}"/>"#, num(w), num(h), Rgb::WHITE.hex());
    let _ = writeln!(out, r#"<g stroke-linecap="round" stroke-linejoin="round">"#);
    let to = |p: Pt| (p.x - a.min.x, a.max.y - p.y);
    for p in drawn(page) {
        element(&mut out, p, o.mono, &to);
    }
    out.push_str("</g>\n</svg>\n");
    out
}

fn element(out: &mut String, p: &Prim, mono: bool, to: &impl Fn(Pt) -> (f64, f64)) {
    let stroke = colour(p.ink, mono).hex();
    let fill = p.fill.map_or_else(|| "none".to_owned(), |f| colour(f, mono).hex());
    let width = num(line_width(p));
    match &p.shape {
        Shape::Circle { center, radius } => {
            let (x, y) = to(*center);
            let _ = writeln!(
                out,
                r#"<circle cx="{}" cy="{}" r="{}" fill="{fill}" stroke="{stroke}" stroke-width="{width}"/>"#,
                num(x),
                num(y),
                num(radius.abs())
            );
        }
        Shape::Text { at, text, size, angle, h, v } => {
            let anchor = match h {
                HAlign::Left => "start",
                HAlign::Center => "middle",
                HAlign::Right => "end",
            };
            for line in text_lines(*at, text, *size, *angle, *v) {
                let (x, y) = to(line.at);
                let _ = writeln!(
                    out,
                    r#"<text x="{x}" y="{y}" transform="rotate({r} {x} {y})" font-family="Helvetica, Arial, sans-serif" font-size="{s}" text-anchor="{anchor}" fill="{stroke}">{t}</text>"#,
                    x = num(x),
                    y = num(y),
                    r = num(-angle),
                    s = num(*size),
                    t = escape(&line.text)
                );
            }
        }
        shape => {
            let Some((points, closed)) = polyline(shape) else { return };
            if points.len() < 2 {
                return;
            }
            let mut d = String::new();
            for (i, q) in points.iter().enumerate() {
                let (x, y) = to(*q);
                let _ = write!(d, "{}{} {} ", if i == 0 { "M" } else { "L" }, num(x), num(y));
            }
            if closed {
                d.push('Z');
            }
            let fill = if closed { fill } else { "none".to_owned() };
            let _ = writeln!(out, r#"<path d="{}" fill="{fill}" stroke="{stroke}" stroke-width="{width}"/>"#, d.trim_end());
        }
    }
}

/// Text escaped for XML content and attributes; control characters are dropped.
pub(super) fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}
