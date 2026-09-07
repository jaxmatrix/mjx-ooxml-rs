//! What resolving a preset shape can go wrong with.
//!
//! # Why none of these is an empty path
//!
//! `mjx-scene`'s own seam already says it, on the method this crate implements: *"a shape that
//! silently drew nothing is a defect a reader reports as 'my slide is missing a box' and nobody
//! finds"*. Every failure here is therefore a value a caller has to handle, and the one place a
//! caller may choose otherwise —
//! [`UnknownShapePolicy::StandIn`](crate::UnknownShapePolicy::StandIn) — answers with the
//! *placeholder*, which is a visible, provenance-marked wrong shape rather than an invisible one.
//!
//! **There is exactly one empty answer in this crate and it is not a failure**: a shape whose
//! extents have no area draws no commands, because there is nothing to draw and a reader would not
//! report a missing box that has no box. [`preset_outline`](crate::preset_outline) says why at
//! length, and `tests/an_adjustment_moves_the_shape.rs` gates it in both directions so that
//! "it drew nothing" cannot spread into the answer to everything.
//!
//! # Why the trait's error is coarser than this one
//!
//! [`GeometryProvider::outline`](mjx_scene::GeometryProvider::outline) answers with
//! [`SceneError`](mjx_scene::SceneError), which is `mjx-scene`'s vocabulary and cannot grow a
//! variant from up here. So every failure of this crate reaches that seam as
//! [`SceneError::UnresolvedOutline`](mjx_scene::SceneError::UnresolvedOutline), naming the handle
//! and nothing more. The reason is not lost: [`PresetGeometryProvider::resolve`] is the same call
//! answering with a [`GeometryError`], and it is what a caller that wants to know *why* should use.
//! Nothing below the display list changes for this crate to exist, which is MJXOFF-201 §3's rule.
//!
//! [`PresetGeometryProvider::resolve`]: crate::PresetGeometryProvider::resolve

use mjx_dml::geometry::GuideError;

/// What resolving a preset outline can go wrong with.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GeometryError {
    /// The handle names no shape in the provider's registry.
    ///
    /// A provider has been paired with a box model that is not its own, or a shape was drawn
    /// without being registered. Either way it is a wiring defect and not a document defect.
    #[error("no shape is registered for the outline handle {outline}")]
    UnregisteredOutline {
        /// The handle the box model issued.
        outline: u64,
    },

    /// The shape is a preset this build has no path table for.
    ///
    /// Today that is one of the 187 `ST_ShapeType` values — `upArrow`, which ECMA-376's own
    /// `presetShapeDefinitions.xml` defines no geometry for; see
    /// [`PRESETS_WITHOUT_GEOMETRY`](crate::PRESETS_WITHOUT_GEOMETRY). The error names the wire
    /// token rather than the Rust variant, because the wire token is what the document said.
    #[error("no preset path table is seeded for the shape `{shape}`")]
    UnseededShape {
        /// The shape's `prst` token, as `a:prstGeom@prst` writes it.
        shape: &'static str,
    },

    /// A guide of the shape's own list could not be evaluated.
    ///
    /// The path table is this workspace's, so this is a table defect rather than a document one —
    /// with one exception that is neither: a shape with a zero side makes `ss` zero, so a guide
    /// such as `*/ 100000 w ss` is infinite and `mjx-dml` refuses it. [`preset_outline`] catches
    /// exactly that case and answers with an empty outline instead, because a shape with no area
    /// has nothing to draw; this variant is what a caller sees when it asks for an adjustment's
    /// *domain* at such a size, where there is no honest answer at all.
    ///
    /// [`preset_outline`]: crate::preset_outline
    #[error("evaluating the guides of `{shape}`: {source}")]
    Guides {
        /// The shape's `prst` token.
        shape: &'static str,
        /// What the evaluator said.
        #[source]
        source: GuideError,
    },

    /// The shape has no geometry **at this size and these adjustment values**, because one of its
    /// own guide formulas has no finite value there and a path reads it.
    ///
    /// Not a defect in the table and not a gap in it: ECMA-376's formulas divide and take square
    /// roots, and at the ends of an adjustment's domain the divisor can be zero.
    /// `circularArrow`'s `dxF1 = "+/ q11 q10 q4"` has no value at `adj5 = 0` — which is that
    /// adjustment's own *minimum*, and therefore a value a handle drag reaches. Six of the 186
    /// presets have such a point; `crates/mjx-geometry/tests/every_preset_stands_where_its_box_is.rs`
    /// names them and the guide each is singular in.
    ///
    /// [`crate::resolve`] leaves a guide with no finite value **undefined** rather than fatal, so a
    /// singularity in a guide nothing draws through — four of the ten shapes that have one are
    /// singular only in the text rectangle's insets — costs nothing. This variant is what happens
    /// when a *path* reads one, and it answers
    /// [`has_no_geometry_to_draw`](Self::has_no_geometry_to_draw) with `true`: a counted stand-in is
    /// a better answer than a page that will not render, and a silent empty path is not an answer
    /// at all.
    #[error(
        "`{shape}` has no geometry at these adjustments: its guide `{guide}` has no finite value"
    )]
    SingularGeometry {
        /// The shape's `prst` token.
        shape: &'static str,
        /// The guide whose formula has no finite value here.
        guide: String,
    },

    /// A coordinate of the shape's path named a guide the shape does not define.
    ///
    /// Distinct from [`Guides`](Self::Guides) on purpose: the guide list evaluated cleanly and a
    /// *path* then reached for a name that is not in it, which is the single most likely mistake a
    /// hand-transcribed or a newly generated table can make.
    #[error("resolving a path command of `{shape}`: {source}")]
    PathCommand {
        /// The shape's `prst` token.
        shape: &'static str,
        /// What the resolver said.
        #[source]
        source: GuideError,
    },
}

impl GeometryError {
    /// The shape's `prst` token, for the failures that name one.
    ///
    /// `None` for [`UnregisteredOutline`](Self::UnregisteredOutline), which fails before any shape
    /// is known.
    #[must_use]
    pub fn shape(&self) -> Option<&'static str> {
        match self {
            Self::UnregisteredOutline { .. } => None,
            Self::UnseededShape { shape }
            | Self::Guides { shape, .. }
            | Self::SingularGeometry { shape, .. }
            | Self::PathCommand { shape, .. } => Some(shape),
        }
    }

    /// Whether this failure is *"there is no geometry to draw here"* rather than *"the geometry
    /// here is wrong"*.
    ///
    /// The distinction is what [`UnknownShapePolicy`](crate::UnknownShapePolicy) switches on. Three
    /// failures are of the first kind — a handle nobody registered, a preset ECMA-376 defines no
    /// geometry for, and a shape whose own formulas are singular at these adjustments — and a
    /// stand-in may legitimately fill any of them, because there is nothing else to draw and the
    /// stand-in is *counted*. A table whose guide list will not evaluate, or whose path names a
    /// guide that does not exist, is of the second kind: a bug that must not be papered over with
    /// a rounded rectangle, because a stand-in would hide the one failure the table's own gates
    /// exist to catch.
    #[must_use]
    pub fn has_no_geometry_to_draw(&self) -> bool {
        matches!(
            self,
            Self::UnregisteredOutline { .. }
                | Self::UnseededShape { .. }
                | Self::SingularGeometry { .. }
        )
    }
}
