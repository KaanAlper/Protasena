//! Dialogs the engine asks the UI to show. The engine never opens a dialog itself: a menu
//! pick, key or click that needs one returns a [`Request`]; the UI shows the dialog and then
//! runs the command's JSON form with the values entered.

use protasena_doc::{ComponentId, LabelId, SheetId, SubcircuitId, TerminalId, TextId};
use protasena_geom::Pt;
use serde::Serialize;

/// A dialog to show, with what it is about.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "dialog", rename_all = "kebab-case")]
pub enum Request {
    /// Pick Devices (then `pick-device` with `pick`).
    PickDevices,
    /// Find and edit a component by reference (then `edit-component`).
    FindComponent,
    /// Edit Component for one part (then `edit-component`).
    EditComponent {
        /// The component.
        id: ComponentId,
    },
    /// Edit Wire Label for a new label on a wire (then `wire-label`).
    WireLabel {
        /// The point on the wire.
        at: Pt,
    },
    /// Edit Wire Label for an existing label (then `edit-item`).
    EditLabel {
        /// The label.
        id: LabelId,
    },
    /// Edit Terminal (then `edit-item`).
    EditTerminal {
        /// The terminal.
        id: TerminalId,
    },
    /// Name a new port on a subcircuit's edge (then `subcircuit-port`).
    SubcircuitPort {
        /// The subcircuit.
        subcircuit: SubcircuitId,
        /// The point on its edge.
        at: Pt,
    },
    /// Edit Subcircuit: its name and ports (then `edit-subcircuit`, or `enter-child-sheet`).
    EditSubcircuit {
        /// The subcircuit.
        id: SubcircuitId,
    },
    /// Enter the text of a new text or script (then `text`).
    Text {
        /// Where it goes.
        at: Pt,
    },
    /// Edit an existing text (then `edit-item`).
    EditText {
        /// The text.
        id: TextId,
    },
    /// Choose a project file to open (then `open`).
    OpenProject,
    /// Choose where to save (then `save-as`).
    SaveProjectAs,
    /// Netlist Compiler options and file (then `export-netlist`).
    ExportNetlist,
    /// Export Graphics options and file (then the `export-*` command of the format).
    ExportGraphics {
        /// The format the menu item writes (`png` for Export Bitmap, which also offers `bmp`).
        format: &'static str,
    },
    /// Print Design: what to print and the PDF file (then `print-design`).
    PrintDesign,
    /// Edit Design Properties (then `design-properties`).
    DesignProperties,
    /// Edit Sheet Properties (then `sheet-properties`).
    SheetProperties {
        /// The sheet.
        sheet: SheetId,
    },
    /// Choose a sheet (then `goto-sheet`).
    GotoSheet,
    /// Global Annotator options (then `annotate`).
    GlobalAnnotator,
    /// Align Objects: which edge or centre to line up (then `align`).
    Align,
    /// Search & Tag: the property, operator and value to tag by (then `search-tag`).
    SearchTag,
    /// Property Assignment Tool: the string, action and parts (then `property-assign`).
    PropertyAssign,
    /// Unsaved changes: ask whether to discard them (then the command with `discard: true`).
    ConfirmDiscard {
        /// The command waiting for the answer.
        then: &'static str,
    },
}
