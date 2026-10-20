//! Design items as primitives: components (symbol, reference, value), wires, junction dots,
//! wire labels, terminals, buses, bus entries and free texts; a whole sheet as a display list.

use protasena_doc::{Bus, BusEntry, Component, ItemId, Junction, Label, Sheet, Terminal, TerminalKind, Text, Wire};
use protasena_geom::{Orient, Pt, Rect, Transform, Vec2};
use protasena_library::{Device, Library};

use crate::geometry::{flip_h, flip_v, readable};
use crate::prim::{DisplayList, HAlign, Ink, Prim, Shape, VAlign};
use crate::symbol::symbol_prims;

/// Wire width in thou.
pub const WIRE_WIDTH: f64 = 8.0;
/// Bus width in thou.
pub const BUS_WIDTH: f64 = 30.0;
/// Junction dot radius in thou.
pub const JUNCTION_RADIUS: f64 = 20.0;
/// Height of reference texts, in thou.
pub const REFERENCE_SIZE: f64 = 100.0;
/// Height of value texts, in thou.
pub const VALUE_SIZE: f64 = 80.0;
/// Height of wire labels and terminal names, in thou.
pub const LABEL_SIZE: f64 = 80.0;
/// Height of free text of size 0, in thou.
pub const TEXT_SIZE: f64 = 100.0;
/// Half size of the box drawn for a component whose device is not in the library.
pub const MISSING_HALF: f64 = 100.0;
/// Gap between a wire label and its wire, in thou.
const LABEL_LIFT: f64 = 15.0;
/// Most points drawn for one wire or bus.
const MAX_POINTS: usize = 4096;

/// How to look a component's device up (the netlist crate's `DeviceSource` is a sibling of
/// this one, so the rule is repeated here: name without case, then the library if given).
pub fn find_device<'a>(library: &'a Library, c: &Component) -> Option<&'a Device> {
    let r = &c.device;
    let matches = |d: &&Device| r.library.is_empty() || d.library.eq_ignore_ascii_case(&r.library);
    library.get(&r.name).filter(matches).or_else(|| library.devices().iter().filter(|d| d.name.eq_ignore_ascii_case(&r.name)).find(matches))
}

/// A component: its symbol (or a box when the device is missing), reference and value.
pub fn component_prims(c: &Component, library: &Library) -> Vec<Prim> {
    let owner = Some(ItemId::Component(c.id));
    let mut out = match find_device(library, c) {
        Some(d) => symbol_prims(d, c.unit, c.transform),
        None => missing_prims(c.transform),
    };
    let origin = c.origin();
    if c.reference_label.visible && !c.reference.is_empty() {
        let at = origin + c.reference_label.offset;
        out.push(text(at, &c.reference, REFERENCE_SIZE, HAlign::Center, VAlign::Bottom, Ink::Reference));
    }
    if c.value_label.visible && !c.value.is_empty() {
        let at = origin + c.value_label.offset;
        out.push(text(at, &c.value, VALUE_SIZE, HAlign::Center, VAlign::Top, Ink::Value));
    }
    out.into_iter().map(|p| p.owned(owner)).collect()
}

fn missing_prims(t: Transform) -> Vec<Prim> {
    let h = MISSING_HALF;
    let corners = [Pt::new(-h, -h), Pt::new(h, -h), Pt::new(h, h), Pt::new(-h, h)].map(|p| t.apply_pt(p));
    vec![
        Prim::new(Shape::Polyline { points: corners.to_vec(), closed: true }, Ink::Missing).width(crate::symbol::BODY_WIDTH),
        text(t.apply_pt(Pt::ORIGIN), "?", REFERENCE_SIZE, HAlign::Center, VAlign::Middle, Ink::Missing),
    ]
}

fn text(at: Pt, s: &str, size: f64, h: HAlign, v: VAlign, ink: Ink) -> Prim {
    Prim::new(Shape::Text { at, text: s.to_owned(), size, angle: 0.0, h, v }, ink)
}

/// A text placed with an orientation: the anchor stays, the direction turns and stays readable.
fn oriented_text(at: Pt, s: &str, size: f64, o: Orient, h: HAlign, v: VAlign, ink: Ink) -> Prim {
    let d = o.apply_vec(Vec2::new(1.0, 0.0));
    let (angle, turned) = readable(d.y.atan2(d.x).to_degrees());
    let (h, v) = if turned { (flip_h(h), flip_v(v)) } else { (h, v) };
    Prim::new(Shape::Text { at, text: s.to_owned(), size, angle, h, v }, ink)
}

/// A wire.
pub fn wire_prims(w: &Wire) -> Vec<Prim> {
    let points: Vec<Pt> = w.points.iter().take(MAX_POINTS).copied().collect();
    vec![Prim::new(Shape::Polyline { points, closed: false }, Ink::Wire).width(WIRE_WIDTH).owned(Some(ItemId::Wire(w.id)))]
}

/// A bus.
pub fn bus_prims(b: &Bus) -> Vec<Prim> {
    let points: Vec<Pt> = b.points.iter().take(MAX_POINTS).copied().collect();
    vec![Prim::new(Shape::Polyline { points, closed: false }, Ink::Bus).width(BUS_WIDTH).owned(Some(ItemId::Bus(b.id)))]
}

/// A bus entry.
pub fn bus_entry_prims(e: &BusEntry) -> Vec<Prim> {
    vec![Prim::new(Shape::Line { a: e.bus_end, b: e.wire_end }, Ink::Bus).width(WIRE_WIDTH).owned(Some(ItemId::BusEntry(e.id)))]
}

/// A junction dot.
pub fn junction_prims(j: &Junction) -> Vec<Prim> {
    let dot = Shape::Circle { center: j.at, radius: JUNCTION_RADIUS };
    vec![Prim::new(dot, Ink::Junction).fill(Some(Ink::Junction)).owned(Some(ItemId::Junction(j.id)))]
}

/// A wire label: its text just above (or beside) the wire at its attachment point.
pub fn label_prims(l: &Label) -> Vec<Prim> {
    let lift = l.orient.apply_vec(Vec2::new(0.0, LABEL_LIFT));
    let p = oriented_text(l.at + lift, &l.text, LABEL_SIZE, l.orient, HAlign::Left, VAlign::Bottom, Ink::Label);
    vec![p.owned(Some(ItemId::Label(l.id)))]
}

/// A free text.
pub fn text_prims(t: &Text) -> Vec<Prim> {
    let size = if t.size > 0.0 && t.size.is_finite() { t.size } else { TEXT_SIZE };
    let p = oriented_text(t.at, &t.text, size, t.orient, HAlign::Left, VAlign::Bottom, Ink::Text);
    vec![p.owned(Some(ItemId::Text(t.id)))]
}

/// A terminal: its symbol (our own shapes: a flag for ports, a bar for power, stacked bars for
/// ground) and its name.
pub fn terminal_prims(t: &Terminal) -> Vec<Prim> {
    let place = Transform::new(t.orient, t.at.to_vec());
    let p = |x: f64, y: f64| place.apply_pt(Pt::new(x, y));
    let line = |a: Pt, b: Pt| Prim::new(Shape::Line { a, b }, Ink::Terminal).width(WIRE_WIDTH);
    let outline = |pts: &[(f64, f64)]| {
        let points = pts.iter().map(|&(x, y)| p(x, y)).collect();
        Prim::new(Shape::Polyline { points, closed: true }, Ink::Terminal).width(WIRE_WIDTH)
    };
    let mut out = Vec::new();
    match t.kind {
        TerminalKind::Power => {
            out.push(line(p(0.0, 0.0), p(0.0, 100.0)));
            out.push(line(p(-60.0, 100.0), p(60.0, 100.0)));
            if !t.name.is_empty() {
                out.push(oriented_text(p(0.0, 130.0), &t.name, LABEL_SIZE, Orient::IDENTITY, HAlign::Center, VAlign::Bottom, Ink::TerminalText));
            }
        }
        TerminalKind::Ground => {
            out.push(line(p(0.0, 0.0), p(0.0, -60.0)));
            for (y, half) in [(-60.0, 60.0), (-90.0, 38.0), (-120.0, 16.0)] {
                out.push(line(p(-half, y), p(half, y)));
            }
        }
        kind => {
            out.push(line(p(0.0, 0.0), p(40.0, 0.0)));
            let shape: &[(f64, f64)] = match kind {
                TerminalKind::Input => &[(40.0, 0.0), (80.0, -40.0), (220.0, -40.0), (220.0, 40.0), (80.0, 40.0)],
                TerminalKind::Output => &[(40.0, -40.0), (180.0, -40.0), (220.0, 0.0), (180.0, 40.0), (40.0, 40.0)],
                TerminalKind::Bidir => &[(40.0, 0.0), (80.0, -40.0), (180.0, -40.0), (220.0, 0.0), (180.0, 40.0), (80.0, 40.0)],
                _ => &[(40.0, -40.0), (220.0, -40.0), (220.0, 40.0), (40.0, 40.0)],
            };
            let mut flag = outline(shape);
            if kind == TerminalKind::Bus {
                flag = flag.width(BUS_WIDTH / 2.0);
            }
            out.push(flag);
            if !t.name.is_empty() {
                out.push(oriented_text(p(250.0, 0.0), &t.name, LABEL_SIZE, t.orient, HAlign::Left, VAlign::Middle, Ink::TerminalText));
            }
        }
    }
    out.into_iter().map(|p| p.owned(Some(ItemId::Terminal(t.id)))).collect()
}

/// The sheet's border: a rectangle of the sheet size centred on the origin.
pub fn sheet_rect(sheet: &Sheet) -> Rect {
    let (w, h) = sheet.size.dimensions();
    let (w, h) = if w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0 { (w, h) } else { (11_693.0, 8_268.0) };
    Rect::new(Pt::new(-w / 2.0, -h / 2.0), Pt::new(w / 2.0, h / 2.0))
}

/// Everything on a sheet in drawing order: border, buses, wires, components, labels,
/// terminals, texts, junction dots on top.
pub fn sheet_display_list(sheet: &Sheet, library: &Library) -> DisplayList {
    let mut list = DisplayList::default();
    let r = sheet_rect(sheet);
    list.push(Prim::new(Shape::Polyline { points: r.corners().to_vec(), closed: true }, Ink::SheetBorder));
    sheet.buses.iter().for_each(|b| list.extend(bus_prims(b)));
    sheet.bus_entries.iter().for_each(|e| list.extend(bus_entry_prims(e)));
    sheet.wires.iter().for_each(|w| list.extend(wire_prims(w)));
    sheet.components.iter().for_each(|c| list.extend(component_prims(c, library)));
    sheet.labels.iter().for_each(|l| list.extend(label_prims(l)));
    sheet.terminals.iter().for_each(|t| list.extend(terminal_prims(t)));
    sheet.texts.iter().for_each(|t| list.extend(text_prims(t)));
    sheet.junctions.iter().for_each(|j| list.extend(junction_prims(j)));
    list
}

/// The primitives for any one item (for previews of dragged items).
pub fn item_prims(item: &protasena_doc::Item, library: &Library) -> Vec<Prim> {
    use protasena_doc::Item;
    match item {
        Item::Component(c) => component_prims(c, library),
        Item::Wire(w) => wire_prims(w),
        Item::Junction(j) => junction_prims(j),
        Item::Label(l) => label_prims(l),
        Item::Terminal(t) => terminal_prims(t),
        Item::Bus(b) => bus_prims(b),
        Item::BusEntry(e) => bus_entry_prims(e),
        Item::Text(t) => text_prims(t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protasena_doc::{ComponentId, DeviceRef, FieldLabel, JunctionId, SheetId, SheetSize, TerminalId, WireId};

    fn texts(prims: &[Prim]) -> Vec<String> {
        prims.iter().filter_map(|p| if let Shape::Text { text, .. } = &p.shape { Some(text.clone()) } else { None }).collect()
    }

    #[test]
    fn components_draw_symbol_reference_and_value() {
        let lib = Library::builtin();
        let mut c = Component::new(ComponentId(7), DeviceRef::new("", "RES"), "R1", "10k").placed(Transform::translation(Vec2::new(500.0, 0.0)));
        c.reference_label = FieldLabel { offset: Vec2::new(0.0, 100.0), visible: true };
        let prims = component_prims(&c, &lib);
        assert_eq!(texts(&prims), ["R1", "10k"]);
        assert!(prims.iter().all(|p| p.owner == Some(ItemId::Component(ComponentId(7)))));
        let r1 = prims.iter().find(|p| p.ink == Ink::Reference).unwrap();
        assert!(matches!(r1.shape, Shape::Text { at, .. } if at == Pt::new(500.0, 100.0)));
        c.value_label.visible = false;
        assert_eq!(texts(&component_prims(&c, &lib)), ["R1"]);
        let missing = Component::new(ComponentId(8), DeviceRef::new("", "NOPE"), "U1", "");
        let prims = component_prims(&missing, &lib);
        assert!(prims.iter().any(|p| p.ink == Ink::Missing));
    }

    #[test]
    fn sheet_list_has_border_and_items_with_owners() {
        let lib = Library::builtin();
        let mut sheet = Sheet::new(SheetId(1), "Root sheet 1", SheetSize::A4);
        sheet.wires.push(Wire { id: WireId(2), points: vec![Pt::ORIGIN, Pt::new(100.0, 0.0)] });
        sheet.junctions.push(Junction { id: JunctionId(3), at: Pt::ORIGIN });
        for kind in [TerminalKind::Default, TerminalKind::Power, TerminalKind::Ground, TerminalKind::Input, TerminalKind::Bus] {
            sheet.terminals.push(Terminal { id: TerminalId(4), at: Pt::ORIGIN, kind, name: "VCC".into(), orient: Orient::rotation(1) });
        }
        let list = sheet_display_list(&sheet, &lib);
        assert_eq!(list.prims[0].ink, Ink::SheetBorder);
        assert_eq!(list.prims.last().unwrap().ink, Ink::Junction, "dots are drawn last");
        assert_eq!(list.of(ItemId::Wire(WireId(2))).count(), 1);
        let b = list.bounds().unwrap();
        assert!(b.width() >= 11_693.0 && b.height() >= 8_268.0);
    }

    #[test]
    fn labels_lift_off_the_wire_and_turn_with_it() {
        let l = Label { id: protasena_doc::LabelId(1), at: Pt::new(100.0, 0.0), text: "CLK".into(), orient: Orient::IDENTITY };
        let p = &label_prims(&l)[0];
        assert!(matches!(p.shape, Shape::Text { at, angle, .. } if at == Pt::new(100.0, 15.0) && angle == 0.0));
        let up = Label { orient: Orient::rotation(1), ..l };
        assert!(matches!(label_prims(&up)[0].shape, Shape::Text { at, angle, .. } if at == Pt::new(85.0, 0.0) && angle == 90.0));
    }
}
