//! DrawingML preset-shape geometry: `a:prstGeom` → `a:avLst` → `a:gd`.
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

use mjx_ooxml_core::{Interner, QuoteStyle, RawAttribute, RawName};
use mjx_ooxml_types::namespaces::DML_MAIN;
use mjx_xml::text::escape_attribute;

mod guide;
mod preset;

pub use guide::{GeometryGuide, GeometryGuideList, GeometryGuideListContent};
pub use preset::{PresetGeometry, PresetGeometryContent};

/// Builds a DrawingML qualified name `a:local` — literal prefix `a` plus the resolved transitional
/// namespace, so a built element serializes as `a:local` and reads back by `(DML_MAIN, local)`.
fn dml_name(interner: &mut Interner, local: &str) -> RawName {
    RawName {
        prefix: Some(interner.intern("a")),
        local: interner.intern(local),
        namespace: Some(interner.intern(DML_MAIN.transitional)),
    }
}

/// Builds an unprefixed, double-quoted attribute `local="value"`, escaping `value` for an attribute.
fn dml_attr(interner: &mut Interner, local: &str, value: &str) -> RawAttribute {
    RawAttribute {
        name: RawName {
            prefix: None,
            local: interner.intern(local),
            namespace: None,
        },
        value: escape_attribute(value).as_bytes().into(),
        quote: QuoteStyle::Double,
    }
}

/// The UTF-8 value of the first unprefixed attribute named `local`, or `None` if absent (or the
/// bytes are not UTF-8). The value is returned verbatim — guide names and formulas contain no
/// XML-special characters in practice, so no unescaping is needed.
fn attr_str<'a>(
    attributes: &'a [RawAttribute],
    interner: &Interner,
    local: &str,
) -> Option<&'a str> {
    attributes
        .iter()
        .find(|attribute| {
            attribute.name.prefix.is_none() && interner.resolve(attribute.name.local) == local
        })
        .and_then(|attribute| std::str::from_utf8(&attribute.value).ok())
}
