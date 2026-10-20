//! File ▸ Export Graphics (Export Bitmap, DXF, Encapsulated Postscript, Adobe PDF, SVG) and
//! File ▸ Print Design (printing to a PDF file). The drawing is the render crate's display
//! list of a sheet; the formats are written by `protasena_render::export`.
//!
//! Not built: Export Metafile (a Windows format), Export Vector File, Print Setup, Printer
//! Information and Mark Output Area (the whole sheet is always exported).

use std::path::Path;

use protasena_doc::Sheet;
use protasena_render::export::{DPI_RANGE, Format, Options, Page, SCALE_RANGE, export};
use protasena_render::{sheet_display_list, sheet_rect};
use serde_json::{Value, json};

use super::file::path_param;
use crate::args::Args;
use crate::error::{EngineError, Result};
use crate::registry::CommandSpec;
use crate::request::Request;
use crate::session::Session;

const PARAMS: &str = "{format?, path?: string, sheet?: name|id|{index}, all_sheets?: bool (PDF), dpi?: 10–2400 (bitmaps, default 200), scale?: 0.01–100 (vector, default 1), mono?: bool} — path defaults to the project file with the format's extension";

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("export-graphics", "Export Graphics", export_graphics)
            .params("{format: \"svg\"|\"pdf\"|\"eps\"|\"dxf\"|\"png\"|\"bmp\", path?, sheet?, all_sheets?, dpi?, scale?, mono?} — as the Export Graphics items"),
        CommandSpec::new("export-bitmap", "Export Bitmap", export_graphics)
            .menu(&["File", "Export Graphics", "Export Bitmap"])
            .params(PARAMS)
            .dialog(|_| Some(Request::ExportGraphics { format: "png" })),
        CommandSpec::new("export-dxf", "Export DXF File", export_graphics)
            .menu(&["File", "Export Graphics", "Export DXF File"])
            .params(PARAMS)
            .dialog(|_| Some(Request::ExportGraphics { format: "dxf" })),
        CommandSpec::new("export-eps", "Export Encapsulated Postscript", export_graphics)
            .menu(&["File", "Export Graphics", "Export Encapsulated Postscript"])
            .params(PARAMS)
            .dialog(|_| Some(Request::ExportGraphics { format: "eps" })),
        CommandSpec::new("export-pdf", "Export Adobe PDF File", export_graphics)
            .menu(&["File", "Export Graphics", "Export Adobe PDF File"])
            .params(PARAMS)
            .dialog(|_| Some(Request::ExportGraphics { format: "pdf" })),
        CommandSpec::new("export-svg", "Export SVG File", export_graphics)
            .menu(&["File", "Export Graphics", "Export SVG File"])
            .params(PARAMS)
            .dialog(|_| Some(Request::ExportGraphics { format: "svg" })),
        CommandSpec::new("print-design", "Print Design", print_design)
            .menu(&["File", "Print Design"])
            .params("{path?: string, all_sheets?: bool (default true), sheet?, mono?: bool, view?: bool} — prints to a PDF file; `view` is handed back for the window to open the file (the engine never opens it)")
            .dialog(|_| Some(Request::PrintDesign)),
    ]
}

/// The formats each command writes, the first being its default.
fn formats_of(cmd: &str) -> &'static [Format] {
    match cmd {
        "export-bitmap" => &[Format::Png, Format::Bmp],
        "export-dxf" => &[Format::Dxf],
        "export-eps" => &[Format::Eps],
        "export-pdf" | "print-design" => &[Format::Pdf],
        "export-svg" => &[Format::Svg],
        _ => &Format::ALL,
    }
}

fn format_param(a: &Args) -> Result<Format> {
    let allowed = formats_of(a.cmd());
    let names = || allowed.iter().map(|f| f.extension()).collect::<Vec<_>>().join(", ");
    match a.str("format")? {
        Some(name) => Format::parse(name).filter(|f| allowed.contains(f)).ok_or_else(|| a.bad(format!("`format` must be one of {}", names()))),
        None if a.cmd() == "export-graphics" => Err(a.bad(format!("`format` is required: one of {}", names()))),
        None => allowed.first().copied().ok_or_else(|| a.bad("no format")),
    }
}

fn range_param(a: &Args, key: &str, range: &std::ops::RangeInclusive<f64>, default: f64) -> Result<f64> {
    match a.f64(key)? {
        None => Ok(default),
        Some(v) if range.contains(&v) => Ok(v),
        Some(_) => Err(a.bad(format!("`{key}` must be between {} and {}", range.start(), range.end()))),
    }
}

/// The page of a sheet: everything on it, over the sheet's border and whatever lies beyond it.
fn page_of(s: &Session, sheet: &Sheet) -> Page {
    let list = sheet_display_list(sheet, &s.library);
    let area = list.bounds().map_or(sheet_rect(sheet), |b| b.union(sheet_rect(sheet)));
    Page { list, area, title: sheet.name.clone() }
}

/// The sheets to draw: all of them, or the one named (the current one by default).
fn pages(s: &Session, a: &Args, all: bool) -> Result<Vec<Page>> {
    if all {
        return Ok(s.project.sheets.iter().map(|sh| page_of(s, sh)).collect());
    }
    let id = a.sheet(s, "sheet")?.unwrap_or(s.sheet);
    let sheet = s.project.sheet(id).ok_or_else(|| EngineError::Other("the project has no sheets".into()))?;
    Ok(vec![page_of(s, sheet)])
}

/// Draws the pages and writes the file; the JSON result names it.
fn write_file(s: &mut Session, a: &Args, format: Format, all: bool) -> Result<Value> {
    if all && !format.is_multi_page() {
        return Err(a.bad(format!("`all_sheets` needs a format with pages (pdf), not {}", format.extension())));
    }
    let options = Options {
        dpi: range_param(a, "dpi", &DPI_RANGE, Options::default().dpi)?,
        scale: range_param(a, "scale", &SCALE_RANGE, 1.0)?,
        mono: a.bool("mono")?.unwrap_or(false),
    };
    let path = match path_param(a, "path")? {
        Some(p) => p,
        None => s
            .path
            .as_deref()
            .map(|p| p.with_extension(format.extension()))
            .ok_or_else(|| a.bad("`path` is required until the project has been saved"))?,
    };
    let pages = pages(s, a, all)?;
    let bytes = export(format, &pages, options).map_err(|e| EngineError::Other(e.to_string()))?;
    write_bytes(&path, &bytes)?;
    s.note(format!("{} written to {}", format.extension().to_ascii_uppercase(), path.display()));
    Ok(json!({ "path": path.display().to_string(), "format": format.extension(), "pages": pages.len(), "bytes": bytes.len() }))
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    protasena_io::atomic_write(path, false, |out| out.write_all(bytes))
        .map_err(|e| EngineError::Export(protasena_io::ExportError::Io { path: path.to_owned(), message: e.to_string() }))
}

fn export_graphics(s: &mut Session, a: &Args) -> Result<Value> {
    let format = format_param(a)?;
    let all = a.bool("all_sheets")?.unwrap_or(false);
    write_file(s, a, format, all)
}

fn print_design(s: &mut Session, a: &Args) -> Result<Value> {
    let all = a.bool("all_sheets")?.unwrap_or(!a.has("sheet"));
    let view = a.bool("view")?.unwrap_or(false);
    let mut v = write_file(s, a, Format::Pdf, all)?;
    if let Some(o) = v.as_object_mut() {
        o.insert("view".into(), json!(view));
    }
    Ok(v)
}
