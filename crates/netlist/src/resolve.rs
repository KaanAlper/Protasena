//! Turns the connectivity graph into nets with stable names, parts and the rules-check report.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use protasena_library::PinKind;

use crate::collect::{Elem, Graph};
use crate::model::{ErcItem, ErcKind, Net, Netlist, Node, Part, PartPin, Severity};
use crate::order::{natural_cmp, node_cmp};

/// Prefix of generated net names.
pub const AUTO_NET_PREFIX: &str = "#NET";

/// Everything one union–find set holds.
#[derive(Default)]
struct Set {
    pins: Vec<usize>,
    conductors: usize,
    names: Vec<(u8, String)>,
}

impl Graph<'_> {
    pub(crate) fn resolve(mut self) -> Netlist {
        let sets = self.sets();
        let mut erc = std::mem::take(&mut self.erc);
        let mut unconnected = Vec::new();
        // (root, net, pins of parts) for every set with at least one pin of a part.
        let mut nets: Vec<(usize, Net, Vec<usize>)> = Vec::new();
        for (&root, set) in &sets {
            let real: Vec<usize> = set.pins.iter().copied().filter(|&p| self.is_part_pin(p)).collect();
            let nodes = self.nodes(&real);
            if nodes.is_empty() {
                continue;
            }
            let has_power_pin = real.len() != set.pins.len();
            if set.conductors == 0 && set.names.is_empty() && !has_power_pin && nodes.len() == 1 {
                if real.iter().any(|&p| self.reportable(p)) {
                    unconnected.extend(nodes);
                }
                continue;
            }
            let mut names = set.names.clone();
            names.sort_by(|(pa, a), (pb, b)| pa.cmp(pb).then_with(|| natural_cmp(a, b)));
            let mut names = names.into_iter().map(|(_, n)| n);
            let name = names.next().unwrap_or_default();
            let aliases: BTreeSet<String> = names.collect();
            nets.push((root, Net { name, nodes, aliases: aliases.into_iter().collect() }, real));
        }
        name_unnamed(&mut nets);
        for (_, net, real) in &nets {
            self.check_net(real, net, &mut erc);
        }
        unconnected.sort_by(node_cmp);
        unconnected.dedup();
        for node in &unconnected {
            erc.push(item(Severity::Warning, ErcKind::UnconnectedPin, None, vec![node.clone()], format!("pin {node} is not connected")));
        }
        let net_of: HashMap<usize, String> = nets.iter().map(|(root, n, _)| (*root, n.name.clone())).collect();
        let elems: Vec<usize> = self.pins.iter().map(|r| r.elem).collect();
        let pin_roots: Vec<usize> = elems.into_iter().map(|e| self.uf.find(e)).collect();
        let parts = self.parts(&net_of, &pin_roots);
        let mut nets: Vec<Net> = nets.into_iter().map(|(_, n, _)| n).collect();
        nets.sort_by(|a, b| natural_cmp(&a.name, &b.name));
        erc.sort_by(|a, b| (a.severity, a.kind).cmp(&(b.severity, b.kind)).then_with(|| natural_cmp(&a.message, &b.message)));
        Netlist { nets, unconnected_pins: unconnected, parts, erc }
    }

    fn sets(&mut self) -> BTreeMap<usize, Set> {
        let mut sets: BTreeMap<usize, Set> = BTreeMap::new();
        for e in 0..self.elems.len() {
            let root = self.uf.find(e);
            let set = sets.entry(root).or_default();
            match self.elems.get(e) {
                Some(Elem::Pin(p)) => set.pins.push(*p),
                Some(Elem::Conductor) => set.conductors += 1,
                Some(Elem::Name) | None => {}
            }
        }
        for (name, &(e, priority)) in &self.names {
            let root = self.uf.find(e);
            sets.entry(root).or_default().names.push((priority, name.clone()));
        }
        sets
    }

    fn is_part_pin(&self, pin: usize) -> bool {
        self.pins.get(pin).and_then(|r| self.comps.get(r.comp)).is_some_and(|c| !c.power)
    }

    /// Whether an unconnected pin is worth reporting (not a no-connect or a hidden pin).
    fn reportable(&self, pin: usize) -> bool {
        self.pins.get(pin).is_some_and(|r| !r.pin.hidden && r.pin.kind != PinKind::NoConnect)
    }

    fn node(&self, pin: usize) -> Option<(Node, PinKind)> {
        let rec = self.pins.get(pin)?;
        let comp = self.comps.get(rec.comp)?;
        Some((Node::new(comp.c.reference.trim(), &rec.pin.number), rec.pin.kind))
    }

    /// Distinct nodes of some pins, in natural order.
    fn nodes(&self, pins: &[usize]) -> Vec<Node> {
        let mut nodes: Vec<Node> = pins.iter().filter_map(|&p| self.node(p)).map(|(n, _)| n).collect();
        nodes.sort_by(node_cmp);
        nodes.dedup();
        nodes
    }

    /// Reports the net's single pin, its extra names and conflicting outputs.
    fn check_net(&self, pins: &[usize], net: &Net, erc: &mut Vec<ErcItem>) {
        if !net.aliases.is_empty() {
            let message = format!("net {} is also named {}", net.name, net.aliases.join(", "));
            erc.push(item(Severity::Warning, ErcKind::MultipleNames, Some(&net.name), Vec::new(), message));
        }
        if let [only] = net.nodes.as_slice() {
            let message = format!("net {} has only one pin, {only}", net.name);
            erc.push(item(Severity::Warning, ErcKind::SingleNodeNet, Some(&net.name), net.nodes.clone(), message));
        }
        let mut drivers: Vec<Node> =
            pins.iter().filter_map(|&p| self.node(p)).filter(|(_, k)| matches!(k, PinKind::Output | PinKind::PowerOut)).map(|(n, _)| n).collect();
        drivers.sort_by(node_cmp);
        drivers.dedup();
        if drivers.len() >= 2 {
            let list: Vec<String> = drivers.iter().map(Node::to_string).collect();
            let message = format!("outputs {} drive net {}", list.join(", "), net.name);
            erc.push(item(Severity::Error, ErcKind::OutputConflict, Some(&net.name), drivers, message));
        }
    }

    fn parts(&self, net_of: &HashMap<usize, String>, pin_roots: &[usize]) -> Vec<Part> {
        // Reference -> components; gates of one part share an entry, other repeats stay apart.
        let mut by_ref: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (i, comp) in self.comps.iter().enumerate().filter(|(_, c)| !c.power) {
            by_ref.entry(comp.c.reference.trim()).or_default().push(i);
        }
        let mut groups: Vec<Vec<usize>> = Vec::new();
        for (reference, members) in by_ref {
            if members.len() > 1 && !self.gates_of_one_part(reference, &members) {
                groups.extend(members.into_iter().map(|m| vec![m]));
            } else {
                groups.push(members);
            }
        }
        let mut parts: Vec<Part> = groups.into_iter().filter_map(|g| self.part(&g, net_of, pin_roots)).collect();
        parts.sort_by(|a, b| natural_cmp(&a.reference, &b.reference));
        parts
    }

    fn part(&self, members: &[usize], net_of: &HashMap<usize, String>, pin_roots: &[usize]) -> Option<Part> {
        let mut members = members.to_vec();
        members.sort_by_key(|&m| self.comps.get(m).map(|c| c.c.unit));
        let first = self.comps.get(*members.first()?)?;
        let mut seen = HashSet::new();
        let mut pins = Vec::new();
        for &m in &members {
            for (i, rec) in self.pins.iter().enumerate().filter(|(_, r)| r.comp == m) {
                if seen.insert(rec.pin.number.clone()) {
                    let net = pin_roots.get(i).and_then(|root| net_of.get(root)).cloned();
                    pins.push(PartPin { number: rec.pin.number.clone(), name: rec.pin.name.clone(), net });
                }
            }
        }
        let sim = first.device.sim_model.as_ref();
        Some(Part {
            reference: first.c.reference.trim().to_owned(),
            device: first.c.device.clone(),
            value: first.c.value.clone(),
            primitive: sim.map(|s| s.primitive.clone()),
            model: sim.map(|s| s.model.clone()),
            pins,
        })
    }
}

/// Names nets without a label or terminal `#NETnnnn`, numbered in the natural order of their
/// lowest pin, skipping numbers an explicit name already uses.
fn name_unnamed(nets: &mut [(usize, Net, Vec<usize>)]) {
    let taken: HashSet<String> = nets.iter().map(|(_, n, _)| n.name.clone()).filter(|n| !n.is_empty()).collect();
    let mut unnamed: Vec<&mut Net> = nets.iter_mut().map(|(_, n, _)| n).filter(|n| n.name.is_empty()).collect();
    unnamed.sort_by(|a, b| match (a.nodes.first(), b.nodes.first()) {
        (Some(x), Some(y)) => node_cmp(x, y),
        _ => std::cmp::Ordering::Equal,
    });
    let mut n: u64 = 0;
    for net in unnamed {
        let name = loop {
            n = n.saturating_add(1);
            let candidate = format!("{AUTO_NET_PREFIX}{n:04}");
            if !taken.contains(&candidate) || n == u64::MAX {
                break candidate;
            }
        };
        net.name = name;
    }
}

fn item(severity: Severity, kind: ErcKind, net: Option<&str>, nodes: Vec<Node>, message: String) -> ErcItem {
    ErcItem { severity, kind, net: net.map(str::to_owned), nodes, message }
}
