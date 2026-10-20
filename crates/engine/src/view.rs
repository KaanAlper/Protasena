//! View state kept in the session for the UI: zoom, pan, grid, snap, cursor style, false
//! origin. None of it is part of the design or of undo.

use protasena_geom::{Pt, Rect, Vec2};
use serde::Serialize;

/// The View menu's snap settings (a radio group). Steps are in thou.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Snap {
    /// Snap 10th: 10 th.
    Th10,
    /// Snap 50th: 50 th.
    Th50,
    /// Snap 0.1in: 100 th (Proteus's default).
    #[default]
    In01,
    /// Snap 0.5in: 500 th.
    In05,
}

impl Snap {
    /// The snap step in thou.
    pub fn step(self) -> f64 {
        match self {
            Snap::Th10 => 10.0,
            Snap::Th50 => 50.0,
            Snap::In01 => 100.0,
            Snap::In05 => 500.0,
        }
    }
}

/// Smallest and largest zoom, in screen pixels per thou.
pub const ZOOM_RANGE: (f64, f64) = (1e-4, 10.0);

/// Factor of one Zoom In / Zoom Out step.
pub const ZOOM_STEP: f64 = 2.0;

/// Pick tolerance in screen pixels.
const PICK_PX: f64 = 6.0;

/// What the editing window shows.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct View {
    /// Sheet point at the middle of the editing window.
    pub center: Pt,
    /// Screen pixels per thou.
    pub zoom: f64,
    /// Toggle Grid (G).
    pub grid: bool,
    /// Snap setting.
    pub snap: Snap,
    /// Toggle X-Cursor (X): a cross-hair cursor instead of the small cross.
    pub x_cursor: bool,
    /// Toggle False Origin (O): where coordinates are measured from, when set.
    pub false_origin: Option<Pt>,
    /// Size of the editing window in pixels (set by the UI), for Zoom To View Entire Sheet.
    pub viewport: (f64, f64),
    /// Counts Redraw Display requests.
    pub redraws: u64,
}

impl Default for View {
    fn default() -> Self {
        Self {
            center: Pt::ORIGIN,
            zoom: 0.1,
            grid: true,
            snap: Snap::default(),
            x_cursor: false,
            false_origin: None,
            viewport: (1000.0, 700.0),
            redraws: 0,
        }
    }
}

impl View {
    /// The pick tolerance in thou at the current zoom.
    pub fn pick_tolerance(&self) -> f64 {
        (PICK_PX / self.zoom).clamp(2.0, 60.0)
    }

    /// Sets the zoom, clamped to [`ZOOM_RANGE`]; non-finite values are ignored.
    pub fn set_zoom(&mut self, zoom: f64) {
        if zoom.is_finite() && zoom > 0.0 {
            self.zoom = zoom.clamp(ZOOM_RANGE.0, ZOOM_RANGE.1);
        }
    }

    /// Shows `area` (a sheet rectangle) as large as fits in the viewport.
    pub fn fit(&mut self, area: Rect) {
        let (w, h) = (area.width().max(1.0), area.height().max(1.0));
        let (vw, vh) = (self.viewport.0.max(1.0), self.viewport.1.max(1.0));
        self.center = area.center();
        self.set_zoom((vw / w).min(vh / h) * 0.95);
    }

    /// Coordinates as the status bar shows them: relative to the false origin when one is set.
    pub fn display_coords(&self, p: Pt) -> Vec2 {
        p - self.false_origin.unwrap_or(Pt::ORIGIN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snaps_and_zoom_limits() {
        assert_eq!(Snap::default().step(), 100.0);
        assert_eq!(Snap::Th10.step(), 10.0);
        let mut v = View::default();
        v.set_zoom(f64::NAN);
        assert_eq!(v.zoom, 0.1);
        v.set_zoom(1e9);
        assert_eq!(v.zoom, ZOOM_RANGE.1);
        v.fit(Rect::new(Pt::new(-500.0, -350.0), Pt::new(500.0, 350.0)));
        assert!((v.zoom - 0.95).abs() < 1e-9);
        assert_eq!(v.center, Pt::ORIGIN);
        v.false_origin = Some(Pt::new(100.0, 0.0));
        assert_eq!(v.display_coords(Pt::new(150.0, 5.0)), Vec2::new(50.0, 5.0));
    }
}
