//! Excel's box model: the second implementation of [`mjx_layout::BoxModel`], and the first one whose
//! layout discipline is a **grid**.
//!
//! # What makes a spreadsheet different from a slide
//!
//! PowerPoint places shapes absolutely, so a slide's layout cost is the number of shapes on it. A
//! worksheet addresses **16,384 columns by 1,048,576 rows** — seventeen billion cells — and its
//! layout cost has to be the number of cells *on screen*, because nothing else is survivable.
//!
//! That one fact decides the whole crate:
//!
//! * **The geometry is two sparse indices**, not two arrays. [`RowGeometry`] holds one record per
//!   row that states a height, a hidden flag or an outline level, and [`ColumnGeometry`] one per
//!   `col` **run** — which is what `CT_Col` already is. Both answer *where is row n* and *which row
//!   is at y* by binary search plus one multiplication, at a cost that does not depend on how far
//!   down the sheet the question is asked.
//! * **Every cell is a binary search in `mjx-sml`'s packed store** (36.8 bytes a populated cell,
//!   nothing for an unpopulated one). Nothing here iterates a coordinate range, in either axis, at
//!   any point.
//! * **The gate measures that rather than assuming it.**
//!   `tests/sparsity_is_measured.rs` lays out a sheet whose only populated cell is `XFD1048576`
//!   under a counting allocator, and asserts both a byte ceiling and a bound on the cells visited.
//!   A small fixture would be laid out identically by a windowed implementation and by one that
//!   walks the grid, which is why there is no small fixture in that gate.
//!
//! ```no_run
//! use mjx_layout::{BoxModel, Constraints, LayoutSize, PageIndex};
//! use mjx_layout_xlsx::{SheetBoxModel, SheetGrid};
//! use mjx_ooxml_core::measure::Emu;
//! use mjx_text::FontResolver;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let workbook = mjx_xlsx::Workbook::open(&std::fs::read("book.xlsx")?)?;
//! let grid = SheetGrid::read(&workbook, 0)?;
//!
//! let mut model = SheetBoxModel::new(FontResolver::builder().with_platform_fonts().build());
//! let constraints = Constraints::single_column(
//!     LayoutSize { width: Emu::from_inches(10.0), height: Emu::from_inches(7.0) },
//!     Emu::ZERO,
//! );
//! let page = model.layout_page(&grid, PageIndex::FIRST, &constraints, None)?;
//! println!("{} fragments in the first band", page.fragments().len());
//! # Ok(())
//! # }
//! ```
//!
//! # What it consumes and never re-derives
//!
//! | Question | Answered by |
//! |---|---|
//! | What formatting does this cell carry? | [`SheetFormatResolver::effective_cell_format`](mjx_xlsx::SheetFormatResolver::effective_cell_format) |
//! | Which font, fill, border, alignment and number format is that? | [`CellFormatResolver`](mjx_sml::CellFormatResolver)'s six aspect accessors |
//! | Which merge covers this cell, and what is its anchor? | [`WorksheetPart::merged_ranges`](mjx_sml::WorksheetPart::merged_ranges) |
//! | What does this cell hold? | `mjx-sml`'s packed cell store, plus the shared-string table |
//! | Where is the sheet frozen or split? | `x:sheetView/pane`, unconverted, exactly as the file wrote it |
//! | What format code does `numFmtId="14"` mean? | [`mjx_sml::builtin_format_code`] and the workbook's own `numFmts` |
//! | Which epoch do this workbook's date serials count from? | [`mjx_xlsx::DateSystem`] |
//!
//! The `xf` indirection, the cell → row → column → default walk, the `cellXfs`/`cellStyleXfs`
//! layering and the `applyX` gating have **all already run** by the time anything here reads a
//! value. That is the whole reason this crate sits above the format tier rather than inside it, and
//! `tests/the_ladder_is_consumed.rs` holds it by grepping this crate's own source for the
//! identifiers a re-derivation would need.
//!
//! # A cell shows what Excel shows, not what the file stores
//!
//! MJXOFF-172 added [`crate::numfmt`]: the `numFmt` evaluator. A cell's text is the *formatted*
//! value — `04/03/2025` rather than `45719`, `$1,234.50` rather than `1234.5` — and the format's
//! own `[Red]` reaches the painter on [`Decoration::text_colour`], which is per **value** and so
//! cannot live on a decoration shared by a whole effective format.
//!
//! The engine is a module of this crate rather than of `mjx-sml`, deliberately: `mjx-sml` reports
//! *which* code is in force and says so in as many words, and turning a code into a string is a
//! renderer's job. It is nevertheless independent of everything else here — it takes a code, a
//! value and a [`mjx_xlsx::DateSystem`] and answers a string — which is what makes it testable
//! against a conformance table rather than against a fragment tree.
//!
//! Every **measurement** comes from `mjx-text`: shaping, bidirectional resolution, script
//! itemisation, face fallback and line breaking. Nothing here measures a glyph.
//!
//! # ⚠ Nothing in this crate is parity with Excel, and it is not described as such
//!
//! ECMA-376 says what the attributes are and is nearly silent on what a renderer does with them, so
//! a number of behaviours here are readings rather than facts. Every one is marked `GUESS:` at the
//! site that makes the choice. The sharpest are collected in [`crate::cell`], [`crate::overflow`]
//! and [`crate::autofit`]: the cell's internal padding, what an `indent` level is worth, what a
//! rotation pivots about, whether a formatted-but-empty cell stops an overflow, and what a fitted
//! column's margin is.
//!
//! Confirmation is a human sitting against real Microsoft Excel on Windows
//! (`docs/validation/07-the-reference-pack.md`). LibreOffice is a change detector and not a
//! reference.
//!
//! # What is deliberately not here
//!
//! * **Charts** — MJXOFF-179 (R23). A chart on a sheet is a drawing, and drawings are R18.
//! * **The inside of a drawing.** MJXOFF-173 places every anchored object — all three modes,
//!   against this crate's own row heights and column widths — and lays out **none** of them. A
//!   drawing's content is DrawingML and laying that out is `mjx-layout-pptx`'s subject; that crate
//!   is at rank 3.6, the same rank as this one, so the edge is *sideways* and the layering gate
//!   refuses it by name. See [`crate::drawings`].
//! * **Sparklines.** `x14:sparklineGroups` lives in a worksheet's `extLst`, which `mjx-sml`
//!   preserves verbatim and deliberately does not model — the same decision that leaves
//!   `x14:dataBar`'s axis and negative fill unmodelled. There is nothing to evaluate here until a
//!   `mjx-sml` workstream models the extension, and reading raw nodes out of the bucket from a
//!   layout crate would be modelling markup two tiers above where markup belongs.
//! * **Formula evaluation**, which does not exist in this loop at all: a cell's cached value is
//!   rendered as stored, which is correct for a viewer.
//! * **Recomputed row heights.** Excel writes a fitted height into the file and this honours it; see
//!   [`crate::autofit`] for why recomputing one and keeping a sparse row index are mutually
//!   exclusive.
//! * **A display list.** This crate never paints and never resolves a resource handle;
//!   `tests/the_seam_holds.rs` refuses `mjx-scene`, `mjx-paint` and `mjx-geometry` by name. Excel's
//!   companion resolver is `mjx-scene-xlsx` at rank 3.7 (MJXOFF-244) — the analogue of
//!   `mjx-scene-pptx`, and the crate that turns every handle here into a paint.
//! * **Gridlines.** `x:sheetView@showGridLines` defaults to *on*, and a worksheet's grey grid is the
//!   most recognisable thing on an Excel screen; it is nevertheless **not** in this fragment tree.
//!   A gridline is a property of the *view* rather than of the document — it is not printed unless
//!   `printOptions@gridLines` says so, it does not move with an edit, and drawing one per visible
//!   row and column would multiply a band's fragment count by an order of magnitude for content no
//!   hit test can ever land on. MJXOFF-244 reported it rather than adding it quietly; where it
//!   belongs (a chrome layer above the box model, or a fragment kind that says *this is furniture*)
//!   is a decision for the child that draws a sheet on a screen.

#![forbid(unsafe_code)]

pub mod address;
pub mod autofit;
pub mod border;
pub mod cell;
pub mod condfmt;
pub mod drawings;
pub mod error;
pub mod geometry;
pub mod merge;
pub mod model;
pub mod numfmt;
pub mod overflow;
pub mod panes;
pub mod print;
pub mod sheet;
pub mod text;

pub use address::CellHit;
pub use autofit::{AutoFit, AutoFitCache};
pub use border::{band_width, bands, BorderBand};
pub use cell::{CellStyle, PlacedLine, PlacedText};
pub use condfmt::{
    AppliedRule, ConditionalEffect, ConditionalEngine, ConditionalIndex, ConditionalSignature,
    DataBarGeometry, IconChoice, RuleValue, ScaleBlend, UnevaluatedReason, UnevaluatedRule,
};
pub use drawings::{place as place_drawings, AnchorMode, PlacedDrawing};
pub use error::SheetLayoutError;
pub use geometry::{
    ColumnGeometry, ColumnSpan, GridGeometry, MaximumDigitWidth, RowGeometry, RowSpan,
};
pub use merge::{MergeIndex, MergedRegion, RegionEdge};
pub use model::{
    BorderEdge, CellBorders, CellFill, CellGradient, CellGradientStop, CellReport, Decoration,
    PageCatalogue, SheetBoxModel,
};
pub use numfmt::{CellValue, CompiledFormat, FormatCache, FormattedValue};
pub use overflow::{Overflow, OverflowDirection};
pub use panes::{PaneRegion, PaneSplit, Window};
pub use print::{paginate, PrintMargins, PrintPage, PrintPagination, PrintSetup};
pub use sheet::SheetGrid;
pub use text::{CellRunStyle, TextEngine};

/// The constraints a caller lays a sheet out under when it has no window of its own to impose.
///
/// A sheet has no page size — a printed page is [`crate::print`]'s subject and a **different**
/// pagination — so this is the *viewport* a reader looks through, and it is the caller's to choose. What this offers
/// is the shape of the answer rather than the answer: a single column, no margins, the whole of
/// `viewport` given over to the grid.
#[must_use]
pub fn constraints_for(viewport: mjx_layout::LayoutSize) -> mjx_layout::Constraints {
    mjx_layout::Constraints::single_column(viewport, mjx_ooxml_core::measure::Emu::ZERO)
}
