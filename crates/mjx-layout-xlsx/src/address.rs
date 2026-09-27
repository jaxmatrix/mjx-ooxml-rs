//! The addressing scheme — how a fragment says which cell it came from, and how a point on a page
//! turns back into that.
//!
//! # This scheme is not new, and that matters more than its details
//!
//! A [`SourceRef`] means nothing without the box model that issued it, so a box model that invents
//! its own numbering has made every consumer above it depend on *this* crate. `mjx-session`
//! (MJXOFF-167) already wrote a workbook's addressing down — `SpreadsheetSession`'s doc comment is
//! the contract — and it did so before any box model existed, because the *editing* side needed it
//! first. So this crate adopts that scheme exactly rather than defining a second one:
//!
//! | Piece | Meaning |
//! |---|---|
//! | [`PartId`] | which sheet, in the order the workbook lists its tabs |
//! | path segment `0` | the zero-based row |
//! | path segment `1` | the zero-based column |
//!
//! So `[6, 2]` under part `0` is `C7` on the first tab, which is exactly what
//! `mjx-session` journals a `SetText` against.
//!
//! # Every fragment of a cell shares the cell's path
//!
//! A cell has no paragraphs and no runs, so — unlike PowerPoint's, where a line's path names the
//! paragraph and a glyph run's names the run — the path stops at two segments for **every** fragment
//! a cell produces: its box, its lines and its glyph runs. What distinguishes them is the character
//! range, which is in the cell's own display text throughout. A hit test therefore lands on
//! `(row, column, byte offset)`, which is the triple an editor needs and the one
//! `SpreadsheetSession::cell_value` already speaks.
//!
//! That is simpler than PowerPoint's and it is not a simplification: it is what the document model
//! is. A cell's text is one string.
//!
//! # A pane has no document node
//!
//! A frozen or split pane is a *view* construct — `sheetView/pane` describes where the window is
//! divided, not what the sheet contains — so the table fragment for each pane region addresses the
//! sheet itself, at the root path. Several pane regions therefore share one address, deliberately: a
//! hit test resolves to a cell, and the tables above it are structure rather than content.

use mjx_layout::{PartId, SourcePath, SourceRef};
use mjx_sml::{AddressError, CellReference};

/// The part a sheet's fragments are addressed under: its position in the workbook's tab strip.
#[must_use]
pub fn part_of(sheet_index: usize) -> PartId {
    PartId::new(clamp(sheet_index))
}

/// Which tab a part number names.
#[must_use]
pub fn sheet_of(part: PartId) -> usize {
    part.number() as usize
}

/// A cell's path: its zero-based row, then its zero-based column.
#[must_use]
pub fn cell_path(row: u32, column: u16) -> SourcePath {
    SourcePath::new(&[row, u32::from(column)])
}

/// The first segment of a drawing's path, which is not a row number.
///
/// A cell's path is `[row, column]` and a sheet has 1,048,576 rows, so `u32::MAX` cannot collide
/// with one. It has to not collide: [`crate::model::SheetBoxModel`]'s `invalidate` reads the first
/// segment of a changed node's path **as a row**, and a drawing that looked like row 4 would make an
/// edit to a picture invalidate the fourth band of the grid.
pub const DRAWING: u32 = u32::MAX;

/// A drawing's path: the [`DRAWING`] sentinel, then the anchor's position in the drawing part.
#[must_use]
pub fn drawing_path(index: usize) -> SourcePath {
    SourcePath::new(&[DRAWING, clamp(index)])
}

/// The sheet's own path — the empty one, which the page box and every pane's table carry.
#[must_use]
pub fn sheet_path() -> SourcePath {
    SourcePath::root()
}

/// A whole node, covering no characters — the page box, a pane's table, a cell's box.
#[must_use]
pub fn node(part: PartId, path: SourcePath) -> SourceRef {
    SourceRef::node(part, path)
}

/// An address covering `bytes` of the cell's display text.
#[must_use]
pub fn span(part: PartId, path: SourcePath, bytes: std::ops::Range<usize>) -> SourceRef {
    SourceRef::new(part, path, clamp(bytes.start)..clamp(bytes.end))
}

/// A `usize` from a document as the `u32` a path segment is, saturating rather than wrapping.
///
/// A wrap would make two different cells share an address, which is worse than a cell at `u32::MAX`
/// in a workbook that cannot exist.
fn clamp(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// What a hit test found: which sheet, which cell, and — when the point landed on text — where in
/// the cell's own display text a caret would go.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CellHit {
    /// Which tab.
    pub sheet: usize,
    /// The zero-based row.
    pub row: u32,
    /// The zero-based column.
    pub column: u16,
    /// The byte offset in the cell's display text, when the point landed on a glyph run.
    pub offset: Option<usize>,
}

impl CellHit {
    /// The address split out of a fragment's [`SourceRef`], or `None` for a fragment that names no
    /// cell — the page box, or a pane's table.
    ///
    /// Unlike PowerPoint's, this needs no depth hint: a cell's path is two segments and a
    /// structural fragment's is none, so the two cannot be confused.
    #[must_use]
    pub fn from_source(source: &SourceRef) -> Option<Self> {
        let segments = source.path().segments();
        let [row, column] = segments else {
            return None;
        };
        let characters = source.characters();
        Some(Self {
            sheet: sheet_of(source.part()),
            row: *row,
            column: u16::try_from(*column).unwrap_or(u16::MAX),
            // A node covers no characters, and that is exactly how a cell's *box* is distinguished
            // from the glyph run inside it: an empty range is the box.
            offset: (characters.end > characters.start).then_some(characters.start as usize),
        })
    }

    /// This hit as the [`CellReference`] every `mjx-sml` and `mjx-xlsx` call takes.
    ///
    /// # Errors
    /// [`AddressError`] when the row or column is outside the grid, which a hit on a laid-out
    /// fragment never is.
    pub fn reference(&self) -> Result<CellReference, AddressError> {
        CellReference::relative(self.column, self.row)
    }
}
