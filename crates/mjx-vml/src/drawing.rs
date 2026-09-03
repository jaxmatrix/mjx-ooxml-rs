//! The VML drawing part itself — the `<xml>` root a `vmlDrawingN.vml` part carries, and the whole
//! part with the interner its names live in.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{
    FromXml as _, Interner, RawAttribute, RawDocument, RawElement, RawName, RawNode, ToXml as _,
};
use mjx_ooxml_types::namespaces::{
    VML_MAIN, VML_OFFICE_DRAWING, VML_PRESENTATION_DRAWING, VML_SPREADSHEET_DRAWING,
};

use crate::build;
use crate::error::VmlError;
use crate::office::{EmbeddedOleObject, ShapeLayout};
use crate::shape::{Shape, ShapeGroup, ShapeGroupContent, ShapeTemplate};

/// One ordered child of a [`Drawing`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DrawingContent {
    /// The drawing's layout header (`o:shapelayout`).
    ShapeLayout(ShapeLayout),
    /// A reusable geometry (`v:shapetype`).
    ShapeTemplate(ShapeTemplate),
    /// A shape (`v:shape`).
    Shape(Shape),
    /// A group of shapes (`v:group`).
    Group(ShapeGroup),
    /// The binding to an embedded OLE object (`o:OLEObject`).
    EmbeddedOleObject(EmbeddedOleObject),
    /// Any other child — `v:background`, `o:shapedefaults`, the primitive shapes, an unknown
    /// extension — kept verbatim.
    Raw(RawNode),
}

/// A VML drawing: the `<xml>` root of a `vmlDrawingN.vml` part, or any element that holds VML shapes
/// (a WordprocessingML `w:pict`, an `mc:Fallback` branch).
///
/// [`FromXml`](mjx_ooxml_core::FromXml) does not check the element's own name, so the same type reads
/// a whole drawing part and a `w:pict` inside a Word body. Everything it does not model rides through
/// the `Raw` bucket byte-for-byte.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = VML_MAIN)]
pub struct Drawing {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "shape", variant = Shape, ty = Shape),
        child(local = "shapetype", variant = ShapeTemplate, ty = ShapeTemplate),
        child(local = "group", variant = Group, ty = ShapeGroup),
        child(ns = VML_OFFICE_DRAWING, local = "shapelayout", variant = ShapeLayout, ty = ShapeLayout),
        child(
            ns = VML_OFFICE_DRAWING,
            local = "OLEObject",
            variant = EmbeddedOleObject,
            ty = EmbeddedOleObject
        )
    )]
    content: Vec<DrawingContent>,
}

impl Drawing {
    /// A fresh, empty `<xml>` drawing root declaring the three namespaces a PowerPoint or Word VML
    /// part uses (`v`, `o`, and the host application's own).
    ///
    /// The root element of a VML drawing part is the unnamespaced `<xml>` — not a VML element — which
    /// is why the name carries no prefix and no namespace.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        let name = RawName {
            prefix: None,
            local: interner.intern("xml"),
            namespace: None,
        };
        let attributes = vec![
            build::namespace_declaration(interner, build::VML_PREFIX, VML_MAIN.transitional),
            build::namespace_declaration(
                interner,
                build::OFFICE_PREFIX,
                VML_OFFICE_DRAWING.transitional,
            ),
            build::namespace_declaration(
                interner,
                build::POWERPOINT_PREFIX,
                VML_PRESENTATION_DRAWING.transitional,
            ),
            build::namespace_declaration(
                interner,
                build::EXCEL_PREFIX,
                VML_SPREADSHEET_DRAWING.transitional,
            ),
        ];
        Self {
            name,
            attributes,
            empty: false,
            content: Vec::new(),
        }
    }

    /// The element's name as the part spells it.
    #[must_use]
    pub fn name(&self) -> &RawName {
        &self.name
    }

    /// Every attribute the root carries, in source order — the namespace declarations included.
    #[must_use]
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }

    /// The drawing's children, typed where this crate models them and verbatim where it does not.
    #[must_use]
    pub fn content(&self) -> &[DrawingContent] {
        &self.content
    }

    /// The drawing's children, mutably.
    pub fn content_mut(&mut self) -> &mut Vec<DrawingContent> {
        &mut self.content
    }

    /// Appends `child` to the drawing.
    pub fn push(&mut self, child: DrawingContent) {
        self.content.push(child);
        self.empty = false;
    }

    /// The drawing's layout header (`o:shapelayout`), or `None`.
    #[must_use]
    pub fn shape_layout(&self) -> Option<&ShapeLayout> {
        self.content.iter().find_map(|child| match child {
            DrawingContent::ShapeLayout(value) => Some(value),
            _ => None,
        })
    }

    /// The drawing's top-level shapes, in document order — not descending into groups.
    pub fn shapes(&self) -> impl Iterator<Item = &Shape> {
        self.content.iter().filter_map(|child| match child {
            DrawingContent::Shape(shape) => Some(shape),
            _ => None,
        })
    }

    /// Every shape in the drawing, groups included, in document order.
    ///
    /// Groups nest, so this walks them; the ids a host document references
    /// ([`Shape::identifier`]) are unique across the whole document, not just the top level.
    #[must_use]
    pub fn all_shapes(&self) -> Vec<&Shape> {
        let mut found = Vec::new();
        for child in &self.content {
            match child {
                DrawingContent::Shape(shape) => found.push(shape),
                DrawingContent::Group(group) => collect_group_shapes(group, &mut found),
                _ => {}
            }
        }
        found
    }

    /// The drawing's shape templates (`v:shapetype`), in document order.
    pub fn shape_templates(&self) -> impl Iterator<Item = &ShapeTemplate> {
        self.content.iter().filter_map(|child| match child {
            DrawingContent::ShapeTemplate(template) => Some(template),
            _ => None,
        })
    }

    /// The OLE-object bindings the drawing carries (`o:OLEObject`), in document order.
    pub fn embedded_ole_objects(&self) -> impl Iterator<Item = &EmbeddedOleObject> {
        self.content.iter().filter_map(|child| match child {
            DrawingContent::EmbeddedOleObject(object) => Some(object),
            _ => None,
        })
    }

    /// The shape whose [`identifier`](Shape::identifier) is `identifier`, searching groups too — the
    /// resolution `p:oleObj@spid`, `p:control@spid` and `o:OLEObject@ShapeID` all need.
    #[must_use]
    pub fn shape_by_identifier(&self, interner: &Interner, identifier: &str) -> Option<&Shape> {
        self.all_shapes()
            .into_iter()
            .find(|shape| shape.identifier(interner).as_deref() == Some(identifier))
    }

    /// The shape whose identifier is `identifier`, mutably.
    pub fn shape_by_identifier_mut(
        &mut self,
        interner: &Interner,
        identifier: &str,
    ) -> Option<&mut Shape> {
        for child in &mut self.content {
            match child {
                DrawingContent::Shape(shape) => {
                    if shape.identifier(interner).as_deref() == Some(identifier) {
                        return Some(shape);
                    }
                }
                DrawingContent::Group(group) => {
                    if let Some(found) = group_shape_by_identifier_mut(group, interner, identifier)
                    {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// The shape template whose `id` is `identifier` — what a shape's
    /// [`template_identifier`](Shape::template_identifier) names.
    #[must_use]
    pub fn shape_template_by_identifier(
        &self,
        interner: &Interner,
        identifier: &str,
    ) -> Option<&ShapeTemplate> {
        self.shape_templates()
            .find(|template| template.identifier(interner).as_deref() == Some(identifier))
    }

    /// The shape the OLE-object binding `object` displays — its `ShapeID` resolved against this
    /// drawing.
    #[must_use]
    pub fn shape_for_ole_object<'a>(
        &'a self,
        interner: &Interner,
        object: &EmbeddedOleObject,
    ) -> Option<&'a Shape> {
        let identifier = object.shape_identifier(interner)?;
        self.shape_by_identifier(interner, &identifier)
    }
}

/// Appends every shape in `group`, descending into nested groups, to `found`.
fn collect_group_shapes<'a>(group: &'a ShapeGroup, found: &mut Vec<&'a Shape>) {
    for child in group.content() {
        match child {
            ShapeGroupContent::Shape(shape) => found.push(shape),
            ShapeGroupContent::Group(nested) => collect_group_shapes(nested, found),
            _ => {}
        }
    }
}

/// The shape with `identifier` inside `group`, mutably, descending into nested groups.
fn group_shape_by_identifier_mut<'a>(
    group: &'a mut ShapeGroup,
    interner: &Interner,
    identifier: &str,
) -> Option<&'a mut Shape> {
    for child in group.content_mut() {
        match child {
            ShapeGroupContent::Shape(shape) => {
                if shape.identifier(interner).as_deref() == Some(identifier) {
                    return Some(shape);
                }
            }
            ShapeGroupContent::Group(nested) => {
                if let Some(found) = group_shape_by_identifier_mut(nested, interner, identifier) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// A whole VML drawing part: the [`Drawing`] together with the [`Interner`] its names resolve
/// through, and the prologue/epilogue nodes the part carried.
///
/// A `Symbol` only means anything in the interner that produced it, so reading a part standalone and
/// writing it back has to carry both. A format crate that already owns the part's
/// [`RawDocument`](mjx_ooxml_core::RawDocument) — through `mjx_opc::Package::part_tree_mut` — should
/// use [`Drawing::from_xml`](mjx_ooxml_core::FromXml::from_xml) directly instead and leave the
/// document in place.
#[derive(Debug)]
pub struct DrawingPart {
    /// The document the drawing was parsed out of, kept whole.
    ///
    /// It carries the interner, the prologue and epilogue, the byte-order-mark flag — and the two
    /// things a byte-identical rewrite needs and nothing else can supply: the **source buffer**, and
    /// the **root element as it was parsed**, whose [source
    /// ranges](mjx_ooxml_core::RawElement::source_span) say which stretches of that buffer may be
    /// copied rather than reconstructed. [`to_bytes`](DrawingPart::to_bytes) writes the model back
    /// over that root with
    /// [`ToXml::write_back`](mjx_ooxml_core::ToXml::write_back), so everything the edit did not
    /// reach re-emits verbatim, start-tag wrapping and all.
    ///
    /// It costs the parsed tree alongside the typed model — the model already holds a full copy of
    /// the markup, so the part is two copies rather than one. That is the price of the guarantee
    /// this type documents; a caller that owns the part's `RawDocument` already (through
    /// `mjx_opc::Package::part_tree_mut`) should use [`Drawing::from_xml`] and `write_back` directly
    /// and pay nothing.
    document: RawDocument,
    drawing: Drawing,
}

/// The XML declaration a freshly authored VML drawing part opens with, matching what Office writes.
const XML_DECLARATION: &str = r#"xml version="1.0" encoding="UTF-8" standalone="yes""#;

impl DrawingPart {
    /// Parses a whole `vmlDrawingN.vml` part.
    ///
    /// # Errors
    /// Returns [`VmlError::Xml`] if the bytes are not well-formed XML, or [`VmlError::Model`] if
    /// content this crate models is malformed.
    pub fn parse(bytes: &[u8]) -> Result<Self, VmlError> {
        let document = mjx_xml::fidelity::parse(bytes)?;
        let drawing = Drawing::from_xml(&document.root, &document.interner)?;
        Ok(Self { document, drawing })
    }

    /// A fresh, empty drawing part: an XML declaration and an `<xml>` root.
    #[must_use]
    pub fn new() -> Self {
        let mut interner = Interner::new();
        let drawing = Drawing::new(&mut interner);
        // No source buffer and a placeholder root: an authored part has no original to copy from,
        // so every element serializes from the model, which is exactly right.
        let root = RawElement::new(*drawing.name(), Vec::new(), Vec::new(), true);
        let document = RawDocument::new(
            interner,
            false,
            vec![
                RawNode::Declaration(XML_DECLARATION.as_bytes().into()),
                RawNode::Text(b"\n".as_slice().into()),
            ],
            root,
            Vec::new(),
        );
        Self { document, drawing }
    }

    /// The drawing this part holds.
    #[must_use]
    pub fn drawing(&self) -> &Drawing {
        &self.drawing
    }

    /// The drawing this part holds, mutably.
    pub fn drawing_mut(&mut self) -> &mut Drawing {
        &mut self.drawing
    }

    /// The interner every name in the drawing resolves through.
    #[must_use]
    pub fn interner(&self) -> &Interner {
        &self.document.interner
    }

    /// The interner, mutably — needed to build new elements ([`Shape::new`] and friends all take one).
    pub fn interner_mut(&mut self) -> &mut Interner {
        &mut self.document.interner
    }

    /// The drawing and its interner together, so a caller can build a shape and push it in one
    /// borrow.
    pub fn drawing_and_interner(&mut self) -> (&mut Drawing, &mut Interner) {
        (&mut self.drawing, &mut self.document.interner)
    }

    /// Serializes the part back to bytes.
    ///
    /// A part parsed and re-emitted without an edit is **byte-identical** — including whitespace
    /// only the source bytes remember, such as a start tag Office wrapped across several lines. An
    /// edit reconstructs the path from the root down to what it changed and nothing else: every
    /// other element is copied straight out of the buffer the part was parsed from.
    pub fn to_bytes(&mut self) -> Vec<u8> {
        // Split the borrow: the model reads and interns, the document receives.
        let Self { document, drawing } = self;
        let rebuilt = drawing.to_xml(&mut document.interner);
        document.root.replace_preserving_verbatim_source(rebuilt);
        mjx_xml::fidelity::serialize_to_vec(document)
    }
}

impl Default for DrawingPart {
    fn default() -> Self {
        Self::new()
    }
}
