//! `a:prstGeom` — a shape's preset geometry.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};
use mjx_ooxml_types::drawingml::{adjustments_of, AdjustmentSpec, PresetShapeType};

use super::GeometryGuideList;
use crate::build::{attr_str, dml_attr, dml_name};

/// A shape adjustment resolved against a concrete [`PresetGeometry`]: its static spec plus the value
/// currently in effect (see [`PresetGeometry::adjustments`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedAdjustment {
    /// The adjustment's static metadata (wire name, axis, default, domain), from the generated table.
    pub spec: &'static AdjustmentSpec,
    /// The current value in native spec units — the `avLst` override if present, else the default.
    pub value: i32,
    /// Whether [`value`](Self::value) came from an explicit `avLst` override (vs. the spec default).
    pub is_overridden: bool,
}

/// The integer of a `val N` guide formula, or `None` for any other (computed) formula.
fn parse_val_formula(formula: &str) -> Option<i32> {
    let mut parts = formula.split_whitespace();
    if parts.next()? != "val" {
        return None;
    }
    parts.next()?.parse().ok()
}

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

    /// The adjustment overrides (`a:avLst`), mutably, or `None` if the shape has none.
    pub fn adjust_values_mut(&mut self) -> Option<&mut GeometryGuideList> {
        self.content.iter_mut().find_map(|item| match item {
            PresetGeometryContent::AdjustValues(list) => Some(list),
            PresetGeometryContent::Raw(_) => None,
        })
    }

    /// The current value of the adjustment named `wire_name` (`adj`, `adj1`, …) in **native spec
    /// units**: the `avLst` override if present, else the shape's default from
    /// [`adjustments_of`](mjx_ooxml_types::drawingml::adjustments_of). `None` if the shape has no such
    /// adjustment and none is overridden.
    #[must_use]
    pub fn adjustment(&self, interner: &Interner, wire_name: &str) -> Option<i32> {
        if let Some(value) = self.overridden_adjustment(interner, wire_name) {
            return Some(value);
        }
        let preset = self.preset(interner)?;
        adjustments_of(preset)
            .iter()
            .find(|spec| spec.wire_name == wire_name)
            .map(|spec| spec.default)
    }

    /// Every adjustment this shape exposes, each resolved to its current value (override or default).
    /// Empty if the shape is fixed-geometry or its `prst` is unknown.
    #[must_use]
    pub fn adjustments(&self, interner: &Interner) -> Vec<ResolvedAdjustment> {
        let Some(preset) = self.preset(interner) else {
            return Vec::new();
        };
        adjustments_of(preset)
            .iter()
            .map(|spec| {
                let overridden = self.overridden_adjustment(interner, spec.wire_name);
                ResolvedAdjustment {
                    spec,
                    value: overridden.unwrap_or(spec.default),
                    is_overridden: overridden.is_some(),
                }
            })
            .collect()
    }

    /// The value of an `avLst` `val N` override for `wire_name`, if one is present and numeric.
    fn overridden_adjustment(&self, interner: &Interner, wire_name: &str) -> Option<i32> {
        self.adjust_values()?
            .guides()
            .find(|guide| guide.name(interner) == Some(wire_name))
            .and_then(|guide| guide.formula(interner))
            .and_then(parse_val_formula)
    }

    /// Sets the adjustment named `wire_name` to `value` (native spec units), upserting the `avLst`
    /// `gd` as `fmla="val {value}"` and **creating the `avLst`** if the shape had none.
    pub fn set_adjustment(&mut self, interner: &mut Interner, wire_name: &str, value: i32) {
        if self.adjust_values().is_none() {
            let list = GeometryGuideList::new(interner, Vec::new());
            self.content.push(PresetGeometryContent::AdjustValues(list));
        }
        if let Some(list) = self.adjust_values_mut() {
            list.set_guide_formula(interner, wire_name, &format!("val {value}"));
        }
    }

    /// The geometry's ordered content (the typed `a:avLst` interleaved with any opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[PresetGeometryContent] {
        &self.content
    }

    /// Sets the preset shape kind, rewriting the existing `prst` attribute in place (or adding one if,
    /// against the schema, it was missing).
    pub fn set_preset(&mut self, interner: &mut Interner, preset: PresetShapeType) {
        crate::build::set_attr(&mut self.attributes, interner, "prst", preset.to_wire());
    }
}
