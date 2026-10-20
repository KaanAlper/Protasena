//! Limits the loader and validator enforce, so a hostile or broken pack fails cleanly instead
//! of exhausting memory. They are far above what real libraries need (the largest KiCad
//! symbols have a few thousand pins; KiCad's whole library has tens of thousands of parts).

/// Largest single JSON file.
pub const MAX_FILE_BYTES: u64 = 64 << 20;
/// Largest sum of all files of a pack.
pub const MAX_PACK_BYTES: u64 = 1 << 30;
/// Libraries in one pack.
pub const MAX_LIBRARIES: usize = 4096;
/// Devices in one library.
pub const MAX_DEVICES: usize = 100_000;
/// Packages in one library.
pub const MAX_PACKAGES: usize = 100_000;
/// Units (including body styles) of one symbol.
pub const MAX_UNITS: usize = 256;
/// Pins of one unit.
pub const MAX_PINS: usize = 10_000;
/// Alternate functions of one pin.
pub const MAX_ALTERNATES: usize = 256;
/// Graphics of one unit or package.
pub const MAX_GRAPHICS: usize = 100_000;
/// Pads of one package.
pub const MAX_PADS: usize = 10_000;
/// Property definitions of one device.
pub const MAX_PROP_DEFS: usize = 256;
/// Packages one device may list.
pub const MAX_DEVICE_PACKAGES: usize = 1024;
/// Length in bytes of a library, device, package or property name.
pub const MAX_NAME_BYTES: usize = 256;
/// Points of one polyline or Bézier chain.
pub const MAX_POINTS: usize = 100_000;
/// Entries of one pin map.
pub const MAX_PIN_MAP: usize = 65_536;
/// Entries of short lists: keywords, keyword choices, model parameters, pad layers.
pub const MAX_LIST: usize = 1024;
