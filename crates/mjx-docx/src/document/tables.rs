//! `w:tbl` (`CT_Tbl`) — the grid, rows, cells, spans and structural edits of a WordprocessingML
//! table.
//!
//! # Word's merge model is a continuation model, not a span model
//!
//! DrawingML's `a:tc` states a whole merged region in one place: `gridSpan`/`rowSpan` attributes on
//! the anchor, and every covered cell is still present, carrying `hMerge`/`vMerge` — a row therefore
//! always holds exactly the grid's column count of `a:tc`, and a cell's position in the row *is* its
//! column index (see `crates/mjx-pptx/src/presentation/tables.rs`, whose ergonomics this module's
//! own `cell_span`/`merge_anchor` mirror — same names, the `(row, column)` argument order, the same
//! `(usize, usize)`/`Option<(usize, usize)>` return shape).
//!
//! WordprocessingML states a *horizontal* span the same way (`w:gridSpan`, ECMA-376 Part 1
//! §17.4.17: *"If this element is omitted, then the number of grid units spanned by this cell shall
//! be assumed to be one."*), but states a *vertical* merge as a **continuation**: the anchor cell
//! carries `w:vMerge w:val="restart"`, and every row it covers below carries its own real `w:tc`
//! stating a bare `w:vMerge` (no `rowSpan` number anywhere). §17.4.84 ("vMerge (Vertically Merged
//! Cell)"), quoted exactly:
//!
//! > If this attribute \[`val`\] is omitted, its value shall be assumed to be continue.
//! >
//! > If this element is omitted, then this cell shall not be part of any vertically merged grouping
//! > of cells, and any vertically merged group of preceding cells shall be closed. If a vertically
//! > merged group of cells do not span the same set of grid columns, then the document is
//! > non-conformant.
//!
//! Annex L.1.5.9 adds the rule this module's own edit code leans on: *"Cells between the first and
//! last merged cell that are part of the vertical merge each must have a vMerge element to continue
//! the vertical merge."*
//!
//! Two consequences follow directly, and both are this ticket's own traps:
//!
//! - **A row's cell count is not its column count.** `w:gridSpan` lets one physical `w:tc` cover
//!   several grid columns with no covered cell created at all, so `(row, column)` addressing must
//!   walk each row's cells accumulating spans ([`Table::resolve_cell`]) rather than indexing
//!   directly — a rectangular, merge-free fixture cannot catch an implementation that conflates the
//!   two, which is why the fixture this ticket asks for is deliberately ragged (`tables.rs` test
//!   `the_row_to_column_map_disagrees_with_cell_index`, and see `tests/tables.rs`).
//! - **Deleting a "covered" cell corrupts the table**, because a covered cell genuinely renders
//!   nothing but is a full sibling `w:tc` the schema requires — removing the wrong row without
//!   rewriting the `w:vMerge` markers either promotes the wrong content or leaves an orphaned
//!   continuation with no anchor above it, exactly the discrepancy [`Table::grid_discrepancies`]
//!   exists to surface rather than let a caller hit silently.
//!
//! `w:hMerge` (`CT_HMerge`) is modeled ([`MergeMarker`], reused for both) purely for round-trip
//! fidelity: **ECMA-376 Part 1 documents no `§17.4.x` prose section for it** (the only "hMerge" hits
//! in Part 1's WordprocessingML reference material are DrawingML's unrelated *attribute* of the same
//! name and raw legacy schema fragments) — modern Word tables express every horizontal merge through
//! `gridSpan` alone, and this module's own `(row, column)` resolution never consults `hMerge`.
//!
//! # The grid invariant, and why this module never panics on it
//!
//! `w:tblGrid` declares the column count; a row's cells are expected to sum, through `gridSpan`, to
//! that count. Real files violate this — a short row, a `vMerge` continuation with no anchor above
//! it, a row with zero cells. [`Table::resolve_cell`] answers `None` rather than panicking whenever a
//! row's own cells do not reach as far as the grid claims, [`Table::merge_anchor`] answers `None`
//! rather than panicking on a broken or bottomless merge chain, and [`Table::grid_discrepancies`]
//! is the active surface for exposing all three malformations a caller (or a test, after a structural
//! edit) can assert against directly.
//!
//! # Recursion depth
//!
//! A table cell holds `Vec<`[`BlockContent`]`>` — the *same* enum [`Body`] and `HdrFtr` hold — so a
//! table nests inside a cell for free, to arbitrary schema-legal depth, with **no depth counter of
//! its own**: the raw tree this crate's `FromXml` walks is already bounded at parse time by
//! `mjx_xml::fidelity::reader::MAXIMUM_DEPTH` (256, measured against stack overflow on both a debug
//! build's 2 MiB thread and an optimised build's 8 MiB one — see that constant's own doc comment),
//! *before* any typed conversion begins. This module's own `FromXml`/`ToXml`/`Drop`/`Clone`
//! recursion for `Table`/`Row`/`Cell`/`BlockContent` can therefore never recurse deeper than that
//! already-enforced bound permits, the same "one bound here bounds every walk written after it"
//! reasoning that constant's doc comment states explicitly. `tests/tables.rs`'s
//! `a_table_nested_three_deep_reads_and_round_trips` exercises three levels — nowhere near the
//! bound — to prove the *reading* path, not the bound itself (which is `mjx-xml`'s own suite's job).

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, Interner, RawAttribute, RawElement, RawName, RawNode,
    Text as TextCodec, ToXml,
};
use mjx_ooxml_types::child_order::CELL_PROPERTIES;
use mjx_ooxml_types::wordprocessingml::DecimalNumber;
pub use mjx_ooxml_types::wordprocessingml::MergedCellType;

use super::body::{
    block_insert_paragraph, block_paragraph, block_paragraph_mut, block_paragraphs,
    block_remove_paragraph, wml_name, BlockContent, Paragraph,
};
use super::paragraph_properties::DecimalNumberValue;

// ---------------------------------------------------------------------------------------------
// Small ordering helpers — mirrors `mjx-dml`'s own `table::{nth_typed_index, typed_insert_index}`
// (`crates/mjx-dml/src/table/mod.rs`) for the same reason: a structural container keeps its typed
// children interleaved with opaque nodes, so "the nth row/cell/column" is not content index `n`.
// Reimplemented rather than imported across the layering boundary — `mjx-dml` sits beside, not
// below, `mjx-docx`; the ~15 lines are the whole of the shared idea.
// ---------------------------------------------------------------------------------------------

/// The index in an interleaved content list of the `nth` (0-based) element matching `is_target`, or
/// `None` when there are fewer than `nth + 1` of them.
fn nth_typed_index<T>(content: &[T], nth: usize, is_target: impl Fn(&T) -> bool) -> Option<usize> {
    content
        .iter()
        .enumerate()
        .filter_map(|(index, item)| is_target(item).then_some(index))
        .nth(nth)
}

/// The content-list index at which to insert so a new element becomes the `nth` (0-based) one
/// matching `is_target`.
fn typed_insert_index<T>(content: &[T], nth: usize, is_target: impl Fn(&T) -> bool) -> usize {
    if let Some(index) = nth_typed_index(content, nth, &is_target) {
        return index;
    }
    content
        .iter()
        .enumerate()
        .filter_map(|(index, item)| is_target(item).then_some(index))
        .next_back()
        .map_or(content.len(), |last| last + 1)
}

// ---------------------------------------------------------------------------------------------
// w:tblGrid (CT_TblGrid / CT_TblGridBase) and w:gridCol (CT_TblGridCol)
// ---------------------------------------------------------------------------------------------

/// `w:gridCol` (`CT_TblGridCol`) — one column of the table grid: an optional width in twips.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "w", prefix = "w", codec = super::run_properties::Twips, accessor = width))]
pub struct GridColumn {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl GridColumn {
    /// A fresh `w:gridCol` with no stated width.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "gridCol"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for GridColumn {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            children: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for GridColumn {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.children.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// One ordered child of a [`Grid`]: a typed [`GridColumn`], or an opaque node (including
/// `w:tblGridChange`, `CT_TblGridChange` — structure-only per this ticket's own scope, so it round-
/// trips byte-for-byte as an unread [`GridContent::Raw`] rather than gaining a type of its own).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridContent {
    /// `w:gridCol` (`CT_TblGridCol`).
    Column(GridColumn),
    /// Any other child — `w:tblGridChange`, whitespace, or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `w:tblGrid` (`CT_TblGrid`, extending `CT_TblGridBase`) — the table's declared column widths, the
/// authority [`Table::column_count`] reads.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Grid {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "gridCol", variant = Column, ty = GridColumn))]
    content: Vec<GridContent>,
}

impl Grid {
    /// A fresh grid of `columns` columns, none stating a width.
    #[must_use]
    pub fn new(interner: &mut Interner, columns: usize) -> Self {
        let content = (0..columns)
            .map(|_| GridContent::Column(GridColumn::new(interner)))
            .collect::<Vec<_>>();
        Self {
            name: wml_name(interner, "tblGrid"),
            attributes: Vec::new(),
            empty: content.is_empty(),
            content,
        }
    }

    /// The grid's columns, in order (opaque children skipped).
    pub fn columns(&self) -> impl Iterator<Item = &GridColumn> {
        self.content.iter().filter_map(|item| match item {
            GridContent::Column(column) => Some(column),
            _ => None,
        })
    }

    /// The number of columns the table declares.
    #[must_use]
    pub fn column_count(&self) -> usize {
        self.columns().count()
    }

    /// The `n`-th column, or `None` if the grid declares fewer.
    #[must_use]
    pub fn column(&self, n: usize) -> Option<&GridColumn> {
        self.columns().nth(n)
    }

    /// The grid's ordered content.
    #[must_use]
    pub fn content(&self) -> &[GridContent] {
        &self.content
    }

    /// Inserts `column` so it becomes the grid's `at`-th column (0-based); `at == column_count`
    /// appends.
    pub fn insert_column_at(&mut self, at: usize, column: GridColumn) {
        let index = typed_insert_index(&self.content, at, |item| {
            matches!(item, GridContent::Column(_))
        });
        self.content.insert(index, GridContent::Column(column));
        self.empty = false;
    }

    /// Removes the grid's `at`-th column and returns it, or `None` if the grid has fewer.
    pub fn remove_column_at(&mut self, at: usize) -> Option<GridColumn> {
        let index = nth_typed_index(&self.content, at, |item| {
            matches!(item, GridContent::Column(_))
        })?;
        match self.content.remove(index) {
            GridContent::Column(column) => Some(column),
            other => {
                self.content.insert(index, other);
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// w:vMerge / w:hMerge (CT_VMerge / CT_HMerge) — one shared shape
// ---------------------------------------------------------------------------------------------

/// `w:vMerge` (`CT_VMerge`) / `w:hMerge` (`CT_HMerge`) — an optional `val` (`ST_Merge`:
/// `continue`/`restart`). Which element this is is `name`, not the Rust type, exactly as
/// [`super::body::Text`] is reused across four `EG_RunInnerContent` members.
///
/// Per ECMA-376 Part 1 §17.4.84: *"If this attribute is omitted, its value shall be assumed to be
/// continue."* [`MergeMarker::effective_kind`] applies that default; [`MergeMarker::kind`] answers
/// the file's own, undefaulted, `val`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<MergedCellType>, accessor = kind))]
pub struct MergeMarker {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl MergeMarker {
    /// Builds a new `local` element (`"vMerge"` or `"hMerge"`). `kind` is written as an explicit
    /// `val` only when given; `None` (the conventional spelling for `continue`, matching Annex
    /// L.1.5.9's own `<w:vMerge/>` example) leaves `val` absent.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str, kind: Option<MergedCellType>) -> Self {
        let mut marker = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        marker.set_kind(interner, kind);
        marker
    }

    /// The file's own `val`, with the ECMA-376 §17.4.84 default applied: `continue` when the element
    /// is present but `val` is absent or unreadable.
    #[must_use]
    pub fn effective_kind(&self, interner: &Interner) -> MergedCellType {
        self.kind(interner)
            .ok()
            .flatten()
            .unwrap_or(MergedCellType::Continue)
    }
}

impl FromXml for MergeMarker {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for MergeMarker {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// ---------------------------------------------------------------------------------------------
// w:tcPr (CT_TcPr) — a cell's properties, gridSpan/hMerge/vMerge typed, everything else raw
// ---------------------------------------------------------------------------------------------

/// One ordered child of [`CellProperties`]: the three structural members this ticket owns, or an
/// opaque node — `cnfStyle`, `tcW`, `tcBorders`, `shd`, `noWrap`, `tcMar`, `textDirection`,
/// `tcFitText`, `vAlign`, `hideMark`, `headers`, `cellIns`/`cellDel`/`cellMerge`, `tcPrChange` all
/// stay [`CellPropertiesContent::Raw`] — MJXOFF-119's (formatting) and MJXOFF-126's (change
/// tracking) own scope, per this ticket's "Not in scope" section, never this module's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellPropertiesContent {
    /// `w:gridSpan` (`CT_DecimalNumber`) — how many grid columns this cell covers.
    GridSpan(DecimalNumberValue),
    /// `w:hMerge` (`CT_HMerge`) — preserved; never consulted by this module's own merge resolution
    /// (see this module's own doc comment for why).
    HorizontalMerge(MergeMarker),
    /// `w:vMerge` (`CT_VMerge`) — the vertical-merge continuation marker this module's `(row,
    /// column)` resolution and structural edits are built on.
    VerticalMerge(MergeMarker),
    /// Any other child — preserved verbatim, in position.
    Raw(RawNode),
}

/// `w:tcPr` (`CT_TcPr`) — a table cell's properties. A fidelity wrapper in the same shape as
/// `run_properties.rs`'s `RunProperties`: three members typed (the ones this ticket's own grid
/// model needs to read and rewrite), everything else opaque and round-tripped exactly as read.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct CellProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "gridSpan", variant = GridSpan, ty = DecimalNumberValue),
        child(local = "hMerge", variant = HorizontalMerge, ty = MergeMarker),
        child(local = "vMerge", variant = VerticalMerge, ty = MergeMarker)
    )]
    content: Vec<CellPropertiesContent>,
}

impl CellProperties {
    /// A fresh, empty `w:tcPr`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "tcPr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Whether this `w:tcPr` states none of the three typed members and preserves no other child
    /// either — the "may as well not be here" state [`Cell::prune_properties_if_empty`] removes.
    #[must_use]
    pub(crate) fn is_fully_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// The schema rank of an existing content item — every unmodeled child is unranked (`None`), so
    /// it never influences where a new typed member is placed, matching `RunProperties::rank`'s own
    /// reasoning (`run_properties.rs`).
    fn rank(item: &CellPropertiesContent) -> Option<u16> {
        match item {
            CellPropertiesContent::GridSpan(_) => CELL_PROPERTIES.rank_of(None, "gridSpan"),
            CellPropertiesContent::HorizontalMerge(_) => CELL_PROPERTIES.rank_of(None, "hMerge"),
            CellPropertiesContent::VerticalMerge(_) => CELL_PROPERTIES.rank_of(None, "vMerge"),
            CellPropertiesContent::Raw(_) => None,
        }
    }

    fn remove(&mut self, is_target: impl Fn(&CellPropertiesContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: CellPropertiesContent) {
        let at = CELL_PROPERTIES.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&CellPropertiesContent) -> bool,
        value: Option<CellPropertiesContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    /// The cell's own `w:gridSpan`, or `None` if it states none.
    #[must_use]
    pub fn grid_span(&self) -> Option<&DecimalNumberValue> {
        self.content.iter().find_map(|item| match item {
            CellPropertiesContent::GridSpan(value) => Some(value),
            _ => None,
        })
    }

    /// How many grid columns this cell covers: the file's own `w:gridSpan`, or `1` (ECMA-376 Part 1
    /// §17.4.17's own default) when absent or unreadable — never below `1`.
    #[must_use]
    pub fn column_span(&self, interner: &Interner) -> usize {
        self.grid_span()
            .and_then(|value| value.value(interner).ok())
            .filter(|span| *span >= 1)
            .map_or(1, |span| span as usize)
    }

    /// Sets (or, given `None` or `Some(1)`, removes) `w:gridSpan` — a span of one is the schema
    /// default, so it is never written, matching `mjx-dml`'s own `TableCell::set_spans`.
    pub fn set_column_span(&mut self, interner: &mut Interner, span: Option<usize>) {
        let is_target =
            |item: &CellPropertiesContent| matches!(item, CellPropertiesContent::GridSpan(_));
        match span.filter(|span| *span > 1) {
            None => self.remove(is_target),
            Some(span) => {
                let value = DecimalNumberValue::new(interner, "gridSpan", span as DecimalNumber);
                self.set(
                    "gridSpan",
                    is_target,
                    Some(CellPropertiesContent::GridSpan(value)),
                );
            }
        }
    }

    /// The cell's own `w:vMerge`, or `None` if it states none (not part of any vertical merge).
    #[must_use]
    pub fn vertical_merge(&self) -> Option<&MergeMarker> {
        self.content.iter().find_map(|item| match item {
            CellPropertiesContent::VerticalMerge(marker) => Some(marker),
            _ => None,
        })
    }

    /// The cell's effective vertical-merge state: `None` if `w:vMerge` is absent (not merged —
    /// ECMA-376 Part 1 §17.4.84's own "closes" rule); `Some(kind)`, defaulted, otherwise.
    #[must_use]
    pub fn vertical_merge_kind(&self, interner: &Interner) -> Option<MergedCellType> {
        self.vertical_merge()
            .map(|marker| marker.effective_kind(interner))
    }

    /// Sets (or, given `None`, removes) `w:vMerge`. `Some(MergedCellType::Continue)` writes the
    /// bare, conventional spelling (`val` absent); `Some(MergedCellType::Restart)` writes `val`
    /// explicitly.
    pub fn set_vertical_merge(&mut self, interner: &mut Interner, kind: Option<MergedCellType>) {
        let is_target =
            |item: &CellPropertiesContent| matches!(item, CellPropertiesContent::VerticalMerge(_));
        match kind {
            None => self.remove(is_target),
            Some(kind) => {
                let stated = (kind != MergedCellType::Continue).then_some(kind);
                let marker = MergeMarker::new(interner, "vMerge", stated);
                self.set(
                    "vMerge",
                    is_target,
                    Some(CellPropertiesContent::VerticalMerge(marker)),
                );
            }
        }
    }

    /// The cell's own `w:hMerge`, or `None` — preserved for fidelity; see this module's own doc
    /// comment for why nothing here treats it as load-bearing.
    #[must_use]
    pub fn horizontal_merge(&self) -> Option<&MergeMarker> {
        self.content.iter().find_map(|item| match item {
            CellPropertiesContent::HorizontalMerge(marker) => Some(marker),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:hMerge`, in the same shape as
    /// [`set_vertical_merge`](Self::set_vertical_merge).
    pub fn set_horizontal_merge(&mut self, interner: &mut Interner, kind: Option<MergedCellType>) {
        let is_target = |item: &CellPropertiesContent| {
            matches!(item, CellPropertiesContent::HorizontalMerge(_))
        };
        match kind {
            None => self.remove(is_target),
            Some(kind) => {
                let stated = (kind != MergedCellType::Continue).then_some(kind);
                let marker = MergeMarker::new(interner, "hMerge", stated);
                self.set(
                    "hMerge",
                    is_target,
                    Some(CellPropertiesContent::HorizontalMerge(marker)),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// w:tc (CT_Tc) — a table cell
// ---------------------------------------------------------------------------------------------

/// `w:tc` (`CT_Tc`) — one table cell: an optional `w:tcPr`, then `EG_BlockLevelElts+` (paragraphs
/// and, recursively, nested tables) — reusing [`BlockContent`] itself, MJXOFF-92's own type
/// generalized by MJXOFF-113 into the shared `block_*` free functions `body.rs` exposes, exactly as
/// `HdrFtr` (`headers.rs`) already does. A cell is this generalization's **third** container, not a
/// new one: [`Cell::paragraph`]/[`Cell::insert_paragraph`]/[`Cell::remove_paragraph`] below call the
/// *same* `block_paragraph`/`block_insert_paragraph`/`block_remove_paragraph` functions `Body` and
/// `HdrFtr` call, over `self.content` — no cell-specific copy of the paragraph API exists.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "id", prefix = "w", codec = TextCodec, accessor = id))]
pub struct Cell {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "tcPr", variant = Properties, ty = CellProperties),
        child(local = "customXml", variant = CustomXml, ty = super::body::Unmodeled),
        child(local = "sdt", variant = StructuredDocumentTag, ty = super::body::Unmodeled),
        child(local = "p", variant = Paragraph, ty = Paragraph),
        child(local = "tbl", variant = Table, ty = Table),
        child(local = "proofErr", variant = ProofingError, ty = super::body::ProofingError),
        child(local = "permStart", variant = PermissionRangeStart, ty = super::body::PermissionRangeStart),
        child(local = "permEnd", variant = PermissionRangeEnd, ty = super::body::PermissionRangeEnd),
        child(local = "sectPr", variant = SectionProperties, ty = super::sections::SectionProperties)
    )]
    content: Vec<BlockContent>,
}

impl Cell {
    /// A fresh `w:tc` holding one empty paragraph — `EG_BlockLevelElts` is `minOccurs="1"`, so an
    /// empty cell is not schema-legal; matches `HdrFtr::new`'s own reasoning.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "tc"),
            attributes: Vec::new(),
            empty: false,
            content: vec![BlockContent::Paragraph(Paragraph::new(interner))],
        }
    }

    /// The cell's own `w:tcPr`, or `None` if it declares none.
    #[must_use]
    pub fn properties(&self) -> Option<&CellProperties> {
        self.content.iter().find_map(|item| match item {
            BlockContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// [`Cell::properties`], mutably.
    pub fn properties_mut(&mut self) -> Option<&mut CellProperties> {
        self.content.iter_mut().find_map(|item| match item {
            BlockContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// Sets (replaces) or removes `w:tcPr` — always at content index `0`, `CT_Tc`'s own
    /// `xsd:sequence` position for it.
    pub fn set_properties(&mut self, properties: Option<CellProperties>) {
        let at = self
            .content
            .iter()
            .position(|item| matches!(item, BlockContent::Properties(_)));
        match (at, properties) {
            (Some(at), Some(properties)) => self.content[at] = BlockContent::Properties(properties),
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(properties)) => {
                self.content.insert(0, BlockContent::Properties(properties))
            }
            (None, None) => {}
        }
    }

    /// [`Cell::properties_mut`], creating an empty `w:tcPr` first if the cell had none.
    pub fn properties_or_insert(&mut self, interner: &mut Interner) -> &mut CellProperties {
        if self.properties().is_none() {
            self.set_properties(Some(CellProperties::new(interner)));
        }
        match self.properties_mut() {
            Some(properties) => properties,
            None => unreachable!("just inserted above"),
        }
    }

    /// Removes `w:tcPr` if it is present but states nothing at all — the "may as well not be here"
    /// state a `set_column_span(None)`/`set_vertical_merge(None)` on an otherwise-bare properties
    /// element can leave behind.
    fn prune_properties_if_empty(&mut self) {
        if self
            .properties()
            .is_some_and(CellProperties::is_fully_empty)
        {
            self.set_properties(None);
        }
    }

    /// How many grid columns this cell covers (`w:tcPr/w:gridSpan`, defaulted); `1` when the cell
    /// has no `w:tcPr` at all.
    #[must_use]
    pub fn column_span(&self, interner: &Interner) -> usize {
        self.properties()
            .map_or(1, |properties| properties.column_span(interner))
    }

    /// Sets this cell's `w:gridSpan`, creating `w:tcPr` first if needed and removing it again if
    /// that leaves it fully empty.
    pub fn set_column_span(&mut self, interner: &mut Interner, span: Option<usize>) {
        if span.filter(|span| *span > 1).is_none() && self.properties().is_none() {
            return;
        }
        self.properties_or_insert(interner)
            .set_column_span(interner, span);
        self.prune_properties_if_empty();
    }

    /// This cell's effective vertical-merge state (`w:tcPr/w:vMerge`, defaulted); `None` when the
    /// cell has no `w:tcPr` at all.
    #[must_use]
    pub fn vertical_merge_kind(&self, interner: &Interner) -> Option<MergedCellType> {
        self.properties()
            .and_then(|properties| properties.vertical_merge_kind(interner))
    }

    /// Whether this cell is a vertical-merge **continuation** — covered by an anchor above it and
    /// rendering nothing of its own.
    #[must_use]
    pub fn is_covered_by_vertical_merge(&self, interner: &Interner) -> bool {
        self.vertical_merge_kind(interner) == Some(MergedCellType::Continue)
    }

    /// Whether this cell is the **anchor** of a vertical merge (`w:vMerge w:val="restart"`).
    #[must_use]
    pub fn is_vertical_merge_anchor(&self, interner: &Interner) -> bool {
        self.vertical_merge_kind(interner) == Some(MergedCellType::Restart)
    }

    /// Sets (or, given `None`, removes) this cell's `w:vMerge`.
    pub fn set_vertical_merge(&mut self, interner: &mut Interner, kind: Option<MergedCellType>) {
        if kind.is_none() && self.properties().is_none() {
            return;
        }
        self.properties_or_insert(interner)
            .set_vertical_merge(interner, kind);
        self.prune_properties_if_empty();
    }

    /// Every paragraph directly in this cell (not inside a nested table), in document order.
    pub fn paragraphs(&self) -> impl Iterator<Item = &Paragraph> {
        block_paragraphs(&self.content)
    }

    /// How many paragraphs this cell holds directly.
    #[must_use]
    pub fn paragraph_count(&self) -> usize {
        self.paragraphs().count()
    }

    /// The paragraph at `path`, or `None` if the address is out of range.
    #[must_use]
    pub fn paragraph(&self, path: impl Into<crate::address::BlockPath>) -> Option<&Paragraph> {
        block_paragraph(&self.content, &path.into())
    }

    /// [`Cell::paragraph`], mutably.
    pub fn paragraph_mut(
        &mut self,
        path: impl Into<crate::address::BlockPath>,
    ) -> Option<&mut Paragraph> {
        block_paragraph_mut(&mut self.content, &path.into())
    }

    /// Inserts `paragraph` so it becomes the paragraph at `path`, shifting later paragraphs one
    /// place later. `path` must address an existing paragraph slot or one past the last.
    #[must_use]
    pub fn insert_paragraph(
        &mut self,
        path: impl Into<crate::address::BlockPath>,
        paragraph: Paragraph,
    ) -> bool {
        let end = self.content.len();
        block_insert_paragraph(&mut self.content, &path.into(), paragraph, || end)
    }

    /// Appends `paragraph` as this cell's new last paragraph.
    pub fn append_paragraph(&mut self, paragraph: Paragraph) {
        self.content.push(BlockContent::Paragraph(paragraph));
    }

    /// Removes and returns the paragraph at `path`, or `None` if the address is out of range.
    pub fn remove_paragraph(
        &mut self,
        path: impl Into<crate::address::BlockPath>,
    ) -> Option<Paragraph> {
        block_remove_paragraph(&mut self.content, &path.into())
    }

    /// This cell's text — each direct paragraph joined by a newline.
    #[must_use]
    pub fn text(&self) -> String {
        self.paragraphs()
            .map(Paragraph::text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Every table nested directly in this cell (not inside a deeper nested table).
    pub fn tables(&self) -> impl Iterator<Item = &Table> {
        self.content.iter().filter_map(|item| match item {
            BlockContent::Table(table) => Some(table),
            _ => None,
        })
    }

    /// The cell's whole ordered content, including its `w:tcPr` when it has one.
    #[must_use]
    pub fn content(&self) -> &[BlockContent] {
        &self.content
    }

    /// Replaces the cell's whole ordered content wholesale — paragraphs, nested tables and `w:tcPr`
    /// together. This is how a cell **promoted** to a vertical-merge anchor takes over the old
    /// anchor's content in one move ([`Table::remove_row`]): the promoted cell's own previously
    /// hidden content is discarded in favour of what was rendering there, and the anchor's `w:tcPr`
    /// (borders, shading, `w:gridSpan`, …) transfers with it — `set_vertical_merge` is then applied
    /// afterward to give the promoted cell its own, correct merge state.
    pub(crate) fn set_content(&mut self, content: Vec<BlockContent>) {
        self.empty = content.is_empty();
        self.content = content;
    }
}

/// One ordered child of a [`Row`]: a typed [`Cell`], or an opaque node (`w:tblPrEx`, `w:trPr` —
/// MJXOFF-119's own scope — a row-level `w:customXml`/`w:sdt` wrapper, or `EG_RunLevelElts` folded
/// into the row's own choice group).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowContent {
    /// `w:tc` (`CT_Tc`).
    Cell(Cell),
    /// Any other child — preserved verbatim, in position.
    Raw(RawNode),
}

/// `w:tr` (`CT_Row`) — one table row: its cells, in physical (not necessarily column) order — see
/// this module's own doc comment for why a row's cell count is not the grid's column count.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Row {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "tc", variant = Cell, ty = Cell))]
    content: Vec<RowContent>,
}

impl Row {
    /// A fresh row of `cells`.
    #[must_use]
    pub fn new(interner: &mut Interner, cells: Vec<Cell>) -> Self {
        let content = cells.into_iter().map(RowContent::Cell).collect::<Vec<_>>();
        Self {
            name: wml_name(interner, "tr"),
            attributes: Vec::new(),
            empty: content.is_empty(),
            content,
        }
    }

    /// The row's cells, in physical order (opaque children skipped).
    pub fn cells(&self) -> impl Iterator<Item = &Cell> {
        self.content.iter().filter_map(|item| match item {
            RowContent::Cell(cell) => Some(cell),
            _ => None,
        })
    }

    /// The row's cells, mutably, in physical order.
    pub fn cells_mut(&mut self) -> impl Iterator<Item = &mut Cell> {
        self.content.iter_mut().filter_map(|item| match item {
            RowContent::Cell(cell) => Some(cell),
            _ => None,
        })
    }

    /// How many `w:tc` this row physically holds — **not** the column count when any cell spans
    /// more than one grid column; see [`Table::column_count`].
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.cells().count()
    }

    /// The cell at physical index `n`, or `None` if the row has fewer.
    #[must_use]
    pub fn cell(&self, n: usize) -> Option<&Cell> {
        self.cells().nth(n)
    }

    /// [`Row::cell`], mutably.
    pub fn cell_mut(&mut self, n: usize) -> Option<&mut Cell> {
        self.cells_mut().nth(n)
    }

    /// The row's ordered content.
    #[must_use]
    pub fn content(&self) -> &[RowContent] {
        &self.content
    }

    /// Inserts `cell` so it becomes the row's `at`-th physical cell (0-based); `at == cell_count`
    /// appends.
    pub fn insert_cell_at(&mut self, at: usize, cell: Cell) {
        let index = typed_insert_index(&self.content, at, |item| {
            matches!(item, RowContent::Cell(_))
        });
        self.content.insert(index, RowContent::Cell(cell));
        self.empty = false;
    }

    /// Removes the row's `at`-th physical cell and returns it, or `None` if the row has fewer.
    pub fn remove_cell_at(&mut self, at: usize) -> Option<Cell> {
        let index = nth_typed_index(&self.content, at, |item| {
            matches!(item, RowContent::Cell(_))
        })?;
        match self.content.remove(index) {
            RowContent::Cell(cell) => Some(cell),
            other => {
                self.content.insert(index, other);
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// w:tbl (CT_Tbl) — the table itself
// ---------------------------------------------------------------------------------------------

/// One ordered child of a [`Table`]: its grid or a row, or an opaque node — `w:tblPr` (MJXOFF-119's
/// own scope), `EG_RangeMarkupElements` (bookmarks and revision ranges ahead of `w:tblPr`), a
/// row-level `w:customXml`/`w:sdt` wrapper (`CT_SdtRow` — MJXOFF-138), and `EG_RunLevelElts` folded
/// into `EG_ContentRowContent`'s own choice all fall to [`TableContent::Raw`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableContent {
    /// `w:tblPr` (`CT_TblPr`) — required by the schema, opaque here; MJXOFF-119 types it.
    Properties(super::body::Unmodeled),
    /// `w:tblGrid` (`CT_TblGrid`).
    Grid(Grid),
    /// `w:tr` (`CT_Row`).
    Row(Row),
    /// Any other child — preserved verbatim, in position.
    Raw(RawNode),
}

/// `w:tbl` (`CT_Tbl`) — a table: its grid and rows.
///
/// # Dimensions
///
/// The column count is the **grid's** (`w:tblGrid` — [`Table::column_count`]), exactly as
/// `mjx-pptx`'s own `Table::column_count` reads its `a:tblGrid`. A row whose cells' spans do not sum
/// to that count is malformed — [`Table::grid_discrepancies`] reports it; nothing here corrects it.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Table {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "tblPr", variant = Properties, ty = super::body::Unmodeled),
        child(local = "tblGrid", variant = Grid, ty = Grid),
        child(local = "tr", variant = Row, ty = Row)
    )]
    content: Vec<TableContent>,
}

impl Table {
    /// A fresh `rows` x `columns` table: an empty `w:tblPr`, a grid of `columns` widthless columns,
    /// and `rows` rows each holding `columns` fresh, empty cells.
    ///
    /// # Panics
    /// Never — `rows`/`columns` of `0` build a table with no rows/cells, which is schema-invalid to
    /// *write* but this constructor does not refuse it; the caller who authors a table (`Document`)
    /// is where that refusal belongs, matching `mjx_pptx::Presentation::add_table`'s own split
    /// between the model (which builds whatever it is asked to) and the facade (which validates).
    #[must_use]
    pub fn new(interner: &mut Interner, rows: usize, columns: usize) -> Self {
        let properties = super::body::Unmodeled::new(interner, "tblPr");
        let grid = Grid::new(interner, columns);
        let table_rows = (0..rows)
            .map(|_| {
                let cells = (0..columns).map(|_| Cell::new(interner)).collect();
                TableContent::Row(Row::new(interner, cells))
            })
            .collect::<Vec<_>>();
        let mut content = vec![
            TableContent::Properties(properties),
            TableContent::Grid(grid),
        ];
        content.extend(table_rows);
        Self {
            name: wml_name(interner, "tbl"),
            attributes: Vec::new(),
            empty: false,
            content,
        }
    }

    /// The table's `w:tblPr`, or `None` if it declares none (schema-required, but never rejected
    /// on read).
    #[must_use]
    pub fn properties(&self) -> Option<&super::body::Unmodeled> {
        self.content.iter().find_map(|item| match item {
            TableContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The table's `w:tblGrid`, or `None` if it declares none (schema-required, but never rejected
    /// on read).
    #[must_use]
    pub fn grid(&self) -> Option<&Grid> {
        self.content.iter().find_map(|item| match item {
            TableContent::Grid(grid) => Some(grid),
            _ => None,
        })
    }

    /// [`Table::grid`], mutably.
    pub fn grid_mut(&mut self) -> Option<&mut Grid> {
        self.content.iter_mut().find_map(|item| match item {
            TableContent::Grid(grid) => Some(grid),
            _ => None,
        })
    }

    /// The table's rows, in order (opaque children skipped).
    pub fn rows(&self) -> impl Iterator<Item = &Row> {
        self.content.iter().filter_map(|item| match item {
            TableContent::Row(row) => Some(row),
            _ => None,
        })
    }

    /// The table's rows, mutably, in order.
    pub fn rows_mut(&mut self) -> impl Iterator<Item = &mut Row> {
        self.content.iter_mut().filter_map(|item| match item {
            TableContent::Row(row) => Some(row),
            _ => None,
        })
    }

    /// The number of rows.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows().count()
    }

    /// The number of columns, as the **grid** declares it; `0` if the table has no `w:tblGrid`.
    #[must_use]
    pub fn column_count(&self) -> usize {
        self.grid().map_or(0, Grid::column_count)
    }

    /// The row at `index`, or `None`.
    #[must_use]
    pub fn row(&self, index: usize) -> Option<&Row> {
        self.rows().nth(index)
    }

    /// [`Table::row`], mutably.
    pub fn row_mut(&mut self, index: usize) -> Option<&mut Row> {
        self.rows_mut().nth(index)
    }

    /// The table's ordered content.
    #[must_use]
    pub fn content(&self) -> &[TableContent] {
        &self.content
    }

    // -------------------------------------------------------------------------------------------
    // (row, column) resolution — the grid-aware walk this ticket's own trap names: a row's cell
    // count is not its column count, so this accumulates gridSpan rather than indexing directly.
    // -------------------------------------------------------------------------------------------

    /// The physical cell covering grid column `column` of `row`, together with the grid column its
    /// span *starts* at — `None` if `row`/`column` is out of range **or** the row's own cells (their
    /// spans summed) do not reach that far, which is the grid-discrepancy case exposed rather than
    /// panicked on.
    fn resolve_cell(
        &self,
        interner: &Interner,
        row: usize,
        column: usize,
    ) -> Option<(usize, &Cell)> {
        if column >= self.column_count() {
            return None;
        }
        let row_ref = self.row(row)?;
        let mut at = 0usize;
        for cell in row_ref.cells() {
            let span = cell.column_span(interner).max(1);
            if column < at + span {
                return Some((at, cell));
            }
            at += span;
        }
        None
    }

    /// The physical index within `row`'s cell list of the cell covering grid column `column`, or
    /// `None` under the same conditions as [`resolve_cell`](Self::resolve_cell).
    fn resolve_physical_index(
        &self,
        interner: &Interner,
        row: usize,
        column: usize,
    ) -> Option<usize> {
        if column >= self.column_count() {
            return None;
        }
        let row_ref = self.row(row)?;
        let mut at = 0usize;
        for (index, cell) in row_ref.cells().enumerate() {
            let span = cell.column_span(interner).max(1);
            if column < at + span {
                return Some(index);
            }
            at += span;
        }
        None
    }

    /// The cell at `(row, column)`, or `None` if the address is out of range or the row is too
    /// short to reach it. **`column` is a grid column, not a physical cell index** — a cell spanning
    /// several grid columns is returned for any `column` its span covers.
    #[must_use]
    pub fn cell(&self, interner: &Interner, row: usize, column: usize) -> Option<&Cell> {
        self.resolve_cell(interner, row, column)
            .map(|(_, cell)| cell)
    }

    /// [`Table::cell`], mutably.
    pub fn cell_mut(
        &mut self,
        interner: &Interner,
        row: usize,
        column: usize,
    ) -> Option<&mut Cell> {
        let physical_index = self.resolve_physical_index(interner, row, column)?;
        self.row_mut(row)?.cell_mut(physical_index)
    }

    /// How many consecutive rows, starting at `anchor_row`, a vertical-merge region covers at grid
    /// column `column` — `1` for an unmerged cell or a broken/malformed chain (this never panics and
    /// never under- or over-counts past what the file actually states).
    fn vertical_run_length(&self, interner: &Interner, anchor_row: usize, column: usize) -> usize {
        let Some((start_column, _)) = self.resolve_cell(interner, anchor_row, column) else {
            return 1;
        };
        let mut length = 1;
        let mut row = anchor_row + 1;
        while let Some((next_start, cell)) = self.resolve_cell(interner, row, start_column) {
            if next_start != start_column
                || cell.vertical_merge_kind(interner) != Some(MergedCellType::Continue)
            {
                break;
            }
            length += 1;
            row += 1;
        }
        length
    }

    /// How many rows and columns the cell at `(row, column)` spans, as `(rows, columns)` — the same
    /// order and shape `mjx_pptx::Presentation::cell_span` answers in (see
    /// `crates/mjx-pptx/src/presentation/tables.rs`).
    ///
    /// `(1, 1)` for an ordinary cell. A cell **covered** by a vertical merge also reports `(1, 1)` —
    /// ask [`merge_anchor`](Self::merge_anchor) which cell actually renders there — matching
    /// PowerPoint's own contract exactly, though for a different underlying reason: a Word
    /// continuation cell simply carries no span number of its own to read.
    #[must_use]
    pub fn cell_span(
        &self,
        interner: &Interner,
        row: usize,
        column: usize,
    ) -> Option<(usize, usize)> {
        let (start_column, cell) = self.resolve_cell(interner, row, column)?;
        let column_span = cell.column_span(interner);
        let row_span = if cell.is_vertical_merge_anchor(interner) {
            self.vertical_run_length(interner, row, start_column)
        } else {
            1
        };
        Some((row_span, column_span))
    }

    /// Which cell actually renders at `(row, column)`: itself when it is not a vertical-merge
    /// continuation, or the anchor of the merged region covering it — `None` when `(row, column)` is
    /// out of range, **or** the continuation's chain is broken (no reachable `restart` above it,
    /// closed by an intervening row with no `w:vMerge` at all — ECMA-376 Part 1 §17.4.84's own
    /// "closes" rule) — the malformed-file case exposed rather than panicked on.
    ///
    /// Same name, `(row, column)` argument order and `Option<(usize, usize)>` return shape as
    /// `mjx_pptx::Presentation::merged_cell_anchor`.
    #[must_use]
    pub fn merge_anchor(
        &self,
        interner: &Interner,
        row: usize,
        column: usize,
    ) -> Option<(usize, usize)> {
        let (start_column, cell) = self.resolve_cell(interner, row, column)?;
        if cell.vertical_merge_kind(interner) != Some(MergedCellType::Continue) {
            return Some((row, start_column));
        }
        let mut at_row = row;
        loop {
            at_row = at_row.checked_sub(1)?;
            let (above_start, above_cell) = self.resolve_cell(interner, at_row, start_column)?;
            match above_cell.vertical_merge_kind(interner) {
                Some(MergedCellType::Restart) => return Some((at_row, above_start)),
                Some(MergedCellType::Continue) => continue,
                None => return None,
            }
        }
    }

    /// Every discrepancy this table's grid currently has — a row whose cells' spans do not sum to
    /// the declared column count, a vertical-merge continuation with no reachable anchor above it,
    /// and a row with zero cells. Never panics; this is the active surface for "expose the
    /// discrepancy" this ticket's own brief asks for, meant to be asserted empty after a structural
    /// edit (`tests/tables.rs` does exactly that after every insert/remove).
    #[must_use]
    pub fn grid_discrepancies(&self, interner: &Interner) -> Vec<GridDiscrepancy> {
        let columns = self.column_count();
        let mut found = Vec::new();
        for (row_index, row) in self.rows().enumerate() {
            if row.cell_count() == 0 {
                found.push(GridDiscrepancy::EmptyRow { row: row_index });
                continue;
            }
            let spanned_columns: usize = row
                .cells()
                .map(|cell| cell.column_span(interner).max(1))
                .sum();
            if spanned_columns != columns {
                found.push(GridDiscrepancy::RowWidthMismatch {
                    row: row_index,
                    declared_columns: columns,
                    spanned_columns,
                });
            }
            let mut at = 0usize;
            for cell in row.cells() {
                let span = cell.column_span(interner).max(1);
                if cell.is_covered_by_vertical_merge(interner)
                    && self.merge_anchor(interner, row_index, at).is_none()
                {
                    found.push(GridDiscrepancy::OrphanedVerticalMerge {
                        row: row_index,
                        column: at,
                    });
                }
                at += span;
            }
        }
        found
    }

    // -------------------------------------------------------------------------------------------
    // Structural edits — insert and remove whole rows and columns, keeping every merge coherent.
    //
    // Word's continuation model makes a *middle* removal cheap: deleting a row that only continues
    // a merge needs no markup change anywhere else — the region is simply one row shorter, because
    // nothing states its length as a number. Only removing the *anchor* row of a region that still
    // has rows below needs a rewrite: **promotion** — the cell directly below takes over the
    // anchor's whole content (`Cell::set_content`) and becomes `w:vMerge w:val="restart"` itself (or
    // loses `w:vMerge` entirely if that leaves a region of one). Each edit reads what it needs from
    // the pre-edit table into owned values first, then mutates — indices shift as rows/cells move,
    // so decisions are made before anything is touched, exactly as `mjx-dml`'s own `Table` does.
    // -------------------------------------------------------------------------------------------

    /// Inserts a new row so it becomes the table's `at`-th row (0-based); `at == row_count` appends.
    /// The new row holds one fresh cell per column, each built by `make_cell` — except where a
    /// vertical merge the new row falls **strictly inside** grows to include it: there, the new cell
    /// is born a continuation (`w:vMerge`, no `val` — the conventional "continue" spelling) with a
    /// matching `w:gridSpan`, so the region stays rectangular and the grid invariant holds.
    ///
    /// `at` must be `<= row_count` — the caller checks that.
    ///
    /// # Errors
    /// Only if a cell `make_cell` builds fails to parse, which a well-formed builder never does.
    pub fn insert_row(
        &mut self,
        interner: &mut Interner,
        at: usize,
        mut make_cell: impl FnMut(&mut Interner) -> RawElement,
    ) -> Result<(), FromXmlError> {
        let columns = self.column_count();
        let crossing = self.vertical_runs_crossing(interner, at);

        let mut cells = Vec::with_capacity(columns);
        let mut column = 0usize;
        let mut crossing = crossing.into_iter().peekable();
        while column < columns {
            if let Some(&(start, span)) = crossing.peek() {
                if start == column {
                    let mut cell = Cell::from_xml(&make_cell(interner), interner)?;
                    cell.set_column_span(interner, (span > 1).then_some(span));
                    cell.set_vertical_merge(interner, Some(MergedCellType::Continue));
                    cells.push(cell);
                    column += span;
                    crossing.next();
                    continue;
                }
            }
            cells.push(Cell::from_xml(&make_cell(interner), interner)?);
            column += 1;
        }

        let row = Row::new(interner, cells);
        self.insert_row_element(at, row);
        Ok(())
    }

    /// Removes the table's `at`-th row. A vertical merge the row lies **strictly inside** (a
    /// continuation, neither the anchor nor the last covered row) shrinks by one automatically —
    /// nothing else to rewrite. A vertical merge **anchored** in this row, with more rows below it,
    /// **promotes** the row directly below: it takes over the removed anchor's whole content
    /// (`Cell::set_content`) and its own `w:vMerge` becomes `restart` (or is removed, if that leaves
    /// a region of exactly one row).
    ///
    /// `at` must be `< row_count`, and the caller refuses removing the table's last row.
    pub fn remove_row(&mut self, interner: &mut Interner, at: usize) {
        let promotions = self.vertical_anchors_at_row(interner, at);
        for (start_column, remaining_run) in promotions {
            let Some(anchor_index) = self.resolve_physical_index(interner, at, start_column) else {
                continue;
            };
            let Some(anchor_content) = self
                .row(at)
                .and_then(|row| row.cell(anchor_index))
                .map(|cell| cell.content().to_vec())
            else {
                continue;
            };
            let Some(below_index) = self.resolve_physical_index(interner, at + 1, start_column)
            else {
                continue;
            };
            if let Some(cell) = self
                .row_mut(at + 1)
                .and_then(|row| row.cell_mut(below_index))
            {
                cell.set_content(anchor_content);
                let new_kind = (remaining_run > 1).then_some(MergedCellType::Restart);
                cell.set_vertical_merge(interner, new_kind);
            }
        }
        self.remove_row_element(at);
    }

    /// Inserts a new column so it becomes the table's `at`-th column (0-based); `at ==
    /// column_count` appends. The grid gains one `w:gridCol` and, in every row, a horizontal merge
    /// the new column falls **strictly inside** grows its `w:gridSpan` by one; everywhere else, every
    /// row gains one fresh cell (built by `make_cell`) at the right physical position, so the grid
    /// and every row's cells stay in step.
    ///
    /// `at` must be `<= column_count`.
    ///
    /// # Errors
    /// Only if a freshly built cell fails to parse.
    pub fn insert_column(
        &mut self,
        interner: &mut Interner,
        at: usize,
        mut make_cell: impl FnMut(&mut Interner) -> RawElement,
    ) -> Result<(), FromXmlError> {
        let rows = self.row_count();
        for row in 0..rows {
            // Does an existing cell's span strictly straddle the insertion line `at`
            // (`start < at < start + span`)? `resolve_cell(row, at - 1)` always returns a cell
            // whose own `start <= at - 1 < at`, so only the right-hand strict inequality needs
            // checking.
            let crossing = (at > 0)
                .then(|| self.resolve_cell(interner, row, at - 1))
                .flatten()
                .and_then(|(start, cell)| {
                    (at < start + cell.column_span(interner).max(1)).then_some(start)
                });
            if let Some(start) = crossing {
                if let Some(physical_index) = self.resolve_physical_index(interner, row, start) {
                    if let Some(cell) = self
                        .row_mut(row)
                        .and_then(|row| row.cell_mut(physical_index))
                    {
                        let span = cell.column_span(interner) + 1;
                        cell.set_column_span(interner, Some(span));
                    }
                }
                continue;
            }
            // Boundary case: insert a brand-new cell at the physical position column `at` starts.
            let physical_index = self.row(row).map_or(0, |row_ref| {
                let mut position = row_ref.cell_count();
                let mut column = 0usize;
                for (index, cell) in row_ref.cells().enumerate() {
                    if column >= at {
                        position = index;
                        break;
                    }
                    column += cell.column_span(interner).max(1);
                }
                position
            });
            let cell = Cell::from_xml(&make_cell(interner), interner)?;
            if let Some(row_ref) = self.row_mut(row) {
                row_ref.insert_cell_at(physical_index, cell);
            }
        }
        if let Some(grid) = self.grid_mut() {
            grid.insert_column_at(at, GridColumn::new(interner));
        }
        Ok(())
    }

    /// Removes the table's `at`-th column: the grid's `w:gridCol`, and in every row either shrinks
    /// the `w:gridSpan` of a cell the column lies **strictly inside** by one, or removes the one
    /// physical cell whose whole (unspanned) extent the column is.
    ///
    /// `at` must be `< column_count`, and the caller refuses removing the table's last column.
    pub fn remove_column(&mut self, interner: &mut Interner, at: usize) {
        let rows = self.row_count();
        for row in 0..rows {
            let Some((_, cell_ref)) = self.resolve_cell(interner, row, at) else {
                continue;
            };
            let span = cell_ref.column_span(interner).max(1);
            let Some(physical_index) = self.resolve_physical_index(interner, row, at) else {
                continue;
            };
            if span > 1 {
                if let Some(cell) = self
                    .row_mut(row)
                    .and_then(|row| row.cell_mut(physical_index))
                {
                    cell.set_column_span(interner, Some(span - 1));
                }
            } else if let Some(row_ref) = self.row_mut(row) {
                row_ref.remove_cell_at(physical_index);
            }
        }
        if let Some(grid) = self.grid_mut() {
            grid.remove_column_at(at);
        }
    }

    // --- Structural helpers -----------------------------------------------------------------------

    fn insert_row_element(&mut self, at: usize, row: Row) {
        let index = typed_insert_index(&self.content, at, |item| {
            matches!(item, TableContent::Row(_))
        });
        self.content.insert(index, TableContent::Row(row));
        self.empty = false;
    }

    fn remove_row_element(&mut self, at: usize) -> Option<Row> {
        let index = nth_typed_index(&self.content, at, |item| {
            matches!(item, TableContent::Row(_))
        })?;
        match self.content.remove(index) {
            TableContent::Row(row) => Some(row),
            other => {
                self.content.insert(index, other);
                None
            }
        }
    }

    /// Vertical-merge anchors, above `at`, whose region **strictly straddles** the line at row
    /// `at` (`anchor_row < at < anchor_row + run_length`) — `(start_column, column_span)`, one entry
    /// per distinct physical anchor cell (never once per grid column it spans).
    fn vertical_runs_crossing(&self, interner: &Interner, at: usize) -> Vec<(usize, usize)> {
        let mut found = Vec::new();
        for row in 0..at.min(self.row_count()) {
            let Some(row_ref) = self.row(row) else {
                continue;
            };
            let mut column = 0usize;
            for cell in row_ref.cells() {
                let span = cell.column_span(interner).max(1);
                if cell.is_vertical_merge_anchor(interner) {
                    let run = self.vertical_run_length(interner, row, column);
                    if row < at && at < row + run {
                        found.push((column, span));
                    }
                }
                column += span;
            }
        }
        found
    }

    /// Vertical-merge anchors sitting **in** row `at` whose region has more than one row —
    /// `(start_column, remaining_run)`, `remaining_run` already `run_length - 1` (the count after
    /// this row is removed).
    fn vertical_anchors_at_row(&self, interner: &Interner, at: usize) -> Vec<(usize, usize)> {
        let mut found = Vec::new();
        let Some(row_ref) = self.row(at) else {
            return found;
        };
        let mut column = 0usize;
        for cell in row_ref.cells() {
            let span = cell.column_span(interner).max(1);
            if cell.is_vertical_merge_anchor(interner) {
                let run = self.vertical_run_length(interner, at, column);
                if run > 1 {
                    found.push((column, run - 1));
                }
            }
            column += span;
        }
        found
    }
}

/// One way a table's `w:tblGrid`/rows disagree with each other or with themselves — never
/// constructed by a panic path, only by [`Table::grid_discrepancies`]' own read-only walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GridDiscrepancy {
    /// A row's cells, their `w:gridSpan`s summed, do not reach the grid's declared column count.
    RowWidthMismatch {
        /// The row's index.
        row: usize,
        /// The grid's own declared column count.
        declared_columns: usize,
        /// What the row's cells actually sum to.
        spanned_columns: usize,
    },
    /// A cell states `w:vMerge` as a continuation, but no reachable row above it anchors that
    /// region with `w:val="restart"` — the chain is broken (ECMA-376 Part 1 §17.4.84's own "closes"
    /// rule fired somewhere above it, or the table simply starts with one).
    OrphanedVerticalMerge {
        /// The continuation cell's row.
        row: usize,
        /// The grid column it starts at.
        column: usize,
    },
    /// A row holds no cells at all.
    EmptyRow {
        /// The row's index.
        row: usize,
    },
}
