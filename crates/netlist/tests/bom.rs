//! The bill of materials: grouping by value, device and package, natural reference order,
//! quantities, gates, power symbols and unannotated parts.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::*;
use protasena_doc::{Item, Op};
use protasena_geom::Orient;
use protasena_netlist::{bill_of_materials, export};

#[test]
fn parts_group_by_value_device_and_package_with_natural_references() {
    let mut d = Design::new();
    for (r, v, x) in [("R10", "10k", 0.0), ("R2", "10k", 1000.0), ("R1", "4k7", 2000.0), ("R3", "10k", 3000.0), ("C1", "100n", 4000.0)] {
        let dev = if r.starts_with('C') { "CAP" } else { "RES" };
        d.part_at(dev, r, v, (x, 0.0), Orient::IDENTITY);
    }
    // A 10k resistor in another package is a line of its own.
    let id = d.part_at("RES", "R4", "10k", (5000.0, 0.0), Orient::IDENTITY);
    let mut c = d.p.component(id).unwrap().clone();
    c.package = Some("RES60".into());
    d.p.apply(Op::Modify { item: Item::Component(c) }).unwrap();
    d.part_at("POWER", "", "+5V", (6000.0, 0.0), Orient::IDENTITY);
    d.part_at("GROUND", "", "", (7000.0, 0.0), Orient::IDENTITY);
    d.new_sheet();
    d.part_at("RES", "R?", "1k", (0.0, 0.0), Orient::IDENTITY);
    d.part_at("RES", "R?", "1k", (1000.0, 0.0), Orient::IDENTITY);
    let bom = bill_of_materials(&d.p, &d.lib);
    let rows: Vec<(usize, String, &str)> = bom.lines.iter().map(|l| (l.quantity, l.references.join(","), l.value.as_str())).collect();
    assert_eq!(
        rows,
        [
            (1, "C1".to_owned(), "100n"),
            (1, "R1".to_owned(), "4k7"),
            (3, "R2,R3,R10".to_owned(), "10k"),
            (1, "R4".to_owned(), "10k"),
            (2, "R?,R?".to_owned(), "1k"),
        ],
        "power symbols are left out, unannotated parts count one by one"
    );
    assert_eq!(bom.total(), 8);
    let tens: Vec<&str> = bom.lines.iter().filter(|l| l.value == "10k").map(|l| l.package.as_str()).collect();
    assert_eq!(tens.len(), 2);
    assert!(tens.contains(&"RES60"));
    assert!(bom.lines.iter().all(|l| !l.description.is_empty()), "descriptions come from the library");
}

#[test]
fn gates_of_one_part_count_once_and_unknown_devices_still_list() {
    let mut d = Design::new();
    let a = d.part_at("7400", "U1", "7400", (0.0, 0.0), Orient::IDENTITY);
    let b = d.part_at("7400", "U1", "7400", (1000.0, 0.0), Orient::IDENTITY);
    let mut gate = d.p.component(b).unwrap().clone();
    gate.unit = 1;
    d.p.apply(Op::Modify { item: Item::Component(gate) }).unwrap();
    let _ = a;
    d.part_at("NOSUCH", "X1", "?", (2000.0, 0.0), Orient::IDENTITY);
    let bom = bill_of_materials(&d.p, &d.lib);
    let u = bom.lines.iter().find(|l| l.device == "7400").unwrap();
    assert_eq!((u.quantity, u.references.clone()), (1, vec!["U1".to_owned()]));
    let x = bom.lines.iter().find(|l| l.device == "NOSUCH").unwrap();
    assert_eq!((x.quantity, x.description.as_str()), (1, ""));
    let csv = export::bom_to_csv(&bom);
    assert_eq!(csv.lines().count(), 3);
    assert!(export::bom_to_html(&bom).contains("<td>U1</td>"));
}
