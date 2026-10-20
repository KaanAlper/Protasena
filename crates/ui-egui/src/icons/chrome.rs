//! Icons of the top toolbars and the status bar.

use super::{Icon, Pen};

/// Draws a toolbar or status icon; `false` when `icon` is not one of them.
pub(super) fn paint(p: &Pen, icon: Icon) -> bool {
    let (ink, acc) = (p.style.ink, p.style.accent);
    match icon {
        Icon::New => page(p, false),
        Icon::Open => {
            p.outline(&[(1.5, 4.0), (6.0, 4.0), (7.5, 5.5), (14.0, 5.5), (14.0, 13.0), (1.5, 13.0)]);
            p.accent(&[(1.5, 13.0), (4.0, 8.0), (15.0, 8.0), (13.0, 13.0)]);
        }
        Icon::Save => {
            p.line(&[(3.0, 10.0), (3.0, 14.0), (13.0, 14.0), (13.0, 10.0)]);
            p.accent(&[(8.0, 2.0), (8.0, 10.0)]);
            p.head((8.0, 11.0), 90.0, acc);
        }
        Icon::Close => {
            p.outline(&[(1.5, 4.0), (6.0, 4.0), (7.5, 5.5), (14.0, 5.5), (14.0, 13.0), (1.5, 13.0)]);
            p.accent(&[(6.0, 7.5), (10.0, 11.5)]);
            p.accent(&[(10.0, 7.5), (6.0, 11.5)]);
        }
        Icon::Home => {
            p.outline(&[(4.0, 7.5), (8.0, 3.5), (12.0, 7.5), (12.0, 14.0), (4.0, 14.0)]);
            p.line(&[(2.0, 9.0), (8.0, 2.5), (14.0, 9.0)]);
            p.accent(&[(7.0, 14.0), (7.0, 10.5), (9.0, 10.5), (9.0, 14.0)]);
        }
        Icon::Schematic => {
            p.line(&[(1.0, 8.0), (3.5, 8.0)]);
            p.accent(&[(3.5, 8.0), (4.75, 5.0), (6.25, 11.0), (7.75, 5.0), (9.25, 11.0), (10.75, 5.0), (12.25, 11.0), (13.0, 8.0)]);
            p.line(&[(13.0, 8.0), (15.0, 8.0)]);
        }
        Icon::Pcb => {
            p.rect(2.0, 3.0, 14.0, 13.0);
            p.accent(&[(5.0, 6.0), (9.0, 6.0), (11.0, 8.0), (11.0, 10.0)]);
            p.accent(&[(5.0, 10.0), (8.0, 10.0)]);
            for c in [(5.0, 6.0), (11.0, 10.0), (5.0, 10.0)] {
                p.disc(c, 1.3, acc);
            }
        }
        Icon::View3d => {
            p.outline(&[(3.0, 6.0), (10.0, 6.0), (10.0, 13.0), (3.0, 13.0)]);
            p.line(&[(3.0, 6.0), (6.0, 3.0), (13.0, 3.0), (10.0, 6.0)]);
            p.line(&[(13.0, 3.0), (13.0, 10.0), (10.0, 13.0)]);
        }
        Icon::Gerber => {
            for (dy, c) in [(5.0, ink), (2.5, ink), (0.0, acc)] {
                p.line_c(&[(2.0, 8.0 + dy), (8.0, 5.0 + dy), (14.0, 8.0 + dy), (8.0, 11.0 + dy), (2.0, 8.0 + dy)], c);
            }
        }
        Icon::Explorer => {
            p.rect(2.0, 2.0, 6.0, 5.5);
            p.line(&[(4.0, 5.5), (4.0, 12.0), (8.0, 12.0)]);
            p.line(&[(4.0, 8.0), (8.0, 8.0)]);
            p.rect(8.0, 6.5, 13.0, 9.5);
            p.rect(8.0, 10.5, 13.0, 13.5);
            p.solid(&[(8.5, 7.0), (12.5, 7.0), (12.5, 9.0), (8.5, 9.0)], acc);
        }
        Icon::Bom => {
            p.rect(3.0, 2.0, 13.0, 14.0);
            for y in [5.0, 8.0, 11.0] {
                p.disc((5.5, y), 0.8, acc);
                p.line(&[(7.5, y), (11.0, y)]);
            }
        }
        Icon::SourceCode => {
            p.line(&[(5.5, 4.0), (1.5, 8.0), (5.5, 12.0)]);
            p.line(&[(10.5, 4.0), (14.5, 8.0), (10.5, 12.0)]);
            p.accent(&[(9.5, 2.5), (6.5, 13.5)]);
        }
        Icon::Notes => {
            p.rect(3.0, 2.5, 13.0, 14.0);
            p.accent(&[(5.0, 2.5), (5.0, 1.0)]);
            p.accent(&[(8.0, 2.5), (8.0, 1.0)]);
            p.accent(&[(11.0, 2.5), (11.0, 1.0)]);
            for y in [6.0, 8.5, 11.0] {
                p.line(&[(5.0, y), (11.0, y)]);
            }
        }
        Icon::Help => {
            p.circle((8.0, 8.0), 6.5, ink);
            p.text((8.0, 8.3), "?", 10.0, acc);
        }
        Icon::Redraw => {
            let end = p.arc((8.0, 8.0), 5.5, 200.0, 470.0, ink);
            p.head(end, 470.0 + 90.0, ink);
            p.disc((8.0, 8.0), 1.2, acc);
        }
        Icon::Grid => {
            for x in [3.5, 8.0, 12.5] {
                for y in [3.5, 8.0, 12.5] {
                    p.disc((x, y), 1.1, if x == 8.0 && y == 8.0 { acc } else { ink });
                }
            }
        }
        Icon::FalseOrigin => {
            p.circle((8.0, 8.0), 4.5, ink);
            p.accent(&[(8.0, 1.5), (8.0, 14.5)]);
            p.accent(&[(1.5, 8.0), (14.5, 8.0)]);
        }
        Icon::Center => {
            for (a, b) in [((8.0, 1.0), (8.0, 5.0)), ((8.0, 11.0), (8.0, 15.0)), ((1.0, 8.0), (5.0, 8.0)), ((11.0, 8.0), (15.0, 8.0))] {
                p.line(&[a, b]);
            }
            p.disc((8.0, 8.0), 2.0, acc);
        }
        Icon::ZoomIn | Icon::ZoomOut | Icon::ZoomAll | Icon::ZoomArea => zoom(p, icon),
        Icon::Undo | Icon::Redo => {
            let flip = |x: f32| if icon == Icon::Redo { 16.0 - x } else { x };
            p.line(&[(flip(13.5), 13.0), (flip(13.5), 9.0), (flip(11.0), 6.5), (flip(4.5), 6.5)]);
            p.head((flip(2.5), 6.5), if icon == Icon::Redo { 0.0 } else { 180.0 }, acc);
        }
        Icon::Cut => {
            p.circle((4.5, 12.0), 2.2, ink);
            p.circle((11.5, 12.0), 2.2, ink);
            p.accent(&[(6.0, 10.3), (11.5, 2.0)]);
            p.accent(&[(10.0, 10.3), (4.5, 2.0)]);
        }
        Icon::Copy => {
            p.rect(2.5, 2.5, 10.0, 10.0);
            p.rect(6.0, 6.0, 13.5, 13.5);
        }
        Icon::Paste => {
            p.rect(3.0, 3.0, 13.0, 14.5);
            p.solid(&[(5.5, 1.5), (10.5, 1.5), (10.5, 4.5), (5.5, 4.5)], acc);
            p.line(&[(5.5, 8.0), (10.5, 8.0)]);
            p.line(&[(5.5, 11.0), (9.0, 11.0)]);
        }
        Icon::BlockCopy | Icon::BlockMove | Icon::BlockRotate | Icon::BlockDelete => block(p, icon),
        Icon::PickParts | Icon::MakeDevice | Icon::Packaging | Icon::Decompose => library(p, icon),
        Icon::Autorouter => {
            p.accent(&[(2.0, 13.0), (2.0, 8.0), (9.0, 8.0), (9.0, 3.0), (14.0, 3.0)]);
            p.disc((2.0, 13.0), 1.5, ink);
            p.disc((14.0, 3.0), 1.5, ink);
        }
        Icon::SearchTag => {
            p.outline(&[(2.0, 4.0), (9.0, 4.0), (13.0, 8.0), (9.0, 12.0), (2.0, 12.0)]);
            p.disc((5.0, 8.0), 1.2, acc);
        }
        Icon::PropertyTool => {
            for (y, w) in [(4.0, 10.0), (8.0, 8.0), (12.0, 11.0)] {
                p.line(&[(2.0, y), (6.0, y)]);
                p.accent(&[(8.0, y), (w + 3.0, y)]);
            }
        }
        Icon::NewSheet | Icon::RemoveSheet | Icon::ExitParent => {
            page(p, true);
            match icon {
                Icon::NewSheet => {
                    p.accent(&[(8.5, 7.0), (8.5, 13.0)]);
                    p.accent(&[(5.5, 10.0), (11.5, 10.0)]);
                }
                Icon::RemoveSheet => p.accent(&[(5.5, 10.0), (11.5, 10.0)]),
                _ => {
                    p.accent(&[(8.5, 13.0), (8.5, 8.0)]);
                    p.head((8.5, 6.5), 270.0, acc);
                }
            }
        }
        Icon::Erc => {
            p.rect(2.0, 2.0, 14.0, 14.0);
            p.thick(&[(4.5, 8.5), (7.0, 11.0), (11.5, 5.0)], acc);
        }
        Icon::Play => p.solid(&[(4.0, 2.5), (13.0, 8.0), (4.0, 13.5)], ink),
        Icon::Step => {
            p.solid(&[(3.0, 2.5), (10.0, 8.0), (3.0, 13.5)], ink);
            p.solid(&[(11.0, 2.5), (13.0, 2.5), (13.0, 13.5), (11.0, 13.5)], ink);
        }
        Icon::Pause => {
            p.solid(&[(4.0, 2.5), (6.5, 2.5), (6.5, 13.5), (4.0, 13.5)], ink);
            p.solid(&[(9.5, 2.5), (12.0, 2.5), (12.0, 13.5), (9.5, 13.5)], ink);
        }
        Icon::Stop => p.solid(&[(3.5, 3.5), (12.5, 3.5), (12.5, 12.5), (3.5, 12.5)], ink),
        Icon::Info => {
            p.circle((8.0, 8.0), 6.0, ink);
            p.line(&[(8.0, 7.0), (8.0, 11.5)]);
            p.disc((8.0, 4.8), 0.9, ink);
        }
        Icon::Cross => {
            p.line(&[(4.0, 4.0), (12.0, 12.0)]);
            p.line(&[(12.0, 4.0), (4.0, 12.0)]);
        }
        _ => return false,
    }
    true
}

/// A page with a folded corner (smaller when `small`).
fn page(p: &Pen, small: bool) {
    let (x0, y0, x1, y1) = if small { (3.0, 1.5, 13.0, 14.5) } else { (3.5, 1.5, 12.5, 14.5) };
    p.outline(&[(x0, y0), (x1 - 3.0, y0), (x1, y0 + 3.0), (x1, y1), (x0, y1)]);
    p.line(&[(x1 - 3.0, y0), (x1 - 3.0, y0 + 3.0), (x1, y0 + 3.0)]);
}

fn zoom(p: &Pen, icon: Icon) {
    let (ink, acc) = (p.style.ink, p.style.accent);
    if icon == Icon::ZoomArea {
        p.dashed_rect(1.5, 1.5, 11.0, 10.0);
    }
    let (c, r) = if icon == Icon::ZoomArea { ((9.5, 9.5), 3.0) } else { ((6.5, 6.5), 4.5) };
    p.circle(c, r, ink);
    p.thick(&[(c.0 + r * 0.75, c.1 + r * 0.75), (14.5, 14.5)], ink);
    match icon {
        Icon::ZoomIn => {
            p.accent(&[(4.5, 6.5), (8.5, 6.5)]);
            p.accent(&[(6.5, 4.5), (6.5, 8.5)]);
        }
        Icon::ZoomOut => p.accent(&[(4.5, 6.5), (8.5, 6.5)]),
        Icon::ZoomAll => p.line_c(&[(4.5, 4.5), (8.5, 4.5), (8.5, 8.5), (4.5, 8.5), (4.5, 4.5)], acc),
        _ => {}
    }
}

fn block(p: &Pen, icon: Icon) {
    let acc = p.style.accent;
    p.dashed_rect(1.5, 1.5, 10.5, 10.5);
    match icon {
        Icon::BlockCopy => p.solid(&[(7.0, 7.0), (14.5, 7.0), (14.5, 14.5), (7.0, 14.5)], acc),
        Icon::BlockMove => {
            p.accent(&[(11.5, 7.5), (11.5, 15.0)]);
            p.accent(&[(7.5, 11.5), (15.0, 11.5)]);
            p.head((11.5, 6.5), 270.0, acc);
            p.head((15.5, 11.5), 0.0, acc);
        }
        Icon::BlockRotate => {
            let end = p.arc((11.0, 11.0), 3.5, 180.0, 450.0, acc);
            p.head(end, 450.0 + 90.0, acc);
        }
        _ => {
            p.thick(&[(8.5, 8.5), (14.5, 14.5)], acc);
            p.thick(&[(14.5, 8.5), (8.5, 14.5)], acc);
        }
    }
}

/// A chip outline with pins on both sides.
fn chip(p: &Pen, x0: f32, x1: f32) {
    p.rect(x0, 3.0, x1, 13.0);
    for y in [5.0, 8.0, 11.0] {
        p.line(&[(x0 - 2.0, y), (x0, y)]);
        p.line(&[(x1, y), (x1 + 2.0, y)]);
    }
}

fn library(p: &Pen, icon: Icon) {
    let acc = p.style.accent;
    match icon {
        Icon::PickParts => {
            chip(p, 4.0, 9.5);
            p.solid(&[(10.5, 9.0), (15.5, 9.0), (15.5, 14.5), (10.5, 14.5)], p.style.fill);
            p.accent(&[(13.0, 9.5), (13.0, 14.0)]);
            p.accent(&[(10.5, 11.75), (15.5, 11.75)]);
        }
        Icon::MakeDevice => {
            chip(p, 4.0, 12.0);
            p.solid(&[(8.0, 5.0), (9.2, 7.6), (12.0, 8.0), (9.2, 8.4), (8.0, 11.0), (6.8, 8.4), (4.0, 8.0), (6.8, 7.6)], acc);
        }
        Icon::Packaging => {
            for x in [3.0, 7.0, 11.0] {
                p.solid(&[(x, 2.0), (x + 2.5, 2.0), (x + 2.5, 5.0), (x, 5.0)], acc);
                p.solid(&[(x, 11.0), (x + 2.5, 11.0), (x + 2.5, 14.0), (x, 14.0)], acc);
            }
            p.dashed_rect(2.0, 6.5, 14.0, 9.5);
        }
        _ => {
            p.dashed_rect(5.0, 5.0, 11.0, 11.0);
            for (a, b, deg) in [((5.0, 5.0), (2.0, 2.0), 225.0), ((11.0, 11.0), (14.0, 14.0), 45.0)] {
                p.accent(&[a, b]);
                p.head(b, deg, acc);
            }
        }
    }
}
