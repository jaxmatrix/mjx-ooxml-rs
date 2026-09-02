//! The 3-D scene and shape properties: camera, lighting, bevels and extrusion.
//!
//! `a:scene3d` says where the viewer stands and where the light comes from; `a:sp3d` says how thick
//! the shape is and what its edges look like. Table cells carry the same two, which is why
//! [`crate::tables::CellFormat`] takes a [`Bevel`] and a [`LightRig`].

use pyo3::prelude::*;
use pyo3::types::PyModule;

use mjx_ooxml as ooxml;

use crate::enums::{BevelPreset, LightRigDirection, LightRigType, PresetCamera, PresetMaterial};
use crate::measures::{Angle, Emu, Fraction};
use crate::paint::ColorSpec;

value_class! {
    /// A point in three dimensions, in English Metric Units.
    Point3D(ooxml::Point3D), derive(Copy, PartialEq, Eq);

    /// A direction in three dimensions.
    Vector3D(ooxml::Vector3D), derive(Copy, PartialEq, Eq);

    /// A rotation stated as latitude, longitude and revolution.
    SphereCoordinates(ooxml::SphereCoordinates), derive(Copy, PartialEq);

    /// Where the viewer stands: one of the sixty-two preset cameras, plus optional field of view,
    /// zoom and rotation.
    Camera(ooxml::Camera), derive(Copy, PartialEq);

    /// Where the light comes from: a rig, a direction, and an optional rotation.
    LightRig(ooxml::LightRig), derive(Copy, PartialEq);

    /// The plane a 3-D scene sits on.
    Backdrop(ooxml::Backdrop), derive(Copy, PartialEq, Eq);

    /// The rounded or chamfered edge of an extruded shape.
    Bevel(ooxml::Bevel), derive(Copy, PartialEq, Eq);

    /// A 3-D scene: a camera and a light rig.
    Scene3DSpec(ooxml::Scene3DSpec), derive(Copy, PartialEq);

    /// A shape's own 3-D properties: depth, extrusion, contour, material and bevels.
    Shape3DSpec(ooxml::Shape3DSpec), derive(PartialEq);
}

#[pymethods]
impl Point3D {
    /// A point in three dimensions.
    #[new]
    fn new(x: Emu, y: Emu, z: Emu) -> Self {
        Self(ooxml::Point3D {
            x: x.0,
            y: y.0,
            z: z.0,
        })
    }

    /// The horizontal coordinate.
    #[getter]
    fn x(&self) -> Emu {
        Emu(self.0.x)
    }

    /// The vertical coordinate.
    #[getter]
    fn y(&self) -> Emu {
        Emu(self.0.y)
    }

    /// The depth coordinate.
    #[getter]
    fn z(&self) -> Emu {
        Emu(self.0.z)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Vector3D {
    /// A direction in three dimensions.
    #[new]
    fn new(x: Emu, y: Emu, z: Emu) -> Self {
        Self(ooxml::Vector3D {
            x: x.0,
            y: y.0,
            z: z.0,
        })
    }

    /// The horizontal component.
    #[getter]
    fn x(&self) -> Emu {
        Emu(self.0.x)
    }

    /// The vertical component.
    #[getter]
    fn y(&self) -> Emu {
        Emu(self.0.y)
    }

    /// The depth component.
    #[getter]
    fn z(&self) -> Emu {
        Emu(self.0.z)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl SphereCoordinates {
    /// A rotation about the three axes.
    #[new]
    fn new(latitude: Angle, longitude: Angle, revolution: Angle) -> Self {
        Self(ooxml::SphereCoordinates {
            latitude: latitude.0,
            longitude: longitude.0,
            revolution: revolution.0,
        })
    }

    /// The rotation about the horizontal axis.
    #[getter]
    fn latitude(&self) -> Angle {
        Angle(self.0.latitude)
    }

    /// The rotation about the vertical axis.
    #[getter]
    fn longitude(&self) -> Angle {
        Angle(self.0.longitude)
    }

    /// The rotation about the view axis.
    #[getter]
    fn revolution(&self) -> Angle {
        Angle(self.0.revolution)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Camera {
    /// A camera: one of the sixty-two presets, optionally overridden.
    #[new]
    #[pyo3(signature = (preset, field_of_view = None, zoom = None, rotation = None))]
    fn new(
        preset: PresetCamera,
        field_of_view: Option<Angle>,
        zoom: Option<Fraction>,
        rotation: Option<SphereCoordinates>,
    ) -> Self {
        Self(ooxml::Camera {
            preset: preset.into(),
            field_of_view: field_of_view.map(|value| value.0),
            zoom: zoom.map(|value| value.0),
            rotation: rotation.map(|value| value.0),
        })
    }

    /// Which of the sixty-two preset cameras.
    #[getter]
    fn preset(&self) -> PyResult<PresetCamera> {
        PresetCamera::from_model(self.0.preset)
    }

    /// The field of view, when stated.
    #[getter]
    fn field_of_view(&self) -> Option<Angle> {
        self.0.field_of_view.map(Angle)
    }

    /// The zoom, when stated.
    #[getter]
    fn zoom(&self) -> Option<Fraction> {
        self.0.zoom.map(Fraction)
    }

    /// The rotation, when stated.
    #[getter]
    fn rotation(&self) -> Option<SphereCoordinates> {
        self.0.rotation.map(SphereCoordinates)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl LightRig {
    /// A light rig: which lighting, from which direction, optionally rotated.
    #[new]
    #[pyo3(signature = (rig, direction, rotation = None))]
    fn new(
        rig: LightRigType,
        direction: LightRigDirection,
        rotation: Option<SphereCoordinates>,
    ) -> Self {
        Self(ooxml::LightRig {
            rig: rig.into(),
            direction: direction.into(),
            rotation: rotation.map(|value| value.0),
        })
    }

    /// Which of the twenty-seven lighting rigs.
    #[getter]
    fn rig(&self) -> PyResult<LightRigType> {
        LightRigType::from_model(self.0.rig)
    }

    /// Which of the eight directions the light comes from.
    #[getter]
    fn direction(&self) -> PyResult<LightRigDirection> {
        LightRigDirection::from_model(self.0.direction)
    }

    /// The rig's own rotation, when stated.
    #[getter]
    fn rotation(&self) -> Option<SphereCoordinates> {
        self.0.rotation.map(SphereCoordinates)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Backdrop {
    /// The plane a 3-D scene sits on: a point on it, and two directions that orient it.
    #[new]
    fn new(anchor: Point3D, normal: Vector3D, up: Vector3D) -> Self {
        Self(ooxml::Backdrop {
            anchor: anchor.0,
            normal: normal.0,
            up: up.0,
        })
    }

    /// A point on the plane.
    #[getter]
    fn anchor(&self) -> Point3D {
        Point3D(self.0.anchor)
    }

    /// The direction perpendicular to the plane.
    #[getter]
    fn normal(&self) -> Vector3D {
        Vector3D(self.0.normal)
    }

    /// The direction that is "up" on the plane.
    #[getter]
    fn up(&self) -> Vector3D {
        Vector3D(self.0.up)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Bevel {
    /// A bevel: how wide, how deep, and which of the twelve shapes.
    #[new]
    #[pyo3(signature = (width = None, height = None, preset = None))]
    fn new(width: Option<Emu>, height: Option<Emu>, preset: Option<BevelPreset>) -> Self {
        Self(ooxml::Bevel {
            width: width.map(|value| value.0),
            height: height.map(|value| value.0),
            preset: preset.map(Into::into),
        })
    }

    /// The bevel's width, when stated.
    #[getter]
    fn width(&self) -> Option<Emu> {
        self.0.width.map(Emu)
    }

    /// The bevel's height, when stated.
    #[getter]
    fn height(&self) -> Option<Emu> {
        self.0.height.map(Emu)
    }

    /// Which of the twelve bevel shapes, when stated.
    #[getter]
    fn preset(&self) -> PyResult<Option<BevelPreset>> {
        self.0.preset.map(BevelPreset::from_model).transpose()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Scene3DSpec {
    /// A 3-D scene: where the viewer stands and where the light comes from.
    #[new]
    fn new(camera: Camera, light_rig: LightRig) -> Self {
        Self(ooxml::Scene3DSpec {
            camera: camera.0,
            light_rig: light_rig.0,
        })
    }

    /// The camera.
    #[getter]
    fn camera(&self) -> Camera {
        Camera(self.0.camera)
    }

    /// The light rig.
    #[getter]
    fn light_rig(&self) -> LightRig {
        LightRig(self.0.light_rig)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Shape3DSpec {
    /// A shape's 3-D properties. Everything is optional; a shape that states none is flat.
    #[new]
    #[pyo3(signature = (
        z = None,
        extrusion_height = None,
        contour_width = None,
        material = None,
        bevel_top = None,
        bevel_bottom = None,
        extrusion_color = None,
        contour_color = None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        z: Option<Emu>,
        extrusion_height: Option<Emu>,
        contour_width: Option<Emu>,
        material: Option<PresetMaterial>,
        bevel_top: Option<Bevel>,
        bevel_bottom: Option<Bevel>,
        extrusion_color: Option<ColorSpec>,
        contour_color: Option<ColorSpec>,
    ) -> Self {
        Self(ooxml::Shape3DSpec {
            z: z.map(|value| value.0),
            extrusion_height: extrusion_height.map(|value| value.0),
            contour_width: contour_width.map(|value| value.0),
            material: material.map(Into::into),
            bevel_top: bevel_top.map(|bevel| bevel.0),
            bevel_bottom: bevel_bottom.map(|bevel| bevel.0),
            extrusion_color: extrusion_color.map(|color| color.0),
            contour_color: contour_color.map(|color| color.0),
        })
    }

    /// How far the shape sits off the scene's plane, when stated.
    #[getter]
    fn z(&self) -> Option<Emu> {
        self.0.z.map(Emu)
    }

    /// How thick the extrusion is, when stated.
    #[getter]
    fn extrusion_height(&self) -> Option<Emu> {
        self.0.extrusion_height.map(Emu)
    }

    /// How wide the contour is, when stated.
    #[getter]
    fn contour_width(&self) -> Option<Emu> {
        self.0.contour_width.map(Emu)
    }

    /// Which of the fifteen surface materials, when stated.
    #[getter]
    fn material(&self) -> PyResult<Option<PresetMaterial>> {
        self.0.material.map(PresetMaterial::from_model).transpose()
    }

    /// The top bevel, when stated.
    #[getter]
    fn bevel_top(&self) -> Option<Bevel> {
        self.0.bevel_top.map(Bevel)
    }

    /// The bottom bevel, when stated.
    #[getter]
    fn bevel_bottom(&self) -> Option<Bevel> {
        self.0.bevel_bottom.map(Bevel)
    }

    /// The extrusion's colour, when stated.
    #[getter]
    fn extrusion_color(&self) -> Option<ColorSpec> {
        self.0.extrusion_color.clone().map(ColorSpec)
    }

    /// The contour's colour, when stated.
    #[getter]
    fn contour_color(&self) -> Option<ColorSpec> {
        self.0.contour_color.clone().map(ColorSpec)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// Adds every class in this module to the extension module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Point3D>()?;
    module.add_class::<Vector3D>()?;
    module.add_class::<SphereCoordinates>()?;
    module.add_class::<Camera>()?;
    module.add_class::<LightRig>()?;
    module.add_class::<Backdrop>()?;
    module.add_class::<Bevel>()?;
    module.add_class::<Scene3DSpec>()?;
    module.add_class::<Shape3DSpec>()
}
