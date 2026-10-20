//! System menu: System Settings (folders), Set Display Options, Set Keyboard Mapping, Set
//! Property Definitions, Set Sheet Sizes, Set Text Editor and Restore Default Settings. They
//! change the per-user system settings, which are saved to the settings file at once (when the
//! session has one) and are not part of the design or of undo.

use std::collections::BTreeMap;

use protasena_io::{NamedSheetSize, PropertyDefinition, SystemSettings};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::args::Args;
use crate::error::{EngineError, Result};
use crate::keymap::{check_override, conflicts, effective};
use crate::patch;
use crate::registry::CommandSpec;
use crate::request::Request;
use crate::session::Session;

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("system-settings", "System Settings", folders)
            .menu(&["System", "System Settings"])
            .params("{library_dir?: path|null, project_dir?: path|null, kicad_libraries?: [path]} — null: none; kicad_libraries: the user's own KiCad libraries (.kicad_sym files, .pretty folders or folders holding them), imported at once")
            .dialog(|_| Some(Request::System { section: "folders" })),
        CommandSpec::new("import-kicad-libraries", "Import KiCad Libraries", |s, _| Ok(s.import_kicad()))
            .params("{} — imports the KiCad libraries of System Settings again (converted files are cached: only changed ones are converted)"),
        CommandSpec::new("set-display-options", "Set Display Options", |s, a| group(s, a, |x| &mut x.display))
            .menu(&["System", "Set Display Options"])
            .params("{grid_lines?: bool, wheel_zoom?: 1.1-10, pick_pixels?: 1-50, reset?: bool}")
            .dialog(|_| Some(Request::System { section: "display" })),
        CommandSpec::new("set-keyboard-mapping", "Set Keyboard Mapping", keyboard)
            .menu(&["System", "Set Keyboard Mapping"])
            .params("{bindings?: {command: key|\"\"|null}, reset?: bool} — \"\" unbinds, null restores the default; no key may run two commands")
            .dialog(|_| Some(Request::KeyboardMapping)),
        CommandSpec::new("set-property-definitions", "Set Property Definitions", properties)
            .menu(&["System", "Set Property Definitions"])
            .params("{definitions: [{name, default?, description?}]} — the whole list")
            .dialog(|_| Some(Request::PropertyDefinitions)),
        CommandSpec::new("set-sheet-sizes", "Set Sheet Sizes", sheet_sizes)
            .menu(&["System", "Set Sheet Sizes"])
            .params("{sizes?: [{name, width, height}] (thou), reset?: bool} — the whole list offered by Edit Sheet Properties")
            .dialog(|_| Some(Request::SheetSizes)),
        CommandSpec::new("set-text-editor", "Set Text Editor", |s, a| group(s, a, |x| &mut x.text_editor))
            .menu(&["System", "Set Text Editor"])
            .params("{font_size?: 6-48, monospace?: bool, reset?: bool}")
            .dialog(|_| Some(Request::System { section: "text_editor" })),
        CommandSpec::new("restore-default-settings", "Restore Default Settings", restore)
            .menu(&["System", "Restore Default Settings"])
            .params("{}")
            .dialog(|_| Some(Request::RestoreDefaults)),
    ]
}

/// Takes `new` as the settings when it is valid as given (nothing is silently changed), and
/// saves it.
fn take(s: &mut Session, a: &Args, new: SystemSettings) -> Result<Value> {
    let (clean, warnings) = new.clone().sanitized();
    if let Some(w) = warnings.first() {
        return Err(a.bad(w.clone()));
    }
    s.apply_settings(clean);
    Ok(s.save_settings())
}

fn group<T>(s: &mut Session, a: &Args, pick: fn(&mut SystemSettings) -> &mut T) -> Result<Value>
where
    T: serde::Serialize + serde::de::DeserializeOwned + Default + Clone,
{
    let mut new = s.settings.clone();
    let current = pick(&mut new).clone();
    *pick(&mut new) = patch::apply(&current, &a.raw(), a, &[])?;
    take(s, a, new)
}

fn folders(s: &mut Session, a: &Args) -> Result<Value> {
    let mut new = s.settings.clone();
    for (key, slot) in [("library_dir", &mut new.library_dir), ("project_dir", &mut new.project_dir)] {
        if a.is_null(key) {
            *slot = None;
        } else if let Some(v) = a.str(key)? {
            *slot = if v.trim().is_empty() { None } else { Some(v.trim().to_owned()) };
        }
    }
    if let Some(paths) = a.strings("kicad_libraries", protasena_io::MAX_KICAD_LIBRARIES)? {
        let mut seen = std::collections::HashSet::new();
        new.kicad_libraries = paths.iter().map(|p| p.trim()).filter(|p| !p.is_empty() && seen.insert(*p)).map(str::to_owned).collect();
    }
    let reimport = new.kicad_libraries != s.settings.kicad_libraries;
    let mut v = take(s, a, new)?;
    if reimport && let (Value::Object(out), Value::Object(more)) = (&mut v, s.import_kicad()) {
        out.extend(more);
    }
    Ok(v)
}

fn keyboard(s: &mut Session, a: &Args) -> Result<Value> {
    let mut keys = if a.bool("reset")?.unwrap_or(false) { BTreeMap::new() } else { s.settings.keys.clone() };
    if let Some(b) = a.get("bindings") {
        let b = b.as_object().ok_or_else(|| a.bad("`bindings` must be an object {command: key}"))?;
        for (id, key) in b {
            match key {
                Value::Null => {
                    let spec = crate::registry::find_command(id).ok_or_else(|| a.bad(format!("there is no command {id:?}")))?;
                    keys.remove(spec.id);
                }
                Value::String(k) => {
                    let (id, key) = check_override(id, k).map_err(|e| a.bad(e))?;
                    let default =
                        crate::registry::find_command(id).and_then(|c| c.shortcut).and_then(crate::keymap::normalize_key).unwrap_or_default();
                    if key == default {
                        keys.remove(id);
                    } else {
                        keys.insert(id.to_owned(), key);
                    }
                }
                _ => return Err(a.bad(format!("the key of {id} must be a string or null"))),
            }
        }
    }
    let clash = conflicts(&effective(&keys));
    if let Some((key, ids)) = clash.first() {
        return Err(a.bad(format!("{key} would run {}: give one of them another key", ids.join(" and "))));
    }
    let mut new = s.settings.clone();
    new.keys = keys;
    take(s, a, new)
}

fn properties(s: &mut Session, a: &Args) -> Result<Value> {
    let list = a.get("definitions").ok_or_else(|| a.bad("`definitions` is required"))?;
    let defs = Vec::<PropertyDefinition>::deserialize(list).map_err(|e| a.bad(e.to_string()))?;
    let mut seen = std::collections::HashSet::new();
    if let Some(d) = defs.iter().find(|d| !seen.insert(d.name.trim().to_ascii_uppercase())) {
        return Err(a.bad(format!("property {} is defined twice", d.name)));
    }
    let mut new = s.settings.clone();
    new.properties = defs.into_iter().map(|d| PropertyDefinition { name: d.name.trim().to_owned(), ..d }).collect();
    take(s, a, new)
}

fn sheet_sizes(s: &mut Session, a: &Args) -> Result<Value> {
    let mut new = s.settings.clone();
    if a.bool("reset")?.unwrap_or(false) {
        new.sheet_sizes = protasena_io::default_sheet_sizes();
    }
    if let Some(list) = a.get("sizes") {
        let sizes = Vec::<NamedSheetSize>::deserialize(list).map_err(|e| a.bad(e.to_string()))?;
        if sizes.is_empty() {
            return Err(a.bad("keep at least one sheet size"));
        }
        new.sheet_sizes = sizes;
    }
    take(s, a, new)
}

fn restore(s: &mut Session, _: &Args) -> Result<Value> {
    s.apply_settings(SystemSettings::default());
    let v = s.save_settings();
    if v.get("saved") == Some(&json!(false)) && s.settings_path.is_some() {
        let why = v.get("error").and_then(Value::as_str).unwrap_or_default();
        return Err(EngineError::Other(format!("the defaults are in use but could not be saved: {why}")));
    }
    Ok(v)
}
