//! Where devices come from: [`DeviceSource`], implemented for a [`Library`].

use protasena_doc::DeviceRef;
use protasena_library::{Device, Library};

/// Resolves a component's [`DeviceRef`] to its library device.
pub trait DeviceSource {
    /// The device, or `None` when it is not available.
    fn device(&self, r: &DeviceRef) -> Option<&Device>;
}

impl DeviceSource for Library {
    /// Looks the name up without case; a non-empty library name must match too (without case).
    fn device(&self, r: &DeviceRef) -> Option<&Device> {
        let matches = |d: &&Device| r.library.is_empty() || d.library.eq_ignore_ascii_case(&r.library);
        self.get(&r.name).filter(matches).or_else(|| self.devices().iter().filter(|d| d.name.eq_ignore_ascii_case(&r.name)).find(matches))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_resolves_by_name_and_library() {
        let lib = Library::builtin();
        assert_eq!(lib.device(&DeviceRef::new("GENERIC", "res")).map(|d| d.name.as_str()), Some("RES"));
        assert_eq!(lib.device(&DeviceRef::new("", "NPN")).map(|d| d.prefix.as_str()), Some("Q"));
        assert!(lib.device(&DeviceRef::new("OTHER", "RES")).is_none());
        assert!(lib.device(&DeviceRef::new("GENERIC", "NOPE")).is_none());
    }
}
