//! Tree → text. Iterative, so even a hand-built tree deeper than any limit cannot overflow
//! the stack.

use std::fmt;

use super::Sexpr;

impl fmt::Display for Sexpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Each frame: the remaining items of an open list.
        let mut stack: Vec<std::slice::Iter<'_, Sexpr>> = Vec::new();
        let mut first = true;
        let mut next = Some(self);
        loop {
            if let Some(node) = next.take() {
                if !first {
                    f.write_str(" ")?;
                }
                first = false;
                match node {
                    Sexpr::List { items, .. } => {
                        f.write_str("(")?;
                        first = true;
                        stack.push(items.iter());
                    }
                    Sexpr::Atom { text, .. } => f.write_str(text)?,
                    Sexpr::Str { text, .. } => write_quoted(f, text)?,
                }
            }
            let Some(top) = stack.last_mut() else { return Ok(()) };
            match top.next() {
                Some(item) => next = Some(item),
                None => {
                    stack.pop();
                    f.write_str(")")?;
                    first = false;
                    if stack.is_empty() {
                        return Ok(());
                    }
                }
            }
        }
    }
}

fn write_quoted(f: &mut fmt::Formatter<'_>, text: &str) -> fmt::Result {
    f.write_str("\"")?;
    for c in text.chars() {
        match c {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\r' => f.write_str("\\r")?,
            '\t' => f.write_str("\\t")?,
            _ => write!(f, "{c}")?,
        }
    }
    f.write_str("\"")
}
