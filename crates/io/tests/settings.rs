//! System settings files: round trip, the configuration directory, and hostile contents that
//! never stop the application from starting.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use std::path::PathBuf;

use common::scratch;
use protasena_io::*;

#[test]
fn settings_round_trip_through_their_file() {
    let dir = scratch("settings-roundtrip");
    let path = dir.join("deep").join("settings.json");
    let mut s = SystemSettings { library_dir: Some("/opt/pack".into()), ..SystemSettings::default() };
    s.display.grid_lines = true;
    s.keys.insert("zoom-in".into(), "Ctrl+Z".into());
    s.keys.insert("undo".into(), String::new());
    s.properties.push(PropertyDefinition { name: "STOCK".into(), default: "-".into(), description: "Stock code".into() });
    s.sheet_sizes.push(NamedSheetSize { name: "Letter tall".into(), width: 8500.0, height: 11000.0 });
    s.text_editor.monospace = true;
    save_settings(&s, &path).unwrap();
    let (back, warnings) = load_settings(&path);
    assert_eq!(back, s);
    assert!(warnings.is_empty(), "{warnings:?}");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("{\n  \"format\": \"protasena-settings\",\n  \"version\": 1,"), "{text}");
}

#[test]
fn a_missing_file_gives_the_defaults_silently() {
    let (s, w) = load_settings(&scratch("settings-missing").join("none.json"));
    assert_eq!(s, SystemSettings::default());
    assert!(w.is_empty());
    assert_eq!(s.sheet_sizes.len(), 10);
}

#[test]
fn hostile_contents_fall_back_value_by_value() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("binary", vec![0xff, 0x00, 0xfe, 0x7b]),
        ("empty", Vec::new()),
        ("array", b"[1, 2, 3]".to_vec()),
        ("foreign", br#"{"format": "something-else", "settings": {}}"#.to_vec()),
        ("deep", format!("{}{}", "[".repeat(10_000), "]".repeat(10_000)).into_bytes()),
        ("huge", vec![b' '; (MAX_SETTINGS_BYTES + 1) as usize]),
        ("truncated", br#"{"format": "protasena-settings", "version": 1, "settings": {"display": {"#.to_vec()),
    ];
    for (name, bytes) in cases {
        let (s, w) = decode_settings(&bytes);
        assert_eq!(s, SystemSettings::default(), "{name}");
        assert!(!w.is_empty(), "{name}: a warning says why");
    }
    let mixed = format!(
        r#"{{"format": "protasena-settings", "version": 99, "settings": {{
            "library_dir": 5, "project_dir": "   ",
            "display": {{"grid_lines": true, "wheel_zoom": 1e308, "pick_pixels": -3}},
            "sheet_sizes": [{{"name": "", "width": 1, "height": 1}}, {{"name": "OK", "width": 5000, "height": 4000}}, {{"name": "X", "width": "wide"}}],
            "keys": {{"zoom-in": "F9", "{long}": "F1"}},
            "properties": [{{"name": "A"}}, {{"name": " "}}],
            "text_editor": {{"font_size": 1000, "monospace": "yes"}},
            "unknown": [1, 2]
        }}}}"#,
        long = "c".repeat(100)
    );
    let (s, w) = decode_settings(mixed.as_bytes());
    assert_eq!(s.library_dir, None);
    assert_eq!(s.project_dir, None);
    assert!(s.display.grid_lines, "the good values of a group are kept");
    assert_eq!((s.display.wheel_zoom, s.display.pick_pixels), (2.0, 6.0));
    assert_eq!(s.sheet_sizes, default_sheet_sizes(), "a list with a malformed entry is ignored whole");
    assert_eq!(s.keys.len(), 1);
    assert_eq!(s.properties.len(), 1);
    assert_eq!(s.text_editor, TextEditorOptions::default(), "a wrong type drops the group");
    assert!(w.len() >= 6, "{w:#?}");
}

#[test]
fn unreadable_paths_give_the_defaults_with_a_warning() {
    let dir = scratch("settings-dir");
    let (s, w) = load_settings(&dir);
    assert_eq!(s, SystemSettings::default());
    assert_eq!(w.len(), 1, "a directory is no settings file");
    std::fs::write(dir.join("blocker"), b"x").unwrap();
    let err = save_settings(&SystemSettings::default(), &dir.join("blocker").join("settings.json"));
    assert!(err.is_err(), "a file in the way of the directory is an error, not a panic");
}

#[test]
fn the_configuration_directory_follows_the_platform_and_the_override() {
    let env = |pairs: &'static [(&'static str, &'static str)]| move |k: &str| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| (*v).to_owned());
    assert_eq!(config_dir_from(env(&[("PROTASENA_CONFIG_DIR", "/cfg"), ("HOME", "/home/u")]), "linux"), Some(PathBuf::from("/cfg")));
    assert_eq!(config_dir_from(env(&[("HOME", "/home/u")]), "linux"), Some(PathBuf::from("/home/u/.config/protasena")));
    assert_eq!(config_dir_from(env(&[("XDG_CONFIG_HOME", "/x"), ("HOME", "/h")]), "linux"), Some(PathBuf::from("/x/protasena")));
    assert_eq!(config_dir_from(env(&[("XDG_CONFIG_HOME", "rel"), ("HOME", "/h")]), "linux"), Some(PathBuf::from("/h/.config/protasena")));
    assert_eq!(config_dir_from(env(&[("APPDATA", "C:\\AppData")]), "windows"), Some(PathBuf::from("C:\\AppData").join("Protasena")));
    assert_eq!(config_dir_from(env(&[("HOME", "/Users/u")]), "macos"), Some(PathBuf::from("/Users/u/Library/Application Support/Protasena")));
    assert_eq!(config_dir_from(env(&[("PROTASENA_CONFIG_DIR", "  ")]), "linux"), None);
}
