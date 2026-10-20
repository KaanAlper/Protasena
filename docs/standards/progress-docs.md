# Progress docs

How the parity and work-list documents are kept.

| Doc                   | Holds |
|-----------------------|-------|
| `docs/gaps.md`        | The work list: every known shortfall against Proteus, ranked; pick work from its top rows |
| `docs/ui-parity.md`   | UI element by element against Labcenter Proteus 8 Professional (Windows) |
| `docs/*-parity.md`    | Further checklists as areas start (file formats, netlist, simulation, PCB) |

Every doc starts with a status line:
`> **Last reviewed:** YYYY-MM-DD · **Last updated:** YYYY-MM-DD · **Change:** minor|major (what) · **Target:** Labcenter Proteus 8 Professional (Windows)`
and ends with a revision-history table (Date | Change | Summary).

Row status: ✅ matches, ◐ partial (differences listed), ✗ missing, ★ intentional improvement
approved by the owner (with a one-line note of the difference). A row changes status only after
a side-by-side observation.

When work lands: update the affected rows, the status line and the revision history in the same
commit. Numbers are either **measured** (counted from code or from a side-by-side comparison) or
**estimated**, and each says which.
