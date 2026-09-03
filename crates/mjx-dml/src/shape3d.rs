//! DrawingML 3-D: `a:scene3d` (`CT_Scene3D`) and `a:sp3d` (`CT_Shape3D`) — the 3-D scene a shape is
//! lit and viewed in, and the extrusion, bevels and material that give its face depth.
//!
//! [`Scene3D`] and [`Shape3D`] are **fidelity wrappers** over their elements (name, attributes,
//! children and self-closing flag preserved verbatim); the modeled facets are read through typed
//! accessors, while an unmodeled child (`extLst`, an MCE bucket) stays opaque so the element
//! round-trips byte-for-byte. [`Scene3DSpec`] / [`Shape3DSpec`] are the interner-free values
//! `mjx-pptx`'s `shape_scene_3d` / `shape_3d_properties` read and write.
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
//! - [`Backdrop`] (`CT_Backdrop`) — the plane the scene's shadows and reflections fall on: a
//!   [`Point3D`] anchor and two [`Vector3D`]s, the plane's normal and its up direction.
//!
//! Every measure follows the rest of this crate: an unstated attribute reads `None`, distinct from
//! the schema default, so a caller can tell "unset" from "zero". A 1:1 mirror of [`crate::effect`].

use mjx_ooxml_core::{
    Enumeration, FromXml, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml,
};

use crate::build::{dml_child, dml_element, fidelity_element_impls};
use crate::codec::{EmuCoordinate, Percentage, SixtyThousandthsOfADegree};
use crate::color::{Color, ColorSpec};
use crate::geometry::{Angle, Emu, Fraction};

pub use mjx_ooxml_types::drawingml::{
    BevelPreset, LightRigDirection, LightRigType, PresetCamera, PresetMaterial,
};

// ---------------------------------------------------------------------------------------------
// The attribute faces of the 3-D child elements
// ---------------------------------------------------------------------------------------------
//
// A bevel, a rotation, a camera, a light rig, a point and a vector are projections out of `a:scene3d`
// and `a:sp3d`'s children, not modeled types. Each declares its attributes through the
// `#[xml(attribute(..))]` grammar over the vector it is handed — borrowed to read, a fresh one to
// write — so one declaration serves both directions and both go through the same generated accessor.
//
// A schema-*required* attribute is declared `required`, so an absent one is a typed error rather than
// a silently substituted value; where this module chooses to carry on regardless (a malformed scene
// must still leave the file readable) it says so at the call site, in one place.

/// `a:bevel` / `a:bevelT` / `a:bevelB` (`CT_Bevel`).
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "w", codec = EmuCoordinate, accessor = width))]
#[xml(attribute(local = "h", codec = EmuCoordinate, accessor = height))]
#[xml(attribute(local = "prst", codec = Enumeration<BevelPreset>, accessor = preset))]
struct BevelAttributes<A> {
    attributes: A,
}

/// `a:rot` (`CT_SphereCoords`) — all three angles are schema-required.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "lat", codec = SixtyThousandthsOfADegree, accessor = latitude, required))]
#[xml(attribute(local = "lon", codec = SixtyThousandthsOfADegree, accessor = longitude, required))]
#[xml(attribute(local = "rev", codec = SixtyThousandthsOfADegree, accessor = revolution, required))]
struct SphereCoordinatesAttributes<A> {
    attributes: A,
}

/// `a:camera` (`CT_Camera`).
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "prst", codec = Enumeration<PresetCamera>, accessor = preset, required))]
#[xml(attribute(local = "fov", codec = SixtyThousandthsOfADegree, accessor = field_of_view))]
#[xml(attribute(local = "zoom", codec = Percentage, accessor = zoom))]
struct CameraAttributes<A> {
    attributes: A,
}

/// `a:lightRig` (`CT_LightRig`) — both attributes are schema-required.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "rig", codec = Enumeration<LightRigType>, accessor = rig, required))]
#[xml(attribute(local = "dir", codec = Enumeration<LightRigDirection>, accessor = direction, required))]
struct LightRigAttributes<A> {
    attributes: A,
}

/// `a:anchor` (`CT_Point3D`) — all three coordinates are schema-required.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "x", codec = EmuCoordinate, accessor = x, required))]
#[xml(attribute(local = "y", codec = EmuCoordinate, accessor = y, required))]
#[xml(attribute(local = "z", codec = EmuCoordinate, accessor = z, required))]
struct Point3DAttributes<A> {
    attributes: A,
}

/// `a:norm` / `a:up` (`CT_Vector3D`) — all three components are schema-required.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "dx", codec = EmuCoordinate, accessor = dx, required))]
#[xml(attribute(local = "dy", codec = EmuCoordinate, accessor = dy, required))]
#[xml(attribute(local = "dz", codec = EmuCoordinate, accessor = dz, required))]
struct Vector3DAttributes<A> {
    attributes: A,
}

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

/// `a:anchor` (`CT_Point3D`) — a point in the scene's 3-D space. All three coordinates are
/// schema-required, so an absent one reads as zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point3D {
    /// The horizontal coordinate (`@x`, EMU).
    pub x: Emu,
    /// The vertical coordinate (`@y`, EMU).
    pub y: Emu,
    /// The depth coordinate (`@z`, EMU).
    pub z: Emu,
}

impl Default for Point3D {
    /// The scene origin — what an absent (schema-required) `a:anchor` reads as.
    fn default() -> Self {
        Self {
            x: Emu::from_emu(0),
            y: Emu::from_emu(0),
            z: Emu::from_emu(0),
        }
    }
}

/// `a:norm` / `a:up` (`CT_Vector3D`) — a direction in the scene's 3-D space, as the three components
/// of a vector. All three are schema-required, so an absent one reads as zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vector3D {
    /// The horizontal component (`@dx`, EMU).
    pub x: Emu,
    /// The vertical component (`@dy`, EMU).
    pub y: Emu,
    /// The depth component (`@dz`, EMU).
    pub z: Emu,
}

impl Default for Vector3D {
    /// The zero vector — what an absent (schema-required) `a:norm` / `a:up` reads as.
    fn default() -> Self {
        Self {
            x: Emu::from_emu(0),
            y: Emu::from_emu(0),
            z: Emu::from_emu(0),
        }
    }
}

/// `a:backdrop` (`CT_Backdrop`) — the plane a 3-D scene's shadows and reflections are cast on,
/// given as a point on the plane and the two directions that orient it.
///
/// All three children are schema-required. Read through [`Scene3D::backdrop`]; the element itself
/// stays verbatim in the [`Scene3D`] wrapper, so reading it never changes what is written back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Backdrop {
    /// A point the plane passes through (`a:anchor`).
    pub anchor: Point3D,
    /// The plane's normal — the direction it faces (`a:norm`).
    pub normal: Vector3D,
    /// The plane's up direction (`a:up`).
    pub up: Vector3D,
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

    /// The plane the scene's shadows and reflections fall on (`a:backdrop`), or `None` if the scene
    /// states none — as almost every scene does, the element being optional and rare.
    #[must_use]
    pub fn backdrop(&self, interner: &Interner) -> Option<Backdrop> {
        let backdrop = dml_child(&self.children, interner, "backdrop")?;
        Some(Backdrop {
            anchor: dml_child(&backdrop.children, interner, "anchor")
                .map(|anchor| {
                    let point = Point3DAttributes {
                        attributes: &anchor.attributes,
                    };
                    // Schema-required, but a malformed backdrop must still leave the file readable,
                    // so an unstated coordinate is the origin rather than a rejected element.
                    Point3D {
                        x: point.x(interner).unwrap_or(ORIGIN),
                        y: point.y(interner).unwrap_or(ORIGIN),
                        z: point.z(interner).unwrap_or(ORIGIN),
                    }
                })
                .unwrap_or_default(),
            normal: read_vector(backdrop, interner, "norm"),
            up: read_vector(backdrop, interner, "up"),
        })
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
/// [`Scene3DSpec::to_scene_3d`]. Rebuilding from a spec drops the rarer internals (the
/// [`Backdrop`](Scene3D::backdrop), `extLst`); to preserve those — the usual case, since a read
/// [`Scene3D`] holds them verbatim — keep the [`Scene3D`] itself.
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
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "z", codec = EmuCoordinate, accessor = z))]
#[xml(attribute(local = "extrusionH", codec = EmuCoordinate, accessor = extrusion_height))]
#[xml(attribute(local = "contourW", codec = EmuCoordinate, accessor = contour_width))]
#[xml(attribute(local = "prstMaterial", codec = Enumeration<PresetMaterial>, accessor = material))]
pub struct Shape3D {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl Shape3D {
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
            z: self.z(interner).ok().flatten(),
            extrusion_height: self.extrusion_height(interner).ok().flatten(),
            contour_width: self.contour_width(interner).ok().flatten(),
            material: self.material(interner).ok().flatten(),
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

        let element = dml_element(interner, "sp3d", Vec::new(), children);
        let mut shape = Shape3D::from_xml(&element, interner).expect("built sp3d is well-formed");
        // `CT_Shape3D`'s schema order, which is the order these four setters append in.
        shape.set_z(interner, self.z);
        shape.set_extrusion_height(interner, self.extrusion_height);
        shape.set_contour_width(interner, self.contour_width);
        shape.set_material(interner, self.material);
        shape
    }
}

// ---------------------------------------------------------------------------------------------
// Reading value types
// ---------------------------------------------------------------------------------------------

/// Reads a `CT_Bevel` element (`a:bevel` / `a:bevelT` / `a:bevelB`). Every field is optional, so a
/// bare `<a:bevelT/>` is a valid bevel that states nothing. `pub(crate)` so the table `a:cell3D`
/// reuses it.
pub(crate) fn read_bevel(element: &RawElement, interner: &Interner) -> Bevel {
    let bevel = BevelAttributes {
        attributes: &element.attributes,
    };
    Bevel {
        width: bevel.width(interner).ok().flatten(),
        height: bevel.height(interner).ok().flatten(),
        preset: bevel.preset(interner).ok().flatten(),
    }
}

/// Reads one `a:norm` / `a:up` child of a backdrop (`CT_Vector3D`); an absent one reads as the zero
/// vector rather than failing, since a malformed backdrop must still leave the file readable.
fn read_vector(backdrop: &RawElement, interner: &Interner, local: &str) -> Vector3D {
    dml_child(&backdrop.children, interner, local)
        .map(|vector| {
            let components = Vector3DAttributes {
                attributes: &vector.attributes,
            };
            Vector3D {
                x: components.dx(interner).unwrap_or(ORIGIN),
                y: components.dy(interner).unwrap_or(ORIGIN),
                z: components.dz(interner).unwrap_or(ORIGIN),
            }
        })
        .unwrap_or_default()
}

/// Reads an `a:rot` (`CT_SphereCoords`). The three angles are schema-required; an absent one reads
/// as zero rather than failing, since a malformed rotation must still leave the file readable.
fn read_sphere_coordinates(element: &RawElement, interner: &Interner) -> SphereCoordinates {
    let rotation = SphereCoordinatesAttributes {
        attributes: &element.attributes,
    };
    SphereCoordinates {
        latitude: rotation.latitude(interner).unwrap_or(ZERO_ANGLE),
        longitude: rotation.longitude(interner).unwrap_or(ZERO_ANGLE),
        revolution: rotation.revolution(interner).unwrap_or(ZERO_ANGLE),
    }
}

/// Reads an `a:camera` (`CT_Camera`), or `None` if it states no preset view (the one required field).
fn read_camera(element: &RawElement, interner: &Interner) -> Option<Camera> {
    let camera = CameraAttributes {
        attributes: &element.attributes,
    };
    Some(Camera {
        preset: camera.preset(interner).ok()?,
        field_of_view: camera.field_of_view(interner).ok().flatten(),
        zoom: camera.zoom(interner).ok().flatten(),
        rotation: dml_child(&element.children, interner, "rot")
            .map(|rot| read_sphere_coordinates(rot, interner)),
    })
}

/// Reads an `a:lightRig` (`CT_LightRig`), or `None` if it states no rig or no direction (both
/// required). `pub(crate)` so the table `a:cell3D` reuses it.
pub(crate) fn read_light_rig(element: &RawElement, interner: &Interner) -> Option<LightRig> {
    let light_rig = LightRigAttributes {
        attributes: &element.attributes,
    };
    Some(LightRig {
        rig: light_rig.rig(interner).ok()?,
        direction: light_rig.direction(interner).ok()?,
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
/// `pub(crate)` so the table `a:cell3D` reuses it (with `local` = `"bevel"`).
pub(crate) fn build_bevel(interner: &mut Interner, local: &str, bevel: &Bevel) -> RawElement {
    let mut attributes = BevelAttributes {
        attributes: Vec::new(),
    };
    attributes.set_width(interner, bevel.width);
    attributes.set_height(interner, bevel.height);
    attributes.set_preset(interner, bevel.preset);
    dml_element(interner, local, attributes.attributes, Vec::new())
}

/// Builds an `a:rot` (`CT_SphereCoords`) — all three angles, since the schema requires them.
fn build_sphere_coordinates(interner: &mut Interner, rot: &SphereCoordinates) -> RawElement {
    let mut attributes = SphereCoordinatesAttributes {
        attributes: Vec::new(),
    };
    attributes.set_latitude(interner, rot.latitude);
    attributes.set_longitude(interner, rot.longitude);
    attributes.set_revolution(interner, rot.revolution);
    dml_element(interner, "rot", attributes.attributes, Vec::new())
}

/// Builds an `a:camera` (`CT_Camera`).
fn build_camera(interner: &mut Interner, camera: &Camera) -> RawElement {
    let mut attributes = CameraAttributes {
        attributes: Vec::new(),
    };
    attributes.set_preset(interner, camera.preset);
    attributes.set_field_of_view(interner, camera.field_of_view);
    attributes.set_zoom(interner, camera.zoom);
    let children = camera
        .rotation
        .map(|rot| vec![RawNode::Element(build_sphere_coordinates(interner, &rot))])
        .unwrap_or_default();
    dml_element(interner, "camera", attributes.attributes, children)
}

/// Builds an `a:lightRig` (`CT_LightRig`). `pub(crate)` so the table `a:cell3D` reuses it.
pub(crate) fn build_light_rig(interner: &mut Interner, light_rig: &LightRig) -> RawElement {
    let mut attributes = LightRigAttributes {
        attributes: Vec::new(),
    };
    attributes.set_rig(interner, light_rig.rig);
    attributes.set_direction(interner, light_rig.direction);
    let children = light_rig
        .rotation
        .map(|rot| vec![RawNode::Element(build_sphere_coordinates(interner, &rot))])
        .unwrap_or_default();
    dml_element(interner, "lightRig", attributes.attributes, children)
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

/// Zero EMU — the value an absent required point or vector component reads as.
const ORIGIN: Emu = Emu::from_emu(0);
