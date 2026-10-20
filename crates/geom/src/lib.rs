//! Protasena geometry (layer L0): double-precision 2D math for schematics.
//!
//! Points and vectors ([`Pt`], [`Vec2`]), rectangles ([`Rect`]), orientations in 90° steps with
//! an optional mirror ([`Orient`]), rigid transforms ([`Transform`]), grid snapping ([`snap`],
//! [`Grid`]), segment distance and intersection ([`Segment`], [`Intersection`]) and orthogonal
//! helpers for wire routing ([`manhattan`], [`is_horizontal`], [`is_vertical`], [`elbow`]).
//!
//! # Conventions
//! - Coordinates are `f64` in world units. The y axis points **up** (mathematical convention),
//!   so a positive angle is counter-clockwise (CCW). Code that draws on a y-down screen flips y
//!   once, at the view transform.
//! - Nothing here panics: non-finite input (NaN, ±∞) flows through arithmetic as IEEE 754
//!   values, and the functions that make decisions document what they do with it.
//! - No workspace dependencies (layer L0). The optional `serde` feature writes [`Pt`] and
//!   [`Vec2`] as `[x, y]` arrays.
//!
//! # Example
//! ```
//! use protasena_geom::{Orient, Pt, Transform, Vec2, snap};
//!
//! // A symbol pin at local (10, 0); the symbol is rotated 90° CCW and placed at (100, 50).
//! let place = Transform::new(Orient::rotation(1), Vec2::new(100.0, 50.0));
//! assert_eq!(place.apply_pt(Pt::new(10.0, 0.0)), Pt::new(100.0, 60.0));
//! assert_eq!(place.inverse().apply_pt(Pt::new(100.0, 60.0)), Pt::new(10.0, 0.0));
//!
//! // The cursor snaps to a 25-unit grid.
//! assert_eq!(snap(Pt::new(37.0, -12.0), 25.0), Pt::new(25.0, 0.0));
//! ```
#![warn(missing_docs)]

mod error;
mod orient;
mod ortho;
mod point;
mod rect;
mod segment;
#[cfg(feature = "serde")]
mod serde_impl;
mod snap;
mod transform;

pub use error::GeomError;
pub use orient::Orient;
pub use ortho::{elbow, is_horizontal, is_vertical, manhattan};
pub use point::{Pt, Vec2};
pub use rect::Rect;
pub use segment::{Intersection, Segment};
pub use snap::{Grid, snap, snap_scalar};
pub use transform::Transform;
