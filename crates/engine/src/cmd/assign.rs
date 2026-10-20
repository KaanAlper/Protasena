//! Tool ▸ Property Assignment Tool (A): sets, renames or removes a property on many
//! components at once, as one undo step. Built the conventional way (Proteus's dialog was not
//! observable):
//! - the string is `NAME=VALUE` (assign), `OLD=NEW` (rename) or `NAME` (remove);
//! - a `#` in an assigned value is replaced by a counter that starts at `count` and steps by
//!   `increment` from one part to the next (`REF=R#` numbers the parts R1, R2, …);
//! - it applies to the tagged components (the selection), every component of the current
//!   sheet, or every component of the design;
//! - `REF`, `VALUE` and `PACKAGE` are the part's own fields; the device cannot be changed here.

use protasena_doc::{Component, Item, ItemId, Op};
use serde_json::{Value, json};

use super::ids_json;
use crate::args::Args;
use crate::error::{EngineError, Result};
use crate::registry::CommandSpec;
use crate::request::Request;
use crate::session::Session;

/// Longest property name, in bytes.
const MAX_NAME: usize = 256;

/// Largest counter start or step.
const MAX_COUNT: i64 = 1_000_000_000;

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("property-assign", "Property Assignment Tool", property_assign)
            .menu(&["Tool", "Property Assignment Tool"])
            .key("A")
            .params("{string: \"NAME=VALUE\"|\"OLD=NEW\"|\"NAME\", action?: \"assign\"|\"rename\"|\"remove\", apply_to?: \"tagged\"|\"sheet\"|\"all\", count?: n, increment?: n} — `#` in a value counts up from `count`")
            .dialog(|_| Some(Request::PropertyAssign)),
    ]
}

/// What the tool does.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Assign { name: String, value: String },
    Rename { from: String, to: String },
    Remove { name: String },
}

/// Which field a name stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Reference,
    Value,
    Package,
    Device,
    Other,
}

fn field(name: &str) -> Field {
    match name.to_ascii_uppercase().as_str() {
        "REF" | "REFERENCE" => Field::Reference,
        "VALUE" => Field::Value,
        "PACKAGE" => Field::Package,
        "DEVICE" => Field::Device,
        _ => Field::Other,
    }
}

fn name_param(a: &Args, name: &str) -> Result<String> {
    let n = name.trim();
    if n.is_empty() || n.len() > MAX_NAME || n.chars().any(char::is_control) {
        return Err(a.bad(format!("a property name must be 1 to {MAX_NAME} bytes without control characters, not {n:?}")));
    }
    Ok(n.to_owned())
}

fn action_param(a: &Args) -> Result<Action> {
    let string = a.str_req("string")?;
    let kind = a.str("action")?.unwrap_or("assign").trim().to_ascii_lowercase();
    let split = string.split_once('=');
    let action = match (kind.as_str(), split) {
        ("assign", Some((n, v))) => Action::Assign { name: name_param(a, n)?, value: v.to_owned() },
        ("rename", Some((f, t))) => Action::Rename { from: name_param(a, f)?, to: name_param(a, t)? },
        ("remove", None) => Action::Remove { name: name_param(a, string)? },
        ("assign" | "rename", None) => return Err(a.bad(format!("`string` must be NAME=VALUE to {kind}"))),
        ("remove", Some(_)) => return Err(a.bad("`string` must be a property name (no =) to remove it")),
        _ => return Err(a.bad(format!("unknown `action` {kind:?}: assign, rename or remove"))),
    };
    let names: Vec<&str> = match &action {
        Action::Assign { name, .. } | Action::Remove { name } => vec![name],
        Action::Rename { from, to } => vec![from, to],
    };
    for n in names {
        match (field(n), &action) {
            (Field::Device, _) => return Err(a.bad("the device cannot be changed here: use Edit Component")),
            (Field::Reference | Field::Value | Field::Package, Action::Rename { .. }) => {
                return Err(a.bad(format!("{n} is a field of every part and cannot be renamed")));
            }
            (Field::Reference | Field::Value, Action::Remove { .. }) => return Err(a.bad(format!("{n} cannot be removed"))),
            _ => {}
        }
    }
    Ok(action)
}

fn count_param(a: &Args, key: &str, default: i64) -> Result<i64> {
    match a.get(key) {
        None => Ok(default),
        Some(v) => v.as_i64().filter(|n| n.abs() <= MAX_COUNT).ok_or_else(|| a.bad(format!("`{key}` must be a whole number within ±{MAX_COUNT}"))),
    }
}

/// The components to change, in order: the tagged ones in tagging order, or the sheet's or the
/// design's in list order.
fn targets(s: &Session, a: &Args) -> Result<Vec<Component>> {
    let apply_to = a.str("apply_to")?.unwrap_or("tagged").trim().to_ascii_lowercase();
    let list: Vec<Component> = match apply_to.as_str() {
        "tagged" => s.selection.iter().filter_map(|id| if let ItemId::Component(c) = id { s.project.component(*c).cloned() } else { None }).collect(),
        "sheet" => s.current_sheet()?.components.clone(),
        "all" => s.project.components().map(|(_, c)| c.clone()).collect(),
        _ => return Err(a.bad(format!("unknown `apply_to` {apply_to:?}: tagged, sheet or all"))),
    };
    if list.is_empty() {
        let why = if apply_to == "tagged" { "no component is tagged: tag some with Search & Tag first" } else { "there are no components" };
        return Err(EngineError::Other(why.into()));
    }
    Ok(list)
}

/// `c` with the action applied (the counter already put into the value).
fn changed(c: &Component, action: &Action) -> Component {
    let mut c = c.clone();
    match action {
        Action::Assign { name, value } => match field(name) {
            Field::Reference => c.reference = value.clone(),
            Field::Value => c.value = value.clone(),
            Field::Package => c.package = Some(value.clone()).filter(|p| !p.is_empty()),
            Field::Device | Field::Other => c.properties.set(name, value),
        },
        Action::Rename { from, to } => {
            if let Some(v) = c.properties.remove(from) {
                c.properties.remove(to);
                c.properties.set(to, &v);
            }
        }
        Action::Remove { name } => match field(name) {
            Field::Package => c.package = None,
            _ => {
                c.properties.remove(name);
            }
        },
    }
    c
}

fn property_assign(s: &mut Session, a: &Args) -> Result<Value> {
    let action = action_param(a)?;
    let (mut counter, step) = (count_param(a, "count", 1)?, count_param(a, "increment", 1)?);
    let mut ops = Vec::new();
    let mut ids = Vec::new();
    for c in targets(s, a)? {
        let this = match &action {
            Action::Assign { name, value } if value.contains('#') => {
                let v = value.replace('#', &counter.to_string());
                counter = counter.saturating_add(step);
                Action::Assign { name: name.clone(), value: v }
            }
            other => other.clone(),
        };
        let new = changed(&c, &this);
        if new != c {
            ids.push(ItemId::Component(c.id));
            ops.push(Op::Modify { item: Item::Component(new) });
        }
    }
    s.commit("property-assign", ops)?;
    Ok(json!({ "changed": ids.len(), "ids": ids_json(&ids) }))
}
