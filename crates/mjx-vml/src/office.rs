//! The Office drawing extensions to VML (`urn:schemas-microsoft-com:office:office`) that carry the
//! *references* — the part of legacy markup a modern consumer actually has to resolve.
//!
//! Names follow the ECMA-376 Part 4 §19.2.2 prose: `o:shapelayout` is §19.2.2.29 *shapelayout (Shape
//! Layout Properties)*, `o:idmap` is §19.2.2.14 *idmap (Shape ID Map)*, `o:OLEObject` is §19.2.2.20
//! *OLEObject (Embedded OLE Object)*, `o:ink` is §19.2.2.15 *ink (Ink)*, and `o:lock` is §19.2.2.18
//! *lock (Shape Protections)*.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawElement, RawName, RawNode};
use mjx_ooxml_types::namespaces::{SHARED_RELATIONSHIP_REFERENCE, VML_MAIN, VML_OFFICE_DRAWING};

use crate::build::{self, fidelity_leaf};

/// One ordered child of a [`ShapeLayout`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShapeLayoutContent {
    /// The shape-id map (`o:idmap`).
    ShapeIdMap(ShapeIdMap),
    /// Any other child — `o:regrouptable`, `o:rules` — kept verbatim.
    Raw(RawNode),
}

/// `o:shapelayout` (`CT_ShapeLayout`) — ECMA-376 Part 4 §19.2.2.29 *shapelayout (Shape Layout
/// Properties)*.
///
/// The header every VML drawing part opens with. Its [`ShapeIdMap`] states which blocks of legacy
/// shape ids this drawing owns.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = VML_OFFICE_DRAWING)]
pub struct ShapeLayout {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "idmap", variant = ShapeIdMap, ty = ShapeIdMap))]
    content: Vec<ShapeLayoutContent>,
}

/// `o:idmap` (`CT_IdMap`) — ECMA-376 Part 4 §19.2.2.14 *idmap (Shape ID Map)*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeIdMap {
    element: RawElement,
}
fidelity_leaf!(ShapeIdMap);

/// `o:OLEObject` (`CT_OLEObject`) — ECMA-376 Part 4 §19.2.2.20 *OLEObject (Embedded OLE Object)*.
///
/// The binding between a [`Shape`](crate::Shape) and the OLE object it displays: `ShapeID` names the
/// shape, `r:id` names the part holding the object's data, and `ProgID` names the application that
/// owns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedOleObject {
    element: RawElement,
}
fidelity_leaf!(EmbeddedOleObject);

/// `o:ink` (`CT_Ink`) — ECMA-376 Part 4 §19.2.2.15 *ink (Ink)*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ink {
    element: RawElement,
}
fidelity_leaf!(Ink);

/// `o:lock` (`CT_Lock`) — ECMA-376 Part 4 §19.2.2.18 *lock (Shape Protections)*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeProtections {
    element: RawElement,
}
fidelity_leaf!(ShapeProtections);

/// Whether an [`EmbeddedOleObject`] holds the object's data or points at it — `o:OLEObject@Type`
/// (`ST_OLEType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OleObjectKind {
    /// The object's data is embedded in this package. Wire value `Embed`.
    Embedded,
    /// The object's data is linked from outside it. Wire value `Link`.
    Linked,
}

impl OleObjectKind {
    /// The kind a wire value names, or `None` for one `ST_OLEType` does not define.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "Embed" => Some(Self::Embedded),
            "Link" => Some(Self::Linked),
            _ => None,
        }
    }

    /// The exact wire value for this kind.
    #[must_use]
    pub fn to_wire(self) -> &'static str {
        match self {
            Self::Embedded => "Embed",
            Self::Linked => "Link",
        }
    }
}

/// How an embedded object is represented visually — `o:OLEObject@DrawAspect`
/// (`ST_OLEDrawAspect`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OleDrawAspect {
    /// Drawn as the object's content. Wire value `Content`.
    Content,
    /// Drawn as an icon standing for the object. Wire value `Icon`.
    Icon,
}

impl OleDrawAspect {
    /// The aspect a wire value names, or `None` for one `ST_OLEDrawAspect` does not define.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "Content" => Some(Self::Content),
            "Icon" => Some(Self::Icon),
            _ => None,
        }
    }

    /// The exact wire value for this aspect.
    #[must_use]
    pub fn to_wire(self) -> &'static str {
        match self {
            Self::Content => "Content",
            Self::Icon => "Icon",
        }
    }
}

/// When a linked object refreshes from its source — `o:OLEObject@UpdateMode`
/// (`ST_OLEUpdateMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OleUpdateMode {
    /// Refreshed whenever the document is opened. Wire value `Always`.
    Always,
    /// Refreshed only when the consumer asks. Wire value `OnCall`.
    OnCall,
}

impl OleUpdateMode {
    /// The mode a wire value names, or `None` for one `ST_OLEUpdateMode` does not define.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "Always" => Some(Self::Always),
            "OnCall" => Some(Self::OnCall),
            _ => None,
        }
    }

    /// The exact wire value for this mode.
    #[must_use]
    pub fn to_wire(self) -> &'static str {
        match self {
            Self::Always => "Always",
            Self::OnCall => "OnCall",
        }
    }
}

impl ShapeLayout {
    /// A fresh `o:shapelayout` whose [`ShapeIdMap`] claims the shape-id block `data`.
    ///
    /// `data` is the comma-separated list of block numbers a drawing owns (`data="1"` for the first
    /// block, which is what a producer writes for a document's first VML drawing).
    #[must_use]
    pub fn new(interner: &mut Interner, data: &str) -> Self {
        let name = build::qname(
            interner,
            build::OFFICE_PREFIX,
            VML_OFFICE_DRAWING,
            "shapelayout",
        );
        let mut attributes = Vec::with_capacity(1);
        build::set_namespaced_attribute(&mut attributes, interner, VML_MAIN, "ext", "edit");
        let id_map = ShapeIdMap::new(interner, data);
        Self {
            name,
            attributes,
            empty: false,
            content: vec![ShapeLayoutContent::ShapeIdMap(id_map)],
        }
    }

    /// The element's name as the part spells it, prefix included.
    #[must_use]
    pub fn name(&self) -> &RawName {
        &self.name
    }

    /// Every attribute the element carries, in source order.
    #[must_use]
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }

    /// The element's children, typed where this crate models them and verbatim where it does not.
    #[must_use]
    pub fn content(&self) -> &[ShapeLayoutContent] {
        &self.content
    }

    /// The shape-id map (`o:idmap`), or `None` when the layout states none.
    #[must_use]
    pub fn shape_id_map(&self) -> Option<&ShapeIdMap> {
        self.content.iter().find_map(|child| match child {
            ShapeLayoutContent::ShapeIdMap(value) => Some(value),
            _ => None,
        })
    }
}

impl ShapeIdMap {
    /// A fresh `o:idmap` claiming the shape-id blocks `data`.
    #[must_use]
    pub fn new(interner: &mut Interner, data: &str) -> Self {
        let mut element = build::element(
            interner,
            build::OFFICE_PREFIX,
            VML_OFFICE_DRAWING,
            "idmap",
            Vec::new(),
            Vec::new(),
        );
        build::set_namespaced_attribute(&mut element.attributes, interner, VML_MAIN, "ext", "edit");
        build::set_attribute(&mut element.attributes, interner, "data", data);
        Self { element }
    }

    /// The shape-id blocks this drawing owns (`o:idmap@data`) — a comma-separated list of block
    /// numbers.
    #[must_use]
    pub fn data(&self, interner: &Interner) -> Option<String> {
        build::attribute(&self.element.attributes, interner, "data")
            .map(std::borrow::Cow::into_owned)
    }
}

impl EmbeddedOleObject {
    /// The identifier of the [`Shape`](crate::Shape) that displays this object
    /// (`o:OLEObject@ShapeID`), or `None`.
    #[must_use]
    pub fn shape_identifier(&self, interner: &Interner) -> Option<String> {
        build::attribute(&self.element.attributes, interner, "ShapeID")
            .map(std::borrow::Cow::into_owned)
    }

    /// The program id of the application that owns the object (`o:OLEObject@ProgID`, e.g.
    /// `Excel.Sheet.12`), or `None`.
    #[must_use]
    pub fn program_id(&self, interner: &Interner) -> Option<String> {
        build::attribute(&self.element.attributes, interner, "ProgID")
            .map(std::borrow::Cow::into_owned)
    }

    /// The relationship id of the part holding the object's data (`o:OLEObject@r:id`), or `None`.
    #[must_use]
    pub fn relationship_id(&self, interner: &Interner) -> Option<String> {
        build::namespaced_attribute(
            &self.element.attributes,
            interner,
            SHARED_RELATIONSHIP_REFERENCE,
            "id",
        )
        .map(std::borrow::Cow::into_owned)
    }

    /// The unique id of the embedded object (`o:OLEObject@ObjectID`), or `None`.
    #[must_use]
    pub fn object_id(&self, interner: &Interner) -> Option<String> {
        build::attribute(&self.element.attributes, interner, "ObjectID")
            .map(std::borrow::Cow::into_owned)
    }

    /// Whether the object's data is embedded or linked (`o:OLEObject@Type`), or `None` when the
    /// attribute is absent or names a value `ST_OLEType` does not define.
    #[must_use]
    pub fn kind(&self, interner: &Interner) -> Option<OleObjectKind> {
        build::attribute(&self.element.attributes, interner, "Type")
            .and_then(|value| OleObjectKind::from_wire(&value))
    }

    /// How the object is represented visually (`o:OLEObject@DrawAspect`), or `None`.
    #[must_use]
    pub fn draw_aspect(&self, interner: &Interner) -> Option<OleDrawAspect> {
        build::attribute(&self.element.attributes, interner, "DrawAspect")
            .and_then(|value| OleDrawAspect::from_wire(&value))
    }

    /// When a linked object refreshes (`o:OLEObject@UpdateMode`), or `None`.
    #[must_use]
    pub fn update_mode(&self, interner: &Interner) -> Option<OleUpdateMode> {
        build::attribute(&self.element.attributes, interner, "UpdateMode")
            .and_then(|value| OleUpdateMode::from_wire(&value))
    }

    /// A fresh `o:OLEObject` binding the shape `shape_identifier` to the object data relationship
    /// `relationship_id`, owned by `program_id`.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        shape_identifier: &str,
        relationship_id: &str,
        program_id: &str,
        kind: OleObjectKind,
    ) -> Self {
        let mut element = build::element(
            interner,
            build::OFFICE_PREFIX,
            VML_OFFICE_DRAWING,
            "OLEObject",
            Vec::new(),
            Vec::new(),
        );
        build::set_attribute(&mut element.attributes, interner, "Type", kind.to_wire());
        build::set_attribute(&mut element.attributes, interner, "ProgID", program_id);
        build::set_attribute(
            &mut element.attributes,
            interner,
            "ShapeID",
            shape_identifier,
        );
        build::set_attribute(
            &mut element.attributes,
            interner,
            "DrawAspect",
            OleDrawAspect::Content.to_wire(),
        );
        build::set_namespaced_attribute(
            &mut element.attributes,
            interner,
            SHARED_RELATIONSHIP_REFERENCE,
            "id",
            relationship_id,
        );
        Self { element }
    }
}

impl Ink {
    /// The ink data the shape carries (`o:ink@i`) — base64-encoded, exactly as the part holds it,
    /// never decoded.
    #[must_use]
    pub fn data(&self, interner: &Interner) -> Option<String> {
        build::attribute(&self.element.attributes, interner, "i").map(std::borrow::Cow::into_owned)
    }

    /// Whether the ink is an annotation over other content rather than ink in its own right
    /// (`o:ink@annotation`), or `None` when unstated.
    #[must_use]
    pub fn is_annotation(&self, interner: &Interner) -> Option<bool> {
        build::attribute(&self.element.attributes, interner, "annotation")
            .and_then(|value| crate::true_false(&value))
    }

    /// The content type of the ink data (`o:ink@contentType`), or `None`.
    #[must_use]
    pub fn content_type(&self, interner: &Interner) -> Option<String> {
        build::attribute(&self.element.attributes, interner, "contentType")
            .map(std::borrow::Cow::into_owned)
    }
}
