//! What shape a shape is, and where it sits: preset geometry (`a:prstGeom`) and the 2-D transform
//! that places it ([`Transform2D`], `a:xfrm`), over the friendly measures ([`Emu`], [`Angle`],
//! [`Fraction`]) both are expressed in.
//!
//! # Preset geometry: `a:prstGeom` → `a:avLst` → `a:gd`
//!
//! A shape's geometry is `spPr > (prstGeom | custGeom)`. A **preset** shape serializes only its
//! preset kind plus an optional list of adjustment overrides:
//!
//! ```xml
//! <a:prstGeom prst="roundRect"><a:avLst><a:gd name="adj" fmla="val 25000"/></a:avLst></a:prstGeom>
//! ```
//!
//! [`PresetGeometry`] (`a:prstGeom`, `CT_PresetGeometry2D`) carries the preset kind (its `prst`
//! attribute, a [`PresetShapeType`](mjx_ooxml_types::drawingml::PresetShapeType)) and an optional
//! [`GeometryGuideList`] (`a:avLst`, `CT_GeomGuideList`) of [`GeometryGuide`]s (`a:gd`,
//! `CT_GeomGuide`, each a `name`/`fmla` pair).
//!
//! This is the **fidelity layer**: it round-trips *any* preset shape byte-for-byte — `prst` and the
//! `avLst` `gd` overrides are preserved verbatim, and unknown attributes/children pass straight
//! through. It exposes typed reads and minimal typed construction. The **named** control parameters
//! that replace the raw `adj` guides live one tier up, in [`ShapeGeometry`]; the numeric domain each
//! of them is clamped to comes from [`PresetGeometry::adjustments_for_size`], which evaluates the
//! shape's own `gdLst` bound guides through the [`formula`] evaluator.
//!
//! # Guide formulas: `a:gdLst` → `a:gd@fmla`
//!
//! A coordinate in a custom geometry, and an adjustment's domain in a preset one, may be a *formula*
//! rather than a number: `<a:gd name="x1" fmla="*/ w adj1 100000"/>`. The [`formula`] module is the
//! evaluator for that language — all seventeen operators, the built-in variables (`w`, `hc`, `ss`,
//! `3cd4`, …), and the declaration-order evaluation that makes a cyclic guide list impossible rather
//! than merely detectable. On top of it, [`CustomGeometrySpec::resolve`] turns a whole geometry
//! into a [`ResolvedCustomGeometry`] — every point an [`Emu`], every arc angle an [`Angle`].
//!
//! # Fidelity mechanism
//!
//! Like the [text model](crate::text), each type stores the framework fields `name` (exact qualified
//! name, output only), `attributes` (verbatim), and `empty` (self-closing flag), plus — for the two
//! container types — an ordered `content` list whose variants are the typed children and a
//! `Raw(RawNode)` catch-all. [`PresetGeometry`] and [`GeometryGuideList`] derive their
//! [`FromXml`](mjx_ooxml_core::FromXml)/[`ToXml`](mjx_ooxml_core::ToXml) impls; [`GeometryGuide`] is
//! an attribute-only leaf (no children, no text) and so hand-writes them.

mod custom;
pub mod formula;
mod guide;
mod measures;
mod preset;
mod resolved;
mod shape;
mod transform;

pub use custom::{
    AdjustAngle, AdjustCoordinate, AdjustHandle, AdjustPoint, ConnectionSite, CustomGeometry,
    CustomGeometrySpec, DrawCommand, GuideSpec, Path2D, Path2DList, Path2DSpec, PathFillMode,
    Point, Rectangle,
};
pub use formula::{
    GuideArgument, GuideContext, GuideError, GuideFormula, GuideFormulaError, GuideOperator,
    ResolvedGuides,
};
pub use guide::{GeometryGuide, GeometryGuideList, GeometryGuideListContent};
pub use measures::{Angle, Emu, FontSize, Fraction, IndentLevel, LineWidth, TextPoint};
pub use preset::{BoundedAdjustment, PresetGeometry, PresetGeometryContent, ResolvedAdjustment};
pub use resolved::{
    ResolvedAdjustHandle, ResolvedConnectionSite, ResolvedCustomGeometry, ResolvedDrawCommand,
    ResolvedPath, ResolvedPoint, ResolvedRectangle,
};
pub use shape::ShapeGeometry;
pub use transform::{Position, Size, Transform2D};
