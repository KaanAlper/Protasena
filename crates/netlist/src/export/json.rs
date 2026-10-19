//! Our JSON netlist: `{"format": "protasena-netlist", "version": 1, "design": …, "nets": […],
//! "unconnected_pins": […], "parts": […], "erc": […]}`.

use serde::{Deserialize, Serialize};

use crate::error::NetlistError;
use crate::model::{ErcItem, Net, Netlist, Node, Part};

/// Format tag of the JSON netlist.
pub const JSON_FORMAT: &str = "protasena-netlist";
/// Version of the JSON netlist.
pub const JSON_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct Document {
    format: String,
    version: u32,
    #[serde(default)]
    design: String,
    #[serde(default)]
    nets: Vec<Net>,
    #[serde(default)]
    unconnected_pins: Vec<Node>,
    #[serde(default)]
    parts: Vec<Part>,
    #[serde(default)]
    erc: Vec<ErcItem>,
}

/// The netlist as pretty-printed JSON, tagged with the design name.
///
/// # Errors
/// [`NetlistError::Json`] if serialization fails (it does not for netlists built here).
pub fn to_json(netlist: &Netlist, design: &str) -> Result<String, NetlistError> {
    let doc = Document {
        format: JSON_FORMAT.to_owned(),
        version: JSON_VERSION,
        design: design.to_owned(),
        nets: netlist.nets.clone(),
        unconnected_pins: netlist.unconnected_pins.clone(),
        parts: netlist.parts.clone(),
        erc: netlist.erc.clone(),
    };
    serde_json::to_string_pretty(&doc).map_err(|e| NetlistError::Json(e.to_string()))
}

/// Reads a JSON netlist back; returns it with the design name.
///
/// # Errors
/// [`NetlistError::Json`] for malformed JSON, [`NetlistError::Format`] for another format tag
/// or a newer version.
pub fn from_json(text: &str) -> Result<(Netlist, String), NetlistError> {
    let doc: Document = serde_json::from_str(text).map_err(|e| NetlistError::Json(e.to_string()))?;
    if doc.format != JSON_FORMAT {
        return Err(NetlistError::Format(format!("format is \"{}\", not \"{JSON_FORMAT}\"", doc.format)));
    }
    if doc.version > JSON_VERSION {
        return Err(NetlistError::Format(format!("version {} is newer than {JSON_VERSION}", doc.version)));
    }
    Ok((Netlist { nets: doc.nets, unconnected_pins: doc.unconnected_pins, parts: doc.parts, erc: doc.erc }, doc.design))
}
