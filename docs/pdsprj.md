# The `.pdsprj` project file

> **Last reviewed:** 2026-10-20 · **Last updated:** 2026-10-20 · **Change:** major (first description: container, manifest, schematic sections, decoded records) · **Target:** Labcenter Proteus 8 Professional (Windows)

What Protasena knows about Proteus 8 project files, in our own words. Everything here was worked
out from data files the program saved (black-box analysis); the program itself was never
inspected. No sample file, byte sequence or library content is reproduced here or in the
repository: tests use small files we write ourselves in the same structure. Reader:
`crates/pdsprj` (`protasena-pdsprj`); import into the design model: `protasena_io::import_pdsprj`.
What is still missing is tracked in `docs/gaps.md` (S5).

Backups (`*.pdsbak`) have the same format.

## Container

A plain ZIP archive: deflated members, no ZIP64, no encryption, no data descriptors. Members:

| Member | Kind | Role | Read |
|---|---|---|---|
| `PROJECT.XML` | small XML | project manifest | release, file version, creation and modification times (Unix seconds), script list |
| `ROOT.DSN` | binary | the schematic: header, configuration, embedded library, circuits | see below |
| `ROOT.CDB` | binary | connectivity cache (duplicates what `ROOT.DSN` says) | listed only |
| `SCRIPTS/*.DAT` | text | script members named by the manifest (power rails) | listed only |

The reader refuses archives over 256 MiB, more than 64 members, members over 128 MiB, members
that would expand more than 256× (above 1 MiB), unknown compression methods and checksum
mismatches. The manifest carries a licence-related attribute: the reader skips every attribute it
does not name, so that value is never stored or reported.

## `ROOT.DSN`

Little-endian. Sections start with an ASCII banner ending in byte 0x1A. Coordinates are signed
32-bit integers in **10 nm units: 2540 per thou** (254 000 per 0.1 in). Strings are either
length-prefixed (one length byte, "pstr" below) or NUL-terminated ("cstr"); text is read as
ISO 8859-1.

### Header (`ISIS SCHEMATIC FILE`, fixed 0x104 bytes)

| Offset | Field |
|---|---|
| 0x16–0x6E | design property texts, NUL-separated (field order not yet known; not decoded) |
| 0x6F, 0x8B | creation and modification dates, `DD/MM/YY`, NUL-padded |
| 0xA7, 0xA9 | u16 program release (×100), u16 file format version |
| 0xAD, 0xB1 | u32 creation and modification times, Unix seconds |

### Design configuration (`DESIGN CONFIGURATION DATA`)

1. A fixed-size preamble (0x2BE bytes: fonts, colour table, sheet settings; not decoded).
2. Graphic styles, then text styles: each a u16 count and records of
   `pstr name, u8 flags, u16 n, n−2 bytes` (`n` counts itself). Only the names are kept.
3. Library lists, each introduced by u16 0xFFFE and ended by an empty name. Entries are
   `pstr name, u32 stamp, u32 n, n−4 bytes`. Two kinds:
   - **Symbols**: graphics of every symbol the design uses (not decoded; name and size kept).
   - **Device definitions**: a binary part (not decoded) and, at the end, a definition text.
     The body starts with the text's offset; the u32 before the text is its length including a
     final NUL. The text is line-based: `{*DEVICE}`, `{*PROPDEFS}`, `{*INDEX}`, `{*COMPONENT}`
     section lines (and `*NAME args` lines such as a pin-out per package), followed by
     `{KEY=VALUE}` records. The reader keeps sections and records generically. These are the
     data behind Pick Devices (index: category, sub-category, manufacturer, description) and
     Edit Component (property definitions: prompt, type, flags; defaults).
4. A u32 trailer.

Designs are self-contained: every used symbol and device travels inside the file. Protasena does
not convert them (they are the library's material); the import lists them in its report.

### Circuits (`ISIS CIRCUIT FILE`)

Header: 7 bytes, u32 sheet width, u32 sheet height (file units), u16, `OBJECT DATA` NUL. Then the
object records, an end mark (byte 0xFF) and a trailer up to the next section. The first circuit
is the root sheet; a further circuit seen so far holds no objects, only simulation settings in
its trailer (not decoded).

Records are not length-prefixed; each starts with a tag byte. **Links** are absolute offsets of
other records in `ROOT.DSN` (0 = none); in every file examined, every link pointed at the start of
a decoded record, which confirms the framing below.

**Text field** (inside records): `0xFF, pstr text, i32 x, i32 y, 26 attribute bytes (u16 flags
at +4, u32 character height at +10, last byte: shown), cstr font, cstr text style, u32 0`; when
flag bit 2 is set, a u32 total length and the rest of the text follow (property blocks).

| Record | Tag | Layout after the tag | Decoded |
|---|---|---|---|
| Wire / bus | 0x00 | 24-byte style block ending `0x02 0x7F`, cstr graphic style (`WIRE`, `BUS WIRE`), u16 0, u16 n, n × (x, y) | style, vertices |
| Component | 0x00 or 0x08, then a text field | text fields (reference `COMPONENT ID`, value `COMPONENT VALUE`, device name, `{KEY=VALUE}` property block), u16 pin count, pstr device, x, y, 17 bytes (u32 ordinal at +4), one u32 link per pin | all but the 17 placement bytes (orientation is presumably there) |
| Node | 0x01 | x, y, 4 × u32 link | position, links |
| Terminal | 0x10 | x, y, u32, pstr symbol (`$TER…`), 6 bytes, text fields (label), u32 link | position, symbol, label, link |
| Generator / probe | 0x30–0x3F | x, y, 6 bytes, pstr name, pstr symbol, text fields, 2 bytes, 2 × u32 link; with tag bit 3 a u8, u32 n and n−4 bytes of settings | position, name, symbol, fields, links |
| Simulation graph | 0x28 (also seen as 0x80 then 0x28) | title pstr, a length-prefixed options text, then axis and trace data | not decoded |

After an unknown record the reader scans forward for the next position where a known record
parses and is followed by another one (or by the end mark); the bytes in between become one
unknown record (tag and size only, never the bytes).

## Import into Protasena

| `.pdsprj` | Protasena |
|---|---|
| circuit with objects | sheet (first: `Root sheet 1`), custom size from the circuit header |
| wire / `BUS WIRE` | wire / bus, thou = units ÷ 2540, y as stored |
| node joining ≥ 3 records | junction dot (nodes joining fewer are not drawn) |
| component | component in library `PDSPRJ` with the device name, reference, value, origin, label offsets and visibility, `PACKAGE` → package, other properties in order; **unrotated** (orientation not decoded) |
| terminal | terminal; kind from the symbol name (`$TER` + Terminals mode name), name from the label |
| generator, probe, graph, unknown records | skipped, counted in the report |
| embedded device definitions and symbols | listed in the report only |
| header dates | design properties created / modified |

## Dependencies

The ZIP reader is our own; deflate decoding uses `miniz_oxide` (MIT OR Zlib OR Apache-2.0, with
`adler2`, 0BSD OR MIT OR Apache-2.0), default features off except allocation, no `std`-only or
platform code (WebAssembly-clean).

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-20 | major | First description: container, manifest, header, configuration, embedded library, circuit records, import mapping |
