//! DrawingML 3-D: `a:scene3d` (`CT_Scene3D`) and `a:sp3d` (`CT_Shape3D`) — the 3-D scene a shape is
//! lit and viewed in, and the extrusion, bevels and material that give its face depth.
//!
//! [`Scene3D`] and [`Shape3D`] are **fidelity wrappers** over their elements (name, attributes,
//! children and self-closing flag preserved verbatim); the modeled facets are read through typed
//! accessors, while an unmodeled child (`a:backdrop`, `extLst`, an MCE bucket) stays opaque so the
//! element round-trips byte-for-byte. [`Scene3DSpec`] / [`Shape3DSpec`] are the interner-free values
//! `mjx-pptx`'s future `shape_scene_3d` / `shape_shape_3d` read and write.
//!
//! The pieces, from the schema:
//! - [`Bevel`] (`CT_Bevel`) — a rounded/chamfered edge: a size (`@w`/`@h`) and a [`BevelPreset`]
//!   profile. `a:sp3d` carries two, a top (`a:bevelT`) and a bottom (`a:bevelB`); a table `a:cell3D`
//!   carries one.
//! - [`LightRig`] (`CT_LightRig`) — how the scene is lit: a [`LightRigType`] and a
//!   [`LightRigDirection`], optionally rotated ([`SphereCoordinates`]).
//! - [`Camera`] (`CT_Camera`) — how it is viewed: a [`PresetCamera`] view, an optional field of view
//!   and zoom, optionally rotated.
//! - [`SphereCoordinates`] (`CT_SphereCoords`) — a latitude/longitude/revolution rotation, shared by
//!   the camera and the light rig.
//!
//! Every measure follows the rest of this crate: an unstated attribute reads `None`, distinct from
//! the schema default, so a caller can tell "unset" from "zero". A 1:1 mirror of [`crate::effect`].

use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml};

use crate::build::{
    attr_angle, attr_emu, attr_fraction, attr_str, dml_attr, dml_child, dml_element,
    fidelity_element_impls, push_angle, push_emu, push_fraction,
};
use crate::color::{Color, ColorSpec};
use crate::geometry::{Angle, Emu, Fraction};

pub use mjx_ooxml_types::drawingml::{
    BevelPreset, LightRigDirection, LightRigType, PresetCamera, PresetMaterial,
};

// ---------------------------------------------------------------------------------------------
// Value types (interner-free)
// ---------------------------------------------------------------------------------------------

/// `a:bevel` / `a:bevelT` / `a:bevelB` (`CT_Bevel`) — a shaped edge profile with a size.
///
/// The wire defaults are `w`=`h`=`76200` EMU (6 pt) and `prst`=`circle`; each field is `None` when
/// the file does not state it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bevel {
    /// The bevel width (`@w`, EMU; schema default `76200`).
    pub width: Option<Emu>,
    /// The bevel height (`@h`, EMU; schema default `76200`).
    pub height: Option<Emu>,
    /// The edge profile (`@prst`; schema default `circle`).
    pub preset: Option<BevelPreset>,
}

/// `a:rot` (`CT_SphereCoords`) — a rotation in spherical coordinates, shared by the camera and the
/// light rig. All three angles are schema-required, so an absent one reads as zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphereCoordinates {
    /// The latitude (`@lat`, a positive fixed angle).
    pub latitude: Angle,
    /// The longitude (`@lon`).
    pub longitude: Angle,
    /// The revolution about the view axis (`@rev`).
    pub revolution: Angle,
}

/// `a:camera` (`CT_Camera`) — how the 3-D scene is viewed: a preset vantage, an optional field of
/// view and zoom, and an optional rotation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// The preset camera view (`@prst`, required).
    pub preset: PresetCamera,
    /// The field of view (`@fov`, an angle; optional).
    pub field_of_view: Option<Angle>,
    /// The zoom (`@zoom`, a percentage; schema default `100%`).
    pub zoom: Option<Fraction>,
    /// A rotation of the camera about the scene (`a:rot`).
    pub rotation: Option<SphereCoordinates>,
}

/// `a:lightRig` (`CT_LightRig`) — how the 3-D scene is lit: a rig, a direction, and an optional
/// rotation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightRig {
    /// The lighting rig (`@rig`, required).
    pub rig: LightRigType,
    /// The direction the light comes from (`@dir`, required).
    pub direction: LightRigDirection,
    /// A rotation of the rig about the scene (`a:rot`).
    pub rotation: Option<SphereCoordinates>,
}

// ---------------------------------------------------------------------------------------------
// Scene3D — the fidelity wrapper over `a:scene3d`
// ---------------------------------------------------------------------------------------------

/// `a:scene3d` (`CT_Scene3D`) — the 3-D scene a shape sits in: a camera and a light rig (both
/// schema-required), an optional backdrop, and extensions.
///
/// A fidelity wrapper: the `a:camera` and `a:lightRig` are read typed; the rarer `a:backdrop` and
/// any `extLst` stay opaque and re-emit verbatim, so the element round-trips byte-for-byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scene3D {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl Scene3D {
    /// The scene's camera (`a:camera`), or `None` if it is absent or states no preset view.
    #[must_use]
    pub fn camera(&self, interner: &Interner) -> Option<Camera> {
        dml_child(&self.children, interner, "camera").and_then(|el| read_camera(el, interner))
    }

    /// The scene's light rig (`a:lightRig`), or `None` if it is absent or states no rig / direction.
    #[must_use]
    pub fn light_rig(&self, interner: &Interner) -> Option<LightRig> {
        dml_child(&self.children, interner, "lightRig").and_then(|el| read_light_rig(el, interner))
    }

    /// This scene as an interner-free [`Scene3DSpec`], or `None` if it is missing either
    /// schema-required part — a scene without a camera or a light rig is not one this describes.
    #[must_use]
    pub fn spec(&self, interner: &Interner) -> Option<Scene3DSpec> {
        Some(Scene3DSpec {
            camera: self.camera(interner)?,
            light_rig: self.light_rig(interner)?,
        })
    }
}

fidelity_element_impls!(Scene3D);

/// An interner-free description of a shape's 3-D scene (`a:scene3d`) — the camera and light rig an
/// interner-less caller reads and writes. Convert with [`Scene3D::spec`] /
/// [`Scene3DSpec::to_scene_3d`]. Rebuilding from a spec drops opaque internals (`a:backdrop`,
/// `extLst`); to preserve those, keep the [`Scene3D`] itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scene3DSpec {
    /// The camera (`a:camera`).
    pub camera: Camera,
    /// The light rig (`a:lightRig`).
    pub light_rig: LightRig,
}

impl Scene3DSpec {
    /// Builds the fidelity [`Scene3D`] for this description, interning against `interner`.
    #[must_use]
    pub fn to_scene_3d(&self, interner: &mut Interner) -> Scene3D {
        let children = vec![
            RawNode::Element(build_camera(interner, &self.camera)),
            RawNode::Element(build_light_rig(interner, &self.light_rig)),
        ];
        let element = dml_element(interner, "scene3d", Vec::new(), children);
        Scene3D::from_xml(&element, interner).expect("built scene3d is well-formed")
    }
}

// ---------------------------------------------------------------------------------------------
// Shape3D — the fidelity wrapper over `a:sp3d`
// ---------------------------------------------------------------------------------------------

/// `a:sp3d` (`CT_Shape3D`) — the shape's own 3-D properties: how far it stands off the scene floor
/// (`@z`), how thick its extrusion (`@extrusionH`) and contour (`@contourW`) are, the material its
/// surface imitates (`@prstMaterial`), a top and bottom [`Bevel`], and the extrusion / contour
/// colors.
///
/// A fidelity wrapper: every modeled facet is read typed; an `extLst` stays opaque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape3D {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl Shape3D {
    /// How far the shape stands off the scene floor (`@z`, EMU; schema default `0`).
    #[must_use]
    pub fn z(&self, interner: &Interner) -> Option<Emu> {
        attr_emu(&self.attributes, interner, "z")
    }

    /// The extrusion (depth) height (`@extrusionH`, EMU; schema default `0`).
    #[must_use]
    pub fn extrusion_height(&self, interner: &Interner) -> Option<Emu> {
        attr_emu(&self.attributes, interner, "extrusionH")
    }

    /// The contour (edge outline) width (`@contourW`, EMU; schema default `0`).
    #[must_use]
    pub fn contour_width(&self, interner: &Interner) -> Option<Emu> {
        attr_emu(&self.attributes, interner, "contourW")
    }

    /// The material the surface imitates (`@prstMaterial`; schema default `warmMatte`).
    #[must_use]
    pub fn material(&self, interner: &Interner) -> Option<PresetMaterial> {
        attr_str(&self.attributes, interner, "prstMaterial").and_then(PresetMaterial::from_wire)
    }

    /// The top bevel (`a:bevelT`), or `None` if absent.
    #[must_use]
    pub fn bevel_top(&self, interner: &Interner) -> Option<Bevel> {
        dml_child(&self.children, interner, "bevelT").map(|el| read_bevel(el, interner))
    }

    /// The bottom bevel (`a:bevelB`), or `None` if absent.
    #[must_use]
    pub fn bevel_bottom(&self, interner: &Interner) -> Option<Bevel> {
        dml_child(&self.children, interner, "bevelB").map(|el| read_bevel(el, interner))
    }

    /// The extrusion color (`a:extrusionClr`'s `EG_ColorChoice`), or `None` if absent.
    #[must_use]
    pub fn extrusion_color(&self, interner: &Interner) -> Option<ColorSpec> {
        color_child(&self.children, interner, "extrusionClr")
    }

    /// The contour color (`a:contourClr`'s `EG_ColorChoice`), or `None` if absent.
    #[must_use]
    pub fn contour_color(&self, interner: &Interner) -> Option<ColorSpec> {
        color_child(&self.children, interner, "contourClr")
    }

    /// This shape's 3-D properties as an interner-free [`Shape3DSpec`]. Rebuilding from the spec drops
    /// any opaque `extLst`.
    #[must_use]
    pub fn spec(&self, interner: &Interner) -> Shape3DSpec {
        Shape3DSpec {
            z: self.z(interner),
            extrusion_height: self.extrusion_height(interner),
            contour_width: self.contour_width(interner),
            material: self.material(interner),
            bevel_top: self.bevel_top(interner),
            bevel_bottom: self.bevel_bottom(interner),
            extrusion_color: self.extrusion_color(interner),
            contour_color: self.contour_color(interner),
        }
    }
}

fidelity_element_impls!(Shape3D);

/// An interner-free description of a shape's 3-D properties (`a:sp3d`) — the friendly value an
/// interner-less caller reads and writes. Convert with [`Shape3D::spec`] /
/// [`Shape3DSpec::to_shape_3d`]. Rebuilding from a spec drops any opaque `extLst`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Shape3DSpec {
    /// How far the shape stands off the scene floor (`@z`).
    pub z: Option<Emu>,
    /// The extrusion height (`@extrusionH`).
    pub extrusion_height: Option<Emu>,
    /// The contour width (`@contourW`).
    pub contour_width: Option<Emu>,
    /// The surface material (`@prstMaterial`).
    pub material: Option<PresetMaterial>,
    /// The top bevel (`a:bevelT`).
    pub bevel_top: Option<Bevel>,
    /// The bottom bevel (`a:bevelB`).
    pub bevel_bottom: Option<Bevel>,
    /// The extrusion color (`a:extrusionClr`).
    pub extrusion_color: Option<ColorSpec>,
    /// The contour color (`a:contourClr`).
    pub contour_color: Option<ColorSpec>,
}

impl Shape3DSpec {
    /// An empty set of 3-D properties — the same as [`Shape3DSpec::default`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds the fidelity [`Shape3D`] for this description, interning against `interner`. The
    /// attributes and children are written in `CT_Shape3D`'s schema order.
    #[must_use]
    pub fn to_shape_3d(&self, interner: &mut Interner) -> Shape3D {
        let mut attrs = Vec::new();
        push_emu(&mut attrs, interner, "z", self.z);
        push_emu(&mut attrs, interner, "extrusionH", self.extrusion_height);
        push_emu(&mut attrs, interner, "contourW", self.contour_width);
        if let Some(material) = self.material {
            attrs.push(dml_attr(interner, "prstMaterial", material.to_wire()));
        }

        let mut children = Vec::new();
        if let Some(bevel) = self.bevel_top {
            children.push(RawNode::Element(build_bevel(interner, "bevelT", &bevel)));
        }
        if let Some(bevel) = self.bevel_bottom {
            children.push(RawNode::Element(build_bevel(interner, "bevelB", &bevel)));
        }
        push_color_child(
            &mut children,
            interner,
            "extrusionClr",
            &self.extrusion_color,
        );
        push_color_child(&mut children, interner, "contourClr", &self.contour_color);

        let element = dml_element(interner, "sp3d", attrs, children);
        Shape3D::from_xml(&element, interner).expect("built sp3d is well-formed")
    }
}

// ---------------------------------------------------------------------------------------------
// Reading value types
// ---------------------------------------------------------------------------------------------

/// Reads a `CT_Bevel` element (`a:bevel` / `a:bevelT` / `a:bevelB`). Every field is optional, so a
/// bare `<a:bevelT/>` is a valid bevel that states nothing.
fn read_bevel(element: &RawElement, interner: &Interner) -> Bevel {
    Bevel {
        width: attr_emu(&element.attributes, interner, "w"),
        height: attr_emu(&element.attributes, interner, "h"),
        preset: attr_str(&element.attributes, interner, "prst").and_then(BevelPreset::from_wire),
    }
}

/// Reads an `a:rot` (`CT_SphereCoords`). The three angles are schema-required; an absent one reads
/// as zero rather than failing, since a malformed rotation must still leave the file readable.
fn read_sphere_coordinates(element: &RawElement, interner: &Interner) -> SphereCoordinates {
    let angle = |name| attr_angle(&element.attributes, interner, name).unwrap_or(ZERO_ANGLE);
    SphereCoordinates {
        latitude: angle("lat"),
        longitude: angle("lon"),
        revolution: angle("rev"),
    }
}

/// Reads an `a:camera` (`CT_Camera`), or `None` if it states no preset view (the one required field).
fn read_camera(element: &RawElement, interner: &Interner) -> Option<Camera> {
    let preset =
        attr_str(&element.attributes, interner, "prst").and_then(PresetCamera::from_wire)?;
    Some(Camera {
        preset,
        field_of_view: attr_angle(&element.attributes, interner, "fov"),
        zoom: attr_fraction(&element.attributes, interner, "zoom"),
        rotation: dml_child(&element.children, interner, "rot")
            .map(|rot| read_sphere_coordinates(rot, interner)),
    })
}

/// Reads an `a:lightRig` (`CT_LightRig`), or `None` if it states no rig or no direction (both
/// required).
fn read_light_rig(element: &RawElement, interner: &Interner) -> Option<LightRig> {
    let rig = attr_str(&element.attributes, interner, "rig").and_then(LightRigType::from_wire)?;
    let direction =
        attr_str(&element.attributes, interner, "dir").and_then(LightRigDirection::from_wire)?;
    Some(LightRig {
        rig,
        direction,
        rotation: dml_child(&element.children, interner, "rot")
            .map(|rot| read_sphere_coordinates(rot, interner)),
    })
}

/// The `EG_ColorChoice` inside a named color-wrapper child (`a:extrusionClr` / `a:contourClr`), as a
/// [`ColorSpec`].
fn color_child(children: &[RawNode], interner: &Interner, local: &str) -> Option<ColorSpec> {
    let wrapper = dml_child(children, interner, local)?;
    crate::build::first_color_child(wrapper, interner).map(|color| color.spec(interner))
}

// ---------------------------------------------------------------------------------------------
// Building value types
// ---------------------------------------------------------------------------------------------

/// Builds a `CT_Bevel` element with the given local name, writing only the attributes that are set.
fn build_bevel(interner: &mut Interner, local: &str, bevel: &Bevel) -> RawElement {
    let mut attrs = Vec::new();
    push_emu(&mut attrs, interner, "w", bevel.width);
    push_emu(&mut attrs, interner, "h", bevel.height);
    if let Some(preset) = bevel.preset {
        attrs.push(dml_attr(interner, "prst", preset.to_wire()));
    }
    dml_element(interner, local, attrs, Vec::new())
}

/// Builds an `a:rot` (`CT_SphereCoords`) — all three angles, since the schema requires them.
fn build_sphere_coordinates(interner: &mut Interner, rot: &SphereCoordinates) -> RawElement {
    let mut attrs = Vec::new();
    push_angle(&mut attrs, interner, "lat", Some(rot.latitude));
    push_angle(&mut attrs, interner, "lon", Some(rot.longitude));
    push_angle(&mut attrs, interner, "rev", Some(rot.revolution));
    dml_element(interner, "rot", attrs, Vec::new())
}

/// Builds an `a:camera` (`CT_Camera`).
fn build_camera(interner: &mut Interner, camera: &Camera) -> RawElement {
    let mut attrs = vec![dml_attr(interner, "prst", camera.preset.to_wire())];
    push_angle(&mut attrs, interner, "fov", camera.field_of_view);
    push_fraction(&mut attrs, interner, "zoom", camera.zoom);
    let children = camera
        .rotation
        .map(|rot| vec![RawNode::Element(build_sphere_coordinates(interner, &rot))])
        .unwrap_or_default();
    dml_element(interner, "camera", attrs, children)
}

/// Builds an `a:lightRig` (`CT_LightRig`).
fn build_light_rig(interner: &mut Interner, light_rig: &LightRig) -> RawElement {
    let attrs = vec![
        dml_attr(interner, "rig", light_rig.rig.to_wire()),
        dml_attr(interner, "dir", light_rig.direction.to_wire()),
    ];
    let children = light_rig
        .rotation
        .map(|rot| vec![RawNode::Element(build_sphere_coordinates(interner, &rot))])
        .unwrap_or_default();
    dml_element(interner, "lightRig", attrs, children)
}

/// Pushes a named color-wrapper child (`a:extrusionClr` / `a:contourClr`) holding `color`'s
/// `EG_ColorChoice`, when the color is set and representable.
fn push_color_child(
    children: &mut Vec<RawNode>,
    interner: &mut Interner,
    local: &str,
    color: &Option<ColorSpec>,
) {
    let Some(color) = color else { return };
    let Some(color) = Color::from_spec(interner, color) else {
        return;
    };
    let choice = RawNode::Element(color.to_xml(interner));
    children.push(RawNode::Element(dml_element(
        interner,
        local,
        Vec::new(),
        vec![choice],
    )));
}

/// Zero degrees — the value an absent required sphere angle reads as.
const ZERO_ANGLE: Angle = Angle::from_radians(0.0);
