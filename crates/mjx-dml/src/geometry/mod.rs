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
//! through. It exposes typed reads and minimal typed construction; the **named** control parameters
//! (`corner_radius_fraction`, …) that replace the raw `adj` guides are a later, per-shape batch and
//! are *not* modeled here.
//!
//! # Fidelity mechanism
//!
//! Like the [text model](crate::text), each type stores the framework fields `name` (exact qualified
//! name, output only), `attributes` (verbatim), and `empty` (self-closing flag), plus — for the two
//! container types — an ordered `content` list whose variants are the typed children and a
//! `Raw(RawNode)` catch-all. [`PresetGeometry`] and [`GeometryGuideList`] derive their
//! [`FromXml`](mjx_ooxml_core::FromXml)/[`ToXml`](mjx_ooxml_core::ToXml) impls; [`GeometryGuide`] is
//! an attribute-only leaf (no children, no text) and so hand-writes them.

mod guide;
mod measures;
mod preset;
mod shape;
mod transform;

pub use guide::{GeometryGuide, GeometryGuideList, GeometryGuideListContent};
pub use measures::{Angle, Emu, FontSize, Fraction, IndentLevel, LineWidth, TextPoint};
pub use preset::{PresetGeometry, PresetGeometryContent, ResolvedAdjustment};
pub use shape::ShapeGeometry;
pub use transform::{Position, Size, Transform2D};
