//! Toolbar, mode and status icons, drawn in code (our own pictograms; nothing is traced from
//! Proteus). Each icon is drawn in a 16 × 16 unit box, y down, scaled to the button.

mod chrome;
mod modes;

use std::cell::Cell;

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2};

/// Every icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Icon {
    // Global toolbar
    New,
    Open,
    Save,
    Close,
    Home,
    Schematic,
    Pcb,
    View3d,
    Gerber,
    Explorer,
    Bom,
    SourceCode,
    Notes,
    Help,
    // View toolbar
    Redraw,
    Grid,
    FalseOrigin,
    Center,
    ZoomIn,
    ZoomOut,
    ZoomAll,
    ZoomArea,
    // Edit toolbar
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    BlockCopy,
    BlockMove,
    BlockRotate,
    BlockDelete,
    PickParts,
    MakeDevice,
    Packaging,
    Decompose,
    // Design toolbar
    Autorouter,
    SearchTag,
    PropertyTool,
    NewSheet,
    RemoveSheet,
    ExitParent,
    Erc,
    // Mode toolbar
    Selection,
    Component,
    JunctionDot,
    WireLabel,
    TextScript,
    Buses,
    Subcircuit,
    Terminals,
    DevicePins,
    Graph,
    ActivePopup,
    Generator,
    Probe,
    Instruments,
    Line2d,
    Box2d,
    Circle2d,
    Arc2d,
    Path2d,
    Text2d,
    Symbols2d,
    Markers2d,
    // Orientation toolbar
    RotateCw,
    RotateCcw,
    MirrorX,
    MirrorY,
    // Status bar and tabs
    Play,
    Step,
    Pause,
    Stop,
    Info,
    Cross,
}

/// The colours an icon is drawn with.
#[derive(Clone, Copy, Debug)]
pub struct IconStyle {
    /// Main lines.
    pub ink: Color32,
    /// Highlights (the "new", "active" or "danger" part of a pictogram).
    pub accent: Color32,
    /// Fills behind outlines.
    pub fill: Color32,
}

/// Draws `icon` into `rect` (a square works best); returns how many shapes it drew.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, style: IconStyle) -> usize {
    let side = rect.width().min(rect.height());
    let box_ = Rect::from_center_size(rect.center(), egui::vec2(side, side));
    let pen = Pen { painter, origin: box_.min, unit: side / 16.0, style, count: Cell::new(0) };
    if !chrome::paint(&pen, icon) {
        modes::paint(&pen, icon);
    }
    pen.count.get()
}

/// Draws in the 16 × 16 icon box.
pub(crate) struct Pen<'a> {
    painter: &'a Painter,
    origin: Pos2,
    unit: f32,
    pub(crate) style: IconStyle,
    count: Cell<usize>,
}

impl Pen<'_> {
    fn at(&self, (x, y): (f32, f32)) -> Pos2 {
        pos2(self.origin.x + x * self.unit, self.origin.y + y * self.unit)
    }

    fn add(&self, shape: Shape) {
        self.count.set(self.count.get() + 1);
        self.painter.add(shape);
    }

    fn stroke(&self, color: Color32) -> Stroke {
        Stroke::new((1.3 * self.unit).max(1.0), color)
    }

    /// An open polyline in the ink colour.
    pub(crate) fn line(&self, pts: &[(f32, f32)]) {
        self.line_c(pts, self.style.ink);
    }

    /// An open polyline in the accent colour.
    pub(crate) fn accent(&self, pts: &[(f32, f32)]) {
        self.line_c(pts, self.style.accent);
    }

    /// An open polyline in `color`.
    pub(crate) fn line_c(&self, pts: &[(f32, f32)], color: Color32) {
        let points: Vec<Pos2> = pts.iter().map(|&p| self.at(p)).collect();
        self.add(Shape::line(points, self.stroke(color)));
    }

    /// A thick open polyline.
    pub(crate) fn thick(&self, pts: &[(f32, f32)], color: Color32) {
        let points: Vec<Pos2> = pts.iter().map(|&p| self.at(p)).collect();
        self.add(Shape::line(points, Stroke::new((2.6 * self.unit).max(2.0), color)));
    }

    /// A closed outline, filled with the fill colour.
    pub(crate) fn outline(&self, pts: &[(f32, f32)]) {
        let points: Vec<Pos2> = pts.iter().map(|&p| self.at(p)).collect();
        self.add(Shape::convex_polygon(points, self.style.fill, self.stroke(self.style.ink)));
    }

    /// A closed shape filled with `color`, no outline.
    pub(crate) fn solid(&self, pts: &[(f32, f32)], color: Color32) {
        let points: Vec<Pos2> = pts.iter().map(|&p| self.at(p)).collect();
        self.add(Shape::convex_polygon(points, color, Stroke::NONE));
    }

    /// A rectangle outline (filled with the fill colour).
    pub(crate) fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.outline(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
    }

    /// A dashed rectangle in the ink colour.
    pub(crate) fn dashed_rect(&self, x0: f32, y0: f32, x1: f32, y1: f32) {
        let pts = [self.at((x0, y0)), self.at((x1, y0)), self.at((x1, y1)), self.at((x0, y1)), self.at((x0, y0))];
        let dash = 2.0 * self.unit;
        for d in Shape::dashed_line(&pts, self.stroke(self.style.ink), dash, dash * 0.8) {
            self.add(d);
        }
    }

    /// A circle outline.
    pub(crate) fn circle(&self, c: (f32, f32), r: f32, color: Color32) {
        self.add(Shape::circle_stroke(self.at(c), r * self.unit, self.stroke(color)));
    }

    /// A filled disc.
    pub(crate) fn disc(&self, c: (f32, f32), r: f32, color: Color32) {
        self.add(Shape::circle_filled(self.at(c), r * self.unit, color));
    }

    /// An arc (degrees, screen orientation: 0 = right, 90 = down) as a polyline.
    pub(crate) fn arc(&self, c: (f32, f32), r: f32, from: f32, to: f32, color: Color32) -> (f32, f32) {
        let n = 16;
        let pts: Vec<(f32, f32)> = (0..=n)
            .map(|i| {
                let a = (from + (to - from) * i as f32 / n as f32).to_radians();
                (c.0 + r * a.cos(), c.1 + r * a.sin())
            })
            .collect();
        self.line_c(&pts, color);
        pts.last().copied().unwrap_or(c)
    }

    /// An arrow head at `tip` pointing along the angle `deg` (screen orientation).
    pub(crate) fn head(&self, tip: (f32, f32), deg: f32, color: Color32) {
        let a = deg.to_radians();
        let back = |d: f32| {
            let b = a + d.to_radians();
            (tip.0 - 3.2 * b.cos(), tip.1 - 3.2 * b.sin())
        };
        self.solid(&[tip, back(32.0), back(-32.0)], color);
    }

    /// A short text in the box (letters as pictograms).
    pub(crate) fn text(&self, c: (f32, f32), s: &str, size: f32, color: Color32) {
        let font = egui::FontId::proportional(size * self.unit);
        self.count.set(self.count.get() + 1);
        self.painter.text(self.at(c), egui::Align2::CENTER_CENTER, s, font, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_draws_something() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let style = IconStyle { ink: Color32::BLACK, accent: Color32::RED, fill: Color32::WHITE };
            let rect = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(24.0, 24.0));
            for icon in ALL {
                assert!(paint(ui.painter(), rect, icon, style) > 0, "{icon:?} draws nothing");
            }
        });
        out.textures_delta.clear();
    }

    /// Every icon, for the test above.
    const ALL: [Icon; 74] = {
        use Icon::*;
        [
            New,
            Open,
            Save,
            Close,
            Home,
            Schematic,
            Pcb,
            View3d,
            Gerber,
            Explorer,
            Bom,
            SourceCode,
            Notes,
            Help,
            Redraw,
            Grid,
            FalseOrigin,
            Center,
            ZoomIn,
            ZoomOut,
            ZoomAll,
            ZoomArea,
            Undo,
            Redo,
            Cut,
            Copy,
            Paste,
            BlockCopy,
            BlockMove,
            BlockRotate,
            BlockDelete,
            PickParts,
            MakeDevice,
            Packaging,
            Decompose,
            Autorouter,
            SearchTag,
            PropertyTool,
            NewSheet,
            RemoveSheet,
            ExitParent,
            Erc,
            Selection,
            Component,
            JunctionDot,
            WireLabel,
            TextScript,
            Buses,
            Subcircuit,
            Terminals,
            DevicePins,
            Graph,
            ActivePopup,
            Generator,
            Probe,
            Instruments,
            Line2d,
            Box2d,
            Circle2d,
            Arc2d,
            Path2d,
            Text2d,
            Symbols2d,
            Markers2d,
            RotateCw,
            RotateCcw,
            MirrorX,
            MirrorY,
            Play,
            Step,
            Pause,
            Stop,
            Info,
            Cross,
        ]
    };
}
