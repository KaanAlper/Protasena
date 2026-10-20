# UI parity

> **Last reviewed:** 2026-10-20 · **Last updated:** 2026-10-20 · **Change:** minor (Global Toolbar ▸ Bill of Materials) · **Target:** Labcenter Proteus 8 Professional (Windows)

Protasena's Schematic Capture interface element by element against Proteus's: menus, toolbars,
the mode toolbar, the overview window, the object selector, the editing window, the status bar
and dialogs. Rows come from the observed UI inventory (`plan/proteus/notes/ui-inventory.md`,
local), written in our own words. Status: ✅ matches, ◐ partial (differences listed), ✗ missing,
★ intentional improvement approved by the owner. Work items: [gaps.md](gaps.md).

**Headline (measured, counted from the rows below): 3 of 269 rows match** (✅ 3, ◐ 175, ✗ 91).
Proteus could only be observed from its menus, window tree and one screenshot (it runs elevated
on the reference PC, so no click, tooltip or dialog reached it): labels, order, shortcut texts
and placement are taken from that observation; every behaviour, enabled rule and dialog is
"not compared" until an observation session can drive Proteus. The look (colours, icons, fonts)
is our own by design and is not a parity criterion.

**Items without a command** (shown disabled, listed as ✗ below; measured from the menu and
toolbar tables in `crates/ui-egui`): 66 menu items (plus the recent-projects and graph lists)
and 12 toolbar buttons have no engine command yet, nor do 8 of the 22 modes, the animation
buttons, the design variant box and the L button.

## Menu bar

Items run their engine command as a menu pick (its dialog first, when it has one); an item is
enabled when its command's enabled rule allows it.

| Menu ▸ item | Shortcut | Status | Notes |
|---|---|---|---|
| Menu bar: File, Edit, View, Tool, Design, Graph, Debug, Library, Template, System, Help |  | ✅ | Names and order as observed (mnemonic underlines are not drawn) |
| File ▸ New Project | Ctrl+N | ◐ | Runs `new`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| File ▸ Open Project | Ctrl+O | ◐ | Runs `open`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| File ▸ Open Sample Project |  | ✗ | Shown disabled: no command yet (item without a command) |
| File ▸ Import Legacy Project |  | ✗ | Shown disabled: no command yet (item without a command) |
| File ▸ Import ECAD Files |  | ✗ | Shown disabled: no command yet (item without a command) |
| File ▸ Save Project | Ctrl+S | ◐ | Runs `save`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| File ▸ Save Project As |  | ◐ | Runs `save-as`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| File ▸ Close Project |  | ✗ | Shown disabled: no command yet (item without a command) |
| File ▸ Import Image |  | ✗ | Shown disabled: no command yet (item without a command) |
| File ▸ Import Project Clip |  | ✗ | Shown disabled: no command yet (item without a command) |
| File ▸ Export Graphics ▸ Export Bitmap |  | ◐ | Runs `export-bitmap`: the current sheet as PNG or BMP at a resolution (200 dpi by default), colour or mono, texts in our own stroke font. Proteus's dialog, options and defaults to compare |
| File ▸ Export Graphics ▸ Export Metafile |  | ✗ | Shown disabled: not built (a Windows-only format, EMF/WMF; no pure-Rust writer chosen) |
| File ▸ Export Graphics ▸ Export DXF File |  | ◐ | Runs `export-dxf`: the current sheet as R12 ASCII DXF in thou, a layer per kind of object, texts as `TEXT`, outlines only (no fills). To compare |
| File ▸ Export Graphics ▸ Export Encapsulated Postscript |  | ◐ | Runs `export-eps`: the current sheet at real size times the scale, texts in Helvetica. To compare |
| File ▸ Export Graphics ▸ Export Adobe PDF File |  | ◐ | Runs `export-pdf`: the current sheet or every sheet (a page each) at real size times the scale, texts in Helvetica (not embedded). To compare |
| File ▸ Export Graphics ▸ Export SVG File |  | ◐ | Runs `export-svg`: the current sheet at real size times the scale, texts as SVG text in Helvetica (or a sans-serif). To compare |
| File ▸ Export Graphics ▸ Export Vector File |  | ✗ | Shown disabled: not built (what Proteus writes here was not observed) |
| File ▸ Export Project Clip |  | ✗ | Shown disabled: no command yet (item without a command) |
| File ▸ Print Design |  | ◐ | Runs `print-design`: prints to a PDF file (every sheet by default) through the Export Graphics writer and can open it in the system viewer; no printer is driven. Proteus prints to the chosen printer: to compare |
| File ▸ Print Setup |  | ✗ | Shown disabled: no command yet (item without a command); printing goes to a PDF file |
| File ▸ Printer Information |  | ✗ | Shown disabled: no command yet (item without a command) |
| File ▸ Mark Output Area |  | ✗ | Shown disabled: no command yet (item without a command); exports always take the whole sheet |
| File ▸ Explore Project Folder |  | ✗ | Shown disabled: no command yet (item without a command) |
| File ▸ Edit Project Description |  | ✗ | Shown disabled: no command yet (item without a command) |
| File ▸ Exit Application | Alt+F4 | ◐ | Closes the window; with unsaved changes it asks Save / Don't Save / Cancel first (as does the window's close button). The question is our own: to compare |
| File ▸ (recent projects list) |  | ✗ | Not built: no recent-project list |
| Edit ▸ Undo Changes | Ctrl+Z | ◐ | Runs `undo`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Edit ▸ Redo Changes | Ctrl+Y | ◐ | Runs `redo`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Edit ▸ Find/Edit Component | E | ◐ | Runs `edit-component`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Edit ▸ Select All Objects |  | ◐ | Runs `select-all`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Edit ▸ Clear Selection |  | ◐ | Runs `clear-selection`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Edit ▸ Cut To Clipboard |  | ◐ | Runs `cut`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Edit ▸ Copy To Clipboard |  | ◐ | Runs `copy`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Edit ▸ Paste From Clipboard |  | ◐ | Runs `paste`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Edit ▸ Align Objects | Ctrl+A | ◐ | Runs `align` (with a selection): lines the selected objects up by their left, right, top or bottom edges or their centres, each move rounded to the snap grid, attached wires following. Built the conventional way: Proteus's choices to compare |
| Edit ▸ Send To Back | Ctrl+B | ◐ | Runs `send-to-back` (with a selection): first in the drawing order of their kind (kinds keep the sheet's drawing order). To compare |
| Edit ▸ Bring To Front | Ctrl+F | ◐ | Runs `bring-to-front` (with a selection): last in the drawing order of their kind. To compare |
| Edit ▸ Tidy Design |  | ◐ | Runs `tidy`: drops devices no part uses from the DEVICES list, and removes junction dots on no wire or bus, zero-length wires and empty texts (one undo step). Our own definition: what Proteus removes to compare |
| View ▸ Redraw Display | R | ◐ | Runs `redraw`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| View ▸ Toggle Grid | G | ◐ | Runs `toggle-grid`; the check mark shows its state. Behaviour not compared |
| View ▸ Toggle False Origin | O | ◐ | Runs `toggle-false-origin`; the check mark shows its state. Behaviour not compared |
| View ▸ Toggle X-Cursor | X | ◐ | Runs `toggle-x-cursor`; the check mark shows its state. Behaviour not compared |
| View ▸ Snap 10th | Ctrl+F1 | ◐ | Runs `snap-10th`; the check mark shows its state. Behaviour not compared |
| View ▸ Snap 50th | F2 | ◐ | Runs `snap-50th`; the check mark shows its state. Behaviour not compared |
| View ▸ Snap 0.1in | F3 | ◐ | Runs `snap-100th`; the check mark shows its state. Behaviour not compared |
| View ▸ Snap 0.5in | F4 | ◐ | Runs `snap-500th`; the check mark shows its state. Behaviour not compared |
| View ▸ Center At Cursor | F5 | ◐ | Runs `center-at-cursor`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| View ▸ Zoom In | F6 | ◐ | Runs `zoom-in`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| View ▸ Zoom Out | F7 | ◐ | Runs `zoom-out`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| View ▸ Zoom To View Entire Sheet | F8 | ◐ | Runs `zoom-all`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| View ▸ Zoom To Area |  | ◐ | The next drag in the editing window gives the area (`zoom-area`); Proteus's interaction not compared |
| View ▸ Toolbar Configuration |  | ✗ | Shown disabled: no command yet (item without a command) |
| Tool ▸ Wire Autorouter | W | ◐ | Runs `toggle-autorouter`; the check mark shows its state. Behaviour not compared |
| Tool ▸ Search & Tag | T | ◐ | Runs `search-tag`: tags (selects) the parts of the current sheet whose property (REF, VALUE, DEVICE, PACKAGE or any other) equals, begins, ends or contains a string, or is less or greater as an engineering value, each but the last two also negated; a new search, AND or OR with the tagged parts; case optional. Built the conventional way: Proteus's options to compare |
| Tool ▸ Property Assignment Tool | A | ◐ | Runs `property-assign`: `NAME=VALUE` assigns, `OLD=NEW` renames, `NAME` removes, on the tagged parts, the sheet's or the design's, as one undo step; `#` counts up from Count by Increment. Proteus's actions (show, hide, resize, on click) not built; to compare |
| Tool ▸ Global Annotator |  | ◐ | Runs `annotate`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Tool ▸ ASCII Data Import Tool |  | ✗ | Shown disabled: no command yet (item without a command) |
| Tool ▸ Electrical Rules Check |  | ◐ | Runs `erc`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Tool ▸ Netlist Compiler |  | ◐ | Runs `export-netlist`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Tool ▸ Model Compiler |  | ✗ | Shown disabled: no command yet (item without a command) |
| Design ▸ Edit Design Properties |  | ◐ | Runs `design-properties`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Design ▸ Edit Sheet Properties |  | ◐ | Runs `sheet-properties`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Design ▸ Edit Design Notes |  | ✗ | Shown disabled: no command yet (item without a command) |
| Design ▸ Configure Power Rails... |  | ✗ | Shown disabled: no command yet (item without a command) |
| Design ▸ New (Root) Sheet |  | ◐ | Runs `new-sheet`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Design ▸ Remove/Delete Sheet |  | ◐ | Runs `remove-sheet`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Design ▸ Goto Previous Root or Sub-sheet | Page-Up | ◐ | Runs `previous-sheet` in hierarchy order (each root sheet, then its sub-sheets depth first). Label, place and shortcut as observed; Proteus's order to compare |
| Design ▸ Goto Next Root or Sub-sheet | Page-Down | ◐ | Runs `next-sheet`, in the same order. Label, place and shortcut as observed; Proteus's order to compare |
| Design ▸ Exit to Parent Sheet | Ctrl+X | ◐ | Runs `exit-to-parent`: on a child sheet, shows the parent sheet with the subcircuit selected; disabled on a root sheet. Label, place and shortcut as observed; behaviour to compare |
| Design ▸ Goto Sheet |  | ◐ | Runs `goto-sheet`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Design ▸ (numbered sheet list) |  | ◐ | Lists the sheets in hierarchy order, the current one checked; a sub-sheet is indented under its parent with its instance path (`SUB1 (SUB1)`); a pick runs `goto-sheet`. Proteus's presentation of sub-sheets to compare |
| Graph ▸ Edit Graph... |  | ✗ | Shown disabled: no command yet (item without a command) |
| Graph ▸ Add Traces... | Ctrl+T | ✗ | Shown disabled: no command yet (item without a command) |
| Graph ▸ Simulate Graph | Space | ✗ | Shown disabled: no command yet (item without a command) |
| Graph ▸ View Simulation Log | Ctrl+V | ✗ | Shown disabled: no command yet (item without a command) |
| Graph ▸ Export Graph Data... |  | ✗ | Shown disabled: no command yet (item without a command) |
| Graph ▸ Clear Graph Data... |  | ✗ | Shown disabled: no command yet (item without a command) |
| Graph ▸ Verify Graphs |  | ✗ | Shown disabled: no command yet (item without a command) |
| Graph ▸ Verify Files |  | ✗ | Shown disabled: no command yet (item without a command) |
| Graph ▸ (numbered graph list) |  | ✗ | No graphs yet |
| Debug ▸ Start VSM Debugging | Ctrl+F12 | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Pause VSM Debugging | Pause | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Stop VSM Debugging | Shift+Pause | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Run Simulation | F12 | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Run Simulation (no breakpoints) | Alt+F12 | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Run Simulation (timed breakpoint) |  | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Step Over Source Line | F10 | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Step Into Source Line | F11 | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Step Out from Source Line | Ctrl+F11 | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Run To Source Line | Ctrl+F10 | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Animated Single Step | Alt+F11 | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Reset Debug Popup Windows |  | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Reset Persistent Model Data |  | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Configure Diagnostics |  | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Enable Remote Debug Monitor |  | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Horz. Tile Popup Windows |  | ✗ | Shown disabled: no command yet (item without a command) |
| Debug ▸ Vertical Tile Popup Windows |  | ✗ | Shown disabled: no command yet (item without a command) |
| Library ▸ Pick Parts | P | ◐ | Runs `pick-device`. Label, place and shortcut as observed; enabled rule, behaviour and dialog not compared |
| Library ▸ Import Parts |  | ✗ | Shown disabled: no command yet (item without a command) |
| Library ▸ Make Device |  | ✗ | Shown disabled: no command yet (item without a command) |
| Library ▸ Make Symbol |  | ✗ | Shown disabled: no command yet (item without a command) |
| Library ▸ Packaging Tool |  | ✗ | Shown disabled: no command yet (item without a command) |
| Library ▸ Decompose |  | ✗ | Shown disabled: no command yet (item without a command) |
| Library ▸ Compile To Library |  | ✗ | Shown disabled: no command yet (item without a command) |
| Library ▸ Place Library |  | ✗ | Shown disabled: no command yet (item without a command) |
| Library ▸ Verify Packagings |  | ✗ | Shown disabled: no command yet (item without a command) |
| Library ▸ Manage Changes |  | ✗ | Shown disabled: no command yet (item without a command) |
| Library ▸ Library Manager |  | ✗ | Shown disabled: no command yet (item without a command) |
| Template ▸ Goto Master Sheet |  | ✗ | Shown disabled: no command yet (item without a command) |
| Template ▸ Set Design Colours |  | ◐ | Runs `set-design-colours` (dialog first): the design's own paper, grid, highlight and drag colours, saved in the design, one undo step; unset colours are our theme's. Built the conventional way with our own dialog (to compare: Proteus's dialog, fields and defaults were not observable) |
| Template ▸ Set Graph & Trace Colours |  | ✗ | Shown disabled: no command yet (item without a command) |
| Template ▸ Set Graphic Styles |  | ◐ | Runs `set-graphic-styles`: line width, colour and fill of our own style list (component, pin, wire, bus, terminal, subcircuit). Built the conventional way with our own dialog (to compare: Proteus's dialog, fields and defaults were not observable); Proteus's named styles not copied |
| Template ▸ Set Text Styles |  | ◐ | Runs `set-text-styles`: height and colour of references, values, pin names and numbers, wire labels, terminal names, free texts without a height of their own, subcircuit names. Built the conventional way with our own dialog (to compare: Proteus's dialog, fields and defaults were not observable) |
| Template ▸ Set 2D Graphics Defaults |  | ◐ | Runs `set-2d-graphics-defaults`: line width, colour and fill of every 2D graphic (graphics have no style of their own yet). Built the conventional way with our own dialog (to compare: Proteus's dialog, fields and defaults were not observable) |
| Template ▸ Set Junction Dot Style |  | ◐ | Runs `set-junction-style`: dot size (diameter) and shape (circle, square, diamond). Built the conventional way with our own dialog (to compare: Proteus's dialog, fields and defaults were not observable) |
| Template ▸ Apply Styles From Template |  | ✗ | Shown disabled: no command yet (item without a command) |
| Template ▸ Save Design as Template |  | ✗ | Shown disabled: no command yet (item without a command) |
| System ▸ System Settings |  | ◐ | Runs `system-settings`: the parts library folder (used when neither `--library` nor `PROTASENA_LIBRARY_DIR` is given, at the next start) and the folder the file choosers start in. Saved at once to the user's settings file (`settings.json` in the configuration directory, `PROTASENA_CONFIG_DIR` overrides it); not part of the design or undo. Built the conventional way with our own dialog (to compare: Proteus's dialog, fields and defaults were not observable) |
| System ▸ Text Viewer |  | ✗ | Shown disabled: no command yet (item without a command) |
| System ▸ Set Display Options |  | ◐ | Runs `set-display-options`: grid as lines, the mouse-wheel zoom factor, the pick distance in pixels. Saved at once to the user's settings file (`settings.json` in the configuration directory, `PROTASENA_CONFIG_DIR` overrides it); not part of the design or undo. Built the conventional way with our own dialog (to compare: Proteus's dialog, fields and defaults were not observable) |
| System ▸ Set Keyboard Mapping |  | ◐ | Runs `set-keyboard-mapping`: a table of every command with a menu item or key, its key editable, conflicts shown while typing and refused (no key runs two commands), Restore Defaults; menus show the keys in use. Saved at once to the user's settings file (`settings.json` in the configuration directory, `PROTASENA_CONFIG_DIR` overrides it); not part of the design or undo. Built the conventional way with our own dialog (to compare: Proteus's dialog, fields and defaults were not observable) |
| System ▸ Set Property Definitions |  | ◐ | Runs `set-property-definitions`: names, defaults and descriptions Edit Component offers for every part. Saved at once to the user's settings file (`settings.json` in the configuration directory, `PROTASENA_CONFIG_DIR` overrides it); not part of the design or undo. Built the conventional way with our own dialog (to compare: Proteus's dialog, fields and defaults were not observable) |
| System ▸ Set Sheet Sizes |  | ◐ | Runs `set-sheet-sizes`: the named sizes Edit Sheet Properties offers (A4…A0, A…E by default), editable, with Restore Defaults. Saved at once to the user's settings file (`settings.json` in the configuration directory, `PROTASENA_CONFIG_DIR` overrides it); not part of the design or undo. Built the conventional way with our own dialog (to compare: Proteus's dialog, fields and defaults were not observable) |
| System ▸ Set Text Editor |  | ◐ | Runs `set-text-editor`: font size and a fixed-width font for the text and script editor. Saved at once to the user's settings file (`settings.json` in the configuration directory, `PROTASENA_CONFIG_DIR` overrides it); not part of the design or undo. Built the conventional way with our own dialog (to compare: Proteus's dialog, fields and defaults were not observable) |
| System ▸ Set Animation Options |  | ✗ | Shown disabled: no command yet (item without a command) |
| System ▸ Set Simulation Options |  | ✗ | Shown disabled: no command yet (item without a command) |
| System ▸ Restore Default Settings |  | ◐ | Runs `restore-default-settings` after our own question: every system setting back to Protasena's defaults (the design's template is kept). Built the conventional way with our own dialog (to compare: Proteus's dialog, fields and defaults were not observable) |
| Help ▸ Overview | F1 | ✗ | Shown disabled: no command yet (item without a command) |
| Help ▸ About Protasena |  | ◐ | Our About window, in place of Proteus's product entries ("About Proteus 8"; "About Qt" is left out) |
| Help ▸ Schematic Capture Help | F1 | ✗ | Shown disabled: no command yet (item without a command) |
| Help ▸ Schematic Capture Tutorial |  | ✗ | Shown disabled: no command yet (item without a command) |
| Help ▸ Simulation Help |  | ✗ | Shown disabled: no command yet (item without a command) |
| Help ▸ VSM Model/SDK Help |  | ✗ | Shown disabled: no command yet (item without a command) |

## Toolbars

Buttons mirror the menu item of the same name.

| Toolbar ▸ button | Shortcut | Status | Notes |
|---|---|---|---|
| Toolbar band: Global, View, Edit, Design toolbars, left to right |  | ✅ | Order and grouping as observed; a narrow window wraps toolbars onto a second row (Proteus's rebar wraps too; its exact wrapping not compared) |
| Global Toolbar ▸ New Project |  | ◐ | Runs `new`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Global Toolbar ▸ Open Project |  | ◐ | Runs `open`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Global Toolbar ▸ Save Project |  | ◐ | Runs `save`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Global Toolbar ▸ Close Project |  | ✗ | Shown disabled: no command yet |
| Global Toolbar ▸ Home Page |  | ✗ | Shown disabled: no command yet |
| Global Toolbar ▸ Schematic Capture |  | ◐ | Shown pressed (the module in use); switching modules does not exist |
| Global Toolbar ▸ PCB Layout |  | ✗ | Shown disabled: no command yet |
| Global Toolbar ▸ 3D Visualizer |  | ✗ | Shown disabled: no command yet |
| Global Toolbar ▸ Gerber Viewer |  | ✗ | Shown disabled: no command yet |
| Global Toolbar ▸ Design Explorer |  | ✗ | Shown disabled: no command yet |
| Global Toolbar ▸ Bill of Materials |  | ◐ | Runs `bill-of-materials` and shows our own report window. Proteus opens a Bill of Materials module of its own (not observed): to compare |
| Global Toolbar ▸ Source Code |  | ✗ | Shown disabled: no command yet |
| Global Toolbar ▸ Project Notes |  | ✗ | Shown disabled: no command yet |
| Global Toolbar ▸ Help |  | ✗ | Shown disabled: no command yet |
| View Toolbar ▸ Redraw Display |  | ◐ | Runs `redraw`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| View Toolbar ▸ Toggle Grid |  | ◐ | Runs `toggle-grid`; shown pressed while on. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| View Toolbar ▸ Toggle False Origin |  | ◐ | Runs `toggle-false-origin`; shown pressed while on. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| View Toolbar ▸ Center At Cursor |  | ◐ | Runs `center-at-cursor`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| View Toolbar ▸ Zoom In |  | ◐ | Runs `zoom-in`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| View Toolbar ▸ Zoom Out |  | ◐ | Runs `zoom-out`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| View Toolbar ▸ Zoom To View Entire Sheet |  | ◐ | Runs `zoom-all`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| View Toolbar ▸ Zoom To Area |  | ◐ | As View ▸ Zoom To Area |
| Edit Toolbar ▸ Undo Changes |  | ◐ | Runs `undo`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Edit Toolbar ▸ Redo Changes |  | ◐ | Runs `redo`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Edit Toolbar ▸ Cut To Clipboard |  | ◐ | Runs `cut`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Edit Toolbar ▸ Copy To Clipboard |  | ◐ | Runs `copy`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Edit Toolbar ▸ Paste From Clipboard |  | ◐ | Runs `paste`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Edit Toolbar ▸ Block Copy |  | ◐ | Runs `block-copy`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Edit Toolbar ▸ Block Move |  | ◐ | Runs `block-move`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Edit Toolbar ▸ Block Rotate |  | ◐ | Runs `block-rotate`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Edit Toolbar ▸ Block Delete |  | ◐ | Runs `block-delete`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Edit Toolbar ▸ Pick Parts |  | ◐ | Runs `pick-device`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Edit Toolbar ▸ Make Device |  | ✗ | Shown disabled: no command yet |
| Edit Toolbar ▸ Packaging Tool |  | ✗ | Shown disabled: no command yet |
| Edit Toolbar ▸ Decompose |  | ✗ | Shown disabled: no command yet |
| Design Toolbar ▸ Wire Autorouter |  | ◐ | Runs `toggle-autorouter`; shown pressed while on. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Design Toolbar ▸ Search & Tag |  | ◐ | Runs `search-tag` (its dialog). Name inferred from the icon; behaviour not compared |
| Design Toolbar ▸ Property Assignment Tool |  | ◐ | Runs `property-assign` (its dialog). Name inferred from the icon; behaviour not compared |
| Design Toolbar ▸ New (Root) Sheet |  | ◐ | Runs `new-sheet`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Design Toolbar ▸ Remove/Delete Sheet |  | ◐ | Runs `remove-sheet`. Name inferred from the icon (tooltips were not readable); behaviour not compared |
| Design Toolbar ▸ Exit to Parent Sheet |  | ◐ | Runs `exit-to-parent` (enabled on a child sheet). Name inferred from the icon; behaviour not compared |
| Design Toolbar ▸ Electrical Rules Check |  | ◐ | Runs `erc`. Name inferred from the icon (tooltips were not readable); behaviour not compared |

## Mode toolbar

| Mode | Shortcut | Status | Notes |
|---|---|---|---|
| Mode toolbar: 22 modes top to bottom in three groups |  | ◐ | Order and groups as observed; names inferred from the icons (tooltips were not readable); a low window wraps the left toolbars into a second column (Proteus's rebar behaviour not compared) |
| Selection Mode |  | ◐ | Engine machine (set-mode `Selection Mode`); clicks, drags and defaults not compared. Object selector: DEVICES |
| Component Mode |  | ◐ | Engine machine (set-mode `Component Mode`); clicks, drags and defaults not compared. Object selector: DEVICES |
| Junction Dot Mode |  | ◐ | Engine machine (set-mode `Junction Dot Mode`); clicks, drags and defaults not compared. Object selector: none |
| Wire Label Mode |  | ◐ | Engine machine (set-mode `Wire Label Mode`); clicks, drags and defaults not compared. Object selector: none |
| Text Script Mode |  | ◐ | Engine machine (set-mode `Text Script Mode`); clicks, drags and defaults not compared. Object selector: none |
| Buses Mode |  | ◐ | Engine machine (set-mode `Buses Mode`); clicks, drags and defaults not compared. Object selector: none |
| Subcircuit Mode |  | ◐ | Engine machine (`subcircuit`): a box by two clicks or a drag, named the next free `SUBn`; a click on its left or right edge asks for a port (Subcircuit Port dialog, kind from the PORTS list; `subcircuit-port`); a click inside selects it, a double click opens Edit Subcircuit. Ports are drawn as module ports inside the box. **To compare** (Proteus not observed): clicks, port placement and shapes, default names. Object selector: PORTS |
| Terminals Mode |  | ◐ | Engine machine (set-mode `Terminals Mode`); clicks, drags and defaults not compared. Object selector: TERMINALS |
| Device Pins Mode |  | ✗ | Shown disabled: the engine has no machine for it yet. Object selector caption PINS (list empty) |
| Graph Mode |  | ✗ | Shown disabled: the engine has no machine for it yet. Object selector caption GRAPHS (list empty) |
| Active Popup Mode |  | ✗ | Shown disabled: the engine has no machine for it yet. Object selector caption none (list empty) |
| Generator Mode |  | ✗ | Shown disabled: the engine has no machine for it yet. Object selector caption GENERATORS (list empty) |
| Probe Mode |  | ✗ | Shown disabled: the engine has no machine for it yet. Object selector caption PROBES (list empty) |
| Virtual Instruments Mode |  | ✗ | Shown disabled: the engine has no machine for it yet. Object selector caption INSTRUMENTS (list empty) |
| 2D Graphics Line Mode |  | ◐ | Engine machine (`line`): two clicks or a press-drag-release give the ends, on the grid. **To compare** (Proteus not observed): clicks, rubber band, line style, defaults. Object selector: GRAPHICS (list empty) |
| 2D Graphics Box Mode |  | ◐ | Engine machine (`box`): two clicks or a press-drag-release give opposite corners. **To compare**: clicks, style, fill, defaults. Object selector: GRAPHICS (list empty) |
| 2D Graphics Circle Mode |  | ◐ | Engine machine (`circle`): centre, then a point on the circle (or a drag). **To compare**: clicks (centre or corner first), style, fill. Object selector: GRAPHICS (list empty) |
| 2D Graphics Arc Mode |  | ◐ | Engine machine (`arc`): three clicks (start, end, a point on the arc) or a drag from start to end and a click. **To compare**: Proteus's arc construction (it may draw quarter arcs), style. Object selector: GRAPHICS (list empty) |
| 2D Graphics Closed Path Mode |  | ◐ | Engine machine (`closed-path`): a click per corner, a double click or the right button closes it (three corners or more), Escape cancels. **To compare**: clicks, curved segments, fill. Object selector: GRAPHICS (list empty) |
| 2D Graphics Text Mode |  | ◐ | Engine machine (set-mode `2D Graphics Text Mode`); clicks, drags and defaults not compared. Object selector: GRAPHICS |
| 2D Graphics Symbols Mode |  | ✗ | Shown disabled: the engine has no machine for it yet. Object selector caption SYMBOLS (list empty) |
| 2D Graphics Markers Mode |  | ✗ | Shown disabled: the engine has no machine for it yet. Object selector caption MARKERS (list empty) |
| Orientation toolbar ▸ Rotate Clockwise |  | ◐ | Runs `rotate-cw` (the selection, else the next placement); behaviour not compared |
| Orientation toolbar ▸ Rotate Anti-clockwise |  | ◐ | Runs `rotate-ccw`; behaviour not compared |
| Orientation toolbar ▸ angle box |  | ◐ | Shows the next placement's angle (`0°`); typing an angle runs `set-orientation`; whether Proteus shows the selection's angle not compared |
| Orientation toolbar ▸ X-Mirror |  | ◐ | Runs `mirror-x`; shown pressed while the placement is mirrored |
| Orientation toolbar ▸ Y-Mirror |  | ◐ | Runs `mirror-y`; never shown pressed (a Y mirror is stored as an X mirror turned 180°) |

## Overview window, object selector, editing window, status bar

| Element | Behaviour | Status | Notes |
|---|---|---|---|
| Title bar | `<project> - Protasena - Schematic Capture` | ◐ | Proteus's pattern with our product name; whether Proteus marks unsaved changes not compared |
| Tab strip ▸ Schematic Capture | The module tab under the toolbars | ◐ | Place and label as observed; its close cross is drawn but does nothing; graph tabs (e.g. a graph opened as a tab) do not exist |
| Overview window | Whole sheet with the visible area marked; click or drag pans there | ◐ | Place as observed; click-to-pan and the drawing of the thumbnail not compared; its size is fixed (Proteus's can be resized) |
| Object selector ▸ P | Opens Pick Devices (`pick-device`) in device modes | ◐ | Enabled only with the DEVICES list; Pick Terminals and the other pickers are not built |
| Object selector ▸ L | Library manager | ✗ | Shown disabled: no library manager |
| Object selector ▸ caption | DEVICES, TERMINALS, PORTS, PINS, GRAPHS, GENERATORS, PROBES, INSTRUMENTS, SYMBOLS, MARKERS, GRAPHICS by mode | ◐ | The captions are the observed list names; which mode shows which caption is our guess (Selection Mode shows DEVICES; Junction Dot, Wire Label, Text Script, Buses and Active Popup show none) — to compare |
| Object selector ▸ DEVICES list | Devices picked into the design; a click chooses the one to place (`select-device`) | ◐ | Order is the order of picking; Proteus's order (alphabetical in the observed design) to compare |
| Object selector ▸ PORTS list | DEFAULT, INPUT, OUTPUT, BIDIR; a click chooses the kind of port (`select-port`) | ◐ | Proteus's list not observed |
| Object selector ▸ TERMINALS list | DEFAULT, INPUT, OUTPUT, BIDIR, POWER, GROUND, BUS; a click chooses the kind (`select-terminal`) | ◐ | Proteus's list (it may hold more kinds) not observed |
| Left pane width | Resizable | ◐ | Proteus's pane is about 125 px; ours starts at 190 points |
| Editing window ▸ sheet | Border of the sheet size, centred on the origin, paper inside | ◐ | Proteus draws its own border and margin; sizes and origin to compare |
| Editing window ▸ grid | Dots (or lines) at the snap step, coarsened to stay readable; Toggle Grid (G) hides it | ◐ | Whether G cycles dots/lines/off in Proteus to compare; the line style is a window setting (`ui.set {grid_style}`) |
| Editing window ▸ design | Components from their symbols (pins, names, numbers), reference and value, wires, junction dots, wire labels, terminals, buses, bus entries, texts | ◐ | Our own drawing; text sizes and positions to compare |
| Editing window ▸ selection | Selected objects drawn in the selection colour | ◐ | Proteus's highlight not compared |
| Editing window ▸ previews | Ghost of the part to place, the routed wire following the mouse, the bus, dragged objects, the selection box, a ring on pin ends where a wire can start | ◐ | Engine previews; Proteus's feedback (cursor shapes, ghost behaviour) not compared |
| Editing window ▸ zoom | F6/F7 by 2 about the centre, F8 whole sheet, the wheel by the display options' factor (2 by default) per notch about the cursor | ◐ | Proteus's wheel step and centring to compare |
| Editing window ▸ pan | Middle-button drag; F5 centres at the cursor | ◐ | Not compared |
| Editing window ▸ cursor | A small cross at the snapped point; X-Cursor (X) draws full-window lines | ◐ | Proteus's cursor shapes not compared |
| Editing window ▸ Goto Child Sheet | Ctrl+C with a subcircuit selected (`enter-child-sheet`; also Edit Subcircuit's button): shows its child sheet, made empty and named after the subcircuit the first time | ◐ | No menu item (right-click context menus, where Proteus may offer it, are not built); key and behaviour to compare |
| Editing window ▸ messages | Prompts and refusals as a short message at the bottom left | ◐ | Our own; Proteus reports these differently (not observed) |
| Status bar: animation buttons, messages, design variant, sheet, coordinates, unit, left to right |  | ✅ | Order as observed |
| Status bar ▸ Play, Step, Pause, Stop | Shown disabled | ✗ | No simulation yet |
| Status bar ▸ messages | `No Messages` or the count of session messages; a click lists them | ◐ | Proteus counts simulation/ERC messages; its list window not observed |
| Status bar ▸ Base Design | Design variant box, shown disabled | ✗ | Variants are not built |
| Status bar ▸ sheet | The current sheet's name (`Root sheet 1`) | ◐ | Whether Proteus's field is clickable not observed |
| Status bar ▸ coordinates | `x: +1100.0   y: -1700.0`: the snapped cursor, from the false origin when one is set | ◐ | Format as observed; whether Proteus shows the snapped or the raw point to compare |
| Status bar ▸ unit | `th` | ◐ | Fixed: other unit systems are not built |
| Interface theme | Light (default) or dark, our own palettes | ◐ | Chosen with `--theme`, `PROTASENA_THEME` or `ui.set`; no menu item (Proteus has no theme choice; a menu place would need the owner's approval) |

## Dialogs

Our own layouts following each dialog's purpose; Proteus's dialogs were not observable, so every field set, order and default is to compare.

| Dialog ▸ field | Default | Status | Notes |
|---|---|---|---|
| Pick Devices ▸ Keywords | empty, focused | ◐ | Every word must occur in the name, description or keywords; exact and leading name matches first. Proteus's dialog was not observable: to compare |
| Pick Devices ▸ Category / Sub-category / Manufacturer lists | all | ◐ | With device counts; each list counts what the lists to its left let through. To compare |
| Pick Devices ▸ results | nothing until keywords or a category | ◐ | Device, Library, Description columns; Proteus's column set (it has further, optional columns) to compare |
| Pick Devices ▸ symbol preview | first gate | ◐ | Drawn from the render crate |
| Pick Devices ▸ PCB preview | the default package | ◐ | Pads and silkscreen only |
| Pick Devices ▸ OK / double click | | ◐ | OK picks and closes, a double click picks and keeps the dialog open (both run `pick-device`); to compare |
| Edit Component ▸ Part Reference, Part Value (each with Hidden) | the part's | ◐ | Runs `edit-component`; field set and order to compare |
| Edit Component ▸ Element | first gate | ◐ | Only for multi-gate devices |
| Edit Component ▸ PCB Package | the part's | ◐ | The device's packages or none |
| Edit Component ▸ Other Properties | the device's property definitions, then System ▸ Set Property Definitions' (with their defaults) | ◐ | A grid of values; an empty value removes the property |
| Find/Edit Component (nothing selected) | | ◐ | Asks for the reference, then opens Edit Component |
| Edit Wire Label ▸ String | empty | ◐ | Runs `wire-label` (new) or `edit-item`; Proteus's tabs and options not observed |
| Edit Wire Label ▸ Global | off | ◐ | Our own checkbox: a global label's name is not prefixed by the instance path in child sheets. How Proteus marks global labels to compare |
| Subcircuit Port ▸ Port Name, Type | empty; the PORTS list's kind | ◐ | Our own dialog after a click on a subcircuit's edge; runs `subcircuit-port`. To compare |
| Edit Subcircuit ▸ Name, Ports (name, type, Remove), Goto/Create Child Sheet | the subcircuit's | ◐ | Our own dialog (double click); runs `edit-subcircuit`, or `enter-child-sheet` from the button. Proteus's fields (its Circuit name) to compare |
| Edit Terminal ▸ String, Type | the terminal's | ◐ | Runs `edit-item` |
| Text (Text Script, 2D text) ▸ Text, Height | empty, default height | ◐ | Runs `text` or `edit-item`; the editor's font follows System ▸ Set Text Editor |
| Edit Design Properties | the project's | ◐ | Project Name, Design Title, Doc. No, Revision, Author, Created, Modified; to compare |
| Edit Sheet Properties | the sheet's | ◐ | Sheet Name and Sheet Size (the sizes of System ▸ Set Sheet Sizes, A4…A0 and A…E by default); Proteus's size list not observed |
| Goto Sheet | the current sheet | ◐ | Numbered sheet list in hierarchy order, sub-sheets indented with their instance path |
| Global Annotator | Incremental | ◐ | Incremental or Total; Proteus's options not observed |
| Template dialogs (Set Design Colours, Graphic Styles, Text Styles, 2D Graphics Defaults, Junction Dot Style) | our defaults (nothing set) | ◐ | Our own grids of widths, heights (empty: our default) and colours ("Own" with a colour button; off: the theme's colour); OK runs the command with the whole group. Template colours show in the editing window and the overview; exports and prints keep our print palette (widths, heights, fills and dot style are drawn there). To compare |
| Set Keyboard Mapping ▸ Find, command list, key, Restore Defaults | the keys in use | ◐ | Our own table; a key is a letter, digit, F1–F24, Page-Up/Down, Delete, Space or Enter with Ctrl, Alt, Shift; Escape is kept for cancelling. To compare |
| Set Property Definitions / Set Sheet Sizes ▸ rows, Add, Remove | the settings in use | ◐ | Our own editable lists. To compare |
| System Settings, Set Display Options, Set Text Editor | the settings in use | ◐ | Our own forms. To compare |
| Restore Default Settings | | ◐ | Our own question with OK and Cancel. To compare |
| Bill of Materials window ▸ table, Export CSV..., Export HTML... | every part of the design | ◐ | Our own window: Quantity, References (natural order), Value, Device, Package, Description, one line per value, device and package; power symbols left out, gates counted once, unannotated parts one by one, all sheets; the exports go through the file chooser (CSV per RFC 4180, a plain HTML page). Proteus's report (categories, columns, templates) not observed: to compare |
| Unsaved changes (New, Open, Exit, window close) | | ◐ | Our own question "The design has unsaved changes. Save them?" with Save (saves, asking for a file when the project has none, and carries on once saved), Don't Save, Cancel; Proteus's prompt (wording, buttons, order, default) to compare |
| Open Project, Save Project As, Netlist Compiler file | | ◐ | The system's file chooser (`.psproj`, `.pdsprj` opens by import); a path prompt when there is none. Proteus's Netlist Compiler options not observed |
| Electrical Rules Check result | | ◐ | A window listing the findings; Proteus's report window not observed |
| Netlist Compiler result | | ◐ | File, nets, parts and findings |
| Export Graphics (Bitmap, DXF, EPS, PDF, SVG) ▸ Format, Scope, Mono, Resolution or Scale, File | PNG (Bitmap); Current Sheet; colour; 200 dpi or 1; the project file with the format's extension | ◐ | Our own dialog; All Sheets only for PDF; Browse... opens the system's file chooser. Proteus's export dialogs (fields, defaults, output area) to compare |
| Print Design ▸ Scope, Mono, Open in the system viewer, File | All Sheets; colour; on when the window can open files; `<project>-print.pdf` in the system's temporary directory | ◐ | Our own dialog; the system viewer is `xdg-open`, `open` or `explorer`, and a failure to start it is reported. Proteus's print dialog (printer, copies, scale, area) to compare |
| Search & Tag ▸ Property, Operator, String, Search Mode, Match Case | REF; Equals; empty; New Search; off | ◐ | Our own dialog; Proteus's field set and defaults to compare |
| Property Assignment Tool ▸ String, Action, Apply To, Count, Increment | empty; Assign; the tagged parts (the sheet when nothing is tagged); 1; 1 | ◐ | Our own dialog; Proteus's actions and Apply To choices to compare |
| Align Objects ▸ Left Edges, Horizontal Centres, Right Edges, Top Edges, Vertical Centres, Bottom Edges | Left Edges | ◐ | Our own dialog; Proteus's choices to compare |

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-20 | major | First version: empty skeleton, sections for the Schematic Capture layout |
| 2026-10-20 | major | Rows for every menu item, toolbar button, mode, pane element, status field and dialog of the M1 Task 9 window; 3 of 253 rows match (layout order), behaviour not yet compared |
| 2026-10-20 | minor | Exit Application and the window's close button ask Save / Don't Save / Cancel with unsaved changes, as New and Open do (to compare) |
| 2026-10-20 | minor | 2D Graphics Line, Box, Circle, Arc and Closed Path modes built the conventional way (to compare) |
| 2026-10-20 | minor | File ▸ Export Graphics: Export Bitmap (PNG, BMP), DXF, Encapsulated Postscript, Adobe PDF and SVG built the conventional way with their dialog (to compare); Metafile and Vector File not built |
| 2026-10-20 | minor | File ▸ Print Design prints to a PDF file and can open it in the system viewer (to compare); Print Setup and Printer Information not built |
| 2026-10-20 | minor | Tool ▸ Search & Tag (T) and Property Assignment Tool (A), with their Design Toolbar buttons and dialogs, built the conventional way (to compare) |
| 2026-10-20 | minor | Edit ▸ Align Objects (Ctrl+A), Send To Back (Ctrl+B), Bring To Front (Ctrl+F) and Tidy Design built the conventional way (to compare) |
| 2026-10-20 | minor | Subcircuit Mode, the PORTS list, Subcircuit Port and Edit Subcircuit dialogs, Goto Child Sheet (Ctrl+C), Design ▸ Exit to Parent Sheet (Ctrl+X) and its toolbar button, sheets in hierarchy order; Edit Wire Label's Global box. Built the conventional way (to compare); 3 of 263 rows match |
| 2026-10-20 | minor | Template ▸ Set Design Colours, Graphic Styles, Text Styles, 2D Graphics Defaults, Junction Dot Style and System ▸ System Settings, Set Display Options, Set Keyboard Mapping, Set Property Definitions, Set Sheet Sizes, Set Text Editor, Restore Default Settings built the conventional way with their dialogs (to compare); 3 of 268 rows match |
| 2026-10-20 | minor | Global Toolbar ▸ Bill of Materials with our own report window and CSV/HTML export (to compare); 3 of 269 rows match |
