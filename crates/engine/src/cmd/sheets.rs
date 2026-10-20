//! Design menu: Edit Sheet Properties, New (Root) Sheet, Remove/Delete Sheet, Goto Sheet and
//! the previous/next sheet keys (in hierarchy order: a root sheet, then its sub-sheets).

use protasena_doc::{Op, Sheet, SheetId, SheetSize};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::args::Args;
use crate::error::{EngineError, Result};
use crate::interact::Machine;
use crate::registry::CommandSpec;
use crate::request::Request;
use crate::session::Session;

/// Longest sheet name, in bytes.
const MAX_NAME: usize = 256;

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("sheet-properties", "Edit Sheet Properties", sheet_properties)
            .menu(&["Design", "Edit Sheet Properties"])
            .params("{sheet?: id|name|{index}, name?: string, size?: \"a4\"|\"a3\"|…|\"e\"|{\"custom\": {width, height}}}")
            .dialog(|s| Some(Request::SheetProperties { sheet: s.sheet })),
        CommandSpec::new("new-sheet", "New (Root) Sheet", new_sheet).menu(&["Design", "New (Root) Sheet"]).params("{name?: string}"),
        CommandSpec::new("remove-sheet", "Remove/Delete Sheet", remove_sheet)
            .menu(&["Design", "Remove/Delete Sheet"])
            .params("{sheet?: id|name|{index}} — the current sheet by default")
            .enabled(several_sheets),
        CommandSpec::new("previous-sheet", "Goto Previous Root or Sub-sheet", |s, _| step_sheet(s, -1))
            .menu(&["Design", "Goto Previous Root or Sub-sheet"])
            .key("Page-Up")
            .params("{}"),
        CommandSpec::new("next-sheet", "Goto Next Root or Sub-sheet", |s, _| step_sheet(s, 1))
            .menu(&["Design", "Goto Next Root or Sub-sheet"])
            .key("Page-Down")
            .params("{}"),
        CommandSpec::new("goto-sheet", "Goto Sheet", goto_sheet)
            .menu(&["Design", "Goto Sheet"])
            .params("{sheet: id|name|{index}}")
            .dialog(|_| Some(Request::GotoSheet)),
    ]
}

fn several_sheets(s: &Session) -> std::result::Result<(), String> {
    if s.project.sheets.len() > 1 { Ok(()) } else { Err("the design has only one sheet".into()) }
}

fn name_param(a: &Args) -> Result<Option<String>> {
    match a.str("name")?.map(str::trim) {
        None => Ok(None),
        Some("") => Err(a.bad("`name` is empty")),
        Some(n) if n.len() > MAX_NAME => Err(a.bad(format!("`name` is longer than {MAX_NAME} bytes"))),
        Some(n) => Ok(Some(n.to_owned())),
    }
}

fn size_param(a: &Args) -> Result<Option<SheetSize>> {
    let Some(v) = a.get("size") else { return Ok(None) };
    let v = match v {
        Value::String(s) => Value::String(s.to_ascii_lowercase()),
        other => other.clone(),
    };
    let size =
        SheetSize::deserialize(&v).map_err(|_| a.bad("`size` must be \"a4\", \"a3\", …, \"e\" or {\"custom\": {\"width\": …, \"height\": …}}"))?;
    let (w, h) = size.dimensions();
    if w > crate::args::MAX_COORD || h > crate::args::MAX_COORD {
        return Err(a.bad("`size` is too large"));
    }
    size.check().map_err(|e| a.bad(e.to_string()))?;
    Ok(Some(size))
}

fn sheet_properties(s: &mut Session, a: &Args) -> Result<Value> {
    let id = a.sheet(s, "sheet")?.unwrap_or(s.sheet);
    let sheet = s.project.sheet(id).ok_or_else(|| a.bad("no such sheet"))?;
    let name = name_param(a)?.unwrap_or_else(|| sheet.name.clone());
    let size = size_param(a)?.unwrap_or(sheet.size);
    if name == sheet.name && size == sheet.size {
        return Ok(json!({ "changed": false }));
    }
    s.commit("sheet-properties", vec![Op::SetSheet { id, name, size }])?;
    Ok(json!({ "changed": true }))
}

/// `Root sheet N` with the smallest N no sheet uses.
fn default_name(s: &Session) -> String {
    (1u64..)
        .map(|n| format!("Root sheet {n}"))
        .take(s.project.sheets.len().saturating_add(2))
        .find(|name| !s.project.sheets.iter().any(|sh| sh.name == *name))
        .unwrap_or_else(|| "Root sheet".to_owned())
}

/// Shows sheet `id` (the selection and the tool in progress are dropped).
pub(super) fn go_to(s: &mut Session, id: SheetId) {
    if s.sheet != id {
        s.sheet = id;
        s.selection.clear();
        s.machine = Machine::Idle;
    }
    s.touch();
}

fn new_sheet(s: &mut Session, a: &Args) -> Result<Value> {
    let name = name_param(a)?.unwrap_or_else(|| default_name(s));
    let size = s.sheet().map_or(SheetSize::A4, |sh| sh.size);
    let id: SheetId = s.fresh()?;
    let index = s.project.sheets.iter().position(|sh| sh.id == s.sheet).map(|i| i.saturating_add(1));
    s.commit("new-sheet", vec![Op::AddSheet { sheet: Sheet::new(id, &name, size), index }])?;
    go_to(s, id);
    Ok(json!({ "sheet": id.0, "name": name }))
}

fn remove_sheet(s: &mut Session, a: &Args) -> Result<Value> {
    let id = a.sheet(s, "sheet")?.unwrap_or(s.sheet);
    let at = s.project.sheets.iter().position(|sh| sh.id == id).ok_or_else(|| a.bad("no such sheet"))?;
    s.commit("remove-sheet", vec![Op::RemoveSheet { id }])?;
    if s.sheet == id || s.project.sheet(s.sheet).is_none() {
        let next = s.project.sheets.get(at).or_else(|| s.project.sheets.last()).map(|sh| sh.id);
        if let Some(next) = next {
            go_to(s, next);
        }
    }
    Ok(json!({ "removed": id.0, "sheet": s.sheet.0 }))
}

/// The previous or next sheet in hierarchy order (each root sheet followed by its sub-sheets).
fn step_sheet(s: &mut Session, step: isize) -> Result<Value> {
    let order = s.project.sheet_tree();
    let at = order.iter().position(|n| n.sheet == s.sheet).ok_or_else(|| EngineError::Other("no current sheet".into()))?;
    let target = at.checked_add_signed(step).and_then(|i| order.get(i)).map(|n| n.sheet);
    if let Some(id) = target {
        go_to(s, id);
    }
    Ok(json!({ "sheet": s.sheet.0 }))
}

fn goto_sheet(s: &mut Session, a: &Args) -> Result<Value> {
    let id = a.sheet(s, "sheet")?.ok_or_else(|| a.bad("`sheet` is required"))?;
    go_to(s, id);
    Ok(json!({ "sheet": id.0 }))
}
