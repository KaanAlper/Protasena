//! Reading a command's JSON parameters. Parameters are hostile: every getter checks the type,
//! finiteness and size, and names the parameter in its error.

use std::collections::HashMap;

use protasena_doc::{ComponentId, ItemId, Sheet, SheetId};
use protasena_geom::{Orient, Pt, Vec2};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::{EngineError, Result};
use crate::session::Session;

/// Largest coordinate accepted, in thou (about 2.5 km: far beyond any sheet).
pub const MAX_COORD: f64 = 1e8;

/// Most points in one point list.
pub const MAX_POINTS: usize = 10_000;

/// Most ids in one id list.
pub const MAX_IDS: usize = 100_000;

/// Longest accepted string, in bytes.
pub const MAX_TEXT: usize = 64 * 1024;

/// A command's parameters with the command id for error messages.
pub struct Args<'a> {
    cmd: &'static str,
    map: Option<&'a Map<String, Value>>,
}

impl<'a> Args<'a> {
    /// Parameters of `cmd`: a JSON object, or `null` for none.
    ///
    /// # Errors
    /// [`EngineError::BadParams`] for any other JSON value.
    pub fn new(cmd: &'static str, v: &'a Value) -> Result<Self> {
        match v {
            Value::Null => Ok(Self { cmd, map: None }),
            Value::Object(m) => Ok(Self { cmd, map: Some(m) }),
            _ => Err(EngineError::BadParams { cmd: cmd.into(), msg: "parameters must be a JSON object".into() }),
        }
    }

    /// The command id.
    pub fn cmd(&self) -> &'static str {
        self.cmd
    }

    /// An error about the parameters.
    pub fn bad(&self, msg: impl Into<String>) -> EngineError {
        EngineError::BadParams { cmd: self.cmd.into(), msg: msg.into() }
    }

    /// The raw value of a parameter (`null` counts as absent).
    pub fn get(&self, key: &str) -> Option<&'a Value> {
        self.map.and_then(|m| m.get(key)).filter(|v| !v.is_null())
    }

    /// Whether a parameter is given as `null` (to clear a value).
    pub fn is_null(&self, key: &str) -> bool {
        self.map.and_then(|m| m.get(key)).is_some_and(Value::is_null)
    }

    /// Whether a parameter is given.
    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// An optional finite number.
    pub fn f64(&self, key: &str) -> Result<Option<f64>> {
        match self.get(key) {
            None => Ok(None),
            Some(v) => {
                v.as_f64().filter(|x| x.is_finite() && x.abs() <= MAX_COORD).map(Some).ok_or_else(|| self.bad(format!("`{key}` must be a number")))
            }
        }
    }

    /// An optional whole number ≥ 0.
    pub fn u64(&self, key: &str) -> Result<Option<u64>> {
        match self.get(key) {
            None => Ok(None),
            Some(v) => v.as_u64().map(Some).ok_or_else(|| self.bad(format!("`{key}` must be a whole number ≥ 0"))),
        }
    }

    /// An optional boolean.
    pub fn bool(&self, key: &str) -> Result<Option<bool>> {
        match self.get(key) {
            None => Ok(None),
            Some(v) => v.as_bool().map(Some).ok_or_else(|| self.bad(format!("`{key}` must be true or false"))),
        }
    }

    /// An optional string of at most [`MAX_TEXT`] bytes.
    pub fn str(&self, key: &str) -> Result<Option<&'a str>> {
        match self.get(key) {
            None => Ok(None),
            Some(Value::String(s)) if s.len() <= MAX_TEXT => Ok(Some(s.as_str())),
            Some(Value::String(_)) => Err(self.bad(format!("`{key}` is longer than {MAX_TEXT} bytes"))),
            Some(_) => Err(self.bad(format!("`{key}` must be a string"))),
        }
    }

    /// A required string.
    pub fn str_req(&self, key: &str) -> Result<&'a str> {
        self.str(key)?.ok_or_else(|| self.bad(format!("`{key}` (string) is required")))
    }

    /// An optional point `[x, y]` or `{"x": …, "y": …}`.
    pub fn point(&self, key: &str) -> Result<Option<Pt>> {
        match self.get(key) {
            None => Ok(None),
            Some(v) => point_value(v).map(Some).ok_or_else(|| self.bad(format!("`{key}` must be a point [x, y] of finite numbers"))),
        }
    }

    /// A required point.
    pub fn point_req(&self, key: &str) -> Result<Pt> {
        self.point(key)?.ok_or_else(|| self.bad(format!("`{key}` (point [x, y]) is required")))
    }

    /// An optional list of points (at most [`MAX_POINTS`]).
    pub fn points(&self, key: &str) -> Result<Option<Vec<Pt>>> {
        let Some(v) = self.get(key) else { return Ok(None) };
        let list = v.as_array().ok_or_else(|| self.bad(format!("`{key}` must be a list of points")))?;
        if list.len() > MAX_POINTS {
            return Err(self.bad(format!("`{key}` has more than {MAX_POINTS} points")));
        }
        list.iter().map(|p| point_value(p).ok_or_else(|| self.bad(format!("`{key}`: {p} is not a point [x, y]")))).collect::<Result<_>>().map(Some)
    }

    /// An optional offset `[dx, dy]`.
    pub fn vec(&self, key: &str) -> Result<Option<Vec2>> {
        Ok(self.point(key)?.map(Pt::to_vec))
    }

    /// An optional orientation: a number of degrees (a multiple of 90), or
    /// `{"angle": 90, "mirror": true}`.
    pub fn orient(&self, key: &str) -> Result<Option<Orient>> {
        let Some(v) = self.get(key) else { return Ok(None) };
        let (angle, mirror) = match v {
            Value::Number(_) => (whole(v), Some(false)),
            Value::Object(o) => (o.get("angle").map_or(Some(0), whole), o.get("mirror").map_or(Some(false), Value::as_bool)),
            _ => (None, None),
        };
        let err = || self.bad(format!("`{key}` must be an angle in degrees (a multiple of 90) or {{\"angle\": …, \"mirror\": …}}"));
        let (Some(angle), Some(mirror)) = (angle, mirror) else { return Err(err()) };
        let angle = i32::try_from(angle.rem_euclid(360)).map_err(|_| err())?;
        Orient::from_angle(angle, mirror).map(Some).map_err(|_| err())
    }

    /// Item ids from `key` (raw numbers or `{"component": 5}` forms), or `None` when absent.
    pub fn ids(&self, s: &Session, key: &str) -> Result<Option<Vec<ItemId>>> {
        let Some(v) = self.get(key) else { return Ok(None) };
        let list = v.as_array().ok_or_else(|| self.bad(format!("`{key}` must be a list of item ids")))?;
        if list.len() > MAX_IDS {
            return Err(self.bad(format!("`{key}` has more than {MAX_IDS} ids")));
        }
        let index = id_index(s);
        list.iter()
            .map(|v| lookup(&index, v).ok_or_else(|| self.bad(format!("`{key}`: {v} is not an item of this project"))))
            .collect::<Result<_>>()
            .map(Some)
    }

    /// One item id from `key`, or `None` when absent.
    pub fn item(&self, s: &Session, key: &str) -> Result<Option<ItemId>> {
        let Some(v) = self.get(key) else { return Ok(None) };
        lookup(&id_index(s), v).map(Some).ok_or_else(|| self.bad(format!("`{key}`: {v} is not an item of this project")))
    }

    /// The items to act on: `ids` when given, else the selection.
    pub fn targets(&self, s: &Session) -> Result<Vec<ItemId>> {
        Ok(self.ids(s, "ids")?.unwrap_or_else(|| s.selection.clone()))
    }

    /// A component named by `key`: a raw id or a reference (`"R1"`, current sheet first).
    pub fn component(&self, s: &Session, key: &str) -> Result<Option<ComponentId>> {
        let Some(v) = self.get(key) else { return Ok(None) };
        let found = match v {
            Value::String(r) => {
                let on_sheet = s.sheet().and_then(|sh| sh.components.iter().find(|c| c.reference == *r).map(|c| c.id));
                on_sheet.or_else(|| s.project.components().find(|(_, c)| c.reference == *r).map(|(_, c)| c.id))
            }
            _ => match lookup(&id_index(s), v) {
                Some(ItemId::Component(id)) => Some(id),
                _ => None,
            },
        };
        found.map(Some).ok_or_else(|| self.bad(format!("`{key}`: {v} is not a component of this project")))
    }

    /// A sheet named by `key`: its id, its name, or `{"index": n}` (0-based).
    pub fn sheet(&self, s: &Session, key: &str) -> Result<Option<SheetId>> {
        let Some(v) = self.get(key) else { return Ok(None) };
        let sheets = &s.project.sheets;
        let found = match v {
            Value::String(name) => sheets.iter().find(|sh| sh.name == *name).map(|sh| sh.id),
            Value::Object(o) => {
                o.get("index").and_then(Value::as_u64).and_then(|i| usize::try_from(i).ok()).and_then(|i| sheets.get(i)).map(|sh| sh.id)
            }
            _ => v.as_u64().and_then(|raw| sheets.iter().find(|sh| sh.id.0 == raw)).map(|sh: &Sheet| sh.id),
        };
        found.map(Some).ok_or_else(|| self.bad(format!("`{key}`: {v} is not a sheet of this project")))
    }
}

/// A whole number given as an integer or as a float without a fraction (`90` or `90.0`).
fn whole(v: &Value) -> Option<i64> {
    v.as_i64().or_else(|| v.as_f64().filter(|f| f.is_finite() && f.fract() == 0.0 && f.abs() <= 1e9).map(|f| f as i64))
}

/// A point from `[x, y]` or `{"x": …, "y": …}`, finite and within [`MAX_COORD`].
pub fn point_value(v: &Value) -> Option<Pt> {
    let (x, y) = match v {
        Value::Array(a) if a.len() == 2 => (a.first()?.as_f64()?, a.get(1)?.as_f64()?),
        Value::Object(o) => (o.get("x")?.as_f64()?, o.get("y")?.as_f64()?),
        _ => return None,
    };
    let ok = |c: f64| c.is_finite() && c.abs() <= MAX_COORD;
    (ok(x) && ok(y)).then_some(Pt::new(x, y))
}

/// Every item of the project by raw id.
fn id_index(s: &Session) -> HashMap<u64, ItemId> {
    s.project.sheets.iter().flat_map(|sh| sh.item_ids()).map(|id| (id.raw(), id)).collect()
}

/// An item id from a raw number or its serde form (`{"component": 5}`), if the project has it.
fn lookup(index: &HashMap<u64, ItemId>, v: &Value) -> Option<ItemId> {
    match v.as_u64() {
        Some(raw) => index.get(&raw).copied(),
        None => ItemId::deserialize(v).ok().filter(|id| index.get(&id.raw()) == Some(id)),
    }
}

/// The JSON form of an item id: its raw number.
pub fn id_json(id: ItemId) -> Value {
    Value::from(id.raw())
}
