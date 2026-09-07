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

/// The `a:graphicData@uri` a chart's graphic frame declares (`http://schemas.openxmlformats.org/
/// drawingml/2006/chart`) — again the same URI as the schema's own transitional namespace,
/// [`mjx_ooxml_types::namespaces::DML_CHART`].
///
/// A chart is the one payload kind whose *envelope* this crate can build even though it cannot type
/// the payload: `c:chart` is a self-closing leaf carrying nothing but `r:id`, so building one needs
/// the chart namespace and the relationships namespace and no chart markup at all. Both host crates
/// place a chart this way — `mjx-pptx` inside a `p:graphicFrame`, `mjx-docx` inside a
/// `wp:inline`/`wp:anchor` — so the URI and the builder live here rather than once per format.
pub const CHART_GRAPHIC_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";

/// An `xmlns:prefix="uri"` declaration, for a subtree whose root introduces a prefix the
/// surrounding part does not already bind — see [`GraphicData::for_chart`]'s own doc comment.
fn namespace_declaration(interner: &mut Interner, prefix: &str, uri: &str) -> RawAttribute {
    RawAttribute {
        name: RawName {
            prefix: Some(interner.intern("xmlns")),
            local: interner.intern(prefix),
            namespace: None,
        },
        value: uri.as_bytes().into(),
        quote: mjx_ooxml_core::QuoteStyle::Double,
    }
}

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
    pub fn for_picture(interner: &mut Interner, picture: Picture) -> Self {
        let mut attributes = GraphicDataAttributes {
            attributes: Vec::new(),
        };
        attributes.set_uri(interner, PICTURE_GRAPHIC_URI);
        Self {
            attributes: attributes.attributes,
            content: GraphicDataContent::Picture(Box::new(picture)),
        }
    }

    /// Builds `<a:graphicData uri="{CHART_GRAPHIC_URI}"><c:chart xmlns:c="…" xmlns:r="…"
    /// r:id="{relationship_id}"/></a:graphicData>` — a reference to a chart part, which is all a
    /// chart's graphic data ever holds.
    ///
    /// The `c:chart` leaf declares both prefixes on itself rather than assuming the surrounding part
    /// binds them, exactly as `mjx-pptx`'s own `build_chart_frame` does and for the same reason: a
    /// blank `word/document.xml` binds `w`/`r` and a slide binds `p`/`a`/`r`, so neither binds `c`,
    /// and a spliced subtree that assumed otherwise would emit unbound markup. Declaring `r:` a
    /// second time where the host already binds it is ordinary, valid XML — the same URI bound to
    /// the same prefix — and is what makes this one builder correct on every host.
    ///
    /// The payload stays [`GraphicDataContent::Other`]: `c:chart` is ChartML, which this crate does
    /// not model and (being rank 2.0, beneath `mjx-chart`'s 2.2) could not reach if it wanted to.
    /// What is built here is the DrawingML *envelope*, which is this crate's own vocabulary.
    #[must_use]
    pub fn for_chart(interner: &mut Interner, relationship_id: &str) -> Self {
        let chart = RawElement::rebuilt(
            RawName {
                prefix: Some(interner.intern("c")),
                local: interner.intern("chart"),
                namespace: Some(
                    interner.intern(mjx_ooxml_types::namespaces::DML_CHART.transitional),
                ),
            },
            vec![
                namespace_declaration(
                    interner,
                    "c",
                    mjx_ooxml_types::namespaces::DML_CHART.transitional,
                ),
                namespace_declaration(
                    interner,
                    "r",
                    mjx_ooxml_types::namespaces::SHARED_RELATIONSHIP_REFERENCE.transitional,
                ),
                RawAttribute {
                    name: RawName {
                        prefix: Some(interner.intern("r")),
                        local: interner.intern("id"),
                        namespace: Some(interner.intern(
                            mjx_ooxml_types::namespaces::SHARED_RELATIONSHIP_REFERENCE.transitional,
                        )),
                    },
                    value: relationship_id.as_bytes().into(),
                    quote: mjx_ooxml_core::QuoteStyle::Double,
                },
            ],
            Vec::new(),
            true,
        );
        let mut attributes = GraphicDataAttributes {
            attributes: Vec::new(),
        };
        attributes.set_uri(interner, CHART_GRAPHIC_URI);
        Self {
            attributes: attributes.attributes,
            content: GraphicDataContent::Other(vec![RawNode::Element(chart)]),
        }
    }

    /// The relationship id of the chart part this graphic data references (`c:chart@r:id`), or
    /// `None` when the payload is not a chart at all.
    ///
    /// This is the read counterpart of [`for_chart`](Self::for_chart), and it is deliberately
    /// keyed on [`CHART_GRAPHIC_URI`] first: a `pic:pic` payload also carries an `r:embed`, and
    /// answering that for "which chart does this drawing show" would be a silent mis-read rather
    /// than a `None`.
    ///
    /// The `r:id` is matched by **local name alone**, exactly as `mjx-pptx`'s own `slide::chart_rel_id`
    /// does, and for a mechanical reason rather than a stylistic one: the fidelity reader resolves
    /// namespaces for *elements* and leaves an attribute's own `namespace` unpopulated, so a
    /// namespace-keyed lookup here would match nothing at all — which is precisely the bug this
    /// method was written with and which `chart_in_word.docx` caught. Local name alone is safe here
    /// because `c:chart` is `CT_RelId`, whose whole content is that one attribute: there is nothing
    /// else an `id` on this element could be. It also reads a producer that binds the relationships
    /// namespace to some prefix other than `r:` correctly, which a prefix-keyed lookup would not.
    #[must_use]
    pub fn chart_relationship_id(&self, interner: &Interner) -> Option<String> {
        if self.uri(interner).as_deref() != Some(CHART_GRAPHIC_URI) {
            return None;
        }
        let GraphicDataContent::Other(nodes) = &self.content else {
            return None;
        };
        nodes.iter().find_map(|node| {
            let RawNode::Element(element) = node else {
                return None;
            };
            if interner.resolve(element.name.local) != "chart" {
                return None;
            }
            element.attributes.iter().find_map(|attribute| {
                (interner.resolve(attribute.name.local) == "id")
                    .then(|| String::from_utf8_lossy(&attribute.value).into_owned())
            })
        })
    }

    /// The payload's own schema (`@uri`), or `None` if malformed. Compare against
    /// [`PICTURE_GRAPHIC_URI`], [`CHART_GRAPHIC_URI`] or a host crate's own graphic-data URI
    /// constants.
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
        // `pic:pic` is declared inside `dml-picture.xsd` itself, so it is `pic:`-namespaced, not
        // `a:` (DML-main) — matched by local name alone here, the same way `graphic_child` finds
        // `a:graphic` regardless of namespace, since the `uri` check just above already establishes
        // which schema this payload is.
        let content = if uri == PICTURE_GRAPHIC_URI {
            element
                .children
                .iter()
                .find_map(|node| match node {
                    RawNode::Element(child) if interner.resolve(child.name.local) == "pic" => {
                        Some(child)
                    }
                    _ => None,
                })
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
