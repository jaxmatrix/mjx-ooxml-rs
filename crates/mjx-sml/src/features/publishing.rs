//! What a sheet records about the world **outside** it: the custom-property parts hung off it, the
//! consolidation it was built from, and the fragments it publishes as HTML.
//!
//! | Type | `sml.xsd` | Element | `CT_Worksheet` rank |
//! |---|---|---|---|
//! | [`DataConsolidation`] | 2433 | `x:dataConsolidate` | **12** |
//! | [`DataReferences`] | 2458 | `x:dataConsolidate/dataRefs` | |
//! | [`DataReference`] | 2464 | `x:dataRefs/dataRef` | |
//! | [`CustomProperties`] | 3037 | `x:customProperties` | **25** |
//! | [`CustomProperty`] | 3042 | `x:customProperties/customPr` | |
//! | [`WebPublishItems`] | 3092 | `x:webPublishItems` | **36** |
//! | [`WebPublishItem`] | 3099 | `x:webPublishItems/webPublishItem` | |
//!
//! Three small `sml.xsd` clusters filling three `CT_Worksheet` slots that nothing else in Phase D
//! claimed. They share this file because they share the one property that matters here: **each holds
//! an edge to something this library does not follow.**
//!
//! * a [`CustomProperty`] names a **part** by `r:id` — an arbitrary blob an application hung off the
//!   sheet under a name of its own choosing. This crate holds the raw identifier, exactly as
//!   [`TablePart`](crate::TablePart) does; resolving it is `mjx-xlsx`'s;
//! * a [`DataReference`] names a range that may be **on another sheet or in another workbook** —
//!   `@sheet` names the sheet, `@r:id` the external link when there is one — and nothing here
//!   resolves a single one of the three;
//! * a [`WebPublishItem`] names a **destination file** on somebody's disk or web server
//!   (`@destinationFile`). It is an untrusted path, treated exactly as MJXOFF-127 treats an external
//!   hyperlink target: preserved as written, never resolved, never rewritten, and never opened.
//!
//! # A consolidation is a record, and nothing here performs one
//!
//! `x:dataConsolidate` says that this sheet's contents were produced by combining ranges with a
//! function — `sum`, `average`, `count` and eight others. It is a **record of that**, in the sense
//! [`crate::features`] gives the word for filters, sorts and validations: reading one tells you what
//! Excel was asked to do, and no call in this workspace combines a range, evaluates a function, or
//! refreshes a consolidated cell. `@link`, which says the consolidation is live rather than pasted,
//! is reported and never honoured.

use mjx_ooxml_core::{
    Enumeration, Interner, Number, RawAttribute, RawElement, RawName, RawNode, Text, ToXml,
};
use mjx_ooxml_types::child_order::DATA_CONSOLIDATION;
use mjx_ooxml_types::spreadsheetml::{DataConsolidateFunction, WebSourceType};
use mjx_ooxml_types::support::OnOff;

use crate::address::CellRange;
use crate::leaf::{attribute_bag, relationship_reference};
use crate::worksheet::rebuild_element;

// -----------------------------------------------------------------------------------------------
// `x:dataConsolidate` — rank 12
// -----------------------------------------------------------------------------------------------

attribute_bag! {
    /// `x:dataRef` (`CT_DataRef`, `sml.xsd:2464`) — one of the ranges a consolidation draws from.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_DataRef`. Wire element: `dataRef`.
    ///
    /// **Every one of its four attributes is optional**, which is unusual and is the whole shape of
    /// the type: a reference states its range (`@ref`), or a defined name (`@name`), or the sheet it
    /// is on (`@sheet`), or the external workbook it is in (`@r:id`) — in whatever combination the
    /// producer needed. A `dataRef` that states none of them is valid markup that names nothing, and
    /// this type reports exactly that rather than refusing the file.
    ///
    /// Nothing here resolves `@name` to a defined name, `@sheet` to a tab, or `@r:id` to an external
    /// link.
    #[xml(attribute(local = "ref", codec = Enumeration<CellRange>, accessor = range))]
    #[xml(attribute(local = "name", codec = Text, accessor = name))]
    #[xml(attribute(local = "sheet", codec = Text, accessor = sheet_name))]
    DataReference, "dataRef"
}

relationship_reference!(DataReference);

/// `x:dataRefs` (`CT_DataRefs`, `sml.xsd:2458`) — the ranges a consolidation draws from.
///
/// **`ST_`/`CT_` symbol:** `CT_DataRefs`. Wire element: `dataRefs`.
///
/// `dataRef` is `minOccurs="0"`, so an empty `<dataRefs/>` is valid markup and a file that writes one
/// keeps it — the shape [`TableParts`](crate::TableParts) has, and not the one every other collection
/// in this file has.
///
/// `@count` is a producer's cache: refreshed when this collection is edited **and the file declared
/// one**, never added to an element that wrote none.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count))]
pub struct DataReferences {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "dataRef", variant = Reference, ty = DataReference))]
    content: Vec<DataReferencesContent>,
}

/// One child of [`DataReferences`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataReferencesContent {
    /// `x:dataRef` — one source range.
    Reference(DataReference),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl DataReferences {
    /// Builds an empty `x:dataRefs`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "dataRefs"),
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
    pub fn content(&self) -> &[DataReferencesContent] {
        &self.content
    }

    /// Every `x:dataRef`, in document order.
    pub fn references(&self) -> impl Iterator<Item = &DataReference> + '_ {
        self.content.iter().filter_map(|item| match item {
            DataReferencesContent::Reference(reference) => Some(reference),
            DataReferencesContent::Raw(_) => None,
        })
    }

    /// How many source ranges the consolidation lists.
    #[must_use]
    pub fn len(&self) -> usize {
        self.references().count()
    }

    /// Whether the element lists none, which the schema permits.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends a source range, refreshing `@count` when the file declared one.
    pub fn push(&mut self, interner: &mut Interner, reference: DataReference) {
        self.content
            .push(DataReferencesContent::Reference(reference));
        self.empty = false;
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
                DataReferencesContent::Reference(reference) => {
                    RawNode::Element(reference.as_raw_element())
                }
                DataReferencesContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for DataReferences {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:dataConsolidate` (`CT_DataConsolidate`, `sml.xsd:2433`) — the consolidation this sheet records.
///
/// **`ST_`/`CT_` symbol:** `CT_DataConsolidate`. Wire element: `dataConsolidate`, rank **12** of
/// `CT_Worksheet` — between `sortState` (11) and `customSheetViews` (13).
///
/// `@function` is `ST_DataConsolidateFunction`, whose generated names are **not** its wire tokens:
/// `count` is [`DataConsolidateFunction::CountNonEmpty`], `countNums` is `CountNumbers`, `stdDev` is
/// `SampleStandardDeviation` and `stdDevp` is `PopulationStandardDeviation`. Read the generated enum
/// rather than inferring a variant from a token.
///
/// The four flags are `xsd:boolean` defaulting to `false`: `@startLabels`, `@leftLabels` and
/// `@topLabels` say where the source ranges carry their labels, and `@link` says the consolidation is
/// a live link rather than a paste. **Nothing here consolidates anything**; see the
/// [module documentation](self).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "function", codec = Enumeration<DataConsolidateFunction>, accessor = function, default = DataConsolidateFunction::Sum))]
#[xml(attribute(local = "startLabels", codec = OnOff, accessor = uses_start_labels, default = false))]
#[xml(attribute(local = "leftLabels", codec = OnOff, accessor = uses_left_labels, default = false))]
#[xml(attribute(local = "topLabels", codec = OnOff, accessor = uses_top_labels, default = false))]
#[xml(attribute(local = "link", codec = OnOff, accessor = is_linked, default = false))]
pub struct DataConsolidation {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "dataRefs", variant = References, ty = DataReferences))]
    content: Vec<DataConsolidationContent>,
}

/// One child of [`DataConsolidation`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataConsolidationContent {
    /// `x:dataRefs` (rank 0) — the source ranges.
    References(DataReferences),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl DataConsolidationContent {
    /// This child's rank in `CT_DataConsolidate`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        match self {
            Self::References(_) => DATA_CONSOLIDATION.rank_of(None, "dataRefs"),
            Self::Raw(_) => None,
        }
    }
}

impl DataConsolidation {
    /// Builds an empty `x:dataConsolidate`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "dataConsolidate"),
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
    pub fn content(&self) -> &[DataConsolidationContent] {
        &self.content
    }

    /// `x:dataRefs` — `None` for a consolidation that lists no sources, which the schema permits.
    #[must_use]
    pub fn references(&self) -> Option<&DataReferences> {
        self.content.iter().find_map(|item| match item {
            DataConsolidationContent::References(refs) => Some(refs),
            DataConsolidationContent::Raw(_) => None,
        })
    }

    /// `x:dataRefs`, mutably.
    pub fn references_mut(&mut self) -> Option<&mut DataReferences> {
        self.content.iter_mut().find_map(|item| match item {
            DataConsolidationContent::References(refs) => Some(refs),
            DataConsolidationContent::Raw(_) => None,
        })
    }

    /// Sets `x:dataRefs`, replacing the existing element where it is or inserting at rank 0.
    pub fn set_references(&mut self, references: DataReferences) {
        if let Some(item) = self
            .content
            .iter_mut()
            .find(|item| matches!(item, DataConsolidationContent::References(_)))
        {
            *item = DataConsolidationContent::References(references);
            return;
        }
        let at = DATA_CONSOLIDATION.insert_index_of_names(
            self.content.iter().map(DataConsolidationContent::rank),
            "dataRefs",
        );
        self.content
            .insert(at, DataConsolidationContent::References(references));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                DataConsolidationContent::References(refs) => {
                    RawNode::Element(refs.as_raw_element())
                }
                DataConsolidationContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for DataConsolidation {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// `x:customProperties` — rank 25
// -----------------------------------------------------------------------------------------------

attribute_bag! {
    /// `x:customPr` (`CT_CustomProperty`, `sml.xsd:3042`) — one named part hung off this sheet.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_CustomProperty`. Wire element: `customPr`.
    ///
    /// Both of its attributes are `use="required"`: `@name`, which is the application's own label for
    /// the property, and `@r:id`, which reaches the part holding its **value**. The value is
    /// therefore not here at all — it is an arbitrary blob in a part of its own, and this type is the
    /// edge to it.
    ///
    /// **Not `docProps/custom.xml`.** That is the *package's* custom properties, a different part
    /// with a different schema; this is a worksheet's.
    #[xml(attribute(local = "name", codec = Text, accessor = name, required))]
    CustomProperty, "customPr"
}

relationship_reference!(CustomProperty);

/// `x:customProperties` (`CT_CustomProperties`, `sml.xsd:3037`) — every custom property part this
/// sheet names, in document order.
///
/// **`ST_`/`CT_` symbol:** `CT_CustomProperties`. Wire element: `customProperties`, rank **25** of
/// `CT_Worksheet`.
///
/// `customPr` is `minOccurs="1"`, and the type declares no `@count`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
#[xml(namespace = SML)]
pub struct CustomProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "customPr", variant = Property, ty = CustomProperty))]
    content: Vec<CustomPropertiesContent>,
}

/// One child of [`CustomProperties`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomPropertiesContent {
    /// `x:customPr` — one named part.
    Property(CustomProperty),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl CustomProperties {
    /// Builds an empty `x:customProperties`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "customProperties"),
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
    pub fn content(&self) -> &[CustomPropertiesContent] {
        &self.content
    }

    /// Every `x:customPr`, in document order.
    pub fn properties(&self) -> impl Iterator<Item = &CustomProperty> + '_ {
        self.content.iter().filter_map(|item| match item {
            CustomPropertiesContent::Property(property) => Some(property),
            CustomPropertiesContent::Raw(_) => None,
        })
    }

    /// How many custom properties the sheet names.
    #[must_use]
    pub fn len(&self) -> usize {
        self.properties().count()
    }

    /// Whether the element lists none, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends a property after the ones already present.
    pub fn push(&mut self, property: CustomProperty) {
        self.content
            .push(CustomPropertiesContent::Property(property));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                CustomPropertiesContent::Property(property) => {
                    RawNode::Element(property.as_raw_element())
                }
                CustomPropertiesContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for CustomProperties {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// `x:webPublishItems` — rank 36
// -----------------------------------------------------------------------------------------------

attribute_bag! {
    /// `x:webPublishItem` (`CT_WebPublishItem`, `sml.xsd:3099`) — one fragment of this sheet
    /// published as HTML, and where it was published to.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_WebPublishItem`. Wire element: `webPublishItem`.
    ///
    /// Four of its eight attributes are `use="required"` — `@id`, `@divId`, `@sourceType` and
    /// `@destinationFile`.
    ///
    /// **`@destinationFile` is an untrusted path**, and it is treated exactly as an external
    /// hyperlink target is: preserved as the file wrote it, never resolved against anything, never
    /// rewritten, and never opened. It may name a UNC share, a local path on somebody else's machine,
    /// or a web server; none of that is this library's business.
    ///
    /// `@sourceType` is `ST_WebSourceType` — `sheet`, `printArea`, `autoFilter`, `range`, `chart`,
    /// `pivotTable`, `query` or `label` — and `@sourceRef` / `@sourceObject` narrow it. Nothing here
    /// resolves either into what it names.
    #[xml(attribute(local = "id", codec = Number<u32>, accessor = id, required))]
    #[xml(attribute(local = "divId", codec = Text, accessor = division_id, required))]
    #[xml(attribute(local = "sourceType", codec = Enumeration<WebSourceType>, accessor = source_type, required))]
    #[xml(attribute(local = "sourceRef", codec = Enumeration<CellRange>, accessor = source_range))]
    #[xml(attribute(local = "sourceObject", codec = Text, accessor = source_object))]
    #[xml(attribute(local = "destinationFile", codec = Text, accessor = destination_file, required))]
    #[xml(attribute(local = "title", codec = Text, accessor = title))]
    #[xml(attribute(local = "autoRepublish", codec = OnOff, accessor = republishes_automatically, default = false))]
    WebPublishItem, "webPublishItem"
}

/// `x:webPublishItems` (`CT_WebPublishItems`, `sml.xsd:3092`) — every published fragment of this
/// sheet, in document order.
///
/// **`ST_`/`CT_` symbol:** `CT_WebPublishItems`. Wire element: `webPublishItems`, rank **36** of
/// `CT_Worksheet` — the last slot before `tableParts` (37) and `extLst` (38).
///
/// **Not** `xl/workbook.xml`'s `webPublishObjects`, which is the workbook-level list
/// ([`WebPublishObjects`](crate::WebPublishObjects), MJXOFF-100's) of the same publishing feature.
///
/// `@count` is a producer's cache, refreshed only when the file declared one.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count))]
pub struct WebPublishItems {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "webPublishItem", variant = Item, ty = WebPublishItem))]
    content: Vec<WebPublishItemsContent>,
}

/// One child of [`WebPublishItems`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebPublishItemsContent {
    /// `x:webPublishItem` — one published fragment.
    Item(WebPublishItem),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl WebPublishItems {
    /// Builds an empty `x:webPublishItems`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "webPublishItems"),
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
    pub fn content(&self) -> &[WebPublishItemsContent] {
        &self.content
    }

    /// Every `x:webPublishItem`, in document order.
    pub fn items(&self) -> impl Iterator<Item = &WebPublishItem> + '_ {
        self.content.iter().filter_map(|item| match item {
            WebPublishItemsContent::Item(published) => Some(published),
            WebPublishItemsContent::Raw(_) => None,
        })
    }

    /// How many fragments the sheet publishes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items().count()
    }

    /// Whether the element lists none, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends a published fragment, refreshing `@count` when the file declared one.
    pub fn push(&mut self, interner: &mut Interner, item: WebPublishItem) {
        self.content.push(WebPublishItemsContent::Item(item));
        self.empty = false;
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
                WebPublishItemsContent::Item(published) => {
                    RawNode::Element(published.as_raw_element())
                }
                WebPublishItemsContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for WebPublishItems {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}
