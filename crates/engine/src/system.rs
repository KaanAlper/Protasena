//! The session's system settings: reading them in from a settings file, the keys they bind,
//! and saving them after a System menu command.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use protasena_io::SystemSettings;
use serde_json::{Value, json};

use crate::keymap;
use crate::session::Session;

impl Session {
    /// The system settings in use.
    pub fn settings(&self) -> &SystemSettings {
        &self.settings
    }

    /// The settings file the System menu saves to, if any.
    pub fn settings_path(&self) -> Option<&Path> {
        self.settings_path.as_deref()
    }

    /// Uses `settings` (read from `path`, which later changes are saved to; `None` keeps them in
    /// memory). Values a settings file may hold but the engine cannot use — out-of-range
    /// numbers, bindings of unknown commands, a key given to two commands — are dropped; the
    /// warnings say which.
    pub fn set_settings(&mut self, settings: SystemSettings, path: Option<PathBuf>) -> Vec<String> {
        let (mut settings, mut warnings) = settings.sanitized();
        let (keys, more) = keymap::sanitize(&settings.keys);
        settings.keys = keys;
        warnings.extend(more);
        self.settings_path = path;
        self.apply_settings(settings);
        warnings
    }

    /// Makes `settings` the ones in use (they are valid).
    pub(crate) fn apply_settings(&mut self, settings: SystemSettings) {
        self.view.pick_pixels = settings.display.pick_pixels;
        self.settings = settings;
        self.touch();
    }

    /// Saves the settings to the settings file; the result says whether it worked (the
    /// settings stay in use either way).
    pub(crate) fn save_settings(&mut self) -> Value {
        let Some(path) = self.settings_path.clone() else { return json!({ "saved": false }) };
        match protasena_io::save_settings(&self.settings, &path) {
            Ok(()) => json!({ "saved": true, "path": path.display().to_string() }),
            Err(e) => {
                self.note(format!("The settings could not be saved: {e}"));
                json!({ "saved": false, "error": e.to_string() })
            }
        }
    }

    /// Every command's key as the keyboard mapping has it.
    pub fn key_map(&self) -> BTreeMap<&'static str, String> {
        keymap::effective(&self.settings.keys)
    }

    /// The key of command `id` (its shortcut text in menus), if it has one.
    pub fn shortcut(&self, id: &str) -> Option<String> {
        let spec = crate::registry::find_command(id)?;
        match self.settings.keys.get(spec.id) {
            Some(k) if k.is_empty() => None,
            Some(k) => keymap::normalize_key(k),
            None => spec.shortcut.map(str::to_owned),
        }
    }
}
