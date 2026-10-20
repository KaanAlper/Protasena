//! Wires: an exact polyline, or from one end to another (pin ends or points) through optional
//! corners, routed by the wire autorouter when it is on. Junction dots are added where the new
//! wire makes a T.

use protasena_doc::{Item, Junction, Op, Wire, WireId};
use protasena_geom::Pt;
use serde_json::{Value, json};

use crate::args::Args;

/// Most corners of a routed wire.
pub(crate) const MAX_VIA: usize = 64;
use crate::error::Result;
use crate::registry::CommandSpec;
use crate::session::Session;
use crate::wiring::{Anchor, dedup, junctions_for, path, pin_anchor};

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![CommandSpec::new("wire", "Wire", wire).params(
        "{points: [[x, y], …]} exactly, or {from, to: [x, y] | {component: id|reference, pin: number}, via?: [[x, y], …] (at most 64), route?: bool (default: the autorouter setting)}",
    )]
}

/// A wire end from a point or `{component, pin}`.
fn anchor(s: &Session, a: &Args, key: &str) -> Result<Anchor> {
    let v = a.get(key).ok_or_else(|| a.bad(format!("`{key}` is required")))?;
    if let Some(p) = crate::args::point_value(v) {
        return Ok(Anchor::Point(p));
    }
    let pin_args = Args::new(a.cmd(), v).map_err(|_| a.bad(format!("`{key}` must be a point or {{component, pin}}")))?;
    let component = pin_args.component(s, "component")?.ok_or_else(|| a.bad(format!("`{key}.component` is required")))?;
    let pin = pin_args.str_req("pin")?;
    let sheet = s.current_sheet()?;
    pin_anchor(sheet, &s.library, component, pin).ok_or_else(|| a.bad(format!("`{key}`: the component on this sheet has no visible pin {pin:?}")))
}

fn wire(s: &mut Session, a: &Args) -> Result<Value> {
    let points: Vec<Pt> = match a.points("points")? {
        Some(points) => dedup(points),
        None => {
            let from = anchor(s, a, "from")?;
            let to = anchor(s, a, "to")?;
            let via = a.points("via")?.unwrap_or_default();
            if via.len() > MAX_VIA {
                return Err(a.bad(format!("`via` has more than {MAX_VIA} corners")));
            }
            let route = a.bool("route")?.unwrap_or(s.autorouter);
            let sheet = s.current_sheet()?;
            dedup(path(sheet, &s.library, &from, &via, &to, route, s.view.snap.step()))
        }
    };
    if points.len() < 2 {
        return Err(a.bad("a wire needs two or more different points"));
    }
    let dots = junctions_for(s.current_sheet()?, &points);
    let id: WireId = s.fresh()?;
    let sheet = s.sheet;
    let mut ops = vec![Op::Add { sheet, item: Item::Wire(Wire { id, points: points.clone() }), index: None }];
    for at in dots {
        let jid = s.fresh()?;
        ops.push(Op::Add { sheet, item: Item::Junction(Junction { id: jid, at }), index: None });
    }
    s.commit("wire", ops)?;
    let pts: Vec<[f64; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
    Ok(json!({ "id": id.0, "points": pts }))
}
