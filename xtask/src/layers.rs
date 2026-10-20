//! `cargo xtask layers`: the crate dependency layering (AGENTS.md "Architecture rules", the
//! table in `docs/specs/2026-10-20-protasena-design.md`).
//!
//! The rule engine works on a small model so it can be unit-tested; [`from_metadata`] builds the
//! model from `cargo metadata --no-deps`.

use std::path::Path;

use serde_json::Value;

/// Where a workspace crate sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// A layered crate: its workspace dependencies must sit in strictly lower layers.
    Layer(u8),
    /// Layer 0 that may depend on no workspace crate at all.
    Standalone,
    /// Applications and build tooling: exempt from the rules.
    Exempt,
}

impl Class {
    fn layer(self) -> Option<u8> {
        match self {
            Class::Layer(l) => Some(l),
            Class::Standalone => Some(0),
            Class::Exempt => None,
        }
    }
}

/// Package name prefix of the library crates (`protasena-geom` lives in `crates/geom`).
pub const PREFIX: &str = "protasena-";

/// The layering table. Names are package names without [`PREFIX`].
pub const TABLE: &[(&str, Class)] = &[
    ("geom", Class::Layer(0)),
    ("kicad", Class::Standalone),
    ("pdsprj", Class::Standalone),
    ("doc", Class::Layer(1)),
    ("library", Class::Layer(1)),
    ("render", Class::Layer(2)),
    ("netlist", Class::Layer(2)),
    ("io", Class::Layer(3)),
    ("engine", Class::Layer(4)),
    ("ui-egui", Class::Layer(5)),
    // apps and tooling
    ("protasena", Class::Exempt),
    ("cli", Class::Exempt),
    ("libconv", Class::Exempt),
    ("xtask", Class::Exempt),
];

/// External crates that are a UI toolkit or windowing dependency. Entries ending in `*` are
/// prefixes.
pub const UI_CRATES: &[&str] = &["egui", "eframe", "egui-wgpu", "egui_kittest", "egui_extras", "wgpu", "winit", "rfd", "bevy*"];

/// The first layer allowed to use [`UI_CRATES`].
pub const UI_MIN_LAYER: u8 = 5;

pub fn short_name(pkg: &str) -> &str {
    pkg.strip_prefix(PREFIX).unwrap_or(pkg)
}

pub fn classify(pkg: &str) -> Option<Class> {
    let s = short_name(pkg);
    TABLE.iter().find(|(n, _)| *n == s).map(|(_, c)| *c)
}

fn is_ui_crate(name: &str) -> bool {
    UI_CRATES.iter().any(|p| match p.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => name == *p,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepKind {
    Normal,
    Dev,
    Build,
}

#[derive(Debug, Clone)]
pub struct Dep {
    pub name: String,
    pub kind: DepKind,
    /// `true` if the dependency is a workspace member (or any path dependency).
    pub workspace: bool,
}

#[derive(Debug, Clone)]
pub struct Crate {
    pub name: String,
    pub deps: Vec<Dep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    Unregistered { krate: String },
    Upward { krate: String, dep: String, from: u8, to: Option<u8>, kind: DepKind },
    StandaloneHasWorkspaceDep { krate: String, dep: String },
    UiBelowMinLayer { krate: String, dep: String, layer: u8 },
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Violation::Unregistered { krate } => {
                write!(f, "{krate}: unknown workspace crate; register it in xtask/src/layers.rs TABLE (and the table in AGENTS.md)")
            }
            Violation::Upward { krate, dep, from, to, kind } => {
                let to = to.map_or_else(|| "app".to_owned(), |l| format!("L{l}"));
                write!(f, "{krate} (L{from}) -> {dep} ({to}) [{kind:?}]: may only depend on strictly lower layers")
            }
            Violation::StandaloneHasWorkspaceDep { krate, dep } => {
                write!(f, "{krate}: standalone crate must not depend on workspace crate {dep}")
            }
            Violation::UiBelowMinLayer { krate, dep, layer } => {
                write!(f, "{krate} (L{layer}) depends on UI crate `{dep}`; UI toolkits are only allowed in L{UI_MIN_LAYER}")
            }
        }
    }
}

/// Check all rules. Returns violations sorted for stable output.
pub fn check(crates: &[Crate]) -> Vec<Violation> {
    let mut out = Vec::new();
    for c in crates {
        let Some(class) = classify(&c.name) else {
            out.push(Violation::Unregistered { krate: c.name.clone() });
            continue;
        };
        let Some(layer) = class.layer() else {
            continue; // exempt
        };
        for d in c.deps.iter().filter(|d| d.name != c.name) {
            if !d.workspace {
                if layer < UI_MIN_LAYER && is_ui_crate(&d.name) {
                    out.push(Violation::UiBelowMinLayer { krate: c.name.clone(), dep: d.name.clone(), layer });
                }
                continue;
            }
            if class == Class::Standalone {
                out.push(Violation::StandaloneHasWorkspaceDep { krate: c.name.clone(), dep: d.name.clone() });
                continue;
            }
            // Unregistered dependencies are reported on their own entry.
            let Some(dep_class) = classify(&d.name) else { continue };
            let to = dep_class.layer();
            if to.is_none_or(|to| to >= layer) {
                out.push(Violation::Upward { krate: c.name.clone(), dep: d.name.clone(), from: layer, to, kind: d.kind });
            }
        }
    }
    out.sort_by_key(|v| v.to_string());
    out.dedup();
    out
}

/// Build the model from `cargo metadata --format-version 1 --no-deps`.
pub fn from_metadata(meta: &Value) -> Result<Vec<Crate>, String> {
    let pkgs = meta.get("packages").and_then(Value::as_array).ok_or("cargo metadata: no packages array")?;
    let members: Vec<&str> = pkgs.iter().filter_map(|p| p.get("name").and_then(Value::as_str)).collect();
    let mut out = Vec::new();
    for p in pkgs {
        let name = p.get("name").and_then(Value::as_str).ok_or("cargo metadata: package without name")?.to_owned();
        let deps = p
            .get("dependencies")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|d| {
                let dname = d.get("name").and_then(Value::as_str).unwrap_or_default().to_owned();
                let kind = match d.get("kind").and_then(Value::as_str) {
                    Some("dev") => DepKind::Dev,
                    Some("build") => DepKind::Build,
                    _ => DepKind::Normal,
                };
                let workspace = members.contains(&dname.as_str()) || d.get("path").is_some_and(Value::is_string);
                Dep { name: dname, kind, workspace }
            })
            .collect();
        out.push(Crate { name, deps });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// `cargo metadata --no-deps` of the workspace at `root`.
pub fn metadata(root: &Path) -> Result<Value, String> {
    let out = crate::cargo(root).args(["metadata", "--format-version", "1", "--no-deps"]).output().map_err(|e| format!("cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err(format!("cargo metadata failed:\n{}", String::from_utf8_lossy(&out.stderr)));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("cargo metadata: bad JSON: {e}"))
}

pub fn describe(class: Option<Class>) -> String {
    match class {
        Some(Class::Layer(l)) => format!("L{l}"),
        Some(Class::Standalone) => "L0 standalone".into(),
        Some(Class::Exempt) => "exempt".into(),
        None => "UNREGISTERED".into(),
    }
}

fn dep_label(d: &Dep) -> String {
    let kind = match d.kind {
        DepKind::Normal => "",
        DepKind::Dev => " (dev)",
        DepKind::Build => " (build)",
    };
    format!("{}{kind}", short_name(&d.name))
}

/// Print the layering of the workspace at `root` and fail on any violation.
pub fn run(root: &Path) -> Result<(), String> {
    let crates = from_metadata(&metadata(root)?)?;
    println!("Dependency layering (AGENTS.md \"Architecture rules\")\n");
    println!("{:<28} {:<14} workspace deps", "crate", "layer");
    for c in &crates {
        let ws: Vec<String> = c.deps.iter().filter(|d| d.workspace).map(dep_label).collect();
        println!("{:<28} {:<14} {}", c.name, describe(classify(&c.name)), ws.join(", "));
    }
    let violations = check(&crates);
    println!();
    if violations.is_empty() {
        println!("OK: {} crates, no layering violations.", crates.len());
        return Ok(());
    }
    println!("{} violation(s):", violations.len());
    for v in &violations {
        println!("  - {v}");
    }
    Err(format!("{} layering violation(s)", violations.len()))
}

#[cfg(test)]
mod tests;
