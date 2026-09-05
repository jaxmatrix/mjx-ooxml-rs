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

use crate::leaf::attribute_bag;

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
