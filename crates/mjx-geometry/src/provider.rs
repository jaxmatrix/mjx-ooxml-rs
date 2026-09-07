//! The registry, and the [`GeometryProvider`] it satisfies.
//!
//! # Why a registry, and not a lookup from the handle itself
//!
//! A [`GeometryProvider`] is handed a `u64` and a rectangle. The number is a
//! `mjx_layout::GeometryRef`, which is deliberately opaque — the box model that issued it decides
//! what it means, and nothing below the seam may know. So *somebody above the seam has to have said
//! what each handle is*, and that somebody is whoever built the fragment tree: it read the
//! `a:prstGeom`, it knows the shape's extents, and it minted the handle. This crate holds the
//! mapping it produced.
//!
//! [`ShapeOutline::from_preset_geometry`] is the bridge, so an application registers a handle
//! straight out of the document model rather than restating a shape's adjustments by hand.
//!
//! # What happens to a shape this build cannot draw
//!
//! 181 of the 187 presets have no path table until MJXOFF-203 lands, and a handle may be unknown
//! outright. Neither may draw nothing:
//!
//! > *"a shape that silently drew nothing is a defect a reader reports as 'my slide is missing a
//! > box' and nobody finds"* — `mjx-scene`'s own seam.
//!
//! So there are exactly two answers, and [`UnknownShapePolicy`] chooses between them per provider:
//! refuse with an error, or draw `mjx-scene`'s stand-in **with its
//! [`OutlineProvenance::Placeholder`](mjx_scene::OutlineProvenance::Placeholder) intact**, so the render is visibly wrong *and* countable.
//! What neither answer does is silently substitute a rectangle.
//!
//! The policy applies only to a *gap in the table*
//! ([`GeometryError::is_a_gap_in_the_table`]). A shape that is seeded and whose guide list will not
//! evaluate is a defect in this crate's own data, and papering over it with a rounded rectangle
//! would hide the one failure the seed table exists to catch — so that is an error under both
//! policies.

use std::collections::HashMap;

use mjx_dml::geometry::{PresetGeometry, Size};
use mjx_ooxml_core::Interner;
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_scene::{GeometryProvider, PlaceholderGeometry, ResolvedOutline, SceneError, SceneRect};

use crate::resolve::preset_outline;
use crate::GeometryError;

/// One `a:avLst` override: an adjustment's wire name and the value in effect.
///
/// The value is in **native spec units** — 1000ths of a percent for a fraction, 60000ths of a
/// degree for an angle — because that is the unit the shape's own guide formulas are written in,
/// and converting on the way in would mean converting back before `pin 0 adj 50000` could be
/// evaluated.
#[derive(Clone, PartialEq, Debug)]
pub struct AdjustmentOverride {
    /// The wire guide name (`adj`, `adj1`, …), as `a:avLst`'s `a:gd@name` writes it.
    pub wire_name: String,
    /// The value, in native spec units.
    pub value: f64,
}

impl AdjustmentOverride {
    /// An override of `wire_name` to `value`.
    #[must_use]
    pub fn new(wire_name: impl Into<String>, value: f64) -> Self {
        Self {
            wire_name: wire_name.into(),
            value,
        }
    }
}

/// What one outline handle stands for: a preset shape, at a size, with its adjustments.
#[derive(Clone, PartialEq, Debug)]
pub struct ShapeOutline {
    /// Which preset (`a:prstGeom@prst`).
    pub preset: PresetShapeType,
    /// The shape's extents in EMU (`a:xfrm/a:ext`) — the `w` and `h` its guides evaluate against.
    ///
    /// Not inferable from the device-pixel box the seam supplies; [`crate::resolve`] says why.
    pub extents: Size,
    /// The shape's `a:avLst` overrides. Empty is normal: an adjustment nobody overrode takes the
    /// default from the generated table.
    pub adjustments: Vec<AdjustmentOverride>,
}

impl ShapeOutline {
    /// A shape with no adjustment overrides.
    #[must_use]
    pub fn new(preset: PresetShapeType, extents: Size) -> Self {
        Self {
            preset,
            extents,
            adjustments: Vec::new(),
        }
    }

    /// The same shape with one more override — for building a registry entry in one expression.
    #[must_use]
    pub fn with_adjustment(mut self, wire_name: impl Into<String>, value: f64) -> Self {
        self.adjustments
            .push(AdjustmentOverride::new(wire_name, value));
        self
    }

    /// What a document's own `a:prstGeom` says, ready to register.
    ///
    /// `None` when the element names no `prst`, or names a token this build does not know — which
    /// is a shape a *future* version of the format defines, and is exactly the case that must reach
    /// the placeholder rather than a wrong outline.
    ///
    /// Only *overridden* adjustments are carried across: the defaults are already in the generated
    /// table, and copying them into every registry entry would put two sources of the same number
    /// in the process.
    #[must_use]
    pub fn from_preset_geometry(
        geometry: &PresetGeometry,
        interner: &Interner,
        extents: Size,
    ) -> Option<Self> {
        let preset = geometry.preset(interner)?;
        let adjustments = geometry
            .adjustments(interner)
            .into_iter()
            .filter(|adjustment| adjustment.is_overridden)
            .map(|adjustment| {
                AdjustmentOverride::new(adjustment.spec.wire_name, f64::from(adjustment.value))
            })
            .collect();
        Some(Self {
            preset,
            extents,
            adjustments,
        })
    }
}

/// What a provider answers with when it has no path table for a shape, or no shape for a handle.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum UnknownShapePolicy {
    /// Answer with [`SceneError::UnresolvedOutline`], so the page fails to render rather than
    /// rendering a lie.
    ///
    /// The default, and the right answer for a gate: `tests/a_preset_renders_as_itself.rs` uses it
    /// to prove a scene of seeded shapes needs no stand-in at all.
    #[default]
    Refuse,
    /// Answer with [`PlaceholderGeometry`]'s framed, crossed rounded rectangle, carrying
    /// [`OutlineProvenance::Placeholder`](mjx_scene::OutlineProvenance::Placeholder).
    ///
    /// The right answer for an application: a deck whose every shape must draw is better served by
    /// 181 obviously-wrong shapes it can count than by a blank page. The count is what makes it
    /// safe — `DrawReport::placeholders` is non-zero, so no golden image taken against it can be
    /// recorded as fidelity.
    StandIn,
}

/// A [`GeometryProvider`] backed by the preset path tables.
///
/// Answers every registered handle whose shape this build has a table for, with
/// [`OutlineProvenance::Document`](mjx_scene::OutlineProvenance::Document); answers the rest by its [`UnknownShapePolicy`].
#[derive(Clone, PartialEq, Debug, Default)]
pub struct PresetGeometryProvider {
    shapes: HashMap<u64, ShapeOutline>,
    unknown_shapes: UnknownShapePolicy,
}

impl PresetGeometryProvider {
    /// An empty provider that refuses a shape it cannot draw.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty provider that stands in for a shape it cannot draw, provenance intact.
    #[must_use]
    pub fn standing_in_for_unknown_shapes() -> Self {
        Self {
            shapes: HashMap::new(),
            unknown_shapes: UnknownShapePolicy::StandIn,
        }
    }

    /// What this provider does with a shape it cannot draw.
    #[must_use]
    pub fn unknown_shape_policy(&self) -> UnknownShapePolicy {
        self.unknown_shapes
    }

    /// Records what `outline` stands for, answering with whatever it stood for before.
    pub fn register(&mut self, outline: u64, shape: ShapeOutline) -> Option<ShapeOutline> {
        self.shapes.insert(outline, shape)
    }

    /// What `outline` stands for, or `None` if nothing registered it.
    #[must_use]
    pub fn shape(&self, outline: u64) -> Option<&ShapeOutline> {
        self.shapes.get(&outline)
    }

    /// How many handles are registered.
    #[must_use]
    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    /// Whether no handle is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// The outline `outline` draws, **with the reason it could not**.
    ///
    /// The same work [`GeometryProvider::outline`] does, minus the policy: this answers with a
    /// [`GeometryError`] that says whether the handle was unregistered, the shape unseeded or the
    /// table's own guides unevaluable, and the trait's answer collapses all three into
    /// [`SceneError::UnresolvedOutline`] because `mjx-scene`'s error vocabulary is `mjx-scene`'s.
    ///
    /// # Errors
    ///
    /// [`GeometryError::UnregisteredOutline`], and whatever [`preset_outline`] reports.
    pub fn resolve(
        &self,
        outline: u64,
        within: SceneRect,
    ) -> Result<ResolvedOutline, GeometryError> {
        let shape = self
            .shapes
            .get(&outline)
            .ok_or(GeometryError::UnregisteredOutline { outline })?;
        preset_outline(shape.preset, shape.extents, &shape.adjustments, within)
    }
}

impl GeometryProvider for PresetGeometryProvider {
    fn outline(&self, outline: u64, within: SceneRect) -> Result<ResolvedOutline, SceneError> {
        match self.resolve(outline, within) {
            Ok(resolved) => Ok(resolved),
            Err(error)
                if error.is_a_gap_in_the_table()
                    && self.unknown_shapes == UnknownShapePolicy::StandIn =>
            {
                PlaceholderGeometry::new().outline(outline, within)
            }
            Err(_) => Err(SceneError::UnresolvedOutline { outline }),
        }
    }
}
