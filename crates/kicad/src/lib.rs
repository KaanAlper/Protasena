//! Protasena KiCad reader (layer L0, standalone).
//!
//! Reads KiCad's S-expression files (`.kicad_sym` symbol libraries, `.kicad_mod` footprints)
//! into plain models. Written from KiCad's published file format documentation only, never from
//! KiCad's code. Input is hostile: depth and size limits, no panics. Empty until M1 Task 3.
