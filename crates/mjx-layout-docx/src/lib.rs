//! Word's box model: the third implementation of [`mjx_layout::BoxModel`], and the only one that
//! **reflows**.
//!
//! # What makes a document different from a slide and from a grid
//!
//! PowerPoint places absolutely: a slide is a page, and page *N* is reachable without ever looking
//! at page *N−1*. A worksheet addresses a grid: a band's first row comes from the page number and
//! the row geometry, by arithmetic. Both paginations are **known in advance**.
//!
//! Word's is **emergent**. Where page 200 begins depends on everything on the 199 pages before it,
//! and a single font substitution moves every boundary in the document. There is no arithmetic that
//! answers *which paragraph does page 200 start at* — which is why `mjx-layout`'s
//! [`Checkpoint`](mjx_layout::Checkpoint) exists, and this crate is its first real consumer.
//!
//! ```no_run
//! use mjx_layout::{BoxModel, Constraints, LayoutSize, PageIndex};
//! use mjx_layout_docx::{DocumentBoxModel, DocumentFlow};
//! use mjx_ooxml_core::measure::Emu;
//! use mjx_text::FontResolver;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut document = mjx_docx::Document::open(&std::fs::read("report.docx")?)?;
//! let flow = DocumentFlow::read(&mut document)?;
//!
//! let mut model = DocumentBoxModel::new(FontResolver::builder().with_platform_fonts().build());
//! let constraints = Constraints::single_column(
//!     LayoutSize { width: Emu::from_inches(8.5), height: Emu::from_inches(11.0) },
//!     Emu::from_inches(1.0),
//! );
//!
//! // Page one, and the checkpoint that ends it.
//! let first = model.layout_page(&flow, PageIndex::FIRST, &constraints, None)?;
//! // Page two, laid out from that checkpoint — not from the beginning.
//! if let Some(resume) = first.continuation() {
//!     let second = model.layout_page(&flow, PageIndex::new(1), &constraints, Some(resume))?;
//!     println!("{} fragments", second.fragments().len());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # What it consumes and never re-derives
//!
//! | Question | Answered by |
//! |---|---|
//! | What formatting does this paragraph carry? | [`mjx_docx::DocumentFormatting`] — the whole ladder, resolved once |
//! | Where may this line end? | `mjx-text`'s UAX #14 and its kinsoku layer |
//! | Which candidate fits the measure? | [`mjx_layout::LineComposer`] |
//! | Where may this word be split? | `mjx-text`'s [`Hyphenator`](mjx_text::Hyphenator) |
//! | How wide is this text? | the shaper, through a [`ShapedRun`](mjx_text::ShapedRun) |
//! | How is a page resumed? | [`mjx_layout::Checkpoint`] |
//!
//! `w:docDefaults`, the `w:basedOn` chains, the character-style tier, the numbering level's own
//! `w:pPr`/`w:rPr` and the twelve toggle properties' XOR recombination have **all already run** by
//! the time anything here reads a value. That is the whole reason this crate sits above the format
//! tier rather than inside it, and `tests/the_ladder_is_consumed.rs` holds it by grepping this
//! crate's own source for the identifiers a re-derivation would need.
//!
//! **`mjx_docx::Document::formatting` was added by this child** and is the reason laying out a long
//! document is affordable at all: `Document::effective_paragraph_properties` re-parses
//! `word/document.xml`, `word/styles.xml` and the theme **on every call**, which is right for a
//! caller asking one question and quadratic for one asking per paragraph.
//!
//! # Where the checkpoint earns its keep, and how that is measured
//!
//! [`DocumentBoxModel::paragraphs_visited`] reports how many paragraphs the last call had to look
//! at. It is not decoration: **a checkpoint that is never used still produces correct output**, so a
//! gate on the fragments is green for an implementation with no checkpoints in it. The gate that is
//! not is `tests/a_checkpoint_is_work_not_output.rs`, which lays page 200 out both ways, asserts the
//! fragments are identical, and asserts the work is not.
//!
//! # What is deliberately not here
//!
//! * **Sections, columns, headers, footers and footnotes** — MJXOFF-175 (R20). One column per page,
//!   from `Constraints`; a `w:br@type="column"` behaves as a page break, which is what it *means*
//!   when there is one column.
//! * **Tables and floating objects** — MJXOFF-176 (R21). A table's own paragraphs are not walked at
//!   all: [`mjx_docx::DocumentFormatting::paragraphs`] is the body's top level.
//! * **Fields, numbering, revision marks and OMML** — MJXOFF-177/178 (R22). A list's *number* is not
//!   drawn; its indents are, because they are ordinary `w:pPr` members the ladder already resolved.
//! * **A display list.** This crate never paints and never resolves a handle;
//!   `tests/the_seam_holds.rs` refuses `mjx-scene`, `mjx-paint` and `mjx-geometry` by name. **Word's
//!   scene companion does not exist yet** — PowerPoint's is `mjx-scene-pptx` and Excel's is
//!   `mjx-scene-xlsx`, both at rank 3.7, and `mjx-scene-docx` is the ticket that has to follow this
//!   one. Until it does, a Word [`FragmentTree`](mjx_layout::FragmentTree) cannot reach pixels.
//! * **A pattern hyphenator.** [`mjx_text::PatternHyphenator`] exists and works; the Liang patterns
//!   it needs are language data and none is committed here. `w:autoHyphenation` therefore hyphenates
//!   only at the soft hyphens an author wrote, unless a caller supplies one through
//!   [`DocumentBoxModel::with_hyphenator`]. See [`crate::flow`].
//! * **`w:sym`.** A symbol is a character code in a *named font*, and `mjx-docx`'s residency
//!   deliberately contributes no character for one rather than drawing the wrong glyph.
//!
//! # ⚠ Nothing in this crate is parity with Word, and it is not described as such
//!
//! ECMA-376 says what the attributes are and is nearly silent on what a renderer does with them, so
//! a number of behaviours here are readings rather than facts. Every one is marked `GUESS:` at the
//! site that makes the choice, and the sharpest are collected in [`crate::style`],
//! [`crate::tabs`], [`crate::justify`] and [`crate::paginate`]: where a tab stop is measured from,
//! whether the space above a paragraph survives a page break, which face a hyphen takes, what a
//! kashida alignment falls back to, and which characters an East Asian line is stretched around.
//!
//! Confirmation is a human sitting against real Microsoft Word on Windows
//! (`docs/validation/07-the-reference-pack.md`). LibreOffice is a change detector and not a
//! reference.

#![forbid(unsafe_code)]

pub mod address;
pub mod checkpoint;
pub mod decoration;
pub mod error;
pub mod flow;
pub mod justify;
pub mod measure;
pub mod model;
pub mod paginate;
pub mod style;
pub mod tabs;
pub mod text;

pub use checkpoint::{Continuation, STATE_BYTES, VERSION};
pub use decoration::{stroke_rect, DecorationCatalogue, ParagraphDecoration, Rule};
pub use error::DocumentLayoutError;
pub use flow::{lay_out, FlowContext, LaidOutLine, ParagraphLayout};
pub use justify::{
    expansion_points, is_east_asian, place, LineContext, LinePlacement, PlacedLeader, PlacedSegment,
};
pub use measure::{border_width, half_of, HAIRLINE};
pub use model::{DocumentBoxModel, DocumentFlow, MAXIMUM_LEADER_GLYPHS};
pub use paginate::{
    assemble, widow_control, FlowPosition, PageAssembly, PageShape, PlacedParagraph,
};
pub use style::{Alignment, LineHeight, ParagraphStyle, RunStyle, ASSUMED_FONT_SIZE_POINTS};
pub use tabs::{leader_character, TabKind, TabRuler, TabStop, DECIMAL_SEPARATOR};
pub use text::{cut, CutPolicy, StyledItem, TextEngine, HYPHEN};

/// The constraints a caller lays a document out under, from one section's own page geometry.
///
/// A `.docx` states its page size and margins in `w:sectPr`, which is more than PowerPoint's box
/// model has (a slide size) and more than Excel's (nothing — a viewport is the caller's). This turns
/// the section's own numbers into a [`Constraints`](mjx_layout::Constraints), so a caller does not
/// have to know that twips are twentieths of a point.
///
/// A section that states no page size gets US Letter with one-inch margins, which is
/// `mjx_docx::PageMargins::NORMAL`'s own geometry — **GUESS:** a document that states nothing is
/// most likely American, and the alternative (A4) is wrong by the same margin in the other
/// direction.
#[must_use]
pub fn constraints_for(section: &mjx_docx::SectionFormatting) -> mjx_layout::Constraints {
    use mjx_ooxml_core::measure::Emu;
    let size = section.page_size.unwrap_or(mjx_docx::PageSize::us_letter());
    let page = mjx_layout::LayoutSize {
        width: Emu::from_twips(i64::from(size.width_twips)),
        height: Emu::from_twips(i64::from(size.height_twips)),
    };
    let margins = section
        .page_margins
        .unwrap_or(mjx_docx::PageMargins::NORMAL);
    mjx_layout::Constraints {
        page,
        content: mjx_layout::LayoutRect::from_edges(
            Emu::from_twips(i64::from(margins.left)),
            Emu::from_twips(i64::from(margins.top)),
            page.width - Emu::from_twips(i64::from(margins.right)),
            page.height - Emu::from_twips(i64::from(margins.bottom)),
        ),
        columns: 1,
        column_gap: Emu::ZERO,
        base_direction: mjx_text::TextDirection::LeftToRight,
        writing_mode: mjx_layout::WritingMode::HorizontalTopToBottom,
    }
}
