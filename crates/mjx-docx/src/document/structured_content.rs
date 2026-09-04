//! Structured content: content controls (`w:sdt`), custom XML (`w:customXml`), smart tags
//! (`w:smartTag`), bidirectional content wrappers (`w:dir`/`w:bdo`) and external content
//! (`w:altChunk`) — the other half of `wml.xsd` MJXOFF-69–74 never allotted an owner to (MJXOFF-138).
//!
//! # `w:sdt` and `w:customXml` are both members of all four content groups
//!
//! `EG_ContentBlockContent`, `EG_ContentRunContent`, `EG_ContentRowContent` and
//! `EG_ContentCellContent` each carry both `customXml` and `sdt` as a peer of the group's own "real"
//! member (`p`/`tbl`, `r`, `tr`, `tc`) — a content control or a custom-XML wrapper can therefore
//! appear anywhere a paragraph, a run, a table row or a table cell can. This module gives each
//! placement its own wrapper type (never a bare `Sdt*`/`CustomXml*` — "structured document tag" is
//! spelled out in the docs, the wire token is named on every item), and every wrapper's own content
//! **reuses** the exact content enum its placement already has a container for:
//!
//! | Placement | Wrapper (`w:sdt`) | Wrapper (`w:customXml`) | Content reused |
//! |---|---|---|---|
//! | Block  | [`ContentControlBlock`] | [`CustomXmlBlock`] | [`super::body::BlockContent`] (`Body`'s own) |
//! | Run    | [`ContentControlRun`]   | [`CustomXmlRun`]   | [`super::body::ParagraphContent`] (`Paragraph`'s own) |
//! | Row    | [`ContentControlRow`]   | [`CustomXmlRow`]   | [`super::tables::TableContent`] (`Table`'s own) |
//! | Cell   | [`ContentControlCell`]  | [`CustomXmlCell`]  | [`super::tables::RowContent`] (`Row`'s own) |
//!
//! Because the wrapped content is the *same* `Vec<BlockContent>`/`Vec<ParagraphContent>`/
//! `Vec<TableContent>`/`Vec<RowContent>` every other container of that placement already holds,
//! MJXOFF-92's paragraph/run APIs and MJXOFF-116's row/cell addressing reach through a content
//! control exactly as they reach through a table cell or a header — no parallel API, no
//! placement-specific duplicate. A run-level control inside a paragraph inside a cell-level control
//! inside a table inside a block-level control is walked one wrapper's own `content()`/`content_mut()`
//! accessor at a time, each returning the same type [`super::body::Body`], `HdrFtr` and
//! `super::tables::Cell` already hand a caller.
//!
//! `w:sdt`'s content sits behind a real child element (`w:sdtContent`, `CT_SdtContent{Block,Run,Row,
//! Cell}`); `w:customXml`'s content is inline in the wrapper element itself (`CT_CustomXml{Block,Run,
//! Row,Cell}`'s own `xsd:sequence` is `customXmlPr?, <the reused group>*`) — the two shapes this
//! module's own four-times-two derive blocks below reflect exactly.
//!
//! # Row/cell addressing sees through a wrapper
//!
//! [`super::tables::Table::rows`]/[`row`](super::tables::Table::row)/[`cell`](super::tables::Table::cell)
//! recurse into a [`ContentControlRow`]/[`CustomXmlRow`]'s own content when a table's row is wrapped —
//! a repeating-section content control wrapping one or more `w:tr` does not break `(row, column)`
//! addressing. The same recursion covers [`Row::cells`](super::tables::Row::cells) for a
//! [`ContentControlCell`]/[`CustomXmlCell`] wrapping one `w:tc`. See `tables.rs`'s own doc comment for
//! the details and the documented boundary (structural row/column insert and remove still address the
//! table's own top-level content only — see [`super::tables::Table::insert_row`]'s own doc comment).
//!
//! # Data binding is a two-part reference
//!
//! [`DataBinding`] carries `xpath`/`storeItemID`/`prefixMappings` verbatim (never validated on
//! read — a malformed reference is still preserved). Resolving it to an actual custom XML part and
//! node is [`crate::Document::resolve_data_binding`], which:
//!
//! 1. Enumerates every Custom XML Data Storage part (`customXml/itemN.xml`, ECMA-376 Part 1 §15.2.4)
//!    related to the main document part by [`crate::constants::REL_CUSTOM_XML_DATA`], each further
//!    related by [`crate::constants::REL_CUSTOM_XML_PROPS`] to its own Custom XML Data Storage
//!    Properties part (`customXml/itemPropsN.xml`, §15.2.6) — parsed only far enough to read
//!    `ds:datastoreItem/@ds:itemID`.
//! 2. Matches `storeItemID` against that `itemID`. **No match is a typed error, never a panic** — a
//!    binding naming a part the package does not carry is exactly the untrusted-input case this
//!    workspace's fidelity rules require reporting rather than crashing on.
//! 3. Walks `xpath` as the absolute, `[`n`]`-indexed element path Word itself always emits (see
//!    [`resolve_xpath`]'s own doc comment for the documented subset — anything else resolves to
//!    `None` rather than being mis-parsed).
//!
//! # Custom XML is preserved, never validated
//!
//! A Custom XML Data Storage part's root is *any* XML a template author chose — arbitrary, by
//! definition (§15.2.4: "Root Namespace: any XML allowed"). `mjx-schema-gate` classifies it as
//! category 2 (preserved foreign markup, like VML/InkML/ActiveX), not category 1 or 3 — see
//! `mjx_schema_gate::categories`'s own module doc and the `PRESERVED_FOREIGN_MARKUP` entry this child
//! adds, pinned by `crates/mjx-docx/tests/schema_gate.rs`'s
//! `a_custom_xml_data_storage_part_is_classified_as_preserved_foreign_markup`.

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, Interner, RawAttribute, RawElement, RawName, RawNode,
    Text as TextCodec, ToXml,
};
use mjx_ooxml_types::namespaces::WML;
use mjx_ooxml_types::shared::{CalendarType, Guid, UnsignedDecimalNumber};
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::wordprocessingml::{
    BidirectionalDirection, DateStorageFormat, DocumentPartBehavior, DocumentPartGallery,
    DocumentPartType, LockingType,
};

use super::body::{wml_name, BlockContent, ParagraphContent, Run, Text};
use super::fields::UnsignedDecimalNumberValue;
use super::paragraph_properties::DecimalNumberValue;
use super::run_properties::{RunProperties, Toggle};
use super::table_properties::TableStringValue;
use super::tables::{RowContent, TableContent};

// =================================================================================================
// Small opaque leaves shared across this module — the same FromXml/ToXml pattern every other leaf
// type in this crate follows (`body.rs::Break`, `table_properties.rs::TableStringValue`, …): typed
// attributes via `mjx_derive::XmlAttributes`, any child content preserved verbatim in `extra`.
// =================================================================================================

/// `CT_Lock` (`w:lock`, "Lock", §17.5.2.19) — the content control's own edit/delete lock, or `None`
/// when the file states none (unlocked, per §17.5.2.19's own default reading of an absent `val`).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<LockingType>, accessor = kind))]
pub struct Lock {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Lock {
    /// Builds a fresh `w:lock` of `kind`.
    #[must_use]
    pub(crate) fn new(interner: &mut Interner, kind: LockingType) -> Self {
        let mut value = Self {
            name: wml_name(interner, "lock"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_kind(interner, Some(kind));
        value
    }
}

impl FromXml for Lock {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Lock {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_DataBinding` (`w:dataBinding`, "XML Data Binding", §17.5.2.6) — the content control's own
/// binding to a Custom XML Data Storage part: an `xpath` naming a node inside it, a `storeItemID`
/// naming the part, and an optional `prefixMappings` (Word's own `xmlns:` shorthand for the prefixes
/// `xpath` uses — space-separated `xmlns:prefix='uri'` pairs, preserved verbatim, never re-parsed
/// here; [`crate::Document::resolve_data_binding`] is what interprets `xpath`/`prefixMappings`
/// together).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "prefixMappings", prefix = "w", codec = TextCodec, accessor = prefix_mappings))]
#[xml(attribute(local = "xpath", prefix = "w", codec = TextCodec, accessor = xpath, required))]
#[xml(attribute(local = "storeItemID", prefix = "w", codec = TextCodec, accessor = store_item_id, required))]
pub struct DataBinding {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl DataBinding {
    /// Builds a fresh `w:dataBinding` naming `store_item_id`/`xpath`, with no `prefixMappings`.
    #[must_use]
    pub(crate) fn new(interner: &mut Interner, store_item_id: &str, xpath: &str) -> Self {
        let mut value = Self {
            name: wml_name(interner, "dataBinding"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_store_item_id(interner, store_item_id);
        value.set_xpath(interner, xpath);
        value
    }
}

impl FromXml for DataBinding {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for DataBinding {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_Placeholder` (`w:placeholder`, "Placeholder Text", §17.5.2.24) — the one required child,
/// `w:docPart` (`CT_String`), naming a building block by name (resolved against
/// [`crate::Document::glossary_document`]'s own `w:docPart`s — this type only carries the name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placeholder {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    doc_part: Option<TableStringValue>,
    /// Any other child a non-conformant file nests here — `CT_Placeholder`'s own sequence has only
    /// `docPart`, so this is always empty for a conformant file; preserved regardless.
    extra: Vec<RawNode>,
}

impl Placeholder {
    /// Builds a fresh `w:placeholder` naming the building block `doc_part_name`.
    #[must_use]
    pub(crate) fn new(interner: &mut Interner, doc_part_name: &str) -> Self {
        Self {
            name: wml_name(interner, "placeholder"),
            attributes: Vec::new(),
            empty: false,
            doc_part: Some(TableStringValue::new(interner, "docPart", doc_part_name)),
            extra: Vec::new(),
        }
    }

    /// The named building block's own name (`w:docPart/@w:val`), or `None` if this `w:placeholder`
    /// states none (illegal per the schema — `docPart` is required — but a malformed file is read,
    /// not panicked on).
    #[must_use]
    pub fn doc_part_name(&self, interner: &Interner) -> Option<String> {
        self.doc_part
            .as_ref()
            .and_then(|value| value.value(interner).ok())
            .map(std::borrow::Cow::into_owned)
    }
}

impl FromXml for Placeholder {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let mut doc_part = None;
        let mut extra = Vec::new();
        for child in &element.children {
            match child {
                RawNode::Element(child)
                    if interner.resolve(child.name.local) == "docPart" && doc_part.is_none() =>
                {
                    doc_part = Some(TableStringValue::from_xml(child, interner)?);
                }
                other => extra.push(other.clone()),
            }
        }
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            doc_part,
            extra,
        })
    }
}

impl ToXml for Placeholder {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let mut children = Vec::with_capacity(self.extra.len() + 1);
        if let Some(doc_part) = &self.doc_part {
            children.push(RawNode::Element(doc_part.to_xml(interner)));
        }
        children.extend(self.extra.iter().cloned());
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_SdtListItem` (`w:listItem`, "List Item", §17.5.2.20) — one entry of a combo box or drop-down
/// list, both attributes optional per the schema (a conformant file states both).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "displayText", prefix = "w", codec = TextCodec, accessor = display_text))]
#[xml(attribute(local = "value", prefix = "w", codec = TextCodec, accessor = value))]
pub struct ContentControlListItem {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl ContentControlListItem {
    /// Builds a fresh `w:listItem` with the given display text and stored value.
    #[must_use]
    pub fn new(interner: &mut Interner, display_text: &str, value: &str) -> Self {
        let mut item = Self {
            name: wml_name(interner, "listItem"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_display_text(interner, Some(display_text));
        item.set_value(interner, Some(value));
        item
    }
}

impl FromXml for ContentControlListItem {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ContentControlListItem {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// One ordered child of a [`ContentControlComboBox`]/[`ContentControlDropDownList`]: its own
/// `listItem*`, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentControlListContent {
    /// `w:listItem` (`CT_SdtListItem`).
    Item(ContentControlListItem),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_SdtComboBox` (`w:comboBox`, "Combo Box", §17.5.2.5) — a combo box's own list of choices, plus
/// the last value a user typed that did not match any of them (`lastValue`, defaulting to `""` when
/// absent per the schema).
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "lastValue", prefix = "w", codec = TextCodec, accessor = last_value))]
pub struct ContentControlComboBox {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "listItem", variant = Item, ty = ContentControlListItem))]
    content: Vec<ContentControlListContent>,
}

impl ContentControlComboBox {
    /// Every choice this combo box offers, in document order.
    pub fn items(&self) -> impl Iterator<Item = &ContentControlListItem> {
        self.content.iter().filter_map(|item| match item {
            ContentControlListContent::Item(item) => Some(item),
            ContentControlListContent::Raw(_) => None,
        })
    }
}

/// `CT_SdtDropDownList` (`w:dropDownList`, "Drop-Down List", §17.5.2.10) — same shape as
/// [`ContentControlComboBox`] (a drop-down forces the typed value to be one of its listed choices;
/// a combo box does not), so this type only differs from it in its own wire name.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "lastValue", prefix = "w", codec = TextCodec, accessor = last_value))]
pub struct ContentControlDropDownList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "listItem", variant = Item, ty = ContentControlListItem))]
    content: Vec<ContentControlListContent>,
}

impl ContentControlDropDownList {
    /// Every choice this drop-down list offers, in document order.
    pub fn items(&self) -> impl Iterator<Item = &ContentControlListItem> {
        self.content.iter().filter_map(|item| match item {
            ContentControlListContent::Item(item) => Some(item),
            ContentControlListContent::Raw(_) => None,
        })
    }
}

/// `CT_CalendarType` (`w:calendar`, "Calendar Type", §17.5.2.1) — the one `val` attribute, `s:
/// ST_CalendarType`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<CalendarType>, accessor = kind))]
pub struct ContentControlCalendarType {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FromXml for ContentControlCalendarType {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ContentControlCalendarType {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_SdtDateMappingType` (`w:storeMappedDataAs`, "Date Storage Format", §17.5.2.35) — the one `val`
/// attribute.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<DateStorageFormat>, accessor = format))]
pub struct ContentControlDateMappingType {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FromXml for ContentControlDateMappingType {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ContentControlDateMappingType {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// One ordered child of a [`ContentControlDate`]: `CT_SdtDate`'s own `dateFormat?, lid?,
/// storeMappedDataAs?, calendar?` sequence — `w:lid` (`CT_Lang`) stays [`super::body::Unmodeled`]-shaped
/// opaque (this child's own scope is the date-control cluster's date semantics, not the shared
/// language-tagging leaf every properties struct in this crate already treats as raw where nothing
/// names it), so it falls to [`ContentControlDateContent::Raw`] like every other unmatched child.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentControlDateContent {
    /// `w:dateFormat` (`CT_String`).
    DateFormat(TableStringValue),
    /// `w:storeMappedDataAs` (`CT_SdtDateMappingType`).
    StoreMappedDataAs(ContentControlDateMappingType),
    /// `w:calendar` (`CT_CalendarType`).
    Calendar(ContentControlCalendarType),
    /// `w:lid`, or any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_SdtDate` (`w:date`, "Date", §17.5.2.7) — a date control's own format, calendar and the
/// resolved full date it last stored (`fullDate`, `s:ST_DateTime` — read as the file's own wire
/// string, never parsed, exactly as every other `ST_DateTime` attribute in this crate).
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "fullDate", prefix = "w", codec = TextCodec, accessor = raw_full_date))]
pub struct ContentControlDate {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "dateFormat", variant = DateFormat, ty = TableStringValue),
        child(local = "storeMappedDataAs", variant = StoreMappedDataAs, ty = ContentControlDateMappingType),
        child(local = "calendar", variant = Calendar, ty = ContentControlCalendarType)
    )]
    content: Vec<ContentControlDateContent>,
}

impl ContentControlDate {
    /// This date control's own display format (`w:dateFormat/@w:val`), or `None` if it states none.
    #[must_use]
    pub fn date_format(&self, interner: &Interner) -> Option<String> {
        self.content
            .iter()
            .find_map(|item| match item {
                ContentControlDateContent::DateFormat(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.value(interner).ok())
            .map(std::borrow::Cow::into_owned)
    }

    /// How this date control's value is mapped into its data binding (`w:storeMappedDataAs/@w:val`),
    /// or `None` if it states none.
    #[must_use]
    pub fn store_mapped_data_as(&self, interner: &Interner) -> Option<DateStorageFormat> {
        self.content
            .iter()
            .find_map(|item| match item {
                ContentControlDateContent::StoreMappedDataAs(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.format(interner).ok().flatten())
    }
}

/// `CT_SdtText` (`w:text`, "Text", §17.5.2.40) — a plain-text control's own `multiLine` flag.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "multiLine", prefix = "w", codec = OnOff, accessor = multi_line))]
pub struct ContentControlText {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FromXml for ContentControlText {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ContentControlText {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// One ordered child of a [`BuildingBlockReference`]: `CT_SdtDocPart`'s own `docPartGallery?,
/// docPartCategory?, docPartUnique?` sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildingBlockReferenceContent {
    /// `w:docPartGallery` (`CT_String`) — which building-block gallery this control offers from.
    Gallery(TableStringValue),
    /// `w:docPartCategory` (`CT_String`) — which category within that gallery.
    Category(TableStringValue),
    /// `w:docPartUnique` (`CT_OnOff`).
    Unique(Toggle),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_SdtDocPart` (`w:docPartObj`/`w:docPartList`, "Building Block Gallery/List", §17.5.2.8 /
/// §17.5.2.9) — the same complex type serves both `EG_SdtControlKind` members (a building-block
/// gallery picker restricted to one building block, or one offering a whole list), which one this is
/// is `name`, not the Rust type, exactly as [`super::body::Text`] is reused across four
/// `EG_RunInnerContent` members.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct BuildingBlockReference {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "docPartGallery", variant = Gallery, ty = TableStringValue),
        child(local = "docPartCategory", variant = Category, ty = TableStringValue),
        child(local = "docPartUnique", variant = Unique, ty = Toggle)
    )]
    content: Vec<BuildingBlockReferenceContent>,
}

impl BuildingBlockReference {
    /// The gallery this control offers building blocks from (`w:docPartGallery/@w:val`), or `None`.
    #[must_use]
    pub fn gallery(&self, interner: &Interner) -> Option<String> {
        self.content
            .iter()
            .find_map(|item| match item {
                BuildingBlockReferenceContent::Gallery(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.value(interner).ok())
            .map(std::borrow::Cow::into_owned)
    }

    /// Whether inserting this control's building block must always create a new copy rather than
    /// reuse an existing instance (`w:docPartUnique`), if the file states either way.
    #[must_use]
    pub fn unique(&self, interner: &Interner) -> Option<bool> {
        self.content
            .iter()
            .find_map(|item| match item {
                BuildingBlockReferenceContent::Unique(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.value(interner).ok())
    }
}

// `CT_UnsignedDecimalNumber` (`w:tabIndex` inside `w:sdtPr`) reuses `fields.rs`'s own
// `UnsignedDecimalNumberValue` (imported above) — `fields.rs` (MJXOFF-121) already built exactly this
// leaf for its own `w:tabIndex` (`CT_FFTextInput`'s own form-field tab order) and doc-commented the
// same "Builds a new `local` element (`"tabIndex"`)" constructor this module needs; declaring a
// second copy here would be exactly the restatement "consume, do not re-create" forbids.

// =================================================================================================
// CT_SdtPr (w:sdtPr, "Structured Document Tag Properties") and CT_SdtEndPr (w:sdtEndPr)
// =================================================================================================

/// One ordered child of [`ContentControlProperties`]: `CT_SdtPr`'s own eleven leading sequence
/// members, then its trailing `xsd:choice` of twelve control-kind members (see
/// [`ContentControlProperties::kind`] for the friendlier, borrowed view over the six that carry a
/// payload). `w14:checkbox`/`w15:repeatingSection` — the two Microsoft extensions a checkbox or
/// repeating-section control needs (neither is in base `wml.xsd`: `CT_SdtPr`'s `xsd:choice` above is
/// exactly the twelve ECMA-376 names) — are not schema children this enum can name, so they fall to
/// [`ContentControlPropertyContent::Raw`] like any other unrecognized child, preserved byte-for-byte
/// in their exact position: dropping them silently would turn a working checkbox or repeating-section
/// control into inert text, which is exactly the failure mode this ticket's own constraints call out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentControlPropertyContent {
    /// `w:rPr` (`CT_RPr`) — the mark's own run properties (e.g. how a picture control's placeholder
    /// glyph renders).
    RunProperties(RunProperties),
    /// `w:alias` (`CT_String`) — a friendly name for the control, shown in the UI.
    Alias(TableStringValue),
    /// `w:tag` (`CT_String`) — a programmatic identifier a macro/automation script reads.
    Tag(TableStringValue),
    /// `w:id` (`CT_DecimalNumber`) — a document-unique numeric id Word assigns.
    Id(DecimalNumberValue),
    /// `w:lock` (`CT_Lock`).
    Lock(Lock),
    /// `w:placeholder` (`CT_Placeholder`) — names a building block whose content is the placeholder
    /// text shown before the control has been filled in.
    Placeholder(Placeholder),
    /// `w:temporary` (`CT_OnOff`) — the control is removed (its content promoted) the first time a
    /// user edits it.
    Temporary(Toggle),
    /// `w:showingPlcHdr` (`CT_OnOff`) — the control is currently showing its placeholder text rather
    /// than user content.
    ShowingPlaceholderText(Toggle),
    /// `w:dataBinding` (`CT_DataBinding`).
    DataBinding(DataBinding),
    /// `w:label` (`CT_DecimalNumber`) — names a building block's own `w:docPartPr/w:name` id this
    /// control groups with.
    Label(DecimalNumberValue),
    /// `w:tabIndex` (`CT_UnsignedDecimalNumber`).
    TabIndex(UnsignedDecimalNumberValue),
    /// `w:equation` (`CT_Empty`) — the control's content is a single inline equation.
    Equation(super::body::Unmodeled),
    /// `w:comboBox` (`CT_SdtComboBox`).
    ComboBox(ContentControlComboBox),
    /// `w:date` (`CT_SdtDate`).
    Date(ContentControlDate),
    /// `w:docPartObj` (`CT_SdtDocPart`) — a building-block-gallery picker restricted to inserting one
    /// specific building block.
    BuildingBlockGallery(BuildingBlockReference),
    /// `w:docPartList` (`CT_SdtDocPart`) — a building-block-gallery picker offering a whole gallery's
    /// list.
    BuildingBlockList(BuildingBlockReference),
    /// `w:dropDownList` (`CT_SdtDropDownList`).
    DropDownList(ContentControlDropDownList),
    /// `w:picture` (`CT_Empty`) — the control's content is a single picture.
    Picture(super::body::Unmodeled),
    /// `w:richText` (`CT_Empty`) — an unrestricted rich-text control (the schema's own default kind).
    RichText(super::body::Unmodeled),
    /// `w:text` (`CT_SdtText`).
    Text(ContentControlText),
    /// `w:citation` (`CT_Empty`) — a citation-source picker.
    Citation(super::body::Unmodeled),
    /// `w:group` (`CT_Empty`) — a group control: its content cannot be edited directly, only inserted
    /// into or deleted as a whole.
    Group(super::body::Unmodeled),
    /// `w:bibliography` (`CT_Empty`) — a bibliography-source picker.
    Bibliography(super::body::Unmodeled),
    /// `w14:checkbox`, `w15:repeatingSection`, or any other child — preserved verbatim.
    Raw(RawNode),
}

/// The content control's own kind (`CT_SdtPr`'s trailing `xsd:choice`), borrowed from whichever
/// [`ContentControlPropertyContent`] variant [`ContentControlProperties::kind`] found — the ergonomic
/// counterpart to that content vector for a caller who wants "what kind of control is this" without
/// matching on the wire enum directly. `None` from `kind()` means the file states none of the twelve
/// (schema-legal: the choice is `minOccurs="0"`; Word's own default reading is rich text).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentControlKind<'a> {
    /// `w:equation`.
    Equation,
    /// `w:comboBox`.
    ComboBox(&'a ContentControlComboBox),
    /// `w:date`.
    Date(&'a ContentControlDate),
    /// `w:docPartObj`.
    BuildingBlockGallery(&'a BuildingBlockReference),
    /// `w:docPartList`.
    BuildingBlockList(&'a BuildingBlockReference),
    /// `w:dropDownList`.
    DropDownList(&'a ContentControlDropDownList),
    /// `w:picture`.
    Picture,
    /// `w:richText`.
    RichText,
    /// `w:text`.
    Text(&'a ContentControlText),
    /// `w:citation`.
    Citation,
    /// `w:group`.
    Group,
    /// `w:bibliography`.
    Bibliography,
}

/// `CT_SdtPr` (`w:sdtPr`, "Structured Document Tag Properties", §17.5.2.32) — everything about a
/// content control except its content: its lock, placeholder, data binding and which of the twelve
/// control kinds it is.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct ContentControlProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "rPr", variant = RunProperties, ty = RunProperties),
        child(local = "alias", variant = Alias, ty = TableStringValue),
        child(local = "tag", variant = Tag, ty = TableStringValue),
        child(local = "id", variant = Id, ty = DecimalNumberValue),
        child(local = "lock", variant = Lock, ty = Lock),
        child(local = "placeholder", variant = Placeholder, ty = Placeholder),
        child(local = "temporary", variant = Temporary, ty = Toggle),
        child(local = "showingPlcHdr", variant = ShowingPlaceholderText, ty = Toggle),
        child(local = "dataBinding", variant = DataBinding, ty = DataBinding),
        child(local = "label", variant = Label, ty = DecimalNumberValue),
        child(local = "tabIndex", variant = TabIndex, ty = UnsignedDecimalNumberValue),
        child(local = "equation", variant = Equation, ty = super::body::Unmodeled),
        child(local = "comboBox", variant = ComboBox, ty = ContentControlComboBox),
        child(local = "date", variant = Date, ty = ContentControlDate),
        child(local = "docPartObj", variant = BuildingBlockGallery, ty = BuildingBlockReference),
        child(local = "docPartList", variant = BuildingBlockList, ty = BuildingBlockReference),
        child(local = "dropDownList", variant = DropDownList, ty = ContentControlDropDownList),
        child(local = "picture", variant = Picture, ty = super::body::Unmodeled),
        child(local = "richText", variant = RichText, ty = super::body::Unmodeled),
        child(local = "text", variant = Text, ty = ContentControlText),
        child(local = "citation", variant = Citation, ty = super::body::Unmodeled),
        child(local = "group", variant = Group, ty = super::body::Unmodeled),
        child(local = "bibliography", variant = Bibliography, ty = super::body::Unmodeled)
    )]
    content: Vec<ContentControlPropertyContent>,
}

/// `CT_SdtPr`'s own generated child-order table — looked up by symbol rather than through a curated
/// named constant (`mjx_ooxml_types::child_order::CELL_PROPERTIES`'s own shape): the generated
/// tables carry an entry for every complex type regardless of whether a curated alias exists, and
/// [`mjx_ooxml_types::child_order::find`] is the documented, `O(schemas)`, by-symbol lookup this
/// module's own doc comment names for exactly this case. Cheap enough to call from every setter
/// below rather than caching — the same "no hashing, no allocation, nothing built per call" cost the
/// tables' own module doc states for a lookup already holding its `&'static ChildOrder`.
fn content_control_properties_table() -> &'static mjx_ooxml_types::child_order::ChildOrder {
    mjx_ooxml_types::child_order::find(WML.transitional, "CT_SdtPr")
        .expect("CT_SdtPr is in the generated wml child-order table")
}

impl ContentControlProperties {
    /// The schema rank of an existing content item — every unrecognized child is unranked (`None`),
    /// so it never influences where a new typed member is placed, matching `CellProperties::rank`'s
    /// own reasoning (`tables.rs`).
    fn rank(item: &ContentControlPropertyContent) -> Option<u16> {
        let table = content_control_properties_table();
        let local = match item {
            ContentControlPropertyContent::RunProperties(_) => "rPr",
            ContentControlPropertyContent::Alias(_) => "alias",
            ContentControlPropertyContent::Tag(_) => "tag",
            ContentControlPropertyContent::Id(_) => "id",
            ContentControlPropertyContent::Lock(_) => "lock",
            ContentControlPropertyContent::Placeholder(_) => "placeholder",
            ContentControlPropertyContent::Temporary(_) => "temporary",
            ContentControlPropertyContent::ShowingPlaceholderText(_) => "showingPlcHdr",
            ContentControlPropertyContent::DataBinding(_) => "dataBinding",
            ContentControlPropertyContent::Label(_) => "label",
            ContentControlPropertyContent::TabIndex(_) => "tabIndex",
            ContentControlPropertyContent::Equation(_) => "equation",
            ContentControlPropertyContent::ComboBox(_) => "comboBox",
            ContentControlPropertyContent::Date(_) => "date",
            ContentControlPropertyContent::BuildingBlockGallery(_) => "docPartObj",
            ContentControlPropertyContent::BuildingBlockList(_) => "docPartList",
            ContentControlPropertyContent::DropDownList(_) => "dropDownList",
            ContentControlPropertyContent::Picture(_) => "picture",
            ContentControlPropertyContent::RichText(_) => "richText",
            ContentControlPropertyContent::Text(_) => "text",
            ContentControlPropertyContent::Citation(_) => "citation",
            ContentControlPropertyContent::Group(_) => "group",
            ContentControlPropertyContent::Bibliography(_) => "bibliography",
            ContentControlPropertyContent::Raw(_) => return None,
        };
        table.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&ContentControlPropertyContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: ContentControlPropertyContent) {
        let table = content_control_properties_table();
        let at = table.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&ContentControlPropertyContent) -> bool,
        value: Option<ContentControlPropertyContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    /// Sets (or, given `None`, removes) this control's own lock (`w:lock`).
    pub fn set_lock(&mut self, interner: &mut Interner, kind: Option<LockingType>) {
        let is_target = |item: &ContentControlPropertyContent| {
            matches!(item, ContentControlPropertyContent::Lock(_))
        };
        let value = kind.map(|kind| ContentControlPropertyContent::Lock(Lock::new(interner, kind)));
        self.set("lock", is_target, value);
    }

    /// Sets (or, given `None`, removes) this control's own placeholder (`w:placeholder`), naming the
    /// building block `doc_part_name`.
    pub fn set_placeholder(&mut self, interner: &mut Interner, doc_part_name: Option<&str>) {
        let is_target = |item: &ContentControlPropertyContent| {
            matches!(item, ContentControlPropertyContent::Placeholder(_))
        };
        let value = doc_part_name.map(|name| {
            ContentControlPropertyContent::Placeholder(Placeholder::new(interner, name))
        });
        self.set("placeholder", is_target, value);
    }

    /// Sets (or, given `None`, removes) this control's own XML data binding (`w:dataBinding`),
    /// naming the Custom XML Data Storage part (`store_item_id`) and node (`xpath`) it binds to —
    /// [`crate::Document::resolve_data_binding`] is what resolves the reference this writes.
    pub fn set_data_binding(&mut self, interner: &mut Interner, binding: Option<(&str, &str)>) {
        let is_target = |item: &ContentControlPropertyContent| {
            matches!(item, ContentControlPropertyContent::DataBinding(_))
        };
        let value = binding.map(|(store_item_id, xpath)| {
            ContentControlPropertyContent::DataBinding(DataBinding::new(
                interner,
                store_item_id,
                xpath,
            ))
        });
        self.set("dataBinding", is_target, value);
    }

    /// This control's own run properties (`w:rPr`), or `None`.
    #[must_use]
    pub fn run_properties(&self) -> Option<&RunProperties> {
        self.content.iter().find_map(|item| match item {
            ContentControlPropertyContent::RunProperties(value) => Some(value),
            _ => None,
        })
    }

    /// This control's friendly name (`w:alias/@w:val`), or `None`.
    #[must_use]
    pub fn alias(&self, interner: &Interner) -> Option<String> {
        self.content
            .iter()
            .find_map(|item| match item {
                ContentControlPropertyContent::Alias(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.value(interner).ok())
            .map(std::borrow::Cow::into_owned)
    }

    /// This control's programmatic tag (`w:tag/@w:val`), or `None`.
    #[must_use]
    pub fn tag(&self, interner: &Interner) -> Option<String> {
        self.content
            .iter()
            .find_map(|item| match item {
                ContentControlPropertyContent::Tag(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.value(interner).ok())
            .map(std::borrow::Cow::into_owned)
    }

    /// This control's own document-unique numeric id (`w:id/@w:val`), or `None`.
    #[must_use]
    pub fn id(&self, interner: &Interner) -> Option<i64> {
        self.content
            .iter()
            .find_map(|item| match item {
                ContentControlPropertyContent::Id(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.value(interner).ok())
    }

    /// This control's own lock (`w:lock`), or `None` if it states none.
    #[must_use]
    pub fn lock(&self) -> Option<&Lock> {
        self.content.iter().find_map(|item| match item {
            ContentControlPropertyContent::Lock(value) => Some(value),
            _ => None,
        })
    }

    /// This control's own placeholder (`w:placeholder`), or `None`.
    #[must_use]
    pub fn placeholder(&self) -> Option<&Placeholder> {
        self.content.iter().find_map(|item| match item {
            ContentControlPropertyContent::Placeholder(value) => Some(value),
            _ => None,
        })
    }

    /// Whether this control is temporary (`w:temporary`), if the file states either way.
    #[must_use]
    pub fn temporary(&self, interner: &Interner) -> Option<bool> {
        self.content
            .iter()
            .find_map(|item| match item {
                ContentControlPropertyContent::Temporary(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.value(interner).ok())
    }

    /// Whether this control is currently showing its placeholder text (`w:showingPlcHdr`), if the
    /// file states either way.
    #[must_use]
    pub fn showing_placeholder_text(&self, interner: &Interner) -> Option<bool> {
        self.content
            .iter()
            .find_map(|item| match item {
                ContentControlPropertyContent::ShowingPlaceholderText(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.value(interner).ok())
    }

    /// This control's own XML data binding (`w:dataBinding`), or `None` if it is not bound to a
    /// custom XML part at all. Resolving it to the part and node it names is
    /// [`crate::Document::resolve_data_binding`].
    #[must_use]
    pub fn data_binding(&self) -> Option<&DataBinding> {
        self.content.iter().find_map(|item| match item {
            ContentControlPropertyContent::DataBinding(value) => Some(value),
            _ => None,
        })
    }

    /// This control's own tab order index (`w:tabIndex/@w:val`), or `None`.
    #[must_use]
    pub fn tab_index(&self, interner: &Interner) -> Option<UnsignedDecimalNumber> {
        self.content
            .iter()
            .find_map(|item| match item {
                ContentControlPropertyContent::TabIndex(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.value(interner).ok())
    }

    /// This control's own kind (rich text, plain text, picture, combo box, drop-down, date,
    /// building-block gallery/list, group, citation, bibliography, or equation) — `None` if the file
    /// states none of the twelve `CT_SdtPr` choice members (Word's own default reading of that case is
    /// rich text, but this accessor reports exactly what the file states, not a defaulted guess).
    #[must_use]
    pub fn kind(&self) -> Option<ContentControlKind<'_>> {
        self.content.iter().find_map(|item| match item {
            ContentControlPropertyContent::Equation(_) => Some(ContentControlKind::Equation),
            ContentControlPropertyContent::ComboBox(value) => {
                Some(ContentControlKind::ComboBox(value))
            }
            ContentControlPropertyContent::Date(value) => Some(ContentControlKind::Date(value)),
            ContentControlPropertyContent::BuildingBlockGallery(value) => {
                Some(ContentControlKind::BuildingBlockGallery(value))
            }
            ContentControlPropertyContent::BuildingBlockList(value) => {
                Some(ContentControlKind::BuildingBlockList(value))
            }
            ContentControlPropertyContent::DropDownList(value) => {
                Some(ContentControlKind::DropDownList(value))
            }
            ContentControlPropertyContent::Picture(_) => Some(ContentControlKind::Picture),
            ContentControlPropertyContent::RichText(_) => Some(ContentControlKind::RichText),
            ContentControlPropertyContent::Text(value) => Some(ContentControlKind::Text(value)),
            ContentControlPropertyContent::Citation(_) => Some(ContentControlKind::Citation),
            ContentControlPropertyContent::Group(_) => Some(ContentControlKind::Group),
            ContentControlPropertyContent::Bibliography(_) => {
                Some(ContentControlKind::Bibliography)
            }
            _ => None,
        })
    }
}

/// One ordered child of [`ContentControlEndProperties`]: `CT_SdtEndPr`'s own `xsd:choice
/// maxOccurs="unbounded"` of `rPr?` — a repeatable choice of an optional element is unusual, but is
/// exactly what `wml.xsd` states; a conformant file carries at most one `w:rPr` in practice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentControlEndPropertyContent {
    /// `w:rPr` (`CT_RPr`) — the run properties applied to the mark inserted after the control's own
    /// end (its "end character properties").
    RunProperties(RunProperties),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_SdtEndPr` (`w:sdtEndPr`, "Structured Document Tag End Character Properties", §17.5.2.31).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct ContentControlEndProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "rPr", variant = RunProperties, ty = RunProperties))]
    content: Vec<ContentControlEndPropertyContent>,
}

impl ContentControlEndProperties {
    /// This mark's own end-character run properties (`w:rPr`), or `None`.
    #[must_use]
    pub fn run_properties(&self) -> Option<&RunProperties> {
        self.content.iter().find_map(|item| match item {
            ContentControlEndPropertyContent::RunProperties(value) => Some(value),
            _ => None,
        })
    }
}

// =================================================================================================
// Custom XML and smart tags: CT_Attr, CT_CustomXmlPr, CT_SmartTagPr
// =================================================================================================

/// `CT_Attr` (`w:attr`, "Custom XML Attribute", §17.5.1.1) — one attribute override a custom XML
/// wrapper or smart tag restates for validation: a namespace URI, a name, and a value.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "uri", prefix = "w", codec = TextCodec, accessor = uri))]
#[xml(attribute(local = "name", prefix = "w", codec = TextCodec, accessor = attribute_name, required))]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = value, required))]
pub struct Attr {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FromXml for Attr {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Attr {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// One ordered child of [`CustomXmlProperties`]: `CT_CustomXmlPr`'s own `placeholder?, attr*`
/// sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomXmlPropertyContent {
    /// `w:placeholder` (`CT_String`) — the placeholder text shown before this custom-XML region has
    /// been filled in, reusing the same `CT_String` shape [`Placeholder`]'s own `docPart` child does.
    Placeholder(TableStringValue),
    /// `w:attr` (`CT_Attr`).
    Attr(Attr),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_CustomXmlPr` (`w:customXmlPr`, "Custom XML Properties", §17.5.1.4) — a custom XML wrapper's
/// own optional placeholder and attribute overrides.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct CustomXmlProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "placeholder", variant = Placeholder, ty = TableStringValue),
        child(local = "attr", variant = Attr, ty = Attr)
    )]
    content: Vec<CustomXmlPropertyContent>,
}

impl CustomXmlProperties {
    /// This wrapper's own placeholder text (`w:placeholder/@w:val`), or `None`.
    #[must_use]
    pub fn placeholder(&self, interner: &Interner) -> Option<String> {
        self.content
            .iter()
            .find_map(|item| match item {
                CustomXmlPropertyContent::Placeholder(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.value(interner).ok())
            .map(std::borrow::Cow::into_owned)
    }

    /// Every attribute override this wrapper states, in document order.
    pub fn attrs(&self) -> impl Iterator<Item = &Attr> {
        self.content.iter().filter_map(|item| match item {
            CustomXmlPropertyContent::Attr(attr) => Some(attr),
            _ => None,
        })
    }
}

/// One ordered child of [`SmartTagProperties`]: `CT_SmartTagPr`'s own `attr*` sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmartTagPropertyContent {
    /// `w:attr` (`CT_Attr`).
    Attr(Attr),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_SmartTagPr` (`w:smartTagPr`, "Smart Tag Properties", §17.5.1.9) — a smart tag's own attribute
/// overrides.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct SmartTagProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "attr", variant = Attr, ty = Attr))]
    content: Vec<SmartTagPropertyContent>,
}

impl SmartTagProperties {
    /// Every attribute override this smart tag states, in document order.
    pub fn attrs(&self) -> impl Iterator<Item = &Attr> {
        self.content.iter().filter_map(|item| match item {
            SmartTagPropertyContent::Attr(attr) => Some(attr),
            _ => None,
        })
    }
}

// =================================================================================================
// w:dir / w:bdo (CT_DirContentRun / CT_BdoContentRun) — bidirectional content wrappers. Both are
// `EG_PContent*` with one optional `val`, so [`ParagraphContent`] serves their content directly, the
// same reuse [`ContentControlContentRun`] below gets for `w:sdtContent`.
// =================================================================================================

/// `CT_DirContentRun` (`w:dir`, "Bidirectional Embedding Level", §17.3.2.8) — a run-level
/// bidirectional-embedding override: `EG_PContent*` content, with an optional `val` naming the
/// embedding direction.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<BidirectionalDirection>, accessor = direction))]
pub struct DirContentRun {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pPr", variant = Properties, ty = super::paragraph_properties::ParagraphProperties),
        child(local = "customXml", variant = CustomXml, ty = CustomXmlRun),
        child(local = "smartTag", variant = SmartTag, ty = SmartTagRun),
        child(local = "sdt", variant = StructuredDocumentTag, ty = ContentControlRun),
        child(local = "dir", variant = BidirectionalEmbedding, ty = DirContentRun),
        child(local = "bdo", variant = BidirectionalOverride, ty = BdoContentRun),
        child(local = "r", variant = Run, ty = Run),
        child(local = "proofErr", variant = ProofingError, ty = super::body::ProofingError),
        child(local = "permStart", variant = PermissionRangeStart, ty = super::body::PermissionRangeStart),
        child(local = "permEnd", variant = PermissionRangeEnd, ty = super::body::PermissionRangeEnd),
        child(local = "fldSimple", variant = SimpleField, ty = super::fields::SimpleField),
        child(local = "hyperlink", variant = Hyperlink, ty = super::body::Hyperlink),
        child(local = "subDoc", variant = SubDocument, ty = super::body::RelationshipReference),
        child(local = "fldData", variant = FieldData, ty = Text),
        child(local = "bookmarkStart", variant = BookmarkStart, ty = super::ranges::Bookmark),
        child(local = "bookmarkEnd", variant = BookmarkEnd, ty = super::ranges::MarkupRange),
        child(local = "commentRangeStart", variant = CommentRangeStart, ty = super::ranges::MarkupRange),
        child(local = "commentRangeEnd", variant = CommentRangeEnd, ty = super::ranges::MarkupRange),
        child(local = "moveFromRangeStart", variant = MoveFromRangeStart, ty = super::revisions::MoveBookmark),
        child(local = "moveFromRangeEnd", variant = MoveFromRangeEnd, ty = super::ranges::MarkupRange),
        child(local = "moveToRangeStart", variant = MoveToRangeStart, ty = super::revisions::MoveBookmark),
        child(local = "moveToRangeEnd", variant = MoveToRangeEnd, ty = super::ranges::MarkupRange),
        child(local = "customXmlInsRangeStart", variant = CustomXmlInsRangeStart, ty = super::revisions::TrackChangeMarker),
        child(local = "customXmlInsRangeEnd", variant = CustomXmlInsRangeEnd, ty = super::ranges::Markup),
        child(local = "customXmlDelRangeStart", variant = CustomXmlDelRangeStart, ty = super::revisions::TrackChangeMarker),
        child(local = "customXmlDelRangeEnd", variant = CustomXmlDelRangeEnd, ty = super::ranges::Markup),
        child(local = "customXmlMoveFromRangeStart", variant = CustomXmlMoveFromRangeStart, ty = super::revisions::TrackChangeMarker),
        child(local = "customXmlMoveFromRangeEnd", variant = CustomXmlMoveFromRangeEnd, ty = super::ranges::Markup),
        child(local = "customXmlMoveToRangeStart", variant = CustomXmlMoveToRangeStart, ty = super::revisions::TrackChangeMarker),
        child(local = "customXmlMoveToRangeEnd", variant = CustomXmlMoveToRangeEnd, ty = super::ranges::Markup),
        child(local = "ins", variant = Ins, ty = super::revisions::RunTrackChange),
        child(local = "del", variant = Del, ty = super::revisions::RunTrackChange),
        child(local = "moveFrom", variant = MoveFrom, ty = super::revisions::RunTrackChange),
        child(local = "moveTo", variant = MoveTo, ty = super::revisions::RunTrackChange),
        child(local = "oMath", variant = Math, ty = mjx_omml::Math, ns = SHARED_MATH),
        child(local = "oMathPara", variant = MathParagraph, ty = mjx_omml::MathParagraph, ns = SHARED_MATH)
    )]
    content: Vec<ParagraphContent>,
}

impl DirContentRun {
    /// This wrapper's own content, in document order.
    #[must_use]
    pub fn content(&self) -> &[ParagraphContent] {
        &self.content
    }

    /// [`DirContentRun::content`], mutably.
    pub fn content_mut(&mut self) -> &mut Vec<ParagraphContent> {
        &mut self.content
    }
}

/// `CT_BdoContentRun` (`w:bdo`, "Bidirectional Override", §17.3.2.3) — the same shape as
/// [`DirContentRun`] (an embedding *level* versus a hard *override* of the bidirectional algorithm),
/// so only the wire name and this doc comment differ.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<BidirectionalDirection>, accessor = direction))]
pub struct BdoContentRun {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pPr", variant = Properties, ty = super::paragraph_properties::ParagraphProperties),
        child(local = "customXml", variant = CustomXml, ty = CustomXmlRun),
        child(local = "smartTag", variant = SmartTag, ty = SmartTagRun),
        child(local = "sdt", variant = StructuredDocumentTag, ty = ContentControlRun),
        child(local = "dir", variant = BidirectionalEmbedding, ty = DirContentRun),
        child(local = "bdo", variant = BidirectionalOverride, ty = BdoContentRun),
        child(local = "r", variant = Run, ty = Run),
        child(local = "proofErr", variant = ProofingError, ty = super::body::ProofingError),
        child(local = "permStart", variant = PermissionRangeStart, ty = super::body::PermissionRangeStart),
        child(local = "permEnd", variant = PermissionRangeEnd, ty = super::body::PermissionRangeEnd),
        child(local = "fldSimple", variant = SimpleField, ty = super::fields::SimpleField),
        child(local = "hyperlink", variant = Hyperlink, ty = super::body::Hyperlink),
        child(local = "subDoc", variant = SubDocument, ty = super::body::RelationshipReference),
        child(local = "fldData", variant = FieldData, ty = Text),
        child(local = "bookmarkStart", variant = BookmarkStart, ty = super::ranges::Bookmark),
        child(local = "bookmarkEnd", variant = BookmarkEnd, ty = super::ranges::MarkupRange),
        child(local = "commentRangeStart", variant = CommentRangeStart, ty = super::ranges::MarkupRange),
        child(local = "commentRangeEnd", variant = CommentRangeEnd, ty = super::ranges::MarkupRange),
        child(local = "moveFromRangeStart", variant = MoveFromRangeStart, ty = super::revisions::MoveBookmark),
        child(local = "moveFromRangeEnd", variant = MoveFromRangeEnd, ty = super::ranges::MarkupRange),
        child(local = "moveToRangeStart", variant = MoveToRangeStart, ty = super::revisions::MoveBookmark),
        child(local = "moveToRangeEnd", variant = MoveToRangeEnd, ty = super::ranges::MarkupRange),
        child(local = "customXmlInsRangeStart", variant = CustomXmlInsRangeStart, ty = super::revisions::TrackChangeMarker),
        child(local = "customXmlInsRangeEnd", variant = CustomXmlInsRangeEnd, ty = super::ranges::Markup),
        child(local = "customXmlDelRangeStart", variant = CustomXmlDelRangeStart, ty = super::revisions::TrackChangeMarker),
        child(local = "customXmlDelRangeEnd", variant = CustomXmlDelRangeEnd, ty = super::ranges::Markup),
        child(local = "customXmlMoveFromRangeStart", variant = CustomXmlMoveFromRangeStart, ty = super::revisions::TrackChangeMarker),
        child(local = "customXmlMoveFromRangeEnd", variant = CustomXmlMoveFromRangeEnd, ty = super::ranges::Markup),
        child(local = "customXmlMoveToRangeStart", variant = CustomXmlMoveToRangeStart, ty = super::revisions::TrackChangeMarker),
        child(local = "customXmlMoveToRangeEnd", variant = CustomXmlMoveToRangeEnd, ty = super::ranges::Markup),
        child(local = "ins", variant = Ins, ty = super::revisions::RunTrackChange),
        child(local = "del", variant = Del, ty = super::revisions::RunTrackChange),
        child(local = "moveFrom", variant = MoveFrom, ty = super::revisions::RunTrackChange),
        child(local = "moveTo", variant = MoveTo, ty = super::revisions::RunTrackChange),
        child(local = "oMath", variant = Math, ty = mjx_omml::Math, ns = SHARED_MATH),
        child(local = "oMathPara", variant = MathParagraph, ty = mjx_omml::MathParagraph, ns = SHARED_MATH)
    )]
    content: Vec<ParagraphContent>,
}

impl BdoContentRun {
    /// This wrapper's own content, in document order.
    #[must_use]
    pub fn content(&self) -> &[ParagraphContent] {
        &self.content
    }

    /// [`BdoContentRun::content`], mutably.
    pub fn content_mut(&mut self) -> &mut Vec<ParagraphContent> {
        &mut self.content
    }
}

// =================================================================================================
// w:sdtContent (CT_SdtContent{Block,Run,Row,Cell}) — a content control's own content element. Each
// wraps *exactly* the reused group with nothing else (`CT_SdtContentBlock` is
// `EG_ContentBlockContent*` and no more), so each struct below is a thin, derive-generated carrier
// around `Vec<BlockContent>`/`Vec<ParagraphContent>`/`Vec<TableContent>`/`Vec<RowContent>` — the same
// type [`super::body::Body`]/[`super::body::Paragraph`]/[`super::tables::Table`]/
// [`super::tables::Row`] already hold, which is what lets every existing paragraph/run/row/cell API
// reach through a content control unchanged.
//
// [`ContentControlContentBlock`]/[`ContentControlContentRow`]/[`ContentControlContentCell`] are also
// reused, by composition, as the group-parsing delegate for [`CustomXmlBlock`]/[`CustomXmlRow`]/
// [`CustomXmlCell`] below — those wrappers mix a `customXmlPr?` leading element with the same group,
// which the `mjx_derive` container derive cannot express directly (it manages exactly one
// `#[xml(children)]` field), so their own hand-written `FromXml`/`ToXml` builds a scratch
// [`RawElement`] holding only the group's own children and calls through to the matching
// `ContentControlContent*::from_xml`/`to_xml` here rather than re-declaring the dispatch table a
// third time. [`ContentControlContentRun`] serves the same second role for [`CustomXmlRun`] and
// [`SmartTagRun`], both of which mix `customXmlPr?`/`smartTagPr?` with `EG_PContent*`.
// =================================================================================================

/// `CT_SdtContentBlock` (`w:sdtContent`, block placement, §17.5.2.34) — `EG_ContentBlockContent*`,
/// nothing else.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct ContentControlContentBlock {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "customXml", variant = CustomXml, ty = CustomXmlBlock),
        child(local = "sdt", variant = StructuredDocumentTag, ty = ContentControlBlock),
        child(local = "p", variant = Paragraph, ty = super::body::Paragraph),
        child(local = "tbl", variant = Table, ty = super::tables::Table),
        child(local = "proofErr", variant = ProofingError, ty = super::body::ProofingError),
        child(local = "permStart", variant = PermissionRangeStart, ty = super::body::PermissionRangeStart),
        child(local = "permEnd", variant = PermissionRangeEnd, ty = super::body::PermissionRangeEnd),
        child(local = "sectPr", variant = SectionProperties, ty = super::sections::SectionProperties),
        child(local = "tcPr", variant = Properties, ty = super::tables::CellProperties),
        child(local = "altChunk", variant = AltChunk, ty = AltChunk)
    )]
    content: Vec<BlockContent>,
}

impl ContentControlContentBlock {
    /// This element's whole ordered content — the same shape
    /// [`super::body::Body::content`]/[`super::body::HdrFtr::content`]/[`super::tables::Cell::content`]
    /// already hand a caller, so every free function in `body.rs` that walks a `&[BlockContent]`
    /// (`block_paragraphs`, `block_tables`, …) works over this unchanged.
    #[must_use]
    pub fn content(&self) -> &[BlockContent] {
        &self.content
    }

    /// [`ContentControlContentBlock::content`], mutably.
    pub fn content_mut(&mut self) -> &mut Vec<BlockContent> {
        &mut self.content
    }
}

/// `CT_SdtContentRun` (`w:sdtContent`, run placement, §17.5.2.35) — `EG_PContent*`, nothing else.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct ContentControlContentRun {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pPr", variant = Properties, ty = super::paragraph_properties::ParagraphProperties),
        child(local = "customXml", variant = CustomXml, ty = CustomXmlRun),
        child(local = "smartTag", variant = SmartTag, ty = SmartTagRun),
        child(local = "sdt", variant = StructuredDocumentTag, ty = ContentControlRun),
        child(local = "dir", variant = BidirectionalEmbedding, ty = DirContentRun),
        child(local = "bdo", variant = BidirectionalOverride, ty = BdoContentRun),
        child(local = "r", variant = Run, ty = Run),
        child(local = "proofErr", variant = ProofingError, ty = super::body::ProofingError),
        child(local = "permStart", variant = PermissionRangeStart, ty = super::body::PermissionRangeStart),
        child(local = "permEnd", variant = PermissionRangeEnd, ty = super::body::PermissionRangeEnd),
        child(local = "fldSimple", variant = SimpleField, ty = super::fields::SimpleField),
        child(local = "hyperlink", variant = Hyperlink, ty = super::body::Hyperlink),
        child(local = "subDoc", variant = SubDocument, ty = super::body::RelationshipReference),
        child(local = "fldData", variant = FieldData, ty = Text),
        child(local = "bookmarkStart", variant = BookmarkStart, ty = super::ranges::Bookmark),
        child(local = "bookmarkEnd", variant = BookmarkEnd, ty = super::ranges::MarkupRange),
        child(local = "commentRangeStart", variant = CommentRangeStart, ty = super::ranges::MarkupRange),
        child(local = "commentRangeEnd", variant = CommentRangeEnd, ty = super::ranges::MarkupRange),
        child(local = "moveFromRangeStart", variant = MoveFromRangeStart, ty = super::revisions::MoveBookmark),
        child(local = "moveFromRangeEnd", variant = MoveFromRangeEnd, ty = super::ranges::MarkupRange),
        child(local = "moveToRangeStart", variant = MoveToRangeStart, ty = super::revisions::MoveBookmark),
        child(local = "moveToRangeEnd", variant = MoveToRangeEnd, ty = super::ranges::MarkupRange),
        child(local = "customXmlInsRangeStart", variant = CustomXmlInsRangeStart, ty = super::revisions::TrackChangeMarker),
        child(local = "customXmlInsRangeEnd", variant = CustomXmlInsRangeEnd, ty = super::ranges::Markup),
        child(local = "customXmlDelRangeStart", variant = CustomXmlDelRangeStart, ty = super::revisions::TrackChangeMarker),
        child(local = "customXmlDelRangeEnd", variant = CustomXmlDelRangeEnd, ty = super::ranges::Markup),
        child(local = "customXmlMoveFromRangeStart", variant = CustomXmlMoveFromRangeStart, ty = super::revisions::TrackChangeMarker),
        child(local = "customXmlMoveFromRangeEnd", variant = CustomXmlMoveFromRangeEnd, ty = super::ranges::Markup),
        child(local = "customXmlMoveToRangeStart", variant = CustomXmlMoveToRangeStart, ty = super::revisions::TrackChangeMarker),
        child(local = "customXmlMoveToRangeEnd", variant = CustomXmlMoveToRangeEnd, ty = super::ranges::Markup),
        child(local = "ins", variant = Ins, ty = super::revisions::RunTrackChange),
        child(local = "del", variant = Del, ty = super::revisions::RunTrackChange),
        child(local = "moveFrom", variant = MoveFrom, ty = super::revisions::RunTrackChange),
        child(local = "moveTo", variant = MoveTo, ty = super::revisions::RunTrackChange),
        child(local = "oMath", variant = Math, ty = mjx_omml::Math, ns = SHARED_MATH),
        child(local = "oMathPara", variant = MathParagraph, ty = mjx_omml::MathParagraph, ns = SHARED_MATH)
    )]
    content: Vec<ParagraphContent>,
}

impl ContentControlContentRun {
    /// This element's whole ordered content — the same shape
    /// [`super::body::Paragraph::content`]/[`super::body::Hyperlink::content`] already hand a caller.
    #[must_use]
    pub fn content(&self) -> &[ParagraphContent] {
        &self.content
    }

    /// [`ContentControlContentRun::content`], mutably.
    pub fn content_mut(&mut self) -> &mut Vec<ParagraphContent> {
        &mut self.content
    }
}

/// `CT_SdtContentRow` (`w:sdtContent`, row placement, §17.5.2.36) — `EG_ContentRowContent*`, nothing
/// else: a repeating-section content control's own rows.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct ContentControlContentRow {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "tr", variant = Row, ty = super::tables::Row),
        child(local = "customXml", variant = CustomXml, ty = CustomXmlRow),
        child(local = "sdt", variant = StructuredDocumentTag, ty = ContentControlRow),
        child(local = "tblPr", variant = Properties, ty = super::table_properties::TableProperties),
        child(local = "tblGrid", variant = Grid, ty = super::tables::Grid)
    )]
    content: Vec<TableContent>,
}

impl ContentControlContentRow {
    /// This element's whole ordered content — the same shape [`super::tables::Table::content`]
    /// already hands a caller, so [`super::tables::Table::rows`] reaches through it.
    #[must_use]
    pub fn content(&self) -> &[TableContent] {
        &self.content
    }

    /// [`ContentControlContentRow::content`], mutably.
    pub fn content_mut(&mut self) -> &mut Vec<TableContent> {
        &mut self.content
    }
}

/// `CT_SdtContentCell` (`w:sdtContent`, cell placement, §17.5.2.33) — `EG_ContentCellContent*`,
/// nothing else: a repeating-section-in-a-row content control's own cells.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct ContentControlContentCell {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "tc", variant = Cell, ty = super::tables::Cell),
        child(local = "customXml", variant = CustomXml, ty = CustomXmlCell),
        child(local = "sdt", variant = StructuredDocumentTag, ty = ContentControlCell),
        child(local = "tblPrEx", variant = Exception, ty = super::table_properties::TableExceptionProperties),
        child(local = "trPr", variant = Properties, ty = super::table_properties::RowProperties)
    )]
    content: Vec<RowContent>,
}

impl ContentControlContentCell {
    /// This element's whole ordered content — the same shape [`super::tables::Row::content`] already
    /// hands a caller, so [`super::tables::Row::cells`] reaches through it.
    #[must_use]
    pub fn content(&self) -> &[RowContent] {
        &self.content
    }

    /// [`ContentControlContentCell::content`], mutably.
    pub fn content_mut(&mut self) -> &mut Vec<RowContent> {
        &mut self.content
    }
}

// =================================================================================================
// w:sdt (CT_SdtBlock / CT_SdtRun / CT_SdtRow / CT_SdtCell) — the content control itself, one type
// per placement. Every one of the four shares the exact shape `sdtPr?, sdtEndPr?, sdtContent?`; only
// which `ContentControlContent*` the third child is differs, so the four blocks below are the same
// pattern repeated with that one type substituted.
// =================================================================================================

/// One ordered child of a [`ContentControlBlock`]: `CT_SdtBlock`'s `sdtPr?, sdtEndPr?, sdtContent?`
/// sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentControlBlockContent {
    /// `w:sdtPr` (`CT_SdtPr`).
    Properties(ContentControlProperties),
    /// `w:sdtEndPr` (`CT_SdtEndPr`).
    EndProperties(ContentControlEndProperties),
    /// `w:sdtContent`.
    Content(ContentControlContentBlock),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_SdtBlock` (`w:sdt`, block placement, §17.5.2.29) — a content control appearing anywhere a
/// paragraph or a table can (`EG_ContentBlockContent`): [`super::body::Body`]'s, `HdrFtr`'s and
/// [`super::tables::Cell`]'s own block content.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct ContentControlBlock {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "sdtPr", variant = Properties, ty = ContentControlProperties),
        child(local = "sdtEndPr", variant = EndProperties, ty = ContentControlEndProperties),
        child(local = "sdtContent", variant = Content, ty = ContentControlContentBlock)
    )]
    content: Vec<ContentControlBlockContent>,
}

impl ContentControlBlock {
    /// This control's own properties (`w:sdtPr`) — its lock, placeholder, data binding and kind — or
    /// `None` if it states none.
    #[must_use]
    pub fn properties(&self) -> Option<&ContentControlProperties> {
        self.content.iter().find_map(|item| match item {
            ContentControlBlockContent::Properties(value) => Some(value),
            _ => None,
        })
    }

    /// This control's own end-character properties (`w:sdtEndPr`), or `None`.
    #[must_use]
    pub fn end_properties(&self) -> Option<&ContentControlEndProperties> {
        self.content.iter().find_map(|item| match item {
            ContentControlBlockContent::EndProperties(value) => Some(value),
            _ => None,
        })
    }

    /// This control's own content (`w:sdtContent`) — its block-level paragraphs and tables — or
    /// `None` when the control carries no content at all (schema-legal; a freshly inserted control
    /// before it has been filled in, say).
    #[must_use]
    pub fn content_block(&self) -> Option<&ContentControlContentBlock> {
        self.content.iter().find_map(|item| match item {
            ContentControlBlockContent::Content(value) => Some(value),
            _ => None,
        })
    }

    /// [`ContentControlBlock::content_block`], mutably.
    pub fn content_block_mut(&mut self) -> Option<&mut ContentControlContentBlock> {
        self.content.iter_mut().find_map(|item| match item {
            ContentControlBlockContent::Content(value) => Some(value),
            _ => None,
        })
    }
}

/// One ordered child of a [`ContentControlRun`]: `CT_SdtRun`'s `sdtPr?, sdtEndPr?, sdtContent?`
/// sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentControlRunContent {
    /// `w:sdtPr` (`CT_SdtPr`).
    Properties(ContentControlProperties),
    /// `w:sdtEndPr` (`CT_SdtEndPr`).
    EndProperties(ContentControlEndProperties),
    /// `w:sdtContent`.
    Content(ContentControlContentRun),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_SdtRun` (`w:sdt`, run placement, §17.5.2.37) — a content control appearing anywhere a run
/// can (`EG_ContentRunContent`): [`super::body::Paragraph`]'s and `Hyperlink`'s own content.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct ContentControlRun {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "sdtPr", variant = Properties, ty = ContentControlProperties),
        child(local = "sdtEndPr", variant = EndProperties, ty = ContentControlEndProperties),
        child(local = "sdtContent", variant = Content, ty = ContentControlContentRun)
    )]
    content: Vec<ContentControlRunContent>,
}

impl ContentControlRun {
    /// This control's own properties (`w:sdtPr`), or `None`.
    #[must_use]
    pub fn properties(&self) -> Option<&ContentControlProperties> {
        self.content.iter().find_map(|item| match item {
            ContentControlRunContent::Properties(value) => Some(value),
            _ => None,
        })
    }

    /// This control's own end-character properties (`w:sdtEndPr`), or `None`.
    #[must_use]
    pub fn end_properties(&self) -> Option<&ContentControlEndProperties> {
        self.content.iter().find_map(|item| match item {
            ContentControlRunContent::EndProperties(value) => Some(value),
            _ => None,
        })
    }

    /// This control's own content (`w:sdtContent`) — its runs — or `None` when empty.
    #[must_use]
    pub fn content_run(&self) -> Option<&ContentControlContentRun> {
        self.content.iter().find_map(|item| match item {
            ContentControlRunContent::Content(value) => Some(value),
            _ => None,
        })
    }

    /// [`ContentControlRun::content_run`], mutably.
    pub fn content_run_mut(&mut self) -> Option<&mut ContentControlContentRun> {
        self.content.iter_mut().find_map(|item| match item {
            ContentControlRunContent::Content(value) => Some(value),
            _ => None,
        })
    }
}

/// One ordered child of a [`ContentControlRow`]: `CT_SdtRow`'s `sdtPr?, sdtEndPr?, sdtContent?`
/// sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentControlRowContent {
    /// `w:sdtPr` (`CT_SdtPr`).
    Properties(ContentControlProperties),
    /// `w:sdtEndPr` (`CT_SdtEndPr`).
    EndProperties(ContentControlEndProperties),
    /// `w:sdtContent`.
    Content(ContentControlContentRow),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_SdtRow` (`w:sdt`, row placement, §17.5.2.38) — a repeating-section content control wrapping
/// one or more `w:tr` (`EG_ContentRowContent`): [`super::tables::Table`]'s own content.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct ContentControlRow {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "sdtPr", variant = Properties, ty = ContentControlProperties),
        child(local = "sdtEndPr", variant = EndProperties, ty = ContentControlEndProperties),
        child(local = "sdtContent", variant = Content, ty = ContentControlContentRow)
    )]
    content: Vec<ContentControlRowContent>,
}

impl ContentControlRow {
    /// This control's own properties (`w:sdtPr`), or `None`.
    #[must_use]
    pub fn properties(&self) -> Option<&ContentControlProperties> {
        self.content.iter().find_map(|item| match item {
            ContentControlRowContent::Properties(value) => Some(value),
            _ => None,
        })
    }

    /// This control's own end-character properties (`w:sdtEndPr`), or `None`.
    #[must_use]
    pub fn end_properties(&self) -> Option<&ContentControlEndProperties> {
        self.content.iter().find_map(|item| match item {
            ContentControlRowContent::EndProperties(value) => Some(value),
            _ => None,
        })
    }

    /// This control's own content (`w:sdtContent`) — the rows it wraps — or `None` when empty.
    #[must_use]
    pub fn content_row(&self) -> Option<&ContentControlContentRow> {
        self.content.iter().find_map(|item| match item {
            ContentControlRowContent::Content(value) => Some(value),
            _ => None,
        })
    }

    /// [`ContentControlRow::content_row`], mutably.
    pub fn content_row_mut(&mut self) -> Option<&mut ContentControlContentRow> {
        self.content.iter_mut().find_map(|item| match item {
            ContentControlRowContent::Content(value) => Some(value),
            _ => None,
        })
    }
}

/// One ordered child of a [`ContentControlCell`]: `CT_SdtCell`'s `sdtPr?, sdtEndPr?, sdtContent?`
/// sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentControlCellContent {
    /// `w:sdtPr` (`CT_SdtPr`).
    Properties(ContentControlProperties),
    /// `w:sdtEndPr` (`CT_SdtEndPr`).
    EndProperties(ContentControlEndProperties),
    /// `w:sdtContent`.
    Content(ContentControlContentCell),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_SdtCell` (`w:sdt`, cell placement, §17.5.2.30) — a content control wrapping one or more
/// `w:tc` (`EG_ContentCellContent`): [`super::tables::Row`]'s own content.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct ContentControlCell {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "sdtPr", variant = Properties, ty = ContentControlProperties),
        child(local = "sdtEndPr", variant = EndProperties, ty = ContentControlEndProperties),
        child(local = "sdtContent", variant = Content, ty = ContentControlContentCell)
    )]
    content: Vec<ContentControlCellContent>,
}

impl ContentControlCell {
    /// This control's own properties (`w:sdtPr`), or `None`.
    #[must_use]
    pub fn properties(&self) -> Option<&ContentControlProperties> {
        self.content.iter().find_map(|item| match item {
            ContentControlCellContent::Properties(value) => Some(value),
            _ => None,
        })
    }

    /// This control's own end-character properties (`w:sdtEndPr`), or `None`.
    #[must_use]
    pub fn end_properties(&self) -> Option<&ContentControlEndProperties> {
        self.content.iter().find_map(|item| match item {
            ContentControlCellContent::EndProperties(value) => Some(value),
            _ => None,
        })
    }

    /// This control's own content (`w:sdtContent`) — the cells it wraps — or `None` when empty.
    #[must_use]
    pub fn content_cell(&self) -> Option<&ContentControlContentCell> {
        self.content.iter().find_map(|item| match item {
            ContentControlCellContent::Content(value) => Some(value),
            _ => None,
        })
    }

    /// [`ContentControlCell::content_cell`], mutably.
    pub fn content_cell_mut(&mut self) -> Option<&mut ContentControlContentCell> {
        self.content.iter_mut().find_map(|item| match item {
            ContentControlCellContent::Content(value) => Some(value),
            _ => None,
        })
    }
}

// =================================================================================================
// w:customXml (CT_CustomXmlBlock / CT_CustomXmlRun / CT_CustomXmlRow / CT_CustomXmlCell) — a custom
// XML wrapper, one type per placement. Unlike `w:sdt`, `CT_CustomXml*`'s own content model puts the
// reused group *directly* in the wrapper's own `xsd:sequence` (`customXmlPr?, <group>*`, no
// `sdtContent`-like intermediate element) — so `mjx_derive`'s container derive, which manages exactly
// one `#[xml(children)]` field, cannot express "one leading element of a different type, then a
// reused Vec" in a single declaration. Each type below is instead hand-written: it keeps
// `customXmlPr` as its own field and its own `content: Vec<…Content>` field (still the exact type
// [`super::body::Body`]/[`super::body::Paragraph`]/[`super::tables::Table`]/[`super::tables::Row`]
// already hold), and its `FromXml`/`ToXml` delegate the group's own parsing/serialization to the
// matching already-derived `ContentControlContent*` struct through a scratch [`RawElement`] holding
// only the group's own children — reusing that struct's generated dispatch table rather than
// restating it a third time (see this module's own top doc comment).
// =================================================================================================

/// The custom-XML wrapper's own two attributes, shared by all four placements: an optional `uri`
/// (the custom element's namespace, when it has one — many real custom-XML vocabularies do not) and
/// the required `element` (its local name).
macro_rules! custom_xml_attributes {
    ($ty:ty) => {
        impl $ty {
            /// This wrapper's own properties (`w:customXmlPr`) — its placeholder text and attribute
            /// overrides — or `None` if it carries none.
            #[must_use]
            pub fn properties(&self) -> Option<&CustomXmlProperties> {
                self.properties.as_ref()
            }
        }
    };
}

/// `CT_CustomXmlBlock` (`w:customXml`, block placement, §17.5.1.3) — a custom-XML wrapper appearing
/// anywhere a paragraph or a table can (`EG_ContentBlockContent`).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "uri", prefix = "w", codec = TextCodec, accessor = uri))]
#[xml(attribute(local = "element", prefix = "w", codec = TextCodec, accessor = element, required))]
pub struct CustomXmlBlock {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    properties: Option<CustomXmlProperties>,
    content: Vec<BlockContent>,
}

custom_xml_attributes!(CustomXmlBlock);

impl CustomXmlBlock {
    /// This wrapper's own block-level content — the same shape [`super::body::Body::content`]
    /// already hands a caller.
    #[must_use]
    pub fn content(&self) -> &[BlockContent] {
        &self.content
    }

    /// [`CustomXmlBlock::content`], mutably.
    pub fn content_mut(&mut self) -> &mut Vec<BlockContent> {
        &mut self.content
    }
}

impl FromXml for CustomXmlBlock {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let (properties, rest) = split_leading_properties(element, interner, "customXmlPr")?;
        let scratch = RawElement::new(element.name, Vec::new(), rest, element.empty);
        let group = ContentControlContentBlock::from_xml(&scratch, interner)?;
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            properties,
            content: group.content,
        })
    }
}

impl ToXml for CustomXmlBlock {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let group = ContentControlContentBlock {
            name: element_name_placeholder(interner),
            attributes: Vec::new(),
            empty: self.content.is_empty(),
            content: self.content.clone(),
        };
        let group_xml = group.to_xml(interner);
        let children = join_properties_and_group(interner, &self.properties, group_xml);
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// Splits `element`'s own children into a leading `local`-named element (parsed as `P`, if present as
/// the *first* child — `mjx_derive`'s own container convention never looks past the first match for a
/// `minOccurs="0" maxOccurs="1"` leading element either) and everything else, verbatim — the shared
/// worker every hand-written `CustomXml*`/`SmartTagRun` `FromXml` in this module uses.
fn split_leading_properties<P: FromXml>(
    element: &RawElement,
    interner: &Interner,
    local: &str,
) -> Result<(Option<P>, Vec<RawNode>), FromXmlError> {
    let mut properties = None;
    let mut rest = Vec::with_capacity(element.children.len());
    for child in &element.children {
        match child {
            RawNode::Element(candidate)
                if properties.is_none() && interner.resolve(candidate.name.local) == local =>
            {
                properties = Some(P::from_xml(candidate, interner)?);
            }
            other => rest.push(other.clone()),
        }
    }
    Ok((properties, rest))
}

/// A throwaway [`RawName`] for a scratch delegate element — its own name never reaches the output;
/// [`ToXml::to_xml`] for every `ContentControlContent*` in this module writes only its own
/// `children`, and the caller (`CustomXml*::to_xml`) rebuilds the real element under its own `name`
/// (interned once, at open time, so this never grows the string table).
fn element_name_placeholder(interner: &mut Interner) -> RawName {
    RawName {
        prefix: None,
        local: interner.intern("scratch"),
        namespace: None,
    }
}

/// Joins a serialized `properties` (if present) ahead of `group`'s own children — the shared worker
/// every hand-written `CustomXml*`/`SmartTagRun` `ToXml` in this module uses.
fn join_properties_and_group<P: ToXml>(
    interner: &mut Interner,
    properties: &Option<P>,
    mut group: RawElement,
) -> Vec<RawNode> {
    let mut children = Vec::with_capacity(group.children.len() + 1);
    if let Some(properties) = properties {
        children.push(RawNode::Element(properties.to_xml(interner)));
    }
    children.append(&mut group.children);
    children
}

/// `CT_CustomXmlRun` (`w:customXml`, run placement, §17.5.1.4 as restated for the run-level member)
/// — a custom-XML wrapper appearing anywhere a run can (`EG_ContentRunContent`).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "uri", prefix = "w", codec = TextCodec, accessor = uri))]
#[xml(attribute(local = "element", prefix = "w", codec = TextCodec, accessor = element, required))]
pub struct CustomXmlRun {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    properties: Option<CustomXmlProperties>,
    content: Vec<ParagraphContent>,
}

custom_xml_attributes!(CustomXmlRun);

impl CustomXmlRun {
    /// This wrapper's own run-level content — the same shape [`super::body::Paragraph::content`]
    /// already hands a caller.
    #[must_use]
    pub fn content(&self) -> &[ParagraphContent] {
        &self.content
    }

    /// [`CustomXmlRun::content`], mutably.
    pub fn content_mut(&mut self) -> &mut Vec<ParagraphContent> {
        &mut self.content
    }
}

impl FromXml for CustomXmlRun {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let (properties, rest) = split_leading_properties(element, interner, "customXmlPr")?;
        let scratch = RawElement::new(element.name, Vec::new(), rest, element.empty);
        let group = ContentControlContentRun::from_xml(&scratch, interner)?;
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            properties,
            content: group.content,
        })
    }
}

impl ToXml for CustomXmlRun {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let group = ContentControlContentRun {
            name: element_name_placeholder(interner),
            attributes: Vec::new(),
            empty: self.content.is_empty(),
            content: self.content.clone(),
        };
        let group_xml = group.to_xml(interner);
        let children = join_properties_and_group(interner, &self.properties, group_xml);
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_CustomXmlRow` (`w:customXml`, row placement) — a custom-XML wrapper around one or more
/// `w:tr` (`EG_ContentRowContent`).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "uri", prefix = "w", codec = TextCodec, accessor = uri))]
#[xml(attribute(local = "element", prefix = "w", codec = TextCodec, accessor = element, required))]
pub struct CustomXmlRow {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    properties: Option<CustomXmlProperties>,
    content: Vec<TableContent>,
}

custom_xml_attributes!(CustomXmlRow);

impl CustomXmlRow {
    /// This wrapper's own row-level content — the same shape [`super::tables::Table::content`]
    /// already hands a caller, so [`super::tables::Table::rows`] reaches through it.
    #[must_use]
    pub fn content(&self) -> &[TableContent] {
        &self.content
    }

    /// [`CustomXmlRow::content`], mutably.
    pub fn content_mut(&mut self) -> &mut Vec<TableContent> {
        &mut self.content
    }
}

impl FromXml for CustomXmlRow {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let (properties, rest) = split_leading_properties(element, interner, "customXmlPr")?;
        let scratch = RawElement::new(element.name, Vec::new(), rest, element.empty);
        let group = ContentControlContentRow::from_xml(&scratch, interner)?;
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            properties,
            content: group.content,
        })
    }
}

impl ToXml for CustomXmlRow {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let group = ContentControlContentRow {
            name: element_name_placeholder(interner),
            attributes: Vec::new(),
            empty: self.content.is_empty(),
            content: self.content.clone(),
        };
        let group_xml = group.to_xml(interner);
        let children = join_properties_and_group(interner, &self.properties, group_xml);
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_CustomXmlCell` (`w:customXml`, cell placement) — a custom-XML wrapper around one or more
/// `w:tc` (`EG_ContentCellContent`).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "uri", prefix = "w", codec = TextCodec, accessor = uri))]
#[xml(attribute(local = "element", prefix = "w", codec = TextCodec, accessor = element, required))]
pub struct CustomXmlCell {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    properties: Option<CustomXmlProperties>,
    content: Vec<RowContent>,
}

custom_xml_attributes!(CustomXmlCell);

impl CustomXmlCell {
    /// This wrapper's own cell-level content — the same shape [`super::tables::Row::content`]
    /// already hands a caller, so [`super::tables::Row::cells`] reaches through it.
    #[must_use]
    pub fn content(&self) -> &[RowContent] {
        &self.content
    }

    /// [`CustomXmlCell::content`], mutably.
    pub fn content_mut(&mut self) -> &mut Vec<RowContent> {
        &mut self.content
    }
}

impl FromXml for CustomXmlCell {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let (properties, rest) = split_leading_properties(element, interner, "customXmlPr")?;
        let scratch = RawElement::new(element.name, Vec::new(), rest, element.empty);
        let group = ContentControlContentCell::from_xml(&scratch, interner)?;
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            properties,
            content: group.content,
        })
    }
}

impl ToXml for CustomXmlCell {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let group = ContentControlContentCell {
            name: element_name_placeholder(interner),
            attributes: Vec::new(),
            empty: self.content.is_empty(),
            content: self.content.clone(),
        };
        let group_xml = group.to_xml(interner);
        let children = join_properties_and_group(interner, &self.properties, group_xml);
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// =================================================================================================
// w:smartTag (CT_SmartTagRun) — the same "leading properties element, then a reused group" shape as
// `CT_CustomXmlRun`, so it is built the same way.
// =================================================================================================

/// `CT_SmartTagRun` (`w:smartTag`, §17.5.1.10) — a run-level smart tag, appearing anywhere a run
/// can (`EG_ContentRunContent`).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "uri", prefix = "w", codec = TextCodec, accessor = uri))]
#[xml(attribute(local = "element", prefix = "w", codec = TextCodec, accessor = element, required))]
pub struct SmartTagRun {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    properties: Option<SmartTagProperties>,
    content: Vec<ParagraphContent>,
}

impl SmartTagRun {
    /// This smart tag's own run-level content — the same shape [`super::body::Paragraph::content`]
    /// already hands a caller.
    #[must_use]
    pub fn content(&self) -> &[ParagraphContent] {
        &self.content
    }

    /// [`SmartTagRun::content`], mutably.
    pub fn content_mut(&mut self) -> &mut Vec<ParagraphContent> {
        &mut self.content
    }

    /// This smart tag's own properties (`w:smartTagPr`) — its attribute overrides — or `None`.
    #[must_use]
    pub fn properties(&self) -> Option<&SmartTagProperties> {
        self.properties.as_ref()
    }
}

impl FromXml for SmartTagRun {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let (properties, rest) = split_leading_properties(element, interner, "smartTagPr")?;
        let scratch = RawElement::new(element.name, Vec::new(), rest, element.empty);
        let group = ContentControlContentRun::from_xml(&scratch, interner)?;
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            properties,
            content: group.content,
        })
    }
}

impl ToXml for SmartTagRun {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let group = ContentControlContentRun {
            name: element_name_placeholder(interner),
            attributes: Vec::new(),
            empty: self.content.is_empty(),
            content: self.content.clone(),
        };
        let group_xml = group.to_xml(interner);
        let children = join_properties_and_group(interner, &self.properties, group_xml);
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// =================================================================================================
// Data-binding xpath resolution — `crate::Document::resolve_data_binding`'s own worker, kept here
// (rather than in `document/mod.rs`) because it is purely a function of a [`DataBinding`]'s own two
// wire strings and a custom XML part's own tree, with no `Package`/part-graph concern of its own.
// =================================================================================================

/// One `xmlns:prefix='uri'` pair parsed out of `w:dataBinding/@prefixMappings` — Word's own
/// concatenated-namespace-declaration shorthand (`xmlns:ns0='http://…' xmlns:ns1='http://…' …`, one
/// token per prefix `xpath` uses, space-separated, single-quoted).
fn parse_prefix_mappings(mappings: &str) -> Vec<(&str, &str)> {
    let mut found = Vec::new();
    let mut rest = mappings;
    while let Some(at) = rest.find("xmlns:") {
        let after_marker = &rest[at + "xmlns:".len()..];
        let Some(equals) = after_marker.find('=') else {
            break;
        };
        let prefix = after_marker[..equals].trim();
        let after_equals = after_marker[equals + 1..].trim_start();
        let Some(quote) = after_equals
            .chars()
            .next()
            .filter(|c| *c == '\'' || *c == '"')
        else {
            break;
        };
        let Some(close) = after_equals[1..].find(quote) else {
            break;
        };
        let uri = &after_equals[1..1 + close];
        if !prefix.is_empty() {
            found.push((prefix, uri));
        }
        rest = &after_equals[1 + close + 1..];
    }
    found
}

/// One step of the absolute, `[`n`]`-indexed element path Word itself always emits for
/// `w:dataBinding/@xpath` (e.g. `/ns0:root[1]/ns0:field[1]`): a namespace prefix, a local name, and a
/// 1-based sibling index among same-named elements.
struct XPathStep<'a> {
    prefix: &'a str,
    local: &'a str,
    index: usize,
}

/// Parses `xpath` into [`XPathStep`]s, or `None` if it uses anything outside the documented subset —
/// a leading `/`, `prefix:local[index]` segments joined by `/`, nothing else (no `@attribute` steps,
/// no `*`, no `//`, no predicate other than a bare integer index). Word itself never emits anything
/// wider than this subset for a content-control data binding; a caller's own hand-authored, wider
/// xpath is a case this crate reports `None` for rather than mis-parsing.
fn parse_xpath_steps(xpath: &str) -> Option<Vec<XPathStep<'_>>> {
    let body = xpath.strip_prefix('/')?;
    if body.is_empty() || body.starts_with('/') {
        return None;
    }
    body.split('/').map(parse_xpath_segment).collect()
}

/// Parses one `prefix:local[index]` xpath segment.
fn parse_xpath_segment(segment: &str) -> Option<XPathStep<'_>> {
    let open = segment.find('[')?;
    let close = segment.strip_suffix(']')?;
    if close.len() != segment.len() - 1 {
        return None;
    }
    let index: usize = segment[open + 1..segment.len() - 1].parse().ok()?;
    if index == 0 {
        return None;
    }
    let (prefix, local) = segment[..open].split_once(':')?;
    if prefix.is_empty() || local.is_empty() {
        return None;
    }
    Some(XPathStep {
        prefix,
        local,
        index,
    })
}

/// Resolves `xpath` (the documented subset [`parse_xpath_steps`] accepts) against `root`, using
/// `prefix_mappings` (parsed via [`parse_prefix_mappings`]) to turn each step's prefix into the
/// namespace URI its element must carry — `None` for anything outside the subset, a namespace prefix
/// `prefix_mappings` does not map, or a step that does not resolve against the tree (an out-of-range
/// index, a wrong local name or namespace, too few or too many steps): never a panic, matching this
/// crate's own untrusted-input contract for every other resolution path.
///
/// [`crate::Document::resolve_data_binding`] is the only caller; see this module's own top doc
/// comment for the three-step resolution it performs around this function.
#[must_use]
pub fn resolve_xpath<'a>(
    root: &'a RawElement,
    interner: &Interner,
    prefix_mappings: Option<&str>,
    xpath: &str,
) -> Option<&'a RawElement> {
    let steps = parse_xpath_steps(xpath)?;
    let mappings = prefix_mappings
        .map(parse_prefix_mappings)
        .unwrap_or_default();
    let namespace_for = |prefix: &str| -> Option<&str> {
        mappings
            .iter()
            .find(|(candidate, _)| *candidate == prefix)
            .map(|(_, uri)| *uri)
    };
    let matches = |element: &RawElement, step: &XPathStep<'_>| -> bool {
        let local_matches = interner.resolve(element.name.local) == step.local;
        let namespace_matches = match namespace_for(step.prefix) {
            Some(expected) => element
                .name
                .namespace
                .is_some_and(|ns| interner.resolve(ns) == expected),
            // No mapping for this prefix — fall back to local-name-only matching, the same
            // permissive-on-untrusted-input reading `is_wml_element` (`document/mod.rs`) applies,
            // rather than refusing the whole resolution over one missing `xmlns:` declaration.
            None => true,
        };
        local_matches && namespace_matches
    };
    let (first, rest) = steps.split_first()?;
    if !matches(root, first) || first.index != 1 {
        return None;
    }
    let mut current = root;
    for step in rest {
        let mut candidates = current.children.iter().filter_map(|child| match child {
            RawNode::Element(element) if matches(element, step) => Some(element),
            _ => None,
        });
        current = candidates.nth(step.index - 1)?;
    }
    Some(current)
}

// =================================================================================================
// w:altChunk / w:altChunkPr (CT_AltChunk / CT_AltChunkPr) — external content import
// =================================================================================================

/// One ordered child of an [`AltChunk`]: `CT_AltChunk`'s own `altChunkPr?` sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AltChunkContent {
    /// `w:altChunkPr` (`CT_AltChunkPr`).
    Properties(AltChunkProperties),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_AltChunk` (`w:altChunk`, "Anchor for Imported External Content", §17.17.2.1) — a block-level
/// anchor naming, through `r:id`, an external-content part this crate never converts: its bytes and
/// content type are read and written exactly as the file carries them. See
/// [`crate::Document::alt_chunks`]/[`crate::Document::alt_chunk_payload`] for resolving the
/// relationship to the part it targets.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "id", prefix = "r", codec = TextCodec, accessor = raw_relationship_id))]
pub struct AltChunk {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "altChunkPr", variant = Properties, ty = AltChunkProperties))]
    content: Vec<AltChunkContent>,
}

impl AltChunk {
    /// Builds a fresh `w:altChunk` naming `relationship_id`, no `w:altChunkPr` — the empty-shell
    /// constructor [`crate::Document::add_alt_chunk`] fills the relationship for.
    #[must_use]
    pub(crate) fn new(interner: &mut Interner, relationship_id: &str) -> Self {
        let mut chunk = Self {
            name: wml_name(interner, "altChunk"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        chunk.set_raw_relationship_id(interner, Some(relationship_id));
        chunk
    }

    /// This anchor's own relationship id (`r:id`), naming the external-content part it imports — or
    /// `None` if the element states none (schema-legal: `r:id` is `use="optional"`, though ECMA-376
    /// Part 1 §17.17.2.1 states a conformant `w:altChunk` with no relationship, or one of the wrong
    /// type, "shall be ignored").
    #[must_use]
    pub fn relationship_id(&self, interner: &Interner) -> Option<String> {
        self.raw_relationship_id(interner)
            .ok()
            .flatten()
            .map(std::borrow::Cow::into_owned)
    }

    /// This anchor's own import properties (`w:altChunkPr`), or `None`.
    #[must_use]
    pub fn properties(&self) -> Option<&AltChunkProperties> {
        self.content.iter().find_map(|item| match item {
            AltChunkContent::Properties(value) => Some(value),
            _ => None,
        })
    }
}

/// `CT_AltChunkPr` (`w:altChunkPr`, "External Content Import Properties", §17.17.2.2) — the one
/// property ECMA-376 defines: whether the source's own formatting is used in place of the host
/// document's formatting when the two disagree.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct AltChunkProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "matchSrc", variant = MatchSource, ty = Toggle))]
    content: Vec<AltChunkPropertyContent>,
}

/// One ordered child of [`AltChunkProperties`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AltChunkPropertyContent {
    /// `w:matchSrc` (`CT_OnOff`) — reuses [`Toggle`]'s own `CT_OnOff` shape.
    MatchSource(Toggle),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl AltChunkProperties {
    /// Whether the imported content's own source formatting is used (`w:matchSrc`), if the file
    /// states either way.
    #[must_use]
    pub fn matches_source_formatting(&self, interner: &Interner) -> Option<bool> {
        self.content
            .iter()
            .find_map(|item| match item {
                AltChunkPropertyContent::MatchSource(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.value(interner).ok())
    }
}

// =================================================================================================
// The glossary document's building blocks: CT_DocParts, CT_DocPart, CT_DocPartPr and the identifying
// leaves underneath it. `CT_DocPart`'s own `docPartBody` is `CT_Body` — [`super::body::Body`] itself,
// reused directly, which is what lets the glossary body read through the exact same paragraph/table
// API the main document body does (this ticket's own "no glossary-specific duplicate" requirement).
// =================================================================================================

/// One ordered child of a [`DocParts`]: `CT_DocParts`'s own `docPart+` (at least one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocPartsContent {
    /// `w:docPart` (`CT_DocPart`).
    DocPart(BuildingBlock),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_DocParts` (`w:docParts`, "Document Parts (Building Blocks)", §17.5.1.5) — the glossary
/// document's own list of building blocks.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct DocParts {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "docPart", variant = DocPart, ty = BuildingBlock))]
    content: Vec<DocPartsContent>,
}

impl DocParts {
    /// Every building block this glossary document declares, in document order.
    pub fn building_blocks(&self) -> impl Iterator<Item = &BuildingBlock> {
        self.content.iter().filter_map(|item| match item {
            DocPartsContent::DocPart(block) => Some(block),
            _ => None,
        })
    }

    /// The building block named `name` (`w:docPartPr/w:name/@w:val`), or `None` if this glossary
    /// document declares none by that name (or none carries a `w:docPartPr` at all).
    #[must_use]
    pub fn building_block(&self, interner: &Interner, name: &str) -> Option<&BuildingBlock> {
        self.building_blocks()
            .find(|block| block.name(interner).as_deref() == Some(name))
    }
}

/// One ordered child of a [`BuildingBlock`]: `CT_DocPart`'s own `docPartPr?, docPartBody?` sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildingBlockContent {
    /// `w:docPartPr` (`CT_DocPartPr`).
    Properties(BuildingBlockProperties),
    /// `w:docPartBody` (`CT_Body`) — [`super::body::Body`] itself.
    Body(super::body::Body),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_DocPart` (`w:docPart`, "Document Part (Building Block)", §17.5.1.3) — one entry of
/// [`DocParts`]: AutoText, a cover page, a headers-gallery entry, or any other reusable building
/// block, identified by [`BuildingBlockProperties`] and holding its content as an ordinary
/// [`super::body::Body`] (**never** a bespoke glossary content type — reusing `Body` is what makes
/// "the glossary body reads through the same block-content API as the main body" true by
/// construction rather than by a second implementation that happens to agree with the first).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct BuildingBlock {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "docPartPr", variant = Properties, ty = BuildingBlockProperties),
        child(local = "docPartBody", variant = Body, ty = super::body::Body)
    )]
    content: Vec<BuildingBlockContent>,
}

impl BuildingBlock {
    /// This building block's own properties (`w:docPartPr`), or `None`.
    #[must_use]
    pub fn properties(&self) -> Option<&BuildingBlockProperties> {
        self.content.iter().find_map(|item| match item {
            BuildingBlockContent::Properties(value) => Some(value),
            _ => None,
        })
    }

    /// This building block's own identifying name (`w:docPartPr/w:name/@w:val`), or `None` if it
    /// carries no `w:docPartPr` at all (schema-legal — `docPartPr` is `minOccurs="0"` — though every
    /// building block a real gallery offers names one).
    #[must_use]
    pub fn name(&self, interner: &Interner) -> Option<String> {
        self.properties()
            .and_then(|properties| properties.name(interner))
    }

    /// This building block's own content (`w:docPartBody`) — an ordinary [`super::body::Body`],
    /// reached through the exact same paragraph/table API the main document body is — or `None` if
    /// this entry carries none.
    #[must_use]
    pub fn body(&self) -> Option<&super::body::Body> {
        self.content.iter().find_map(|item| match item {
            BuildingBlockContent::Body(value) => Some(value),
            _ => None,
        })
    }

    /// [`BuildingBlock::body`], mutably.
    pub fn body_mut(&mut self) -> Option<&mut super::body::Body> {
        self.content.iter_mut().find_map(|item| match item {
            BuildingBlockContent::Body(value) => Some(value),
            _ => None,
        })
    }
}

/// `CT_DocPartName` (`w:name`, "Document Part (Building Block) Name", §17.5.1.7).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = value, required))]
#[xml(attribute(local = "decorated", prefix = "w", codec = OnOff, accessor = decorated))]
pub struct BuildingBlockName {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FromXml for BuildingBlockName {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for BuildingBlockName {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_DocPartGallery` (`w:gallery`, inside [`BuildingBlockCategory`], §17.5.1.6 as restated by
/// `w:category`) — which gallery gathers this building block's own category. Distinct from
/// [`BuildingBlockReference`], which is `CT_SdtDocPart` (a *content control's* own gallery/list
/// picker) — the two share a schema neighbourhood but not a complex type.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<DocumentPartGallery>, accessor = gallery, required))]
pub struct BuildingBlockCategoryGallery {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FromXml for BuildingBlockCategoryGallery {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for BuildingBlockCategoryGallery {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// One ordered child of a [`BuildingBlockCategory`]: `CT_DocPartCategory`'s own `name, gallery`
/// sequence (both required).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildingBlockCategoryContent {
    /// `w:name` (`CT_String`).
    Name(TableStringValue),
    /// `w:gallery` (`CT_DocPartGallery`).
    Gallery(BuildingBlockCategoryGallery),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_DocPartCategory` (`w:category`, "Document Part (Building Block) Category", §17.5.1.2) — which
/// named category, within which gallery, a building block belongs to.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct BuildingBlockCategory {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "name", variant = Name, ty = TableStringValue),
        child(local = "gallery", variant = Gallery, ty = BuildingBlockCategoryGallery)
    )]
    content: Vec<BuildingBlockCategoryContent>,
}

impl BuildingBlockCategory {
    /// This category's own name (`w:name/@w:val`), or `None`.
    #[must_use]
    pub fn category_name(&self, interner: &Interner) -> Option<String> {
        self.content
            .iter()
            .find_map(|item| match item {
                BuildingBlockCategoryContent::Name(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.value(interner).ok())
            .map(std::borrow::Cow::into_owned)
    }

    /// Which gallery this category belongs to (`w:gallery/@w:val`), or `None`.
    #[must_use]
    pub fn gallery(&self, interner: &Interner) -> Option<DocumentPartGallery> {
        self.content
            .iter()
            .find_map(|item| match item {
                BuildingBlockCategoryContent::Gallery(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.gallery(interner).ok())
    }
}

/// `CT_DocPartType` (`w:type`, "Document Part (Building Block) Type", §17.5.1.13).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<DocumentPartType>, accessor = kind, required))]
pub struct BuildingBlockType {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FromXml for BuildingBlockType {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for BuildingBlockType {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// One ordered child of [`BuildingBlockTypes`]: `CT_DocPartTypes`'s own `type+` (at least one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildingBlockTypesContent {
    /// `w:type` (`CT_DocPartType`).
    Type(BuildingBlockType),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_DocPartTypes` (`w:types`, "Document Part (Building Block) Types", §17.5.1.14) — every
/// gallery-view context this building block is offered in, plus whether it applies to every context
/// (`all`).
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "all", prefix = "w", codec = OnOff, accessor = applies_to_all))]
pub struct BuildingBlockTypes {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "type", variant = Type, ty = BuildingBlockType))]
    content: Vec<BuildingBlockTypesContent>,
}

impl BuildingBlockTypes {
    /// Every type context this building block declares, in document order.
    pub fn types(&self) -> impl Iterator<Item = &BuildingBlockType> {
        self.content.iter().filter_map(|item| match item {
            BuildingBlockTypesContent::Type(value) => Some(value),
            _ => None,
        })
    }
}

/// `CT_DocPartBehavior` (`w:behavior`, "Document Part (Building Block) Insertion Behavior",
/// §17.5.1.1 as restated for `w:behaviors`'s own child — not to be confused with `CT_Attr`'s own
/// §17.5.1.1, which ECMA-376 Part 1 numbers identically for the two same-named clauses in different
/// sub-sections; verified directly against `wml.xsd`, not the numbering, which is ambiguous here).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<DocumentPartBehavior>, accessor = kind, required))]
pub struct BuildingBlockBehavior {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FromXml for BuildingBlockBehavior {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for BuildingBlockBehavior {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// One ordered child of [`BuildingBlockBehaviors`]: `CT_DocPartBehaviors`'s own `behavior+` (at least
/// one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildingBlockBehaviorsContent {
    /// `w:behavior` (`CT_DocPartBehavior`).
    Behavior(BuildingBlockBehavior),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_DocPartBehaviors` (`w:behaviors`, "Document Part (Building Block) Insertion Behaviors",
/// §17.5.1.2) — every insertion behaviour offered for this building block.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct BuildingBlockBehaviors {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "behavior", variant = Behavior, ty = BuildingBlockBehavior))]
    content: Vec<BuildingBlockBehaviorsContent>,
}

impl BuildingBlockBehaviors {
    /// Every insertion behaviour this building block offers, in document order.
    pub fn behaviors(&self) -> impl Iterator<Item = &BuildingBlockBehavior> {
        self.content.iter().filter_map(|item| match item {
            BuildingBlockBehaviorsContent::Behavior(value) => Some(value),
            _ => None,
        })
    }
}

/// One `xsd:all` member of [`BuildingBlockProperties`]: `CT_DocPartPr`'s seven members, each
/// `minOccurs="0"` except `name` — `xsd:all` places no order constraint on a conformant file, but
/// this crate preserves whatever order a *given* file actually wrote (conformant or not), the same
/// contract every other content enum in this crate keeps for an `xsd:sequence`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildingBlockPropertyContent {
    /// `w:name` (`CT_DocPartName`) — the building block's own identifying name.
    Name(BuildingBlockName),
    /// `w:style` (`CT_String`) — the paragraph/character style applied to inserted content.
    Style(TableStringValue),
    /// `w:category` (`CT_DocPartCategory`).
    Category(BuildingBlockCategory),
    /// `w:types` (`CT_DocPartTypes`).
    Types(BuildingBlockTypes),
    /// `w:behaviors` (`CT_DocPartBehaviors`).
    Behaviors(BuildingBlockBehaviors),
    /// `w:description` (`CT_String`).
    Description(TableStringValue),
    /// `w:guid` (`CT_Guid`) — read as the file's own wire string via [`Guid`], never parsed further.
    Guid(GuidValue),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_Guid` (`w:guid`, "Document Part (Building Block) GUID", §17.5.1.6 restated for the `guid`
/// child of `w:docPartPr`) — the raw `s:ST_Guid` wire string, read via [`Guid`], never validated as a
/// well-formed GUID (an untrusted file's malformed value is still preserved, not rejected).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = raw_value))]
pub struct GuidValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl GuidValue {
    /// This value's own `@w:val`, wrapped as a [`Guid`] — `None` if the attribute is absent or
    /// malformed.
    #[must_use]
    pub fn value(&self, interner: &Interner) -> Option<Guid> {
        self.raw_value(interner)
            .ok()
            .flatten()
            .map(|value| Guid(value.into_owned()))
    }
}

impl FromXml for GuidValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for GuidValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_DocPartPr` (`w:docPartPr`, "Document Part (Building Block) Properties", §17.5.1.4 as restated
/// for the `xsd:all` group `wml.xsd` actually declares for it) — everything identifying a building
/// block: its name (required), style, category, offered types, insertion behaviours, description and
/// GUID.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct BuildingBlockProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "name", variant = Name, ty = BuildingBlockName),
        child(local = "style", variant = Style, ty = TableStringValue),
        child(local = "category", variant = Category, ty = BuildingBlockCategory),
        child(local = "types", variant = Types, ty = BuildingBlockTypes),
        child(local = "behaviors", variant = Behaviors, ty = BuildingBlockBehaviors),
        child(local = "description", variant = Description, ty = TableStringValue),
        child(local = "guid", variant = Guid, ty = GuidValue)
    )]
    content: Vec<BuildingBlockPropertyContent>,
}

impl BuildingBlockProperties {
    /// This building block's own identifying name (`w:name/@w:val`), or `None` if it states none
    /// (illegal per the schema — `name` is the one required `xsd:all` member — but a malformed file
    /// is read, not panicked on).
    #[must_use]
    pub fn name(&self, interner: &Interner) -> Option<String> {
        self.content
            .iter()
            .find_map(|item| match item {
                BuildingBlockPropertyContent::Name(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.value(interner).ok())
            .map(std::borrow::Cow::into_owned)
    }

    /// This building block's own category (`w:category`), or `None`.
    #[must_use]
    pub fn category(&self) -> Option<&BuildingBlockCategory> {
        self.content.iter().find_map(|item| match item {
            BuildingBlockPropertyContent::Category(value) => Some(value),
            _ => None,
        })
    }

    /// This building block's own offered types (`w:types`), or `None`.
    #[must_use]
    pub fn types(&self) -> Option<&BuildingBlockTypes> {
        self.content.iter().find_map(|item| match item {
            BuildingBlockPropertyContent::Types(value) => Some(value),
            _ => None,
        })
    }

    /// This building block's own insertion behaviours (`w:behaviors`), or `None`.
    #[must_use]
    pub fn behaviors(&self) -> Option<&BuildingBlockBehaviors> {
        self.content.iter().find_map(|item| match item {
            BuildingBlockPropertyContent::Behaviors(value) => Some(value),
            _ => None,
        })
    }

    /// This building block's own GUID (`w:guid/@w:val`), or `None`.
    #[must_use]
    pub fn guid(&self, interner: &Interner) -> Option<Guid> {
        self.content
            .iter()
            .find_map(|item| match item {
                BuildingBlockPropertyContent::Guid(value) => Some(value),
                _ => None,
            })
            .and_then(|value| value.value(interner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Lock::new` writes the requested lock kind, readable straight back through the derived
    /// `kind` accessor — the constructor a future writer builds a fresh `w:lock` with.
    #[test]
    fn lock_new_writes_the_requested_kind() {
        let mut interner = Interner::new();
        let lock = Lock::new(&mut interner, LockingType::TagCannotBeDeleted);
        assert_eq!(
            lock.kind(&interner),
            Ok(Some(LockingType::TagCannotBeDeleted))
        );
    }

    /// `DataBinding::new` writes `storeItemID`/`xpath`, leaving `prefixMappings` absent —
    /// [`Document::resolve_data_binding`]'s own two required fields, readable straight back.
    #[test]
    fn data_binding_new_writes_store_item_id_and_xpath() {
        let mut interner = Interner::new();
        let binding = DataBinding::new(&mut interner, "{GUID}", "/ns0:root[1]");
        assert_eq!(binding.store_item_id(&interner).as_deref(), Ok("{GUID}"));
        assert_eq!(binding.xpath(&interner).as_deref(), Ok("/ns0:root[1]"));
        assert_eq!(binding.prefix_mappings(&interner), Ok(None));
    }

    /// `Placeholder::new` writes `w:docPart` naming the building block, readable straight back
    /// through [`Placeholder::doc_part_name`].
    #[test]
    fn placeholder_new_names_the_building_block() {
        let mut interner = Interner::new();
        let placeholder = Placeholder::new(&mut interner, "Cover Page 1");
        assert_eq!(
            placeholder.doc_part_name(&interner).as_deref(),
            Some("Cover Page 1")
        );
    }

    /// `CustomXmlBlock::from_xml` reads `w:element`/`w:uri` and its own block-level content — the
    /// production path every `w:customXml` wrapper in this crate goes through.
    #[test]
    fn custom_xml_block_reads_its_own_element_uri_and_content() {
        const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
        let xml = format!(
            r#"<w:customXml xmlns:w="{W}" w:uri="http://schemas.example.com/customer" w:element="root"><w:p/></w:customXml>"#
        );
        let doc = mjx_xml::fidelity::parse(xml.as_bytes()).expect("fragment parses");
        let block =
            CustomXmlBlock::from_xml(&doc.root, &doc.interner).expect("CustomXmlBlock::from_xml");
        assert_eq!(block.element(&doc.interner).as_deref(), Ok("root"));
        assert_eq!(
            block
                .uri(&doc.interner)
                .expect("uri is readable")
                .as_deref(),
            Some("http://schemas.example.com/customer")
        );
        assert!(block.properties().is_none());
        assert_eq!(block.content().len(), 1);
        assert!(matches!(block.content()[0], BlockContent::Paragraph(_)));
    }

    /// `set_lock`/`set_placeholder`/`set_data_binding` insert at `CT_SdtPr`'s own schema rank (lock
    /// before placeholder before dataBinding, per the sequence) regardless of call order, and `None`
    /// removes each again — the setter surface [`Lock::new`]/[`Placeholder::new`]/[`DataBinding::new`]
    /// exist for.
    #[test]
    fn content_control_properties_setters_insert_at_the_correct_schema_rank() {
        let mut interner = Interner::new();
        let mut properties = ContentControlProperties {
            name: wml_name(&mut interner, "sdtPr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };

        // Deliberately set out of schema order: dataBinding, then placeholder, then lock.
        properties.set_data_binding(&mut interner, Some(("{GUID}", "/ns0:root[1]")));
        properties.set_placeholder(&mut interner, Some("Cover Page 1"));
        properties.set_lock(&mut interner, Some(LockingType::TagCannotBeDeleted));

        let order: Vec<&'static str> = properties
            .content
            .iter()
            .map(|item| match item {
                ContentControlPropertyContent::Lock(_) => "lock",
                ContentControlPropertyContent::Placeholder(_) => "placeholder",
                ContentControlPropertyContent::DataBinding(_) => "dataBinding",
                _ => "other",
            })
            .collect();
        assert_eq!(
            order,
            vec!["lock", "placeholder", "dataBinding"],
            "the three must land in CT_SdtPr's own schema order regardless of call order"
        );

        assert_eq!(
            properties.lock().unwrap().kind(&interner),
            Ok(Some(LockingType::TagCannotBeDeleted))
        );
        assert_eq!(
            properties
                .placeholder()
                .unwrap()
                .doc_part_name(&interner)
                .as_deref(),
            Some("Cover Page 1")
        );
        assert_eq!(
            properties
                .data_binding()
                .unwrap()
                .store_item_id(&interner)
                .as_deref(),
            Ok("{GUID}")
        );

        properties.set_lock(&mut interner, None);
        assert!(properties.lock().is_none());
    }
}
