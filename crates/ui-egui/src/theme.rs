//! Design tokens: Protasena's own palette. Every colour the window paints comes from here.
//! Proteus gives only the layout; its colours are never sampled (clean-room): the light theme is
//! a warm-neutral frame with a near-white paper sheet and cobalt accents, the dark theme a
//! graphite frame with a deep slate sheet.
//!
//! [`Tokens::get`] returns the palette of the theme in use; [`apply`] turns it into egui's
//! visuals. Inks of the drawing ([`protasena_render::Ink`]) map to colours in [`Tokens::ink`].

use std::cell::Cell;

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, Theme, Visuals};
use protasena_render::Ink;
use serde::{Deserialize, Serialize};

/// The interface theme the user chose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePref {
    /// The light theme (the default).
    #[default]
    Light,
    /// The dark theme.
    Dark,
}

impl ThemePref {
    /// egui's theme for it.
    pub fn theme(self) -> Theme {
        match self {
            ThemePref::Light => Theme::Light,
            ThemePref::Dark => Theme::Dark,
        }
    }
}

thread_local! {
    static ACTIVE: Cell<Theme> = const { Cell::new(Theme::Light) };
}

/// Makes `theme` the one [`Tokens::get`] answers for (set once per frame).
pub fn set_active(theme: Theme) {
    ACTIVE.with(|a| a.set(theme));
}

/// Every colour of the interface and the drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    /// Window frame: menu bar, toolbar band, status bar.
    pub chrome: Color32,
    /// Raised surfaces on the frame: toolbar groups, the active tab.
    pub chrome_hi: Color32,
    /// The left pane and lists.
    pub panel: Color32,
    /// Lines between areas.
    pub border: Color32,
    /// Separators inside toolbars and menus.
    pub separator: Color32,
    /// Text.
    pub text: Color32,
    /// Secondary text (captions, shortcuts).
    pub text_dim: Color32,
    /// Disabled text and icons.
    pub text_faint: Color32,
    /// Accent: active mode, pressed toggles, focus.
    pub accent: Color32,
    /// Accent background for selected rows and pressed buttons.
    pub accent_soft: Color32,
    /// Text on the accent colour.
    pub on_accent: Color32,
    /// Background of a control under the pointer.
    pub hover: Color32,
    /// Text fields.
    pub field: Color32,
    /// The editing window outside the sheet.
    pub canvas: Color32,
    /// The sheet.
    pub sheet: Color32,
    /// The sheet border.
    pub sheet_border: Color32,
    /// Grid dots and minor lines.
    pub grid: Color32,
    /// Major grid lines.
    pub grid_line: Color32,
    /// Component bodies and pins.
    pub body: Color32,
    /// Body fill of closed shapes.
    pub body_fill: Color32,
    /// Pin names.
    pub pin_name: Color32,
    /// Pin numbers.
    pub pin_number: Color32,
    /// References.
    pub reference: Color32,
    /// Values.
    pub value: Color32,
    /// Wires and junction dots.
    pub wire: Color32,
    /// Buses.
    pub bus: Color32,
    /// Wire labels.
    pub label: Color32,
    /// Terminals.
    pub terminal: Color32,
    /// Free text.
    pub text_item: Color32,
    /// Parts whose device is missing.
    pub missing: Color32,
    /// Selected objects.
    pub selection: Color32,
    /// Ghosts and rubber bands of the tool in use.
    pub preview: Color32,
    /// Package pads.
    pub pad: Color32,
    /// Package silkscreen.
    pub silk: Color32,
    /// The cursor cross.
    pub cursor: Color32,
    /// Warnings in the status bar and result windows.
    pub warn: Color32,
}

impl Tokens {
    /// The light theme.
    pub const LIGHT: Tokens = Tokens {
        chrome: hex(0xEEEDEA),
        chrome_hi: hex(0xF8F7F4),
        panel: hex(0xFCFBF9),
        border: hex(0xD9D5CE),
        separator: hex(0xCFCAC2),
        text: hex(0x1F2328),
        text_dim: hex(0x5B616A),
        text_faint: hex(0xA4A9B0),
        accent: hex(0x2F6FDB),
        accent_soft: hex(0xDCE7FA),
        on_accent: hex(0xFFFFFF),
        hover: hex(0xE3E1DC),
        field: hex(0xFFFFFF),
        canvas: hex(0xE4E1DB),
        sheet: hex(0xFBF9F6),
        sheet_border: hex(0x8E8A84),
        grid: hex(0xCFC9C0),
        grid_line: hex(0xE2DDD5),
        body: hex(0x2B3240),
        body_fill: hex(0xEEF3FA),
        pin_name: hex(0x4A5260),
        pin_number: hex(0x8A919C),
        reference: hex(0x1F2328),
        value: hex(0x4A5260),
        wire: hex(0x2A5DB0),
        bus: hex(0x7A4CC2),
        label: hex(0x0E7C86),
        terminal: hex(0xB7791F),
        text_item: hex(0x39404B),
        missing: hex(0xC2410C),
        selection: hex(0xF2711C),
        preview: hex(0x2F6FDB),
        pad: hex(0xC9A227),
        silk: hex(0x5B616A),
        cursor: hex(0x5B616A),
        warn: hex(0xB45309),
    };

    /// The dark theme.
    pub const DARK: Tokens = Tokens {
        chrome: hex(0x1F2227),
        chrome_hi: hex(0x282C32),
        panel: hex(0x22262B),
        border: hex(0x343940),
        separator: hex(0x3D434B),
        text: hex(0xE4E7EB),
        text_dim: hex(0xA2A9B3),
        text_faint: hex(0x646B75),
        accent: hex(0x5B8DEF),
        accent_soft: hex(0x2A3A5A),
        on_accent: hex(0xFFFFFF),
        hover: hex(0x2E333A),
        field: hex(0x181B1F),
        canvas: hex(0x15171A),
        sheet: hex(0x1D2024),
        sheet_border: hex(0x5C636D),
        grid: hex(0x3A3F46),
        grid_line: hex(0x2B3036),
        body: hex(0xC7D0DB),
        body_fill: hex(0x26303C),
        pin_name: hex(0xA2ABB6),
        pin_number: hex(0x7D8691),
        reference: hex(0xE4E7EB),
        value: hex(0xB4BCC6),
        wire: hex(0x6FA8F5),
        bus: hex(0xB794F4),
        label: hex(0x4FC3CF),
        terminal: hex(0xE0A84A),
        text_item: hex(0xD0D5DB),
        missing: hex(0xF08A4B),
        selection: hex(0xFF8F3F),
        preview: hex(0x5B8DEF),
        pad: hex(0xD4AF37),
        silk: hex(0xA2A9B3),
        cursor: hex(0xA2A9B3),
        warn: hex(0xF0B354),
    };

    /// The palette of `theme`.
    pub fn of(theme: Theme) -> Tokens {
        match theme {
            Theme::Light => Self::LIGHT,
            Theme::Dark => Self::DARK,
        }
    }

    /// The palette of the theme in use.
    pub fn get() -> Tokens {
        Self::of(ACTIVE.with(Cell::get))
    }

    /// The colour of a drawing ink.
    pub fn ink(&self, ink: Ink) -> Color32 {
        match ink {
            Ink::Body | Ink::Pin | Ink::BodyText => self.body,
            Ink::BodyFill => self.body_fill,
            Ink::PinName => self.pin_name,
            Ink::PinNumber => self.pin_number,
            Ink::Reference => self.reference,
            Ink::Value => self.value,
            Ink::Missing => self.missing,
            Ink::Wire | Ink::Junction => self.wire,
            Ink::Bus => self.bus,
            Ink::Label => self.label,
            Ink::Terminal | Ink::TerminalText => self.terminal,
            Ink::Text => self.text_item,
            Ink::Graphic => self.body,
            Ink::SheetBorder => self.sheet_border,
            Ink::Pad => self.pad,
            Ink::Silk => self.silk,
        }
    }
}

const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb(((rgb >> 16) & 0xFF) as u8, ((rgb >> 8) & 0xFF) as u8, (rgb & 0xFF) as u8)
}

/// Interface text size, in points.
pub const BODY_SIZE: f32 = 13.0;
/// Small text (captions, status fields, list rows), in points.
pub const SMALL_SIZE: f32 = 12.0;

/// The interface font.
pub fn body() -> FontId {
    FontId::new(BODY_SIZE, FontFamily::Proportional)
}

/// The small interface font.
pub fn small() -> FontId {
    FontId::new(SMALL_SIZE, FontFamily::Proportional)
}

/// Turns the palette into egui's visuals and text styles.
pub fn apply(ctx: &egui::Context, theme: Theme) {
    let t = Tokens::of(theme);
    let mut v = match theme {
        Theme::Light => Visuals::light(),
        Theme::Dark => Visuals::dark(),
    };
    let r = CornerRadius::same(4);
    v.panel_fill = t.chrome;
    v.window_fill = t.panel;
    v.faint_bg_color = t.chrome_hi;
    v.extreme_bg_color = t.field;
    v.window_stroke = Stroke::new(1.0, t.border);
    v.window_corner_radius = CornerRadius::same(8);
    v.menu_corner_radius = CornerRadius::same(6);
    v.selection.bg_fill = t.accent_soft;
    v.selection.stroke = Stroke::new(1.0, t.accent);
    v.hyperlink_color = t.accent;
    v.warn_fg_color = t.warn;
    for (w, bg) in [
        (&mut v.widgets.noninteractive, t.panel),
        (&mut v.widgets.inactive, t.chrome_hi),
        (&mut v.widgets.hovered, t.hover),
        (&mut v.widgets.active, t.accent_soft),
        (&mut v.widgets.open, t.hover),
    ] {
        w.bg_fill = bg;
        w.weak_bg_fill = bg;
        w.corner_radius = r;
        w.fg_stroke = Stroke::new(1.0, t.text);
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.text);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, t.border);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, t.separator);
    v.widgets.active.bg_stroke = Stroke::new(1.0, t.accent);
    ctx.set_visuals_of(theme, v);
    ctx.set_theme(theme);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(8.0, 3.0);
        s.spacing.interact_size.y = 22.0;
        s.text_styles.insert(egui::TextStyle::Body, body());
        s.text_styles.insert(egui::TextStyle::Button, body());
        s.text_styles.insert(egui::TextStyle::Small, small());
    });
}

/// Largest interface font file read.
const MAX_FONT_BYTES: u64 = 32 * 1024 * 1024;

/// Uses an installed sans-serif font for the interface when one is found (fonts are never
/// bundled), else keeps egui's own.
pub fn install_fonts(ctx: &egui::Context) {
    let candidates: &[&str] = &[
        "/usr/share/fonts/inter/Inter-Regular.ttf",
        "/usr/share/fonts/TTF/Inter-Regular.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
    ];
    let Some(bytes) = candidates.iter().find_map(|p| read_capped(std::path::Path::new(p))) else { return };
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("protasena-ui".into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.insert(0, "protasena-ui".into());
    }
    ctx.set_fonts(fonts);
}

fn read_capped(path: &std::path::Path) -> Option<Vec<u8>> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_FONT_BYTES {
        return None;
    }
    std::fs::read(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(c: Color32) -> f32 {
        (0.2126 * f32::from(c.r()) + 0.7152 * f32::from(c.g()) + 0.0722 * f32::from(c.b())) / 255.0
    }

    #[test]
    fn drawing_inks_stand_out_from_the_sheet_in_both_themes() {
        for t in [Tokens::LIGHT, Tokens::DARK] {
            for ink in [Ink::Body, Ink::Wire, Ink::Label, Ink::Reference, Ink::Terminal, Ink::Bus] {
                let d = (luminance(t.ink(ink)) - luminance(t.sheet)).abs();
                assert!(d > 0.3, "{ink:?} against the sheet: {d}");
            }
            assert!((luminance(t.text) - luminance(t.chrome)).abs() > 0.6);
            assert_ne!(t.selection, t.wire);
        }
    }

    #[test]
    fn the_light_sheet_is_a_calm_near_white() {
        let s = Tokens::LIGHT.sheet;
        assert!(luminance(s) > 0.95 && s.r() >= s.b(), "warm and light: {s:?}");
    }
}
