//! Where a page's edges are, which section decides them, and what happens when the answer changes
//! half way through a document.
//!
//! # A section is not a chapter, and its properties sit at its END
//!
//! `mjx-docx`'s own `sections.rs` states the trap and this module is its first layout consumer: a
//! `w:sectPr` inside a paragraph's `w:pPr` **ends** the section that paragraph belongs to. The
//! body-level one governs whatever is left. So the geometry of page one comes from the *first*
//! section's `w:sectPr`, which is usually not the last element of the file — and **an engine that
//! read only the body-level `w:sectPr` would lay every page of every document out at the last
//! section's page size**, which is invisible in a single-section fixture and wrong in every
//! multi-section one. `tests/a_section_changes_the_page.rs` is the mutation that catches it.
//!
//! # The document's geometry wins, and the caller's [`Constraints`] is what it falls back to
//!
//! MJXOFF-174 laid every page out at the [`Constraints`] the caller passed, because there was one
//! section and the caller could read it. With several there is no such call: which section page 200
//! belongs to is not knowable without laying out the 199 before it, so the caller cannot choose.
//!
//! So a section that states `w:pgSz` or `w:pgMar` **overrides** the caller's constraints for its own
//! pages, and a section that states neither inherits them unchanged. That is the standing rule of
//! this project applied to a page: the document the user opened decides what its pages look like,
//! and our own defaults fill in only where the file is silent. A caller that wants to *impose* a
//! page size — a print preview at a different paper size — is asking a different question, and the
//! honest answer to it is to rewrite the sections rather than to have the box model quietly ignore
//! them.
//!
//! # Mirrored margins are a property of the page number, not of the section
//!
//! `w:mirrorMargins` is a document-wide setting and it swaps `w:pgMar`'s left and right on an **even**
//! page, so that the binding margin is always on the inside. It therefore cannot be resolved when the
//! section is read; it is applied here, per page, which is why [`SectionGeometry::of`] takes a page
//! number at all.

use mjx_docx::{DocumentLayoutSettings, PageMargins, PageSize, SectionFormatting};
use mjx_layout::{Constraints, LayoutRect, LayoutSize};
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::wordprocessingml::SectionBreakType;

/// One column of a section's text area.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ColumnBand {
    /// Its left edge, from the page's own left edge.
    pub left: Emu,
    /// Its right edge.
    pub right: Emu,
}

impl ColumnBand {
    /// How wide it is.
    #[must_use]
    pub fn width(self) -> Emu {
        self.right - self.left
    }
}

/// What one page of one section looks like, before its header and footer claim any of it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SectionGeometry {
    /// The whole sheet.
    pub page: LayoutSize,
    /// The text area: the page less its margins, with the gutter already added to the binding side.
    pub body: LayoutRect,
    /// `w:pgMar@header` — how far below the page's top edge the header's own band begins.
    pub header_distance: Emu,
    /// `w:pgMar@footer` — how far above the page's bottom edge the footer's band ends.
    pub footer_distance: Emu,
    /// The columns the body is divided into, left to right. Never empty.
    pub columns: Vec<ColumnBand>,
    /// `w:cols@sep` — whether a vertical rule is drawn between two columns.
    pub column_separator: bool,
}

impl SectionGeometry {
    /// The geometry of a page of `section`, falling back to `constraints` wherever the section
    /// states nothing.
    ///
    /// `page_number` is the number the page **displays**, and it is read for one thing only:
    /// `w:mirrorMargins` swaps the left and right margins on an even page.
    #[must_use]
    pub fn of(
        section: Option<&SectionFormatting>,
        constraints: &Constraints,
        settings: &DocumentLayoutSettings,
        page_number: i64,
    ) -> Self {
        let page = section
            .and_then(|section| section.page_size)
            .map_or(constraints.page, page_size_of);
        let body = match section.and_then(|section| section.page_margins) {
            Some(margins) => body_of(page, margins, settings.mirror_margins, page_number),
            // The section states no margins, so the caller's content area is used — but scaled to
            // nothing: it is a rectangle on *its* page, and a section that changed the page size
            // without stating margins would otherwise put the text area off the sheet.
            None => clamp(constraints.content, page),
        };
        let (header_distance, footer_distance) =
            match section.and_then(|section| section.page_margins) {
                Some(margins) => (
                    Emu::from_twips(i64::from(margins.header)),
                    Emu::from_twips(i64::from(margins.footer)),
                ),
                None => (
                    Emu::from_twips(i64::from(PageMargins::NORMAL.header)),
                    Emu::from_twips(i64::from(PageMargins::NORMAL.footer)),
                ),
            };
        let (columns, column_separator) = match section {
            Some(section) => columns_of(section, body, constraints),
            None => (bands_from_constraints(body, constraints), false),
        };
        Self {
            page,
            body,
            header_distance,
            footer_distance,
            columns,
            column_separator,
        }
    }

    /// How many columns there are — never zero.
    #[must_use]
    pub fn column_count(&self) -> usize {
        self.columns.len().max(1)
    }

    /// The band at `index`, or the whole body when there is no such column.
    #[must_use]
    pub fn column(&self, index: usize) -> ColumnBand {
        self.columns.get(index).copied().unwrap_or(ColumnBand {
            left: self.body.left,
            right: self.body.right,
        })
    }
}

/// Whether a section break of `kind` starts a new page.
///
/// **GUESS:** `w:type` carries no schema default and `mjx-docx` deliberately refuses to assert one,
/// so an absent `w:type` is read here as `nextPage` — which is what Word does, and the alternative
/// (treating it as `continuous`) would run two sections' page geometry together on one sheet.
#[must_use]
pub fn starts_a_page(kind: Option<SectionBreakType>) -> bool {
    !matches!(
        kind.unwrap_or(SectionBreakType::NextPage),
        SectionBreakType::Continuous
    )
}

/// Whether a section break of `kind` demands a page of a particular parity, and which.
///
/// `evenPage` and `oddPage` do more than start a page: they start a page **of the stated parity**,
/// which means a blank one is opened when the next page would have the wrong one. That blank page is
/// content-free and completely invisible to any assertion about which paragraph is on which page —
/// `tests/a_section_changes_the_page.rs` asserts the page *count* for exactly that reason.
#[must_use]
pub fn required_parity(kind: Option<SectionBreakType>) -> Option<Parity> {
    match kind {
        Some(SectionBreakType::EvenPage) => Some(Parity::Even),
        Some(SectionBreakType::OddPage) => Some(Parity::Odd),
        _ => None,
    }
}

/// Which page numbers a section may begin on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Parity {
    /// An even page number.
    Even,
    /// An odd one.
    Odd,
}

impl Parity {
    /// Whether `number` has this parity.
    #[must_use]
    pub fn holds_for(self, number: i64) -> bool {
        let even = number.rem_euclid(2) == 0;
        match self {
            Self::Even => even,
            Self::Odd => !even,
        }
    }
}

/// The page a `w:pgSz` describes.
fn page_size_of(size: PageSize) -> LayoutSize {
    LayoutSize {
        width: Emu::from_twips(i64::from(size.width_twips)),
        height: Emu::from_twips(i64::from(size.height_twips)),
    }
}

/// The text area a `w:pgMar` leaves, with the gutter on the binding side.
fn body_of(page: LayoutSize, margins: PageMargins, mirrored: bool, page_number: i64) -> LayoutRect {
    let gutter = Emu::from_twips(i64::from(margins.gutter));
    let mut left = Emu::from_twips(i64::from(margins.left));
    let mut right = Emu::from_twips(i64::from(margins.right));
    let even = page_number.rem_euclid(2) == 0;
    if mirrored && even {
        std::mem::swap(&mut left, &mut right);
    }
    // The gutter is extra binding space, and it is on the inside edge: the left of an odd page and
    // the right of an even one when margins are mirrored, and always the left otherwise.
    // **GUESS:** `w:rtlGutter` moves it to the other side and is not honoured here; a document that
    // sets it has its binding space on the wrong edge, which is a visible half-inch and is named in
    // the provenance ledger rather than hidden.
    if mirrored && even {
        right += gutter;
    } else {
        left += gutter;
    }
    LayoutRect::from_edges(
        left,
        Emu::from_twips(i64::from(margins.top)),
        page.width - right,
        page.height - Emu::from_twips(i64::from(margins.bottom)),
    )
}

/// A caller's content rectangle, kept on the page the section actually has.
fn clamp(content: LayoutRect, page: LayoutSize) -> LayoutRect {
    LayoutRect::from_edges(
        content.left.maximum(Emu::ZERO),
        content.top.maximum(Emu::ZERO),
        content.right.minimum(page.width),
        content.bottom.minimum(page.height),
    )
}

/// The bands `w:cols` divides `body` into.
fn columns_of(
    section: &SectionFormatting,
    body: LayoutRect,
    constraints: &Constraints,
) -> (Vec<ColumnBand>, bool) {
    let columns = &section.columns;
    if columns.count <= 1 && columns.columns.is_empty() {
        // One column, or no `w:cols` at all. The caller's own column request is honoured here and
        // only here: a document that says nothing about columns is a document a caller may divide.
        return (bands_from_constraints(body, constraints), columns.separator);
    }
    if columns.columns.is_empty() {
        let count = columns.count.max(1);
        let gap = Emu::from_twips(columns.space_twips);
        return (equal_bands(body, count, gap), columns.separator);
    }
    // An explicit `w:col` list: each column's own width, each gap its own `w:col@space`. A stated
    // width of zero means the file wrote a `w:col` with no `w:w`; the band is then empty and the
    // engine's own "at least one line" rule is what keeps the page moving.
    let mut bands = Vec::with_capacity(columns.columns.len());
    let mut left = body.left;
    for column in &columns.columns {
        let width = Emu::from_twips(column.width_twips);
        let right = (left + width).minimum(body.right);
        bands.push(ColumnBand { left, right });
        left = right + Emu::from_twips(column.space_after_twips);
    }
    (bands, columns.separator)
}

/// `count` equal columns separated by `gap`.
fn equal_bands(body: LayoutRect, count: usize, gap: Emu) -> Vec<ColumnBand> {
    let gaps = gap.times(i64::try_from(count).unwrap_or(1).saturating_sub(1));
    let usable = body.width() - gaps;
    if usable <= Emu::ZERO {
        return vec![ColumnBand {
            left: body.left,
            right: body.right,
        }];
    }
    let width = usable.divided_by(i64::try_from(count).unwrap_or(1).max(1));
    (0..count)
        .map(|index| {
            let left = body.left + (width + gap).times(i64::try_from(index).unwrap_or(0));
            ColumnBand {
                left,
                right: left + width,
            }
        })
        .collect()
}

/// The bands a caller's own [`Constraints`] asks for, mapped on to `body`.
fn bands_from_constraints(body: LayoutRect, constraints: &Constraints) -> Vec<ColumnBand> {
    let count = usize::from(constraints.column_count());
    if count <= 1 {
        return vec![ColumnBand {
            left: body.left,
            right: body.right,
        }];
    }
    equal_bands(body, count, constraints.column_gap)
}
