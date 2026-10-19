//! PCB packages (footprints): pads and drawing, in [thou](crate::units), y up.
//!
//! Kept small until PCB layout starts, but it holds what a footprint needs to be rebuilt:
//! pad positions, shapes, sizes, drills and layers, plus the drawing per layer.

use protasena_geom::{Pt, Vec2};
use serde::{Deserialize, Serialize};

use crate::symbol::Graphic;

/// Layer names used by the built-in packages (any other name is kept as is).
pub mod layer {
    /// Copper on every layer (through-hole pads).
    pub const ALL_COPPER: &str = "copper.*";
    /// Top copper.
    pub const TOP_COPPER: &str = "copper.top";
    /// Solder mask on both sides.
    pub const ALL_MASK: &str = "mask.*";
    /// Top silkscreen.
    pub const TOP_SILK: &str = "silk.top";
}

/// A footprint.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Package {
    /// Package name, unique in the pack (`RES40`, `DIL08`, …).
    pub name: String,
    /// The pack library this package belongs to (filled in by the loader).
    #[serde(default)]
    pub library: String,
    /// Short description in our own words.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Pads.
    #[serde(default, deserialize_with = "crate::bounded::pads")]
    pub pads: Vec<Pad>,
    /// Drawing (silkscreen, courtyard, assembly outline, …).
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "crate::bounded::layer_graphics")]
    pub graphics: Vec<LayerGraphic>,
}

impl Package {
    /// The pad with this name.
    pub fn pad(&self, name: &str) -> Option<&Pad> {
        self.pads.iter().find(|p| p.name == name)
    }
}

/// A graphic on one layer of a package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerGraphic {
    /// Layer name.
    pub layer: String,
    /// The drawing.
    pub graphic: Graphic,
}

/// A pad.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pad {
    /// Pad name (usually its number); what device pin maps refer to.
    pub name: String,
    /// Centre.
    pub at: Pt,
    /// Rotation in degrees, counter-clockwise.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub angle: f64,
    /// Copper shape.
    pub shape: PadShape,
    /// Width and height (before rotation).
    pub size: Vec2,
    /// Drill diameter for through-hole pads; `None` for surface-mount pads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drill: Option<f64>,
    /// Layers the pad is on.
    #[serde(default, deserialize_with = "crate::bounded::strings")]
    pub layers: Vec<String>,
}

impl Pad {
    /// A through-hole pad on all copper and mask layers.
    pub fn through_hole(name: &str, at: Pt, shape: PadShape, size: f64, drill: f64) -> Self {
        Self {
            name: name.to_owned(),
            at,
            angle: 0.0,
            shape,
            size: Vec2::new(size, size),
            drill: Some(drill),
            layers: vec![layer::ALL_COPPER.to_owned(), layer::ALL_MASK.to_owned()],
        }
    }

    /// Whether every number of the pad is finite and the sizes are not negative.
    pub fn is_sane(&self) -> bool {
        let drill_ok = self.drill.is_none_or(|d| d.is_finite() && d >= 0.0);
        self.at.is_finite() && self.angle.is_finite() && self.size.is_finite() && self.size.x >= 0.0 && self.size.y >= 0.0 && drill_ok
    }
}

/// Copper shape of a pad.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PadShape {
    /// Circle (diameter = size.x).
    Circle,
    /// Rectangle.
    Rect,
    /// Oval / obround.
    Oval,
    /// Rectangle with rounded corners; `ratio` = corner radius / shorter side.
    RoundRect {
        /// Corner radius as a fraction of the shorter side (0..=0.5).
        ratio: f64,
    },
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}
