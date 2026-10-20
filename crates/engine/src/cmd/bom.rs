//! The Global Toolbar's Bill of Materials: the design's parts grouped by value, device and
//! package (see `protasena_netlist::bill_of_materials`), as JSON, or written as CSV or HTML.

use protasena_netlist::{bill_of_materials, export};
use serde_json::{Value, json};

use super::file::path_param;
use crate::args::Args;
use crate::error::{EngineError, Result};
use crate::registry::CommandSpec;
use crate::request::Request;
use crate::session::Session;

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("bill-of-materials", "Bill of Materials", bom)
            .params("{format?: \"json\"|\"csv\"|\"html\", path?} — without a path the report comes back (csv and html as `text` too); with one it is written, in the format of its extension by default")
            .dialog(|_| Some(Request::BillOfMaterials)),
    ]
}

/// The format asked for, else the path's extension's, else JSON.
fn format(a: &Args, path: Option<&std::path::Path>) -> Result<&'static str> {
    let by_ext = path.and_then(|p| p.extension()).map(|e| e.to_string_lossy().to_ascii_lowercase());
    let name = match a.str("format")? {
        Some(f) => f.to_ascii_lowercase(),
        None => by_ext.unwrap_or_else(|| "json".to_owned()),
    };
    match name.as_str() {
        "json" => Ok("json"),
        "csv" => Ok("csv"),
        "html" | "htm" => Ok("html"),
        other => Err(a.bad(format!("unknown bill of materials format {other:?} (json, csv or html)"))),
    }
}

fn bom(s: &mut Session, a: &Args) -> Result<Value> {
    let path = path_param(a, "path")?;
    let format = format(a, path.as_deref())?;
    let bom = bill_of_materials(&s.project, &s.library);
    let text = match format {
        "csv" => export::bom_to_csv(&bom),
        "html" => export::bom_to_html(&bom),
        _ => serde_json::to_string_pretty(&bom).map_err(|e| EngineError::Other(e.to_string()))? + "\n",
    };
    let mut out = json!({ "design": bom.design, "total": bom.total(), "lines": bom.lines, "format": format });
    let Some(o) = out.as_object_mut() else { return Ok(out) };
    match path {
        Some(path) => {
            protasena_io::atomic_write(&path, false, |w| w.write_all(text.as_bytes()))
                .map_err(|e| EngineError::Export(protasena_io::ExportError::Io { path: path.clone(), message: e.to_string() }))?;
            s.note(format!("Bill of materials written to {}", path.display()));
            o.insert("path".into(), json!(path.display().to_string()));
        }
        None if format != "json" => {
            o.insert("text".into(), json!(text));
        }
        None => {}
    }
    Ok(out)
}
