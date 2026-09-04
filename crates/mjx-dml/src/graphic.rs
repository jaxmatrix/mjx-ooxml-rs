//! `a:graphic`/`a:graphicData` (`CT_GraphicalObject`/`CT_GraphicalObjectData`, `dml-main.xsd`) — the
//! generic envelope every "frame me something" DrawingML host places around one payload, told apart
//! by `a:graphicData@uri`: a picture, a chart, a SmartArt diagram, a table, an OLE object, or (new in
//! this child) a Word shape/group/canvas/graphic-frame.
//!
//! # What this type dispatches, and what it does not
//!
//! [`GraphicData`] types **only the payload kinds `mjx-dml` fully owns without reaching upward**:
//! [`crate::picture::Picture`] (`pic:pic`). Everything else — a chart's `c:chart`, a diagram's
//! `dgm:relIds`, a table's `a:tbl`, an OLE object's MCE-wrapped `p:oleObj`/`w:*`-shaped fallback, and
//! (deliberately, see below) a Word shape/group/canvas/graphic-frame — stays
//! [`GraphicDataContent::Other`], preserved byte-for-byte via its raw children.
//!
//! **Why the Word shape kinds are not dispatched here even though this child models them**:
//! `wp:wsp`'s own optional text box (`CT_TextboxInfo` → `CT_TxbxContent`) is `EG_BlockLevelElts` —
//! WordprocessingML paragraph/table content — which only a WML-consuming crate can type without this
//! crate reaching upward past `mjx-dml`'s own tier. So [`crate::wordprocessing_drawing`] models the
//! *placement* wrapper (`wp:inline`/`wp:anchor`, wrap modes, `wp:graphicFrame`, `wp:wgp`/`wp:wpc`)
//! here, but the shape itself lives in `mjx-docx`, and `mjx-docx` reads a drawing's `a:graphic`
//! through this generic type, then — when its own `a:graphicData@uri` names a Word shape — parses
//! `GraphicDataContent::Other`'s raw payload into its own `WordprocessingShape` on demand. See that
//! crate's own module doc for the full argument.

use mjx_ooxml_core::{
    FromXml, FromXmlError, Interner, RawAttribute, RawElement, RawName, RawNode, Text, ToXml,
};

use crate::build::{dml_child, dml_name};
use crate::picture::Picture;

/// The `a:graphicData@uri` a picture's graphic frame declares (`http://schemas.openxmlformats.org/
/// drawingml/2006/picture`) — the same URI as [`mjx_ooxml_types::namespaces::DML_PICTURE`]'s own
/// transitional namespace, which is what makes "the uri names a schema's target namespace" true for
/// every kind this workspace has met so far.
pub const PICTURE_GRAPHIC_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/picture";

/// One `a:graphicData` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphicDataContent {
    /// `pic:pic` — a picture. The only payload kind this crate fully types; see this module's own
    /// doc comment for why every other kind (including the two other Word shape kinds this child
    /// models the *placement* of) is [`GraphicDataContent::Other`] instead.
    Picture(Box<Picture>),
    /// Any other payload — a chart, a diagram, a table, an OLE object, a Word shape/group/canvas/
    /// graphic frame — preserved verbatim as its own raw children.
    Other(Vec<RawNode>),
}

/// `a:graphicData` (`CT_GraphicalObjectData`) — one required `@uri` naming the payload's schema, plus
/// the payload itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphicData {
    attributes: Vec<RawAttribute>,
    content: GraphicDataContent,
}

impl GraphicData {
    /// Builds `<a:graphicData uri="{PICTURE_GRAPHIC_URI}">{picture}</a:graphicData>`.
    #[must_use]
    pub fn for_picture(picture: Picture) -> Self {
        Self {
            attributes: Vec::new(),
            content: GraphicDataContent::Picture(Box::new(picture)),
        }
    }

    /// The payload's own schema (`@uri`), or `None` if malformed. Compare against
    /// [`PICTURE_GRAPHIC_URI`] or a host crate's own graphic-data URI constants.
    #[must_use]
    pub fn uri(&self, interner: &Interner) -> Option<String> {
        GraphicDataAttributes {
            attributes: &self.attributes,
        }
        .uri(interner)
        .ok()
        .map(std::borrow::Cow::into_owned)
    }

    /// The typed payload, when this graphic data is a picture.
    #[must_use]
    pub fn picture(&self) -> Option<&Picture> {
        match &self.content {
            GraphicDataContent::Picture(picture) => Some(picture),
            GraphicDataContent::Other(_) => None,
        }
    }

    /// The payload's own content, typed or opaque.
    #[must_use]
    pub fn content(&self) -> &GraphicDataContent {
        &self.content
    }

    /// The raw, unparsed children of this graphic data — always available, even for a
    /// [`GraphicDataContent::Picture`], so a caller that wants the exact wire bytes of *any* payload
    /// kind (to hand a chart's `c:chart` to `mjx-chart` once that becomes reachable, say) never has to
    /// re-derive them from a typed value.
    #[must_use]
    pub fn raw_content(&self, interner: &mut Interner) -> Vec<RawNode> {
        match &self.content {
            GraphicDataContent::Picture(picture) => {
                vec![RawNode::Element(picture.to_xml(interner))]
            }
            GraphicDataContent::Other(nodes) => nodes.clone(),
        }
    }
}

#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "uri", codec = Text, accessor = uri, required))]
struct GraphicDataAttributes<A> {
    attributes: A,
}

impl FromXml for GraphicData {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let attributes = GraphicDataAttributes {
            attributes: &element.attributes,
        };
        let uri = attributes.uri(interner)?.into_owned();
        let content = if uri == PICTURE_GRAPHIC_URI {
            dml_child(&element.children, interner, "pic")
                .and_then(|pic_element| Picture::from_xml(pic_element, interner).ok())
                .map(Box::new)
                .map(GraphicDataContent::Picture)
        } else {
            None
        }
        .unwrap_or_else(|| GraphicDataContent::Other(element.children.clone()));
        Ok(Self {
            attributes: element.attributes.clone(),
            content,
        })
    }
}

impl ToXml for GraphicData {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let children = self.raw_content(interner);
        RawElement::rebuilt(
            dml_name(interner, "graphicData"),
            self.attributes.clone(),
            children,
            false,
        )
    }
}

/// `a:graphic` (`CT_GraphicalObject`) — one required `a:graphicData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Graphic {
    name: RawName,
    attributes: Vec<RawAttribute>,
    data: GraphicData,
}

impl Graphic {
    /// Builds `<a:graphic>{data}</a:graphic>`.
    #[must_use]
    pub fn new(interner: &mut Interner, data: GraphicData) -> Self {
        Self {
            name: dml_name(interner, "graphic"),
            attributes: Vec::new(),
            data,
        }
    }

    /// The graphic's one payload envelope (`a:graphicData`).
    #[must_use]
    pub fn data(&self) -> &GraphicData {
        &self.data
    }

    /// The graphic's one payload envelope (`a:graphicData`), mutably.
    pub fn data_mut(&mut self) -> &mut GraphicData {
        &mut self.data
    }
}

impl FromXml for Graphic {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let data_element = dml_child(&element.children, interner, "graphicData").ok_or(
            mjx_ooxml_core::AttributeError::Missing {
                attribute: "a:graphicData",
            },
        )?;
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            data: GraphicData::from_xml(data_element, interner)?,
        })
    }
}

impl ToXml for Graphic {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let data_node = RawNode::Element(self.data.to_xml(interner));
        RawElement::rebuilt(self.name, self.attributes.clone(), vec![data_node], false)
    }
}
