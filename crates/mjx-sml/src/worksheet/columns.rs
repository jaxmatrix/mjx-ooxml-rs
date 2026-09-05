//! The sheet's column geometry: the default row height and column width, and the run-length column
//! runs that override them.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_SheetFormatPr` | 2222 | `x:sheetFormatPr` |
//! | `CT_Cols` | 2233 | `x:cols` |
//! | `CT_Col` | 2238 | `x:cols/col` |
//!
//! # `cols` is `maxOccurs="unbounded"`, and merging two blocks changes the file
//!
//! This is the one thing about column geometry that a model gets wrong by being tidy.
//! `CT_Worksheet` declares `cols` **unbounded** (`sml.xsd:2176`), so a worksheet may hold several
//! `<cols>` blocks in a row, and every one of them is a separate element with its own start and end
//! tag. Two blocks holding one `col` each and one block holding two `col`s describe the same column
//! widths and are **different bytes**.
//!
//! So [`WorksheetPart`](crate::WorksheetPart) holds a *list* of [`ColumnBlock`]s and never a merged
//! one. `crates/mjx-sml/tests/worksheet_spine.rs` proves the distinction is load-bearing by merging
//! two blocks and watching the byte-identity assertion fail — which is the mutation the ticket for
//! this child names.
//!
//! # A `col` is a run, not a column
//!
//! `CT_Col` carries `min` and `max`: `<col min="1" max="3" width="6.72"/>` is *three* columns with
//! one width, written once. That is why the type here is [`ColumnRun`] rather than `Column`, and
//! why nothing expands a run into per-column records — a sheet that sets one width for all 16,384
//! columns writes a single element, and expanding it would cost four orders of magnitude for no
//! information.
//!
//! `tests/fixtures/sample.xlsx` writes three runs of one column each, in one block.

use mjx_ooxml_core::{Interner, Number, RawAttribute, RawElement, RawName, RawNode, ToXml};
use mjx_ooxml_types::support::OnOff;

use crate::address::CellSpan;
use crate::error::SmlError;
use crate::leaf::attribute_bag;

use super::frame::WorksheetPart;
use super::rebuild_element;

attribute_bag! {
    /// `x:sheetFormatPr` (`CT_SheetFormatPr`, `sml.xsd:2222`) — the sheet's default row height and
    /// column width, and the outline depth it reaches.
    ///
    /// `defaultRowHeight` is the one attribute the schema declares `use="required"`, so it is
    /// declared required here: a getter reports
    /// [`AttributeError::Missing`](mjx_ooxml_core::AttributeError::Missing) rather than substituting
    /// a height the file does not state.
    ///
    /// `baseColWidth` is a **character count** (the number of `0` glyphs of the Normal style's font
    /// that fit in a column), while `defaultColWidth` is that same count expressed as a fraction —
    /// two units for one quantity, which is why both are carried as the numbers the file wrote and
    /// neither is derived from the other.
    ///
    /// `sample.xlsx` writes `defaultColWidth`, `defaultRowHeight`, `zeroHeight`, `outlineLevelRow`
    /// and `outlineLevelCol`, and writes the element `<sheetFormatPr …></sheetFormatPr>` rather than
    /// self-closing — a distinction the `empty` flag records and a round-trip has to reproduce.
    #[xml(attribute(local = "baseColWidth", codec = Number<u32>, accessor = base_column_character_width, default = 8))]
    #[xml(attribute(local = "defaultColWidth", codec = Number<f64>, accessor = default_column_width))]
    #[xml(attribute(local = "defaultRowHeight", codec = Number<f64>, accessor = default_row_height, required))]
    #[xml(attribute(local = "customHeight", codec = OnOff, accessor = default_row_height_is_custom, default = false))]
    #[xml(attribute(local = "zeroHeight", codec = OnOff, accessor = rows_hidden_by_default, default = false))]
    #[xml(attribute(local = "thickTop", codec = OnOff, accessor = rows_have_thick_top_border, default = false))]
    #[xml(attribute(local = "thickBottom", codec = OnOff, accessor = rows_have_thick_bottom_border, default = false))]
    #[xml(attribute(local = "outlineLevelRow", codec = Number<u8>, accessor = deepest_row_outline_level, default = 0))]
    #[xml(attribute(local = "outlineLevelCol", codec = Number<u8>, accessor = deepest_column_outline_level, default = 0))]
    SheetFormatProperties, "sheetFormatPr"
}

attribute_bag! {
    /// `x:cols/col` (`CT_Col`, `sml.xsd:2238`) — one **run** of columns, `min` through `max`
    /// inclusive, and the width and format they share.
    ///
    /// Both `min` and `max` are `use="required"` and one-based, as `A` is column 1. They are
    /// declared required here for the same reason [`SheetFormatProperties::default_row_height`] is:
    /// a run with no bounds is not a run, and inventing `1` would be inventing markup.
    ///
    /// `@style` is an index into `cellXfs` — the same indirection a cell's `@s` uses, which
    /// MJXOFF-108 (D09) resolves. It is carried here as the number the file wrote.
    ///
    /// `@width` is in the same character-count units as
    /// [`SheetFormatProperties::default_column_width`], and `@customWidth` says whether the user set
    /// it or a consumer computed it — a distinction Excel keeps, so this does too.
    #[xml(attribute(local = "min", codec = Number<u32>, accessor = first_column, required))]
    #[xml(attribute(local = "max", codec = Number<u32>, accessor = last_column, required))]
    #[xml(attribute(local = "width", codec = Number<f64>, accessor = width))]
    #[xml(attribute(local = "style", codec = Number<u32>, accessor = style_index, default = 0))]
    #[xml(attribute(local = "hidden", codec = OnOff, accessor = hidden, default = false))]
    #[xml(attribute(local = "bestFit", codec = OnOff, accessor = best_fit, default = false))]
    #[xml(attribute(local = "customWidth", codec = OnOff, accessor = custom_width, default = false))]
    #[xml(attribute(local = "phonetic", codec = OnOff, accessor = shows_phonetic, default = false))]
    #[xml(attribute(local = "outlineLevel", codec = Number<u8>, accessor = outline_level, default = 0))]
    #[xml(attribute(local = "collapsed", codec = OnOff, accessor = collapsed, default = false))]
    ColumnRun, "col"
}

/// `x:cols` (`CT_Cols`, `sml.xsd:2233`) — **one** block of column runs.
///
/// One block, not all of them: `CT_Worksheet` declares `cols` unbounded, so
/// [`WorksheetPart::column_blocks`](crate::WorksheetPart::column_blocks) hands back every block a
/// worksheet wrote. See this module's own documentation for why merging them is a change to the
/// file rather than a tidy-up.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
#[xml(namespace = SML)]
pub struct ColumnBlock {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "col", variant = Run, ty = ColumnRun))]
    content: Vec<ColumnBlockContent>,
}

/// One child of [`ColumnBlock`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnBlockContent {
    /// `x:col` — a run of columns.
    Run(ColumnRun),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl ColumnBlock {
    /// Builds an empty `x:cols`, bound to `prefix` or to the default namespace.
    ///
    /// The schema declares `col` `minOccurs="1"`, so a block with no runs is invalid; it is still
    /// constructible, because a caller builds one and then fills it.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "cols"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including the ones this type does not model.
    #[must_use]
    pub fn content(&self) -> &[ColumnBlockContent] {
        &self.content
    }

    /// Every `x:col` in this block, in document order.
    pub fn runs(&self) -> impl Iterator<Item = &ColumnRun> + '_ {
        self.content.iter().filter_map(|item| match item {
            ColumnBlockContent::Run(run) => Some(run),
            ColumnBlockContent::Raw(_) => None,
        })
    }

    /// How many `x:col` runs this block holds.
    #[must_use]
    pub fn run_count(&self) -> usize {
        self.runs().count()
    }

    /// The `index`-th `x:col` of this block, mutably.
    pub fn run_mut(&mut self, index: usize) -> Option<&mut ColumnRun> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                ColumnBlockContent::Run(run) => Some(run),
                ColumnBlockContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends a run after the ones already present.
    pub fn push(&mut self, run: ColumnRun) {
        self.content.push(ColumnBlockContent::Run(run));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                ColumnBlockContent::Run(run) => RawNode::Element(run.as_raw_element()),
                ColumnBlockContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for ColumnBlock {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -------------------------------------------------------------------------------------------
// Column geometry as a mutation surface: `ColumnWidth`, and run splitting
// -------------------------------------------------------------------------------------------

/// A column width and the claim the file makes about where it came from.
///
/// The two variants are the two spellings `col` has, and a caller has to pick one — which is the
/// whole point of the type. `@width` and `@customWidth` are **not** independent knobs a caller could
/// forget half of: ECMA-376 Part 1 §18.3.1.13 describes `customWidth` as *"Flag indicating that the
/// column width for the affected column(s) is different from the default or has been manually
/// set"*, so a `width` written without it claims *a consumer computed this to fit*, and Excel is
/// free to recompute it on the next layout. A caller who sets a width and finds Excel ignoring it
/// has been failed by an API that let the two travel apart.
///
/// This type is what stops that: there is no `set_column_width(w: f64)` anywhere in this workspace,
/// because there is no width without the claim beside it.
///
/// The unit is the one `sml.xsd` uses throughout: **characters of the maximum digit width** of the
/// Normal style's font, not points and not EMU. It is the same unit as
/// [`SheetFormatProperties::default_column_width`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnWidth {
    /// `width="…" customWidth="1"` — the width a person set. A consumer honours it.
    Custom(f64),
    /// `width="…"` with no `customWidth` — a width a consumer computed to fit the content, which it
    /// may compute again. This is what Excel writes for an auto-fitted column, so it has to be
    /// expressible; it is not what a caller who wants a particular width should choose.
    Fitted(f64),
}

impl ColumnWidth {
    /// The width itself, in characters of the maximum digit width.
    #[must_use]
    pub fn characters(self) -> f64 {
        match self {
            Self::Custom(width) | Self::Fitted(width) => width,
        }
    }

    /// Whether this width is written with `customWidth="1"`.
    #[must_use]
    pub fn is_custom(self) -> bool {
        matches!(self, Self::Custom(_))
    }
}

/// One run's bounds as the wire states them: **one-based** and inclusive.
type WireSpan = (u32, u32);

/// The wire span a zero-based [`CellSpan`] names, ordered.
///
/// [`CellSpan`] deliberately preserves an inverted `3:1` for `row@spans`, where the file's spelling
/// is the value. Here the span is a *request*, not a value read back, so the two bounds are ordered:
/// there is no meaning to "set the width of columns 3 through 1".
fn wire_span(columns: CellSpan) -> WireSpan {
    let first = columns.first_column().min(columns.last_column());
    let last = columns.first_column().max(columns.last_column());
    (u32::from(first) + 1, u32::from(last) + 1)
}

impl ColumnBlock {
    /// This block's children, mutably — for a caller that has to insert or replace runs rather than
    /// append them.
    ///
    /// The run-splitting in [`WorksheetPart::set_column_width`](crate::WorksheetPart) needs this:
    /// replacing one `col` with the two or three it splits into is a positional edit, and doing it
    /// through [`push`](Self::push) would move every run to the end of the block.
    pub fn content_mut(&mut self) -> &mut Vec<ColumnBlockContent> {
        &mut self.content
    }
}

impl WorksheetPart {
    /// The `col` run covering `column` (zero-based), searched across every `cols` block in document
    /// order.
    ///
    /// `None` when no run covers it, which is the common case: a sheet writes a run only for the
    /// columns that differ from `sheetFormatPr`'s defaults.
    ///
    /// # Errors
    /// [`SmlError::Model`] if a `col` is missing `@min` or `@max`, or wrote one that will not parse.
    /// Both are `use="required"`, and a run with no bounds is not a run this can search.
    pub fn column_run_covering(&self, column: u16) -> Result<Option<&ColumnRun>, SmlError> {
        let wanted = u32::from(column) + 1;
        for block in self.column_blocks() {
            for run in block.runs() {
                let (first, last) = run_bounds(run, self.interner())?;
                if first <= wanted && wanted <= last {
                    return Ok(Some(run));
                }
            }
        }
        Ok(None)
    }

    /// Sets the width of every column in `columns`, splitting any run that reaches outside it.
    ///
    /// `None` removes both `@width` and `@customWidth` from the affected columns, which returns them
    /// to `sheetFormatPr`'s default width.
    ///
    /// See [`ColumnWidth`] for why a width and its `customWidth` flag travel together.
    ///
    /// # The split, in four cases
    ///
    /// `CT_Col` is a **run**: `<col min="1" max="5" width="6.72"/>` is five columns written once. So
    /// changing one column inside a run cannot change the run — it has to break it apart, and how
    /// many pieces it breaks into depends on where the target sits:
    ///
    /// | Case | Run | Target | Result |
    /// |---|---|---|---|
    /// | **Three-way** | `1..=5` | `3` | `1..=2`, `3..=3`, `4..=5` |
    /// | **Left edge** | `1..=5` | `1` | `1..=1`, `2..=5` |
    /// | **Right edge** | `1..=5` | `5` | `1..=4`, `5..=5` |
    /// | **Exact match** | `3..=3` | `3` | `3..=3`, edited in place — no split at all |
    ///
    /// The pieces that fall *outside* the target keep every attribute of the run they came from,
    /// including ones this crate does not model; only the piece inside is handed to `apply`. Getting
    /// that backwards is the failure the ticket for this child names: every column in the sheet
    /// changing width because the whole run was edited.
    ///
    /// Columns of `columns` that **no** run covers get a fresh run each, grouped into maximal
    /// contiguous stretches so that setting one width over ten bare columns writes one `col` and not
    /// ten. A new run goes into the last `cols` block, at the position that keeps the block ascending
    /// by `@min`; a worksheet with no `cols` block at all gets one, at rank 4.
    ///
    /// # Adjacent runs are never merged
    ///
    /// Two runs that end up with identical attributes are left as two. `CT_Worksheet` declares `cols`
    /// `maxOccurs="unbounded"` and `CT_Cols` declares `col` `maxOccurs="unbounded"`, so the number of
    /// elements is part of the file: coalescing `1..=2` and `3..=5` into `1..=5` describes the same
    /// widths in different bytes, and this library does not rewrite bytes nobody asked it to.
    ///
    /// # Edit isolation
    ///
    /// Only the `cols` blocks that actually change give up their verbatim bytes. A worksheet with
    /// three blocks, one of which holds the target column, re-emits the other two straight from the
    /// file — which is why this makes a read-only pass first and touches
    /// [`column_block_mut`](Self::column_block_mut) only for the blocks the pass named.
    ///
    /// # Errors
    /// As [`column_run_covering`](Self::column_run_covering).
    pub fn set_column_width(
        &mut self,
        columns: CellSpan,
        width: Option<ColumnWidth>,
    ) -> Result<(), SmlError> {
        self.edit_column_span(columns, |run, interner| {
            run.set_width(interner, width.map(ColumnWidth::characters));
            run.set_custom_width(
                interner,
                width.map(ColumnWidth::is_custom).filter(|set| *set),
            );
        })
    }

    /// Hides or shows every column in `columns`, splitting any run that reaches outside it.
    ///
    /// `false` **removes** `@hidden` rather than writing `hidden="0"`: the schema's default is
    /// `false`, and the shorter spelling is what Excel writes.
    ///
    /// # Errors
    /// As [`column_run_covering`](Self::column_run_covering).
    pub fn set_column_hidden(&mut self, columns: CellSpan, hidden: bool) -> Result<(), SmlError> {
        self.edit_column_span(columns, |run, interner| {
            run.set_hidden(interner, hidden.then_some(true));
        })
    }

    /// Sets `@bestFit` — whether a consumer should size these columns to their content — splitting
    /// any run that reaches outside `columns`.
    ///
    /// `bestFit` is *not* `customWidth`'s opposite: it asks a consumer to recompute the width, while
    /// [`ColumnWidth`] says where the width currently written came from. A column can carry both.
    ///
    /// # Errors
    /// As [`column_run_covering`](Self::column_run_covering).
    pub fn set_column_best_fit(
        &mut self,
        columns: CellSpan,
        best_fit: bool,
    ) -> Result<(), SmlError> {
        self.edit_column_span(columns, |run, interner| {
            run.set_best_fit(interner, best_fit.then_some(true));
        })
    }

    /// Sets `@collapsed` on every column in `columns`, splitting any run that reaches outside it.
    ///
    /// # Errors
    /// As [`column_run_covering`](Self::column_run_covering).
    pub fn set_column_collapsed(
        &mut self,
        columns: CellSpan,
        collapsed: bool,
    ) -> Result<(), SmlError> {
        self.edit_column_span(columns, |run, interner| {
            run.set_collapsed(interner, collapsed.then_some(true));
        })
    }

    /// Sets `@style` — the `cellXfs` index these columns default to — splitting any run that reaches
    /// outside `columns`.
    ///
    /// `None` removes the attribute, which is index `0`. The index is **not** checked against
    /// `xl/styles.xml`: this crate has never heard of a package, and MJXOFF-108's resolver is where
    /// a dangling index is reported.
    ///
    /// # Errors
    /// As [`column_run_covering`](Self::column_run_covering).
    pub fn set_column_style(
        &mut self,
        columns: CellSpan,
        style: Option<u32>,
    ) -> Result<(), SmlError> {
        self.edit_column_span(columns, |run, interner| {
            run.set_style_index(interner, style);
        })
    }

    /// Sets the outline level of every column in `columns`, splitting any run that reaches outside
    /// it, and raises `sheetFormatPr@outlineLevelCol` when the new level is deeper than the one the
    /// sheet declares.
    ///
    /// ECMA-376 Part 1 says of `outlineLevelCol` that *"these values shall be in synch with the
    /// actual sheet outline levels"*, so writing a level deeper than the declared maximum without
    /// raising it would be authoring the disagreement this library reports in other people's files.
    /// The maximum is only ever **raised**, and only when the sheet has a `sheetFormatPr` to raise
    /// it on: `@defaultRowHeight` is `use="required"`, so authoring that element to record a maximum
    /// would mean inventing a default row height. Lowering it after the deepest column is flattened
    /// is [`recompute_outline_levels`](Self::recompute_outline_levels), which is the caller's ask.
    ///
    /// # Errors
    /// As [`column_run_covering`](Self::column_run_covering).
    pub fn set_column_outline_level(
        &mut self,
        columns: CellSpan,
        level: u8,
    ) -> Result<(), SmlError> {
        self.edit_column_span(columns, |run, interner| {
            run.set_outline_level(interner, (level != 0).then_some(level));
        })?;
        self.raise_column_outline_maximum(level);
        Ok(())
    }

    /// Applies `apply` to exactly the columns of `columns`, and to no others.
    ///
    /// The splitting rule, and everything it guarantees, is documented on
    /// [`set_column_width`](Self::set_column_width) — the public call it is reached through most
    /// often. Every other column setter is one line: this, with a different `apply`.
    fn edit_column_span(
        &mut self,
        columns: CellSpan,
        apply: impl Fn(&mut ColumnRun, &mut Interner),
    ) -> Result<(), SmlError> {
        let (target_first, target_last) = wire_span(columns);

        // Pass one: read-only. Find the runs that overlap the target and the stretches of it that
        // nothing covers, without touching a single block.
        let mut overlaps: Vec<(usize, usize, WireSpan)> = Vec::new();
        let mut covered: Vec<WireSpan> = Vec::new();
        for (block_index, block) in self.column_blocks().enumerate() {
            for (run_index, run) in block.runs().enumerate() {
                let bounds = run_bounds(run, self.interner())?;
                let overlap = (bounds.0.max(target_first), bounds.1.min(target_last));
                if overlap.0 > overlap.1 {
                    continue;
                }
                overlaps.push((block_index, run_index, bounds));
                covered.push(overlap);
            }
        }
        let bare = uncovered_within((target_first, target_last), &mut covered);
        let last_block = self.column_blocks().count().checked_sub(1);

        // Pass two: mutate, block by block, and only the blocks pass one named. Splitting is applied
        // from the highest run index down so the indices pass one recorded stay valid.
        let prefix = self.own_prefix();
        let mut block_indices: Vec<usize> = overlaps.iter().map(|(block, ..)| *block).collect();
        block_indices.sort_unstable();
        block_indices.dedup();

        self.with_interner(|part, interner| {
            for block_index in block_indices {
                let Some(block) = part.column_block_mut(block_index) else {
                    continue;
                };
                let mut in_block: Vec<(usize, WireSpan)> = overlaps
                    .iter()
                    .filter(|(block, ..)| *block == block_index)
                    .map(|(_, run, bounds)| (*run, *bounds))
                    .collect();
                in_block.sort_unstable_by_key(|(run, _)| core::cmp::Reverse(*run));
                for (run_index, bounds) in in_block {
                    split_run_in_place(
                        block,
                        run_index,
                        bounds,
                        (target_first, target_last),
                        interner,
                        &apply,
                    );
                }
            }
        });

        if bare.is_empty() {
            return Ok(());
        }
        if last_block.is_none() {
            let block = ColumnBlock::new(self.interner_mut(), prefix.as_deref());
            self.push_column_block(block);
        }
        let target_block = self
            .column_blocks()
            .count()
            .checked_sub(1)
            .expect("a cols block was just ensured");
        self.with_interner(|part, interner| {
            let block = part
                .column_block_mut(target_block)
                .expect("the block index came from the block count");
            for span in bare {
                let mut run = ColumnRun::new(interner, prefix.as_deref());
                run.set_first_column(interner, span.0);
                run.set_last_column(interner, span.1);
                apply(&mut run, interner);
                insert_run_ascending(block, run, span.0, interner);
            }
        });
        Ok(())
    }

    /// Raises `sheetFormatPr@outlineLevelCol` to `level` if the sheet declares a shallower one.
    ///
    /// Never lowers it, and never authors a `sheetFormatPr` that is not there — see
    /// [`set_column_outline_level`](Self::set_column_outline_level).
    pub(super) fn raise_column_outline_maximum(&mut self, level: u8) {
        self.with_interner(|part, interner| {
            let Some(format) = part.format_properties() else {
                return;
            };
            if format.deepest_column_outline_level(interner).unwrap_or(0) >= level {
                return;
            }
            if let Some(format) = part.format_properties_mut() {
                format.set_deepest_column_outline_level(interner, Some(level));
            }
        });
    }
}

/// One run's `@min` and `@max`, as the wire states them.
fn run_bounds(run: &ColumnRun, interner: &Interner) -> Result<WireSpan, SmlError> {
    let first = run
        .first_column(interner)
        .map_err(mjx_ooxml_core::FromXmlError::from)?;
    let last = run
        .last_column(interner)
        .map_err(mjx_ooxml_core::FromXmlError::from)?;
    Ok((first.min(last), first.max(last)))
}

/// Replaces the run at `run_index` with the up-to-three runs the target splits it into, handing only
/// the middle one to `apply`.
///
/// The pieces outside the target are **clones of the original run** with only `@min`/`@max` rewritten,
/// so a `@style`, a `@phonetic` or an attribute this crate has never modelled survives on them.
fn split_run_in_place(
    block: &mut ColumnBlock,
    run_index: usize,
    bounds: WireSpan,
    target: WireSpan,
    interner: &mut Interner,
    apply: &impl Fn(&mut ColumnRun, &mut Interner),
) {
    let Some(at) = block
        .content()
        .iter()
        .enumerate()
        .filter(|(_, item)| matches!(item, ColumnBlockContent::Run(_)))
        .map(|(at, _)| at)
        .nth(run_index)
    else {
        return;
    };
    let ColumnBlockContent::Run(original) = &block.content()[at] else {
        return;
    };
    let original = original.clone();

    let mut pieces: Vec<ColumnBlockContent> = Vec::with_capacity(3);
    if bounds.0 < target.0 {
        let mut left = original.clone();
        left.set_last_column(interner, target.0 - 1);
        pieces.push(ColumnBlockContent::Run(left));
    }
    let mut middle = original.clone();
    middle.set_first_column(interner, bounds.0.max(target.0));
    middle.set_last_column(interner, bounds.1.min(target.1));
    apply(&mut middle, interner);
    pieces.push(ColumnBlockContent::Run(middle));
    if bounds.1 > target.1 {
        let mut right = original;
        right.set_first_column(interner, target.1 + 1);
        pieces.push(ColumnBlockContent::Run(right));
    }

    block.content_mut().splice(at..=at, pieces);
}

/// Inserts `run` before the first run whose `@min` exceeds `first`, appending when there is none.
///
/// Keeps an ascending block ascending, which is how every producer writes one, and leaves a block
/// that was already out of order no more out of order than it was.
fn insert_run_ascending(block: &mut ColumnBlock, run: ColumnRun, first: u32, interner: &Interner) {
    let at = block
        .content()
        .iter()
        .position(|item| match item {
            ColumnBlockContent::Run(existing) => existing
                .first_column(interner)
                .is_ok_and(|existing| existing > first),
            ColumnBlockContent::Raw(_) => false,
        })
        .unwrap_or(block.content().len());
    block.content_mut().insert(at, ColumnBlockContent::Run(run));
}

/// The stretches of `target` that none of `covered` reaches, as maximal contiguous spans.
///
/// `covered` is sorted in place, which is why it is taken as `&mut [_]`: the caller has no further
/// use for the order it built the list in, and sorting a copy would allocate a second one.
fn uncovered_within(target: WireSpan, covered: &mut [WireSpan]) -> Vec<WireSpan> {
    covered.sort_unstable();
    let mut bare = Vec::new();
    let mut at = target.0;
    for &(first, last) in &*covered {
        if first > at {
            bare.push((at, first - 1));
        }
        at = at.max(last.saturating_add(1));
        if at > target.1 {
            return bare;
        }
    }
    if at <= target.1 {
        bare.push((at, target.1));
    }
    bare
}
