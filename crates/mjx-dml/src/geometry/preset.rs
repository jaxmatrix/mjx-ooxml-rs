//! `a:prstGeom` — a shape's preset geometry.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_xml::text::escape_attribute;

use super::{attr_str, dml_attr, dml_name, GeometryGuideList};

/// One ordered child of a [`PresetGeometry`]: the typed adjust-value list, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresetGeometryContent {
    /// The adjust-value list (`a:avLst`).
    AdjustValues(GeometryGuideList),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `a:prstGeom` — preset geometry (`CT_PresetGeometry2D`): a preset shape kind (`prst`) plus an
/// optional `a:avLst` of adjustment overrides.
///
/// The preset kind is read as a [`PresetShapeType`]; unknown/future `prst` tokens still round-trip
/// (they are preserved verbatim and readable via [`preset_token`](Self::preset_token)) even though
/// [`preset`](Self::preset) cannot name them.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct PresetGeometry {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "avLst", variant = AdjustValues, ty = GeometryGuideList))]
    content: Vec<PresetGeometryContent>,
}

impl PresetGeometry {
    /// Builds `<a:prstGeom prst="{preset}">…</a:prstGeom>`, with the given `adjust_values` (or a
    /// self-closing `<a:prstGeom prst="…"/>` when `None`).
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        preset: PresetShapeType,
        adjust_values: Option<GeometryGuideList>,
    ) -> Self {
        let empty = adjust_values.is_none();
        let content = match adjust_values {
            Some(list) => vec![PresetGeometryContent::AdjustValues(list)],
            None => Vec::new(),
        };
        Self {
            name: dml_name(interner, "prstGeom"),
            attributes: vec![dml_attr(interner, "prst", preset.to_wire())],
            empty,
            content,
        }
    }

    /// The preset shape kind, or `None` if `prst` is absent or names a token this build does not know.
    #[must_use]
    pub fn preset(&self, interner: &Interner) -> Option<PresetShapeType> {
        self.preset_token(interner)
            .and_then(PresetShapeType::from_wire)
    }

    /// The raw `prst` wire token (e.g. `roundRect`), preserving unknown/future shapes; `None` if absent.
    #[must_use]
    pub fn preset_token(&self, interner: &Interner) -> Option<&str> {
        attr_str(&self.attributes, interner, "prst")
    }

    /// The adjustment overrides (`a:avLst`), or `None` if the shape has none.
    #[must_use]
    pub fn adjust_values(&self) -> Option<&GeometryGuideList> {
        self.content.iter().find_map(|item| match item {
            PresetGeometryContent::AdjustValues(list) => Some(list),
            PresetGeometryContent::Raw(_) => None,
        })
    }

    /// The geometry's ordered content (the typed `a:avLst` interleaved with any opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[PresetGeometryContent] {
        &self.content
    }

    /// Sets the preset shape kind, rewriting the existing `prst` attribute in place (or adding one if,
    /// against the schema, it was missing).
    pub fn set_preset(&mut self, interner: &mut Interner, preset: PresetShapeType) {
        let prst = interner.intern("prst");
        let value: Box<[u8]> = escape_attribute(preset.to_wire()).as_bytes().into();
        match self
            .attributes
            .iter_mut()
            .find(|attribute| attribute.name.prefix.is_none() && attribute.name.local == prst)
        {
            Some(attribute) => attribute.value = value,
            None => self
                .attributes
                .push(dml_attr(interner, "prst", preset.to_wire())),
        }
    }
}
