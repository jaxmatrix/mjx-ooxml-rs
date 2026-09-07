//! Preset shape geometry: what a `<a:prstGeom prst="roundRect"/>` actually draws.
//!
//! # What this crate is
//!
//! `mjx-scene` puts a [`GeometryProvider`](mjx_scene::GeometryProvider) between a display list and
//! the shapes in it: a [`Geometry::Unresolved`](mjx_scene::Geometry::Unresolved) carries an opaque
//! handle and a box, and somebody above the seam turns the pair into a path. Until this crate that
//! somebody was [`PlaceholderGeometry`](mjx_scene::PlaceholderGeometry), which draws a framed,
//! crossed rounded rectangle that is not any shape DrawingML defines — deliberately, so that a
//! placeholder render could never be mistaken for a fidelity render.
//!
//! This crate is the real one. [`PresetGeometryProvider`] holds a registry of handles, each naming
//! a [`mjx_ooxml_types::drawingml::PresetShapeType`], the shape's extents and its
//! `a:avLst` overrides; answering a handle evaluates the shape's guide list against those extents
//! and resolves its path into device pixels. Every outline it answers with carries
//! [`OutlineProvenance::Document`](mjx_scene::OutlineProvenance::Document).
//!
//! # Rank 2.5, and what the number makes impossible
//!
//! `mjx-layout` is 1.6 and `mjx-scene` is 1.7 for reasons their own crate documentation gives, and
//! both are about what those crates may **not** reach. This one is the mirror image: it is placed
//! by what may not reach **it**.
//!
//! A preset path table is DrawingML. It names a
//! [`mjx_ooxml_types::drawingml::PresetShapeType`], it is written in the guide
//! formula language of ECMA-376 Part 1 §20.1.9.11, and it resolves through `mjx-dml`'s evaluator.
//! So the crate that holds it is *above* `mjx-dml` (2.0), and the two crates whose whole purpose is
//! not to have heard of DrawingML must stay below it:
//!
//! * **`mjx-scene` (1.7) cannot depend on this crate**, because 2.5 is above 1.7 and
//!   `xtask/tests/layering.rs` refuses an edge that points up. That is the same refusal that keeps
//!   `mjx-scene → mjx-dml` illegal, and it is what stops the provider being "just moved into
//!   `mjx-scene`" the first time somebody finds the seam inconvenient. A display list that could
//!   read a preset table would be a display list that knows what a `.pptx` is.
//! * **`mjx-layout` (1.6) cannot depend on this crate**, for the same arithmetic. A box model
//!   issues a `mjx_layout::GeometryRef` — a bare number — precisely because it must not know what
//!   the number means; an edge from 1.6 to 2.5 would let it resolve its own handles and the seam
//!   would be decoration.
//! * **`mjx-paint` (5.5) is above this crate and so *could* depend on it**, which is exactly the
//!   hole rank cannot close at the top of the ladder. It is closed the way the painter's other
//!   seam is closed — by name, in `crates/mjx-paint/tests/the_seam_holds.rs`, which lists
//!   `mjx-geometry` among the crates a painter may not mention. A painter that constructed its own
//!   preset provider would have learned what a preset shape is, which is the one thing the display
//!   list exists to spare it.
//!
//! What the rank does **not** buy, stated so nobody relies on it: 2.5 is below the format tier, so
//! `mjx-pptx` (3.0) may legally depend on this crate. That is intended — a format crate is allowed
//! to know what its own shapes look like — and it is why the number is 2.5 rather than, say, 5.4:
//! a provider the application can construct is the point, and the application is above everything.
//!
//! # The pipeline, and what is reused rather than rebuilt
//!
//! ```text
//!   handle ─▶ ShapeOutline { preset, extents, adjustments }        (this crate's registry)
//!          ─▶ ResolvedGuides                                       (mjx-dml: formula.rs)
//!          ─▶ Vec<DrawCommand>                                     (mjx-dml's vocabulary, from the table)
//!          ─▶ Vec<ResolvedDrawCommand>                             (mjx-dml: resolved.rs)
//!          ─▶ Vec<PathCommand> in device pixels                    (this crate: arcs and the map)
//! ```
//!
//! Only the first and last steps are new. There is **no second command vocabulary**: the table
//! stores [`PresetPathStep`]s only because [`DrawCommand`](mjx_dml::geometry::DrawCommand) owns
//! `String`s and cannot be written in a `static`, and every step converts to exactly one
//! `DrawCommand` by [`PresetPathStep::to_draw_command`]. The only geometry this crate computes for
//! itself is the one `mjx-dml` deliberately does not: an `a:arcTo` is an elliptical arc and a
//! [`PathCommand`](mjx_scene::PathCommand) has no arc, so [`arc`] decomposes one into cubics.
//!
//! # The table is 186 shapes, and the six hand-written ones stayed as its reference
//!
//! [`generated`] holds every preset ECMA-376's `presetShapeDefinitions.xml` defines geometry for,
//! extracted mechanically by `cargo run -p xtask -- codegen` (MJXOFF-203). [`seeded_shapes`]
//! returns those.
//!
//! **186 and not 187**, and that is the file's arithmetic rather than a gap in the extraction:
//! `ST_ShapeType` declares 187 values and the geometry file has no `upArrow` element at all.
//! [`PRESETS_WITHOUT_GEOMETRY`] names what is left over, derived from the difference rather than
//! written down.
//!
//! # A shape is more than its outline: where its text goes, and where a connector attaches
//!
//! Two of `presetShapeDefinitions.xml`'s elements say nothing about what a shape *draws* and
//! everything about how it is used (MJXOFF-204), and both resolve through the same guide
//! environment and the same affine map the paths do:
//!
//! * **`a:rect`, the text rectangle** — [`preset_text_rectangle`]. 181 of the 186 declare one, and
//!   136 of those inset it from the shape's own box: a rounded rectangle's text starts 29.289 % of
//!   the corner radius in, a chevron's past the notch. A renderer that laid text against the
//!   bounding box instead would still draw text, in the wrong place, which is why the answer is a
//!   four-armed [`TextRectangle`] and the bounding-box fallback is a **named call** rather than a
//!   default.
//! * **`a:cxnLst`, the connection sites** — [`preset_connection_sites`]. 856 places a connector can
//!   attach, each a point *and* an outgoing angle, because an elbow connector leaving the top of a
//!   box has to travel up before it turns. Thirteen presets declare none, and nine of those are
//!   themselves connectors.
//!
//! Two of ECMA-376's own rows are wrong in these elements, and `xtask`'s `RECT_ERRATA` and
//! `CONNECTION_ERRATA` correct them with the file's own sibling rows as evidence and its text
//! guarded: `pie` names its top edge with a horizontal guide, and `squareTabs`'s sixth site names
//! its `y` with one. Both were found by the gates rather than by reading — a text rectangle outside
//! its shape, and a connection site below its shape's bottom edge.
//!
//! [`seed`] still holds the six presets MJXOFF-202 transcribed by hand from ECMA-376 Part 1's own
//! prose, before `References/` was available. They are no longer the table — they are what the
//! table was **checked against**, in `tests/the_two_routes_agree.rs`: a hand transcription from the
//! spec's prose and a mechanical extraction from the spec's XML are two routes to one answer, and
//! they must agree. Each seed row's [`Derivation`] says how independent its route really was,
//! because a comparison whose two sides share a source proves nothing — and four of the six took a
//! constant or an idiom from tables this workspace had already generated from that same XML.

pub mod arc;
pub mod error;
pub mod generated;
pub mod provider;
pub mod resolve;
pub mod seed;
pub mod table;

pub use arc::{
    arc_to_cubics, parametric_angle, ArcSegment, ShapePoint, MAXIMUM_ARC_SEGMENT_RADIANS,
};
pub use error::GeometryError;
pub use generated::PRESETS_WITHOUT_GEOMETRY;
pub use provider::{AdjustmentOverride, PresetGeometryProvider, ShapeOutline, UnknownShapePolicy};
pub use resolve::{
    adjustment_domains, connection_sites_of_definition, contours_of_definition,
    outline_of_definition, preset_connection_sites, preset_contours, preset_outline,
    preset_text_rectangle, text_rectangle_of_definition, AdjustmentDomain, ConnectionPoint,
    PresetContour, TextRectangle,
};
pub use table::{
    definition_of, seeded_shapes, PresetAngle, PresetConnectionSite, PresetCoordinate, PresetPath,
    PresetPathStep, PresetPoint, PresetShapeDefinition, PresetTextRectangle,
};

/// The two types this crate's public surface is written in that belong to the tiers below it,
/// re-exported for the reason `mjx-scene` re-exports [`Color`](mjx_scene::Color): a caller must be
/// able to register a shape and read an answer without naming a second crate to say which shape and
/// how big.
///
/// [`mjx_dml::geometry::Size`] is `a:xfrm/a:ext` and [`PresetShapeType`] is `a:prstGeom@prst`;
/// both are the document's own vocabulary and neither is redefined here, because a second `Size` in
/// one workspace is the defect the layering rule exists to prevent.
pub use mjx_dml::geometry::Size;

pub use mjx_ooxml_types::drawingml::{PathFillMode, PresetShapeType};

/// The unit a connection site's outgoing direction is answered in, re-exported for the same reason
/// as [`Size`]: [`ConnectionPoint::angle`] is written in it, and a caller must be able to read the
/// answer without naming a second crate.
pub use mjx_ooxml_core::measure::Angle;

/// How independently a seed shape's geometry was arrived at.
///
/// MJXOFF-202's whole reason for hand-authoring six shapes is that MJXOFF-203 can then diff its
/// mechanical extraction of `presetShapeDefinitions.xml` against a transcription that did **not**
/// come from that file. A comparison whose two sides share a source proves nothing, so each seed
/// row records which it is rather than letting the next child assume the best case.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Derivation {
    /// Arrived at from the shape's description and the built-in variables alone, with no knowledge
    /// of `presetShapeDefinitions.xml` involved — there is only one way to draw the shape.
    FromFirstPrinciples,
    /// Arrived at from the shape's description, but with at least one number or idiom taken from a
    /// table this workspace already generated **out of the same XML** MJXOFF-203 will read — an
    /// adjustment's domain from `adjustments_of`, a bound guide from `adjustment_bound_guides_of`,
    /// or a formula quoted in `mjx-dml`'s own documentation. The *paths* are still independent; the
    /// constants are not.
    ConstantsFromTheGeneratedTables,
    /// Read straight out of ECMA-376's `presetShapeDefinitions.xml` by
    /// `cargo run -p xtask -- codegen` — no transcription, no naming, no interpretation beyond
    /// `CT_Path2D`'s schema defaults.
    ///
    /// The normative artefact itself, which is why a disagreement between a row marked this and a
    /// row marked either of the other two is settled **in this one's favour**: the other two are
    /// somebody reading prose, and this is the file the prose describes.
    ExtractedFromTheGeometryFile,
}

impl Derivation {
    /// Every value, so a gate can assert both are exercised rather than trusting they are.
    pub const ALL: [Self; 3] = [
        Self::FromFirstPrinciples,
        Self::ConstantsFromTheGeneratedTables,
        Self::ExtractedFromTheGeometryFile,
    ];
}
