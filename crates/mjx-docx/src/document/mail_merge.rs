//! Mail merge (`w:settings/w:mailMerge`, `CT_MailMerge` and its Office Data Source Object cluster,
//! §17.15) and `word/recipients.xml` (`CT_Recipients`, §17.14.28) — MJXOFF-136's own file.
//!
//! **ODSO** is *Office Data Source Object* — the mail-merge data-connection description
//! (`w:odso`/`CT_Odso`); the expansion belongs here rather than in a bare `Odso` type name, per this
//! child's own naming constraint. A mail-merge document whose `w:mailMerge`/`w:recipients` are
//! dropped on save stops being a mail-merge document even though every paragraph is intact — the
//! reason this cluster gets a full authoring surface rather than a read-only tier.
//!
//! `word/recipients.xml` is one of `wml.xsd`'s fourteen global elements (`w:recipients`); unlike
//! `word/settings.xml`/`word/webSettings.xml`/`word/fontTable.xml`, `PartKind::Recipients` existed
//! in C1's part graph but [`super::parts::DocumentParts`] never resolved it — this child adds the
//! resolution (see `parts.rs`'s own diff).

use mjx_ooxml_core::{
    AttributeCodec, AttributeError, Enumeration, FromXml, FromXmlError, Interner,
    InvalidAttributeValue, RawAttribute, RawElement, RawName, RawNode, Text as TextCodec, ToXml,
};
use mjx_ooxml_types::child_order::{MAIL_MERGE, ODSO};
use mjx_ooxml_types::wordprocessingml::{
    MailMergeDataType, MailMergeDestination, MailMergeDocumentType, MailMergeFieldMappingType,
    MailMergeSourceType,
};

use super::body::{wml_name, RelationshipReference};
use super::paragraph_properties::DecimalNumberValue;
use super::run_properties::{Lang, Toggle};
use super::styles::StyleString;

// =================================================================================================
// Attribute codecs this module needs: the three `xsd:string`-based (non-`FromStr`) generated wire
// wrappers — same shape as `settings::DocTypeCodec`.
// =================================================================================================

/// `ST_MailMergeDataType` (`w:dataType/@val`) as an attribute value — an unrestricted wire string.
#[derive(Debug)]
pub struct DataTypeCodec;

impl AttributeCodec for DataTypeCodec {
    type Value<'a> = MailMergeDataType;
    type Input<'a> = MailMergeDataType;

    fn decode<'a>(
        raw: std::borrow::Cow<'a, str>,
    ) -> Result<MailMergeDataType, InvalidAttributeValue> {
        Ok(MailMergeDataType::from_wire(&raw))
    }

    fn encode<'a>(value: MailMergeDataType) -> std::borrow::Cow<'a, str> {
        std::borrow::Cow::Owned(value.to_wire().to_owned())
    }
}

// =================================================================================================
// Attribute-only leaves: CT_MailMergeDocType, CT_MailMergeDataType, CT_MailMergeDest,
// CT_MailMergeSourceType, CT_MailMergeOdsoFMDFieldType.
// =================================================================================================

/// `w:mainDocumentType` (`CT_MailMergeDocType`, §17.15.1.67) — what kind of mail-merge main
/// document this is.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<MailMergeDocumentType>, accessor = kind, required))]
pub struct MailMergeDocumentTypeValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl MailMergeDocumentTypeValue {
    /// Builds a new `w:mainDocumentType` of `kind`.
    #[must_use]
    pub fn new(interner: &mut Interner, kind: MailMergeDocumentType) -> Self {
        let mut item = Self {
            name: wml_name(interner, "mainDocumentType"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_kind(interner, kind);
        item
    }
}

impl FromXml for MailMergeDocumentTypeValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for MailMergeDocumentTypeValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:dataType` (`CT_MailMergeDataType`, §17.15.1.16) — the merge data source's own kind
/// (unrestricted string; Word writes values like `"textFile"`, `"native"`).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = DataTypeCodec, accessor = value, required))]
pub struct MailMergeDataTypeValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl MailMergeDataTypeValue {
    /// Builds a new `w:dataType` of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, value: MailMergeDataType) -> Self {
        let mut item = Self {
            name: wml_name(interner, "dataType"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for MailMergeDataTypeValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for MailMergeDataTypeValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:destination` (`CT_MailMergeDest`, §17.15.1.17) — where a completed merge goes (a new
/// document, the printer, e-mail, fax).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<MailMergeDestination>, accessor = kind, required))]
pub struct MailMergeDestinationValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl MailMergeDestinationValue {
    /// Builds a new `w:destination` of `kind`.
    #[must_use]
    pub fn new(interner: &mut Interner, kind: MailMergeDestination) -> Self {
        let mut item = Self {
            name: wml_name(interner, "destination"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_kind(interner, kind);
        item
    }
}

impl FromXml for MailMergeDestinationValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for MailMergeDestinationValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:type` inside `w:odso` (`CT_MailMergeSourceType`, §17.15.1.96) — the ODSO connection's own
/// source kind (ODBC, OLE DB, a text file, …).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<MailMergeSourceType>, accessor = kind, required))]
pub struct MailMergeSourceTypeValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl MailMergeSourceTypeValue {
    /// Builds a new `w:type` of `kind`.
    #[must_use]
    pub fn new(interner: &mut Interner, kind: MailMergeSourceType) -> Self {
        let mut item = Self {
            name: wml_name(interner, "type"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_kind(interner, kind);
        item
    }
}

impl FromXml for MailMergeSourceTypeValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for MailMergeSourceTypeValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:type` inside `w:fieldMapData` (`CT_MailMergeOdsoFMDFieldType`, §17.15.1.68) — which merge
/// field this field-mapping entry supplies (address block, greeting line, …).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<MailMergeFieldMappingType>, accessor = kind, required))]
pub struct MailMergeFieldMappingTypeValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl MailMergeFieldMappingTypeValue {
    /// Builds a new `w:type` of `kind`.
    #[must_use]
    pub fn new(interner: &mut Interner, kind: MailMergeFieldMappingType) -> Self {
        let mut item = Self {
            name: wml_name(interner, "type"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_kind(interner, kind);
        item
    }
}

impl FromXml for MailMergeFieldMappingTypeValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for MailMergeFieldMappingTypeValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// =================================================================================================
// CT_OdsoFieldMapData, CT_Odso
// =================================================================================================

/// `w:fieldMapData` (`CT_OdsoFieldMapData`, §17.15.1.36) — one column-to-merge-field mapping,
/// repeatable.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct OdsoFieldMapEntry {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "type", variant = Kind, ty = MailMergeFieldMappingTypeValue),
        child(local = "name", variant = Name, ty = StyleString),
        child(local = "mappedName", variant = MappedName, ty = StyleString),
        child(local = "column", variant = Column, ty = DecimalNumberValue),
        child(local = "lid", variant = LanguageId, ty = LanguageValue),
        child(local = "dynamicAddress", variant = DynamicAddress, ty = Toggle)
    )]
    content: Vec<OdsoFieldMapEntryContent>,
}

/// One child of [`OdsoFieldMapEntry`]: `CT_OdsoFieldMapData`'s own six, in schema order —
/// hand-ordered directly (six slots; the same reasoning as [`super::settings::CaptionsContent`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OdsoFieldMapEntryContent {
    /// `w:type`.
    Kind(MailMergeFieldMappingTypeValue),
    /// `w:name`.
    Name(StyleString),
    /// `w:mappedName`.
    MappedName(StyleString),
    /// `w:column`.
    Column(DecimalNumberValue),
    /// `w:lid`.
    LanguageId(LanguageValue),
    /// `w:dynamicAddress`.
    DynamicAddress(Toggle),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl OdsoFieldMapEntry {
    /// Builds a new, empty `w:fieldMapData`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "fieldMapData"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }
}

/// `w:lid` (`CT_Lang`) — a single required language tag, distinct from `w:lang`/`w:themeFontLang`'s
/// `CT_Language` (`val`/`eastAsia`/`bidi`).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Lang, accessor = language, required))]
pub struct LanguageValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl LanguageValue {
    /// Builds a new `w:lid` of `language`.
    #[must_use]
    pub fn new(interner: &mut Interner, language: mjx_ooxml_types::shared::LanguageTag) -> Self {
        let mut item = Self {
            name: wml_name(interner, "lid"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_language(interner, language);
        item
    }
}

impl FromXml for LanguageValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for LanguageValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:odso` (`CT_Odso`, §17.15.1.75) — the Office Data Source Object description: connection,
/// source table, then field-mapping and per-recipient data.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Odso {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "udl", variant = ConnectionString, ty = StyleString),
        child(local = "table", variant = Table, ty = StyleString),
        child(local = "src", variant = Source, ty = RelationshipReference),
        child(local = "colDelim", variant = ColumnDelimiter, ty = DecimalNumberValue),
        child(local = "type", variant = Kind, ty = MailMergeSourceTypeValue),
        child(local = "fHdr", variant = HasHeaderRow, ty = Toggle),
        child(local = "fieldMapData", variant = FieldMap, ty = OdsoFieldMapEntry),
        child(local = "recipientData", variant = RecipientDataSource, ty = RelationshipReference)
    )]
    content: Vec<OdsoContent>,
}

/// One child of [`Odso`]: `CT_Odso`'s own eight, ranked from the generated [`ODSO`] table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OdsoContent {
    /// `w:udl` — the connection string.
    ConnectionString(StyleString),
    /// `w:table`.
    Table(StyleString),
    /// `w:src` — `CT_Rel`.
    Source(RelationshipReference),
    /// `w:colDelim`.
    ColumnDelimiter(DecimalNumberValue),
    /// `w:type`.
    Kind(MailMergeSourceTypeValue),
    /// `w:fHdr`.
    HasHeaderRow(Toggle),
    /// `w:fieldMapData` — repeatable.
    FieldMap(OdsoFieldMapEntry),
    /// `w:recipientData` — `CT_Rel`, repeatable (a legacy per-recipient reference; the modern
    /// mechanism is `word/recipients.xml` itself, see [`Recipients`]).
    RecipientDataSource(RelationshipReference),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl Odso {
    /// Builds a new, empty `w:odso`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "odso"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn local(item: &OdsoContent) -> Option<&'static str> {
        Some(match item {
            OdsoContent::ConnectionString(_) => "udl",
            OdsoContent::Table(_) => "table",
            OdsoContent::Source(_) => "src",
            OdsoContent::ColumnDelimiter(_) => "colDelim",
            OdsoContent::Kind(_) => "type",
            OdsoContent::HasHeaderRow(_) => "fHdr",
            OdsoContent::FieldMap(_) => "fieldMapData",
            OdsoContent::RecipientDataSource(_) => "recipientData",
            OdsoContent::Raw(_) => return None,
        })
    }

    fn rank(item: &OdsoContent) -> Option<u16> {
        Self::local(item).and_then(|local| ODSO.rank_of(None, local))
    }

    fn insert(&mut self, local: &str, item: OdsoContent) {
        let at = ODSO.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    /// Every `w:fieldMapData`, in document order.
    pub fn field_maps(&self) -> impl Iterator<Item = &OdsoFieldMapEntry> {
        self.content.iter().filter_map(|item| match item {
            OdsoContent::FieldMap(value) => Some(value),
            _ => None,
        })
    }

    /// Appends a new `w:fieldMapData` at its schema rank.
    pub fn add_field_map(&mut self, value: OdsoFieldMapEntry) {
        self.insert("fieldMapData", OdsoContent::FieldMap(value));
    }

    /// Every `w:recipientData` (`CT_Rel`), in document order.
    pub fn recipient_data_sources(&self) -> impl Iterator<Item = &RelationshipReference> {
        self.content.iter().filter_map(|item| match item {
            OdsoContent::RecipientDataSource(value) => Some(value),
            _ => None,
        })
    }

    /// Appends a new `w:recipientData` at its schema rank.
    pub fn add_recipient_data_source(&mut self, value: RelationshipReference) {
        self.insert("recipientData", OdsoContent::RecipientDataSource(value));
    }
}

// =================================================================================================
// CT_MailMerge
// =================================================================================================

/// `w:mailMerge` (`CT_MailMerge`, §17.15.1.66) — a document's own mail-merge configuration.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct MailMergeSettings {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "mainDocumentType", variant = MainDocumentType, ty = MailMergeDocumentTypeValue),
        child(local = "linkToQuery", variant = LinkToQuery, ty = Toggle),
        child(local = "dataType", variant = DataType, ty = MailMergeDataTypeValue),
        child(local = "connectString", variant = ConnectString, ty = StyleString),
        child(local = "query", variant = Query, ty = StyleString),
        child(local = "dataSource", variant = DataSource, ty = RelationshipReference),
        child(local = "headerSource", variant = HeaderSource, ty = RelationshipReference),
        child(local = "doNotSuppressBlankLines", variant = DoNotSuppressBlankLines, ty = Toggle),
        child(local = "destination", variant = Destination, ty = MailMergeDestinationValue),
        child(local = "addressFieldName", variant = AddressFieldName, ty = StyleString),
        child(local = "mailSubject", variant = MailSubject, ty = StyleString),
        child(local = "mailAsAttachment", variant = MailAsAttachment, ty = Toggle),
        child(local = "viewMergedData", variant = ViewMergedData, ty = Toggle),
        child(local = "activeRecord", variant = ActiveRecord, ty = DecimalNumberValue),
        child(local = "checkErrors", variant = CheckErrors, ty = DecimalNumberValue),
        child(local = "odso", variant = DataDescription, ty = Odso)
    )]
    content: Vec<MailMergeContent>,
}

/// One child of [`MailMergeSettings`]: `CT_MailMerge`'s own sixteen, ranked from the generated
/// [`MAIL_MERGE`] table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailMergeContent {
    /// `w:mainDocumentType` — required per the schema (`minOccurs="1"`), though reading never
    /// rejects a file that omits it (fidelity-first).
    MainDocumentType(MailMergeDocumentTypeValue),
    /// `w:linkToQuery`.
    LinkToQuery(Toggle),
    /// `w:dataType` — required per the schema; see [`MailMergeContent::MainDocumentType`].
    DataType(MailMergeDataTypeValue),
    /// `w:connectString`.
    ConnectString(StyleString),
    /// `w:query`.
    Query(StyleString),
    /// `w:dataSource` — `CT_Rel`.
    DataSource(RelationshipReference),
    /// `w:headerSource` — `CT_Rel`.
    HeaderSource(RelationshipReference),
    /// `w:doNotSuppressBlankLines`.
    DoNotSuppressBlankLines(Toggle),
    /// `w:destination`.
    Destination(MailMergeDestinationValue),
    /// `w:addressFieldName`.
    AddressFieldName(StyleString),
    /// `w:mailSubject`.
    MailSubject(StyleString),
    /// `w:mailAsAttachment`.
    MailAsAttachment(Toggle),
    /// `w:viewMergedData`.
    ViewMergedData(Toggle),
    /// `w:activeRecord`.
    ActiveRecord(DecimalNumberValue),
    /// `w:checkErrors`.
    CheckErrors(DecimalNumberValue),
    /// `w:odso` — this module's own [`Odso`] cluster.
    DataDescription(Odso),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl MailMergeSettings {
    /// Builds a new, empty `w:mailMerge`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "mailMerge"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn local(item: &MailMergeContent) -> Option<&'static str> {
        Some(match item {
            MailMergeContent::MainDocumentType(_) => "mainDocumentType",
            MailMergeContent::LinkToQuery(_) => "linkToQuery",
            MailMergeContent::DataType(_) => "dataType",
            MailMergeContent::ConnectString(_) => "connectString",
            MailMergeContent::Query(_) => "query",
            MailMergeContent::DataSource(_) => "dataSource",
            MailMergeContent::HeaderSource(_) => "headerSource",
            MailMergeContent::DoNotSuppressBlankLines(_) => "doNotSuppressBlankLines",
            MailMergeContent::Destination(_) => "destination",
            MailMergeContent::AddressFieldName(_) => "addressFieldName",
            MailMergeContent::MailSubject(_) => "mailSubject",
            MailMergeContent::MailAsAttachment(_) => "mailAsAttachment",
            MailMergeContent::ViewMergedData(_) => "viewMergedData",
            MailMergeContent::ActiveRecord(_) => "activeRecord",
            MailMergeContent::CheckErrors(_) => "checkErrors",
            MailMergeContent::DataDescription(_) => "odso",
            MailMergeContent::Raw(_) => return None,
        })
    }

    fn rank(item: &MailMergeContent) -> Option<u16> {
        Self::local(item).and_then(|local| MAIL_MERGE.rank_of(None, local))
    }

    fn remove(&mut self, is_target: impl Fn(&MailMergeContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: MailMergeContent) {
        let at = MAIL_MERGE.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&MailMergeContent) -> bool,
        value: Option<MailMergeContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    super::property_macros::value_property!(
        MailMergeContent,
        main_document_type,
        set_main_document_type,
        MainDocumentType,
        MailMergeDocumentTypeValue,
        "mainDocumentType",
        "`w:mainDocumentType`."
    );
    super::property_macros::toggle_property!(
        MailMergeContent,
        link_to_query,
        set_link_to_query,
        LinkToQuery,
        "linkToQuery",
        "`w:linkToQuery`."
    );
    super::property_macros::value_property!(
        MailMergeContent,
        data_type,
        set_data_type,
        DataType,
        MailMergeDataTypeValue,
        "dataType",
        "`w:dataType`."
    );
    super::property_macros::value_property!(
        MailMergeContent,
        connect_string,
        set_connect_string,
        ConnectString,
        StyleString,
        "connectString",
        "`w:connectString`."
    );
    super::property_macros::value_property!(
        MailMergeContent,
        query,
        set_query,
        Query,
        StyleString,
        "query",
        "`w:query`."
    );
    super::property_macros::value_property!(
        MailMergeContent,
        data_source,
        set_data_source,
        DataSource,
        RelationshipReference,
        "dataSource",
        "`w:dataSource`."
    );
    super::property_macros::value_property!(
        MailMergeContent,
        header_source,
        set_header_source,
        HeaderSource,
        RelationshipReference,
        "headerSource",
        "`w:headerSource`."
    );
    super::property_macros::toggle_property!(
        MailMergeContent,
        do_not_suppress_blank_lines,
        set_do_not_suppress_blank_lines,
        DoNotSuppressBlankLines,
        "doNotSuppressBlankLines",
        "`w:doNotSuppressBlankLines`."
    );
    super::property_macros::value_property!(
        MailMergeContent,
        destination,
        set_destination,
        Destination,
        MailMergeDestinationValue,
        "destination",
        "`w:destination`."
    );
    super::property_macros::value_property!(
        MailMergeContent,
        address_field_name,
        set_address_field_name,
        AddressFieldName,
        StyleString,
        "addressFieldName",
        "`w:addressFieldName`."
    );
    super::property_macros::value_property!(
        MailMergeContent,
        mail_subject,
        set_mail_subject,
        MailSubject,
        StyleString,
        "mailSubject",
        "`w:mailSubject`."
    );
    super::property_macros::toggle_property!(
        MailMergeContent,
        mail_as_attachment,
        set_mail_as_attachment,
        MailAsAttachment,
        "mailAsAttachment",
        "`w:mailAsAttachment`."
    );
    super::property_macros::toggle_property!(
        MailMergeContent,
        view_merged_data,
        set_view_merged_data,
        ViewMergedData,
        "viewMergedData",
        "`w:viewMergedData`."
    );
    super::property_macros::value_property!(
        MailMergeContent,
        active_record,
        set_active_record,
        ActiveRecord,
        DecimalNumberValue,
        "activeRecord",
        "`w:activeRecord`."
    );
    super::property_macros::value_property!(
        MailMergeContent,
        check_errors,
        set_check_errors,
        CheckErrors,
        DecimalNumberValue,
        "checkErrors",
        "`w:checkErrors`."
    );
    super::property_macros::value_property!(
        MailMergeContent,
        data_description,
        set_data_description,
        DataDescription,
        Odso,
        "odso",
        "`w:odso` — the ODSO (Office Data Source Object) connection description."
    );
}

// =================================================================================================
// word/recipients.xml (CT_Recipients) — the fourteenth wml.xsd global element this crate reaches.
// =================================================================================================

/// `w:uniqueTag` (`CT_Base64Binary`) — an opaque base64-encoded per-recipient tag. Typed as text:
/// the base64 encoding *is* the wire form, so there is nothing to decode, and nothing here ever
/// computes or verifies one.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = base64, required))]
pub struct Base64BinaryValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Base64BinaryValue {
    /// Builds a new `w:uniqueTag` carrying `base64` verbatim.
    #[must_use]
    pub fn new(interner: &mut Interner, base64: &str) -> Self {
        let mut item = Self {
            name: wml_name(interner, "uniqueTag"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_base64(interner, base64);
        item
    }
}

impl FromXml for Base64BinaryValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Base64BinaryValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:recipientData` (`CT_RecipientData`, §17.14.27) — one recipient row's own merge-eligibility
/// flag, source row number and unique tag. `CT_RecipientData`'s three children are always in this
/// fixed order (`active?, column, uniqueTag`), so this type is built and read positionally rather
/// than through a ranked `Vec<enum>` — there is no reordering to defend against once the value is
/// itself immutable-shaped, and every foreign extension a real file might still carry is preserved
/// through [`RecipientData::extra`].
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct RecipientData {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "active", variant = Active, ty = Toggle),
        child(local = "column", variant = Column, ty = DecimalNumberValue),
        child(local = "uniqueTag", variant = UniqueTag, ty = Base64BinaryValue)
    )]
    content: Vec<RecipientDataContent>,
}

/// One child of [`RecipientData`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecipientDataContent {
    /// `w:active`.
    Active(Toggle),
    /// `w:column` — required per the schema.
    Column(DecimalNumberValue),
    /// `w:uniqueTag` — required per the schema.
    UniqueTag(Base64BinaryValue),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl RecipientData {
    /// Builds a new `w:recipientData` naming source row `column`, tagged `unique_tag` (base64,
    /// verbatim), `w:active` absent (present-and-defaulting-true per `CT_OnOff`'s own rule).
    #[must_use]
    pub fn new(interner: &mut Interner, column: i64, unique_tag: &str) -> Self {
        Self {
            name: wml_name(interner, "recipientData"),
            attributes: Vec::new(),
            empty: false,
            content: vec![
                RecipientDataContent::Column(DecimalNumberValue::new(interner, "column", column)),
                RecipientDataContent::UniqueTag(Base64BinaryValue::new(interner, unique_tag)),
            ],
        }
    }

    /// Whether this row is included in the merge (`w:active`'s own tri-state; `None` when the
    /// element is absent — per §17.14.1's own prose, absence means *included*).
    pub fn active(
        &self,
        interner: &Interner,
    ) -> Result<Option<bool>, mjx_ooxml_core::AttributeError> {
        self.content
            .iter()
            .find_map(|item| match item {
                RecipientDataContent::Active(value) => Some(value),
                _ => None,
            })
            .map(|value| value.value(interner))
            .transpose()
    }

    /// Sets (or, with `None`, removes) `w:active`.
    pub fn set_active(&mut self, interner: &mut Interner, value: Option<bool>) {
        self.content
            .retain(|item| !matches!(item, RecipientDataContent::Active(_)));
        if let Some(value) = value {
            let mut toggle = Toggle::new(interner, "active");
            toggle.set_value(interner, Some(value));
            self.content.insert(0, RecipientDataContent::Active(toggle));
        }
    }

    /// `w:column` — the row's own source-data column/index.
    #[must_use]
    pub fn column(&self) -> &DecimalNumberValue {
        self.content
            .iter()
            .find_map(|item| match item {
                RecipientDataContent::Column(value) => Some(value),
                _ => None,
            })
            .expect("RecipientData::new always writes w:column")
    }

    /// `w:uniqueTag` — the row's own opaque per-recipient tag.
    #[must_use]
    pub fn unique_tag(&self) -> &Base64BinaryValue {
        self.content
            .iter()
            .find_map(|item| match item {
                RecipientDataContent::UniqueTag(value) => Some(value),
                _ => None,
            })
            .expect("RecipientData::new always writes w:uniqueTag")
    }
}

/// `word/recipients.xml`'s own root (`w:recipients`, `CT_Recipients`, §17.14.28) — the Mail Merge
/// Recipient Data part: every row a mail-merge document's own data source resolved to, cached so
/// the merge can be replayed without re-querying the original source.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Recipients {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "recipientData", variant = Row, ty = RecipientData))]
    content: Vec<RecipientsContent>,
}

/// One child of [`Recipients`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecipientsContent {
    /// `w:recipientData` — repeatable, at least one per the schema (`minOccurs="1"`; reading never
    /// rejects a file with none — fidelity-first).
    Row(RecipientData),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl Recipients {
    /// Builds a new, empty `word/recipients.xml` root — no rows until [`Recipients::add_row`]
    /// states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "recipients"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every recipient row, in document order.
    pub fn rows(&self) -> impl Iterator<Item = &RecipientData> {
        self.content.iter().filter_map(|item| match item {
            RecipientsContent::Row(value) => Some(value),
            RecipientsContent::Raw(_) => None,
        })
    }

    /// Appends `row` — the schema imposes no order among `w:recipientData` siblings.
    pub fn add_row(&mut self, row: RecipientData) {
        self.content.push(RecipientsContent::Row(row));
        self.empty = false;
    }
}
