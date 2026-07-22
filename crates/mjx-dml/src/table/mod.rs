//! DrawingML tables: `a:tbl` (`CT_Table`) — the grid a `p:graphicFrame` frames.
//!
//! A table is one of the graphical objects a frame can hold, and structurally it is the simplest
//! thing in DrawingML that is genuinely two-dimensional:
//!
//! ```xml
//! <a:tbl>
//!   <a:tblPr firstRow="1" bandRow="1"/>
//!   <a:tblGrid><a:gridCol w="3048000"/></a:tblGrid>
//!   <a:tr h="370840">
//!     <a:tc><a:txBody>…</a:txBody><a:tcPr/></a:tc>
//!   </a:tr>
//! </a:tbl>
//! ```
//!
//! # How little of this is new
//!
//! A cell's content is a `CT_TextBody` — the *same* type a shape's `p:txBody` is — so the whole
//! [text tree](crate::text) and its formatting model apply inside a cell unchanged. A cell's borders
//! are six [`LineProperties`](crate::line::LineProperties), the type an outline already uses; a
//! cell's fill is the same `EG_FillProperties` group as a shape's. Widths, heights and margins are
//! [`Emu`](crate::geometry::Emu).
//!
//! What is actually new is the **shape** of the thing: a grid whose column count is declared once in
//! `a:tblGrid` and repeated implicitly by every row's cells.
//!
//! # The grid stays rectangular
//!
//! Merging does not remove cells. A merged region is anchored at its top-left cell, which carries
//! `gridSpan` (columns) and `rowSpan` (rows), and **every covered cell remains present** as an
//! `a:tc` carrying `hMerge` or `vMerge`. So a row always holds exactly as many `a:tc` as the grid has
//! `a:gridCol`, and `(row, column)` addressing never has holes — a covered cell is addressable and
//! can name the anchor covering it.
//!
//! # Fidelity
//!
//! The structural containers ([`Table`], [`TableGrid`], [`TableRow`], [`TableCell`]) keep an ordered
//! `content` list whose variants are the typed children plus a `Raw` catch-all, exactly as the text
//! tree does. The property bags ([`TableProperties`], [`TableCellProperties`]) keep their children
//! opaque and expose typed accessors, exactly as `a:ln` does. Either way an `extLst`, a `cell3D`, an
//! MCE bucket or an unknown attribute round-trips byte-for-byte.
//!
//! A table's style (`a:tableStyle` / `a:tableStyleId`, and the `tableStyles.xml` part the latter
//! names) is **preserved but not yet modeled** — it is its own piece of work.

mod cell;
mod grid;
mod properties;
mod row;
#[allow(clippy::module_inception)]
mod table;

pub use cell::{
    CellBorder, TableCell, TableCellContent, TableCellProperties, TextAnchoring, TextDirection,
    TextHorizontalOverflow,
};
pub use grid::{TableColumn, TableGrid, TableGridContent};
pub use properties::{TablePart, TableProperties};
pub use row::{TableRow, TableRowContent};
pub use table::{Table, TableContent};

/// The index in an interleaved content list of the `nth` (0-based) element matching `is_target`, or
/// `None` when there are fewer than `nth + 1` of them.
///
/// A structural container keeps its typed children interleaved with opaque nodes (whitespace, an
/// `extLst`, an unknown element), so the `nth` *row* / *column* / *cell* is not at content index
/// `nth` — this walks past the opaque nodes to find it.
pub(super) fn nth_typed_index<T>(
    content: &[T],
    nth: usize,
    is_target: impl Fn(&T) -> bool,
) -> Option<usize> {
    content
        .iter()
        .enumerate()
        .filter_map(|(index, item)| is_target(item).then_some(index))
        .nth(nth)
}

/// The content-list index at which to insert so a new element becomes the `nth` (0-based) one
/// matching `is_target`: at the current `nth` match (pushing it and everything after it right), or
/// right after the last match when `nth` equals the count (appending). Keeps the new element beside
/// its typed siblings rather than at a blind index that could land it among leading properties.
pub(super) fn typed_insert_index<T>(
    content: &[T],
    nth: usize,
    is_target: impl Fn(&T) -> bool,
) -> usize {
    if let Some(index) = nth_typed_index(content, nth, &is_target) {
        return index;
    }
    content
        .iter()
        .enumerate()
        .filter_map(|(index, item)| is_target(item).then_some(index))
        .next_back()
        .map_or(content.len(), |last| last + 1)
}
