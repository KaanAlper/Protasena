use protasena_doc::{Component, ComponentId, DeviceRef, Sheet, SheetId, SheetSize, Wire, WireId};
use protasena_geom::{Pt, Rect, Transform, Vec2};
use protasena_library::Library;

use super::*;
use crate::{sheet_display_list, sheet_rect};

fn small_sheet() -> Sheet {
    let mut sheet = Sheet::new(SheetId(1), "Root sheet 1", SheetSize::A4);
    let c =
        Component::new(ComponentId(2), DeviceRef::new("", "RES"), "R1", "10k").placed(Transform::new(protasena_geom::Orient::IDENTITY, Vec2::ZERO));
    sheet.components.push(c);
    sheet.wires.push(Wire { id: WireId(3), points: vec![Pt::new(200.0, 0.0), Pt::new(800.0, 0.0)] });
    sheet
}

fn page() -> Page {
    let sheet = small_sheet();
    Page { list: sheet_display_list(&sheet, &Library::builtin()), area: sheet_rect(&sheet), title: sheet.name.clone() }
}

fn every_shape() -> Page {
    let mut list = DisplayList::default();
    list.push(Prim::new(Shape::Line { a: Pt::new(0.0, 0.0), b: Pt::new(100.0, 0.0) }, Ink::Wire).width(8.0));
    list.push(Prim::new(Shape::Circle { center: Pt::new(50.0, 50.0), radius: 20.0 }, Ink::Junction).fill(Some(Ink::Junction)));
    list.push(Prim::new(Shape::Arc { center: Pt::ORIGIN, radius: 50.0, start: 0.0, sweep: 1.5 }, Ink::Graphic));
    list.push(Prim::new(Shape::Bezier { points: [Pt::ORIGIN, Pt::new(10.0, 40.0), Pt::new(60.0, 40.0), Pt::new(90.0, 0.0)] }, Ink::Body));
    list.push(
        Prim::new(Shape::Polyline { points: vec![Pt::ORIGIN, Pt::new(80.0, 0.0), Pt::new(80.0, 60.0)], closed: true }, Ink::Body)
            .fill(Some(Ink::BodyFill)),
    );
    let text =
        Shape::Text { at: Pt::new(10.0, 10.0), text: "A<&>\"(x)\\\nµΩ".into(), size: 40.0, angle: 90.0, h: HAlign::Right, v: VAlign::Middle };
    list.push(Prim::new(text, Ink::Text));
    list.push(Prim::new(Shape::Line { a: Pt::new(f64::NAN, 0.0), b: Pt::ORIGIN }, Ink::Wire));
    list.push(Prim::new(Shape::Line { a: Pt::ORIGIN, b: Pt::new(1.0, 1.0) }, Ink::Wire).width(f64::INFINITY));
    Page { list, area: Rect::new(Pt::new(-100.0, -100.0), Pt::new(200.0, 200.0)), title: "every (shape)".into() }
}

#[test]
fn formats_have_names_and_kinds() {
    for f in Format::ALL {
        assert_eq!(Format::parse(&f.extension().to_ascii_uppercase()), Some(f));
    }
    assert_eq!(Format::parse("wmf"), None);
    assert!(Format::Png.is_bitmap() && Format::Bmp.is_bitmap() && !Format::Svg.is_bitmap());
    assert!(Format::Pdf.is_multi_page() && !Format::Eps.is_multi_page());
}

#[test]
fn svg_keeps_texts_as_text_at_real_size() {
    let svg = String::from_utf8(export(Format::Svg, &[page()], Options::default()).unwrap()).unwrap();
    assert!(svg.starts_with("<?xml"));
    assert!(svg.contains(r#"width="11.693in" height="8.268in" viewBox="0 0 11693 8268""#), "{}", &svg[..300]);
    assert!(svg.contains(">R1</text>") && svg.contains(">10k</text>"));
    assert!(svg.contains(&colour(Ink::Wire, false).hex()));
    assert!(!svg.contains("NaN") && !svg.contains("inf"));
    let odd = String::from_utf8(export(Format::Svg, &[every_shape()], Options { scale: 2.0, ..Options::default() }).unwrap()).unwrap();
    assert!(odd.contains("A&lt;&amp;&gt;&quot;(x)\\</text>") && odd.contains(">µΩ</text>"));
    assert!(odd.contains("<title>every (shape)</title>") && odd.contains(r#"width="0.6in""#));
    assert!(!odd.contains("NaN") && !odd.contains("inf"));
}

#[test]
fn mono_draws_black_on_white() {
    let svg = String::from_utf8(export(Format::Svg, &[page()], Options { mono: true, ..Options::default() }).unwrap()).unwrap();
    assert!(!svg.contains(&colour(Ink::Wire, false).hex()));
    assert!(svg.contains("#000000"));
    assert_eq!(colour(Ink::BodyFill, true), Rgb::WHITE);
}

/// Checks a PDF's cross-reference table: every offset starts its object.
fn check_xref(pdf: &[u8]) -> usize {
    let tail_at = pdf.windows(10).rposition(|w| w == b"startxref\n").unwrap() + 10;
    let tail = std::str::from_utf8(&pdf[tail_at..]).unwrap();
    let start: usize = tail.lines().next().unwrap().parse().unwrap();
    let table = std::str::from_utf8(&pdf[start..]).unwrap();
    assert!(table.starts_with("xref\n"));
    let mut lines = table.lines().skip(1);
    let count: usize = lines.next().unwrap().split(' ').nth(1).unwrap().parse().unwrap();
    lines.next();
    for id in 1..count {
        let offset: usize = lines.next().unwrap()[..10].parse().unwrap();
        assert!(pdf[offset..].starts_with(format!("{id} 0 obj").as_bytes()), "object {id}");
    }
    count
}

#[test]
fn pdf_has_a_page_per_sheet_and_a_valid_cross_reference() {
    let pdf = export(Format::Pdf, &[page(), every_shape()], Options::default()).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(pdf.ends_with(b"%%EOF\n"));
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/Count 2") && text.contains("/BaseFont /Helvetica"));
    assert!(text.contains("/MediaBox [0 0 841.896 595.296]"), "A4 landscape in points");
    assert_eq!(check_xref(&pdf), 9);
}

#[test]
fn eps_and_dxf_are_text_files_with_their_headers() {
    let eps = String::from_utf8(export(Format::Eps, &[every_shape()], Options::default()).unwrap()).unwrap();
    assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 22 22\n"));
    assert!(eps.contains("(A<&>\"\\(x\\)\\\\) dup stringwidth") && eps.contains("(??)"));
    assert!(eps.ends_with("%%EOF\n"));
    let dxf = String::from_utf8(export(Format::Dxf, &[page()], Options::default()).unwrap()).unwrap();
    assert!(dxf.starts_with("  0\nSECTION\n  2\nHEADER\n"));
    assert!(dxf.contains("TEXT\n  8\nTEXT\n") && dxf.contains("\nR1\n") && dxf.contains("POLYLINE\n  8\nWIRES\n"));
    assert!(dxf.ends_with("  0\nEOF\n"));
}

#[test]
fn bitmaps_are_drawn_at_the_resolution() {
    let p = page();
    let pixmap = raster::render(&p, Options { dpi: 100.0, ..Options::default() }).unwrap();
    assert_eq!((pixmap.width(), pixmap.height()), (1170, 827));
    let fine = raster::render(&every_shape(), Options { dpi: 1000.0, ..Options::default() }).unwrap();
    assert_eq!((fine.width(), fine.height()), (300, 300));
    let at = |x: u32, y: u32| fine.pixel(x, y).map(|c| (c.red(), c.green(), c.blue()));
    let w = colour(Ink::Wire, false);
    assert_eq!(at(195, 200), Some((w.0, w.1, w.2)), "on the 8 thou wire at (95, 0)");
    assert_eq!(at(10, 10), Some((255, 255, 255)), "paper");
    let png = export(Format::Png, std::slice::from_ref(&p), Options { dpi: 100.0, ..Options::default() }).unwrap();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR"));
    assert_eq!(&png[16..24], &[0, 0, 4, 146, 0, 0, 3, 59]);
    assert!(png.ends_with(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xae, 0x42, 0x60, 0x82]));
    let bmp = export(Format::Bmp, &[p], Options { dpi: 100.0, ..Options::default() }).unwrap();
    assert_eq!(&bmp[..2], b"BM");
    assert_eq!(u32::from_le_bytes(bmp[2..6].try_into().unwrap()) as usize, bmp.len());
    assert_eq!(bmp.len(), 54 + 3512 * 827, "rows padded to four bytes");
    assert!(export(Format::Png, &[every_shape()], Options::default()).is_ok());
}

#[test]
fn hostile_options_and_areas_are_refused() {
    let p = page();
    assert_eq!(export(Format::Svg, &[], Options::default()), Err(ExportError::NoPages));
    for bad in
        [Options { dpi: f64::NAN, ..Options::default() }, Options { dpi: 5.0, ..Options::default() }, Options { scale: 0.0, ..Options::default() }]
    {
        assert!(matches!(export(Format::Png, std::slice::from_ref(&p), bad), Err(ExportError::BadOption(_))), "{bad:?}");
    }
    let flat = Page { area: Rect::new(Pt::ORIGIN, Pt::new(100.0, 0.0)), ..p.clone() };
    assert_eq!(export(Format::Pdf, &[flat], Options::default()), Err(ExportError::BadArea));
    let huge = Page { area: Rect::new(Pt::ORIGIN, Pt::new(1e7, 1e7)), ..p.clone() };
    let e = export(Format::Png, &[huge], Options { dpi: 2400.0, ..Options::default() }).unwrap_err();
    assert!(matches!(e, ExportError::TooLarge { .. }) && e.to_string().contains("lower resolution"), "{e}");
    let mut long = p;
    long.list.push(Prim::new(
        Shape::Text { at: Pt::ORIGIN, text: "x".repeat(100_000), size: 50.0, angle: 0.0, h: HAlign::Left, v: VAlign::Bottom },
        Ink::Text,
    ));
    let svg = export(Format::Svg, &[long], Options::default()).unwrap();
    assert!(svg.len() < 100_000, "a text is drawn up to {MAX_TEXT_CHARS} characters");
}

#[test]
fn numbers_are_short_and_finite() {
    assert_eq!(num(1.0), "1");
    assert_eq!(num(-0.0001), "0");
    assert_eq!(num(2.5), "2.5");
    assert_eq!(num(f64::NAN), "0");
    assert_eq!(num(1234.56789), "1234.568");
}
