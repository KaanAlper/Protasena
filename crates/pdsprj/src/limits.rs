//! What the reader accepts. Every input-sized allocation and loop is bounded by one of these.

/// Largest archive read: 256 MiB.
pub const MAX_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;

/// Most members in an archive (Proteus writes four).
pub const MAX_MEMBERS: usize = 64;

/// Largest member, uncompressed: 128 MiB.
pub const MAX_MEMBER_BYTES: u64 = 128 * 1024 * 1024;

/// Largest expansion (uncompressed / compressed size) accepted for members above
/// [`RATIO_FREE_BYTES`]. Proteus's members compress by less than 10×.
pub const MAX_RATIO: u64 = 256;

/// Members up to this size (1 MiB) are not checked against [`MAX_RATIO`].
pub const RATIO_FREE_BYTES: u64 = 1024 * 1024;

/// Largest project manifest (`PROJECT.XML`) parsed: 1 MiB.
pub const MAX_MANIFEST_BYTES: usize = 1024 * 1024;

/// Most entries in a manifest list (scripts).
pub const MAX_MANIFEST_ENTRIES: usize = 1024;

/// Most graphic or text styles in the design configuration.
pub const MAX_STYLES: usize = 4096;

/// Most embedded library entries (symbols and device definitions together).
pub const MAX_LIBRARY_ENTRIES: usize = 65_536;

/// Most circuit sections in a design.
pub const MAX_CIRCUITS: usize = 4096;

/// Most records decoded in one circuit.
pub const MAX_RECORDS: usize = 1_000_000;

/// Most text fields in one record.
pub const MAX_FIELDS: usize = 64;

/// Most pins of one component.
pub const MAX_PINS: usize = 4096;

/// Longest text block (a property block, a device definition): 16 MiB.
pub const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;

/// Most lines in one device-definition text.
pub const MAX_TEXT_LINES: usize = 100_000;

/// Most candidate positions tried, per circuit, when looking for the next known record after an
/// unknown one. Past it the rest of the circuit is one unknown record.
pub const MAX_RESYNC_TRIES: usize = 1_000_000;
