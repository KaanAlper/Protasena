//! Changing a settings group from a command's JSON form: the parameters are a patch of the
//! group's JSON (`{"paper": "#ffffff"}`). A key left out keeps its value, `null` puts back the
//! default, objects merge key by key, and `"reset": true` starts from the defaults. Unknown keys
//! and values of the wrong type are refused.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::args::Args;
use crate::error::Result;

/// Deepest nesting of a patch followed.
const MAX_DEPTH: usize = 8;

fn merge(
    cur: &mut Map<String, Value>,
    def: &Map<String, Value>,
    patch: &Map<String, Value>,
    path: &str,
    depth: usize,
) -> std::result::Result<(), String> {
    if depth > MAX_DEPTH {
        return Err(format!("`{path}` is nested too deeply"));
    }
    for (k, v) in patch {
        let here = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
        let (Some(slot), Some(d)) = (cur.get_mut(k), def.get(k)) else { return Err(format!("there is no setting `{here}`")) };
        match (v, slot, d) {
            (Value::Null, slot, d) => *slot = d.clone(),
            (Value::Object(p), Value::Object(c), Value::Object(d)) => merge(c, d, p, &here, depth + 1)?,
            (Value::Object(_), _, _) => return Err(format!("`{here}` takes a value, not an object")),
            (v, slot, _) => *slot = v.clone(),
        }
    }
    Ok(())
}

/// `current` with the patch in `a` applied (see the module docs). `skip` names parameters that
/// are not part of the group.
///
/// # Errors
/// [`crate::EngineError::BadParams`] for unknown settings and values that do not read.
pub fn apply<T: Serialize + DeserializeOwned + Default>(current: &T, params: &Value, a: &Args, skip: &[&str]) -> Result<T> {
    let to_json = |t: &T| serde_json::to_value(t).map_err(|e| a.bad(e.to_string()));
    let def = to_json(&T::default())?;
    let reset = a.bool("reset")?.unwrap_or(false);
    let mut cur = if reset { def.clone() } else { to_json(current)? };
    if let (Some(patch), Some(c), Some(d)) = (params.as_object(), cur.as_object_mut(), def.as_object()) {
        let patch: Map<String, Value> =
            patch.iter().filter(|(k, _)| k.as_str() != "reset" && !skip.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
        merge(c, d, &patch, "", 0).map_err(|e| a.bad(e))?;
    }
    T::deserialize(&cur).map_err(|e| a.bad(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use protasena_doc::{Colour, GraphicStyles};
    use serde_json::json;

    #[test]
    fn patches_merge_reset_and_refuse_unknown_keys() {
        let mut styles = GraphicStyles::default();
        styles.wire.width = Some(10.0);
        let p = json!({ "bus": { "colour": "#102030" }, "pin": { "width": 5 } });
        let a = Args::new("t", &p).unwrap();
        let out = apply(&styles, &p, &a, &[]).unwrap();
        assert_eq!((out.wire.width, out.bus.colour, out.pin.width), (Some(10.0), Colour::parse("#102030"), Some(5.0)));
        let p = json!({ "wire": null });
        let out = apply(&out, &p, &Args::new("t", &p).unwrap(), &[]).unwrap();
        assert_eq!(out.wire, Default::default());
        let p = json!({ "reset": true, "pin": { "width": 3 } });
        let out = apply(&styles, &p, &Args::new("t", &p).unwrap(), &[]).unwrap();
        assert_eq!((out.wire.width, out.pin.width), (None, Some(3.0)));
        for bad in [json!({ "wires": {} }), json!({ "wire": { "colour": "blue" } }), json!({ "wire": 5 }), json!({ "wire": { "width": "x" } })] {
            assert!(apply(&styles, &bad, &Args::new("t", &bad).unwrap(), &[]).is_err(), "{bad}");
        }
    }
}
