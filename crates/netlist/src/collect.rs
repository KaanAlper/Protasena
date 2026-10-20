//! Gathers the connectivity graph of a project: every pin, wire, junction, label, terminal and
//! net name becomes an element of a union–find, joined by the rules in the crate docs.

use std::collections::{BTreeMap, HashMap};

use protasena_doc::{Component, Project, Sheet, TerminalKind, split_reference};
use protasena_geom::{Pt, Segment};
use protasena_library::{Device, PinKind};

use crate::geometry::{SegmentIndex, key, usable};
use crate::model::{ErcItem, ErcKind, Severity};
use crate::order::UnionFind;
use crate::pins::{PlacedPin, placed_pins};
use crate::source::DeviceSource;

/// Name priority: power names win over labels and plain terminals.
pub(crate) const POWER_NAME: u8 = 0;
/// Name priority of labels and signal terminals.
pub(crate) const SIGNAL_NAME: u8 = 1;

/// One component the library knows.
pub(crate) struct Comp<'a> {
    pub c: &'a Component,
    pub device: &'a Device,
    /// Power symbols (ground, rails) join nets but are not parts.
    pub power: bool,
}

/// One pin of a known component.
pub(crate) struct PinRec {
    pub comp: usize,
    pub pin: PlacedPin,
    pub elem: usize,
}

/// What an element of the union–find stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Elem {
    /// A pin (index into `pins`).
    Pin(usize),
    /// A wire, junction, label or terminal.
    Conductor,
    /// A global net name (key of `names`).
    Name,
}

/// The connectivity graph.
#[derive(Default)]
pub(crate) struct Graph<'a> {
    pub uf: UnionFind,
    pub elems: Vec<Elem>,
    pub comps: Vec<Comp<'a>>,
    pub pins: Vec<PinRec>,
    /// Net name -> (element, best priority).
    pub names: BTreeMap<String, (usize, u8)>,
    pub erc: Vec<ErcItem>,
}

/// Attachment points and wire segments of one sheet.
#[derive(Default)]
struct SheetGeom {
    points: Vec<(Pt, usize)>,
    segments: SegmentIndex,
}

impl<'a> Graph<'a> {
    pub(crate) fn build<S: DeviceSource + ?Sized>(project: &'a Project, devices: &'a S) -> Graph<'a> {
        let mut g = Graph::default();
        for sheet in &project.sheets {
            let mut geom = SheetGeom::default();
            g.components(sheet, devices, &mut geom);
            g.conductors(sheet, &mut geom);
            g.connect(&geom);
        }
        g.merge_gates();
        g
    }

    fn add(&mut self, kind: Elem) -> usize {
        self.elems.push(kind);
        self.uf.add()
    }

    fn name(&mut self, name: &str, priority: u8, elem: usize) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let e = match self.names.get_mut(name) {
            Some((e, p)) => {
                *p = (*p).min(priority);
                *e
            }
            None => {
                let e = self.add(Elem::Name);
                self.names.insert(name.to_owned(), (e, priority));
                e
            }
        };
        self.uf.union(e, elem);
    }

    fn warn_geometry(&mut self, what: String) {
        let message = format!("{what} has a coordinate that is not finite or out of range; it connects to nothing there");
        self.erc.push(ErcItem { severity: Severity::Warning, kind: ErcKind::InvalidGeometry, net: None, nodes: Vec::new(), message });
    }

    fn components<S: DeviceSource + ?Sized>(&mut self, sheet: &'a Sheet, devices: &'a S, geom: &mut SheetGeom) {
        for c in &sheet.components {
            let Some(device) = devices.device(&c.device) else {
                let message = format!("{} ({}): device {} is not in the library", c.reference, c.id, c.device.name);
                self.erc.push(ErcItem { severity: Severity::Error, kind: ErcKind::UnknownDevice, net: None, nodes: Vec::new(), message });
                continue;
            };
            let placed = placed_pins(c, device);
            if placed.is_empty() && !device.symbol.units.is_empty() {
                let message = format!("{} ({}): device {} has no gate {}", c.reference, c.id, device.name, c.unit);
                self.erc.push(ErcItem { severity: Severity::Error, kind: ErcKind::UnknownDevice, net: None, nodes: Vec::new(), message });
            }
            let power = device.symbol.power;
            let comp = self.comps.len();
            self.comps.push(Comp { c, device, power });
            let mut bad = false;
            for pin in placed {
                let elem = self.add(Elem::Pin(self.pins.len()));
                if power {
                    let value = c.value.trim();
                    self.name(if value.is_empty() { &pin.name } else { value }, POWER_NAME, elem);
                }
                if pin.hidden {
                    if matches!(pin.kind, PinKind::PowerIn | PinKind::PowerOut) {
                        self.name(&pin.name, POWER_NAME, elem);
                    }
                } else if usable(pin.at) {
                    geom.points.push((pin.at, elem));
                } else {
                    bad = true;
                }
                self.pins.push(PinRec { comp, pin, elem });
            }
            if bad {
                self.warn_geometry(format!("{} ({})", c.reference, c.id));
            }
        }
    }

    fn conductors(&mut self, sheet: &Sheet, geom: &mut SheetGeom) {
        for w in &sheet.wires {
            let elem = self.add(Elem::Conductor);
            let mut ok = w.points.iter().all(|&p| usable(p));
            for pair in w.points.windows(2) {
                if let [a, b] = pair {
                    ok &= geom.segments.insert(Segment::new(*a, *b), elem);
                }
            }
            for end in [w.points.first(), w.points.last()].into_iter().flatten() {
                if usable(*end) {
                    geom.points.push((*end, elem));
                }
            }
            if !ok {
                self.warn_geometry(w.id.to_string());
            }
        }
        let mut points: Vec<(Pt, String, Option<(String, u8)>)> = Vec::new();
        points.extend(sheet.junctions.iter().map(|j| (j.at, j.id.to_string(), None)));
        points.extend(sheet.labels.iter().map(|l| (l.at, l.id.to_string(), Some((l.text.clone(), SIGNAL_NAME)))));
        for t in &sheet.terminals {
            let name = match t.kind {
                TerminalKind::Bus => continue,
                TerminalKind::Power if t.name.trim().is_empty() => Some(("VCC".to_owned(), POWER_NAME)),
                TerminalKind::Ground if t.name.trim().is_empty() => Some(("GND".to_owned(), POWER_NAME)),
                TerminalKind::Power | TerminalKind::Ground => Some((t.name.clone(), POWER_NAME)),
                TerminalKind::Default | TerminalKind::Input | TerminalKind::Output | TerminalKind::Bidir => Some((t.name.clone(), SIGNAL_NAME)),
            };
            points.push((t.at, t.id.to_string(), name));
        }
        for (at, what, name) in points {
            let elem = self.add(Elem::Conductor);
            if let Some((name, priority)) = name {
                self.name(&name, priority, elem);
            }
            if usable(at) {
                geom.points.push((at, elem));
            } else {
                self.warn_geometry(what);
            }
        }
    }

    /// Coincident attachment points join; a point on a wire segment joins that wire.
    fn connect(&mut self, geom: &SheetGeom) {
        let mut first_at: HashMap<(i64, i64), usize> = HashMap::new();
        for &(p, elem) in &geom.points {
            if let Some(k) = key(p) {
                let first = *first_at.entry(k).or_insert(elem);
                self.uf.union(first, elem);
            }
            for wire in geom.segments.hits(p) {
                self.uf.union(wire, elem);
            }
        }
    }

    /// Gates of one part (same annotated reference, same device, different units) share pins
    /// by number; any other repeated reference is reported.
    fn merge_gates(&mut self) {
        let mut groups: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (i, comp) in self.comps.iter().enumerate().filter(|(_, c)| !c.power) {
            groups.entry(comp.c.reference.trim()).or_default().push(i);
        }
        for (reference, members) in groups.into_iter().filter(|(_, m)| m.len() > 1) {
            if self.gates_of_one_part(reference, &members) {
                let mut by_number: HashMap<&str, usize> = HashMap::new();
                for rec in self.pins.iter().filter(|p| members.contains(&p.comp)) {
                    let first = *by_number.entry(rec.pin.number.as_str()).or_insert(rec.elem);
                    self.uf.union(first, rec.elem);
                }
            } else {
                let message = format!("{} parts share the reference \"{reference}\"", members.len());
                self.erc.push(ErcItem { severity: Severity::Error, kind: ErcKind::DuplicateReference, net: None, nodes: Vec::new(), message });
            }
        }
    }

    pub(crate) fn gates_of_one_part(&self, reference: &str, members: &[usize]) -> bool {
        let annotated = matches!(split_reference(reference), Some((_, Some(_))));
        let comps: Vec<&Comp> = members.iter().filter_map(|&i| self.comps.get(i)).collect();
        let same_device = comps.windows(2).all(|w| matches!(w, [a, b] if a.device.name.eq_ignore_ascii_case(&b.device.name)));
        let mut units: Vec<u32> = comps.iter().map(|c| c.c.unit).collect();
        units.sort_unstable();
        units.dedup();
        annotated && same_device && units.len() == comps.len()
    }
}
