//! `a:spPr` (`CT_ShapeProperties`) — a shape or picture's visual properties: transform, geometry,
//! fill, outline, effects, and 3-D.
//!
//! **The type MJXOFF-107 and MJXOFF-131 both found missing.** `crates/mjx-pptx/src/slide.rs`
//! navigates `p:spPr`/`a:spPr` ad hoc (61 references) rather than through a shared type, because none
//! existed. This one does, and is the piece that lets [`crate::picture::Picture`] (`pic:pic`) and
//! [`WordprocessingShape`](https://docs.rs/mjx-docx) (`wps:wsp`, `mjx-docx`, whose own `spPr` is the
//! exact same `CT_ShapeProperties`) share one model rather than each inventing its own. This child
//! does **not** migrate `mjx-pptx`'s existing, already-tested ad hoc navigation to use it — that is a
//! separate refactor with its own risk, not required by anything this child's own "Done when" asks
//! for — so `mjx-pptx` keeps its own reader for now while every new caller has a real type to reach
//! for.
//!
//! A fidelity wrapper, in the same shape as [`crate::line::LineProperties`] and
//! [`crate::geometry::CustomGeometry`]: `bwMode` is typed via the derive macro; every child is
//! exposed through a typed accessor while the storage stays the raw `children` list, so an unmodeled
//! child (`extLst`, an `effectDag`) round-trips byte-for-byte. Placement of a **new** child uses the
//! *generated* [`mjx_ooxml_types::child_order::SHAPE_PROPERTIES`] table — `dml-main` has carried this
//! type's ordering since A7c, so there is no hand-rolled rank table to keep in sync with the schema.

use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawName, RawNode, Text, ToXml};
use mjx_ooxml_types::child_order::SHAPE_PROPERTIES;

use crate::build::{dml_child, dml_child_mut, dml_name, fidelity_element_impls, first_fill_child};
use crate::effect::EffectList;
use crate::fill::Fill;
use crate::geometry::{CustomGeometry, PresetGeometry, Transform2D};
use crate::line::LineProperties;
use crate::shape3d::{Scene3D, Shape3D};

/// The `EG_Geometry` choice: `a:prstGeom` or `a:custGeom`, at most one of either.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapeGeometryChoice {
    /// `a:prstGeom` — a named preset outline.
    Preset(PresetGeometry),
    /// `a:custGeom` — an explicit path outline.
    Custom(CustomGeometry),
}

/// `a:spPr` (`CT_ShapeProperties`) — see this module's own doc comment.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
// `ST_BlackWhiteMode` is not (yet) in `mjx-ooxml-types`'s curated DrawingML enumeration allowlist —
// nothing in this workspace reads a shape's black-and-white print mode today — so the raw wire token
// is exposed as text rather than adding an enum this child would be the only caller of.
#[xml(attribute(local = "bwMode", codec = Text, accessor = black_and_white_mode_token))]
pub struct ShapeProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl ShapeProperties {
    /// Builds a self-closing `<a:spPr/>`. **Most host schemas do not want this**: `CT_ShapeProperties`
    /// is DrawingML-main's own type, but `spPr` is a *local* element declaration inside each host's
    /// own complex type (`pic:pic`'s `CT_Picture`, `wp:wsp`'s `CT_WordprocessingShape`,
    /// PresentationML's own `CT_Shape`), so the wrapper element takes that host's own namespace —
    /// `pic:spPr`, `wp:spPr`, `p:spPr` — never literally `a:spPr` except inside `dml-main.xsd`'s own
    /// types. Use [`Self::with_name`] with the host's own qualified name instead.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str) -> Self {
        let name = dml_name(interner, local);
        Self::with_name(interner, name)
    }

    /// [`Self::new`], with the wrapper element's fully qualified name given explicitly — the
    /// constructor every cross-schema host of this type actually wants; see [`Self::new`]'s own doc
    /// comment for why.
    #[must_use]
    pub fn with_name(_interner: &mut Interner, name: RawName) -> Self {
        Self {
            name,
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }

    /// The shape's 2-D transform (`a:xfrm`), or `None` if it declares none — meaning it inherits one
    /// from wherever its own host places it (a group's child mapping, a placeholder's layout).
    #[must_use]
    pub fn transform(&self, interner: &Interner) -> Option<Transform2D> {
        dml_child(&self.children, interner, "xfrm")
            .map(|element| Transform2D::read(element, interner))
    }

    /// Sets the shape's 2-D transform, editing the existing `a:xfrm` in place when there is one
    /// (preserving any attribute this type does not itself set) or inserting a fresh one at its
    /// schema rank.
    pub fn set_transform(&mut self, interner: &mut Interner, transform: Transform2D) {
        if let Some(element) = dml_child_mut(&mut self.children, interner, "xfrm") {
            transform.apply(element, interner);
            self.empty = false;
            return;
        }
        let mut element = Transform2D::empty_element(interner);
        transform.apply(&mut element, interner);
        SHAPE_PROPERTIES.insert(&mut self.children, interner, element);
        self.empty = false;
    }

    /// The shape's geometry (`a:prstGeom` or `a:custGeom`), or `None` if it declares neither —
    /// meaning it inherits one from a placeholder or group ancestor.
    #[must_use]
    pub fn geometry(&self, interner: &Interner) -> Option<ShapeGeometryChoice> {
        if let Some(element) = dml_child(&self.children, interner, "prstGeom") {
            return PresetGeometry::from_xml(element, interner)
                .ok()
                .map(ShapeGeometryChoice::Preset);
        }
        if let Some(element) = dml_child(&self.children, interner, "custGeom") {
            return CustomGeometry::from_xml(element, interner)
                .ok()
                .map(ShapeGeometryChoice::Custom);
        }
        None
    }

    /// Sets the shape's geometry, replacing whichever of `a:prstGeom`/`a:custGeom` is present (the
    /// two are an `xsd:choice` — only one stands) or inserting the new one at its schema rank.
    pub fn set_geometry(&mut self, interner: &mut Interner, geometry: ShapeGeometryChoice) {
        let element = match &geometry {
            ShapeGeometryChoice::Preset(preset) => preset.to_xml(interner),
            ShapeGeometryChoice::Custom(custom) => custom.to_xml(interner),
        };
        SHAPE_PROPERTIES.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "prstGeom" || local == "custGeom"
        });
        self.empty = false;
    }

    /// The shape's fill (`EG_FillProperties`: `a:noFill`/`a:solidFill`/`a:gradFill`/`a:blipFill`/
    /// `a:pattFill`/`a:grpFill`), or `None` if it declares none — meaning it inherits one.
    #[must_use]
    pub fn fill(&self, interner: &Interner) -> Option<Fill> {
        first_fill_child(&self.children, interner)
            .and_then(|element| Fill::from_xml(element, interner).ok())
    }

    /// Sets the shape's fill, replacing whichever of the six `EG_FillProperties` alternatives is
    /// present or inserting the new one at its schema rank.
    pub fn set_fill(&mut self, interner: &mut Interner, fill: &Fill) {
        let element = fill.to_xml(interner);
        SHAPE_PROPERTIES.replace_or_insert(
            &mut self.children,
            interner,
            element,
            Fill::is_fill_local,
        );
        self.empty = false;
    }

    /// The shape's outline (`a:ln`), or `None` if it declares none — meaning it inherits one.
    #[must_use]
    pub fn line(&self, interner: &Interner) -> Option<LineProperties> {
        dml_child(&self.children, interner, "ln")
            .and_then(|element| LineProperties::from_xml(element, interner).ok())
    }

    /// Sets the shape's outline, replacing an existing `a:ln` in place or inserting a fresh one at
    /// its schema rank.
    pub fn set_line(&mut self, interner: &mut Interner, line: &LineProperties) {
        let element = line.to_xml(interner);
        SHAPE_PROPERTIES
            .replace_or_insert(&mut self.children, interner, element, |local| local == "ln");
        self.empty = false;
    }

    /// The shape's rendered-effect list (`a:effectLst`), or `None` if it declares none or declares
    /// the rarer `a:effectDag` alternative instead (preserved opaque — see
    /// [`crate::effect::EffectList`]'s own module doc for why `effectDag` is not modeled).
    #[must_use]
    pub fn effects(&self, interner: &Interner) -> Option<EffectList> {
        dml_child(&self.children, interner, "effectLst")
            .and_then(|element| EffectList::from_xml(element, interner).ok())
    }

    /// Sets the shape's effect list, replacing an existing `a:effectLst`/`a:effectDag` in place or
    /// inserting a fresh `a:effectLst` at its schema rank.
    pub fn set_effects(&mut self, interner: &mut Interner, effects: &EffectList) {
        let element = effects.to_xml(interner);
        SHAPE_PROPERTIES.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "effectLst" || local == "effectDag"
        });
        self.empty = false;
    }

    /// The shape's 3-D scene (`a:scene3d`), or `None` if it declares none.
    #[must_use]
    pub fn scene_3d(&self, interner: &Interner) -> Option<Scene3D> {
        dml_child(&self.children, interner, "scene3d")
            .and_then(|element| Scene3D::from_xml(element, interner).ok())
    }

    /// The shape's own 3-D extrusion (`a:sp3d`), or `None` if it declares none.
    #[must_use]
    pub fn shape_3d(&self, interner: &Interner) -> Option<Shape3D> {
        dml_child(&self.children, interner, "sp3d")
            .and_then(|element| Shape3D::from_xml(element, interner).ok())
    }
}

fidelity_element_impls!(ShapeProperties);
