# Asset attribution

Every non-code file under `crates/`, `apps/`, `assets/` and `docs/images/` (images, icons, fonts,
test fixtures, data files) is listed here with its author, source and licence. A row names one
file as `` `path` `` or a whole directory as `` `dir/` ``. `cargo xtask assets` (part of
`cargo xtask ci`) fails if a file is missing from this table.

**Policy (mandatory):** Protasena contains **no Labcenter Electronics material**: no Proteus
icons, images, artwork, colours taken from screenshots, help text, libraries (symbols, packages,
simulation models) or sample designs. Screenshots of Proteus are never committed. Every asset is
original work by Protasena contributors or third-party material under an open licence (OSI open
source, public domain / CC0, or Creative Commons that allows redistribution). KiCad-derived
library data never enters this repository: it lives in `KaanAlper/Protasena-library`
(CC-BY-SA 4.0, see `NOTICE`). Test fixtures are small files we write ourselves.

Generated-in-code assets are original and have no file to list (none yet).

| Asset | Author | Source | Licence | Notes |
|---|---|---|---|---|
| `crates/kicad/tests/data/` | Protasena contributors | Written by hand for the `protasena-kicad` tests (synthetic designs, no KiCad library content) | CC0-1.0 | Small KiCad-format fixtures (`.kicad_sym`, `.kicad_mod`) |
