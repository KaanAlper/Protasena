//! Protasena's stroke font for bitmaps: every printable ASCII character (and `µ`, `Ω`, `°`,
//! `±`) as strokes on a grid four units wide, the capitals six units high on the baseline,
//! descenders two units below. Drawn in code by Protasena contributors (MIT OR Apache-2.0); a
//! character it lacks is drawn as a box.
//!
//! A glyph is written as strokes separated by `;`, each stroke as points `x,y` separated by
//! spaces.

use protasena_geom::{Pt, Vec2};

use super::{BASELINE, h_shift};
use crate::geometry::CHAR_WIDTH;
use crate::prim::{HAlign, VAlign};

/// Grid units of a capital's height.
const CAP_UNITS: f64 = 6.0;

/// Width of a glyph in grid units.
const GLYPH_UNITS: f64 = 4.0;

/// Capital height relative to the text height.
const CAP: f64 = 0.7;

/// Stroke width relative to the text height.
pub const STROKE: f64 = 0.09;

/// The glyph of a character.
fn glyph(c: char) -> &'static str {
    match c {
        ' ' => "",
        '!' => "2,6 2,2;2,0.4 2,0",
        '"' => "1,6 1,5;3,6 3,5",
        '#' => "1,5 1,1;3,5 3,1;0,4 4,4;0,2 4,2",
        '$' => "4,5 3,6 1,6 0,5 0,4 1,3 3,3 4,2 4,1 3,0 1,0 0,1;2,7 2,-1",
        '%' => "0,0 4,6;0,6 1,6 1,5 0,5 0,6;3,1 4,1 4,0 3,0 3,1",
        '&' => "4,0 1,4 1,5 2,6 3,5 3,4 0,2 0,1 1,0 2,0 4,2",
        '\'' => "2,6 2,5",
        '(' => "3,7 2,6 2,0 3,-1",
        ')' => "1,7 2,6 2,0 1,-1",
        '*' => "2,5 2,1;0,4 4,2;0,2 4,4",
        '+' => "2,5 2,1;0,3 4,3",
        ',' => "2,0.5 2,0 1,-1",
        '-' => "0,3 4,3",
        '.' => "2,0.4 2,0",
        '/' => "0,0 4,6",
        '0' => "1,0 0,1 0,5 1,6 3,6 4,5 4,1 3,0 1,0;4,5 0,1",
        '1' => "1,5 2,6 2,0;1,0 3,0",
        '2' => "0,5 1,6 3,6 4,5 4,4 0,0 4,0",
        '3' => "0,5 1,6 3,6 4,5 4,4 3,3 1,3;3,3 4,2 4,1 3,0 1,0 0,1",
        '4' => "3,0 3,6 0,2 4,2",
        '5' => "4,6 0,6 0,3 3,3 4,2 4,1 3,0 1,0 0,1",
        '6' => "4,5 3,6 1,6 0,5 0,1 1,0 3,0 4,1 4,2 3,3 0,3",
        '7' => "0,6 4,6 1,0",
        '8' => "1,3 0,4 0,5 1,6 3,6 4,5 4,4 3,3 1,3 0,2 0,1 1,0 3,0 4,1 4,2 3,3",
        '9' => "0,1 1,0 3,0 4,1 4,5 3,6 1,6 0,5 0,4 1,3 4,3",
        ':' => "2,4 2,3.6;2,0.4 2,0",
        ';' => "2,4 2,3.6;2,0.5 2,0 1,-1",
        '<' => "4,5 0,3 4,1",
        '=' => "0,4 4,4;0,2 4,2",
        '>' => "0,5 4,3 0,1",
        '?' => "0,5 1,6 3,6 4,5 4,4 2,3 2,2;2,0.4 2,0",
        '@' => "3,2 3,4 1,4 1,2 3,2 4,3 4,5 3,6 1,6 0,5 0,1 1,0 4,0",
        'A' => "0,0 0,4 2,6 4,4 4,0;0,3 4,3",
        'B' => "0,0 0,6 3,6 4,5 4,4 3,3 0,3;3,3 4,2 4,1 3,0 0,0",
        'C' => "4,5 3,6 1,6 0,5 0,1 1,0 3,0 4,1",
        'D' => "0,0 0,6 2,6 4,4 4,2 2,0 0,0",
        'E' => "4,6 0,6 0,0 4,0;0,3 3,3",
        'F' => "4,6 0,6 0,0;0,3 3,3",
        'G' => "4,5 3,6 1,6 0,5 0,1 1,0 3,0 4,1 4,3 2,3",
        'H' => "0,0 0,6;4,0 4,6;0,3 4,3",
        'I' => "1,6 3,6;2,6 2,0;1,0 3,0",
        'J' => "4,6 4,1 3,0 1,0 0,1",
        'K' => "0,0 0,6;4,6 0,2;1,3 4,0",
        'L' => "0,6 0,0 4,0",
        'M' => "0,0 0,6 2,3 4,6 4,0",
        'N' => "0,0 0,6 4,0 4,6",
        'O' => "1,0 0,1 0,5 1,6 3,6 4,5 4,1 3,0 1,0",
        'P' => "0,0 0,6 3,6 4,5 4,4 3,3 0,3",
        'Q' => "1,0 0,1 0,5 1,6 3,6 4,5 4,1 3,0 1,0;2,2 4,0",
        'R' => "0,0 0,6 3,6 4,5 4,4 3,3 0,3;2,3 4,0",
        'S' => "4,5 3,6 1,6 0,5 0,4 1,3 3,3 4,2 4,1 3,0 1,0 0,1",
        'T' => "0,6 4,6;2,6 2,0",
        'U' => "0,6 0,1 1,0 3,0 4,1 4,6",
        'V' => "0,6 2,0 4,6",
        'W' => "0,6 1,0 2,3 3,0 4,6",
        'X' => "0,0 4,6;0,6 4,0",
        'Y' => "0,6 2,3 4,6;2,3 2,0",
        'Z' => "0,6 4,6 0,0 4,0",
        '[' => "3,7 2,7 2,-1 3,-1",
        '\\' => "0,6 4,0",
        ']' => "1,7 2,7 2,-1 1,-1",
        '^' => "1,5 2,6 3,5",
        '_' => "0,-1 4,-1",
        '`' => "1,6 2,5",
        'a' => "4,4 4,0;4,3 3,4 1,4 0,3 0,1 1,0 3,0 4,1",
        'b' => "0,6 0,0;0,3 1,4 3,4 4,3 4,1 3,0 1,0 0,1",
        'c' => "4,3 3,4 1,4 0,3 0,1 1,0 3,0 4,1",
        'd' => "4,6 4,0;4,3 3,4 1,4 0,3 0,1 1,0 3,0 4,1",
        'e' => "0,2 4,2 4,3 3,4 1,4 0,3 0,1 1,0 3,0 4,1",
        'f' => "4,6 3,6 2,5 2,0;1,4 3,4",
        'g' => "4,4 4,-1 3,-2 1,-2 0,-1;4,3 3,4 1,4 0,3 0,1 1,0 3,0 4,1",
        'h' => "0,6 0,0;0,3 1,4 3,4 4,3 4,0",
        'i' => "2,4 2,0;2,6 2,5.4",
        'j' => "3,4 3,-1 2,-2 1,-2 0,-1;3,6 3,5.4",
        'k' => "0,6 0,0;4,4 0,1;1,2 4,0",
        'l' => "1,6 2,6 2,1 3,0",
        'm' => "0,4 0,0;0,3 1,4 2,3 2,0;2,3 3,4 4,3 4,0",
        'n' => "0,4 0,0;0,3 1,4 3,4 4,3 4,0",
        'o' => "1,0 0,1 0,3 1,4 3,4 4,3 4,1 3,0 1,0",
        'p' => "0,4 0,-2;0,3 1,4 3,4 4,3 4,1 3,0 1,0 0,1",
        'q' => "4,4 4,-2;4,3 3,4 1,4 0,3 0,1 1,0 3,0 4,1",
        'r' => "0,4 0,0;0,3 1,4 3,4 4,3",
        's' => "4,3 3,4 1,4 0,3 1,2 3,2 4,1 3,0 1,0 0,1",
        't' => "2,6 2,1 3,0 4,0;0,4 4,4",
        'u' => "0,4 0,1 1,0 3,0 4,1;4,4 4,0",
        'v' => "0,4 2,0 4,4",
        'w' => "0,4 1,0 2,2 3,0 4,4",
        'x' => "0,4 4,0;0,0 4,4",
        'y' => "0,4 2,0;4,4 1,-2",
        'z' => "0,4 4,4 0,0 4,0",
        '{' => "3,7 2,6 2,4 1,3 2,2 2,0 3,-1",
        '|' => "2,7 2,-1",
        '}' => "1,7 2,6 2,4 3,3 2,2 2,0 1,-1",
        '~' => "0,3 1,4 3,2 4,3",
        'µ' => "0,4 0,-2;0,1 1,0 3,0 4,1;4,4 4,0",
        'Ω' => "0,0 1,0 1,1 0,3 0,5 1,6 3,6 4,5 4,3 3,1 3,0 4,0",
        '°' => "1,6 2,6 2,5 1,5 1,6",
        '±' => "2,5 2,1;0,3 4,3;0,0 4,0",
        _ => "0,0 4,0 4,6 0,6 0,0",
    }
}

/// The strokes of a glyph in grid units.
fn strokes(c: char) -> impl Iterator<Item = Vec<(f64, f64)>> {
    glyph(c).split(';').filter(|s| !s.is_empty()).map(|stroke| {
        stroke
            .split(' ')
            .filter_map(|p| {
                let (x, y) = p.split_once(',')?;
                Some((x.parse().ok()?, y.parse().ok()?))
            })
            .collect()
    })
}

/// The strokes of a text run in sheet coordinates (each a polyline), laid out like the
/// estimated text box ([`crate::geometry::text_box`]): characters [`CHAR_WIDTH`] times the
/// height apart, lines one height apart, turned by `angle` about the anchor.
pub fn text_strokes(at: Pt, text: &str, size: f64, angle: f64, h: HAlign, v: VAlign) -> Vec<Vec<Pt>> {
    let unit = size * CAP / CAP_UNITS;
    let advance = size * CHAR_WIDTH;
    let inset = (advance - GLYPH_UNITS * unit) / 2.0;
    let (sin, cos) = angle.to_radians().sin_cos();
    let turn = |x: f64, y: f64| at + Vec2::new(x * cos - y * sin, x * sin + y * cos);
    let lines: Vec<&str> = text.lines().take(super::MAX_TEXT_CHARS).collect();
    let count = lines.len().max(1);
    let bottom = match v {
        VAlign::Bottom => 0.0,
        VAlign::Middle => -(count as f64) * size / 2.0,
        VAlign::Top => -(count as f64) * size,
    };
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let chars: Vec<char> = line.chars().take(super::MAX_TEXT_CHARS).collect();
        let base = bottom + (count - 1 - i) as f64 * size + BASELINE * size;
        let left = -(chars.len() as f64) * advance * h_shift(h);
        for (k, c) in chars.iter().enumerate() {
            let x0 = left + k as f64 * advance + inset;
            for s in strokes(*c) {
                out.push(s.iter().map(|&(gx, gy)| turn(x0 + gx * unit, base + gy * unit)).collect());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_glyph_parses_into_strokes_on_the_grid() {
        let chars = (' '..='~').chain(['µ', 'Ω', '°', '±', '\u{2603}']);
        for c in chars {
            let text = glyph(c);
            let points = text.split([';', ' ']).filter(|p| !p.is_empty()).count();
            let parsed: usize = strokes(c).map(|s| s.len()).sum();
            assert_eq!(parsed, points, "{c:?}: every point parses");
            for s in strokes(c) {
                assert!(s.len() >= 2, "{c:?}: a stroke has two points");
                assert!(s.iter().all(|&(x, y)| (0.0..=4.0).contains(&x) && (-2.0..=7.0).contains(&y)), "{c:?}");
            }
        }
        assert_eq!(glyph(' '), "");
    }

    #[test]
    fn texts_are_laid_out_like_their_estimated_box() {
        let s = text_strokes(Pt::ORIGIN, "HH", 100.0, 0.0, HAlign::Center, VAlign::Bottom);
        let xs: Vec<f64> = s.iter().flatten().map(|p| p.x).collect();
        let ys: Vec<f64> = s.iter().flatten().map(|p| p.y).collect();
        let (min_x, max_x) = xs.iter().fold((f64::MAX, f64::MIN), |(a, b), &x| (a.min(x), b.max(x)));
        assert!(min_x >= -60.0 && max_x <= 60.0 && (min_x + max_x).abs() < 1e-9, "{min_x} {max_x}");
        assert!(ys.iter().all(|&y| (0.0..=100.0).contains(&y)));
        let up = text_strokes(Pt::ORIGIN, "I", 100.0, 90.0, HAlign::Left, VAlign::Bottom);
        assert!(up.iter().flatten().all(|p| p.x <= 1e-9), "turned texts go up, left of the anchor");
    }
}
