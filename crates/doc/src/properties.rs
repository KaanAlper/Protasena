//! Component property values: an ordered list of `(name, value)` pairs.

/// Property values of a component in Edit Component order. Names compare without case, like
/// Proteus property keys; each name occurs at most once.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(transparent))]
pub struct Properties(Vec<(String, String)>);

impl Properties {
    /// No properties.
    pub fn new() -> Self {
        Self::default()
    }

    /// Properties from pairs; a later pair with the same name (without case) replaces an
    /// earlier one in its place.
    pub fn from_pairs<I: IntoIterator<Item = (String, String)>>(pairs: I) -> Self {
        let mut props = Self::new();
        for (name, value) in pairs {
            props.set(&name, &value);
        }
        props
    }

    /// The value of a property.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }

    /// Sets a property, keeping its place if it exists and appending it otherwise.
    pub fn set(&mut self, name: &str, value: &str) {
        match self.0.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(name)) {
            Some(slot) => slot.1 = value.to_owned(),
            None => self.0.push((name.to_owned(), value.to_owned())),
        }
    }

    /// The same properties with one set (see [`Properties::set`]).
    pub fn with(mut self, name: &str, value: &str) -> Self {
        self.set(name, value);
        self
    }

    /// Removes a property and returns its value.
    pub fn remove(&mut self, name: &str) -> Option<String> {
        let at = self.0.iter().position(|(k, _)| k.eq_ignore_ascii_case(name))?;
        Some(self.0.remove(at).1)
    }

    /// The pairs in order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Number of properties.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The first name that occurs twice (without case), if any: only possible in data that was
    /// deserialized, never through the methods above.
    pub fn duplicate_name(&self) -> Option<&str> {
        self.0.iter().enumerate().find_map(|(i, (k, _))| self.0.iter().skip(i + 1).any(|(o, _)| o.eq_ignore_ascii_case(k)).then_some(k.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_order_and_ignores_case() {
        let mut p = Properties::new().with("TOLERANCE", "5%").with("POWER", "0.25W");
        p.set("tolerance", "1%");
        assert_eq!(p.iter().collect::<Vec<_>>(), [("TOLERANCE", "1%"), ("POWER", "0.25W")]);
        assert_eq!(p.get("Power"), Some("0.25W"));
        assert_eq!(p.remove("POWER"), Some("0.25W".to_owned()));
        assert_eq!(p.remove("POWER"), None);
        assert_eq!(p.len(), 1);
        assert_eq!(p.duplicate_name(), None);
        let q = Properties::from_pairs([("a".into(), "1".into()), ("A".into(), "2".into())]);
        assert_eq!(q.iter().collect::<Vec<_>>(), [("a", "2")]);
    }
}
