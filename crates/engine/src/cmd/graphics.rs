//! 2D graphics: lines, boxes, circles, arcs and closed paths (the 2D Graphics modes of the Mode
//! toolbar). Each command adds one graphic in one undo step; moving, turning, copying and
//! deleting them are the Edit and Block commands every item has.

use protasena_doc::{Graphic, GraphicShape, Item, MAX_PATH_POINTS, Op};
use protasena_geom::arc_through;
use serde_json::{Value, json};

use crate::args::{Args, MAX_COORD};
use crate::error::Result;
use crate::mode::Mode;
use crate::registry::CommandSpec;
use crate::session::Session;
use crate::wiring::dedup;

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("line", "2D Line", line).params("{from: [x, y], to: [x, y]}").machine(Mode::Line2d),
        CommandSpec::new("box", "2D Box", box_).params("{from: [x, y], to: [x, y]} — opposite corners").machine(Mode::Box2d),
        CommandSpec::new("circle", "2D Circle", circle).params("{center: [x, y], radius: thou}").machine(Mode::Circle2d),
        CommandSpec::new("arc", "2D Arc", arc)
            .params("{start: [x, y], mid: [x, y], end: [x, y]} — mid: any point on the arc between the ends")
            .machine(Mode::Arc2d),
        CommandSpec::new("closed-path", "2D Closed Path", closed_path)
            .params("{points: [[x, y], …]} — three or more corners")
            .machine(Mode::ClosedPath2d),
    ]
}

fn add(s: &mut Session, label: &str, shape: GraphicShape) -> Result<Value> {
    shape.check()?;
    let id = s.fresh()?;
    let sheet = s.sheet;
    s.commit(label, vec![Op::Add { sheet, item: Item::Graphic(Graphic { id, shape }), index: None }])?;
    Ok(json!({ "id": id.0 }))
}

fn line(s: &mut Session, a: &Args) -> Result<Value> {
    let (from, to) = (a.point_req("from")?, a.point_req("to")?);
    if from == to {
        return Err(a.bad("a line needs two different ends"));
    }
    add(s, "line", GraphicShape::Line { a: from, b: to })
}

fn box_(s: &mut Session, a: &Args) -> Result<Value> {
    let (from, to) = (a.point_req("from")?, a.point_req("to")?);
    if from.x == to.x || from.y == to.y {
        return Err(a.bad("a box needs a width and a height"));
    }
    add(s, "box", GraphicShape::Box { a: from, b: to })
}

fn circle(s: &mut Session, a: &Args) -> Result<Value> {
    let center = a.point_req("center")?;
    let radius = a.f64("radius")?.ok_or_else(|| a.bad("`radius` (thou) is required"))?;
    if !(radius > 0.0 && radius <= MAX_COORD) {
        return Err(a.bad("`radius` must be above 0"));
    }
    add(s, "circle", GraphicShape::Circle { center, radius })
}

fn arc(s: &mut Session, a: &Args) -> Result<Value> {
    let (start, mid, end) = (a.point_req("start")?, a.point_req("mid")?, a.point_req("end")?);
    if start == end || arc_through(start, mid, end).is_none() {
        return Err(a.bad("an arc needs three points that are not on one line"));
    }
    add(s, "arc", GraphicShape::Arc { start, mid, end })
}

fn closed_path(s: &mut Session, a: &Args) -> Result<Value> {
    let mut points = dedup(a.points("points")?.ok_or_else(|| a.bad("`points` is required"))?);
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    if points.len() < 3 || points.len() > MAX_PATH_POINTS {
        return Err(a.bad(format!("a closed path needs 3 to {MAX_PATH_POINTS} different corners")));
    }
    add(s, "closed-path", GraphicShape::Path { points })
}
