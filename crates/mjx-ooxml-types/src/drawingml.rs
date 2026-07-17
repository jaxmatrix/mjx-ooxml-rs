//! DrawingML preset-shape geometry: the shape-kind enum and per-shape adjustment metadata.
//!
//! [`PresetShapeType`] and the [`adjustments_of`] table are **generated** from the ECMA-376
//! schemas + `presetShapeDefinitions.xml` (regenerate with `cargo run -p xtask -- codegen`); the
//! metadata types below are hand-written. Together they form the **mechanical tier** of the
//! preset-shape semantic model: standard-faithful adjustment facts (default, axis, domain) in native
//! spec units, keyed by shape. The ergonomic typed tier (named parameters in friendly units) is built
//! on top of these in `mjx-dml`.

pub use crate::generated::drawingml::{
    adjustments_of, ColorSchemeSlot, PatternType, PresetShapeType, SchemeColor,
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
    /// The name of a `gdLst` guide whose formula computes the bound (data-dependent on the shape's
    /// `w`/`h` and other adjustments). Resolving it to a number needs the guide-formula evaluator,
    /// which is deferred to the rendering phase.
    Guide(&'static str),
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
