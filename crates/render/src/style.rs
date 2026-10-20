//! A design's template settings applied to its drawing: line widths of the graphic styles, text
//! heights of the text styles, the 2D graphics defaults, the junction dot style, and the colour
//! each template setting gives an ink (the toolkit uses it instead of its theme's).

use std::collections::HashMap;

use protasena_doc::{Colour, ItemId, JunctionShape, Sheet, Template};
use protasena_geom::Pt;

use crate::items::JUNCTION_RADIUS;
use crate::prim::{DisplayList, Ink, Prim, Shape};

/// The colour the template gives an ink, if it sets one.
pub fn ink_colour(t: &Template, ink: Ink) -> Option<Colour> {
    let (g, x) = (&t.graphic_styles, &t.text_styles);
    match ink {
        Ink::Body | Ink::BodyText => g.component.colour,
        Ink::BodyFill => g.component.fill,
        Ink::Pin => g.pin.colour,
        Ink::PinName => x.pin_name.colour,
        Ink::PinNumber => x.pin_number.colour,
        Ink::Reference => x.reference.colour,
        Ink::Value => x.value.colour,
        Ink::Wire | Ink::Junction => g.wire.colour,
        Ink::Bus => g.bus.colour,
        Ink::Label => x.label.colour,
        Ink::Terminal => g.terminal.colour,
        Ink::TerminalText => x.terminal.colour,
        Ink::Text => x.text.colour,
        Ink::Graphic => t.graphics_2d.colour,
        Ink::GraphicFill => t.graphics_2d.fill,
        Ink::Subcircuit | Ink::Port => g.subcircuit.colour,
        Ink::SubcircuitFill => g.subcircuit.fill,
        Ink::Missing | Ink::SheetBorder | Ink::Pad | Ink::Silk => None,
    }
}

/// The line width the template gives an ink's lines, if it sets one.
fn width(t: &Template, ink: Ink) -> Option<f64> {
    let g = &t.graphic_styles;
    match ink {
        Ink::Body => g.component.width,
        Ink::Pin => g.pin.width,
        Ink::Wire => g.wire.width,
        Ink::Bus => g.bus.width,
        Ink::Terminal => g.terminal.width,
        Ink::Subcircuit | Ink::Port => g.subcircuit.width,
        Ink::Graphic => t.graphics_2d.width,
        _ => None,
    }
}

/// The height the template gives an ink's texts, if it sets one (free texts: see [`styled`]).
fn height(t: &Template, ink: Ink) -> Option<f64> {
    let x = &t.text_styles;
    match ink {
        Ink::Reference => x.reference.height,
        Ink::Value => x.value.height,
        Ink::PinName => x.pin_name.height,
        Ink::PinNumber => x.pin_number.height,
        Ink::Label => x.label.height,
        Ink::TerminalText => x.terminal.height,
        Ink::Subcircuit | Ink::Port => x.subcircuit.height,
        _ => None,
    }
}

fn usable(v: Option<f64>) -> Option<f64> {
    v.filter(|v| v.is_finite() && *v > 0.0)
}

/// A junction dot at `at` in the template's style.
fn junction(t: &Template, at: Pt, owner: Option<ItemId>) -> Prim {
    let half = usable(t.junction.size).map_or(JUNCTION_RADIUS, |s| s / 2.0);
    let shape = match t.junction.shape {
        JunctionShape::Circle => Shape::Circle { center: at, radius: half },
        JunctionShape::Square => {
            let p = |dx: f64, dy: f64| Pt::new(at.x + dx * half, at.y + dy * half);
            Shape::Polyline { points: vec![p(-1.0, -1.0), p(1.0, -1.0), p(1.0, 1.0), p(-1.0, 1.0)], closed: true }
        }
        JunctionShape::Diamond => {
            let p = |dx: f64, dy: f64| Pt::new(at.x + dx * half, at.y + dy * half);
            Shape::Polyline { points: vec![p(0.0, -1.0), p(1.0, 0.0), p(0.0, 1.0), p(-1.0, 0.0)], closed: true }
        }
    };
    Prim::new(shape, Ink::Junction).fill(Some(Ink::Junction)).owned(owner)
}

/// `list` (a sheet's display list) drawn with the template's widths, heights, 2D graphics fill
/// and junction dots. Free texts with a height of their own keep it.
pub fn styled(list: DisplayList, sheet: &Sheet, t: &Template) -> DisplayList {
    if t.is_default() {
        return list;
    }
    let own_size: HashMap<ItemId, bool> = sheet.texts.iter().map(|x| (ItemId::Text(x.id), x.size > 0.0 && x.size.is_finite())).collect();
    let text_height = usable(t.text_styles.text.height);
    let prims = list
        .prims
        .into_iter()
        .map(|p| {
            if p.ink == Ink::Junction
                && let Shape::Circle { center, .. } = p.shape
            {
                return junction(t, center, p.owner);
            }
            let mut p = p;
            if let Shape::Text { size, .. } = &mut p.shape {
                let free = p.ink == Ink::Text && p.owner.is_some_and(|o| own_size.get(&o) == Some(&false));
                if let Some(h) = usable(height(t, p.ink)).or(if free { text_height } else { None }) {
                    *size = h;
                }
            } else if let Some(w) = usable(width(t, p.ink)) {
                p.width = w;
            }
            let closed = matches!(p.shape, Shape::Circle { .. } | Shape::Polyline { closed: true, .. });
            if p.ink == Ink::Graphic && closed && t.graphics_2d.fill.is_some() {
                p.fill = Some(Ink::GraphicFill);
            }
            p
        })
        .collect();
    DisplayList { prims }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protasena_doc::{Graphic, GraphicId, GraphicShape, Junction, JunctionId, SheetId, SheetSize, Text, TextId};
    use protasena_library::Library;

    #[test]
    fn widths_heights_fills_and_junctions_follow_the_template() {
        let mut sheet = Sheet::new(SheetId(1), "S", SheetSize::A4);
        sheet.junctions.push(Junction { id: JunctionId(2), at: Pt::ORIGIN });
        sheet.texts.push(Text { id: TextId(3), at: Pt::ORIGIN, text: "a".into(), size: 0.0, orient: Default::default() });
        sheet.texts.push(Text { id: TextId(4), at: Pt::ORIGIN, text: "b".into(), size: 55.0, orient: Default::default() });
        sheet.graphics.push(Graphic { id: GraphicId(5), shape: GraphicShape::Box { a: Pt::ORIGIN, b: Pt::new(100.0, 100.0) } });
        let lib = Library::builtin();
        let plain = crate::sheet_display_list(&sheet, &lib);
        assert_eq!(styled(plain.clone(), &sheet, &Template::default()), plain, "the default template changes nothing");
        let mut t = Template { junction: protasena_doc::JunctionStyle { size: Some(60.0), shape: JunctionShape::Square }, ..Template::default() };
        t.text_styles.text.height = Some(150.0);
        t.graphics_2d.width = Some(20.0);
        t.graphics_2d.fill = Colour::parse("#ff0000");
        let list = styled(plain, &sheet, &t);
        let dot = list.of(ItemId::Junction(JunctionId(2))).next().unwrap();
        assert!(matches!(&dot.shape, Shape::Polyline { points, closed: true } if points[0] == Pt::new(-30.0, -30.0)));
        let size = |id| match list.of(ItemId::Text(TextId(id))).next().unwrap().shape {
            Shape::Text { size, .. } => size,
            _ => 0.0,
        };
        assert_eq!((size(3), size(4)), (150.0, 55.0), "a text with its own height keeps it");
        let g = list.of(ItemId::Graphic(GraphicId(5))).next().unwrap();
        assert_eq!((g.width, g.fill), (20.0, Some(Ink::GraphicFill)));
        assert_eq!(ink_colour(&t, Ink::GraphicFill), Colour::parse("#ff0000"));
        assert_eq!(ink_colour(&t, Ink::Wire), None);
    }
}
