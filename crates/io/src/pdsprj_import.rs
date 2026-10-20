//! Importing Proteus `.pdsprj` projects (read-only, partial; see `docs/pdsprj.md`).
//!
//! Every circuit that holds objects becomes a sheet. Wires, buses, junctions (nodes joining
//! three or more records), components and terminals are carried over, with coordinates
//! converted from 10 nm file units to thou. Orientation is not decoded yet: components and
//! terminals are placed unrotated, and the report says so. Generators, probes and records of
//! unknown kinds are skipped and counted. Embedded device definitions are listed in the report
//! only; components name their device in the [`PDSPRJ_LIBRARY`] library.

use std::path::Path;

use protasena_doc::{
    Bus, BusId, Component, ComponentId, DesignProperties, DeviceRef, FIRST_SHEET_NAME, FieldLabel, Junction, JunctionId, Project, Properties, Sheet,
    SheetId, SheetSize, Terminal, TerminalId, TerminalKind, Wire, WireId,
};
use protasena_geom::{Pt, Transform, Vec2};
use protasena_pdsprj::{self as pds, Circuit, Coord, Record};

use crate::pdsprj_report::{EmbeddedDevice, ImportError, ImportReport};

/// Library name that imported components reference their devices in.
pub const PDSPRJ_LIBRARY: &str = "PDSPRJ";

/// Imports a `.pdsprj` (or `.pdsbak`) file. The project is named after the file.
///
/// # Errors
/// [`ImportError`] when the archive or its `ROOT.DSN` cannot be read.
pub fn import_pdsprj(path: &Path) -> Result<(Project, ImportReport), ImportError> {
    let read = pds::Project::open(path)?;
    let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(convert(&read, &name))
}

/// Imports a `.pdsprj` held in memory.
///
/// # Errors
/// As [`import_pdsprj`].
pub fn import_pdsprj_bytes(bytes: Vec<u8>, name: &str) -> Result<(Project, ImportReport), ImportError> {
    Ok(convert(&pds::Project::from_bytes(bytes)?, name))
}

struct Ids(u64);

impl Ids {
    fn next(&mut self) -> u64 {
        let id = self.0;
        self.0 = self.0.saturating_add(1);
        id
    }
}

fn pt(c: Coord) -> Pt {
    let (x, y) = c.to_thou();
    Pt::new(x, y)
}

/// Converts a read project into the design model.
pub fn convert(read: &pds::Project, name: &str) -> (Project, ImportReport) {
    let design = &read.design;
    let mut report = ImportReport {
        members: read.members.iter().map(|m| m.name.clone()).collect(),
        release: read.manifest.as_ref().and_then(|m| m.release).or(Some(u32::from(design.header.release)).filter(|&r| r != 0)),
        ..ImportReport::default()
    };
    let mut ids = Ids(1);
    let mut sheets = Vec::new();
    let with_objects: Vec<&Circuit> = design.circuits.iter().filter(|c| !c.records.is_empty()).collect();
    let circuits = if with_objects.is_empty() { design.circuits.iter().take(1).collect() } else { with_objects };
    for circuit in circuits {
        let index = sheets.len();
        let name = if index == 0 { FIRST_SHEET_NAME.to_owned() } else { format!("Sheet {}", index.saturating_add(1)) };
        sheets.push(convert_circuit(circuit, name, &mut ids, &mut report));
    }
    if sheets.is_empty() {
        sheets.push(Sheet::new(SheetId(ids.next()), FIRST_SHEET_NAME, SheetSize::default()));
        report.warnings.push("the design has no circuit section: the project starts with one empty sheet".into());
    }
    report.sheets = sheets.len();
    describe_library(design.config.as_ref(), &mut report);
    summarise(&mut report);
    let design_props = DesignProperties {
        created: design.header.created_date.clone(),
        modified: design.header.modified_date.clone(),
        ..DesignProperties::default()
    };
    let project = Project { name: name.to_owned(), design: design_props, sheets, next_id: ids.0, ..Project::default() };
    report.warnings.extend(project.issues().iter().map(ToString::to_string));
    (project, report)
}

fn convert_circuit(circuit: &Circuit, name: String, ids: &mut Ids, report: &mut ImportReport) -> Sheet {
    let size = match (circuit.sheet_width, circuit.sheet_height) {
        (w, h) if w > 0 && h > 0 => {
            let k = f64::from(pds::UNITS_PER_THOU);
            SheetSize::Custom { width: f64::from(w) / k, height: f64::from(h) / k }
        }
        _ => SheetSize::default(),
    };
    let mut sheet = Sheet::new(SheetId(ids.next()), &name, size);
    for record in &circuit.records {
        match record {
            Record::Wire(w) if w.points.len() < 2 => report.warnings.push(format!("a wire with {} point(s) was skipped", w.points.len())),
            Record::Wire(w) => {
                let points = w.points.iter().copied().map(pt).collect();
                if w.style.eq_ignore_ascii_case("BUS WIRE") {
                    sheet.buses.push(Bus { id: BusId(ids.next()), points });
                    report.buses = report.buses.saturating_add(1);
                } else {
                    sheet.wires.push(Wire { id: WireId(ids.next()), points });
                    report.wires = report.wires.saturating_add(1);
                }
            }
            Record::Node(n) if n.degree() >= 3 => {
                sheet.junctions.push(Junction { id: JunctionId(ids.next()), at: pt(n.at) });
                report.junctions = report.junctions.saturating_add(1);
            }
            Record::Node(_) => report.skipped_nodes = report.skipped_nodes.saturating_add(1),
            Record::Component(c) => {
                sheet.components.push(component(c, ComponentId(ids.next())));
                report.components = report.components.saturating_add(1);
            }
            Record::Terminal(t) => {
                let kind = terminal_kind(&t.symbol);
                sheet.terminals.push(Terminal {
                    id: TerminalId(ids.next()),
                    at: pt(t.at),
                    kind,
                    name: t.label().to_owned(),
                    orient: Default::default(),
                });
                report.terminals = report.terminals.saturating_add(1);
            }
            Record::Generator(_) => report.skipped_generators = report.skipped_generators.saturating_add(1),
            Record::Unknown { tag, bytes_len, .. } => report.count_unknown(*tag, *bytes_len),
        }
    }
    sheet
}

fn component(c: &pds::Component, id: ComponentId) -> Component {
    let origin = pt(c.at);
    let label =
        |field: Option<&pds::TextField>| field.map_or_else(FieldLabel::default, |f| FieldLabel { offset: pt(f.at) - origin, visible: f.visible });
    let mut package = None;
    let mut properties = Properties::new();
    for (key, value) in c.properties() {
        if key.eq_ignore_ascii_case("PACKAGE") {
            package = Some(value);
        } else {
            properties.set(&key, &value);
        }
    }
    Component {
        id,
        device: DeviceRef::new(PDSPRJ_LIBRARY, &c.device),
        unit: 0,
        reference: c.reference().to_owned(),
        value: c.value().to_owned(),
        transform: Transform::translation(Vec2::new(origin.x, origin.y)),
        package,
        properties,
        reference_label: label(c.reference_field()),
        value_label: label(c.value_field()),
    }
}

/// The kind of a terminal from its symbol name (`$TER` + the Terminals mode name).
fn terminal_kind(symbol: &str) -> TerminalKind {
    let kind = symbol.strip_prefix("$TER").unwrap_or(symbol);
    match kind.to_ascii_uppercase().as_str() {
        "GROUND" => TerminalKind::Ground,
        "POWER" => TerminalKind::Power,
        "INPUT" => TerminalKind::Input,
        "OUTPUT" => TerminalKind::Output,
        "BIDIR" => TerminalKind::Bidir,
        "BUS" => TerminalKind::Bus,
        _ => TerminalKind::Default,
    }
}

fn describe_library(config: Option<&pds::Config>, report: &mut ImportReport) {
    let Some(config) = config else {
        report.warnings.push("the design configuration was not recognised: styles and the embedded library were not read".into());
        return;
    };
    report.embedded_symbols = config.symbols.len();
    report.embedded_devices = config
        .devices
        .iter()
        .map(|d| EmbeddedDevice {
            name: d.name.clone(),
            category: d.section("INDEX").and_then(|s| s.get("CAT")).map(str::to_owned),
            property_defs: d.section("PROPDEFS").map_or(0, |s| s.records.len()),
        })
        .collect();
}

fn summarise(report: &mut ImportReport) {
    let mut notes = Vec::new();
    if report.components > 0 || report.terminals > 0 {
        notes.push(format!(
            "orientation is not decoded yet: {} component(s) and {} terminal(s) are placed unrotated",
            report.components, report.terminals
        ));
    }
    if report.skipped_generators > 0 {
        notes.push(format!("{} generator(s) and probe(s) were skipped (simulation objects are not supported yet)", report.skipped_generators));
    }
    let unknown: usize = report.unknown_records.iter().map(|u| u.count).sum();
    if unknown > 0 {
        let tags: Vec<String> = report.unknown_records.iter().map(|u| format!("0x{:02X}", u.tag)).collect();
        notes.push(format!("{unknown} record(s) of unknown kinds were skipped (tags {})", tags.join(", ")));
    }
    if !report.embedded_devices.is_empty() {
        notes.push(format!(
            "{} embedded device definition(s) were not converted; components reference them in library {PDSPRJ_LIBRARY}",
            report.embedded_devices.len()
        ));
    }
    report.warnings.extend(notes);
}
