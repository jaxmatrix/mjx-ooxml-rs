//! What a sheet records **about its own cells** without changing them: watched cells, the errors it
//! tells a consumer not to flag, and the smart tags a user attached.
//!
//! | Type | `sml.xsd` | Element | `CT_Worksheet` rank |
//! |---|---|---|---|
//! | [`CellWatches`] | 2947 | `x:cellWatches` | **26** |
//! | [`CellWatch`] | 2952 | `x:cellWatches/cellWatch` | |
//! | [`IgnoredErrors`] | 3154 | `x:ignoredErrors` | **27** |
//! | [`IgnoredError`] | 3160 | `x:ignoredErrors/ignoredError` | |
//! | [`SmartTags`] | 2479 | `x:smartTags` | **28** |
//! | [`CellSmartTags`] | 2485 | `x:smartTags/cellSmartTags` | |
//! | [`CellSmartTag`] | 2491 | `x:cellSmartTags/cellSmartTag` | |
//! | [`CellSmartTagProperty`] | 2500 | `x:cellSmartTag/cellSmartTagPr` | |
//!
//! Three unrelated features share this file because they share a shape and a rule, not because they
//! share a subject: each is a small `sml.xsd` cluster nothing else in Phase D claimed, each is one
//! of the three worksheet slots that stand together at ranks 26–28, and **each is a record a
//! consumer acts on, which this library reports and never acts on itself.** That last is the same
//! sentence [`crate::features`] writes for filters, sorts and validations, and it is worth writing
//! again because these three are even easier to mistake for instructions:
//!
//! * an [`IgnoredError`] does not mean the error is gone. It means Excel was told to stop drawing
//!   the green triangle over that range. **Nothing here evaluates a cell, so nothing here could know
//!   whether the error it names is even present** — reading one tells you what the file suppresses,
//!   not what it would have shown;
//! * a [`CellWatch`] is a cell somebody added to Excel's *Watch Window*. It changes no value and no
//!   format; it is a bookmark;
//! * a [`CellSmartTag`] names a recogniser by `@type`, an index into `xl/workbook.xml`'s
//!   `smartTagTypes` list — [`SmartTagTypes`](crate::SmartTagTypes), a **different** cluster that
//!   MJXOFF-100 (D06) modelled at the workbook tier. Nothing here resolves the index, and nothing
//!   here runs a recogniser.
//!
//! # Two `smartTag` vocabularies, and they are not the same one
//!
//! Worth stating because the names collide almost exactly. `xl/workbook.xml` carries
//! `CT_SmartTagPr`, `CT_SmartTagTypes` and `CT_SmartTagType` — *which* recognisers this workbook
//! knows and how they behave — and those are
//! [`SmartTagProperties`](crate::SmartTagProperties), [`SmartTagTypes`](crate::SmartTagTypes) and
//! [`SmartTagType`](crate::SmartTagType), built by MJXOFF-100. A **worksheet** carries
//! `CT_SmartTags` — *where on this sheet* a recogniser fired — and that is [`SmartTags`], here.
//! Neither is a view of the other and neither resolves into the other.

use mjx_ooxml_core::{
    Enumeration, Interner, Number, RawAttribute, RawElement, RawName, RawNode, Text, ToXml,
};
use mjx_ooxml_types::child_order::WORKSHEET_IGNORED_ERRORS;
use mjx_ooxml_types::support::OnOff;

use crate::address::{CellRangeList, CellReference};
use crate::leaf::attribute_bag;
use crate::worksheet::rebuild_element;

// -----------------------------------------------------------------------------------------------
// The leaves
// -----------------------------------------------------------------------------------------------

attribute_bag! {
    /// `x:cellWatch` (`CT_CellWatch`, `sml.xsd:2952`) — one cell in Excel's Watch Window.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_CellWatch`. Wire element: `cellWatch`.
    ///
    /// One attribute, `@r`, `use="required"`. It is an `ST_CellRef` — a **single cell**, not a range,
    /// which is what separates this from every other `@r`-like attribute in the neighbourhood — so it
    /// decodes to MJXOFF-93's [`CellReference`] rather than to a `CellRange`.
    #[xml(attribute(local = "r", codec = Enumeration<CellReference>, accessor = reference, required))]
    CellWatch, "cellWatch"
}

attribute_bag! {
    /// `x:ignoredError` (`CT_IgnoredError`, `sml.xsd:3160`) — one range, and the nine error kinds a
    /// consumer is told not to flag over it.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_IgnoredError`. Wire element: `ignoredError`.
    ///
    /// `@sqref` is the only `use="required"` attribute and is an `ST_Sqref` — MJXOFF-93's
    /// [`CellRangeList`], the same type `x:dataValidation` and `x:conditionalFormatting` carry, so a
    /// single element can suppress an error over several disjoint ranges at once.
    ///
    /// The nine flags are `xsd:boolean` with a schema default of `false`, and they are **not**
    /// mutually exclusive: one element may suppress several kinds over the same ranges, and one that
    /// sets none suppresses nothing at all while still being valid markup.
    ///
    /// **A flag says "do not draw the indicator", never "there is no error".** See this module's own
    /// documentation.
    #[xml(attribute(local = "sqref", codec = Enumeration<CellRangeList>, accessor = ranges, required))]
    #[xml(attribute(local = "evalError", codec = OnOff, accessor = ignores_evaluation_error, default = false))]
    #[xml(attribute(local = "twoDigitTextYear", codec = OnOff, accessor = ignores_two_digit_text_year, default = false))]
    #[xml(attribute(local = "numberStoredAsText", codec = OnOff, accessor = ignores_number_stored_as_text, default = false))]
    #[xml(attribute(local = "formula", codec = OnOff, accessor = ignores_inconsistent_formula, default = false))]
    #[xml(attribute(local = "formulaRange", codec = OnOff, accessor = ignores_formula_range, default = false))]
    #[xml(attribute(local = "unlockedFormula", codec = OnOff, accessor = ignores_unlocked_formula, default = false))]
    #[xml(attribute(local = "emptyCellReference", codec = OnOff, accessor = ignores_empty_cell_reference, default = false))]
    #[xml(attribute(local = "listDataValidation", codec = OnOff, accessor = ignores_list_data_validation, default = false))]
    #[xml(attribute(local = "calculatedColumn", codec = OnOff, accessor = ignores_calculated_column, default = false))]
    IgnoredError, "ignoredError"
}

attribute_bag! {
    /// `x:cellSmartTagPr` (`CT_CellSmartTagPr`, `sml.xsd:2500`) — **one** key-value pair a smart tag
    /// carries.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_CellSmartTagPr`. Wire element: `cellSmartTagPr`, declared
    /// `maxOccurs="unbounded"` — so each element is one property and the type is named in the
    /// singular, however plural the `Pr` suffix reads.
    ///
    /// Both attributes are `use="required"` and both are opaque `ST_Xstring`: their meaning belongs
    /// to whichever recogniser wrote them, and nothing here interprets either.
    #[xml(attribute(local = "key", codec = Text, accessor = key, required))]
    #[xml(attribute(local = "val", codec = Text, accessor = value, required))]
    CellSmartTagProperty, "cellSmartTagPr"
}

// -----------------------------------------------------------------------------------------------
// `x:cellWatches` — rank 26
// -----------------------------------------------------------------------------------------------

/// `x:cellWatches` (`CT_CellWatches`, `sml.xsd:2947`) — every watched cell, in document order.
///
/// **`ST_`/`CT_` symbol:** `CT_CellWatches`. Wire element: `cellWatches`, rank **26** of
/// `CT_Worksheet`.
///
/// The schema declares `cellWatch` `minOccurs="1"`, so a sheet that writes this element writes at
/// least one entry, and it declares **no `@count`** — there is no cache here to refresh.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
#[xml(namespace = SML)]
pub struct CellWatches {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "cellWatch", variant = Watch, ty = CellWatch))]
    content: Vec<CellWatchesContent>,
}

/// One child of [`CellWatches`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellWatchesContent {
    /// `x:cellWatch` — one watched cell.
    Watch(CellWatch),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl CellWatches {
    /// Builds an empty `x:cellWatches`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "cellWatches"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[CellWatchesContent] {
        &self.content
    }

    /// Every `x:cellWatch`, in document order.
    pub fn watches(&self) -> impl Iterator<Item = &CellWatch> + '_ {
        self.content.iter().filter_map(|item| match item {
            CellWatchesContent::Watch(watch) => Some(watch),
            CellWatchesContent::Raw(_) => None,
        })
    }

    /// How many cells the sheet watches.
    #[must_use]
    pub fn len(&self) -> usize {
        self.watches().count()
    }

    /// Whether the element lists no watched cell, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends a watched cell after the ones already present.
    ///
    /// **Nothing deduplicates.** `CT_CellWatches` declares no uniqueness rule and Excel writes what
    /// a user asked for; removing a repeat would be a correction nobody requested.
    pub fn push(&mut self, watch: CellWatch) {
        self.content.push(CellWatchesContent::Watch(watch));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                CellWatchesContent::Watch(watch) => RawNode::Element(watch.as_raw_element()),
                CellWatchesContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for CellWatches {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// `x:ignoredErrors` — rank 27, and the one type in this file with two child slots
// -----------------------------------------------------------------------------------------------

/// `x:ignoredErrors` (`CT_IgnoredErrors`, `sml.xsd:3154`) — every error-suppression record on the
/// sheet, in document order.
///
/// **`ST_`/`CT_` symbol:** `CT_IgnoredErrors`. Wire element: `ignoredErrors`, rank **27** of
/// `CT_Worksheet`.
///
/// The only type in this file whose sequence has **two** slots — `ignoredError` (rank 0,
/// `minOccurs="1"`, unbounded) then `extLst` (rank 1) — which is why [`push`](Self::push) places
/// through the generated [`WORKSHEET_IGNORED_ERRORS`] table instead of appending. A sheet that
/// carries an `extLst` here and gained a record appended at the end would be a sheet this library
/// wrote out of `xsd:sequence` order.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
#[xml(namespace = SML)]
pub struct IgnoredErrors {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "ignoredError", variant = Error, ty = IgnoredError))]
    content: Vec<IgnoredErrorsContent>,
}

/// One child of [`IgnoredErrors`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IgnoredErrorsContent {
    /// `x:ignoredError` (rank 0) — one suppression record.
    Error(IgnoredError),
    /// `x:extLst` (rank 1), and anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl IgnoredErrorsContent {
    /// This child's rank in `CT_IgnoredErrors`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        match self {
            Self::Error(_) => WORKSHEET_IGNORED_ERRORS.rank_of(None, "ignoredError"),
            Self::Raw(_) => None,
        }
    }
}

impl IgnoredErrors {
    /// Builds an empty `x:ignoredErrors`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "ignoredErrors"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, `extLst` included.
    #[must_use]
    pub fn content(&self) -> &[IgnoredErrorsContent] {
        &self.content
    }

    /// Every `x:ignoredError`, in document order.
    pub fn records(&self) -> impl Iterator<Item = &IgnoredError> + '_ {
        self.content.iter().filter_map(|item| match item {
            IgnoredErrorsContent::Error(record) => Some(record),
            IgnoredErrorsContent::Raw(_) => None,
        })
    }

    /// How many suppression records the sheet holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records().count()
    }

    /// Whether the element lists no record, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends a record after the ones already present, and **before** an `extLst` if there is one.
    pub fn push(&mut self, record: IgnoredError) {
        let at = WORKSHEET_IGNORED_ERRORS.insert_index_of_names(
            self.content.iter().map(IgnoredErrorsContent::rank),
            "ignoredError",
        );
        self.content.insert(at, IgnoredErrorsContent::Error(record));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                IgnoredErrorsContent::Error(record) => RawNode::Element(record.as_raw_element()),
                IgnoredErrorsContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for IgnoredErrors {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// `x:smartTags` — rank 28, three levels deep
// -----------------------------------------------------------------------------------------------

/// `x:cellSmartTag` (`CT_CellSmartTag`, `sml.xsd:2491`) — one recogniser's hit on one cell, and the
/// properties it recorded.
///
/// **`ST_`/`CT_` symbol:** `CT_CellSmartTag`. Wire element: `cellSmartTag`.
///
/// `@type` is `use="required"` and is an **index into `xl/workbook.xml`'s `smartTagTypes` list**, not
/// a name. This crate holds the number; resolving it is a caller's, and this crate never does it —
/// the same rule a `@dxfId` and a `@styleId` follow.
///
/// `@deleted` records that the user **removed the tag** while the record was kept in the file, and
/// `@xmlBased` that it came from an XML map rather than from a recogniser. Both default to `false`,
/// and neither is acted on.
///
/// The accessor's `_was_` shape copies [`ScenarioInputCells`](crate::ScenarioInputCells)' own
/// accessor for `CT_InputCells`' identically-named attribute, and for the same reason: this is a
/// **real removal by the user** reported as a fact about the file, not the chart family's *"draw
/// nothing here"*, which this workspace spells `suppressed`. Nothing is switched off — the record
/// stays in the file and is written out exactly as it was read.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "type", codec = Number<u32>, accessor = type_index, required))]
#[xml(attribute(local = "deleted", codec = OnOff, accessor = smart_tag_was_deleted, default = false))]
#[xml(attribute(local = "xmlBased", codec = OnOff, accessor = is_xml_based, default = false))]
pub struct CellSmartTag {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "cellSmartTagPr", variant = Property, ty = CellSmartTagProperty))]
    content: Vec<CellSmartTagContent>,
}

/// One child of [`CellSmartTag`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellSmartTagContent {
    /// `x:cellSmartTagPr` — one key-value pair.
    Property(CellSmartTagProperty),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl CellSmartTag {
    /// Builds an empty `x:cellSmartTag`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "cellSmartTag"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[CellSmartTagContent] {
        &self.content
    }

    /// Every `x:cellSmartTagPr`, in document order.
    pub fn properties(&self) -> impl Iterator<Item = &CellSmartTagProperty> + '_ {
        self.content.iter().filter_map(|item| match item {
            CellSmartTagContent::Property(property) => Some(property),
            CellSmartTagContent::Raw(_) => None,
        })
    }

    /// Appends a property after the ones already present.
    pub fn push(&mut self, property: CellSmartTagProperty) {
        self.content.push(CellSmartTagContent::Property(property));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                CellSmartTagContent::Property(property) => {
                    RawNode::Element(property.as_raw_element())
                }
                CellSmartTagContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for CellSmartTag {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:cellSmartTags` (`CT_CellSmartTags`, `sml.xsd:2485`) — every recogniser hit on **one** cell.
///
/// **`ST_`/`CT_` symbol:** `CT_CellSmartTags`. Wire element: `cellSmartTags`.
///
/// `@r` is `use="required"` and is an `ST_CellRef` — a single cell, as [`CellWatch`]'s is. This is
/// the level that carries the address; the tags inside it do not repeat it.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "r", codec = Enumeration<CellReference>, accessor = reference, required))]
pub struct CellSmartTags {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "cellSmartTag", variant = Tag, ty = CellSmartTag))]
    content: Vec<CellSmartTagsContent>,
}

/// One child of [`CellSmartTags`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellSmartTagsContent {
    /// `x:cellSmartTag` — one recogniser hit.
    Tag(CellSmartTag),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl CellSmartTags {
    /// Builds an empty `x:cellSmartTags`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "cellSmartTags"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[CellSmartTagsContent] {
        &self.content
    }

    /// Every `x:cellSmartTag`, in document order.
    pub fn tags(&self) -> impl Iterator<Item = &CellSmartTag> + '_ {
        self.content.iter().filter_map(|item| match item {
            CellSmartTagsContent::Tag(tag) => Some(tag),
            CellSmartTagsContent::Raw(_) => None,
        })
    }

    /// Appends a tag after the ones already present.
    pub fn push(&mut self, tag: CellSmartTag) {
        self.content.push(CellSmartTagsContent::Tag(tag));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                CellSmartTagsContent::Tag(tag) => RawNode::Element(tag.as_raw_element()),
                CellSmartTagsContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for CellSmartTags {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:smartTags` (`CT_SmartTags`, `sml.xsd:2479`) — every cell on the sheet a recogniser tagged.
///
/// **`ST_`/`CT_` symbol:** `CT_SmartTags`. Wire element: `smartTags`, rank **28** of `CT_Worksheet`.
///
/// **Not** `xl/workbook.xml`'s `smartTagTypes`, which is a different cluster with almost the same
/// name; see this module's own documentation.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
#[xml(namespace = SML)]
pub struct SmartTags {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "cellSmartTags", variant = Cell, ty = CellSmartTags))]
    content: Vec<SmartTagsContent>,
}

/// One child of [`SmartTags`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmartTagsContent {
    /// `x:cellSmartTags` — one cell's tags.
    Cell(CellSmartTags),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl SmartTags {
    /// Builds an empty `x:smartTags`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "smartTags"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[SmartTagsContent] {
        &self.content
    }

    /// Every `x:cellSmartTags`, in document order — one per tagged cell.
    pub fn cells(&self) -> impl Iterator<Item = &CellSmartTags> + '_ {
        self.content.iter().filter_map(|item| match item {
            SmartTagsContent::Cell(cell) => Some(cell),
            SmartTagsContent::Raw(_) => None,
        })
    }

    /// How many cells the sheet tags.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cells().count()
    }

    /// Whether the element lists no tagged cell, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends one cell's tags after the ones already present.
    pub fn push(&mut self, cell: CellSmartTags) {
        self.content.push(SmartTagsContent::Cell(cell));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                SmartTagsContent::Cell(cell) => RawNode::Element(cell.as_raw_element()),
                SmartTagsContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for SmartTags {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}
