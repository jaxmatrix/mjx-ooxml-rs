//! Page breaks: `CT_PageBreak` and `CT_Break`, in the sheet's two axes.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_PageBreak` | 2402 | `x:rowBreaks` (rank 23) **and** `x:colBreaks` (rank 24) |
//! | `CT_Break` | 2409 | `x:rowBreaks/brk`, `x:colBreaks/brk` |
//!
//! # One complex type, two slots
//!
//! `rowBreaks` and `colBreaks` are the same `CT_PageBreak`, so [`PageBreaks`] is one type reached
//! through two accessors. Only the slot says which axis a list belongs to, which is why
//! [`BreakAxis`] exists: a `PageBreaks` value handed around on its own carries no axis, and a
//! function that took one without being told which axis it came from would be a function that could
//! write row breaks into `colBreaks`.
//!
//! # Breaks are grid structure, not print setup
//!
//! `pageMargins`, `pageSetup`, `printOptions` and `headerFooter` are MJXOFF-129's (D17). Breaks
//! stand with them in the reading order of a printed sheet and *not* in the schema's own grouping:
//! `CT_PageBreak` is declared beside `CT_OutlinePr` and `CT_MergeCells` at `sml.xsd:2402`, four
//! hundred lines above `CT_PageSetup`. This child follows the schema, because a break says which
//! rows fall on which page — a fact about the grid — while a margin says how wide the paper is.
//!
//! # `man` is what makes a break the user's
//!
//! A break with `man="1"` was placed by a person; one without was computed by the consumer to fit
//! the paper, and Excel is free to move it on the next repagination. `@count` counts every `brk`,
//! and `@manualBreakCount` counts the subset that carry `man`. Both are hints, kept in step with the
//! collection when it is edited and preserved when it is not — and neither is *added* to an element
//! that wrote none.

use mjx_ooxml_core::{Interner, Number, RawAttribute, RawElement, RawName, RawNode, ToXml};
use mjx_ooxml_types::support::OnOff;

use crate::leaf::attribute_bag;

use super::rebuild_element;

/// Which of `CT_Worksheet`'s two `CT_PageBreak` slots a list of breaks belongs to.
///
/// A [`PageBreaks`] value does not know: both slots hold the identical complex type, and only the
/// element name distinguishes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BreakAxis {
    /// `x:rowBreaks` — breaks *between rows*, each `@id` a row.
    Row,
    /// `x:colBreaks` — breaks *between columns*, each `@id` a column.
    Column,
}

impl BreakAxis {
    /// The wire local name of the slot this axis names.
    #[must_use]
    pub fn wire_local(self) -> &'static str {
        match self {
            Self::Row => "rowBreaks",
            Self::Column => "colBreaks",
        }
    }
}

attribute_bag! {
    /// `x:brk` (`CT_Break`, `sml.xsd:2409`) — one page break.
    ///
    /// Five attributes, every one `use="optional"` with a schema default, so a bare `<brk/>` is
    /// legal markup meaning "an automatic break before row 0 spanning nothing".
    ///
    /// `@id` is the row or column the break falls **before**, one-based on the wire like every other
    /// row and column number in `sml.xsd`, and carried here as the number the file wrote — the axis
    /// is the slot's, see [`BreakAxis`]. `@min` and `@max` bound the break along the *other* axis, so
    /// a full-width row break writes `max="16383"` and a full-height column break `max="1048575"`;
    /// `tests/fixtures/worksheet_spine.xlsx` writes exactly those two.
    ///
    /// `@man` says a person placed the break. `@pt` says the break came from a pivot table, which is
    /// a fact about where the break came from rather than about where it is.
    #[xml(attribute(local = "id", codec = Number<u32>, accessor = at, default = 0))]
    #[xml(attribute(local = "min", codec = Number<u32>, accessor = first, default = 0))]
    #[xml(attribute(local = "max", codec = Number<u32>, accessor = last, default = 0))]
    #[xml(attribute(local = "man", codec = OnOff, accessor = is_manual, default = false))]
    #[xml(attribute(local = "pt", codec = OnOff, accessor = is_from_pivot_table, default = false))]
    PageBreak, "brk"
}

/// `x:rowBreaks` or `x:colBreaks` (`CT_PageBreak`, `sml.xsd:2402`) — one axis's page breaks, in
/// document order.
///
/// The schema declares `brk` `minOccurs="0"`, so unlike [`MergedCells`](super::MergedCells) an empty
/// element here is valid markup and is preserved rather than removed.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count, default = 0))]
#[xml(attribute(local = "manualBreakCount", codec = Number<u32>, accessor = declared_manual_count, default = 0))]
pub struct PageBreaks {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "brk", variant = Break, ty = PageBreak))]
    content: Vec<PageBreaksContent>,
}

/// One child of [`PageBreaks`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageBreaksContent {
    /// `x:brk` — one break.
    Break(PageBreak),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl PageBreaks {
    /// Builds an empty break list for `axis`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>, axis: BreakAxis) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, axis.wire_local()),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including the ones this type does not model.
    #[must_use]
    pub fn content(&self) -> &[PageBreaksContent] {
        &self.content
    }

    /// Every `x:brk`, in document order.
    pub fn breaks(&self) -> impl Iterator<Item = &PageBreak> + '_ {
        self.content.iter().filter_map(|item| match item {
            PageBreaksContent::Break(entry) => Some(entry),
            PageBreaksContent::Raw(_) => None,
        })
    }

    /// How many `x:brk` children this element holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.breaks().count()
    }

    /// Whether the element holds no break at all — legal here, unlike an empty `x:mergeCells`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// How many of the breaks carry `man="1"` — the number `@manualBreakCount` claims.
    #[must_use]
    pub fn manual_count(&self, interner: &Interner) -> usize {
        self.breaks()
            .filter(|entry| entry.is_manual(interner).unwrap_or(false))
            .count()
    }

    /// The `index`-th `x:brk`, mutably.
    pub fn break_mut(&mut self, index: usize) -> Option<&mut PageBreak> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                PageBreaksContent::Break(entry) => Some(entry),
                PageBreaksContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends a break after the ones already present, updating `@count` and `@manualBreakCount`
    /// when the file declared them.
    pub fn push(&mut self, interner: &mut Interner, entry: PageBreak) {
        self.content.push(PageBreaksContent::Break(entry));
        self.empty = false;
        self.refresh_counts(interner);
    }

    /// Removes the `index`-th `x:brk`, updating the two counts when the file declared them.
    ///
    /// `None` when the element holds fewer than `index + 1` breaks.
    pub fn remove(&mut self, interner: &mut Interner, index: usize) -> Option<PageBreak> {
        let at = self
            .content
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, PageBreaksContent::Break(_)))
            .map(|(at, _)| at)
            .nth(index)?;
        let removed = match self.content.remove(at) {
            PageBreaksContent::Break(entry) => entry,
            PageBreaksContent::Raw(_) => unreachable!("the position was filtered on `Break`"),
        };
        self.refresh_counts(interner);
        Some(removed)
    }

    /// Writes `@count` and `@manualBreakCount` from the breaks actually present — each only onto an
    /// element that already declared it.
    ///
    /// The two are decided **independently**: a file that wrote `count` and not `manualBreakCount`
    /// gets `count` back in step and gains nothing it did not have.
    fn refresh_counts(&mut self, interner: &mut Interner) {
        let declared_total = mjx_xml::attribute::find(&self.attributes, interner, None, "count");
        if declared_total.is_some() {
            let count = u32::try_from(self.len()).unwrap_or(u32::MAX);
            self.set_declared_count(interner, Some(count));
        }
        let declared_manual =
            mjx_xml::attribute::find(&self.attributes, interner, None, "manualBreakCount");
        if declared_manual.is_some() {
            let manual = u32::try_from(self.manual_count(interner)).unwrap_or(u32::MAX);
            self.set_declared_manual_count(interner, Some(manual));
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                PageBreaksContent::Break(entry) => RawNode::Element(entry.as_raw_element()),
                PageBreaksContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for PageBreaks {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}
