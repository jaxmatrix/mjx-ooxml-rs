//! `w:drawing` (`CT_Drawing`), `w:object` (`CT_Object`) and `w:control` (`CT_Control`) — the
//! run-level content MJXOFF-92 (C2) left as [`super::body::Unmodeled`] placeholders naming this
//! child as their owner. `w:pict` (`CT_Picture`) needs no type of its own here at all: it is read
//! and written directly as [`mjx_vml::Drawing`], exactly as MJXOFF-113 already does for a header's
//! own `w:pict` (`headers.rs`'s own doc comment) — `Drawing::from_xml`/`to_xml` do not check the
//! element's own name, so the same type serves a VML drawing part's `<xml>` root and a Word run's
//! `w:pict` alike, and its own `Raw` bucket already preserves `w:pict`'s trailing `movie`/`control`
//! children (`CT_Rel`/`CT_Control`, neither VML) byte-for-byte even though this crate's own `Control`
//! below is what a caller reaches for the *typed* reading of one inside `w:object` instead.
//!
//! # `CT_WordprocessingShape`/`CT_TextboxInfo`/`CT_TxbxContent` live here, not in `mjx-dml`
//!
//! All three are declared in `dml-wordprocessingDrawing.xsd` alongside the seventeen types
//! [`mjx_dml::wordprocessing_drawing`] models, but `CT_TxbxContent`'s content is `w:EG_BlockLevelElts`
//! — paragraphs and tables, `mjx-docx`'s own vocabulary — so a shape or its text box cannot be typed
//! below `mjx-docx` without `mjx-dml` reaching upward past its own tier. [`WordprocessingShape`]
//! reuses `mjx-dml` for everything that *is* pure DrawingML (its `spPr`, `mjx_dml::ShapeProperties`),
//! and [`TextBoxContent`] reuses `body.rs`'s own `BlockContent`/`block_paragraph`* mechanism — the
//! sixth container that mechanism serves, per MJXOFF-126's "extend, don't copy" instruction — rather
//! than inventing a second paragraph model. See `mjx_dml::wordprocessing_drawing`'s own module doc
//! for the layering argument in full; this module's doc comment is its `mjx-docx`-side mirror.

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, Interner, RawAttribute, RawElement, RawName, RawNode, Text,
    ToXml,
};
use mjx_ooxml_types::namespaces::DML_WORDPROCESSING_DRAWING;
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::wordprocessingml::{ObjectDrawAspect, ObjectUpdateMode};

use super::body::{wml_name, BlockContent, RelationshipReference, Unmodeled};
use super::run_properties::Twips;

/// Builds a `wp:local` qualified name (the `dml-wordprocessingDrawing` namespace).
fn wp_name(interner: &mut Interner, local: &str) -> RawName {
    RawName {
        prefix: Some(interner.intern("wp")),
        local: interner.intern(local),
        namespace: Some(interner.intern(DML_WORDPROCESSING_DRAWING.transitional)),
    }
}

/// Whether `name` is in the `wp:` namespace, matching both its Strict and Transitional URIs.
fn is_wp(name: &RawName, interner: &Interner) -> bool {
    let namespace = name.namespace.map(|symbol| interner.resolve(symbol));
    namespace == Some(DML_WORDPROCESSING_DRAWING.transitional)
        || namespace == DML_WORDPROCESSING_DRAWING.strict
}

/// The first `wp:`-namespaced element in `children` named `local`.
fn wp_child<'a>(
    children: &'a [RawNode],
    interner: &Interner,
    local: &str,
) -> Option<&'a RawElement> {
    children.iter().find_map(|node| match node {
        RawNode::Element(child)
            if is_wp(&child.name, interner) && interner.resolve(child.name.local) == local =>
        {
            Some(child)
        }
        _ => None,
    })
}

// =================================================================================================
// CT_Drawing (w:drawing) — wp:inline | wp:anchor, one or more.
// =================================================================================================

/// One ordered child of a [`Drawing`]: `wp:inline` or `wp:anchor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawingContent {
    /// `wp:inline` (`CT_Inline`) — flows with the surrounding text like a large character.
    Inline(mjx_dml::wordprocessing_drawing::Inline),
    /// `wp:anchor` (`CT_Anchor`) — floats at an explicit position, with a wrap mode.
    Anchored(mjx_dml::wordprocessing_drawing::Anchor),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `w:drawing` (`CT_Drawing`, "DrawingML Object", §17.3.3.9) — one or more placements
/// (`EG_WrapType`'s own choice of `wp:inline`/`wp:anchor`, `xsd:choice minOccurs="1"
/// maxOccurs="unbounded"`), though every fixture and every writer this crate has met carries exactly
/// one.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Drawing {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(ns = DML_WORDPROCESSING_DRAWING, local = "inline", variant = Inline, ty = mjx_dml::wordprocessing_drawing::Inline),
        child(ns = DML_WORDPROCESSING_DRAWING, local = "anchor", variant = Anchored, ty = mjx_dml::wordprocessing_drawing::Anchor)
    )]
    content: Vec<DrawingContent>,
}

impl Drawing {
    /// Builds `<w:drawing>{placement}</w:drawing>` — a single-placement drawing, which is what
    /// every writer in this crate produces (the schema's `maxOccurs="unbounded"` is read-side
    /// leniency for a file this crate did not write).
    #[must_use]
    pub fn new(interner: &mut Interner, placement: DrawingContent) -> Self {
        Self {
            name: wml_name(interner, "drawing"),
            attributes: Vec::new(),
            empty: false,
            content: vec![placement],
        }
    }

    /// This drawing's own placements, in order (almost always exactly one).
    #[must_use]
    pub fn content(&self) -> &[DrawingContent] {
        &self.content
    }

    /// [`Drawing::content`], mutably.
    pub fn content_mut(&mut self) -> &mut Vec<DrawingContent> {
        &mut self.content
    }

    /// The first inline placement, if this drawing carries one.
    #[must_use]
    pub fn inline(&self) -> Option<&mjx_dml::wordprocessing_drawing::Inline> {
        self.content.iter().find_map(|item| match item {
            DrawingContent::Inline(inline) => Some(inline),
            _ => None,
        })
    }

    /// The first anchored (floating) placement, if this drawing carries one.
    #[must_use]
    pub fn anchor(&self) -> Option<&mjx_dml::wordprocessing_drawing::Anchor> {
        self.content.iter().find_map(|item| match item {
            DrawingContent::Anchored(anchor) => Some(anchor),
            _ => None,
        })
    }
}

// =================================================================================================
// CT_WordprocessingShape (wp:wsp) — cNvPr?, (cNvSpPr|cNvCnPr), spPr, style?, extLst?,
// (txbx|linkedTxbx)?, bodyPr.
// =================================================================================================

/// One ordered child of a [`WordprocessingShape`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordprocessingShapeContent {
    /// `wp:cNvPr` (`a:CT_NonVisualDrawingProps`) — the shape's identity.
    DrawingProperties(mjx_dml::NonVisualDrawingProps),
    /// `wp:cNvSpPr` (`a:CT_NonVisualDrawingShapeProps`) — a plain shape's own lock list.
    ShapeProperties(mjx_dml::NonVisualDrawingShapeProperties),
    /// `wp:cNvCnPr` (`a:CT_NonVisualConnectorProperties`) — a connector's own lock list, the
    /// alternative to [`Self::ShapeProperties`].
    ConnectorProperties(mjx_dml::NonVisualConnectorProperties),
    /// `wp:spPr` (`a:CT_ShapeProperties`) — the shape's visual properties; reuses `mjx-dml`'s own
    /// type directly, exactly the reuse this ticket asks for.
    Properties(mjx_dml::ShapeProperties),
    /// `wp:style` (`a:CT_ShapeStyle`) — unmodeled; preserved verbatim.
    Style(Unmodeled),
    /// `wp:extLst` — unmodeled; preserved verbatim.
    Extensions(Unmodeled),
    /// `wp:txbx` (`CT_TextboxInfo`) — the shape's own text box content, the alternative to
    /// [`Self::LinkedTextBox`].
    TextBox(TextboxInfo),
    /// `wp:linkedTxbx` (`mjx_dml::wordprocessing_drawing::LinkedTextboxInformation`) — a pointer to
    /// another shape's own text-box chain, the alternative to [`Self::TextBox`].
    LinkedTextBox(mjx_dml::wordprocessing_drawing::LinkedTextboxInformation),
    /// `wp:bodyPr` (`a:CT_TextBodyProperties`) — unmodeled (nothing in this workspace types
    /// `a:bodyPr` yet, [`mjx_dml::text::TextBody`]'s own doc comment says the same for its copy of
    /// this element); preserved verbatim.
    BodyProperties(Unmodeled),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `wp:wsp` (`CT_WordprocessingShape`) — a Word drawing shape: identity, shape-or-connector lock
/// list, visual properties (reusing `mjx-dml`), an optional style reference, an optional text box
/// (own or linked), then body properties.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "normalEastAsianFlow", codec = OnOff, accessor = normal_east_asian_flow))]
pub struct WordprocessingShape {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl WordprocessingShape {
    /// The shape's own visual properties (`wp:spPr`) — `None` only when the element is malformed
    /// (the schema requires it).
    #[must_use]
    pub fn shape_properties(&self, interner: &Interner) -> Option<mjx_dml::ShapeProperties> {
        wp_child(&self.children, interner, "spPr")
            .and_then(|el| mjx_dml::ShapeProperties::from_xml(el, interner).ok())
    }

    /// The shape's own text box content (`wp:txbx/wp:txbxContent`), if it carries an own text box
    /// rather than a linked one (or none at all).
    #[must_use]
    pub fn text_box(&self, interner: &Interner) -> Option<TextboxInfo> {
        wp_child(&self.children, interner, "txbx")
            .and_then(|el| TextboxInfo::from_xml(el, interner).ok())
    }

    /// The shape's linked text-box pointer (`wp:linkedTxbx`), if it names one instead of carrying
    /// its own content.
    #[must_use]
    pub fn linked_text_box(
        &self,
        interner: &Interner,
    ) -> Option<mjx_dml::wordprocessing_drawing::LinkedTextboxInformation> {
        wp_child(&self.children, interner, "linkedTxbx").and_then(|el| {
            mjx_dml::wordprocessing_drawing::LinkedTextboxInformation::from_xml(el, interner).ok()
        })
    }
}

impl FromXml for WordprocessingShape {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            children: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for WordprocessingShape {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        RawElement::rebuilt(
            self.name,
            self.attributes.clone(),
            self.children.clone(),
            false,
        )
    }
}

// =================================================================================================
// CT_TextboxInfo (wp:txbx) and CT_TxbxContent (wp:txbxContent)
// =================================================================================================

/// `wp:txbx` (`CT_TextboxInfo`) — a shape's own text box: its content, then an optional extension
/// list; `@id` defaults to `0`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", codec = Enumeration<u16>, accessor = id))]
pub struct TextboxInfo {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl TextboxInfo {
    /// The text box's own content (`wp:txbxContent`) — `None` only when the element is malformed
    /// (the schema requires it).
    #[must_use]
    pub fn content(&self, interner: &Interner) -> Option<TextBoxContent> {
        wp_child(&self.children, interner, "txbxContent")
            .and_then(|el| TextBoxContent::from_xml(el, interner).ok())
    }
}

impl FromXml for TextboxInfo {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            children: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for TextboxInfo {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        RawElement::rebuilt(
            self.name,
            self.attributes.clone(),
            self.children.clone(),
            false,
        )
    }
}

/// `wp:txbxContent` (`CT_TxbxContent`) — `w:EG_BlockLevelElts`, the exact block-level content model
/// [`super::body::Body`]/[`super::headers::HdrFtr`]/a table cell already carry, reusing
/// [`BlockContent`] directly — the sixth container `body.rs`'s own `block_paragraphs`/
/// `block_paragraph`/`block_paragraph_mut` mechanism serves, per MJXOFF-126's "extend, don't copy"
/// instruction, rather than a text-box-specific duplicate of MJXOFF-92's paragraph model.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = DML_WORDPROCESSING_DRAWING)]
pub struct TextBoxContent {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(ns = WML, local = "customXml", variant = CustomXml, ty = Unmodeled),
        child(ns = WML, local = "sdt", variant = StructuredDocumentTag, ty = Unmodeled),
        child(ns = WML, local = "p", variant = Paragraph, ty = super::body::Paragraph),
        child(ns = WML, local = "tbl", variant = Table, ty = super::tables::Table),
        child(ns = WML, local = "proofErr", variant = ProofingError, ty = super::body::ProofingError),
        child(ns = WML, local = "permStart", variant = PermissionRangeStart, ty = super::body::PermissionRangeStart),
        child(ns = WML, local = "permEnd", variant = PermissionRangeEnd, ty = super::body::PermissionRangeEnd),
        child(ns = WML, local = "sectPr", variant = SectionProperties, ty = super::sections::SectionProperties),
        child(ns = WML, local = "tcPr", variant = Properties, ty = super::tables::CellProperties)
    )]
    content: Vec<BlockContent>,
}

impl TextBoxContent {
    /// Builds `<wp:txbxContent><w:p/></wp:txbxContent>` — a text box is never legally empty (the
    /// schema's own group is `minOccurs="1"`), so a fresh one starts with one empty paragraph, the
    /// same reasoning [`super::headers::HdrFtr::new`] applies.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wp_name(interner, "txbxContent"),
            attributes: Vec::new(),
            empty: false,
            content: vec![BlockContent::Paragraph(super::body::Paragraph::new(
                interner,
            ))],
        }
    }

    /// This text box's own block-level content, in document order.
    #[must_use]
    pub fn content(&self) -> &[BlockContent] {
        &self.content
    }

    /// [`TextBoxContent::content`], mutably.
    pub fn content_mut(&mut self) -> &mut Vec<BlockContent> {
        &mut self.content
    }

    /// Every paragraph in this text box, in document order (top-level only — the same limit
    /// [`super::body::block_paragraphs`] documents for a table cell).
    pub fn paragraphs(&self) -> impl Iterator<Item = &super::body::Paragraph> {
        super::body::block_paragraphs(&self.content)
    }
}

// =================================================================================================
// CT_Object (w:object) — the leading VML/office wildcard (opaque), an optional w:drawing, then
// (control | objectLink | objectEmbed | movie)?.
// =================================================================================================

/// One typed piece of an [`EmbeddedObject`]'s own trailing choice/optional `w:drawing`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbeddedObjectContent {
    /// `w:drawing` (`CT_Drawing`) — a preview image, the same type `EG_RunInnerContent`'s own
    /// `w:drawing` member uses.
    Drawing(Drawing),
    /// `w:control` (`CT_Control`) — an ActiveX control, the alternative to
    /// [`Self::ObjectLink`]/[`Self::ObjectEmbed`]/[`Self::Movie`].
    Control(Control),
    /// `w:objectLink` (`CT_ObjectLink`) — a linked OLE object.
    ObjectLink(ObjectLink),
    /// `w:objectEmbed` (`CT_ObjectEmbed`) — an embedded OLE object.
    ObjectEmbed(ObjectEmbed),
    /// `w:movie` (`CT_Rel`) — a legacy movie reference; reuses [`RelationshipReference`], exactly as
    /// `w:contentPart`/`w:subDoc` already do for the same schema type.
    Movie(RelationshipReference),
    /// Any other child — the leading VML/office wildcard sequence (`urn:schemas-microsoft-com:vml`/
    /// `:office:office`, `xsd:any processContents="lax"`, unbounded) included, and whitespace or an
    /// unknown element — preserved verbatim. OLE preview shapes (`v:shape`) and their binding
    /// (`o:OLEObject`) round-trip through this bucket byte-for-byte; a caller that needs them typed
    /// reaches for [`mjx_vml::Drawing::from_xml`] directly on one of these raw nodes, the same trick
    /// this module's own doc comment describes for `w:pict`.
    Raw(RawNode),
}

/// `w:object` (`CT_Object`, "Embedded Object", §17.3.3.19) — an OLE object or legacy movie: a leading
/// VML/office wildcard sequence (preserved raw), an optional `w:drawing` preview, then at most one of
/// `w:control`/`w:objectLink`/`w:objectEmbed`/`w:movie`. `@dxaOrig`/`@dyaOrig` are the object's
/// original size **in twips** — distinct from `w:drawing`'s own EMU-valued extent, so this type keeps
/// them under their own twips-named accessors rather than conflating the two units in one field.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "dxaOrig", codec = Twips, accessor = original_width_twips))]
#[xml(attribute(local = "dyaOrig", codec = Twips, accessor = original_height_twips))]
pub struct EmbeddedObject {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "drawing", variant = Drawing, ty = Drawing),
        child(local = "control", variant = Control, ty = Control),
        child(local = "objectLink", variant = ObjectLink, ty = ObjectLink),
        child(local = "objectEmbed", variant = ObjectEmbed, ty = ObjectEmbed),
        child(local = "movie", variant = Movie, ty = RelationshipReference)
    )]
    content: Vec<EmbeddedObjectContent>,
}

impl EmbeddedObject {
    /// This object's own content, in document order (the raw VML/office wildcard sequence
    /// interleaved with whichever of `w:drawing`/`w:control`/`w:objectLink`/`w:objectEmbed`/`w:movie`
    /// it carries).
    #[must_use]
    pub fn content(&self) -> &[EmbeddedObjectContent] {
        &self.content
    }

    /// The object's own preview drawing (`w:drawing`), or `None` if it carries none.
    #[must_use]
    pub fn drawing(&self) -> Option<&Drawing> {
        self.content.iter().find_map(|item| match item {
            EmbeddedObjectContent::Drawing(drawing) => Some(drawing),
            _ => None,
        })
    }

    /// The embedded OLE object's own binding (`w:objectEmbed`), or `None` if this object is linked,
    /// a control, a movie, or carries none of the four.
    #[must_use]
    pub fn object_embed(&self) -> Option<&ObjectEmbed> {
        self.content.iter().find_map(|item| match item {
            EmbeddedObjectContent::ObjectEmbed(embed) => Some(embed),
            _ => None,
        })
    }

    /// The linked OLE object's own binding (`w:objectLink`), or `None` if this object is embedded, a
    /// control, a movie, or carries none of the four.
    #[must_use]
    pub fn object_link(&self) -> Option<&ObjectLink> {
        self.content.iter().find_map(|item| match item {
            EmbeddedObjectContent::ObjectLink(link) => Some(link),
            _ => None,
        })
    }

    /// The ActiveX control this object wraps (`w:control`), or `None` if it is an OLE object, a
    /// movie, or carries none of the four.
    #[must_use]
    pub fn control(&self) -> Option<&Control> {
        self.content.iter().find_map(|item| match item {
            EmbeddedObjectContent::Control(control) => Some(control),
            _ => None,
        })
    }
}

// =================================================================================================
// CT_ObjectEmbed (w:objectEmbed) / CT_ObjectLink (w:objectLink, extends CT_ObjectEmbed)
// =================================================================================================

/// `w:objectEmbed` (`CT_ObjectEmbed`) — an embedded OLE object's binding: its own draw aspect, the
/// relationship id resolving to the OLE payload part (preserved verbatim, never re-encoded — this
/// type only ever names the part, it does not touch its bytes), the program id, its shape id in the
/// accompanying VML preview, and any field codes it carries.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "drawAspect", codec = Enumeration<ObjectDrawAspect>, accessor = draw_aspect))]
#[xml(attribute(local = "id", prefix = "r", codec = Text, accessor = relationship_id, required))]
#[xml(attribute(local = "progId", codec = Text, accessor = raw_program_id))]
#[xml(attribute(local = "shapeId", codec = Text, accessor = raw_shape_id))]
#[xml(attribute(local = "fieldCodes", codec = Text, accessor = raw_field_codes))]
pub struct ObjectEmbed {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl ObjectEmbed {
    /// Builds `<w:objectEmbed r:id="{relationship_id}"/>` naming the OLE payload's own part.
    #[must_use]
    pub fn new(interner: &mut Interner, relationship_id: &str) -> Self {
        let mut value = Self {
            name: wml_name_local(interner, "objectEmbed"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        value.set_relationship_id(interner, relationship_id);
        value
    }

    /// The program id (`@progId`, e.g. `"Excel.Sheet.12"`), or `None` if absent/malformed.
    #[must_use]
    pub fn program_id(&self, interner: &Interner) -> Option<String> {
        self.raw_program_id(interner)
            .ok()
            .flatten()
            .map(std::borrow::Cow::into_owned)
    }
}

impl FromXml for ObjectEmbed {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            children: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ObjectEmbed {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let empty = self.empty && self.children.is_empty();
        RawElement::rebuilt(
            self.name,
            self.attributes.clone(),
            self.children.clone(),
            empty,
        )
    }
}

fn wml_name_local(interner: &mut Interner, local: &str) -> RawName {
    wml_name(interner, local)
}

/// `w:objectLink` (`CT_ObjectLink`) — a **linked** (as opposed to embedded) OLE object:
/// [`ObjectEmbed`]'s own attributes plus `@updateMode` (required) and `@lockedField`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "drawAspect", codec = Enumeration<ObjectDrawAspect>, accessor = draw_aspect))]
#[xml(attribute(local = "id", prefix = "r", codec = Text, accessor = relationship_id, required))]
#[xml(attribute(local = "progId", codec = Text, accessor = raw_program_id))]
#[xml(attribute(local = "shapeId", codec = Text, accessor = raw_shape_id))]
#[xml(attribute(local = "fieldCodes", codec = Text, accessor = raw_field_codes))]
#[xml(attribute(local = "updateMode", codec = Enumeration<ObjectUpdateMode>, accessor = update_mode, required))]
#[xml(attribute(local = "lockedField", codec = OnOff, accessor = locked_field))]
pub struct ObjectLink {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl ObjectLink {
    /// Builds `<w:objectLink r:id="{relationship_id}" updateMode="{update_mode}"/>`.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        relationship_id: &str,
        update_mode: ObjectUpdateMode,
    ) -> Self {
        let mut value = Self {
            name: wml_name_local(interner, "objectLink"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        value.set_relationship_id(interner, relationship_id);
        value.set_update_mode(interner, update_mode);
        value
    }

    /// The program id (`@progId`, e.g. `"Excel.Sheet.12"`), or `None` if absent/malformed.
    #[must_use]
    pub fn program_id(&self, interner: &Interner) -> Option<String> {
        self.raw_program_id(interner)
            .ok()
            .flatten()
            .map(std::borrow::Cow::into_owned)
    }
}

impl FromXml for ObjectLink {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            children: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ObjectLink {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let empty = self.empty && self.children.is_empty();
        RawElement::rebuilt(
            self.name,
            self.attributes.clone(),
            self.children.clone(),
            empty,
        )
    }
}

// =================================================================================================
// CT_Control (w:control) — an ActiveX control reference. No content model at all.
// =================================================================================================

/// `w:control` (`CT_Control`) — an ActiveX control: an optional display name, an optional VML shape
/// id it is bound to, and an optional relationship id resolving to the control's own persisted-state
/// part (preserved verbatim, never re-encoded — this type only ever names the part).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "name", codec = Text, accessor = raw_name))]
#[xml(attribute(local = "shapeid", codec = Text, accessor = raw_shape_id))]
#[xml(attribute(local = "id", prefix = "r", codec = Text, accessor = relationship_id))]
pub struct Control {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
}

impl Control {
    /// Builds a self-closing `<w:control/>` with no attributes stated.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name_local(interner, "control"),
            attributes: Vec::new(),
            empty: true,
        }
    }

    /// The control's own display name (`@name`), or `None` if absent/malformed.
    #[must_use]
    pub fn control_name(&self, interner: &Interner) -> Option<String> {
        self.raw_name(interner)
            .ok()
            .flatten()
            .map(std::borrow::Cow::into_owned)
    }
}

impl FromXml for Control {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Control {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        RawElement::rebuilt(self.name, self.attributes.clone(), Vec::new(), true)
    }
}
