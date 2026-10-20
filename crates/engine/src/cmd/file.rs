//! File menu: New Project, Open Project (Protasena `.psproj` files, and Proteus `.pdsprj` files,
//! which are imported), Save Project, Save Project As; Tool ▸ Netlist Compiler (netlist export).
//!
//! Paths are used as given: the callers (the window, the CLI and the local control channel)
//! act for the user who runs Protasena, with that user's file access.

use std::path::{Path, PathBuf};

use protasena_doc::{FIRST_SHEET_NAME, Project, Sheet, SheetId, SheetSize};
use protasena_io::NetlistFormat;
use serde_json::{Value, json};

use crate::args::Args;
use crate::error::{EngineError, Result};
use crate::registry::CommandSpec;
use crate::request::Request;
use crate::session::Session;

/// Longest accepted path, in bytes.
const MAX_PATH: usize = 4096;

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("new", "New Project", new)
            .menu(&["File", "New Project"])
            .key("Ctrl+N")
            .params("{name?: string, discard?: bool} — refused with unsaved changes unless discard")
            .dialog(confirm_new),
        CommandSpec::new("open", "Open Project", open)
            .menu(&["File", "Open Project"])
            .key("Ctrl+O")
            .params("{path: string, discard?: bool}")
            .dialog(|_| Some(Request::OpenProject)),
        CommandSpec::new("save", "Save Project", save)
            .menu(&["File", "Save Project"])
            .key("Ctrl+S")
            .params("{path?: string} — defaults to the project's file")
            .dialog(|s| s.path.is_none().then_some(Request::SaveProjectAs)),
        CommandSpec::new("save-as", "Save Project As", save_as)
            .menu(&["File", "Save Project As"])
            .params("{path: string}")
            .dialog(|_| Some(Request::SaveProjectAs)),
        CommandSpec::new("export-netlist", "Netlist Compiler", export_netlist)
            .menu(&["Tool", "Netlist Compiler"])
            .params("{path?: string, format?: \"json\"|\"spice\"} — path defaults to the project file with the format's extension")
            .dialog(|_| Some(Request::ExportNetlist)),
    ]
}

fn confirm_new(s: &Session) -> Option<Request> {
    s.dirty.then_some(Request::ConfirmDiscard { then: "new" })
}

pub(super) fn path_param(a: &Args, key: &str) -> Result<Option<PathBuf>> {
    match a.str(key)? {
        None => Ok(None),
        Some("") => Err(a.bad(format!("`{key}` is empty"))),
        Some(p) if p.len() > MAX_PATH => Err(a.bad(format!("`{key}` is longer than {MAX_PATH} bytes"))),
        Some(p) => Ok(Some(PathBuf::from(p))),
    }
}

fn refuse_unsaved(s: &Session, a: &Args) -> Result<()> {
    if s.dirty && !a.bool("discard")?.unwrap_or(false) {
        return Err(EngineError::Other("the project has unsaved changes: save it first, or pass `discard: true`".into()));
    }
    Ok(())
}

fn new(s: &mut Session, a: &Args) -> Result<Value> {
    refuse_unsaved(s, a)?;
    let name = a.str("name")?.unwrap_or("UNTITLED");
    s.replace_project(Project::new(name), None);
    Ok(json!({ "name": name }))
}

fn open(s: &mut Session, a: &Args) -> Result<Value> {
    refuse_unsaved(s, a)?;
    let path = path_param(a, "path")?.ok_or_else(|| a.bad("`path` (string) is required"))?;
    if is_pdsprj(&path) {
        return import(s, path);
    }
    let (mut project, report) = protasena_io::load(&path)?;
    let warnings: Vec<String> = report.warnings.iter().map(ToString::to_string).collect();
    ensure_sheet(&mut project)?;
    s.replace_project(project, Some(path.clone()));
    for w in &warnings {
        s.note(format!("{}: {w}", path.display()));
    }
    Ok(json!({ "path": path.display().to_string(), "warnings": warnings }))
}

/// Whether `path` names a Proteus project (`.pdsprj`, any case).
fn is_pdsprj(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("pdsprj"))
}

/// Opens a Proteus project by importing it. The imported project has no file of its own: Save
/// asks for a new one, so the `.pdsprj` is never written over.
fn import(s: &mut Session, path: PathBuf) -> Result<Value> {
    let (mut project, report) = protasena_io::import_pdsprj(&path).map_err(|e| EngineError::Other(format!("{}: {e}", path.display())))?;
    ensure_sheet(&mut project)?;
    s.replace_project(project, None);
    s.note(format!("Imported {}: {} components, {} wires, {} terminals", path.display(), report.components, report.wires, report.terminals));
    for w in &report.warnings {
        s.note(format!("{}: {w}", path.display()));
    }
    Ok(json!({
        "path": path.display().to_string(), "imported": true, "warnings": report.warnings,
        "components": report.components, "wires": report.wires, "terminals": report.terminals,
    }))
}

/// Gives a project without sheets its first sheet.
fn ensure_sheet(project: &mut Project) -> Result<()> {
    if project.sheets.is_empty() {
        let id: SheetId = project.fresh_id()?;
        project.sheets.push(Sheet::new(id, FIRST_SHEET_NAME, SheetSize::A4));
    }
    Ok(())
}

fn write(s: &mut Session, path: PathBuf) -> Result<Value> {
    protasena_io::save(&s.project, &path)?;
    s.path = Some(path.clone());
    s.dirty = false;
    s.touch();
    Ok(json!({ "path": path.display().to_string() }))
}

fn save(s: &mut Session, a: &Args) -> Result<Value> {
    let path = match path_param(a, "path")? {
        Some(p) => p,
        None => s.path.clone().ok_or_else(|| EngineError::Other("the project has no file yet: use save-as with a path".into()))?,
    };
    write(s, path)
}

fn save_as(s: &mut Session, a: &Args) -> Result<Value> {
    let path = path_param(a, "path")?.ok_or_else(|| a.bad("`path` (string) is required"))?;
    write(s, path)
}

fn export_netlist(s: &mut Session, a: &Args) -> Result<Value> {
    let format = match a.str("format")?.map(str::to_ascii_lowercase).as_deref() {
        None | Some("json") => NetlistFormat::Json,
        Some("spice") => NetlistFormat::Spice,
        Some(other) => return Err(a.bad(format!("`format` must be \"json\" or \"spice\", not {other:?}"))),
    };
    let path = match path_param(a, "path")? {
        Some(p) => p,
        None => s.path.as_deref().map(|p| with_extension(p, format)).ok_or_else(|| a.bad("`path` is required until the project has been saved"))?,
    };
    let netlist = protasena_io::export_netlist(&s.project, &s.library, &path, format)?;
    s.note(format!("Netlist written to {}", path.display()));
    Ok(json!({ "path": path.display().to_string(), "nets": netlist.nets.len(), "parts": netlist.parts.len(), "erc": netlist.erc.len() }))
}

fn with_extension(p: &Path, format: NetlistFormat) -> PathBuf {
    p.with_extension(format.extension())
}
