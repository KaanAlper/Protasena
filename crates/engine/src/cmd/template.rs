//! Template menu: Set Design Colours, Set Graphic Styles, Set Text Styles, Set 2D Graphics
//! Defaults and Set Junction Dot Style. They change the design's template settings, one undo
//! step each; the parameters are a patch of the group (see [`crate::patch`]), so `{}` changes
//! nothing and `{"reset": true}` puts Protasena's defaults back.

use protasena_doc::{Op, Template};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::args::Args;
use crate::error::Result;
use crate::patch;
use crate::registry::CommandSpec;
use crate::request::Request;
use crate::session::Session;

const COLOURS: &str = "{paper?, grid?, highlight?, drag?: \"#rrggbb\"|null, reset?: bool} — null: our default";
const STYLE: &str = "{width?: thou|null, colour?: \"#rrggbb\"|null, fill?: \"#rrggbb\"|null}";

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("set-design-colours", "Set Design Colours", |s, a| set(s, a, "set-design-colours", |t| &mut t.colours))
            .menu(&["Template", "Set Design Colours"])
            .params(COLOURS)
            .dialog(|_| Some(Request::Template { section: "colours" })),
        CommandSpec::new("set-graphic-styles", "Set Graphic Styles", |s, a| set(s, a, "set-graphic-styles", |t| &mut t.graphic_styles))
            .menu(&["Template", "Set Graphic Styles"])
            .params("{component?, pin?, wire?, bus?, terminal?, subcircuit?: {width?, colour?, fill?}, reset?} — each a style; null: our default")
            .dialog(|_| Some(Request::Template { section: "graphic_styles" })),
        CommandSpec::new("set-text-styles", "Set Text Styles", |s, a| set(s, a, "set-text-styles", |t| &mut t.text_styles))
            .menu(&["Template", "Set Text Styles"])
            .params("{reference?, value?, pin_name?, pin_number?, label?, terminal?, text?, subcircuit?: {height?: thou|null, colour?: \"#rrggbb\"|null}, reset?}")
            .dialog(|_| Some(Request::Template { section: "text_styles" })),
        CommandSpec::new("set-2d-graphics-defaults", "Set 2D Graphics Defaults", |s, a| set(s, a, "set-2d-graphics-defaults", |t| &mut t.graphics_2d))
            .menu(&["Template", "Set 2D Graphics Defaults"])
            .params(STYLE)
            .dialog(|_| Some(Request::Template { section: "graphics_2d" })),
        CommandSpec::new("set-junction-style", "Set Junction Dot Style", |s, a| set(s, a, "set-junction-style", |t| &mut t.junction))
            .menu(&["Template", "Set Junction Dot Style"])
            .params("{size?: thou|null (the dot's diameter), shape?: \"circle\"|\"square\"|\"diamond\", reset?}")
            .dialog(|_| Some(Request::Template { section: "junction" })),
    ]
}

/// Applies the patch in `a` to one group of the template, as one undo step.
fn set<T>(s: &mut Session, a: &Args, label: &str, group: fn(&mut Template) -> &mut T) -> Result<Value>
where
    T: Serialize + DeserializeOwned + Default + PartialEq + Clone,
{
    let mut template = s.project.template;
    let current = group(&mut template).clone();
    let params = a.raw();
    let new = patch::apply(&current, &params, a, &[])?;
    let changed = new != current;
    let shown = serde_json::to_value(&new).unwrap_or(Value::Null);
    if changed {
        *group(&mut template) = new;
        template.check().map_err(|e| a.bad(e.to_string()))?;
        s.commit(label, vec![Op::SetTemplate { template: Box::new(template) }])?;
    }
    Ok(json!({ "changed": changed, "settings": shown }))
}
