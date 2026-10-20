//! KiCad board layer names → Protasena layer names.
//!
//! Protasena names a layer `<kind>.<side>`: `copper.top`, `copper.bottom`, `copper.in1`,
//! `copper.*` (every copper layer), `mask.*`, `paste.top`, `silk.top`, `fab.top`,
//! `courtyard.top`, `adhesive.top`; plus `edge`, `margin`, `user.drawings`, `user.comments`,
//! `user.eco1`, `user.eco2` and `user.N`. A name we do not know is kept as `kicad.<name>`.

/// The Protasena name of a KiCad layer.
pub fn map(kicad: &str) -> String {
    match kicad {
        "Edge.Cuts" => return "edge".to_owned(),
        "Margin" => return "margin".to_owned(),
        "Dwgs.User" => return "user.drawings".to_owned(),
        "Cmts.User" => return "user.comments".to_owned(),
        "Eco1.User" => return "user.eco1".to_owned(),
        "Eco2.User" => return "user.eco2".to_owned(),
        _ => {}
    }
    if let Some(n) = kicad.strip_prefix("User.").filter(|n| is_number(n)) {
        return format!("user.{n}");
    }
    let Some((side, kind)) = kicad.split_once('.') else { return format!("kicad.{kicad}") };
    let kind = match kind {
        "Cu" => "copper",
        "Mask" => "mask",
        "Paste" => "paste",
        "SilkS" => "silk",
        "Fab" => "fab",
        "CrtYd" => "courtyard",
        "Adhes" => "adhesive",
        _ => return format!("kicad.{kicad}"),
    };
    let side = match side {
        "F" => "top".to_owned(),
        "B" => "bottom".to_owned(),
        "*" | "F&B" => "*".to_owned(),
        inner => match inner.strip_prefix("In").filter(|n| is_number(n)) {
            Some(n) if kind == "copper" => format!("in{n}"),
            _ => return format!("kicad.{kicad}"),
        },
    };
    format!("{kind}.{side}")
}

/// Whether a mapped layer is copper.
pub fn is_copper(layer: &str) -> bool {
    layer.starts_with("copper.")
}

fn is_number(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::map;

    #[test]
    fn maps_the_layers_kicad_libraries_use() {
        for (k, p) in [
            ("F.Cu", "copper.top"),
            ("B.Cu", "copper.bottom"),
            ("*.Cu", "copper.*"),
            ("F&B.Cu", "copper.*"),
            ("In12.Cu", "copper.in12"),
            ("*.Mask", "mask.*"),
            ("F.Paste", "paste.top"),
            ("F.SilkS", "silk.top"),
            ("B.Fab", "fab.bottom"),
            ("F.CrtYd", "courtyard.top"),
            ("F.Adhes", "adhesive.top"),
            ("Edge.Cuts", "edge"),
            ("Dwgs.User", "user.drawings"),
            ("Cmts.User", "user.comments"),
            ("User.2", "user.2"),
            ("Margin", "margin"),
        ] {
            assert_eq!(map(k), p, "{k}");
        }
        assert_eq!(map("In1.Mask"), "kicad.In1.Mask");
        assert_eq!(map("Weird"), "kicad.Weird");
        assert_eq!(map("X.Cu"), "kicad.X.Cu");
    }
}
