//! `a:prstGeom` — a shape's preset geometry.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode, Text};
use mjx_ooxml_types::drawingml::{
    adjustment_bound_guides_of, adjustments_of, AdjustmentBound, AdjustmentSpec, PresetShapeType,
};

use super::formula::{GuideContext, GuideError, ResolvedGuides};
use super::GeometryGuideList;
use crate::build::dml_name;

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

/// A shape adjustment resolved against a **concrete shape size**: its value and its numeric domain.
///
/// [`ResolvedAdjustment`] can only report the domain the generated table holds, and half of the
/// spec's bounds are not numbers at all but the names of `gdLst` guides (`maxAdj1`, `maxAng`, …)
/// whose value depends on the shape's width and height. Give
/// [`PresetGeometry::adjustments_for_size`] a size and those bounds become numbers too.
///
/// Every field is in **native spec units**: fractions in 1000ths of a percent (`100_000` = 100%),
/// angles in 60000ths of a degree, lengths in EMU — whichever the adjustment's
/// [`axis`](AdjustmentSpec::axis) implies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundedAdjustment {
    /// The adjustment's static metadata (wire name, axis, default, domain), from the generated table.
    pub spec: &'static AdjustmentSpec,
    /// The current value — the evaluated `avLst` override if there is one, else the spec default.
    pub value: f64,
    /// Whether [`value`](Self::value) came from an explicit `avLst` override (vs. the spec default).
    pub is_overridden: bool,
    /// The domain's lower bound, evaluated.
    pub minimum: f64,
    /// The domain's upper bound, evaluated.
    pub maximum: f64,
}

impl BoundedAdjustment {
    /// [`value`](Self::value) brought inside its domain, by the rule the `pin` guide formula states
    /// (ECMA-376 Part 1 §20.1.9.11): below the minimum reads as the minimum, above the maximum reads
    /// as the maximum. Written out rather than delegated to [`f64::clamp`], which panics when a
    /// shape's bounds cross.
    #[must_use]
    pub fn pinned_value(self) -> f64 {
        if self.value < self.minimum {
            self.minimum
        } else if self.value > self.maximum {
            self.maximum
        } else {
            self.value
        }
    }
}

/// A resolved guide value as a native adjustment integer, saturating rather than wrapping at the
/// edges of the range (`ST_AdjCoordinate` and `ST_Angle` are integral on the wire).
fn round_to_native(value: f64) -> i32 {
    value.round() as i32
}

/// A domain bound as a number: a literal as itself, a guide name looked up in `guides`.
fn resolve_bound(bound: AdjustmentBound, guides: &ResolvedGuides<'_>) -> Result<f64, GuideError> {
    match bound {
        AdjustmentBound::Literal(literal) => Ok(f64::from(literal)),
        AdjustmentBound::Guide(name) => guides.resolve(name),
    }
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
///
/// `@prst` is declared as [`Text`] rather than as an `Enumeration<PresetShapeType>`, deliberately:
/// this type exposes **both** readings of the same bytes — the typed [`preset`](Self::preset) and
/// the raw [`preset_token`](Self::preset_token), which is what lets a shape kind this build does not
/// know still be named. The typed reading layers `PresetShapeType::from_wire` — the generated
/// enumeration's own wire mapping, the very one `Enumeration<T>` would call — over that one read.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_MAIN)]
#[xml(attribute(local = "prst", codec = Text, accessor = preset_token, required))]
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
        let mut geometry = Self {
            name: dml_name(interner, "prstGeom"),
            attributes: Vec::new(),
            empty,
            content,
        };
        geometry.set_preset_token(interner, preset.to_wire());
        geometry
    }

    /// The preset shape kind, or `None` if `prst` is absent or names a token this build does not know.
    #[must_use]
    pub fn preset(&self, interner: &Interner) -> Option<PresetShapeType> {
        self.preset_token(interner)
            .ok()
            .as_deref()
            .and_then(PresetShapeType::from_wire)
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
    ///
    /// The override is a guide formula, not necessarily a literal: `val 25000` and `*/ 50000 1 2`
    /// both read as `25000`. It is evaluated with **no shape size** (see
    /// [`adjust_value_overrides`](Self::adjust_value_overrides)), so an override that reaches for `w`
    /// or `h` is skipped and the spec default stands; ask
    /// [`adjustments_for_size`](Self::adjustments_for_size) when the size is known.
    #[must_use]
    pub fn adjustment(&self, interner: &Interner, wire_name: &str) -> Option<i32> {
        if let Some(value) = self.adjust_value_overrides(interner).guide(wire_name) {
            return Some(round_to_native(value));
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
        let overrides = self.adjust_value_overrides(interner);
        adjustments_of(preset)
            .iter()
            .map(|spec| {
                let overridden = overrides.guide(spec.wire_name);
                ResolvedAdjustment {
                    spec,
                    value: overridden.map_or(spec.default, round_to_native),
                    is_overridden: overridden.is_some(),
                }
            })
            .collect()
    }

    /// Every adjustment this shape exposes, resolved against a **concrete shape size** — value *and*
    /// numeric domain, with each `gdLst` guide bound (`maxAdj1`, `maxAng`, …) evaluated.
    ///
    /// The environment is built the way the format itself builds it: the shape's current adjustment
    /// values first (an `avLst` override where there is one, the spec default otherwise), then the
    /// `gdLst` guides its bounds depend on, from
    /// [`adjustment_bound_guides_of`](mjx_ooxml_types::drawingml::adjustment_bound_guides_of), in
    /// declaration order.
    ///
    /// Empty if the shape is fixed-geometry or its `prst` is unknown.
    ///
    /// # Errors
    ///
    /// [`GuideError`] if a bound guide cannot be evaluated — which, the table being the spec's own,
    /// means a degenerate size (a zero width or height divides by zero in guides such as
    /// `*/ 50000 w ss`).
    pub fn adjustments_for_size(
        &self,
        interner: &Interner,
        context: GuideContext,
    ) -> Result<Vec<BoundedAdjustment>, GuideError> {
        let Some(preset) = self.preset(interner) else {
            return Ok(Vec::new());
        };
        let specs = adjustments_of(preset);
        let overrides = self.adjust_value_overrides(interner);

        let mut environment = ResolvedGuides::new(context);
        let mut current = Vec::with_capacity(specs.len());
        for spec in specs {
            let overridden = overrides.guide(spec.wire_name);
            let value = overridden.unwrap_or_else(|| f64::from(spec.default));
            environment.define(spec.wire_name, value);
            current.push((value, overridden.is_some()));
        }
        environment.extend(
            adjustment_bound_guides_of(preset)
                .iter()
                .map(|guide| (guide.wire_name, guide.formula)),
        )?;

        specs
            .iter()
            .zip(current)
            .map(|(spec, (value, is_overridden))| {
                Ok(BoundedAdjustment {
                    spec,
                    value,
                    is_overridden,
                    minimum: resolve_bound(spec.min, &environment)?,
                    maximum: resolve_bound(spec.max, &environment)?,
                })
            })
            .collect()
    }

    /// The `avLst` overrides, evaluated in declaration order — the raw environment behind
    /// [`adjustment`](Self::adjustment) and [`adjustments`](Self::adjustments).
    ///
    /// Built [`without_size`](ResolvedGuides::without_size), because an `avLst` holds literal seeds
    /// (ECMA-376 Part 1 §20.1.9.12: a `val` formula "should only be used within the `avLst`") and a
    /// caller asking for an adjustment has not said how big the shape is. A guide that does not
    /// evaluate — a formula naming `w`, a malformed one, one missing `name` or `fmla` — is **skipped**
    /// rather than failing the read, so a broken `avLst` costs only its own override: every other
    /// adjustment, and every default, still reads.
    #[must_use]
    pub fn adjust_value_overrides<'a>(&'a self, interner: &'a Interner) -> ResolvedGuides<'a> {
        let mut resolved = ResolvedGuides::without_size();
        let Some(list) = self.adjust_values() else {
            return resolved;
        };
        for guide in list.guides() {
            let (Ok(name), Ok(formula)) = (guide.name(interner), guide.formula(interner)) else {
                continue;
            };
            if let Ok(value) = resolved.evaluate_formula(&formula) {
                resolved.define(name, value);
            }
        }
        resolved
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
        self.set_preset_token(interner, preset.to_wire());
    }
}
