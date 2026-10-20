//! Edit ▸ Find/Edit Component, Tool ▸ Global Annotator and Electrical Rules Check, Design ▸
//! Edit Design Properties.

use protasena_doc::{DesignProperties, Item, ItemId, Op};
use serde_json::{Value, json};

use crate::args::Args;
use crate::error::{EngineError, Result};
use crate::registry::CommandSpec;
use crate::request::Request;
use crate::session::Session;

/// Most properties set by one edit.
const MAX_PROPERTIES: usize = 1000;

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("edit-component", "Find/Edit Component", edit_component)
            .menu(&["Edit", "Find/Edit Component"])
            .key("E")
            .params("{component?: id|reference (default: the one selected part), reference?, value?, package?: string|null, unit?: n, properties?: {NAME: value|null}, show_reference?, show_value?: bool}")
            .dialog(edit_dialog),
        CommandSpec::new("annotate", "Global Annotator", annotate)
            .menu(&["Tool", "Global Annotator"])
            .params("{mode?: \"incremental\"|\"total\"} — incremental numbers unannotated parts; total renumbers all")
            .dialog(|_| Some(Request::GlobalAnnotator)),
        CommandSpec::new("erc", "Electrical Rules Check", erc).menu(&["Tool", "Electrical Rules Check"]).params("{}"),
        CommandSpec::new("design-properties", "Edit Design Properties", design_properties)
            .menu(&["Design", "Edit Design Properties"])
            .params("{name?, title?, doc_number?, revision?, author?, created?, modified?: string}")
            .dialog(|_| Some(Request::DesignProperties)),
    ]
}

fn selected_component(s: &Session) -> Option<protasena_doc::ComponentId> {
    match s.selection.as_slice() {
        [ItemId::Component(id)] => Some(*id),
        _ => None,
    }
}

fn edit_dialog(s: &Session) -> Option<Request> {
    Some(selected_component(s).map_or(Request::FindComponent, |id| Request::EditComponent { id }))
}

fn edit_component(s: &mut Session, a: &Args) -> Result<Value> {
    let id = match a.component(s, "component")? {
        Some(id) => id,
        None => selected_component(s).ok_or_else(|| a.bad("name the `component` (or select one part)"))?,
    };
    let mut c = s.project.component(id).cloned().ok_or_else(|| a.bad("no such component"))?;
    if let Some(r) = a.str("reference")? {
        c.reference = r.trim().to_owned();
    }
    if let Some(v) = a.str("value")? {
        c.value = v.to_owned();
    }
    if a.is_null("package") {
        c.package = None;
    } else if let Some(package) = a.str("package")? {
        c.package = Some(package.to_owned()).filter(|p| !p.is_empty());
    }
    if let Some(unit) = a.u64("unit")? {
        c.unit = u32::try_from(unit).map_err(|_| a.bad("`unit` is too large"))?;
    }
    if let Some(v) = a.get("properties") {
        let props = v.as_object().ok_or_else(|| a.bad("`properties` must be an object {NAME: value|null}"))?;
        if props.len() > MAX_PROPERTIES {
            return Err(a.bad(format!("`properties` has more than {MAX_PROPERTIES} entries")));
        }
        for (name, value) in props {
            if name.trim().is_empty() || name.len() > crate::args::MAX_TEXT {
                return Err(a.bad("property names must be non-empty"));
            }
            match value {
                Value::Null => {
                    c.properties.remove(name);
                }
                Value::String(v) if v.len() <= crate::args::MAX_TEXT => c.properties.set(name, v),
                _ => return Err(a.bad(format!("property {name:?} must be a string or null"))),
            }
        }
    }
    if let Some(show) = a.bool("show_reference")? {
        c.reference_label.visible = show;
    }
    if let Some(show) = a.bool("show_value")? {
        c.value_label.visible = show;
    }
    let reference = c.reference.clone();
    s.commit("edit-component", vec![Op::Modify { item: Item::Component(c) }])?;
    Ok(json!({ "id": id.0, "reference": reference }))
}

fn annotate(s: &mut Session, a: &Args) -> Result<Value> {
    let op = match a.str("mode")?.map(str::to_ascii_lowercase).as_deref() {
        None | Some("incremental") => s.project.annotate(),
        Some("total") => s.project.reannotate_all(),
        Some(other) => return Err(a.bad(format!("`mode` must be \"incremental\" or \"total\", not {other:?}"))),
    };
    let changed = match &op {
        Op::Batch { ops } => ops.len(),
        _ => 1,
    };
    s.commit("annotate", vec![op])?;
    Ok(json!({ "changed": changed }))
}

fn erc(s: &mut Session, _: &Args) -> Result<Value> {
    let netlist = protasena_netlist::build(&s.project, &s.library);
    let ok = netlist.erc.is_empty();
    s.note(if ok { "Electrical Rules Check: no errors".to_owned() } else { format!("Electrical Rules Check: {} findings", netlist.erc.len()) });
    let erc = serde_json::to_value(&netlist.erc).map_err(|e| EngineError::Other(e.to_string()))?;
    let unconnected = serde_json::to_value(&netlist.unconnected_pins).map_err(|e| EngineError::Other(e.to_string()))?;
    Ok(json!({ "ok": ok, "nets": netlist.nets.len(), "parts": netlist.parts.len(), "erc": erc, "unconnected": unconnected }))
}

fn design_properties(s: &mut Session, a: &Args) -> Result<Value> {
    let old = s.project.design.clone();
    let field = |key: &str, old: &str| -> Result<String> { Ok(a.str(key)?.map_or_else(|| old.to_owned(), str::to_owned)) };
    let design = DesignProperties {
        title: field("title", &old.title)?,
        doc_number: field("doc_number", &old.doc_number)?,
        revision: field("revision", &old.revision)?,
        author: field("author", &old.author)?,
        created: field("created", &old.created)?,
        modified: field("modified", &old.modified)?,
    };
    let name = field("name", &s.project.name)?;
    if design == old && name == s.project.name {
        return Ok(json!({ "changed": false }));
    }
    s.commit("design-properties", vec![Op::SetDesign { name, design }])?;
    Ok(json!({ "changed": true }))
}
