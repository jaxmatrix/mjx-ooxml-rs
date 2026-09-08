//! What laying a worksheet out can fail with.
//!
//! Three sources, kept apart on purpose: the shared machinery's ([`LayoutError`] — a foreign
//! checkpoint, a page past the end of the sheet), the font engine's ([`FontError`] — a face that
//! will not shape), and the workbook's ([`XlsxError`] — a malformed package, a `col` with no `@min`,
//! a style index naming no `xf`).
//!
//! # Nothing here is a panic
//!
//! A worksheet comes from an untrusted file. A `col` run whose `@max` is below its `@min`, a row
//! height of `NaN`, a merge that reaches outside the grid, a `sheetFormatPr` with no
//! `defaultRowHeight`, a pane frozen at column 40,000 — every one of them produces a page that looks
//! wrong rather than a crash, and `tests/no_panic_on_a_layout_path.rs` holds that by scanning the
//! source rather than by assertion.

use mjx_layout::LayoutError;
use mjx_sml::SmlError;
use mjx_text::FontError;
use mjx_xlsx::XlsxError;
use thiserror::Error;

/// A failure laying out a worksheet.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SheetLayoutError {
    /// The shared layout machinery refused something — a checkpoint from another box model, a
    /// checkpoint for another page, a page past the last band of rows, an empty content area.
    #[error(transparent)]
    Layout(#[from] LayoutError),

    /// A face would not shape.
    #[error(transparent)]
    Text(#[from] FontError),

    /// The worksheet's own markup would not answer — a `col` missing `@min` or `@max`, a
    /// `mergeCell@ref` that will not parse, a style index naming no record in `cellXfs`.
    ///
    /// Separate from [`SheetLayoutError::Workbook`] because the two come from different tiers:
    /// `mjx-sml` reports what the *markup* got wrong and `mjx-xlsx` what the *package* did, and a
    /// caller that wants to tell a malformed `styles.xml` from a missing part can.
    #[error(transparent)]
    Sml(#[from] SmlError),

    /// The workbook would not answer — a malformed part, a tab index out of range, a `col` missing
    /// a required bound, a style index naming no record in `cellXfs`.
    #[error(transparent)]
    Workbook(#[from] XlsxError),

    /// The workbook has no tab at the index a snapshot was asked for.
    #[error("the workbook has no sheet {requested}; it has {count}")]
    NoSuchSheet {
        /// The index asked for.
        requested: usize,
        /// How many tabs the workbook lists.
        count: usize,
    },

    /// The tab at this index reaches no worksheet part, or reaches one with no styles part beside
    /// it.
    ///
    /// A chartsheet, a dialogsheet and a macrosheet all answer this: they are sheets in the tab
    /// strip and none of them has a cell grid, so there is nothing here to lay out.
    #[error("sheet {index} has no worksheet grid to lay out")]
    NotAWorksheet {
        /// Which tab.
        index: usize,
    },

    /// A continuation carried something other than the eight bytes this box model writes.
    ///
    /// Reachable only by handing this model a checkpoint whose signature matches and whose state
    /// does not, which is what the perturbation test does deliberately.
    #[error(
        "a continuation carried {0} bytes; this box model writes exactly eight — the row the next \
         band starts at, and the row the sheet ends at"
    )]
    MalformedContinuation(usize),
}
