//! Lossless reading: tokens the models do not know are kept as [`Unknown`] trees, and
//! [`Report`] counts them (and enumerated values outside the documented set).

use std::collections::BTreeMap;

use crate::sexpr::Sexpr;

/// A token kept as it was read, because the model has no field for it.
#[derive(Debug, Clone, PartialEq)]
pub struct Unknown {
    /// The token it was found in, e.g. `"pin"` or `"footprint"`.
    pub context: &'static str,
    /// The unread token itself.
    pub tree: Sexpr,
}

impl Unknown {
    pub fn new(context: &'static str, tree: &Sexpr) -> Self {
        Self { context, tree: tree.clone() }
    }

    /// The report key: `context/name`.
    pub fn key(&self) -> String {
        format!("{}/{}", self.context, self.tree.name())
    }
}

/// Counts of everything a model kept without understanding it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    /// Unknown tokens by `context/name`.
    pub unknown_tokens: BTreeMap<String, usize>,
    /// Enumerated values outside the documented set by `context=value`
    /// (kept in the model as `Other(value)`).
    pub other_values: BTreeMap<String, usize>,
    /// The first unknown tree seen for each key of `unknown_tokens`.
    pub examples: BTreeMap<String, Sexpr>,
}

impl Report {
    pub fn add_unknown(&mut self, u: &Unknown) {
        let key = u.key();
        if !self.examples.contains_key(&key) {
            self.examples.insert(key.clone(), u.tree.clone());
        }
        *self.unknown_tokens.entry(key).or_default() += 1;
    }

    pub fn add_other(&mut self, context: &str, value: &str) {
        *self.other_values.entry(format!("{context}={value}")).or_default() += 1;
    }

    /// Adds every count of `other`.
    pub fn merge(&mut self, other: &Report) {
        for (k, n) in &other.unknown_tokens {
            *self.unknown_tokens.entry(k.clone()).or_default() += n;
        }
        for (k, n) in &other.other_values {
            *self.other_values.entry(k.clone()).or_default() += n;
        }
        for (k, t) in &other.examples {
            if !self.examples.contains_key(k) {
                self.examples.insert(k.clone(), t.clone());
            }
        }
    }

    /// Number of unknown tokens.
    pub fn unknown_count(&self) -> usize {
        self.unknown_tokens.values().sum()
    }

    /// True when nothing was left unread.
    pub fn is_clean(&self) -> bool {
        self.unknown_tokens.is_empty() && self.other_values.is_empty()
    }
}

/// Walks a model and adds everything it kept unread to a [`Report`].
pub trait Inspect {
    fn visit(&self, report: &mut Report);

    /// A fresh report of this model.
    fn report(&self) -> Report {
        let mut r = Report::default();
        self.visit(&mut r);
        r
    }
}

impl Inspect for Unknown {
    fn visit(&self, report: &mut Report) {
        report.add_unknown(self);
    }
}

impl<T: Inspect> Inspect for [T] {
    fn visit(&self, report: &mut Report) {
        for t in self {
            t.visit(report);
        }
    }
}

impl<T: Inspect> Inspect for Vec<T> {
    fn visit(&self, report: &mut Report) {
        self.as_slice().visit(report);
    }
}

impl<T: Inspect + ?Sized> Inspect for &T {
    fn visit(&self, report: &mut Report) {
        (**self).visit(report);
    }
}

impl<T: Inspect> Inspect for Option<T> {
    fn visit(&self, report: &mut Report) {
        if let Some(t) = self {
            t.visit(report);
        }
    }
}

/// Enumerations with a documented set of values and an `Other(String)` fallback.
macro_rules! keyword_enum {
    ($(#[$meta:meta])* $name:ident, $ctx:literal { $($variant:ident = $kw:literal),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum $name {
            $($variant,)*
            /// A value outside the documented set, kept as written.
            Other(String),
        }

        impl $name {
            /// The value for a keyword; unknown keywords become `Other`.
            pub fn from_keyword(kw: &str) -> Self {
                match kw {
                    $($kw => Self::$variant,)*
                    other => Self::Other(other.to_owned()),
                }
            }

            /// The keyword as written in the file.
            pub fn keyword(&self) -> &str {
                match self {
                    $(Self::$variant => $kw,)*
                    Self::Other(s) => s,
                }
            }
        }

        impl $crate::report::Inspect for $name {
            fn visit(&self, report: &mut $crate::report::Report) {
                if let Self::Other(s) = self {
                    report.add_other($ctx, s);
                }
            }
        }
    };
}
pub(crate) use keyword_enum;
