//! PCB packages for the Pick Devices package preview: pads (filled) and silkscreen drawing.

use std::f64::consts::{FRAC_PI_2, PI};

use protasena_geom::{Pt, Transform, Vec2};
use protasena_library::{Package, Pad, PadShape, layer};

use crate::prim::{Ink, Prim, Shape};
use crate::symbol::graphic;

/// Most pads and graphics drawn for one package.
const MAX_ITEMS: usize = 8192;

/// The package drawn as seen from the top: silkscreen outlines, then pads.
pub fn package_prims(p: &Package) -> Vec<Prim> {
    let mut out: Vec<Prim> = p
        .graphics
        .iter()
        .take(MAX_ITEMS)
        .filter(|g| g.layer == layer::TOP_SILK || g.layer.starts_with("silk") || g.layer.starts_with("fab"))
        .filter(|g| g.graphic.shape.is_finite())
        .filter_map(|g| graphic(&g.graphic, Transform::IDENTITY))
        .map(|prim| Prim { ink: Ink::Silk, fill: None, ..prim })
        .collect();
    out.extend(p.pads.iter().take(MAX_ITEMS).filter(|pad| pad.is_sane()).map(pad_prim));
    out
}

fn pad_prim(pad: &Pad) -> Prim {
    let (w, h) = (pad.size.x / 2.0, pad.size.y / 2.0);
    let turn = Transform::new(protasena_geom::Orient::IDENTITY, pad.at.to_vec());
    let (sin, cos) = pad.angle.to_radians().sin_cos();
    let p = |x: f64, y: f64| turn.apply_pt(Pt::new(x * cos - y * sin, x * sin + y * cos));
    let shape = match pad.shape {
        PadShape::Circle => Shape::Circle { center: pad.at, radius: w.max(h) },
        PadShape::Rect | PadShape::RoundRect { .. } => Shape::Polyline { points: vec![p(-w, -h), p(w, -h), p(w, h), p(-w, h)], closed: true },
        PadShape::Oval => {
            // A stadium: two half circles at the ends of the long axis.
            let r = w.min(h);
            let (ends, start) = if w >= h {
                ([Vec2::new(w - r, 0.0), Vec2::new(r - w, 0.0)], -FRAC_PI_2)
            } else {
                ([Vec2::new(0.0, h - r), Vec2::new(0.0, r - h)], 0.0)
            };
            let steps = 8;
            let mut points = Vec::with_capacity(2 * steps + 2);
            for (k, end) in ends.into_iter().enumerate() {
                for i in 0..=steps {
                    let a = start + PI * (k as f64 + i as f64 / steps as f64);
                    points.push(p(end.x + r * a.cos(), end.y + r * a.sin()));
                }
            }
            Shape::Polyline { points, closed: true }
        }
    };
    Prim::new(shape, Ink::Pad).fill(Some(Ink::Pad))
}

#[cfg(test)]
mod tests {
    use super::*;
    use protasena_library::Library;

    #[test]
    fn builtin_packages_draw_their_pads() {
        let lib = Library::builtin();
        let pkg = lib.packages().first().unwrap();
        let prims = package_prims(pkg);
        assert_eq!(prims.iter().filter(|p| p.ink == Ink::Pad).count(), pkg.pads.len());
        assert!(prims.iter().all(|p| p.bounds().is_some()));
    }

    #[test]
    fn oval_and_rotated_pads_stay_around_their_centre() {
        let mut pad = Pad::through_hole("1", Pt::new(100.0, 0.0), PadShape::Oval, 60.0, 30.0);
        pad.size = Vec2::new(100.0, 60.0);
        pad.angle = 90.0;
        let b = pad_prim(&pad).bounds().unwrap();
        assert!((b.center().x - 100.0).abs() < 1e-6 && (b.height() - 100.0).abs() < 1e-6, "{b:?}");
    }
}
