//! DrawingML preset-shape geometry: the shape-kind enum and per-shape adjustment metadata.
//!
//! [`PresetShapeType`] and the [`adjustments_of`] table are **generated** from the ECMA-376
//! schemas + `presetShapeDefinitions.xml` (regenerate with `cargo run -p xtask -- codegen`); the
//! metadata types below are hand-written. Together they form the **mechanical tier** of the
//! preset-shape semantic model: standard-faithful adjustment facts (default, axis, domain) in native
//! spec units, keyed by shape. The ergonomic typed tier (named parameters in friendly units) is built
//! on top of these in `mjx-dml`.

pub use crate::generated::drawingml::{
    adjustable_shapes, adjustment_bound_guides_of, adjustments_of, AutonumberScheme, BevelPreset,
    BlendMode, ColorSchemeSlot, CompoundLine, FontAlignment, FontCollectionIndex,
    LightRigDirection, LightRigType, LineCap, LineEndLength, LineEndType, LineEndWidth, OnOffStyle,
    PathFillMode, PatternType, PenAlignment, PresetCamera, PresetLineDash, PresetMaterial,
    PresetShadow, PresetShapeType, RectangleAlignment, SchemeColor, TabAlignment, TextAlignment,
    TextAnchoring, TextCapitalization, TextDirection, TextHorizontalOverflow, TextStrike,
    TextUnderline,
};

/// The axis a shape adjustment controls, disclosed by which `ahLst` handle reference names its guide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdjustmentAxis {
    /// A horizontal offset / width fraction (`ahXY gdRefX`).
    Horizontal,
    /// A vertical thickness / height fraction (`ahXY gdRefY`).
    Vertical,
    /// An angle in 60000ths of a degree (`ahPolar gdRefAng`; a full turn is `21_600_000`).
    Angle,
    /// A radius fraction (`ahPolar gdRefR`).
    Radius,
}

/// A shape adjustment's domain bound: a literal, or the name of a computed geometry guide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustmentBound {
    /// A literal bound in native spec units.
    Literal(i32),
    /// The name of a guide whose formula computes the bound (data-dependent on the shape's `w`/`h`
    /// and other adjustments). [`adjustment_bound_guides_of`] carries the `gdLst` guides it is
    /// computed from; evaluating them turns the bound into a number (`mjx-dml`'s
    /// `PresetGeometry::adjustments_for_size` does exactly that). A handful name a built-in variable
    /// (`star24`/`star32` bound their point depth to `ssd2`) rather than a `gdLst` guide.
    Guide(&'static str),
}

/// One guide of a preset shape's geometry — a `name`/`fmla` pair exactly as
/// `presetShapeDefinitions.xml` writes it (see [`adjustment_bound_guides_of`]).
///
/// The formula is kept as its wire text, not parsed: the guide-formula language belongs to `mjx-dml`,
/// which sits above this crate, and this tier stays purely mechanical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetGuide {
    /// The guide's `name` attribute, as other guides and coordinates reference it.
    pub wire_name: &'static str,
    /// The guide's `fmla` attribute, e.g. `*/ 50000 w ss`.
    pub formula: &'static str,
}

/// The metadata for one user-facing shape adjustment (see [`adjustments_of`]).
///
/// Values are in **native spec units**: horizontal/vertical/radius fractions are in 1000ths of a
/// percent (`100_000` = 100%), angles are in 60000ths of a degree. This is the standard-faithful
/// form; the ergonomic typed tier converts to friendly units (fractions `0.0..=1.0`, radians, points).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdjustmentSpec {
    /// The wire guide name (`adj`, `adj1`, …) as written in a shape's `avLst`.
    pub wire_name: &'static str,
    /// Which axis the adjustment controls.
    pub axis: AdjustmentAxis,
    /// The default value (the guide's `val` seed) used when the shape does not override it.
    pub default: i32,
    /// The lower bound of the value domain.
    pub min: AdjustmentBound,
    /// The upper bound of the value domain.
    pub max: AdjustmentBound,
}
