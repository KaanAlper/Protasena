//! System settings (Proteus's System menu): per-user, not part of any design. They live in one
//! JSON file in the user's configuration directory ([`crate::settings_path`]).
//!
//! Reading never fails: the file is hostile, and a value that is missing, of the wrong type or
//! out of range falls back to its default with a warning, leaving every other value as read.

use std::collections::BTreeMap;

use protasena_doc::SheetSize;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Format tag of a settings file.
pub const SETTINGS_FORMAT: &str = "protasena-settings";
/// Version of the settings file this build writes.
pub const SETTINGS_VERSION: u32 = 1;
/// Largest settings file read: 1 MiB.
pub const MAX_SETTINGS_BYTES: u64 = 1024 * 1024;
/// Most entries of each list (sheet sizes, property definitions) and of the key map.
pub const MAX_ENTRIES: usize = 1000;
/// Longest string kept, in bytes.
pub const MAX_STRING: usize = 4096;
/// Most KiCad library paths kept.
pub const MAX_KICAD_LIBRARIES: usize = 256;

/// System ▸ Set Display Options.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayOptions {
    /// Draw the grid as lines instead of dots.
    pub grid_lines: bool,
    /// Zoom factor of one mouse-wheel notch (1.1 to 10).
    pub wheel_zoom: f64,
    /// How near the pointer must be to pick an object, in screen pixels (1 to 50).
    pub pick_pixels: f64,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self { grid_lines: false, wheel_zoom: 2.0, pick_pixels: 6.0 }
    }
}

/// One sheet size of System ▸ Set Sheet Sizes, in thou.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedSheetSize {
    /// Name shown in Edit Sheet Properties.
    pub name: String,
    /// Width in thou.
    pub width: f64,
    /// Height in thou.
    pub height: f64,
}

/// Smallest and largest sheet side accepted, in thou.
pub const SHEET_SIDE: (f64, f64) = (100.0, 1_000_000.0);

/// One property definition of System ▸ Set Property Definitions: a property name parts are
/// offered in Edit Component, with its default value and a description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyDefinition {
    /// Property name.
    pub name: String,
    /// Default value offered.
    #[serde(default)]
    pub default: String,
    /// What it is for.
    #[serde(default)]
    pub description: String,
}

/// System ▸ Set Text Editor: how multi-line texts are edited.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextEditorOptions {
    /// Font size in points (6 to 48).
    pub font_size: f64,
    /// A fixed-width font.
    pub monospace: bool,
}

impl Default for TextEditorOptions {
    fn default() -> Self {
        Self { font_size: 13.0, monospace: false }
    }
}

/// Every system setting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SystemSettings {
    /// System Settings: the parts library pack directory (used when neither `--library` nor
    /// `PROTASENA_LIBRARY_DIR` gives one).
    pub library_dir: Option<String>,
    /// System Settings: the folder the file choosers start in.
    pub project_dir: Option<String>,
    /// System Settings and the Library Manager: the user's own KiCad libraries, imported at
    /// run time (`.kicad_sym` files, `.pretty` folders, or folders holding them), in order.
    pub kicad_libraries: Vec<String>,
    /// Set Display Options.
    pub display: DisplayOptions,
    /// Set Sheet Sizes: the sizes Edit Sheet Properties offers.
    pub sheet_sizes: Vec<NamedSheetSize>,
    /// Set Keyboard Mapping: command id → key, where it differs from the default (an empty key
    /// unbinds the command).
    pub keys: BTreeMap<String, String>,
    /// Set Property Definitions.
    pub properties: Vec<PropertyDefinition>,
    /// Set Text Editor.
    pub text_editor: TextEditorOptions,
}

/// The built-in sheet sizes (ISO A4 to A0, ANSI A to E), landscape.
pub fn default_sheet_sizes() -> Vec<NamedSheetSize> {
    use SheetSize::*;
    [("A4", A4), ("A3", A3), ("A2", A2), ("A1", A1), ("A0", A0), ("A", A), ("B", B), ("C", C), ("D", D), ("E", E)]
        .into_iter()
        .map(|(name, s)| {
            let (width, height) = s.dimensions();
            NamedSheetSize { name: name.to_owned(), width, height }
        })
        .collect()
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            library_dir: None,
            project_dir: None,
            kicad_libraries: Vec::new(),
            display: DisplayOptions::default(),
            sheet_sizes: default_sheet_sizes(),
            keys: BTreeMap::new(),
            properties: Vec::new(),
            text_editor: TextEditorOptions::default(),
        }
    }
}

fn number_ok(v: f64, (lo, hi): (f64, f64)) -> bool {
    v.is_finite() && v >= lo && v <= hi
}

fn short(s: &str) -> bool {
    s.len() <= MAX_STRING
}

impl SystemSettings {
    /// The settings with every out-of-range value replaced by its default and invalid entries
    /// dropped; the warnings say what was changed.
    pub fn sanitized(mut self) -> (SystemSettings, Vec<String>) {
        let mut w = Vec::new();
        let d = DisplayOptions::default();
        if !number_ok(self.display.wheel_zoom, (1.1, 10.0)) {
            w.push(format!("display.wheel_zoom {} is out of range (1.1 to 10)", self.display.wheel_zoom));
            self.display.wheel_zoom = d.wheel_zoom;
        }
        if !number_ok(self.display.pick_pixels, (1.0, 50.0)) {
            w.push(format!("display.pick_pixels {} is out of range (1 to 50)", self.display.pick_pixels));
            self.display.pick_pixels = d.pick_pixels;
        }
        if !number_ok(self.text_editor.font_size, (6.0, 48.0)) {
            w.push(format!("text_editor.font_size {} is out of range (6 to 48)", self.text_editor.font_size));
            self.text_editor.font_size = TextEditorOptions::default().font_size;
        }
        for dir in [&mut self.library_dir, &mut self.project_dir] {
            if dir.as_deref().is_some_and(|d| d.trim().is_empty() || !short(d) || d.contains('\0')) {
                w.push("a folder setting is empty or not a usable path".to_owned());
                *dir = None;
            }
        }
        let before = self.kicad_libraries.len();
        let mut seen = std::collections::HashSet::new();
        self.kicad_libraries.retain(|p| !p.trim().is_empty() && short(p) && !p.contains('\0') && seen.insert(p.clone()));
        self.kicad_libraries.truncate(MAX_KICAD_LIBRARIES);
        if self.kicad_libraries.len() != before {
            w.push(format!("{} KiCad library paths were dropped (empty, repeated or too many)", before - self.kicad_libraries.len()));
        }
        let before = self.sheet_sizes.len();
        self.sheet_sizes.retain(|s| !s.name.trim().is_empty() && short(&s.name) && number_ok(s.width, SHEET_SIDE) && number_ok(s.height, SHEET_SIDE));
        self.sheet_sizes.truncate(MAX_ENTRIES);
        if self.sheet_sizes.len() != before {
            w.push(format!("{} sheet sizes were dropped (no name, or a side out of range)", before - self.sheet_sizes.len()));
        }
        if self.sheet_sizes.is_empty() {
            self.sheet_sizes = default_sheet_sizes();
        }
        let before = self.properties.len();
        self.properties.retain(|p| !p.name.trim().is_empty() && short(&p.name) && short(&p.default) && short(&p.description));
        self.properties.truncate(MAX_ENTRIES);
        if self.properties.len() != before {
            w.push(format!("{} property definitions were dropped", before - self.properties.len()));
        }
        let before = self.keys.len();
        self.keys.retain(|cmd, key| cmd.len() <= 64 && key.len() <= 64);
        self.keys = std::mem::take(&mut self.keys).into_iter().take(MAX_ENTRIES).collect();
        if self.keys.len() != before {
            w.push(format!("{} key bindings were dropped", before - self.keys.len()));
        }
        (self, w)
    }
}

/// One field of the settings object, or its default with a warning when it does not read.
fn field<T: DeserializeOwned + Default>(obj: &Map<String, Value>, key: &str, warnings: &mut Vec<String>) -> T {
    match obj.get(key) {
        None | Some(Value::Null) => T::default(),
        Some(v) => T::deserialize(v).unwrap_or_else(|e| {
            warnings.push(format!("setting {key} is ignored: {e}"));
            T::default()
        }),
    }
}

/// Reads settings file bytes leniently (see the module docs). Never fails.
pub fn decode_settings(bytes: &[u8]) -> (SystemSettings, Vec<String>) {
    let mut w = Vec::new();
    let defaults = |mut w: Vec<String>, why: String| {
        w.push(format!("{why}; the default settings are used"));
        (SystemSettings::default(), w)
    };
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_SETTINGS_BYTES {
        return defaults(w, format!("the settings file is larger than {MAX_SETTINGS_BYTES} bytes"));
    }
    if crate::limits::check_depth(bytes).is_err() {
        return defaults(w, "the settings file is nested too deeply".to_owned());
    }
    let root: Value = match serde_json::from_slice(bytes) {
        Ok(v) => v,
        Err(e) => return defaults(w, format!("the settings file is not JSON ({e})")),
    };
    let Some(root) = root.as_object() else { return defaults(w, "the settings file is not a JSON object".to_owned()) };
    match root.get("format") {
        Some(Value::String(f)) if f == SETTINGS_FORMAT => {}
        _ => return defaults(w, format!("the file is not tagged \"{SETTINGS_FORMAT}\"")),
    }
    if root.get("version").and_then(Value::as_u64).is_none_or(|v| v > u64::from(SETTINGS_VERSION)) {
        w.push("the settings file has another version; what can be read is used".to_owned());
    }
    let empty = Map::new();
    let obj = root.get("settings").and_then(Value::as_object).unwrap_or(&empty);
    let mut s = SystemSettings {
        library_dir: field(obj, "library_dir", &mut w),
        project_dir: field(obj, "project_dir", &mut w),
        kicad_libraries: field(obj, "kicad_libraries", &mut w),
        display: field(obj, "display", &mut w),
        sheet_sizes: field(obj, "sheet_sizes", &mut w),
        keys: field(obj, "keys", &mut w),
        properties: field(obj, "properties", &mut w),
        text_editor: field(obj, "text_editor", &mut w),
    };
    if !obj.contains_key("sheet_sizes") {
        s.sheet_sizes = default_sheet_sizes();
    }
    let (s, more) = s.sanitized();
    w.extend(more);
    (s, w)
}

#[derive(Serialize)]
struct FileOut<'a> {
    format: &'a str,
    version: u32,
    settings: &'a SystemSettings,
}

/// The settings as file text (pretty JSON with a final newline).
///
/// # Errors
/// [`crate::SaveError::Json`] if serialization fails (a NaN number, which sanitized settings
/// never hold).
pub fn encode_settings(s: &SystemSettings) -> Result<String, crate::SaveError> {
    let mut text =
        crate::pretty::to_string(&FileOut { format: SETTINGS_FORMAT, version: SETTINGS_VERSION, settings: s }).map_err(crate::SaveError::Json)?;
    text.push('\n');
    Ok(text)
}
