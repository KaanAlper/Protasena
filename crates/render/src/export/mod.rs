//! File ▸ Export Graphics and Print Design: display lists written as files. Vector formats
//! (SVG, PDF, Encapsulated PostScript, DXF) keep the drawing's real size (times `scale`);
//! bitmaps (PNG, BMP) are rendered at `dpi` dots per inch.
//!
//! - **Text**: SVG, PDF and EPS keep texts as text in Helvetica (a standard font every PDF and
//!   PostScript reader has; nothing is embedded), DXF as `TEXT` entities. Bitmaps draw texts
//!   with Protasena's own stroke font ([`font`]), drawn in code.
//! - **Colours** come from our own print palette ([`colour`]) on white, or black on white
//!   with `mono`. The colours of the window's theme are not used.
//! - **Pages**: PDF writes one page per [`Page`]; the other formats write the first.
//!
//! # Never crash
//! Non-finite primitives are skipped, texts are capped in length, and a bitmap larger than
//! [`MAX_PIXELS`] is refused instead of allocated.

mod dxf;
mod eps;
pub mod font;
mod pdf;
mod raster;
mod svg;

use std::fmt;

use protasena_geom::{Pt, Rect, Vec2};

use crate::geometry::{flatten_arc, flatten_bezier};
use crate::prim::{DisplayList, HAlign, Ink, Prim, Shape, VAlign};

/// Largest bitmap rendered, in pixels.
pub const MAX_PIXELS: u64 = 64_000_000;

/// Accepted bitmap resolutions, in dots per inch.
pub const DPI_RANGE: std::ops::RangeInclusive<f64> = 10.0..=2400.0;

/// Accepted scale factors of the vector formats.
pub const SCALE_RANGE: std::ops::RangeInclusive<f64> = 0.01..=100.0;

/// Most characters of one text drawn.
const MAX_TEXT_CHARS: usize = 1024;

/// Deviation of flattened arcs and curves from the true shape, in thou.
const FLATNESS: f64 = 0.5;

/// Width of a hairline (width 0) in the vector formats, in thou.
const HAIRLINE: f64 = 2.0;

/// The baseline of a text above the bottom of its box, as a fraction of the text height.
const BASELINE: f64 = 0.2;

/// An output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Scalable Vector Graphics.
    Svg,
    /// Adobe PDF (one page per sheet).
    Pdf,
    /// Encapsulated PostScript.
    Eps,
    /// AutoCAD DXF (R12, ASCII; units: thou).
    Dxf,
    /// Portable Network Graphics.
    Png,
    /// Windows bitmap (24 bits per pixel).
    Bmp,
}

impl Format {
    /// Every format.
    pub const ALL: [Format; 6] = [Format::Svg, Format::Pdf, Format::Eps, Format::Dxf, Format::Png, Format::Bmp];

    /// The format named `name` (`svg`, `pdf`, `eps`, `dxf`, `png`, `bmp`; any case).
    pub fn parse(name: &str) -> Option<Format> {
        Format::ALL.into_iter().find(|f| f.extension().eq_ignore_ascii_case(name.trim()))
    }

    /// The file extension (and parameter name).
    pub fn extension(self) -> &'static str {
        match self {
            Format::Svg => "svg",
            Format::Pdf => "pdf",
            Format::Eps => "eps",
            Format::Dxf => "dxf",
            Format::Png => "png",
            Format::Bmp => "bmp",
        }
    }

    /// Whether the format is a bitmap (rendered at a resolution).
    pub fn is_bitmap(self) -> bool {
        matches!(self, Format::Png | Format::Bmp)
    }

    /// Whether the format holds several pages.
    pub fn is_multi_page(self) -> bool {
        self == Format::Pdf
    }
}

/// How to write.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// Size factor of the vector formats (1 = real size).
    pub scale: f64,
    /// Resolution of bitmaps, in dots per inch.
    pub dpi: f64,
    /// Black on white instead of the colour palette.
    pub mono: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { scale: 1.0, dpi: 200.0, mono: false }
    }
}

/// One page: what is drawn and the area of the sheet it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// The primitives.
    pub list: DisplayList,
    /// The area shown (sheet coordinates).
    pub area: Rect,
    /// A title (the sheet's name), for formats that keep one.
    pub title: String,
}

/// Why nothing was written.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportError {
    /// No page to write.
    NoPages,
    /// The area is empty or not finite.
    BadArea,
    /// A parameter is out of range.
    BadOption(String),
    /// The bitmap would be too large.
    TooLarge {
        /// Width in pixels.
        width: u64,
        /// Height in pixels.
        height: u64,
    },
    /// The image could not be encoded.
    Encode(String),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPages => write!(f, "nothing to export"),
            Self::BadArea => write!(f, "the area to export is empty"),
            Self::BadOption(m) => write!(f, "{m}"),
            Self::TooLarge { width, height } => {
                write!(f, "the bitmap would be {width} × {height} pixels (at most {MAX_PIXELS} pixels): choose a lower resolution")
            }
            Self::Encode(m) => write!(f, "the image could not be encoded: {m}"),
        }
    }
}

impl std::error::Error for ExportError {}

/// Writes `pages` in `format` (all pages for PDF, the first for the others).
///
/// # Errors
/// [`ExportError`] for no pages, an empty area, options out of range or a bitmap above
/// [`MAX_PIXELS`].
pub fn export(format: Format, pages: &[Page], options: Options) -> Result<Vec<u8>, ExportError> {
    let first = pages.first().ok_or(ExportError::NoPages)?;
    check(options)?;
    for p in pages {
        let ok = p.area.min.is_finite() && p.area.max.is_finite() && p.area.width() > 0.0 && p.area.height() > 0.0;
        if !ok {
            return Err(ExportError::BadArea);
        }
    }
    Ok(match format {
        Format::Svg => svg::write(first, options).into_bytes(),
        Format::Pdf => pdf::write(pages, options),
        Format::Eps => eps::write(first, options).into_bytes(),
        Format::Dxf => dxf::write(first).into_bytes(),
        Format::Png => raster::png(first, options)?,
        Format::Bmp => raster::bmp(first, options)?,
    })
}

fn check(o: Options) -> Result<(), ExportError> {
    if !(o.scale.is_finite() && SCALE_RANGE.contains(&o.scale)) {
        return Err(ExportError::BadOption(format!("the scale must be between {} and {}", SCALE_RANGE.start(), SCALE_RANGE.end())));
    }
    if !(o.dpi.is_finite() && DPI_RANGE.contains(&o.dpi)) {
        return Err(ExportError::BadOption(format!("the resolution must be between {} and {} dpi", DPI_RANGE.start(), DPI_RANGE.end())));
    }
    Ok(())
}

/// An sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// White paper.
    pub const WHITE: Rgb = Rgb(255, 255, 255);
    /// Black ink.
    pub const BLACK: Rgb = Rgb(0, 0, 0);

    /// `#rrggbb`.
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }

    /// The channels from 0 to 1.
    pub fn unit(self) -> (f64, f64, f64) {
        (f64::from(self.0) / 255.0, f64::from(self.1) / 255.0, f64::from(self.2) / 255.0)
    }
}

/// The print colour of an ink (our own palette for white paper), or black/white with `mono`.
pub fn colour(ink: Ink, mono: bool) -> Rgb {
    if mono {
        return if matches!(ink, Ink::BodyFill | Ink::GraphicFill | Ink::SubcircuitFill) { Rgb::WHITE } else { Rgb::BLACK };
    }
    match ink {
        Ink::Body | Ink::Pin | Ink::BodyText | Ink::Graphic | Ink::Subcircuit | Ink::Port => Rgb(138, 28, 28),
        Ink::BodyFill | Ink::GraphicFill | Ink::SubcircuitFill => Rgb(253, 246, 227),
        Ink::PinName => Rgb(31, 63, 122),
        Ink::PinNumber => Rgb(85, 85, 85),
        Ink::Reference | Ink::Value | Ink::Text => Rgb(42, 42, 42),
        Ink::Missing => Rgb(192, 57, 43),
        Ink::Wire | Ink::Junction => Rgb(27, 110, 42),
        Ink::Bus => Rgb(31, 79, 160),
        Ink::Label => Rgb(13, 92, 99),
        Ink::Terminal | Ink::TerminalText => Rgb(107, 44, 145),
        Ink::SheetBorder => Rgb(58, 74, 107),
        Ink::Pad => Rgb(184, 134, 11),
        Ink::Silk => Rgb(68, 68, 68),
    }
}

/// The finite primitives of a page, in drawing order.
fn drawn(page: &Page) -> impl Iterator<Item = &Prim> {
    page.list.prims.iter().filter(|p| p.shape.is_finite() && p.width.is_finite())
}

/// A primitive's outline as a polyline and whether it is closed; `None` for circles and texts
/// (every format draws those natively).
fn polyline(shape: &Shape) -> Option<(Vec<Pt>, bool)> {
    match shape {
        Shape::Line { a, b } => Some((vec![*a, *b], false)),
        Shape::Polyline { points, closed } => Some((points.clone(), *closed)),
        Shape::Arc { center, radius, start, sweep } => Some((flatten_arc(*center, *radius, *start, *sweep, FLATNESS), false)),
        Shape::Bezier { points } => Some((flatten_bezier(*points, 32), false)),
        Shape::Circle { .. } | Shape::Text { .. } => None,
    }
}

/// The drawn line width of a primitive in the vector formats, in thou.
fn line_width(p: &Prim) -> f64 {
    if p.width > 0.0 { p.width } else { HAIRLINE }
}

/// One line of a text: the point on its baseline that the alignment refers to, and its text.
struct TextLine {
    at: Pt,
    text: String,
}

/// The lines of a text run, the first on top, with their baseline anchors.
fn text_lines(at: Pt, text: &str, size: f64, angle: f64, v: VAlign) -> Vec<TextLine> {
    let lines: Vec<String> = text.lines().map(|l| l.chars().take(MAX_TEXT_CHARS).collect()).take(MAX_TEXT_CHARS).collect();
    let n = lines.len().max(1) as f64;
    let height = n * size;
    let bottom = match v {
        VAlign::Bottom => 0.0,
        VAlign::Middle => -height / 2.0,
        VAlign::Top => -height,
    };
    let (sin, cos) = angle.to_radians().sin_cos();
    let count = lines.len();
    lines
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            let y = bottom + (count - 1 - i) as f64 * size + BASELINE * size;
            TextLine { at: at + Vec2::new(-y * sin, y * cos), text }
        })
        .collect()
}

/// The fraction of a line's width left of its anchor.
fn h_shift(h: HAlign) -> f64 {
    match h {
        HAlign::Left => 0.0,
        HAlign::Center => 0.5,
        HAlign::Right => 1.0,
    }
}

/// A number for a text format: at most three decimals, no trailing zeros, never `NaN`.
fn num(x: f64) -> String {
    if !x.is_finite() {
        return "0".to_owned();
    }
    let s = format!("{:.3}", x);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() { "0".to_owned() } else { s.to_owned() }
}

#[cfg(test)]
mod tests;
