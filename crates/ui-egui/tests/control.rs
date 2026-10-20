//! The control channel as the app's transport uses it: requests on a channel, answered by the
//! window during its frames.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use std::time::Duration;

use common::*;
use protasena_ui_egui::ControlRequest;
use serde_json::{Value, json};

fn ask(
    h: &mut egui_kittest::Harness<'_, protasena_ui_egui::App>,
    tx: &std::sync::mpsc::Sender<ControlRequest>,
    method: &str,
    params: Value,
) -> Value {
    let (req, pending) = ControlRequest::new(method, params);
    tx.send(req).unwrap();
    h.run_steps(3);
    pending.wait(Duration::from_secs(5))
}

#[test]
fn requests_reach_the_engine_and_the_window() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut h = harness_with(1600.0, 1000.0, 1.0, |app| app.set_control(rx));
    assert_eq!(ask(&mut h, &tx, "ping", json!({})), json!({"ok": true, "result": "pong"}));
    let r = ask(&mut h, &tx, "engine.execute", json!({"command": "pick-device", "params": {"pick": "RES"}}));
    assert_eq!(r["result"]["picked"], "RES");
    let r = ask(&mut h, &tx, "engine.execute", json!({"command": "place-component", "params": {"at": [0, 0]}}));
    assert_eq!(r["result"]["reference"], "R1");
    assert_eq!(ask(&mut h, &tx, "engine.state", json!({}))["result"]["components"], 1);
    let r = ask(&mut h, &tx, "engine.execute", json!({"command": "no-such-command"}));
    assert_eq!(r["ok"], false);
    let menus = ask(&mut h, &tx, "ui.elements", json!({"role": "menu"}));
    assert_eq!(menus["result"].as_array().unwrap().len(), 11);
    let inspect = ask(&mut h, &tx, "ui.inspect", json!({}));
    assert!(inspect["result"]["regions"].as_array().unwrap().iter().any(|r| r["name"] == "Editing window"));
    assert_eq!(inspect["result"]["title"], "UNTITLED - Protasena - Schematic Capture");
    let r = ask(&mut h, &tx, "engine.invoke", json!({"command": "pick-device"}));
    assert_eq!(r["result"]["dialog"], "Pick Devices", "a menu pick opens the dialog");
    assert_eq!(ask(&mut h, &tx, "ui.dialog.close", json!({}))["ok"], true);
    let r = ask(&mut h, &tx, "ui.set", json!({"theme": "dark", "grid_style": "lines"}));
    assert_eq!(r["result"], json!({"theme": "dark", "grid_style": "lines"}));
    assert_eq!(ask(&mut h, &tx, "ui.set", json!({"theme": "purple"}))["ok"], false);
    assert_eq!(ask(&mut h, &tx, "frobnicate", json!({}))["ok"], false);
    assert_eq!(ask(&mut h, &tx, "ui.click", json!({"x": "left"}))["ok"], false, "hostile parameters are refused");
}

#[test]
fn a_key_sent_through_the_channel_runs_its_shortcut() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut h = harness_with(1600.0, 1000.0, 1.0, |app| app.set_control(rx));
    assert!(h.state().session.view().grid);
    assert_eq!(ask(&mut h, &tx, "ui.key", json!({"key": "G"}))["ok"], true);
    for _ in 0..4 {
        let mut raw = egui::RawInput::default();
        h.state_mut().raw_input_hook(&mut raw);
        for e in raw.events {
            h.event(e);
        }
        h.run_steps(1);
    }
    assert!(!h.state().session.view().grid, "the injected G reached Toggle Grid");
}
