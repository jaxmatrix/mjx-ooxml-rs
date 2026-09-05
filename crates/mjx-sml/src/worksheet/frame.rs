//! `xl/worksheets/sheetN.xml` — `CT_Worksheet`, the widest content model in the schema.
//!
//! # Thirty-nine slots, seven modelled, thirty-two held
//!
//! `CT_Worksheet` (`sml.xsd:2170`) is a **39-slot `xsd:sequence`** — ten times `CT_Slide`'s and
//! twice `CT_Workbook`'s. Thirty-two of those slots belong to later Phase D children, and this type
//! holds every one of them **in its schema position**, as the markup the file wrote. A worksheet
//! whose `pageSetup` survives a round-trip is proof the frame works, not proof `pageSetup` was
//! modelled.
//!
//! | rank | element | held as |
//! |---|---|---|
//! | 0 | `sheetPr` | [`SheetProperties`] |
//! | 1 | `dimension` | [`SheetDimension`] |
//! | 2 | `sheetViews` | [`SheetViews`] |
//! | 3 | `sheetFormatPr` | [`SheetFormatProperties`] |
//! | 4 | `cols` | [`ColumnBlock`] — **`maxOccurs="unbounded"`**, so a list |
//! | 5 | `sheetData` | [`SheetData`] — MJXOFF-95's packed store, not a subtree |
//! | 6 | `sheetCalcPr` | [`SheetCalculationProperties`] |
//! | 7–38 | `sheetProtection` … `extLst` | [`WorksheetContent::Raw`], verbatim and in position |
//!
//! The ranks are never written down. Every placement goes through
//! [`mjx_ooxml_types::child_order::WORKSHEET`], generated from `sml.xsd` by
//! `cargo run -p xtask -- codegen`; MJXOFF-89 (A7c) deleted fourteen hand-rolled ordering tables and
//! this file is not going to add a fifteenth.
//!
//! # Why this type owns a document rather than borrowing one
//!
//! Every other whole-part model in this workspace — [`WorkbookPart`](crate::WorkbookPart),
//! `mjx_docx`'s settings, `mjx_pptx`'s presentation — is a *view* over a
//! [`RawDocument`](mjx_ooxml_core::RawDocument) the package holds, and returns to it through
//! [`ToXml::write_back`](mjx_ooxml_core::ToXml::write_back). A worksheet cannot be, and the reason is
//! a measurement rather than a preference.
//!
//! `docs/BENCHMARKS.md` records a 300,000-cell worksheet costing **913 bytes of peak resident set
//! per cell** held as a `RawElement` tree. MJXOFF-95's store holds the same sheet in 36.8, and
//! `crates/mjx-sml/tests/cell_store_allocation.rs` bounds it at 48 with a counting global allocator.
//! A frame that borrowed a cached tree would keep that tree alive for as long as the workbook is
//! open, and the 25× would be given straight back. So this type **consumes** the document: it takes
//! the interner and the shared source buffer, models the seven slots it knows, keeps the other
//! thirty-two as moved [`RawNode`]s (a move, never a clone — `RawElement`'s `Clone` drops the
//! verbatim source range and a move does not), and lets the tree drop.
//!
//! Consuming the document is what makes [`write_into`](WorksheetPart::write_into) a **byte** writer
//! rather than a tree rebuild, which is in turn what lets the `sheetData` slot be a packed store.
//! `mjx_xml::fidelity::serialize_start_tag` exists for this.
//!
//! # Copy-on-write, restated at a fourth granularity
//!
//! `mjx-opc` gives copy-on-write per *part*; `RawElement` gives it per *subtree*;
//! [`SheetData`](crate::SheetData) restates it per *sheet, row and cell*. This type restates it once
//! more, per **slot**:
//!
//! * **The whole part.** Until anything is edited, [`write_into`](WorksheetPart::write_into) is one
//!   `extend_from_slice` of the buffer the part was parsed from. Prologue, root start tag, every
//!   slot, whitespace between them: all of it, unexamined.
//! * **One slot.** After an edit somewhere, every *other* slot still writes from its own bytes — an
//!   unmodelled child is a [`RawNode`] that kept its source range, and a modelled one keeps the
//!   [`RawElement`] it was read from beside the model.
//! * **Below `sheetData`.** The store's own three levels take over.
//!
//! The rule that makes the slot level sound is the one [`SheetData`] states for itself: **exactly
//! one door**. A modelled slot's verbatim element is dropped by the `_mut` accessor that hands out
//! `&mut`, and by the setter that replaces it — the two ways a caller can reach one — so a slot
//! whose bytes are still claimed is a slot nothing has been able to change.

use std::sync::Arc;

use mjx_ooxml_core::{
    FromXml, Interner, RawAttribute, RawDocument, RawElement, RawElementContent, RawName, RawNode,
};
use mjx_ooxml_types::child_order::WORKSHEET;
use mjx_ooxml_types::namespaces::SML;

use crate::address::{CellRange, CellReference};
use crate::cells::{Cell, CellValue, Row, SheetData};
use crate::error::SmlError;

use super::columns::{ColumnBlock, SheetFormatProperties};
use super::grid::{SheetCalculationProperties, SheetDimension};
use super::views::{SheetProperties, SheetViews};

/// One child of [`WorksheetPart`]: seven modelled slots, and everything else.
#[derive(Debug)]
pub enum WorksheetContent {
    /// `x:sheetPr` (rank 0).
    Properties(SheetProperties),
    /// `x:dimension` (rank 1) — a cached bounding box; see the [module documentation](crate::worksheet).
    Dimension(SheetDimension),
    /// `x:sheetViews` (rank 2).
    SheetViews(SheetViews),
    /// `x:sheetFormatPr` (rank 3).
    FormatProperties(SheetFormatProperties),
    /// `x:cols` (rank 4) — one block. The schema declares the slot `maxOccurs="unbounded"`, so
    /// several of these may stand in a row and merging them would change the file.
    Columns(ColumnBlock),
    /// `x:sheetData` (rank 5) — MJXOFF-95's packed cell store, not a subtree.
    SheetData(SheetData),
    /// `x:sheetCalcPr` (rank 6).
    CalculationProperties(SheetCalculationProperties),
    /// Everything this type does not model: the thirty-two later slots, any foreign element, any
    /// `mc:AlternateContent`, and the text, comments and processing instructions between siblings.
    ///
    /// Preserved verbatim and in position: placement skips a node it cannot rank, so an unmodelled
    /// child never moves and never moves anything else.
    Raw(RawNode),
}

impl WorksheetContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    #[must_use]
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::Properties(_) => "sheetPr",
            Self::Dimension(_) => "dimension",
            Self::SheetViews(_) => "sheetViews",
            Self::FormatProperties(_) => "sheetFormatPr",
            Self::Columns(_) => "cols",
            Self::SheetData(_) => "sheetData",
            Self::CalculationProperties(_) => "sheetCalcPr",
            Self::Raw(_) => return None,
        })
    }

    /// This child rebuilt as an element, for a slot whose verbatim bytes are no longer claimed.
    ///
    /// `None` for the two variants that are not elements in this model's terms:
    /// [`Raw`](Self::Raw), which *is* a node already, and [`SheetData`](Self::SheetData), which
    /// writes its own bytes.
    #[must_use]
    fn as_raw_element(&self) -> Option<RawElement> {
        Some(match self {
            Self::Properties(value) => value.as_raw_element(),
            Self::Dimension(value) => value.as_raw_element(),
            Self::SheetViews(value) => value.as_raw_element(),
            Self::FormatProperties(value) => value.as_raw_element(),
            Self::Columns(value) => value.as_raw_element(),
            Self::CalculationProperties(value) => value.as_raw_element(),
            Self::SheetData(_) | Self::Raw(_) => return None,
        })
    }
}

/// One child of the worksheet, and the bytes it may still be written from.
#[derive(Debug)]
struct Slot {
    /// The element as it was **moved** out of the parsed tree, with its verbatim source range
    /// intact — or `None` once the slot has been authored, replaced, or reached mutably.
    ///
    /// Never set for [`WorksheetContent::Raw`] (the node carries its own range) or for
    /// [`WorksheetContent::SheetData`] (the store carries three of its own).
    verbatim: Option<RawElement>,
    value: WorksheetContent,
}

impl Slot {
    /// A slot whose value must be written from the model.
    fn authored(value: WorksheetContent) -> Self {
        Self {
            verbatim: None,
            value,
        }
    }

    /// This child's rank in `CT_Worksheet`'s `xsd:sequence`, from the generated table.
    ///
    /// `None` for a node the table does not name, which is exactly the set of nodes placement must
    /// step over rather than treat as a boundary.
    fn rank(&self) -> Option<u16> {
        WORKSHEET.rank_of(None, self.value.local()?)
    }

    /// Gives up the claim on this slot's original bytes, because it is about to be changed.
    fn dirty(&mut self) {
        self.verbatim = None;
    }

    fn write_into(&self, interner: &Interner, source: Option<&[u8]>, out: &mut Vec<u8>) {
        match (&self.verbatim, &self.value) {
            (_, WorksheetContent::SheetData(store)) => store.write_into(out),
            (_, WorksheetContent::Raw(node)) => {
                mjx_xml::fidelity::serialize_node(node, interner, source, out);
            }
            (Some(original), _) => {
                mjx_xml::fidelity::serialize_element(original, interner, source, out);
            }
            (None, value) => {
                if let Some(element) = value.as_raw_element() {
                    mjx_xml::fidelity::serialize_element(&element, interner, source, out);
                }
            }
        }
    }
}

/// `x:worksheet` (`CT_Worksheet`, `sml.xsd:2170`) — the whole worksheet part.
///
/// See the [module documentation](crate::worksheet) for the thirty-nine slots, for why this type owns its
/// document rather than borrowing one, and for the slot-level copy-on-write that makes holding
/// thirty-two unmodelled children cost nothing.
#[derive(Debug)]
pub struct WorksheetPart {
    /// The interner every [`RawName`] below was interned in — moved out of the document this part
    /// was parsed from, because that document is dropped and the names outlive it.
    interner: Interner,
    /// The part's own bytes, shared with whoever else holds them. `None` for an authored part.
    source: Option<Arc<[u8]>>,
    /// Whether the part began with a UTF-8 byte-order mark.
    bom: bool,
    /// Nodes before the root element: the XML declaration, and any comment or PI beside it.
    prologue: Vec<RawNode>,
    /// Nodes after the root element.
    epilogue: Vec<RawNode>,
    /// The root element's qualified name, as the file wrote it.
    name: RawName,
    /// The root element's attributes, in order — every `xmlns:` declaration among them.
    attributes: Vec<RawAttribute>,
    /// Whether the root was written `<worksheet/>`.
    empty: bool,
    /// Every child, in document order.
    content: Vec<Slot>,
    /// Whether anything at all has been changed since the part was read. While this is false and
    /// [`source`](Self::source) is present, the whole part is one `memcpy`.
    edited: bool,
}

/// Declares one singleton slot: a borrowing getter, a mutable getter that gives up the slot's
/// verbatim bytes, and a setter that replaces the existing child in place or inserts a new one at
/// its rank in `CT_Worksheet`'s sequence.
///
/// Six slots share these three bodies, and writing them out six times would be six chances to reach
/// for the wrong variant. The mutable getter is **the one door** the slot-level copy-on-write rests
/// on: it is the only way a caller can reach a modelled child mutably, and it is where the claim on
/// the original bytes is dropped.
macro_rules! singleton_slot {
    ($getter:ident, $getter_mut:ident, $setter:ident, $variant:ident, $ty:ty, $local:literal, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<&$ty> {
            self.content.iter().find_map(|slot| match &slot.value {
                WorksheetContent::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("`x:", $local, "`, mutably — `None` if the worksheet has none.\n\n\
            Reaching a slot through here gives up its verbatim bytes: it and the part are marked \
            edited, so the slot re-emits from the model and every *other* slot still re-emits from \
            the file. See the [module documentation](crate::worksheet).")]
        #[must_use]
        pub fn $getter_mut(&mut self) -> Option<&mut $ty> {
            let slot = self.content.iter_mut().find(|slot| {
                matches!(&slot.value, WorksheetContent::$variant(_))
            })?;
            slot.dirty();
            self.edited = true;
            match &mut slot.value {
                WorksheetContent::$variant(value) => Some(value),
                _ => unreachable!("the slot was just matched on this variant"),
            }
        }

        #[doc = concat!("Sets `x:", $local, "`: `None` removes it; `Some(value)` replaces the \
            existing element **where it is**, or inserts a new one at its rank in `CT_Worksheet`'s \
            `xsd:sequence`.")]
        pub fn $setter(&mut self, value: Option<$ty>) {
            let is_target =
                |slot: &Slot| matches!(&slot.value, WorksheetContent::$variant(_));
            self.replace_or_insert(
                $local,
                is_target,
                value.map(WorksheetContent::$variant),
            );
        }
    };
}

impl WorksheetPart {
    /// Reads a whole worksheet part out of the document it was parsed from, **consuming** it.
    ///
    /// `Ok(None)` when the document's root is not an `x:worksheet` — the caller handed over a
    /// chartsheet, a dialogsheet or some other part entirely, which is a question rather than an
    /// error, exactly as [`WorkbookPart::read_part`](crate::WorkbookPart::read_part) treats it. The
    /// document is dropped either way; a caller that needs it back should parse again.
    ///
    /// # Errors
    /// [`SmlError::Model`] if a modelled element does not match its complex type,
    /// [`SmlError::Address`] if a cell's `@r` is not a cell reference (the store is keyed on it), or
    /// [`SmlError::PackedStoreTooLarge`] for a part past the store's four-gigabyte byte space.
    /// Nothing else a well-formed file can *say* is refused.
    pub fn read_document(document: RawDocument) -> Result<Option<Self>, SmlError> {
        {
            let namespace = document
                .root
                .name
                .namespace
                .map(|symbol| document.interner.resolve(symbol));
            let in_spreadsheetml = namespace == Some(SML.transitional)
                || (namespace.is_some() && namespace == SML.strict);
            if !in_spreadsheetml
                || document.interner.resolve(document.root.name.local) != "worksheet"
            {
                return Ok(None);
            }
        }
        let source = document.shared_source().cloned();
        let RawDocument {
            interner,
            bom,
            prologue,
            root,
            epilogue,
            ..
        } = document;
        let name = root.name;
        let empty = root.empty;
        let RawElementContent {
            attributes,
            children,
        } = root.into_content();

        let mut content = Vec::with_capacity(children.len());
        for node in children {
            content.push(read_slot(node, &interner, source.as_ref())?);
        }
        Ok(Some(Self {
            interner,
            source,
            bom,
            prologue,
            epilogue,
            name,
            attributes,
            empty,
            content,
            edited: false,
        }))
    }

    /// Parses `bytes` and reads the worksheet part in them, sharing the buffer so that every slot
    /// nobody edits re-emits from it.
    ///
    /// # Errors
    /// [`SmlError::Xml`] if the part is not well-formed, otherwise as
    /// [`read_document`](Self::read_document).
    pub fn read_part(bytes: &[u8]) -> Result<Option<Self>, SmlError> {
        Self::read_shared(Arc::from(bytes))
    }

    /// [`read_part`](Self::read_part) for a caller that already holds the bytes in an [`Arc`] — a
    /// package, above all, which has them for its own part-level copy-on-write.
    ///
    /// # Errors
    /// As [`read_part`](Self::read_part).
    pub fn read_shared(source: Arc<[u8]>) -> Result<Option<Self>, SmlError> {
        Self::read_document(mjx_xml::fidelity::parse_shared(source)?)
    }

    /// An empty `x:worksheet`, authored rather than read, bound to `prefix` or to the default
    /// namespace.
    ///
    /// Declares no namespaces of its own: a part written from this has to bind at least the
    /// SpreadsheetML namespace. MJXOFF-112 (D10) is what writes whole parts from nothing; this
    /// exists so the model is constructible rather than only readable.
    #[must_use]
    pub fn authored(prefix: Option<&str>) -> Self {
        let mut interner = Interner::default();
        let name = crate::leaf::sml_name(&mut interner, prefix, "worksheet");
        Self {
            interner,
            source: None,
            bom: false,
            prologue: Vec::new(),
            epilogue: Vec::new(),
            name,
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
            edited: true,
        }
    }

    /// The interner every name in this part was interned in.
    ///
    /// Hand it to any accessor that takes one — the attribute getters on every type here do,
    /// because an attribute's value is bytes and its name is a symbol.
    #[must_use]
    pub fn interner(&self) -> &Interner {
        &self.interner
    }

    /// The interner, mutably — for a caller building a child to insert.
    ///
    /// Interning a string does not change the part, so this does **not** mark it edited; the setters
    /// that take the resulting value do.
    pub fn interner_mut(&mut self) -> &mut Interner {
        &mut self.interner
    }

    /// The prefix this part binds to the relationship-reference namespace, from its own root-element
    /// `xmlns:` declarations — `r` in every file this project has read, and the producer's choice
    /// rather than the schema's.
    ///
    /// `None` means the part binds the namespace nowhere, so no element in it can carry an `r:id` at
    /// all — which is a statement about the `drawing`, `legacyDrawing`, `hyperlinks`, `oleObjects`,
    /// `controls`, `picture` and `tableParts` slots, every one of which reaches another part through
    /// one.
    #[must_use]
    pub fn relationship_prefix(&self) -> Option<&str> {
        crate::leaf::namespace_prefix(
            &self.attributes,
            &self.interner,
            crate::leaf::RELATIONSHIP_REFERENCE,
        )
    }

    /// Whether the whole part can still be written straight out of the bytes it was read from.
    ///
    /// False for an authored part, for one read without a source buffer, and for one anything has
    /// been changed in — after which the *slots* still answer the same question one at a time.
    #[must_use]
    pub fn is_verbatim(&self) -> bool {
        !self.edited && self.source.is_some()
    }

    /// Every child, in document order, including the thirty-two slot kinds this type does not
    /// model.
    ///
    /// An iterator rather than a slice: each child is stored beside the claim on its original
    /// bytes, and that bookkeeping is this type's own business.
    pub fn children(&self) -> impl ExactSizeIterator<Item = &WorksheetContent> + '_ {
        self.content.iter().map(|slot| &slot.value)
    }

    /// The local name of every **element** child, in document order — the thirty-two unmodelled
    /// slots included.
    ///
    /// This is what an ordering assertion is written against: it says what the part *will emit*,
    /// which is the thing schema order is a property of. A modelled slot answers with the wire name
    /// its type is declared under; an unmodelled one answers with the name the file wrote, resolved
    /// through this part's own interner. Text, comments and processing instructions are not children
    /// the schema orders and are not listed.
    pub fn child_element_locals(&self) -> impl Iterator<Item = &str> + '_ {
        self.content.iter().filter_map(|slot| match &slot.value {
            WorksheetContent::Raw(RawNode::Element(element)) => {
                Some(self.interner.resolve(element.name.local))
            }
            WorksheetContent::Raw(_) => None,
            modelled => modelled.local(),
        })
    }

    singleton_slot!(
        properties,
        properties_mut,
        set_properties,
        Properties,
        SheetProperties,
        "sheetPr",
        "`x:sheetPr` — the sheet tab's colour, outline behaviour, page-setup flags and nine \
         attributes. `None` if the worksheet writes none."
    );
    singleton_slot!(
        dimension,
        dimension_mut,
        set_dimension,
        Dimension,
        SheetDimension,
        "dimension",
        "`x:dimension` — the **cached** bounding box of the populated cells, exactly as the file \
         wrote it. Nothing recomputes it on a read; see the [module documentation](crate::worksheet) \
         for the whole rule."
    );
    singleton_slot!(
        sheet_views,
        sheet_views_mut,
        set_sheet_views,
        SheetViews,
        SheetViews,
        "sheetViews",
        "`x:sheetViews` — one view per workbook window: gridline and heading visibility, zoom, the \
         frozen or split pane, and the selection in each pane."
    );
    singleton_slot!(
        format_properties,
        format_properties_mut,
        set_format_properties,
        FormatProperties,
        SheetFormatProperties,
        "sheetFormatPr",
        "`x:sheetFormatPr` — the sheet's default row height and column width, and the outline depth \
         it reaches."
    );
    singleton_slot!(
        calculation_properties,
        calculation_properties_mut,
        set_calculation_properties,
        CalculationProperties,
        SheetCalculationProperties,
        "sheetCalcPr",
        "`x:sheetCalcPr` — whether a consumer should recalculate this sheet on load. Reported, never \
         acted on."
    );

    /// Every `x:cols` block, in document order.
    ///
    /// A list, not an `Option`: `CT_Worksheet` declares this slot `maxOccurs="unbounded"`, so a
    /// worksheet may hold several blocks in a row and **merging them changes the file**. See
    /// [`ColumnBlock`]'s own documentation.
    pub fn column_blocks(&self) -> impl Iterator<Item = &ColumnBlock> + '_ {
        self.content.iter().filter_map(|slot| match &slot.value {
            WorksheetContent::Columns(block) => Some(block),
            _ => None,
        })
    }

    /// The `index`-th `x:cols` block, mutably.
    pub fn column_block_mut(&mut self, index: usize) -> Option<&mut ColumnBlock> {
        let slot = self
            .content
            .iter_mut()
            .filter(|slot| matches!(&slot.value, WorksheetContent::Columns(_)))
            .nth(index)?;
        slot.dirty();
        self.edited = true;
        match &mut slot.value {
            WorksheetContent::Columns(block) => Some(block),
            _ => unreachable!("the slot was just filtered on this variant"),
        }
    }

    /// Appends one `x:cols` block at its rank in the sequence, after any already present.
    pub fn push_column_block(&mut self, block: ColumnBlock) {
        let at = self.insert_index("cols");
        self.content
            .insert(at, Slot::authored(WorksheetContent::Columns(block)));
        self.empty = false;
        self.edited = true;
    }

    /// `x:sheetData` — the cell store. `None` for a worksheet that writes none, which the schema
    /// forbids (`minOccurs="1"`) and which is therefore a defect in the file rather than a shape
    /// this refuses to read.
    #[must_use]
    pub fn sheet_data(&self) -> Option<&SheetData> {
        self.content.iter().find_map(|slot| match &slot.value {
            WorksheetContent::SheetData(store) => Some(store),
            _ => None,
        })
    }

    /// `x:sheetData`, mutably — `None` if the worksheet has none.
    ///
    /// Marks the part edited, which is what stops the whole-part verbatim shortcut. The store's own
    /// three levels of copy-on-write then decide how much of `sheetData` is re-emitted from the
    /// file, which is nearly all of it for a single-cell edit.
    #[must_use]
    pub fn sheet_data_mut(&mut self) -> Option<&mut SheetData> {
        let slot = self
            .content
            .iter_mut()
            .find(|slot| matches!(&slot.value, WorksheetContent::SheetData(_)))?;
        self.edited = true;
        match &mut slot.value {
            WorksheetContent::SheetData(store) => Some(store),
            _ => unreachable!("the slot was just matched on this variant"),
        }
    }

    /// `x:sheetData`, mutably, creating an empty one at rank 5 if the worksheet has none.
    pub fn sheet_data_or_insert(&mut self) -> &mut SheetData {
        if self.sheet_data().is_none() {
            let prefix = self
                .name
                .prefix
                .map(|symbol| self.interner.resolve(symbol).to_owned());
            let store = SheetData::authored(prefix.as_deref());
            let at = self.insert_index("sheetData");
            self.content
                .insert(at, Slot::authored(WorksheetContent::SheetData(store)));
            self.empty = false;
        }
        self.edited = true;
        self.sheet_data_mut()
            .expect("the sheetData slot was just ensured")
    }

    // -------------------------------------------------------------------------------------------
    // The curated cell surface
    // -------------------------------------------------------------------------------------------

    /// The cell at `reference`, or `None` — for a worksheet with no `sheetData` too.
    #[must_use]
    pub fn cell(&self, reference: CellReference) -> Option<Cell<'_>> {
        self.sheet_data()?.cell(reference)
    }

    /// Every populated cell, row by row and in each row's own order.
    pub fn cells(&self) -> impl Iterator<Item = Cell<'_>> + '_ {
        self.sheet_data().into_iter().flat_map(SheetData::cells)
    }

    /// Every populated row, in the order the file wrote them.
    pub fn rows(&self) -> impl Iterator<Item = Row<'_>> + '_ {
        self.sheet_data().into_iter().flat_map(SheetData::rows)
    }

    /// How many rows the sheet holds — populated rows, not addressable ones.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.sheet_data().map_or(0, SheetData::row_count)
    }

    /// How many cells the sheet holds, across every row.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.sheet_data().map_or(0, SheetData::cell_count)
    }

    /// Sets the value of the cell at `reference`, creating the cell, its row and the `sheetData`
    /// element if any of them is missing.
    ///
    /// Every other row, every other cell and every other worksheet child is left byte-identical:
    /// the store rewrites the one row it touched and copies the rest, and the slots beside
    /// `sheetData` are never visited.
    ///
    /// **The one thing this does change beside the cell** is `x:dimension`, and only when the cell
    /// falls outside the box the file recorded. Preserving a producer's stale cache is fidelity;
    /// writing a cell outside a box *this library then leaves wrong* would be authoring one. A
    /// `dimension` whose `@ref` is a whole-column or whole-row form already spans the axis and is
    /// left alone. See [`SheetDimension`]'s own documentation.
    ///
    /// # Errors
    /// [`SmlError`] as [`SheetData::set_cell_value`].
    pub fn set_cell_value(
        &mut self,
        reference: CellReference,
        value: CellValue<'_>,
    ) -> Result<(), SmlError> {
        self.sheet_data_or_insert()
            .set_cell_value(reference, value)?;
        self.widen_dimension_for(reference);
        Ok(())
    }

    /// Replaces `x:dimension`'s cached box with the one the populated cells actually occupy.
    ///
    /// **The caller's ask, never implicit.** Returns the range written, or `None` when the sheet has
    /// no populated cells or no `dimension` element to write into.
    pub fn recompute_dimension(&mut self) -> Option<CellRange> {
        let range = range_between(self.populated_bounds()?)?;
        self.write_dimension(range).then_some(range)
    }

    // -------------------------------------------------------------------------------------------
    // Writing
    // -------------------------------------------------------------------------------------------

    /// Appends the whole part — declaration, root element and all — to `out`.
    ///
    /// A part nobody edited is one copy of its own buffer. A part with one edited cell copies every
    /// slot but `sheetData`, and inside `sheetData` every row but the one that changed.
    pub fn write_into(&self, out: &mut Vec<u8>) {
        if let (false, Some(source)) = (self.edited, self.source.as_deref()) {
            out.extend_from_slice(source);
            return;
        }
        let source = self.source.as_deref();
        if self.bom {
            out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
        }
        for node in &self.prologue {
            mjx_xml::fidelity::serialize_node(node, &self.interner, source, out);
        }
        let self_closing = self.empty && self.content.is_empty();
        mjx_xml::fidelity::serialize_start_tag(
            &self.name,
            &self.attributes,
            self_closing,
            &self.interner,
            out,
        );
        if !self_closing {
            for slot in &self.content {
                slot.write_into(&self.interner, source, out);
            }
            mjx_xml::fidelity::serialize_end_tag(&self.name, &self.interner, out);
        }
        for node in &self.epilogue {
            mjx_xml::fidelity::serialize_node(node, &self.interner, source, out);
        }
    }

    /// The whole part as bytes.
    #[must_use]
    pub fn to_markup(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.source.as_ref().map_or(1024, |bytes| bytes.len()));
        self.write_into(&mut out);
        out
    }

    // -------------------------------------------------------------------------------------------
    // Placement
    // -------------------------------------------------------------------------------------------

    /// Where a child named `local` belongs among the current children.
    ///
    /// One call into the generated table: unranked nodes are stepped over rather than treated as a
    /// boundary, so a comment or an `mc:AlternateContent` between two slots neither moves nor
    /// displaces what is inserted next to it.
    fn insert_index(&self, local: &str) -> usize {
        WORKSHEET.insert_index_of_names(self.content.iter().map(Slot::rank), local)
    }

    /// Replaces the first child `is_target` accepts, keeping its position; inserts at the schema
    /// rank when there is none; removes it when `value` is `None`.
    fn replace_or_insert(
        &mut self,
        local: &str,
        is_target: impl Fn(&Slot) -> bool,
        value: Option<WorksheetContent>,
    ) {
        let existing = self.content.iter().position(&is_target);
        match (existing, value) {
            (Some(at), Some(value)) => self.content[at] = Slot::authored(value),
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(value)) => {
                let at = self.insert_index(local);
                self.content.insert(at, Slot::authored(value));
                self.empty = false;
            }
            (None, None) => return,
        }
        self.edited = true;
    }

    /// The rectangle the populated cells occupy, zero-based and inclusive, or `None` for a sheet
    /// with none.
    fn populated_bounds(&self) -> Option<(u16, u32, u16, u32)> {
        let mut bounds: Option<(u16, u32, u16, u32)> = None;
        for cell in self.cells() {
            let reference = cell.reference();
            let (column, row) = (reference.column(), reference.row());
            bounds = Some(match bounds {
                None => (column, row, column, row),
                Some((first_column, first_row, last_column, last_row)) => (
                    first_column.min(column),
                    first_row.min(row),
                    last_column.max(column),
                    last_row.max(row),
                ),
            });
        }
        bounds
    }

    /// Widens `x:dimension` to contain `reference`, if the element is there and does not already.
    fn widen_dimension_for(&mut self, reference: CellReference) {
        let Some(dimension) = self.dimension() else {
            return;
        };
        let Ok(range) = dimension.range(&self.interner) else {
            // A `@ref` that is absent or unparseable is the file's defect, and inventing a box to
            // replace it would be repairing markup this library did not write.
            return;
        };
        if !matches!(range, CellRange::Cell(_) | CellRange::Cells { .. })
            || range.contains(reference)
        {
            return;
        }
        let bounds = range.normalized_bounds();
        let widened = (
            bounds.first_column().min(reference.column()),
            bounds.first_row().min(reference.row()),
            bounds.last_column().max(reference.column()),
            bounds.last_row().max(reference.row()),
        );
        let Some(widened) = range_between(widened) else {
            return;
        };
        self.write_dimension(widened);
    }

    /// Writes `range` into the `x:dimension` element, reporting whether there was one.
    ///
    /// The interner is swapped out and back because the generated setter wants it mutably at the
    /// same moment [`dimension_mut`](Self::dimension_mut) holds `self` mutably, and the two live in
    /// one struct. Swapping is a pointer move of an empty `Interner`, not a rebuild of this one.
    fn write_dimension(&mut self, range: CellRange) -> bool {
        let mut interner = Interner::default();
        core::mem::swap(&mut interner, &mut self.interner);
        let written = match self.dimension_mut() {
            Some(dimension) => {
                dimension.set_range(&mut interner, range);
                true
            }
            None => false,
        };
        core::mem::swap(&mut interner, &mut self.interner);
        written
    }
}

/// The range covering `(first_column, first_row, last_column, last_row)`, zero-based and inclusive.
fn range_between(bounds: (u16, u32, u16, u32)) -> Option<CellRange> {
    let start = CellReference::relative(bounds.0, bounds.1).ok()?;
    let end = CellReference::relative(bounds.2, bounds.3).ok()?;
    Some(if start == end {
        CellRange::Cell(start)
    } else {
        CellRange::Cells { start, end }
    })
}

/// Reads one child node of `x:worksheet` into a slot.
///
/// A node is modelled only when it is an element **in the SpreadsheetML namespace** with one of the
/// seven local names this frame knows. An element merely *named* `sheetData` in somebody else's
/// namespace is unmodelled markup, and goes into the bucket with its prefix intact.
fn read_slot(
    node: RawNode,
    interner: &Interner,
    source: Option<&Arc<[u8]>>,
) -> Result<Slot, SmlError> {
    let RawNode::Element(element) = node else {
        return Ok(Slot {
            verbatim: None,
            value: WorksheetContent::Raw(node),
        });
    };
    let namespace = element
        .name
        .namespace
        .map(|symbol| interner.resolve(symbol));
    let in_spreadsheetml =
        namespace == Some(SML.transitional) || (namespace.is_some() && namespace == SML.strict);
    if !in_spreadsheetml {
        return Ok(Slot {
            verbatim: None,
            value: WorksheetContent::Raw(RawNode::Element(element)),
        });
    }
    let value = match interner.resolve(element.name.local) {
        "sheetPr" => WorksheetContent::Properties(SheetProperties::from_xml(&element, interner)?),
        "dimension" => WorksheetContent::Dimension(SheetDimension::from_xml(&element, interner)?),
        "sheetViews" => WorksheetContent::SheetViews(SheetViews::from_xml(&element, interner)?),
        "sheetFormatPr" => {
            WorksheetContent::FormatProperties(SheetFormatProperties::from_xml(&element, interner)?)
        }
        "cols" => WorksheetContent::Columns(ColumnBlock::from_xml(&element, interner)?),
        "sheetData" => {
            // The store shares the part's buffer, so every untouched row re-emits from it. It keeps
            // its own byte ranges rather than the element, which is dropped here — that is the whole
            // point of the packed store.
            return Ok(Slot {
                verbatim: None,
                value: WorksheetContent::SheetData(SheetData::read(&element, interner, source)?),
            });
        }
        "sheetCalcPr" => WorksheetContent::CalculationProperties(
            SheetCalculationProperties::from_xml(&element, interner)?,
        ),
        _ => {
            return Ok(Slot {
                verbatim: None,
                value: WorksheetContent::Raw(RawNode::Element(element)),
            })
        }
    };
    Ok(Slot {
        // The element is **moved**, not cloned, so its verbatim source range survives — a cloned
        // `RawElement` drops it, which is why every other whole-part model in this workspace has to
        // write back through the tree it came from and this one does not.
        verbatim: Some(element),
        value,
    })
}
