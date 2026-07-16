//! `a:avLst` (a list of adjustment guides) and `a:gd` (one guide).

use mjx_ooxml_core::{
    FromXml, FromXmlError, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml,
};

use super::{attr_str, dml_attr, dml_name};

/// `a:gd` — one geometry guide (`CT_GeomGuide`): a `name` and a formula `fmla`.
///
/// In an `a:avLst` these are the shape's adjustment overrides (e.g. `name="adj" fmla="val 25000"`);
/// both attributes are required by the schema. The guide has no child elements, so this is an
/// attribute-only leaf: it preserves its `name`/attributes/`empty` flag (and any unexpected children,
/// for fidelity) verbatim, and exposes the two attribute values by name. Its
/// [`FromXml`]/[`ToXml`] impls are hand-written because the derive models element children, not
/// attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeometryGuide {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl GeometryGuide {
    /// Builds a guide `<a:gd name="{name}" fmla="{formula}"/>`.
    #[must_use]
    pub fn new(interner: &mut Interner, name: &str, formula: &str) -> Self {
        Self {
            name: dml_name(interner, "gd"),
            attributes: vec![
                dml_attr(interner, "name", name),
                dml_attr(interner, "fmla", formula),
            ],
            children: Vec::new(),
            empty: true,
        }
    }

    /// The guide's `name` (e.g. `adj`, `adj1`), or `None` if the attribute is absent.
    #[must_use]
    pub fn name(&self, interner: &Interner) -> Option<&str> {
        attr_str(&self.attributes, interner, "name")
    }

    /// The guide's formula `fmla` (e.g. `val 25000`, `*/ h adj 100000`), or `None` if absent.
    #[must_use]
    pub fn formula(&self, interner: &Interner) -> Option<&str> {
        attr_str(&self.attributes, interner, "fmla")
    }

    /// The guide's attributes, verbatim.
    #[must_use]
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }
}

impl FromXml for GeometryGuide {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            children: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for GeometryGuide {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.children.clone();
        // Preserve the self-closing flag, but never contradict "self-closing ⇒ no children".
        let empty = self.empty && children.is_empty();
        RawElement {
            name: self.name,
            attributes: self.attributes.clone(),
            children,
            empty,
        }
    }
}

/// One ordered child of a [`GeometryGuideList`]: a typed [`GeometryGuide`], or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeometryGuideListContent {
    /// A geometry guide (`a:gd`).
    Guide(GeometryGuide),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `a:avLst` — an adjust-value list (`CT_GeomGuideList`): zero or more `a:gd` guides.
///
/// As the `avLst` of an `a:prstGeom`, these are the shape's adjustment overrides. Only the guides are
/// typed; anything else is kept opaque so the list round-trips.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct GeometryGuideList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "gd", variant = Guide, ty = GeometryGuide))]
    content: Vec<GeometryGuideListContent>,
}

impl GeometryGuideList {
    /// Builds an `a:avLst` holding `guides` in order (self-closing `<a:avLst/>` when empty).
    #[must_use]
    pub fn new(interner: &mut Interner, guides: Vec<GeometryGuide>) -> Self {
        let empty = guides.is_empty();
        Self {
            name: dml_name(interner, "avLst"),
            attributes: Vec::new(),
            empty,
            content: guides
                .into_iter()
                .map(GeometryGuideListContent::Guide)
                .collect(),
        }
    }

    /// The typed guides (`a:gd`), in order (opaque children are skipped).
    pub fn guides(&self) -> impl Iterator<Item = &GeometryGuide> {
        self.content.iter().filter_map(|item| match item {
            GeometryGuideListContent::Guide(guide) => Some(guide),
            GeometryGuideListContent::Raw(_) => None,
        })
    }

    /// The list's ordered content (typed guides interleaved with any opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[GeometryGuideListContent] {
        &self.content
    }
}
