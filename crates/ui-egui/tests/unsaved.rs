//! Unsaved changes: Exit, the window's close button, New and Open ask Save / Don't Save /
//! Cancel while the design has unsaved changes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use std::path::PathBuf;

use common::*;
use egui::{Key, Modifiers, ViewportCommand, ViewportEvent, ViewportId};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use protasena_ui_egui::App;
use protasena_ui_egui::dialogs::{Dialog, PathPurpose};
use protasena_ui_egui::unsaved::Then;
use serde_json::json;

fn dirty() -> Harness<'static, App> {
    let mut h = harness(1600.0, 1000.0, 1.0);
    h.state_mut().run("junction", json!({ "at": [0, 0] })).unwrap();
    h.run_steps(2);
    assert!(h.state().session.is_dirty());
    h
}

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("unsaved");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    let _ = std::fs::remove_file(&p);
    p
}

/// The window manager asks to close the window; returns whether the app cancelled it.
fn close_window(h: &mut Harness<'_, App>) -> bool {
    h.input_mut().viewports.entry(ViewportId::ROOT).or_default().events.push(ViewportEvent::Close);
    h.step();
    let cancelled = h.output().viewport_output.get(&ViewportId::ROOT).is_some_and(|v| v.commands.contains(&ViewportCommand::CancelClose));
    h.run_steps(2);
    cancelled
}

fn click(h: &mut Harness<'_, App>, label: &str) {
    h.get_by_label(label).click();
    h.run_steps(3);
}

fn exit_from_menu(h: &mut Harness<'_, App>) {
    click(h, "File");
    click(h, "Exit Application");
}

fn asks(h: &Harness<'_, App>) -> Option<Then> {
    match &h.state().dialog {
        Some(Dialog::SaveChanges { then }) => Some(*then),
        _ => None,
    }
}

#[test]
fn without_changes_exit_and_the_close_button_close_at_once() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    assert!(!close_window(&mut h), "nothing to lose: the close goes ahead");
    assert!(h.state().dialog.is_none());
    exit_from_menu(&mut h);
    assert!(h.state().quit_requested && h.state().dialog.is_none());
}

#[test]
fn exit_asks_and_cancel_keeps_the_window_and_the_changes() {
    let mut h = dirty();
    exit_from_menu(&mut h);
    assert_eq!(asks(&h), Some(Then::Exit));
    assert!(!h.state().quit_requested);
    for button in ["Save", "Don't Save", "Cancel"] {
        assert!(find(&h, button, "button").enabled, "{button}");
    }
    click(&mut h, "Cancel");
    assert!(h.state().dialog.is_none() && !h.state().quit_requested && h.state().session.is_dirty());
}

#[test]
fn the_close_button_is_held_until_dont_save() {
    let mut h = dirty();
    assert!(close_window(&mut h), "the close is cancelled while the question is open");
    assert_eq!(asks(&h), Some(Then::Exit));
    click(&mut h, "Don't Save");
    assert!(h.state().quit_requested);
    assert!(!close_window(&mut h), "the close that follows goes ahead");
}

#[test]
fn save_writes_the_project_file_and_then_closes() {
    let path = scratch("closing.psproj");
    let mut h = dirty();
    h.state_mut().run("save-as", json!({ "path": path.display().to_string() })).unwrap();
    h.state_mut().run("junction", json!({ "at": [100, 0] })).unwrap();
    h.run_steps(2);
    assert!(close_window(&mut h));
    click(&mut h, "Save");
    assert!(h.state().quit_requested && !h.state().session.is_dirty());
    assert!(std::fs::read_to_string(&path).unwrap().contains("100"), "the second junction was saved");
}

#[test]
fn save_without_a_file_asks_for_one_and_cancelling_that_stays() {
    let mut h = dirty();
    exit_from_menu(&mut h);
    click(&mut h, "Save");
    assert!(matches!(&h.state().dialog, Some(Dialog::Path(p)) if p.purpose == PathPurpose::SaveAs));
    click(&mut h, "Cancel");
    assert!(h.state().dialog.is_none() && !h.state().quit_requested);
    let path = scratch("later.psproj");
    h.state_mut().run("save-as", json!({ "path": path.display().to_string() })).unwrap();
    h.run_steps(2);
    assert!(!h.state().quit_requested, "a later save does not close the window");
}

#[test]
fn save_through_the_path_prompt_closes_once_saved() {
    let path = scratch("prompted.psproj");
    let mut h = dirty();
    exit_from_menu(&mut h);
    click(&mut h, "Save");
    let Some(Dialog::Path(p)) = &mut h.state_mut().dialog else { panic!("no path prompt") };
    p.path = path.display().to_string();
    h.run_steps(2);
    click(&mut h, "OK");
    assert!(path.exists() && h.state().quit_requested);
}

#[test]
fn new_while_dirty_asks_and_dont_save_starts_afresh() {
    let mut h = dirty();
    press(&mut h, Modifiers::CTRL, Key::N);
    assert_eq!(asks(&h), Some(Then::New));
    click(&mut h, "Don't Save");
    assert!(!h.state().session.is_dirty());
    assert!(h.state().session.sheet().unwrap().junctions.is_empty());
    assert!(!h.state().quit_requested);
}

#[test]
fn open_while_dirty_asks_and_dont_save_goes_on_to_the_file() {
    let mut h = dirty();
    press(&mut h, Modifiers::CTRL, Key::O);
    assert_eq!(asks(&h), Some(Then::Open));
    click(&mut h, "Don't Save");
    assert!(matches!(&h.state().dialog, Some(Dialog::Path(p)) if p.purpose == PathPurpose::Open { discard: true }));
}

#[test]
fn quit_from_the_control_channel_does_not_ask() {
    let mut h = dirty();
    h.state_mut().quit_now();
    h.run_steps(2);
    assert!(h.state().quit_requested && h.state().dialog.is_none());
    assert!(!close_window(&mut h));
}
