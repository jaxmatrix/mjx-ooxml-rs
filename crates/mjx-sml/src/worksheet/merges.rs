//! Merged ranges: `CT_MergeCells` and `CT_MergeCell`, and what a merge does — and does not — do to
//! the cells beneath it.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_MergeCells` | 2470 | `x:mergeCells` (rank 14 of `CT_Worksheet`) |
//! | `CT_MergeCell` | 2476 | `x:mergeCells/mergeCell` |
//!
//! # A merge is a list, not a property of a cell
//!
//! SpreadsheetML records merging in exactly one place: a flat list of ranges near the end of the
//! worksheet. No cell says "I am merged"; the cells in a merged range are ordinary cells, and the
//! only thing that makes `B2` part of `A1:C3` is that `A1:C3` appears in this list. That is the
//! opposite of DrawingML, where a covered table cell carries `hMerge`/`vMerge` and
//! [`Table::merge_anchor`](https://docs.rs/mjx-dml) walks left and up to find its anchor.
//!
//! The **vocabulary** is nonetheless the one the workspace already settled for DrawingML tables in
//! `crates/mjx-pptx/src/presentation/tables.rs` (MJXOFF-60, A7): a merged region has an *anchor*, a
//! cell is *covered* by a merge, and a cell has a *span*. A reader who has met the PowerPoint
//! surface meets no second dialect here, even though the markup has nothing in common.
//!
//! ECMA-376 Part 1 §18.3.1.55 states the one rule that gives the anchor its meaning: *"The
//! formatting and content for the merged range is always stored in the top left cell."* That is why
//! [`WorksheetPart::merge_anchor`] exists and why MJXOFF-108's
//! [`EffectiveCellFormat`](crate::styles::effective) is resolved for the *anchor* rather than for the
//! cell the caller asked about.
//!
//! # What merging does not do
//!
//! [`WorksheetPart::merge_cells`] adds a range to the list and **touches no cell**. It does not
//! create the cells inside the range, and it does not clear the values of the ones already there.
//!
//! That is deliberate, and it is worth separating the two halves of the reason:
//!
//! * **Creating them would author markup the file does not need.** ECMA-376 Part 1 requires only
//!   that content and formatting live in the top-left cell; it does *not* say the covered cells have
//!   to exist. Excel happens to write them, as empty `<c>` elements carrying a style, but that is a
//!   producer's habit and not a rule this library may enforce on a file somebody else wrote.
//! * **Clearing them would destroy data nobody asked to lose.** A merge laid over populated cells is
//!   a real shape in files Excel wrote — Excel warns and discards on *its* merge command, which is a
//!   user-interface decision, not a file-format one. Here the values are preserved and
//!   [`WorksheetPart::grid_anomalies`](crate::worksheet::GridAnomaly) reports them, which is the
//!   whole family rule this phase is built on.
//!
//! # What merging refuses
//!
//! Two shapes are refused at the door rather than written and left for Excel to repair:
//!
//! * a range that **overlaps a merge already there** — [`SmlError::MergeOverlapsExistingMerge`],
//!   because the alternative is silently producing a workbook Excel opens with a repair dialog;
//! * a **single-cell** range — [`SmlError::DegenerateMerge`], because `<mergeCell ref="A1"/>` merges
//!   nothing and is the same repair.
//!
//! A file that already says either of those keeps saying it: refusing to *author* a shape and
//! refusing to *read* one are different acts, and only the first is this library's business.

use mjx_ooxml_core::{
    Enumeration, Interner, Number, RawAttribute, RawElement, RawName, RawNode, ToXml,
};

use crate::address::{CellRange, CellReference, GridBounds};
use crate::error::SmlError;
use crate::leaf::attribute_bag;

use super::frame::WorksheetPart;
use super::rebuild_element;

attribute_bag! {
    /// `x:mergeCell` (`CT_MergeCell`, `sml.xsd:2476`) — one merged range.
    ///
    /// One attribute, `@ref`, `use="required"`. It is an `ST_Ref`, which is MJXOFF-93's
    /// [`CellRange`] — the same parser `dimension`, `sqref` and `oleSize` go through.
    #[xml(attribute(local = "ref", codec = Enumeration<CellRange>, accessor = range, required))]
    MergedRange, "mergeCell"
}

/// `x:mergeCells` (`CT_MergeCells`, `sml.xsd:2470`) — every merged range in the sheet, in document
/// order.
///
/// The schema declares `mergeCell` `minOccurs="1"`, so a worksheet that writes this element at all
/// writes at least one range. That is why [`WorksheetPart::unmerge_cells`] removes the whole element
/// when it takes the last range out, rather than leaving an empty `<mergeCells/>` the gate would
/// reject.
///
/// `@count` is a hint. It is updated when the collection is edited **and the file declared one**,
/// and never added to an element that wrote none — the rule every counted table in this crate
/// follows (see [`crate::styles`]).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count))]
pub struct MergedCells {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "mergeCell", variant = Merge, ty = MergedRange))]
    content: Vec<MergedCellsContent>,
}

/// One child of [`MergedCells`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergedCellsContent {
    /// `x:mergeCell` — one merged range.
    Merge(MergedRange),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl MergedCells {
    /// Builds an empty `x:mergeCells`, bound to `prefix` or to the default namespace.
    ///
    /// The schema declares `mergeCell` `minOccurs="1"`, so a block with no ranges is invalid; it is
    /// still constructible, because a caller builds one and then fills it.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "mergeCells"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including the ones this type does not model.
    #[must_use]
    pub fn content(&self) -> &[MergedCellsContent] {
        &self.content
    }

    /// Every `x:mergeCell`, in document order.
    pub fn merges(&self) -> impl Iterator<Item = &MergedRange> + '_ {
        self.content.iter().filter_map(|item| match item {
            MergedCellsContent::Merge(merge) => Some(merge),
            MergedCellsContent::Raw(_) => None,
        })
    }

    /// How many `x:mergeCell` children this element holds — the number `@count` claims.
    #[must_use]
    pub fn len(&self) -> usize {
        self.merges().count()
    }

    /// Whether the element holds no merged range at all, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `index`-th `x:mergeCell`, mutably.
    pub fn merge_mut(&mut self, index: usize) -> Option<&mut MergedRange> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                MergedCellsContent::Merge(merge) => Some(merge),
                MergedCellsContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends a range after the ones already present, updating `@count` when the file declared one.
    pub fn push(&mut self, interner: &mut Interner, merge: MergedRange) {
        self.content.push(MergedCellsContent::Merge(merge));
        self.empty = false;
        self.refresh_count(interner);
    }

    /// Removes the `index`-th `x:mergeCell`, updating `@count` when the file declared one.
    ///
    /// `None` when the element holds fewer than `index + 1` ranges. Markup between the ranges is
    /// left exactly where it is: only the range element itself is taken out.
    pub fn remove(&mut self, interner: &mut Interner, index: usize) -> Option<MergedRange> {
        let at = self
            .content
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, MergedCellsContent::Merge(_)))
            .map(|(at, _)| at)
            .nth(index)?;
        let removed = match self.content.remove(at) {
            MergedCellsContent::Merge(merge) => merge,
            MergedCellsContent::Raw(_) => unreachable!("the position was filtered on `Merge`"),
        };
        self.refresh_count(interner);
        Some(removed)
    }

    /// Writes `@count` from the ranges actually present — but only onto an element that already
    /// declared one.
    fn refresh_count(&mut self, interner: &mut Interner) {
        if self.declared_count(interner).ok().flatten().is_some() {
            let count = u32::try_from(self.len()).unwrap_or(u32::MAX);
            self.set_declared_count(interner, Some(count));
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                MergedCellsContent::Merge(merge) => RawNode::Element(merge.as_raw_element()),
                MergedCellsContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for MergedCells {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -------------------------------------------------------------------------------------------
// The merge surface on the worksheet
// -------------------------------------------------------------------------------------------

impl WorksheetPart {
    /// Every merged range in the sheet, in document order.
    ///
    /// # Errors
    /// [`SmlError::Address`] if a `mergeCell@ref` is absent or does not parse. A merge whose
    /// reference cannot be read is not a merge this can answer *around* — every query below is built
    /// on this list, and a silently shortened list would report a covered cell as free.
    /// [`WorksheetPart::grid_anomalies`](crate::worksheet::GridAnomaly) names the offending index
    /// without failing, for a caller who wants to describe the file rather than use it.
    pub fn merged_ranges(&self) -> Result<Vec<CellRange>, SmlError> {
        let Some(merges) = self.merged_cells() else {
            return Ok(Vec::new());
        };
        merges
            .merges()
            .map(|merge| {
                merge
                    .range(self.interner())
                    .map_err(|error| SmlError::Model(error.into()))
            })
            .collect()
    }

    /// The merged range `cell` belongs to, or `None` when it belongs to none.
    ///
    /// Answers for **every** cell of the range, the anchor included — which is what "expose the
    /// merge this cell belongs to" means: a caller holding `C3` of `A1:C3` should not have to know
    /// where the region starts in order to ask about it.
    ///
    /// # Errors
    /// As [`merged_ranges`](Self::merged_ranges).
    pub fn merged_range_containing(
        &self,
        cell: CellReference,
    ) -> Result<Option<CellRange>, SmlError> {
        Ok(self
            .merged_ranges()?
            .into_iter()
            .find(|range| range.contains(cell)))
    }

    /// The cell that actually renders at `cell` — the top-left of the merge covering it, or `cell`
    /// itself when it is covered by none.
    ///
    /// The name is [`Table::merge_anchor`](https://docs.rs/mjx-dml)'s, and so is the meaning. ECMA-376
    /// Part 1 §18.3.1.55: *"The formatting and content for the merged range is always stored in the
    /// top left cell."*
    ///
    /// The anchor is always **relative**, whatever `$` anchoring the range's own `@ref` carried.
    /// `$A$7:$C$7` and `A7:C7` name the same three cells, and an anchor is a position this call
    /// derived rather than a reference the file wrote — carrying a `$` into it would attach a
    /// meaning to a value nobody spelled. A caller that wants the range as the file spelled it has
    /// [`merged_range_containing`](Self::merged_range_containing).
    ///
    /// # Errors
    /// As [`merged_ranges`](Self::merged_ranges).
    pub fn merge_anchor(&self, cell: CellReference) -> Result<CellReference, SmlError> {
        let Some(range) = self.merged_range_containing(cell)? else {
            return Ok(cell);
        };
        let bounds = range.normalized_bounds();
        Ok(CellReference::relative(
            bounds.first_column(),
            bounds.first_row(),
        )?)
    }

    /// Whether `cell` is inside a merged range and is **not** its anchor — a cell the grid shows
    /// nothing of, because the region's content is drawn from its top-left.
    ///
    /// [`TableCell::is_covered_by_merge`](https://docs.rs/mjx-dml)'s name and meaning.
    ///
    /// # Errors
    /// As [`merged_ranges`](Self::merged_ranges).
    pub fn is_covered_by_merge(&self, cell: CellReference) -> Result<bool, SmlError> {
        let anchor = self.merge_anchor(cell)?;
        Ok(anchor.column() != cell.column() || anchor.row() != cell.row())
    }

    /// How many rows and columns the cell at `cell` spans, as `(rows, columns)`.
    ///
    /// `(1, 1)` for an ordinary cell **and for a cell covered by a merge** — ask
    /// [`merge_anchor`](Self::merge_anchor) which cell renders there. That is
    /// [`Slide::cell_span`](https://docs.rs/mjx-pptx)'s rule, restated: a span belongs to the anchor,
    /// and a covered cell reports the one cell it occupies.
    ///
    /// # Errors
    /// As [`merged_ranges`](Self::merged_ranges).
    pub fn cell_span(&self, cell: CellReference) -> Result<(u32, u32), SmlError> {
        let Some(range) = self.merged_range_containing(cell)? else {
            return Ok((1, 1));
        };
        let bounds = range.normalized_bounds();
        if bounds.first_column() != cell.column() || bounds.first_row() != cell.row() {
            return Ok((1, 1));
        }
        Ok(span_of(bounds))
    }

    /// Records `range` as merged.
    ///
    /// **Touches no cell.** See this module's own documentation for why creating the covered cells
    /// and why clearing their values are both refused.
    ///
    /// The range is appended to `x:mergeCells`, which is created at its rank in `CT_Worksheet`'s
    /// sequence if the worksheet has none. `@count` follows the collection when the file declared
    /// one.
    ///
    /// # Errors
    /// [`SmlError::DegenerateMerge`] for a range covering one cell;
    /// [`SmlError::MergeOverlapsExistingMerge`] when a merge already there intersects `range`;
    /// [`SmlError::Address`] if a merge already there has an unreadable `@ref`, since an overlap
    /// cannot be ruled out against a range that will not parse.
    pub fn merge_cells(&mut self, range: CellRange) -> Result<(), SmlError> {
        let bounds = range.normalized_bounds();
        if span_of(bounds) == (1, 1) {
            return Err(SmlError::DegenerateMerge { range });
        }
        if let Some(existing) = self
            .merged_ranges()?
            .into_iter()
            .find(|existing| intersects(existing.normalized_bounds(), bounds))
        {
            return Err(SmlError::MergeOverlapsExistingMerge {
                requested: range,
                existing,
            });
        }

        let prefix = self.own_prefix();
        if self.merged_cells().is_none() {
            let block = MergedCells::new(self.interner_mut(), prefix.as_deref());
            self.set_merged_cells(Some(block));
        }
        self.with_interner(|part, interner| {
            let block = part
                .merged_cells_mut()
                .expect("the mergeCells element was just ensured");
            let mut merge = MergedRange::new(interner, prefix.as_deref());
            merge.set_range(interner, range);
            block.push(interner, merge);
        });
        Ok(())
    }

    /// Removes the merged range whose `@ref` covers exactly the same cells as `range`, reporting
    /// whether one was there.
    ///
    /// The comparison is on the **normalized bounds**, so `C3:A1` unmerges `A1:C3`: the two name one
    /// rectangle, and refusing on the spelling would be refusing on something the file chose.
    ///
    /// When the last range goes, the whole `x:mergeCells` element goes with it — the schema declares
    /// `mergeCell` `minOccurs="1"`, so an empty one is markup no validator accepts.
    ///
    /// # Errors
    /// As [`merged_ranges`](Self::merged_ranges).
    pub fn unmerge_cells(&mut self, range: CellRange) -> Result<bool, SmlError> {
        let wanted = range.normalized_bounds();
        let Some(index) = self
            .merged_ranges()?
            .into_iter()
            .position(|existing| existing.normalized_bounds() == wanted)
        else {
            return Ok(false);
        };
        let emptied = self.with_interner(|part, interner| {
            let block = part
                .merged_cells_mut()
                .expect("a range was just found in the element");
            block.remove(interner, index);
            block.is_empty()
        });
        if emptied {
            self.set_merged_cells(None);
        }
        Ok(true)
    }

    /// Reaches this part's own children while holding its interner mutably.
    ///
    /// The interner and the slots live in one struct, and every model here writes an attribute
    /// through `&mut Interner` while the slot itself is borrowed from `self`. Swapping an empty
    /// interner in for the length of the call is a pointer move, not a rebuild — the same trick
    /// [`WorksheetPart::recompute_dimension`] uses, factored out because four subject modules now
    /// need it.
    ///
    /// **While `edit` runs, the part's own interner is empty.** Every name `edit` needs must come
    /// from the one it is handed; resolving through [`WorksheetPart::interner`] inside the closure
    /// would resolve against nothing.
    pub(super) fn with_interner<R>(
        &mut self,
        edit: impl FnOnce(&mut Self, &mut Interner) -> R,
    ) -> R {
        let mut interner = Interner::default();
        core::mem::swap(&mut interner, self.interner_mut());
        let result = edit(self, &mut interner);
        core::mem::swap(&mut interner, self.interner_mut());
        result
    }
}

/// The `(rows, columns)` a rectangle covers.
fn span_of(bounds: GridBounds) -> (u32, u32) {
    (
        bounds.last_row() - bounds.first_row() + 1,
        u32::from(bounds.last_column() - bounds.first_column()) + 1,
    )
}

/// Whether two rectangles share a cell.
fn intersects(left: GridBounds, right: GridBounds) -> bool {
    left.first_column() <= right.last_column()
        && right.first_column() <= left.last_column()
        && left.first_row() <= right.last_row()
        && right.first_row() <= left.last_row()
}
