//! The `.psproj` layout: serde_json's pretty printing (two-space indent), except that an array
//! whose first element is a number, string, bool or null stays on one line, so a point is
//! `[200.0, 0.0]` and a property `["TOLERANCE", "1%"]` — one line per point in a diff.

use std::io::{self, Write};

use serde_json::ser::{CompactFormatter, Formatter};

const INDENT: &[u8] = b"  ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layout {
    /// No element yet.
    Undecided,
    /// Elements on the array's line.
    Inline,
    /// One element per line.
    Block,
}

#[derive(Debug, Clone, Copy)]
enum Frame {
    Array {
        layout: Layout,
        /// `Some(first)` between `begin_array_value` and the start of that value.
        pending: Option<bool>,
    },
    Object {
        has_value: bool,
    },
}

/// The formatter (one per document).
#[derive(Debug, Default)]
pub(crate) struct ProjectFormatter {
    stack: Vec<Frame>,
}

impl ProjectFormatter {
    fn newline_indent<W: ?Sized + Write>(w: &mut W, level: usize) -> io::Result<()> {
        w.write_all(b"\n")?;
        (0..level).try_for_each(|_| w.write_all(INDENT))
    }

    /// Writes the separator before a value that is an element of the enclosing array.
    fn start_value<W: ?Sized + Write>(&mut self, w: &mut W, container: bool) -> io::Result<()> {
        let level = self.stack.len();
        let Some(Frame::Array { layout, pending }) = self.stack.last_mut() else { return Ok(()) };
        let Some(first) = pending.take() else { return Ok(()) };
        if *layout == Layout::Undecided {
            *layout = if container { Layout::Block } else { Layout::Inline };
        }
        match (*layout, first) {
            (Layout::Inline, true) => Ok(()),
            (Layout::Inline, false) => w.write_all(b", "),
            (_, true) => Self::newline_indent(w, level),
            (_, false) => {
                w.write_all(b",")?;
                Self::newline_indent(w, level)
            }
        }
    }
}

/// Scalar writers: separator first, then serde_json's compact form of the value.
macro_rules! scalars {
    ($($name:ident: $ty:ty),* $(,)?) => {
        $(
            fn $name<W: ?Sized + Write>(&mut self, w: &mut W, value: $ty) -> io::Result<()> {
                self.start_value(w, false)?;
                CompactFormatter.$name(w, value)
            }
        )*
    };
}

impl Formatter for ProjectFormatter {
    scalars!(
        write_bool: bool, write_i8: i8, write_i16: i16, write_i32: i32, write_i64: i64, write_i128: i128,
        write_u8: u8, write_u16: u16, write_u32: u32, write_u64: u64, write_u128: u128,
        write_f32: f32, write_f64: f64, write_number_str: &str,
    );

    fn write_null<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.start_value(w, false)?;
        w.write_all(b"null")
    }

    fn begin_string<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        // Object keys arrive here too; `start_value` writes nothing for them.
        self.start_value(w, false)?;
        w.write_all(b"\"")
    }

    fn begin_array<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.start_value(w, true)?;
        self.stack.push(Frame::Array { layout: Layout::Undecided, pending: None });
        w.write_all(b"[")
    }

    fn end_array<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        if let Some(Frame::Array { layout: Layout::Block, .. }) = self.stack.pop() {
            Self::newline_indent(w, self.stack.len())?;
        }
        w.write_all(b"]")
    }

    fn begin_array_value<W: ?Sized + Write>(&mut self, _w: &mut W, first: bool) -> io::Result<()> {
        if let Some(Frame::Array { pending, .. }) = self.stack.last_mut() {
            *pending = Some(first);
        }
        Ok(())
    }

    fn end_array_value<W: ?Sized + Write>(&mut self, _w: &mut W) -> io::Result<()> {
        Ok(())
    }

    fn begin_object<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.start_value(w, true)?;
        self.stack.push(Frame::Object { has_value: false });
        w.write_all(b"{")
    }

    fn end_object<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        if let Some(Frame::Object { has_value: true }) = self.stack.pop() {
            Self::newline_indent(w, self.stack.len())?;
        }
        w.write_all(b"}")
    }

    fn begin_object_key<W: ?Sized + Write>(&mut self, w: &mut W, first: bool) -> io::Result<()> {
        if !first {
            w.write_all(b",")?;
        }
        Self::newline_indent(w, self.stack.len())
    }

    fn begin_object_value<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        w.write_all(b": ")
    }

    fn end_object_value<W: ?Sized + Write>(&mut self, _w: &mut W) -> io::Result<()> {
        if let Some(Frame::Object { has_value }) = self.stack.last_mut() {
            *has_value = true;
        }
        Ok(())
    }
}

/// `value` as `.psproj`-style pretty JSON.
pub(crate) fn to_string<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, String> {
    let mut ser = serde_json::Serializer::with_formatter(Vec::new(), ProjectFormatter::default());
    value.serialize(&mut ser).map_err(|e| e.to_string())?;
    String::from_utf8(ser.into_inner()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn scalar_arrays_stay_on_one_line() {
        let v = json!({"a": [1.5, -2.0], "b": [], "c": {}, "d": [[1, 2], [3, 4]], "e": [{"x": "y"}], "f": ["k", "v", null, true]});
        let expected = r#"{
  "a": [1.5, -2.0],
  "b": [],
  "c": {},
  "d": [
    [1, 2],
    [3, 4]
  ],
  "e": [
    {
      "x": "y"
    }
  ],
  "f": ["k", "v", null, true]
}"#;
        assert_eq!(to_string(&v).unwrap(), expected);
        assert_eq!(serde_json::from_str::<serde_json::Value>(expected).unwrap(), v);
    }

    #[test]
    fn top_level_scalars_and_escapes() {
        assert_eq!(to_string(&json!("a\"b\n")).unwrap(), r#""a\"b\n""#);
        assert_eq!(to_string(&json!([0.1, 1e300, u64::MAX])).unwrap(), "[0.1, 1e+300, 18446744073709551615]");
    }
}
