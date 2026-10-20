//! Bitmaps: the page rendered with tiny-skia at the chosen resolution on white, anti-aliased,
//! then written as PNG (8-bit RGB, with the resolution in `pHYs`) or BMP (24-bit, bottom-up).

use protasena_geom::Pt;
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

use super::font::{STROKE, text_strokes};
use super::{ExportError, MAX_PIXELS, Options, Page, Rgb, colour, drawn, polyline};
use crate::prim::{Prim, Shape};

/// Thou per inch.
const THOU_PER_INCH: f64 = 1000.0;

/// Metres per inch (for the resolution fields).
const METRES_PER_INCH: f64 = 0.0254;

/// The page as RGBA pixels, with its size.
pub(super) fn render(page: &Page, o: Options) -> Result<Pixmap, ExportError> {
    let a = page.area;
    let k = o.dpi / THOU_PER_INCH;
    let (w, h) = ((a.width() * k).ceil(), (a.height() * k).ceil());
    let too_large = || ExportError::TooLarge { width: w.max(0.0) as u64, height: h.max(0.0) as u64 };
    if !(w.is_finite() && h.is_finite()) || w * h > MAX_PIXELS as f64 {
        return Err(too_large());
    }
    let (wi, hi) = (u32::try_from(w.max(1.0) as u64).map_err(|_| too_large())?, u32::try_from(h.max(1.0) as u64).map_err(|_| too_large())?);
    let mut pixmap = Pixmap::new(wi, hi).ok_or_else(too_large)?;
    pixmap.fill(tiny_skia::Color::WHITE);
    let t = Transform::from_row(k as f32, 0.0, 0.0, -k as f32, (-a.min.x * k) as f32, (a.max.y * k) as f32);
    let hairline = 1.0 / k;
    for p in drawn(page) {
        draw(&mut pixmap, p, o.mono, t, hairline);
    }
    Ok(pixmap)
}

fn paint_of(c: Rgb) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(c.0, c.1, c.2, 255);
    paint.anti_alias = true;
    paint
}

fn stroke_of(width: f64) -> Stroke {
    Stroke { width: width as f32, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() }
}

fn path_of(points: &[Pt], closed: bool) -> Option<tiny_skia::Path> {
    let mut b = PathBuilder::new();
    let (first, rest) = points.split_first()?;
    b.move_to(first.x as f32, first.y as f32);
    for q in rest {
        b.line_to(q.x as f32, q.y as f32);
    }
    if closed {
        b.close();
    }
    b.finish()
}

fn draw(pixmap: &mut Pixmap, p: &Prim, mono: bool, t: Transform, hairline: f64) {
    let ink = paint_of(colour(p.ink, mono));
    let stroke = stroke_of(p.width.max(hairline));
    match &p.shape {
        Shape::Circle { center, radius } => {
            let Some(path) = PathBuilder::from_circle(center.x as f32, center.y as f32, radius.abs().max(hairline / 2.0) as f32) else { return };
            if let Some(f) = p.fill {
                pixmap.fill_path(&path, &paint_of(colour(f, mono)), FillRule::Winding, t, None);
            }
            pixmap.stroke_path(&path, &ink, &stroke, t, None);
        }
        Shape::Text { at, text, size, angle, h, v } => {
            let pen = stroke_of((size * STROKE).max(hairline));
            for s in text_strokes(*at, text, *size, *angle, *h, *v) {
                if let Some(path) = path_of(&s, false) {
                    pixmap.stroke_path(&path, &ink, &pen, t, None);
                }
            }
        }
        shape => {
            let Some((points, closed)) = polyline(shape) else { return };
            let Some(path) = path_of(&points, closed) else { return };
            if let (Some(f), true) = (p.fill, closed) {
                pixmap.fill_path(&path, &paint_of(colour(f, mono)), FillRule::EvenOdd, t, None);
            }
            pixmap.stroke_path(&path, &ink, &stroke, t, None);
        }
    }
}

/// Rows of RGB bytes, top first.
fn rgb_rows(pixmap: &Pixmap) -> impl Iterator<Item = Vec<u8>> + '_ {
    let w = pixmap.width() as usize;
    pixmap.data().chunks(w.max(1) * 4).map(|row| row.as_chunks::<4>().0.iter().flat_map(|px| px.iter().take(3).copied()).collect())
}

fn dots_per_metre(dpi: f64) -> u32 {
    let d = (dpi / METRES_PER_INCH).round();
    if d.is_finite() && d > 0.0 && d < f64::from(u32::MAX) { d as u32 } else { 0 }
}

/// The page as a PNG file.
pub(super) fn png(page: &Page, o: Options) -> Result<Vec<u8>, ExportError> {
    let pixmap = render(page, o)?;
    let mut raw = Vec::with_capacity((pixmap.width() as usize * 3 + 1) * pixmap.height() as usize);
    for row in rgb_rows(&pixmap) {
        raw.push(0); // filter: none
        raw.extend_from_slice(&row);
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&pixmap.width().to_be_bytes());
    ihdr.extend_from_slice(&pixmap.height().to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8 bits, RGB, deflate, no filter set, no interlace
    chunk(&mut out, b"IHDR", &ihdr);
    let dpm = dots_per_metre(o.dpi).to_be_bytes();
    let mut phys = Vec::with_capacity(9);
    phys.extend_from_slice(&dpm);
    phys.extend_from_slice(&dpm);
    phys.push(1); // unit: metre
    chunk(&mut out, b"pHYs", &phys);
    chunk(&mut out, b"IDAT", &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6));
    chunk(&mut out, b"IEND", &[]);
    Ok(out)
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&u32::try_from(data.len()).unwrap_or(u32::MAX).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(out.get(start..).unwrap_or_default());
    out.extend_from_slice(&crc.to_be_bytes());
}

/// CRC-32 (ISO 3309, as PNG uses it).
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// The page as a 24-bit BMP file.
pub(super) fn bmp(page: &Page, o: Options) -> Result<Vec<u8>, ExportError> {
    let pixmap = render(page, o)?;
    let (w, h) = (pixmap.width(), pixmap.height());
    let row_bytes = (w as usize * 3).div_ceil(4) * 4;
    let image = row_bytes * h as usize;
    let size = u32::try_from(54 + image).map_err(|_| ExportError::TooLarge { width: u64::from(w), height: u64::from(h) })?;
    let mut out = Vec::with_capacity(54 + image);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // no compression
    out.extend_from_slice(&u32::try_from(image).unwrap_or(0).to_le_bytes());
    let dpm = dots_per_metre(o.dpi).to_le_bytes();
    out.extend_from_slice(&dpm);
    out.extend_from_slice(&dpm);
    out.extend_from_slice(&[0; 8]);
    let rows: Vec<Vec<u8>> = rgb_rows(&pixmap).collect();
    for row in rows.iter().rev() {
        let start = out.len();
        out.extend(row.as_chunks::<3>().0.iter().flat_map(|px| px.iter().rev().copied()));
        out.resize(start + row_bytes, 0);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_is_the_png_one() {
        assert_eq!(crc32(b"IEND"), 0xae42_6082);
        assert_eq!(crc32(b""), 0);
    }
}
