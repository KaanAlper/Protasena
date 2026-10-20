//! Keyboard shortcuts: keys pressed while no text field has the keyboard go to the engine in
//! its shortcut notation (`Ctrl+Z`, `F6`, `G`, `Page-Up`), which runs the bound command
//! ([`protasena_engine::Session::key`]); Escape also closes the open dialog.

use crate::App;

/// The engine's name for a key with modifiers (`Ctrl+F1`, `Shift+Pause`, `Page-Down`, `G`).
pub fn shortcut_name(key: egui::Key, m: egui::Modifiers) -> Option<String> {
    use egui::Key;
    let base = match key {
        Key::PageUp => "Page-Up".to_owned(),
        Key::PageDown => "Page-Down".to_owned(),
        Key::Delete => "Delete".to_owned(),
        Key::Escape => "Escape".to_owned(),
        Key::Space => "Space".to_owned(),
        Key::Enter => "Enter".to_owned(),
        k => {
            let name = k.name();
            let usable = (name.len() == 1 && name.chars().all(|c| c.is_ascii_alphanumeric()))
                || (name.starts_with('F') && name.len() <= 3 && name.chars().skip(1).all(|c| c.is_ascii_digit()));
            if !usable {
                return None;
            }
            name.to_ascii_uppercase()
        }
    };
    let mut s = String::new();
    if m.ctrl || m.command {
        s.push_str("Ctrl+");
    }
    if m.alt {
        s.push_str("Alt+");
    }
    if m.shift {
        s.push_str("Shift+");
    }
    s.push_str(&base);
    Some(s)
}

/// Sends this frame's key presses to the engine.
pub(crate) fn handle(app: &mut App, ctx: &egui::Context) {
    let presses: Vec<(egui::Key, egui::Modifiers)> = ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Key { key, pressed: true, modifiers, .. } => Some((*key, *modifiers)),
                _ => None,
            })
            .collect()
    });
    if presses.is_empty() {
        return;
    }
    for (key, m) in presses {
        if key == egui::Key::Escape && app.dialog.is_some() {
            app.dialog = None;
            continue;
        }
        if app.dialog.is_some() || ctx.egui_wants_keyboard_input() {
            continue;
        }
        if key == egui::Key::Escape && app.canvas.zoom_area.is_some() {
            app.canvas.zoom_area = None;
            continue;
        }
        let Some(name) = shortcut_name(key, m) else { continue };
        match app.session.key(&name) {
            Ok(fb) => app.feedback(fb),
            Err(e) => app.notify(e.to_string(), true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Key, Modifiers};

    #[test]
    fn keys_get_the_engine_s_names() {
        assert_eq!(shortcut_name(Key::F6, Modifiers::NONE).as_deref(), Some("F6"));
        assert_eq!(shortcut_name(Key::G, Modifiers::NONE).as_deref(), Some("G"));
        assert_eq!(shortcut_name(Key::Z, Modifiers::CTRL).as_deref(), Some("Ctrl+Z"));
        assert_eq!(shortcut_name(Key::F1, Modifiers::COMMAND).as_deref(), Some("Ctrl+F1"));
        assert_eq!(shortcut_name(Key::PageUp, Modifiers::NONE).as_deref(), Some("Page-Up"));
        assert_eq!(shortcut_name(Key::F12, Modifiers::ALT).as_deref(), Some("Alt+F12"));
        assert_eq!(shortcut_name(Key::ArrowLeft, Modifiers::NONE), None);
    }

    #[test]
    fn every_shortcut_of_the_engine_can_be_typed() {
        for spec in protasena_engine::command_specs() {
            let Some(sc) = spec.shortcut else { continue };
            let base = sc.rsplit('+').next().unwrap_or(sc);
            let key = crate::control::key_from(base).unwrap_or_else(|| panic!("{}: {sc}", spec.id));
            let m = Modifiers { ctrl: sc.contains("Ctrl+"), shift: sc.contains("Shift+"), alt: sc.contains("Alt+"), ..Modifiers::NONE };
            let name = shortcut_name(key, m).unwrap_or_default();
            assert_eq!(protasena_engine::command_for_key(&name).map(|c| c.id), Some(spec.id), "{sc} → {name}");
        }
    }
}
