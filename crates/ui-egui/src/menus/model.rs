//! The menu bar as data: Proteus 8 Schematic Capture's menus and items in their order, with
//! separators and shortcut texts, and what each item runs. Items without an engine command are
//! shown disabled (listed in `docs/ui-parity.md`).

use protasena_engine::Snap;

/// What an item does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// Runs the engine command with this id as a menu pick (its dialog first, if it has one).
    Cmd(&'static str),
    /// A window action.
    Ui(UiAct),
    /// Not built yet: the item is disabled.
    None,
}

/// Window actions behind menu items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiAct {
    /// File ▸ Exit Application.
    Exit,
    /// View ▸ Zoom To Area: the next drag in the editing window gives the area.
    ZoomArea,
    /// Help ▸ About.
    About,
}

impl UiAct {
    /// The engine command the action ends in, if any.
    pub fn command(self) -> Option<&'static str> {
        match self {
            UiAct::ZoomArea => Some("zoom-area"),
            UiAct::Exit | UiAct::About => None,
        }
    }
}

/// The state a checked item shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    /// View ▸ Toggle Grid.
    Grid,
    /// View ▸ Toggle False Origin.
    FalseOrigin,
    /// View ▸ Toggle X-Cursor.
    XCursor,
    /// View ▸ Snap … (a radio group).
    Snap(Snap),
    /// Tool ▸ Wire Autorouter.
    Autorouter,
}

/// One menu entry.
#[derive(Clone, Copy, Debug)]
pub enum Entry {
    /// An item.
    Item {
        /// Label (Proteus's).
        label: &'static str,
        /// Shortcut text (Proteus's).
        shortcut: Option<&'static str>,
        /// What it does.
        act: Act,
        /// Check mark, if it shows one.
        check: Option<Check>,
    },
    /// A separator.
    Sep,
    /// A submenu.
    Sub {
        /// Label.
        label: &'static str,
        /// Its entries.
        entries: &'static [Entry],
    },
    /// Design: the numbered list of sheets (the current one checked).
    Sheets,
}

/// A menu of the menu bar.
#[derive(Clone, Copy, Debug)]
pub struct Menu {
    /// Its name.
    pub label: &'static str,
    /// Its entries.
    pub entries: &'static [Entry],
}

const fn cmd(label: &'static str, shortcut: Option<&'static str>, id: &'static str) -> Entry {
    Entry::Item { label, shortcut, act: Act::Cmd(id), check: None }
}

const fn checked(label: &'static str, shortcut: &'static str, id: &'static str, check: Check) -> Entry {
    Entry::Item { label, shortcut: Some(shortcut), act: Act::Cmd(id), check: Some(check) }
}

const fn off(label: &'static str, shortcut: Option<&'static str>) -> Entry {
    Entry::Item { label, shortcut, act: Act::None, check: None }
}

const fn ui(label: &'static str, shortcut: Option<&'static str>, act: UiAct) -> Entry {
    Entry::Item { label, shortcut, act: Act::Ui(act), check: None }
}

use Entry::Sep;

const EXPORT_GRAPHICS: &[Entry] = &[
    cmd("Export Bitmap", None, "export-bitmap"),
    off("Export Metafile", None),
    cmd("Export DXF File", None, "export-dxf"),
    cmd("Export Encapsulated Postscript", None, "export-eps"),
    cmd("Export Adobe PDF File", None, "export-pdf"),
    cmd("Export SVG File", None, "export-svg"),
    off("Export Vector File", None),
];

const FILE: &[Entry] = &[
    cmd("New Project", Some("Ctrl+N"), "new"),
    cmd("Open Project", Some("Ctrl+O"), "open"),
    off("Open Sample Project", None),
    off("Import Legacy Project", None),
    off("Import ECAD Files", None),
    cmd("Save Project", Some("Ctrl+S"), "save"),
    cmd("Save Project As", None, "save-as"),
    off("Close Project", None),
    Sep,
    off("Import Image", None),
    off("Import Project Clip", None),
    Sep,
    Entry::Sub { label: "Export Graphics", entries: EXPORT_GRAPHICS },
    off("Export Project Clip", None),
    Sep,
    cmd("Print Design", None, "print-design"),
    off("Print Setup", None),
    off("Printer Information", None),
    off("Mark Output Area", None),
    Sep,
    off("Explore Project Folder", None),
    off("Edit Project Description", None),
    Sep,
    ui("Exit Application", Some("Alt+F4"), UiAct::Exit),
];

const EDIT: &[Entry] = &[
    cmd("Undo Changes", Some("Ctrl+Z"), "undo"),
    cmd("Redo Changes", Some("Ctrl+Y"), "redo"),
    Sep,
    cmd("Find/Edit Component", Some("E"), "edit-component"),
    Sep,
    cmd("Select All Objects", None, "select-all"),
    cmd("Clear Selection", None, "clear-selection"),
    cmd("Cut To Clipboard", None, "cut"),
    cmd("Copy To Clipboard", None, "copy"),
    cmd("Paste From Clipboard", None, "paste"),
    Sep,
    cmd("Align Objects", Some("Ctrl+A"), "align"),
    cmd("Send To Back", Some("Ctrl+B"), "send-to-back"),
    cmd("Bring To Front", Some("Ctrl+F"), "bring-to-front"),
    Sep,
    cmd("Tidy Design", None, "tidy"),
];

const VIEW: &[Entry] = &[
    cmd("Redraw Display", Some("R"), "redraw"),
    checked("Toggle Grid", "G", "toggle-grid", Check::Grid),
    checked("Toggle False Origin", "O", "toggle-false-origin", Check::FalseOrigin),
    checked("Toggle X-Cursor", "X", "toggle-x-cursor", Check::XCursor),
    Sep,
    checked("Snap 10th", "Ctrl+F1", "snap-10th", Check::Snap(Snap::Th10)),
    checked("Snap 50th", "F2", "snap-50th", Check::Snap(Snap::Th50)),
    checked("Snap 0.1in", "F3", "snap-100th", Check::Snap(Snap::In01)),
    checked("Snap 0.5in", "F4", "snap-500th", Check::Snap(Snap::In05)),
    Sep,
    cmd("Center At Cursor", Some("F5"), "center-at-cursor"),
    cmd("Zoom In", Some("F6"), "zoom-in"),
    cmd("Zoom Out", Some("F7"), "zoom-out"),
    cmd("Zoom To View Entire Sheet", Some("F8"), "zoom-all"),
    ui("Zoom To Area", None, UiAct::ZoomArea),
    Sep,
    off("Toolbar Configuration", None),
];

const TOOL: &[Entry] = &[
    checked("Wire Autorouter", "W", "toggle-autorouter", Check::Autorouter),
    cmd("Search & Tag", Some("T"), "search-tag"),
    cmd("Property Assignment Tool", Some("A"), "property-assign"),
    Sep,
    cmd("Global Annotator", None, "annotate"),
    off("ASCII Data Import Tool", None),
    cmd("Electrical Rules Check", None, "erc"),
    cmd("Netlist Compiler", None, "export-netlist"),
    off("Model Compiler", None),
];

const DESIGN: &[Entry] = &[
    cmd("Edit Design Properties", None, "design-properties"),
    cmd("Edit Sheet Properties", None, "sheet-properties"),
    off("Edit Design Notes", None),
    Sep,
    off("Configure Power Rails...", None),
    Sep,
    cmd("New (Root) Sheet", None, "new-sheet"),
    cmd("Remove/Delete Sheet", None, "remove-sheet"),
    cmd("Goto Previous Root or Sub-sheet", Some("Page-Up"), "previous-sheet"),
    cmd("Goto Next Root or Sub-sheet", Some("Page-Down"), "next-sheet"),
    off("Exit to Parent Sheet", Some("Ctrl+X")),
    cmd("Goto Sheet", None, "goto-sheet"),
    Sep,
    Entry::Sheets,
];

const GRAPH: &[Entry] = &[
    off("Edit Graph...", None),
    off("Add Traces...", Some("Ctrl+T")),
    off("Simulate Graph", Some("Space")),
    off("View Simulation Log", Some("Ctrl+V")),
    off("Export Graph Data...", None),
    off("Clear Graph Data...", None),
    Sep,
    off("Verify Graphs", None),
    off("Verify Files", None),
];

const DEBUG: &[Entry] = &[
    off("Start VSM Debugging", Some("Ctrl+F12")),
    off("Pause VSM Debugging", Some("Pause")),
    off("Stop VSM Debugging", Some("Shift+Pause")),
    Sep,
    off("Run Simulation", Some("F12")),
    off("Run Simulation (no breakpoints)", Some("Alt+F12")),
    off("Run Simulation (timed breakpoint)", None),
    Sep,
    off("Step Over Source Line", Some("F10")),
    off("Step Into Source Line", Some("F11")),
    off("Step Out from Source Line", Some("Ctrl+F11")),
    off("Run To Source Line", Some("Ctrl+F10")),
    off("Animated Single Step", Some("Alt+F11")),
    Sep,
    off("Reset Debug Popup Windows", None),
    off("Reset Persistent Model Data", None),
    off("Configure Diagnostics", None),
    off("Enable Remote Debug Monitor", None),
    Sep,
    off("Horz. Tile Popup Windows", None),
    off("Vertical Tile Popup Windows", None),
];

const LIBRARY: &[Entry] = &[
    cmd("Pick Parts", Some("P"), "pick-device"),
    off("Import Parts", None),
    Sep,
    off("Make Device", None),
    off("Make Symbol", None),
    off("Packaging Tool", None),
    off("Decompose", None),
    Sep,
    off("Compile To Library", None),
    off("Place Library", None),
    off("Verify Packagings", None),
    off("Manage Changes", None),
    Sep,
    off("Library Manager", None),
];

const TEMPLATE: &[Entry] = &[
    off("Goto Master Sheet", None),
    Sep,
    off("Set Design Colours", None),
    off("Set Graph & Trace Colours", None),
    off("Set Graphic Styles", None),
    off("Set Text Styles", None),
    off("Set 2D Graphics Defaults", None),
    off("Set Junction Dot Style", None),
    Sep,
    off("Apply Styles From Template", None),
    off("Save Design as Template", None),
];

const SYSTEM: &[Entry] = &[
    off("System Settings", None),
    Sep,
    off("Text Viewer", None),
    Sep,
    off("Set Display Options", None),
    off("Set Keyboard Mapping", None),
    off("Set Property Definitions", None),
    off("Set Sheet Sizes", None),
    off("Set Text Editor", None),
    Sep,
    off("Set Animation Options", None),
    off("Set Simulation Options", None),
    Sep,
    off("Restore Default Settings", None),
];

const HELP: &[Entry] = &[
    off("Overview", Some("F1")),
    Sep,
    ui("About Protasena", None, UiAct::About),
    Sep,
    off("Schematic Capture Help", Some("F1")),
    off("Schematic Capture Tutorial", None),
    off("Simulation Help", None),
    off("VSM Model/SDK Help", None),
];

/// The menu bar, left to right.
pub const MENUS: &[Menu] = &[
    Menu { label: "File", entries: FILE },
    Menu { label: "Edit", entries: EDIT },
    Menu { label: "View", entries: VIEW },
    Menu { label: "Tool", entries: TOOL },
    Menu { label: "Design", entries: DESIGN },
    Menu { label: "Graph", entries: GRAPH },
    Menu { label: "Debug", entries: DEBUG },
    Menu { label: "Library", entries: LIBRARY },
    Menu { label: "Template", entries: TEMPLATE },
    Menu { label: "System", entries: SYSTEM },
    Menu { label: "Help", entries: HELP },
];

/// Every item with its menu path (submenus included), in order.
pub fn items() -> Vec<(Vec<&'static str>, Entry)> {
    fn walk(path: &[&'static str], entries: &'static [Entry], out: &mut Vec<(Vec<&'static str>, Entry)>) {
        for e in entries {
            match e {
                Entry::Item { label, .. } => out.push(([path, &[*label]].concat(), *e)),
                Entry::Sub { label, entries } => walk(&[path, &[*label]].concat(), entries, out),
                Entry::Sep | Entry::Sheets => {}
            }
        }
    }
    let mut out = Vec::new();
    for m in MENUS {
        walk(&[m.label], m.entries, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use protasena_engine::{command_specs, find_command};

    #[test]
    fn menus_are_proteus_s_in_order() {
        let names: Vec<_> = MENUS.iter().map(|m| m.label).collect();
        assert_eq!(names, ["File", "Edit", "View", "Tool", "Design", "Graph", "Debug", "Library", "Template", "System", "Help"]);
    }

    #[test]
    fn items_with_a_command_match_its_menu_path_and_shortcut() {
        for (path, e) in items() {
            let Entry::Item { shortcut, act: Act::Cmd(id), .. } = e else { continue };
            let spec = find_command(id).unwrap_or_else(|| panic!("{path:?}: no command {id}"));
            assert_eq!(spec.menu, path.as_slice(), "{id}");
            assert_eq!(spec.shortcut, shortcut, "{id}");
        }
    }

    #[test]
    fn every_engine_command_with_a_menu_path_has_its_item() {
        let all = items();
        for spec in command_specs().iter().filter(|s| !s.menu.is_empty()) {
            let found = all.iter().any(|(path, e)| {
                path.as_slice() == spec.menu
                    && match e {
                        Entry::Item { act: Act::Cmd(id), .. } => *id == spec.id,
                        Entry::Item { act: Act::Ui(u), .. } => u.command() == Some(spec.id),
                        _ => false,
                    }
            });
            assert!(found, "{} ({:?}) is not in the menus", spec.id, spec.menu);
        }
    }

    #[test]
    fn no_two_separators_follow_each_other() {
        for m in MENUS {
            let seps: Vec<bool> = m.entries.iter().map(|e| matches!(e, Entry::Sep)).collect();
            assert!(!seps.windows(2).any(|w| w == [true, true]), "{}", m.label);
            assert_ne!(seps.first(), Some(&true), "{}", m.label);
        }
    }
}
