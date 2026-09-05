//! `x:webPublishing`, `x:webPublishObjects` and the smart-tag pair — the four remaining slots of
//! `CT_Workbook`'s sequence, all of them legacy.
//!
//! Save-as-web-page (`CT_WebPublishing` at `sml.xsd:4388`, `CT_WebPublishObjects` at `4424`,
//! `CT_WebPublishObject` at `4431`) and smart tags (`CT_SmartTagPr` at `4257`, `CT_SmartTagTypes` at
//! `4268`, `CT_SmartTagType` at `4273`) are features no current version of Excel authors. They are
//! modelled here for the reason the whole sequence is: an old workbook that carries them and a
//! writer that dropped them would produce a file that lost something, and the elements occupy ranks
//! 13, 14, 15 and 17, so anything written after them has to know they exist.
//!
//! `mjx_docx::document::web_settings` takes the same position on WordprocessingML's own frameset and
//! `w:div` markup, and for the same reason.

use mjx_ooxml_core::{Enumeration, Number, RawAttribute, RawName, RawNode, Text};
use mjx_ooxml_types::spreadsheetml::{SmartTagDisplay, TargetScreenSize};
use mjx_ooxml_types::support::OnOff;

use super::leaf::attribute_bag;

attribute_bag! {
    /// `x:smartTagPr` (`CT_SmartTagPr`) — whether smart-tag data is embedded in the workbook, and
    /// how a consumer indicates one.
    #[xml(attribute(local = "embed", codec = OnOff, accessor = embed_smart_tags, default = false))]
    #[xml(attribute(local = "show", codec = Enumeration<SmartTagDisplay>, accessor = smart_tag_display, default = SmartTagDisplay::All))]
    SmartTagProperties, "smartTagPr"
}

attribute_bag! {
    /// `x:smartTagType` (`CT_SmartTagType`) — one recognised smart-tag type: its namespace, its
    /// name, and where its recogniser lives.
    #[xml(attribute(local = "namespaceUri", codec = Text, accessor = namespace_uri))]
    #[xml(attribute(local = "name", codec = Text, accessor = name))]
    #[xml(attribute(local = "url", codec = Text, accessor = url))]
    SmartTagType, "smartTagType"
}

attribute_bag! {
    /// `x:webPublishing` (`CT_WebPublishing`) — how the workbook was rendered when it was saved as a
    /// web page.
    #[xml(attribute(local = "css", codec = OnOff, accessor = use_css, default = true))]
    #[xml(attribute(local = "thicket", codec = OnOff, accessor = use_thicket_folder, default = true))]
    #[xml(attribute(local = "longFileNames", codec = OnOff, accessor = use_long_file_names, default = true))]
    #[xml(attribute(local = "vml", codec = OnOff, accessor = use_vml, default = false))]
    #[xml(attribute(local = "allowPng", codec = OnOff, accessor = allow_png, default = false))]
    #[xml(attribute(local = "targetScreenSize", codec = Enumeration<TargetScreenSize>, accessor = target_screen_size, default = TargetScreenSize::Resolution800By600))]
    #[xml(attribute(local = "dpi", codec = Number<u32>, accessor = dots_per_inch, default = 96))]
    #[xml(attribute(local = "codePage", codec = Number<u32>, accessor = code_page))]
    #[xml(attribute(local = "characterSet", codec = Text, accessor = character_set))]
    WebPublishing, "webPublishing"
}

attribute_bag! {
    /// `x:webPublishObject` (`CT_WebPublishObject`) — one range or object published to a web page,
    /// and the file it was published to.
    #[xml(attribute(local = "id", codec = Number<u32>, accessor = id, required))]
    #[xml(attribute(local = "divId", codec = Text, accessor = html_div_id, required))]
    #[xml(attribute(local = "sourceObject", codec = Text, accessor = source_object))]
    #[xml(attribute(local = "destinationFile", codec = Text, accessor = destination_file, required))]
    #[xml(attribute(local = "title", codec = Text, accessor = title))]
    #[xml(attribute(local = "autoRepublish", codec = OnOff, accessor = auto_republish, default = false))]
    WebPublishObject, "webPublishObject"
}

/// `x:smartTagTypes` (`CT_SmartTagTypes`) — the recognised smart-tag types, in document order.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = SML)]
pub struct SmartTagTypes {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "smartTagType", variant = Type, ty = SmartTagType))]
    content: Vec<SmartTagTypesContent>,
}

/// One child of [`SmartTagTypes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmartTagTypesContent {
    /// `x:smartTagType`.
    Type(SmartTagType),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl SmartTagTypes {
    /// Builds an empty `x:smartTagTypes`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut mjx_ooxml_core::Interner, prefix: Option<&str>) -> Self {
        Self {
            name: super::leaf::sml_name(interner, prefix, "smartTagTypes"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every `x:smartTagType`, in document order.
    pub fn types(&self) -> impl Iterator<Item = &SmartTagType> + '_ {
        self.content.iter().filter_map(|item| match item {
            SmartTagTypesContent::Type(kind) => Some(kind),
            SmartTagTypesContent::Raw(_) => None,
        })
    }

    /// Appends a type after the ones already present.
    pub fn push(&mut self, kind: SmartTagType) {
        self.content.push(SmartTagTypesContent::Type(kind));
    }
}

/// `x:webPublishObjects` (`CT_WebPublishObjects`) — the published objects, plus the count the file
/// states.
///
/// `@count` is a producer hint like every other count in this schema: read, never recomputed. A
/// writer that derived it would change bytes in a file whose author wrote something else on purpose.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = stated_count))]
pub struct WebPublishObjects {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "webPublishObject", variant = Object, ty = WebPublishObject))]
    content: Vec<WebPublishObjectsContent>,
}

/// One child of [`WebPublishObjects`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebPublishObjectsContent {
    /// `x:webPublishObject`.
    Object(WebPublishObject),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl WebPublishObjects {
    /// Builds an empty `x:webPublishObjects`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut mjx_ooxml_core::Interner, prefix: Option<&str>) -> Self {
        Self {
            name: super::leaf::sml_name(interner, prefix, "webPublishObjects"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every `x:webPublishObject`, in document order.
    pub fn objects(&self) -> impl Iterator<Item = &WebPublishObject> + '_ {
        self.content.iter().filter_map(|item| match item {
            WebPublishObjectsContent::Object(object) => Some(object),
            WebPublishObjectsContent::Raw(_) => None,
        })
    }

    /// Appends an object after the ones already present.
    pub fn push(&mut self, object: WebPublishObject) {
        self.content.push(WebPublishObjectsContent::Object(object));
    }
}
