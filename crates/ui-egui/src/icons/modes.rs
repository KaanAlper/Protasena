//! Icons of the Mode toolbar and the Orientation toolbar.

use super::{Icon, Pen};

/// Draws a mode or orientation icon (anything else draws a placeholder dot).
pub(super) fn paint(p: &Pen, icon: Icon) {
    let (ink, acc) = (p.style.ink, p.style.accent);
    match icon {
        Icon::Selection => p.outline(&[(4.0, 2.0), (12.5, 10.0), (8.5, 10.5), (10.5, 14.5), (8.8, 15.2), (6.8, 11.2), (4.0, 13.5)]),
        Icon::Component => {
            p.outline(&[(4.0, 3.0), (13.0, 8.0), (4.0, 13.0)]);
            p.line(&[(1.0, 5.5), (4.0, 5.5)]);
            p.line(&[(1.0, 10.5), (4.0, 10.5)]);
            p.accent(&[(13.0, 8.0), (15.5, 8.0)]);
        }
        Icon::JunctionDot => {
            p.line(&[(1.0, 8.0), (15.0, 8.0)]);
            p.line(&[(8.0, 8.0), (8.0, 15.0)]);
            p.disc((8.0, 8.0), 2.6, acc);
        }
        Icon::WireLabel => {
            p.line(&[(1.0, 12.5), (15.0, 12.5)]);
            p.text((8.0, 7.0), "AB", 7.5, acc);
        }
        Icon::TextScript => {
            p.rect(2.5, 2.0, 13.5, 14.0);
            for (y, w) in [(5.0, 11.0), (8.0, 9.0), (11.0, 10.0)] {
                p.accent(&[(4.5, y), (w, y)]);
            }
        }
        Icon::Buses => {
            p.thick(&[(2.0, 4.0), (2.0, 13.0), (14.0, 13.0)], acc);
            p.line(&[(2.0, 7.0), (5.0, 4.0), (10.0, 4.0)]);
            p.line(&[(2.0, 10.0), (5.0, 7.0), (10.0, 7.0)]);
        }
        Icon::Subcircuit => {
            p.rect(4.0, 3.0, 12.0, 13.0);
            p.line(&[(1.0, 6.0), (4.0, 6.0)]);
            p.line(&[(1.0, 10.0), (4.0, 10.0)]);
            p.line(&[(12.0, 8.0), (15.0, 8.0)]);
            p.solid(&[(6.5, 6.0), (9.5, 6.0), (9.5, 10.0), (6.5, 10.0)], acc);
        }
        Icon::Terminals => {
            p.line(&[(1.0, 8.0), (4.0, 8.0)]);
            p.outline(&[(4.0, 5.0), (11.5, 5.0), (14.5, 8.0), (11.5, 11.0), (4.0, 11.0)]);
            p.disc((1.5, 8.0), 1.0, acc);
        }
        Icon::DevicePins => {
            p.line(&[(4.0, 8.0), (13.0, 8.0)]);
            p.disc((3.0, 8.0), 1.6, acc);
            p.line(&[(13.0, 3.0), (13.0, 13.0)]);
            p.text((8.5, 4.5), "1", 6.0, ink);
        }
        Icon::Graph => {
            p.line(&[(2.0, 2.0), (2.0, 14.0), (14.5, 14.0)]);
            let wave: Vec<(f32, f32)> = (0..=12).map(|i| (3.0 + i as f32, 8.0 - 4.0 * (i as f32 * 0.55).sin())).collect();
            p.accent(&wave);
        }
        Icon::ActivePopup => {
            p.dashed_rect(1.5, 2.5, 14.5, 13.5);
            p.solid(&[(4.0, 5.0), (12.0, 5.0), (12.0, 7.0), (4.0, 7.0)], acc);
        }
        Icon::Generator => {
            p.circle((8.0, 8.0), 6.0, ink);
            let wave: Vec<(f32, f32)> = (0..=8).map(|i| (4.0 + i as f32, 8.0 - 2.5 * (i as f32 * 0.785).sin())).collect();
            p.accent(&wave);
        }
        Icon::Probe => {
            p.thick(&[(3.0, 13.0), (8.0, 8.0)], acc);
            p.outline(&[(8.0, 6.0), (12.0, 2.0), (14.0, 4.0), (10.0, 8.0)]);
            p.disc((2.5, 13.5), 1.2, ink);
        }
        Icon::Instruments => {
            p.rect(1.5, 3.0, 14.5, 13.0);
            p.arc((8.0, 11.5), 5.0, 200.0, 340.0, ink);
            p.accent(&[(8.0, 11.5), (11.0, 7.0)]);
        }
        Icon::Line2d => p.line(&[(2.5, 13.5), (13.5, 2.5)]),
        Icon::Box2d => p.rect(2.5, 4.0, 13.5, 12.0),
        Icon::Circle2d => p.circle((8.0, 8.0), 5.5, ink),
        Icon::Arc2d => {
            p.arc((8.0, 11.0), 6.0, 180.0, 360.0, ink);
        }
        Icon::Path2d => p.outline(&[(2.0, 12.0), (4.0, 3.0), (10.0, 5.0), (14.0, 2.5), (13.0, 13.0)]),
        Icon::Text2d => p.text((8.0, 8.5), "A", 13.0, ink),
        Icon::Symbols2d => {
            let star: Vec<(f32, f32)> = (0..10)
                .map(|i| {
                    let a = (i as f32 * 36.0 - 90.0).to_radians();
                    let r = if i % 2 == 0 { 6.5 } else { 2.8 };
                    (8.0 + r * a.cos(), 8.5 + r * a.sin())
                })
                .collect();
            let mut closed = star.clone();
            closed.extend(star.first().copied());
            p.line(&closed);
        }
        Icon::Markers2d => {
            p.line(&[(8.0, 2.0), (8.0, 14.0)]);
            p.line(&[(2.0, 8.0), (14.0, 8.0)]);
            p.circle((8.0, 8.0), 3.0, acc);
        }
        Icon::RotateCw | Icon::RotateCcw => {
            let cw = icon == Icon::RotateCw;
            let (from, to) = if cw { (150.0, 420.0) } else { (390.0, 120.0) };
            let end = p.arc((8.0, 8.5), 5.5, from, to, ink);
            p.head(end, if cw { to + 90.0 } else { to - 90.0 }, acc);
        }
        Icon::MirrorX => {
            p.dashed_axis_v();
            p.outline(&[(6.5, 4.0), (6.5, 12.0), (1.5, 12.0)]);
            p.solid(&[(9.5, 4.0), (9.5, 12.0), (14.5, 12.0)], acc);
        }
        Icon::MirrorY => {
            p.dashed_axis_h();
            p.outline(&[(4.0, 6.5), (12.0, 6.5), (12.0, 1.5)]);
            p.solid(&[(4.0, 9.5), (12.0, 9.5), (12.0, 14.5)], acc);
        }
        _ => p.disc((8.0, 8.0), 2.0, ink),
    }
}

impl Pen<'_> {
    fn dashed_axis_v(&self) {
        for y in [1.0, 5.0, 9.0, 13.0] {
            self.line(&[(8.0, y), (8.0, y + 2.0)]);
        }
    }

    fn dashed_axis_h(&self) {
        for x in [1.0, 5.0, 9.0, 13.0] {
            self.line(&[(x, 8.0), (x + 2.0, 8.0)]);
        }
    }
}
