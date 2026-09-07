//! [`Workbook`] — the binding-shaped Excel surface, and the one place in this facade where the
//! shape of an API is a performance decision rather than a taste decision.
//!
//! A `Workbook` is an [`mjx_xlsx::Workbook`] with its Rust ergonomics traded for portability,
//! mirroring [`crate::Deck`]'s own relationship to [`mjx_pptx::Presentation`] and
//! [`crate::Document`]'s to [`mjx_docx::Document`]:
//!
//! | `mjx_xlsx::Workbook`      | `mjx_ooxml::Workbook`  | why |
//! |---------------------------|-------------------------|-----|
//! | `mjx_sml::CellReference`  | `&str` — `"B7"`         | an eight-byte address a binding would have to wrap in a class, for a value every spreadsheet user already spells |
//! | `mjx_sml::CellRange`      | `&str` — `"A1:C3"`      | likewise |
//! | `mjx_sml::CellValue<'_>`  | [`CellInput`] / [`CellData`] | a borrowed enum cannot outlive the call in a binding |
//! | `usize`                   | `u32`                   | one width on every target, host-independent |
//! | `&PartName`               | `&str`                  | a validated handle cannot cross the boundary and come back |
//! | `impl FnOnce(&T, &Interner) -> R` | a concrete return type | neither PyO3 nor wasm-bindgen can accept a Rust closure argument |
//! | `Result<_, XlsxError>`    | [`Result<_, Error>`](crate::Error) | eleven variants, and the forty-two below them, collapse to eleven codes |
//!
//! # ⚠ Why there is no per-cell reader or writer here
//!
//! **This is the deliberate difference from every other surface in this crate, and the reason is
//! measured rather than aesthetic.**
//!
//! `mjx_xlsx::Workbook` holds no parsed worksheet — reading one never dirties the package, and there
//! is nothing to invalidate — so *every* per-sheet accessor parses that sheet's part again. On the
//! 300,000-cell workbook `docs/BENCHMARKS.md` describes, in a release build, that is **387 ms to
//! read one cell** and **485 ms to write one**, against 14.8 ms to open the whole file; a loop that
//! sets 4,000 cells one at a time takes **18.39 s** where one read, 4,000 edits and one write take
//! **1.12 ms**. The ratio is 16,427× and it grows with the square of the cell count. All of that is
//! written down in [*Large workbooks*](mjx_xlsx::guide::large_workbooks), which also gives the Rust
//! answer: hold the [`mjx_sml::WorksheetPart`] yourself between one
//! [`worksheet_markup`](mjx_xlsx::Workbook::worksheet_markup) and one
//! [`write_worksheet_markup`](mjx_xlsx::Workbook::write_worksheet_markup).
//!
//! **That answer does not cross a foreign function boundary.** `WorksheetPart` is an
//! interner-bound model; a binding cannot hand it out, and a Python or JavaScript caller has no
//! escape hatch to reach it with. So a facade that exposed `cell_value(sheet, "A1")` would be
//! shipping the 387 ms-per-cell loop as the natural idiom, in the one place a caller *cannot* reach
//! past it. A call four orders of magnitude off its own alternative, named as though it were
//! ordinary, is a defect in the API rather than in the caller.
//!
//! So the cell door here is a **range**, in both directions:
//!
//! * [`Workbook::read_range`] parses the sheet **once** and answers with an owned [`CellBlock`] of
//!   plain values.
//! * [`Workbook::write_cells`] parses **once**, applies every edit to the model, and serializes
//!   **once**.
//!
//! Nothing is lost for the one-cell case: `read_range(0, "A1")` is one call and one parse, exactly
//! what a per-cell reader would have cost. What is lost is the *shape* that invites the loop.
//! `crates/mjx-ooxml/tests/workbook_boundary.rs` holds this to an allocation bound rather than to a
//! stopwatch, so the guarantee is checked deterministically on every machine.
//!
//! Rust callers who want the per-cell calls anyway still have them, in full, one layer down through
//! [`Workbook::workbook_mut`] — the same escape hatch [`crate::Deck::presentation_mut`] is.
//!
//! # What is left to `mjx_xlsx::Workbook`, and why
//!
//! `mjx_xlsx::Workbook` carries a surface several times this one's size. This facade selects the
//! ones a caller opening, reading, editing and saving a workbook needs, and leaves the rest
//! reachable through [`Workbook::workbook_mut`]:
//!
//! - **The closure-taking markup doors** (`workbook_markup`, `edit_workbook_markup`,
//!   `calculation_chain`, `table_markup`, `edit_table_markup`, `conditional_rules_for`,
//!   `auto_filter`, `data_validations`, `comments_markup`, `edit_comments_markup`,
//!   `drawing_markup`, `edit_drawing_markup`, `vml_drawing_markup`, `edit_vml_drawing_markup`, the
//!   three `with_vml_shape_for_*`) — every one takes a closure over an interner-bound reference,
//!   which is the one shape a foreign function boundary cannot carry. Where a concrete answer
//!   exists, this facade builds it by calling the closure-taking method *internally* (that is what
//!   [`Workbook::auto_filter_range`] and [`Workbook::data_validation_ranges`] are).
//! - **The interner-bound models** (`worksheet_markup`, `write_worksheet_markup`,
//!   `worksheet_markup_of`, `sheet_markup`, `sheet_markup_of`, `write_sheet_markup`,
//!   `styles_markup`, `shared_strings`, `sheet_formatting`) — these take no closure and borrow
//!   nothing; each hands back an owned `mjx-sml` model whose every string is an index into an
//!   interner that stays behind, so the value is meaningless on the far side of a boundary. **This
//!   group used to be listed above as closure doors, which none of them is** (MJXOFF-214). A Rust
//!   caller reaching them through [`Workbook::workbook_mut`] is the per-sheet editing loop
//!   [*Large workbooks*](mjx_xlsx::guide::large_workbooks) recommends.
//! - **The borrowed views** (`worksheet`, `worksheet_by_name`, `sheet_by_name`, `visible_sheets`,
//!   `parts`, `part_inventory`, `conditional_cell_format`, `package`) — each hands back a value
//!   borrowing the workbook for a caller-controlled lifetime, which neither PyO3 nor wasm-bindgen
//!   can express. What they answer is here as owned data: [`Workbook::sheets`],
//!   [`Workbook::effective_cell_format`], [`Workbook::part_names`].
//! - **The authoring vocabularies for conditional formatting, autofilters, data validation and
//!   worksheet tables** (`add_conditional_formatting`, `append_differential_format`,
//!   `set_auto_filter`, `add_data_validation`, `add_table`) — each takes an `mjx-sml` spec *tree*
//!   (`ConditionalRuleSpec` alone reaches `ConditionalRuleSpecKind`, `ConditionalValueObjectSpec`,
//!   `ColorScaleSpec`, `DataBarSpec` and `IconSetSpec`; `AutoFilterSpec` reaches `FilterColumnSpec`,
//!   `FilterSpecKind`, `CustomFilterSpec`, `SortStateSpec` and `SortConditionSpec`). Projecting five
//!   builder trees is a surface of its own, not a paragraph of this one; **what each of them
//!   *produces* is readable here** — [`Workbook::auto_filter_range`],
//!   [`Workbook::data_validation_ranges`], [`Workbook::conditional_formatting_ranges`] and
//!   [`Workbook::sheet_tables`] all report what a sheet actually holds. The cell-format vocabulary
//!   *is* projected, because it is four flat structs rather than a tree, and because a caller who
//!   cannot make a style index cannot use [`Workbook::set_cell_style`].
//! - **`from_package`** — it takes an `mjx_opc::Package`, which this facade seals for the reason
//!   [`crate::Deck::presentation_mut`]'s own documentation gives.
//! - **`blank_with_properties`** — it takes an `mjx_opc::doc_props::CoreProperties` and an
//!   `ExtendedProperties`, so an authored workbook can carry a title, a creator and a created time.
//!   Nothing here sets them, and neither binding can — the one entry on this list that is a **gap
//!   rather than a decision**, and [`crate::Deck`] and [`crate::Document`] have exactly the same one.
//!
//! **Six calls are renames rather than omissions**, and are here under a name that says which
//! subject they belong to: `add_comment`, `comment_at`, `set_comment_text` and `remove_comment` are
//! [`Workbook::add_cell_comment`], [`Workbook::cell_comment`],
//! [`Workbook::set_cell_comment_text`] and [`Workbook::remove_cell_comment`]; `set_cell_hyperlink`
//! is split into [`Workbook::set_cell_hyperlink_url`] and
//! [`Workbook::set_cell_hyperlink_location`], because an external target and an internal one are
//! two different records in the file; and `sheet_index_by_name` is [`Workbook::sheet_index`].
//!
//! **This list is checked, not trusted**, the same way [`crate::deck`]'s and [`crate::document`]'s
//! are: `xtask/tests/facade_curation.rs` holds it to the real difference between the two surfaces
//! in both directions. It replaced a sentence here that said *roughly seventy public methods* when
//! `mjx_xlsx::Workbook` had **165** (MJXOFF-214).
//!
//! # One workbook, one thread
//!
//! Same discipline as [`crate::Deck`] and [`crate::Document`]: nothing here hands back a view into
//! the workbook, and nothing here takes a callback, so a second borrow can never be live. Share a
//! workbook between threads by moving it, not by aliasing it.

use crate::error::{Error, ErrorCode};
use crate::format::{format_of, Format};

mod cells;
mod charts;
mod comments;
mod drawings;
mod features;
mod grid;
mod hyperlinks;
mod names;
mod paths;
mod preserved;
mod print;
mod sheets;
mod styles;
mod tables;

pub use cells::{CellBlock, CellData, CellInput, CellWrite};
pub use charts::{
    ChartRangeSeries, ChartSeriesFreshnessInfo, RangeCellInfo, ResolvedRangeInfo,
    SheetChartWorkbookInfo,
};
pub use comments::{CommentBoxInfo, SheetCommentInfo};
pub use drawings::{AnchorBoundsInfo, AnchorShiftInfo, SheetDrawingInfo, SheetDrawingObjectInfo};
pub use grid::{GridAnomalyInfo, GridAnomalyKind};
pub use hyperlinks::SheetHyperlinkInfo;
pub use names::DefinedName;
pub use preserved::{
    PreservedPart, PreservedPartsSummary, RevisionSessionInfo, SharedWorkbookUserInfo,
    SheetPivotTableInfo, SheetQueryTableInfo, WorkbookConnectionInfo, WorkbookExternalLinkInfo,
    WorkbookRevisionState, WorkbookXmlMapsInfo, XmlMapInfo,
};
pub use sheets::{SheetSummary, WorkbookWindowInfo};
pub use tables::{SheetTableColumnInfo, SheetTableInfo};

/// An open Excel workbook.
///
/// ```no_run
/// use mjx_ooxml::{CellInput, CellWrite, Workbook};
///
/// # fn main() -> Result<(), mjx_ooxml::Error> {
/// let mut workbook = Workbook::blank()?;
/// workbook.write_cells(
///     0,
///     &[
///         CellWrite::new("A1", CellInput::SharedText("Region".into())),
///         CellWrite::new("B1", CellInput::SharedText("Growth".into())),
///         CellWrite::new("A2", CellInput::SharedText("North America".into())),
///         CellWrite::new("B2", CellInput::Number(12.0)),
///     ],
/// )?;
/// let bytes = workbook.save()?;
/// # let _ = bytes;
/// # Ok(())
/// # }
/// ```
///
/// # Addressing
///
/// A **sheet** is a `u32` — its position in the workbook's tab list, which is what
/// [`sheets`](Self::sheets) enumerates. A **cell** is A1 text: `"B7"`, `"$B$7"`, either accepted and
/// both meaning the same cell. A **range** is A1 text too: `"A1:C3"`, `"A:C"` for whole columns,
/// `"1:3"` for whole rows. Nothing here takes a `(row, column)` pair, so nothing here can be
/// transposed by a caller who guessed the order.
#[derive(Debug)]
pub struct Workbook {
    workbook: mjx_xlsx::Workbook,
    format: Format,
}

impl Workbook {
    /// A new workbook with nothing in it: one empty worksheet named `Sheet1`, a styles part, and the
    /// package around them.
    ///
    /// Nothing is read from disk and no template is embedded — every part is authored from this
    /// library's own element builders, which is what makes a workbook buildable in a browser or from
    /// a `pip install` with no input file.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`] if the authored seed does not read back as the part it was
    /// written as, which is a bug here rather than anything a caller did.
    pub fn blank() -> Result<Self, Error> {
        Ok(Self {
            workbook: mjx_xlsx::Workbook::blank()?,
            format: Format::Workbook,
        })
    }

    /// Opens a workbook from the bytes of a `.xlsx`, `.xlsm`, `.xltx` or `.xltm`.
    ///
    /// The format is [detected](crate::detect_format) from the package before anything is parsed as
    /// SpreadsheetML, so a PowerPoint or Word document is refused by name rather than by a parse
    /// failure, and the package is read exactly once.
    ///
    /// # `.xlsb` is refused by design, not by schedule
    ///
    /// A binary workbook ([`Format::WorkbookBinary`]) is a conforming OPC package whose main part is
    /// the MS-XLSB binary record stream and not SpreadsheetML at all. There is no markup in it for
    /// this library to read, so it is refused with an [`ErrorCode::UnsupportedFormat`] whose message
    /// says exactly that — a different sentence from the one a `.pptx` gets, because it is a
    /// different fact. Nothing later in this project's roadmap changes it.
    ///
    /// # Errors
    /// - [`ErrorCode::Io`] if the bytes are not a readable ZIP container.
    /// - [`ErrorCode::UnsupportedFormat`] if the package is a PowerPoint or Word document, a `.xlsb`,
    ///   or an OPC package that is not an Office document at all.
    /// - [`ErrorCode::MalformedDocument`] if it is a workbook whose `xl/workbook.xml` or
    ///   relationships are not what the schema requires.
    pub fn open(bytes: &[u8]) -> Result<Self, Error> {
        let package = mjx_xlsx::Package::open(bytes)?;
        let format = format_of(&package)?;
        if format == Format::WorkbookBinary {
            return Err(Error::new(
                ErrorCode::UnsupportedFormat,
                "these bytes are a binary workbook (.xlsb), whose main part is the MS-XLSB record \
                 stream rather than SpreadsheetML — there is no markup in it for this library to \
                 read, and that is by design rather than by schedule",
            ));
        }
        if format.family() != crate::FormatFamily::Spreadsheet {
            let hint = match format.family() {
                crate::FormatFamily::Presentation => {
                    " — open it with mjx_ooxml::Deck::open instead"
                }
                crate::FormatFamily::WordProcessing => {
                    " — open it with mjx_ooxml::Document::open instead"
                }
                _ => "",
            };
            return Err(Error::new(
                ErrorCode::UnsupportedFormat,
                format!(
                    "this call opens SpreadsheetML only; these bytes are {:?} (.{}), which is not a \
                     workbook{hint}",
                    format,
                    format.conventional_extension()
                ),
            ));
        }
        Ok(Self {
            workbook: mjx_xlsx::Workbook::from_package(package)?,
            format,
        })
    }

    /// Which SpreadsheetML format this workbook was opened as — a workbook or a template,
    /// macro-enabled or not.
    ///
    /// It survives editing and saving: this library never rewrites the main part's content type, so
    /// a `.xltx` opened, edited and saved is still a `.xltx`. A workbook built with
    /// [`blank`](Self::blank) is a [`Format::Workbook`].
    #[must_use]
    pub fn format(&self) -> Format {
        self.format
    }

    /// Serializes the workbook back to `.xlsx` container bytes, **after** checking its invariants.
    ///
    /// Every part that was not edited is re-emitted byte-identically; only parts this library
    /// actually changed are re-serialized. [`validate`](Self::validate) runs first, for the reason
    /// [`crate::Deck::save`] gives at length: a facade that lets a caller write a file the layer
    /// below refuses would be a regression, not a convenience.
    ///
    /// # Errors
    /// [`ErrorCode::InvalidDocument`] if a packaging or SpreadsheetML invariant is broken, or
    /// [`ErrorCode::Io`] if the ZIP writer fails.
    pub fn save(&self) -> Result<Vec<u8>, Error> {
        Ok(self.workbook.save()?)
    }

    /// Serializes the workbook **without** checking its invariants — the deliberate override for
    /// [`save`](Self::save).
    ///
    /// # Errors
    /// [`ErrorCode::Io`] if the ZIP writer fails.
    pub fn save_unchecked(&self) -> Result<Vec<u8>, Error> {
        Ok(self.workbook.save_unchecked()?)
    }

    /// Checks every invariant [`save`](Self::save) enforces, without writing anything.
    ///
    /// # Errors
    /// [`ErrorCode::InvalidDocument`], carrying the first invariant broken as its
    /// [`source`](std::error::Error::source).
    pub fn validate(&self) -> Result<(), Error> {
        Ok(self.workbook.validate()?)
    }

    /// The underlying [`mjx_xlsx::Workbook`], for reading.
    ///
    /// The Rust-only door to everything this facade does not restate — the closure-taking markup
    /// readers, the borrowed views, the four authoring vocabularies, and the per-cell
    /// [`cell_text`](mjx_xlsx::Workbook::cell_text) this surface deliberately does not carry.
    /// Bindings do not expose it, because none of those signatures crosses a foreign function
    /// boundary.
    #[must_use]
    pub fn workbook(&self) -> &mjx_xlsx::Workbook {
        &self.workbook
    }

    /// The underlying [`mjx_xlsx::Workbook`], for editing. See [`workbook`](Self::workbook).
    ///
    /// This widens nothing: every invariant is enforced on [`save`](Self::save), which is the same
    /// `mjx_xlsx::Workbook::save` either way. It is the *package* that stays sealed — there is no
    /// `Workbook::package`, because handing out `&mut Package` would give a caller the whole part
    /// graph and make those invariants unenforceable.
    pub fn workbook_mut(&mut self) -> &mut mjx_xlsx::Workbook {
        &mut self.workbook
    }

    /// Consumes the facade wrapper and returns the [`mjx_xlsx::Workbook`] inside it.
    #[must_use]
    pub fn into_workbook(self) -> mjx_xlsx::Workbook {
        self.workbook
    }
}

impl From<mjx_xlsx::Workbook> for Workbook {
    /// Wraps a workbook this crate did not open — the inverse of
    /// [`into_workbook`](Workbook::into_workbook). Its [`format`](Workbook::format) reports
    /// [`Format::Workbook`], since an `mjx_xlsx::Workbook` carries no record of the content type it
    /// was opened under.
    fn from(workbook: mjx_xlsx::Workbook) -> Self {
        Self {
            workbook,
            format: Format::Workbook,
        }
    }
}
