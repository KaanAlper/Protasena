//! The keyboard mapping (System ▸ Set Keyboard Mapping): each command's key is Proteus's default
//! shortcut unless the system settings bind it to another key or unbind it. No key may run two
//! commands.

use std::collections::{BTreeMap, HashMap};

use crate::registry::{CommandSpec, command_specs, find_command};

/// Keys that cannot be bound: Escape always cancels what is in progress.
const RESERVED: &[&str] = &["Escape"];

/// A key in the engine's notation, with its modifiers in a fixed order (`Ctrl+Alt+Shift+F5`,
/// `Page-Up`, `G`); `None` when it is not a key the editing window can send: a letter, a digit,
/// `F1`–`F24`, `Page-Up`, `Page-Down`, `Delete`, `Space` or `Enter`, with any of Ctrl, Alt and
/// Shift.
pub fn normalize_key(key: &str) -> Option<String> {
    let parts: Vec<String> = key.split('+').map(|p| p.trim().to_ascii_lowercase()).collect();
    let (base, mods) = parts.split_last()?;
    let (mut ctrl, mut alt, mut shift) = (false, false, false);
    for m in mods {
        match m.as_str() {
            "ctrl" | "control" | "cmd" => ctrl = true,
            "alt" => alt = true,
            "shift" => shift = true,
            _ => return None,
        }
    }
    let base = match base.as_str() {
        "page-up" | "pageup" | "pgup" => "Page-Up".to_owned(),
        "page-down" | "pagedown" | "pgdn" => "Page-Down".to_owned(),
        "delete" | "del" => "Delete".to_owned(),
        "space" => "Space".to_owned(),
        "enter" | "return" => "Enter".to_owned(),
        "escape" | "esc" => "Escape".to_owned(),
        b if b.len() == 1 && b.chars().all(|c| c.is_ascii_alphanumeric()) => b.to_ascii_uppercase(),
        b if b.starts_with('f') && b.get(1..).and_then(|n| n.parse::<u8>().ok()).is_some_and(|n| (1..=24).contains(&n)) && !b.starts_with("f0") => {
            b.to_ascii_uppercase()
        }
        _ => return None,
    };
    let mut s = String::new();
    for (on, name) in [(ctrl, "Ctrl+"), (alt, "Alt+"), (shift, "Shift+")] {
        if on {
            s.push_str(name);
        }
    }
    s.push_str(&base);
    Some(s)
}

/// Every command's key with these overrides (command id → key; an empty key unbinds it).
pub fn effective(overrides: &BTreeMap<String, String>) -> BTreeMap<&'static str, String> {
    command_specs()
        .iter()
        .filter_map(|spec: &CommandSpec| {
            let key = match overrides.get(spec.id) {
                Some(k) => normalize_key(k)?,
                None => normalize_key(spec.shortcut?)?,
            };
            Some((spec.id, key))
        })
        .collect()
}

/// Keys bound to more than one command, with the commands.
pub fn conflicts(map: &BTreeMap<&'static str, String>) -> Vec<(String, Vec<&'static str>)> {
    let mut by_key: BTreeMap<&str, Vec<&'static str>> = BTreeMap::new();
    for (id, key) in map {
        by_key.entry(key.as_str()).or_default().push(id);
    }
    by_key.into_iter().filter(|(_, ids)| ids.len() > 1).map(|(k, ids)| (k.to_owned(), ids)).collect()
}

/// Why an override cannot be used, if it cannot.
pub fn check_override(id: &str, key: &str) -> Result<(&'static str, String), String> {
    let spec = find_command(id).ok_or_else(|| format!("there is no command {id:?}"))?;
    if key.trim().is_empty() {
        return Ok((spec.id, String::new()));
    }
    let k = normalize_key(key).ok_or_else(|| {
        format!("{key:?} is not a key (a letter, digit, F1-F24, Page-Up, Page-Down, Delete, Space or Enter, with Ctrl, Alt or Shift)")
    })?;
    if RESERVED.contains(&k.as_str()) {
        return Err(format!("{k} is kept for cancelling"));
    }
    Ok((spec.id, k))
}

/// The usable overrides of a settings file: unknown commands, malformed keys and bindings that
/// would give a key to two commands are dropped (with warnings).
pub fn sanitize(overrides: &BTreeMap<String, String>) -> (BTreeMap<String, String>, Vec<String>) {
    let mut warnings = Vec::new();
    let mut out = BTreeMap::new();
    for (id, key) in overrides {
        match check_override(id, key) {
            Ok((id, key)) => {
                out.insert(id.to_owned(), key);
            }
            Err(e) => warnings.push(format!("keyboard mapping of {id}: {e}")),
        }
    }
    for (key, ids) in conflicts(&effective(&out)) {
        warnings.push(format!("keyboard mapping: {key} is bound to {}; the changed bindings are dropped", ids.join(", ")));
        for id in ids {
            out.remove(id);
        }
    }
    // Dropping an override can bring a default back into a clash; drop until none is left.
    while let Some((_, ids)) = conflicts(&effective(&out)).into_iter().next() {
        if !ids.iter().any(|id| out.remove(*id).is_some()) {
            break;
        }
    }
    (out, warnings)
}

/// The command a key runs with these overrides.
pub fn command_for(overrides: &BTreeMap<String, String>, key: &str) -> Option<&'static CommandSpec> {
    let k = normalize_key(key)?;
    let map = effective(overrides);
    let id = map.iter().find(|(_, v)| **v == k).map(|(id, _)| *id)?;
    find_command(id)
}

/// The keys of every command in a map, for quick lookups.
pub fn by_key(map: &BTreeMap<&'static str, String>) -> HashMap<String, &'static str> {
    map.iter().map(|(id, k)| (k.clone(), *id)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_normalize_to_one_spelling() {
        assert_eq!(normalize_key("shift+ctrl+f5").as_deref(), Some("Ctrl+Shift+F5"));
        assert_eq!(normalize_key(" Page-Up ").as_deref(), Some("Page-Up"));
        assert_eq!(normalize_key("g").as_deref(), Some("G"));
        for bad in ["", "+", "Ctrl+", "Hyper+G", "F0", "F25", "F05", "AB", "Ctrl+Ctrl", "ü"] {
            assert_eq!(normalize_key(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_defaults_have_no_conflicts_and_every_default_is_a_key() {
        let map = effective(&BTreeMap::new());
        assert!(conflicts(&map).is_empty(), "{:?}", conflicts(&map));
        let with_keys = command_specs().iter().filter(|c| c.shortcut.is_some()).count();
        assert_eq!(map.len(), with_keys);
    }

    #[test]
    fn overrides_rebind_unbind_and_clashes_are_dropped() {
        let mut o = BTreeMap::new();
        o.insert("zoom-in".to_owned(), "ctrl+shift+i".to_owned());
        o.insert("zoom-out".to_owned(), String::new());
        assert_eq!(command_for(&o, "Ctrl+Shift+I").map(|c| c.id), Some("zoom-in"));
        assert!(command_for(&o, "F7").is_none(), "unbound");
        assert!(command_for(&o, "F6").is_none(), "zoom-in moved away from F6");
        o.insert("redraw".to_owned(), "G".to_owned()); // G is Toggle Grid's
        o.insert("nope".to_owned(), "F9".to_owned());
        o.insert("undo".to_owned(), "Escape".to_owned());
        let (clean, warnings) = sanitize(&o);
        assert_eq!(warnings.len(), 3, "{warnings:?}");
        assert!(!clean.contains_key("redraw") && !clean.contains_key("nope") && !clean.contains_key("undo"));
        assert!(conflicts(&effective(&clean)).is_empty());
        assert_eq!(by_key(&effective(&clean)).get("G"), Some(&"toggle-grid"));
    }
}
