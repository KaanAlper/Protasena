//! A test window: the real app in an egui_kittest harness, with egui's own fonts.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use egui::vec2;
use egui_kittest::Harness;
use protasena_engine::Session;
use protasena_ui_egui::control::{self, Widget};
use protasena_ui_egui::{App, Services};

/// The window at `width` × `height` points and `scale` pixels per point, after a few frames.
pub fn harness(width: f32, height: f32, scale: f32) -> Harness<'static, App> {
    harness_with(width, height, scale, |_| {})
}

/// As [`harness`], after `setup` changed the app.
pub fn harness_with(width: f32, height: f32, scale: f32, setup: impl FnOnce(&mut App)) -> Harness<'static, App> {
    let mut app = App::new(Session::new(), Services::default());
    app.use_system_fonts(false);
    setup(&mut app);
    let mut h = Harness::builder().with_size(vec2(width, height)).with_pixels_per_point(scale).build_ui_state(
        |ui, app: &mut App| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    h
}

/// The controls drawn in the last frame.
pub fn widgets(h: &Harness<'_, App>) -> Vec<Widget> {
    control::widgets(&h.ctx)
}

/// The controls with `role`.
pub fn with_role(h: &Harness<'_, App>, role: &str) -> Vec<Widget> {
    widgets(h).into_iter().filter(|w| w.role == role).collect()
}

/// The control with `label` and `role`.
pub fn find(h: &Harness<'_, App>, label: &str, role: &str) -> Widget {
    widgets(h).into_iter().find(|w| w.label == label && w.role == role).unwrap_or_else(|| panic!("no {role} {label:?}"))
}

/// The area called `name`.
pub fn region(h: &Harness<'_, App>, name: &str) -> egui::Rect {
    find(h, name, "region").rect
}

/// Whether two rectangles share more than a hairline.
pub fn overlap(a: egui::Rect, b: egui::Rect) -> bool {
    let i = a.intersect(b);
    i.width() > 0.5 && i.height() > 0.5
}

/// Presses `key` with `modifiers` and lets the window handle it.
pub fn press(h: &mut Harness<'_, App>, modifiers: egui::Modifiers, key: egui::Key) {
    h.key_press_modifiers(modifiers, key);
    h.run_steps(3);
}

/// Types text into the focused field.
pub fn type_text(h: &mut Harness<'_, App>, text: &str) {
    h.event(egui::Event::Text(text.to_owned()));
    h.run_steps(3);
}
