//! A design's template settings (Proteus's Template menu): design colours, graphic styles,
//! text styles, 2D graphics defaults and the junction dot style. They are saved with the design.
//!
//! Every value is optional: `None` means "Protasena's own default" (the theme's colour, the
//! built-in width or height), so a design without template settings looks as Protasena draws
//! it, and Proteus's defaults are never copied.

use std::fmt;

use crate::error::DocError;

/// Widest line a style may give, in thou.
pub const MAX_STYLE_WIDTH: f64 = 1000.0;
/// Tallest text a style may give, in thou.
pub const MAX_STYLE_HEIGHT: f64 = 10_000.0;
/// Largest junction dot, in thou (diameter).
pub const MAX_JUNCTION_SIZE: f64 = 1000.0;

/// An RGB colour, written `#rrggbb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Colour(pub [u8; 3]);

impl Colour {
    /// Parses `#rrggbb` or `rrggbb` (hex digits in any case).
    pub fn parse(s: &str) -> Option<Colour> {
        let hex = s.trim().strip_prefix('#').unwrap_or(s.trim());
        if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let byte = |i: usize| hex.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok());
        Some(Colour([byte(0)?, byte(2)?, byte(4)?]))
    }
}

impl fmt::Display for Colour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [r, g, b] = self.0;
        write!(f, "#{r:02x}{g:02x}{b:02x}")
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Colour {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Colour {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = <std::borrow::Cow<'de, str>>::deserialize(d)?;
        Colour::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("{s:?} is not a colour #rrggbb")))
    }
}

/// Template ▸ Set Design Colours: the colours of the editing window itself.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct DesignColours {
    /// The sheet (paper).
    pub paper: Option<Colour>,
    /// Grid dots and lines.
    pub grid: Option<Colour>,
    /// Selected (tagged) objects.
    pub highlight: Option<Colour>,
    /// Objects being dragged or placed.
    pub drag: Option<Colour>,
}

/// One graphic style: line width, line colour and (for closed shapes) fill colour.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct GraphicStyle {
    /// Line width in thou.
    pub width: Option<f64>,
    /// Line colour.
    pub colour: Option<Colour>,
    /// Fill colour.
    pub fill: Option<Colour>,
}

/// Template ▸ Set Graphic Styles: the styles objects are drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct GraphicStyles {
    /// Component bodies (fill: the body fill).
    pub component: GraphicStyle,
    /// Pins.
    pub pin: GraphicStyle,
    /// Wires and junction dots.
    pub wire: GraphicStyle,
    /// Buses and bus entries.
    pub bus: GraphicStyle,
    /// Terminals.
    pub terminal: GraphicStyle,
    /// Subcircuit boxes and ports (fill: the box).
    pub subcircuit: GraphicStyle,
}

/// One text style: height and colour.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct TextStyle {
    /// Character height in thou.
    pub height: Option<f64>,
    /// Colour.
    pub colour: Option<Colour>,
}

/// Template ▸ Set Text Styles: the styles texts are drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct TextStyles {
    /// Part references.
    pub reference: TextStyle,
    /// Part values.
    pub value: TextStyle,
    /// Pin names.
    pub pin_name: TextStyle,
    /// Pin numbers.
    pub pin_number: TextStyle,
    /// Wire labels.
    pub label: TextStyle,
    /// Terminal names.
    pub terminal: TextStyle,
    /// Free texts and scripts without a height of their own.
    pub text: TextStyle,
    /// Subcircuit and port names.
    pub subcircuit: TextStyle,
}

/// Template ▸ Set 2D Graphics Defaults: how 2D graphics are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct Graphics2d {
    /// Line width in thou.
    pub width: Option<f64>,
    /// Line colour.
    pub colour: Option<Colour>,
    /// Fill colour of boxes, circles and closed paths (none: not filled).
    pub fill: Option<Colour>,
}

/// The shape of junction dots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "snake_case"))]
pub enum JunctionShape {
    /// A round dot.
    #[default]
    Circle,
    /// A square.
    Square,
    /// A diamond (a square on its corner).
    Diamond,
}

/// Template ▸ Set Junction Dot Style.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct JunctionStyle {
    /// Diameter (or side) in thou.
    pub size: Option<f64>,
    /// Shape.
    pub shape: JunctionShape,
}

/// The template settings of a design.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct Template {
    /// Design colours.
    pub colours: DesignColours,
    /// Graphic styles.
    pub graphic_styles: GraphicStyles,
    /// Text styles.
    pub text_styles: TextStyles,
    /// 2D graphics defaults.
    pub graphics_2d: Graphics2d,
    /// Junction dot style.
    pub junction: JunctionStyle,
}

fn in_range(v: Option<f64>, max: f64, what: &'static str) -> Result<(), DocError> {
    match v {
        None => Ok(()),
        Some(x) if !x.is_finite() => Err(DocError::NonFinite(what)),
        Some(x) if x > 0.0 && x <= max => Ok(()),
        Some(x) => Err(DocError::Invalid(format!("{what} {x} is not above 0 and at most {max}"))),
    }
}

impl Template {
    /// Whether every setting is Protasena's default.
    pub fn is_default(&self) -> bool {
        *self == Template::default()
    }

    /// Checks widths, heights and the junction size: finite, above 0 and at most
    /// [`MAX_STYLE_WIDTH`], [`MAX_STYLE_HEIGHT`] and [`MAX_JUNCTION_SIZE`].
    ///
    /// # Errors
    /// [`DocError::NonFinite`] or [`DocError::Invalid`] naming the setting.
    pub fn check(&self) -> Result<(), DocError> {
        let g = &self.graphic_styles;
        for (s, what) in [
            (g.component, "component line width"),
            (g.pin, "pin line width"),
            (g.wire, "wire width"),
            (g.bus, "bus width"),
            (g.terminal, "terminal line width"),
            (g.subcircuit, "subcircuit line width"),
        ] {
            in_range(s.width, MAX_STYLE_WIDTH, what)?;
        }
        let t = &self.text_styles;
        for s in [t.reference, t.value, t.pin_name, t.pin_number, t.label, t.terminal, t.text, t.subcircuit] {
            in_range(s.height, MAX_STYLE_HEIGHT, "text height")?;
        }
        in_range(self.graphics_2d.width, MAX_STYLE_WIDTH, "2D graphics line width")?;
        in_range(self.junction.size, MAX_JUNCTION_SIZE, "junction dot size")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_parse_and_print_as_hex() {
        assert_eq!(Colour::parse("#1a2B3c"), Some(Colour([0x1a, 0x2b, 0x3c])));
        assert_eq!(Colour::parse(" 000000 "), Some(Colour([0, 0, 0])));
        for bad in ["", "#12345", "#1234567", "#gg0000", "#ééé", "red"] {
            assert_eq!(Colour::parse(bad), None, "{bad}");
        }
        assert_eq!(Colour([255, 0, 16]).to_string(), "#ff0010");
    }

    #[test]
    fn checks_sizes_and_widths() {
        let mut t = Template::default();
        assert!(t.is_default() && t.check().is_ok());
        t.junction.size = Some(40.0);
        t.text_styles.label.height = Some(120.0);
        assert!(t.check().is_ok() && !t.is_default());
        t.graphic_styles.wire.width = Some(0.0);
        assert!(matches!(t.check(), Err(DocError::Invalid(_))));
        t.graphic_styles.wire.width = Some(f64::NAN);
        assert_eq!(t.check(), Err(DocError::NonFinite("wire width")));
        t.graphic_styles.wire.width = None;
        t.junction.size = Some(1e9);
        assert!(t.check().is_err());
    }
}
