//! What the paginator actually fills a column with: a **block**, of which a paragraph and a table
//! are the two kinds.
//!
//! # One paginator, two kinds of content
//!
//! MJXOFF-174's [`crate::paginate::fill_column`] knew about paragraphs and lines. Nothing in it was
//! *about* paragraphs: it placed things that have a height, may be cut between units, may refuse to
//! be cut at all, and may insist on sharing a column with what follows. A table has every one of
//! those properties — its units are [`crate::table::Slice`]s rather than lines — so it enters the
//! same loop rather than getting a loop of its own, which is what keeps a page's content in one
//! ordered list and therefore keeps `w:keepNext` working *across* a table.
//!
//! The two differences a table brings are named here rather than special-cased inside the loop:
//!
//! * [`BlockLayout::repeated_units`] — a table's `w:tblHeader` rows are redrawn at the top of every
//!   column the table continues into, so a continuation is taller than the units it holds. A
//!   paragraph answers zero, and a paginator that asked a paragraph the question gets the behaviour
//!   it had before this module existed.
//! * [`BlockLayout::widow_control`] — a rule about *lines*, and meaningless for rows. A table
//!   answers `false`, which is not a simplification: refusing to leave one row alone at the foot of a
//!   page is `w:cantSplit`'s job, and the author says which rows.

use std::ops::Range;

use mjx_ooxml_core::measure::Emu;

use crate::flow::ParagraphLayout;
use crate::style::ParagraphStyle;
use crate::table::TableLayout;

/// The pagination-relevant facts about a block, whatever kind it is.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct BlockConstraints {
    /// `w:pageBreakBefore`.
    pub page_break_before: bool,
    /// `w:keepLines` — the block's units may not be separated.
    pub keep_units_together: bool,
    /// `w:keepNext`.
    pub keep_with_next: bool,
    /// `w:widowControl`.
    pub widow_control: bool,
    /// `w:contextualSpacing`.
    pub contextual_spacing: bool,
}

impl BlockConstraints {
    /// The constraints a paragraph's resolved style states.
    #[must_use]
    pub fn of(style: &ParagraphStyle) -> Self {
        Self {
            page_break_before: style.page_break_before,
            keep_units_together: style.keep_lines_together,
            keep_with_next: style.keep_with_next,
            widow_control: style.widow_control,
            contextual_spacing: style.contextual_spacing,
        }
    }
}

/// One block of a content stream, laid out.
#[derive(Clone, PartialEq, Debug)]
pub enum BlockLayout {
    /// A `w:p`, whose units are its lines.
    Paragraph(ParagraphLayout),
    /// A `w:tbl`, whose units are its slices.
    ///
    /// Boxed for the reason [`mjx_docx::BlockFormatting`] boxes its own: a table is two orders of
    /// magnitude larger than a paragraph layout's header, and every cached paragraph would otherwise
    /// pay for it.
    Table(Box<TableLayout>),
}

impl BlockLayout {
    /// How many units it has — lines, or slices.
    #[must_use]
    pub fn unit_count(&self) -> usize {
        match self {
            Self::Paragraph(layout) => layout.lines.len(),
            Self::Table(layout) => layout.slices.len(),
        }
    }

    /// How tall units `units` are.
    #[must_use]
    pub fn height_of(&self, units: Range<usize>) -> Emu {
        match self {
            Self::Paragraph(layout) => layout.height_of(units),
            Self::Table(layout) => layout.height_of(units),
        }
    }

    /// How tall one unit is.
    #[must_use]
    pub fn unit_height(&self, unit: usize) -> Emu {
        self.height_of(unit..unit + 1)
    }

    /// The space above it, before contextual spacing has had its say.
    #[must_use]
    pub fn space_before(&self) -> Emu {
        match self {
            Self::Paragraph(layout) => layout.space_before,
            // A table states no space of its own; §17.4 gives `w:tbl` nothing analogous to
            // `w:spacing/@before`, and Word draws a table hard against the paragraph above it.
            Self::Table(_) => Emu::ZERO,
        }
    }

    /// The space below it.
    #[must_use]
    pub fn space_after(&self) -> Emu {
        match self {
            Self::Paragraph(layout) => layout.space_after,
            Self::Table(_) => Emu::ZERO,
        }
    }

    /// Its pagination constraints.
    #[must_use]
    pub fn constraints(&self) -> BlockConstraints {
        match self {
            Self::Paragraph(layout) => BlockConstraints::of(&layout.style),
            Self::Table(layout) => layout.constraints,
        }
    }

    /// Whether `w:widowControl` applies at all — it is a rule about lines.
    #[must_use]
    pub fn widow_control(&self) -> bool {
        matches!(self, Self::Paragraph(_)) && self.constraints().widow_control
    }

    /// How many leading units are redrawn at the top of every continuation.
    ///
    /// A table's repeating header rows; zero for everything else.
    #[must_use]
    pub fn repeated_units(&self) -> usize {
        match self {
            Self::Paragraph(_) => 0,
            Self::Table(layout) => layout.repeated_slices,
        }
    }

    /// How tall that repeated prefix is.
    #[must_use]
    pub fn repeated_height(&self) -> Emu {
        match self {
            Self::Paragraph(_) => Emu::ZERO,
            Self::Table(layout) => layout.repeated_height(),
        }
    }

    /// The paragraph layout, when it is one.
    #[must_use]
    pub fn as_paragraph(&self) -> Option<&ParagraphLayout> {
        match self {
            Self::Paragraph(layout) => Some(layout),
            Self::Table(_) => None,
        }
    }

    /// The table layout, when it is one.
    #[must_use]
    pub fn as_table(&self) -> Option<&TableLayout> {
        match self {
            Self::Table(layout) => Some(layout),
            Self::Paragraph(_) => None,
        }
    }
}
