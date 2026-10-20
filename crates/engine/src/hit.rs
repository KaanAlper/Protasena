//! Hit testing: what lies under a point of the editing window, and what a box encloses.

use std::collections::HashSet;

use protasena_doc::{BusEntryId, BusId, ComponentId, GraphicId, ItemId, JunctionId, LabelId, Sheet, TerminalId, TextId, WireId};
use protasena_geom::{Orient, Pt, Rect, Segment, Vec2};
use protasena_library::Library;
use serde::Serialize;

use crate::bodies::{outline_bounds, outward, visible_pins};

/// Default text height in thou (labels and texts of size 0).
pub const TEXT_SIZE: f64 = 100.0;

/// Reach of a terminal symbol from its connection point, in thou.
const TERMINAL_REACH: f64 = 100.0;

/// Something under the cursor. Variants are listed by priority: a pin end wins over the wire
/// that ends on it, which wins over the component body behind it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Hit {
    /// A visible pin's connection point.
    PinEnd {
        /// The component.
        component: ComponentId,
        /// Pin number.
        pin: String,
        /// Connection point.
        at: Pt,
        /// Direction a wire leaves the pin (away from the body).
        out: Vec2,
    },
    /// The first or last point of a wire.
    WireEnd {
        /// The wire.
        wire: WireId,
        /// The end point.
        at: Pt,
    },
    /// A junction dot.
    Junction {
        /// The junction.
        id: JunctionId,
        /// Its position.
        at: Pt,
    },
    /// A terminal.
    Terminal {
        /// The terminal.
        id: TerminalId,
        /// Its connection point.
        at: Pt,
    },
    /// A wire label.
    Label {
        /// The label.
        id: LabelId,
    },
    /// A free text.
    Text {
        /// The text.
        id: TextId,
    },
    /// A wire, away from its ends.
    Wire {
        /// The wire.
        wire: WireId,
        /// The closest point on the wire.
        at: Pt,
    },
    /// A bus.
    Bus {
        /// The bus.
        id: BusId,
        /// The closest point on the bus.
        at: Pt,
    },
    /// A bus entry.
    BusEntry {
        /// The bus entry.
        id: BusEntryId,
    },
    /// A 2D graphic's outline.
    Graphic {
        /// The graphic.
        id: GraphicId,
    },
    /// A component body.
    Component {
        /// The component.
        id: ComponentId,
    },
}

impl Hit {
    /// The item the hit belongs to (a pin end belongs to its component).
    pub fn item(&self) -> ItemId {
        match self {
            Hit::PinEnd { component, .. } | Hit::Component { id: component } => ItemId::Component(*component),
            Hit::WireEnd { wire, .. } | Hit::Wire { wire, .. } => ItemId::Wire(*wire),
            Hit::Junction { id, .. } => ItemId::Junction(*id),
            Hit::Terminal { id, .. } => ItemId::Terminal(*id),
            Hit::Label { id } => ItemId::Label(*id),
            Hit::Text { id } => ItemId::Text(*id),
            Hit::Bus { id, .. } => ItemId::Bus(*id),
            Hit::BusEntry { id } => ItemId::BusEntry(*id),
            Hit::Graphic { id } => ItemId::Graphic(*id),
        }
    }

    /// The point a wire attaches to, for hits a wire can start or end on.
    pub fn attach_point(&self) -> Option<Pt> {
        match self {
            Hit::PinEnd { at, .. } | Hit::WireEnd { at, .. } | Hit::Junction { at, .. } | Hit::Terminal { at, .. } | Hit::Wire { at, .. } => {
                Some(*at)
            }
            _ => None,
        }
    }

    /// Where a wire may start: pin ends, wire ends, junctions and terminals (Proteus starts
    /// wires from connection points; a plain click on a wire selects or labels it).
    pub fn starts_wire(&self) -> bool {
        matches!(self, Hit::PinEnd { .. } | Hit::WireEnd { .. } | Hit::Junction { .. } | Hit::Terminal { .. })
    }

    fn rank(&self) -> u8 {
        match self {
            Hit::PinEnd { .. } => 0,
            Hit::WireEnd { .. } | Hit::Junction { .. } => 1,
            Hit::Terminal { .. } | Hit::Label { .. } => 2,
            Hit::Text { .. } => 3,
            Hit::Wire { .. } => 4,
            Hit::Bus { .. } | Hit::BusEntry { .. } | Hit::Graphic { .. } => 5,
            Hit::Component { .. } => 6,
        }
    }
}

/// Everything within `tol` of `p` on the sheet, best first (by priority, then distance).
pub fn hits(sheet: &Sheet, library: &Library, p: Pt, tol: f64) -> Vec<Hit> {
    if !p.is_finite() || !tol.is_finite() || tol < 0.0 {
        return Vec::new();
    }
    let mut found: Vec<(Hit, f64)> = Vec::new();
    for c in &sheet.components {
        for pin in visible_pins(library, c) {
            let d = pin.at.distance(p);
            if d <= tol {
                found.push((Hit::PinEnd { component: c.id, pin: pin.number.clone(), at: pin.at, out: outward(&pin) }, d));
            }
        }
        let outline = outline_bounds(library, c);
        if outline.inflate(tol).contains(p) {
            found.push((Hit::Component { id: c.id }, outline.center().distance(p)));
        }
    }
    for w in &sheet.wires {
        let ends = [w.points.first(), w.points.last()];
        let end = ends.into_iter().flatten().map(|&e| (e, e.distance(p))).filter(|(_, d)| *d <= tol).min_by(|a, b| a.1.total_cmp(&b.1));
        match end {
            Some((at, d)) => found.push((Hit::WireEnd { wire: w.id, at }, d)),
            None => {
                if let Some((at, d)) = closest_on(&w.points, p).filter(|(_, d)| *d <= tol) {
                    found.push((Hit::Wire { wire: w.id, at }, d));
                }
            }
        }
    }
    for j in &sheet.junctions {
        push_near(&mut found, j.at, p, tol, Hit::Junction { id: j.id, at: j.at });
    }
    for t in &sheet.terminals {
        let reach = Segment::new(t.at, t.at + t.orient.apply_vec(Vec2::new(TERMINAL_REACH, 0.0)));
        let d = reach.distance_to_pt(p);
        if d <= tol {
            found.push((Hit::Terminal { id: t.id, at: t.at }, t.at.distance(p)));
        }
    }
    for l in &sheet.labels {
        if text_box(l.at, &l.text, TEXT_SIZE, l.orient).inflate(tol).contains(p) {
            found.push((Hit::Label { id: l.id }, l.at.distance(p)));
        }
    }
    for t in &sheet.texts {
        let size = if t.size > 0.0 { t.size } else { TEXT_SIZE };
        if text_box(t.at, &t.text, size, t.orient).inflate(tol).contains(p) {
            found.push((Hit::Text { id: t.id }, t.at.distance(p)));
        }
    }
    for b in &sheet.buses {
        if let Some((at, d)) = closest_on(&b.points, p).filter(|(_, d)| *d <= tol) {
            found.push((Hit::Bus { id: b.id, at }, d));
        }
    }
    for e in &sheet.bus_entries {
        let d = Segment::new(e.bus_end, e.wire_end).distance_to_pt(p);
        if d <= tol {
            found.push((Hit::BusEntry { id: e.id }, d));
        }
    }
    for g in &sheet.graphics {
        let d = crate::graphics::distance(&g.shape, p);
        if d <= tol {
            found.push((Hit::Graphic { id: g.id }, d));
        }
    }
    found.sort_by(|(a, da), (b, db)| a.rank().cmp(&b.rank()).then(da.total_cmp(db)));
    found.into_iter().map(|(h, _)| h).collect()
}

/// The best hit within `tol` of `p`.
pub fn hit(sheet: &Sheet, library: &Library, p: Pt, tol: f64) -> Option<Hit> {
    hits(sheet, library, p, tol).into_iter().next()
}

/// The bounds of an item on the sheet.
pub fn item_bounds(sheet: &Sheet, library: &Library, id: ItemId) -> Option<Rect> {
    match id {
        ItemId::Component(i) => sheet.components.iter().find(|c| c.id == i).map(|c| outline_bounds(library, c)),
        ItemId::Wire(i) => sheet.wires.iter().find(|w| w.id == i).and_then(|w| Rect::from_points(w.points.iter().copied())),
        ItemId::Bus(i) => sheet.buses.iter().find(|b| b.id == i).and_then(|b| Rect::from_points(b.points.iter().copied())),
        ItemId::Junction(i) => sheet.junctions.iter().find(|j| j.id == i).map(|j| Rect::from_point(j.at)),
        ItemId::Terminal(i) => sheet.terminals.iter().find(|t| t.id == i).map(|t| Rect::from_point(t.at)),
        ItemId::Label(i) => sheet.labels.iter().find(|l| l.id == i).map(|l| Rect::from_point(l.at)),
        ItemId::Text(i) => sheet.texts.iter().find(|t| t.id == i).map(|t| Rect::from_point(t.at)),
        ItemId::BusEntry(i) => sheet.bus_entries.iter().find(|e| e.id == i).map(|e| Rect::new(e.bus_end, e.wire_end)),
        ItemId::Graphic(i) => sheet.graphics.iter().find(|g| g.id == i).and_then(|g| g.shape.bounds()),
    }
}

/// The items wholly inside `area` (box selection).
pub fn items_in(sheet: &Sheet, library: &Library, area: Rect) -> Vec<ItemId> {
    let area = area.normalized();
    every_bounds(sheet, library).into_iter().filter(|(_, b)| area.contains_rect(*b)).map(|(id, _)| id).collect()
}

/// The union of the bounds of some items.
pub fn bounds_of(sheet: &Sheet, library: &Library, ids: &[ItemId]) -> Option<Rect> {
    let wanted: HashSet<ItemId> = ids.iter().copied().collect();
    every_bounds(sheet, library).into_iter().filter(|(id, _)| wanted.contains(id)).map(|(_, b)| b).reduce(Rect::union)
}

/// The bounds of every item on the sheet, in one pass.
fn every_bounds(sheet: &Sheet, library: &Library) -> Vec<(ItemId, Rect)> {
    let poly = |pts: &[Pt]| Rect::from_points(pts.iter().copied());
    let mut out: Vec<(ItemId, Rect)> = Vec::new();
    out.extend(sheet.components.iter().map(|c| (ItemId::Component(c.id), outline_bounds(library, c))));
    out.extend(sheet.wires.iter().filter_map(|w| Some((ItemId::Wire(w.id), poly(&w.points)?))));
    out.extend(sheet.buses.iter().filter_map(|b| Some((ItemId::Bus(b.id), poly(&b.points)?))));
    out.extend(sheet.junctions.iter().map(|j| (ItemId::Junction(j.id), Rect::from_point(j.at))));
    out.extend(sheet.terminals.iter().map(|t| (ItemId::Terminal(t.id), Rect::from_point(t.at))));
    out.extend(sheet.labels.iter().map(|l| (ItemId::Label(l.id), Rect::from_point(l.at))));
    out.extend(sheet.texts.iter().map(|t| (ItemId::Text(t.id), Rect::from_point(t.at))));
    out.extend(sheet.bus_entries.iter().map(|e| (ItemId::BusEntry(e.id), Rect::new(e.bus_end, e.wire_end))));
    out.extend(sheet.graphics.iter().filter_map(|g| Some((ItemId::Graphic(g.id), g.shape.bounds()?))));
    out
}

/// The closest point of a polyline to `p`, with its distance.
pub fn closest_on(points: &[Pt], p: Pt) -> Option<(Pt, f64)> {
    points
        .windows(2)
        .filter_map(|w| match w {
            [a, b] => {
                let c = Segment::new(*a, *b).closest_point(p);
                Some((c, c.distance(p)))
            }
            _ => None,
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// A rough box around a one-line text anchored at its start.
fn text_box(at: Pt, text: &str, size: f64, orient: Orient) -> Rect {
    let width = size * 0.6 * text.chars().count().clamp(1, 1024) as f64;
    Rect::new(at, at + orient.apply_vec(Vec2::new(width, size)))
}

fn push_near(found: &mut Vec<(Hit, f64)>, at: Pt, p: Pt, tol: f64, hit: Hit) {
    let d = at.distance(p);
    if d <= tol {
        found.push((hit, d));
    }
}
