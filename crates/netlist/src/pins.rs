//! Pin positions in sheet space: a device's symbol pins placed by a component's transform.

use protasena_doc::Component;
use protasena_geom::Pt;
use protasena_library::{Device, Pin, PinDir, PinKind, Unit};

/// One pin of a placed component, in sheet coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedPin {
    /// Pin number.
    pub number: String,
    /// Pin name.
    pub name: String,
    /// Electrical type.
    pub kind: PinKind,
    /// Connection point (where wires attach), on the sheet.
    pub at: Pt,
    /// Direction from the connection point towards the body, on the sheet.
    pub dir: PinDir,
    /// Hidden pin (not drawn; power pins among them join nets by name).
    pub hidden: bool,
}

/// The units that make up gate `gate` of a symbol: the `gate`-th distinct unit name, in body
/// style 1 when it has one (else its first body style). See [`protasena_library::Symbol::gate`].
pub fn gate_units(device: &Device, gate: u32) -> Vec<&Unit> {
    device.symbol.gate(gate)
}

/// The pins of `component` (its gate of `device`) placed on the sheet. Empty when the gate does
/// not exist.
pub fn placed_pins(component: &Component, device: &Device) -> Vec<PlacedPin> {
    gate_units(device, component.unit).into_iter().flat_map(|u| u.pins.iter()).map(|p| place(component, p)).collect()
}

fn place(component: &Component, pin: &Pin) -> PlacedPin {
    let t = component.transform;
    PlacedPin {
        number: pin.number.clone(),
        name: pin.name.clone(),
        kind: pin.kind,
        at: t.apply_pt(pin.at),
        dir: pin.orient.transformed(t.orient),
        hidden: pin.hidden,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protasena_doc::{ComponentId, DeviceRef};
    use protasena_geom::{Orient, Transform, Vec2};
    use protasena_library::{Library, Symbol};

    #[test]
    fn pins_follow_rotation_mirror_and_offset() {
        let lib = Library::builtin();
        let res = lib.get("RES").unwrap();
        let place = |o: Orient| {
            let c = Component::new(ComponentId(1), DeviceRef::new("GENERIC", "RES"), "R1", "").placed(Transform::new(o, Vec2::new(1000.0, 500.0)));
            placed_pins(&c, res).into_iter().map(|p| (p.at, p.dir)).collect::<Vec<_>>()
        };
        assert_eq!(place(Orient::IDENTITY), [(Pt::new(800.0, 500.0), PinDir::Right), (Pt::new(1200.0, 500.0), PinDir::Left)]);
        assert_eq!(place(Orient::rotation(1)), [(Pt::new(1000.0, 300.0), PinDir::Up), (Pt::new(1000.0, 700.0), PinDir::Down)]);
        assert_eq!(place(Orient::MIRROR_X), [(Pt::new(1200.0, 500.0), PinDir::Left), (Pt::new(800.0, 500.0), PinDir::Right)]);
    }

    #[test]
    fn gates_pick_units_by_distinct_name() {
        let unit = |name: &str, style: u8| Unit { name: name.into(), body_style: style, ..Unit::default() };
        let dev = Device { symbol: Symbol { units: vec![unit("A", 2), unit("A", 1), unit("B", 2)], power: false }.into(), ..Device::default() };
        assert_eq!(gate_units(&dev, 0).iter().map(|u| u.body_style).collect::<Vec<_>>(), [1]);
        assert_eq!(gate_units(&dev, 1).iter().map(|u| (u.name.as_str(), u.body_style)).collect::<Vec<_>>(), [("B", 2)]);
        assert!(gate_units(&dev, 2).is_empty());
        assert!(gate_units(&dev, u32::MAX).is_empty());
    }
}
