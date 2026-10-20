//! A small design builder over the built-in library.
#![allow(dead_code, clippy::unwrap_used, clippy::indexing_slicing)]

use protasena_doc::*;
use protasena_geom::{Orient, Pt, Transform, Vec2};
use protasena_library::Library;
use protasena_netlist::Netlist;

pub struct Design {
    pub p: Project,
    pub lib: Library,
    pub sheet: SheetId,
}

pub fn pt(x: f64, y: f64) -> Pt {
    Pt::new(x, y)
}

impl Design {
    pub fn new() -> Self {
        let p = Project::new("test");
        let sheet = p.sheets[0].id;
        Self { p, lib: Library::builtin(), sheet }
    }

    fn add(&mut self, item: Item) {
        self.p.apply(Op::Add { sheet: self.sheet, item, index: None }).unwrap();
    }

    /// Adds a second sheet and makes it current.
    pub fn new_sheet(&mut self) -> SheetId {
        let id = self.p.fresh_id().unwrap();
        self.p.apply(Op::AddSheet { sheet: Sheet::new(id, "Sheet 2", SheetSize::A4), index: None }).unwrap();
        self.sheet = id;
        id
    }

    pub fn part_at(&mut self, device: &str, reference: &str, value: &str, at: (f64, f64), orient: Orient) -> ComponentId {
        let id = self.p.fresh_id().unwrap();
        let c = Component::new(id, DeviceRef::new("GENERIC", device), reference, value).placed(Transform::new(orient, Vec2::new(at.0, at.1)));
        self.add(Item::Component(c));
        id
    }

    pub fn part(&mut self, device: &str, reference: &str, at: (f64, f64)) -> ComponentId {
        self.part_at(device, reference, "", at, Orient::IDENTITY)
    }

    pub fn wire(&mut self, points: &[(f64, f64)]) -> WireId {
        let id = self.p.fresh_id().unwrap();
        self.add(Item::Wire(Wire { id, points: points.iter().map(|&(x, y)| pt(x, y)).collect() }));
        id
    }

    pub fn junction(&mut self, at: (f64, f64)) {
        let id = self.p.fresh_id().unwrap();
        self.add(Item::Junction(Junction { id, at: pt(at.0, at.1) }));
    }

    pub fn label(&mut self, at: (f64, f64), text: &str) {
        let id = self.p.fresh_id().unwrap();
        self.add(Item::Label(Label { id, at: pt(at.0, at.1), text: text.to_owned(), orient: Orient::IDENTITY, global: false }));
    }

    pub fn terminal(&mut self, at: (f64, f64), kind: TerminalKind, name: &str) {
        let id = self.p.fresh_id().unwrap();
        self.add(Item::Terminal(Terminal { id, at: pt(at.0, at.1), kind, name: name.to_owned(), orient: Orient::IDENTITY }));
    }

    pub fn global_label(&mut self, at: (f64, f64), text: &str) {
        let id = self.p.fresh_id().unwrap();
        self.add(Item::Label(Label { id, at: pt(at.0, at.1), text: text.to_owned(), orient: Orient::IDENTITY, global: true }));
    }

    /// A subcircuit box from `a` to `b` named `name` with `ports` (`(at, name, kind)`), linked
    /// to `child`.
    pub fn subcircuit(
        &mut self,
        a: (f64, f64),
        b: (f64, f64),
        name: &str,
        ports: &[((f64, f64), &str, PortKind)],
        child: Option<SheetId>,
    ) -> SubcircuitId {
        let id = self.p.fresh_id().unwrap();
        let mut s = Subcircuit::new(id, pt(a.0, a.1), pt(b.0, b.1), name);
        s.ports = ports.iter().map(|&(at, name, kind)| Port { at: pt(at.0, at.1), name: name.to_owned(), kind }).collect();
        s.child = child;
        self.add(Item::Subcircuit(s));
        id
    }

    /// Adds a sheet named `name` without making it current.
    pub fn sheet_named(&mut self, name: &str) -> SheetId {
        let id = self.p.fresh_id().unwrap();
        self.p.apply(Op::AddSheet { sheet: Sheet::new(id, name, SheetSize::A4), index: None }).unwrap();
        id
    }

    pub fn netlist(&self) -> Netlist {
        protasena_netlist::build(&self.p, &self.lib)
    }
}

/// The nodes of the net a pin is on, as `R1.2` strings.
pub fn net_nodes(n: &Netlist, reference: &str, pin: &str) -> Vec<String> {
    n.net_of(reference, pin).map(|net| net.nodes.iter().map(ToString::to_string).collect()).unwrap_or_default()
}

pub fn net_name(n: &Netlist, reference: &str, pin: &str) -> Option<String> {
    n.net_of(reference, pin).map(|net| net.name.clone())
}
