//! The 3-D scene and shape properties: camera, lighting, bevels and extrusion.
//!
//! `a:scene3d` says where the viewer stands and where the light comes from; `a:sp3d` says how thick
//! the shape is and what its edges look like. Table cells carry the same two, which is why
//! [`crate::tables::CellFormat`] takes a [`Bevel`] and a [`LightRig`].

use wasm_bindgen::prelude::*;

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

#[wasm_bindgen]
impl Point3D {
    /// A point in three dimensions.
    #[wasm_bindgen(constructor)]
    pub fn new(x: &Emu, y: &Emu, z: &Emu) -> Self {
        Self(ooxml::Point3D {
            x: x.0,
            y: y.0,
            z: z.0,
        })
    }

    /// The horizontal coordinate.
    #[wasm_bindgen(getter, js_name = "x")]
    pub fn x(&self) -> Emu {
        Emu(self.0.x)
    }

    /// The vertical coordinate.
    #[wasm_bindgen(getter, js_name = "y")]
    pub fn y(&self) -> Emu {
        Emu(self.0.y)
    }

    /// The depth coordinate.
    #[wasm_bindgen(getter, js_name = "z")]
    pub fn z(&self) -> Emu {
        Emu(self.0.z)
    }
}

#[wasm_bindgen]
impl Vector3D {
    /// A direction in three dimensions.
    #[wasm_bindgen(constructor)]
    pub fn new(x: &Emu, y: &Emu, z: &Emu) -> Self {
        Self(ooxml::Vector3D {
            x: x.0,
            y: y.0,
            z: z.0,
        })
    }

    /// The horizontal component.
    #[wasm_bindgen(getter, js_name = "x")]
    pub fn x(&self) -> Emu {
        Emu(self.0.x)
    }

    /// The vertical component.
    #[wasm_bindgen(getter, js_name = "y")]
    pub fn y(&self) -> Emu {
        Emu(self.0.y)
    }

    /// The depth component.
    #[wasm_bindgen(getter, js_name = "z")]
    pub fn z(&self) -> Emu {
        Emu(self.0.z)
    }
}

#[wasm_bindgen]
impl SphereCoordinates {
    /// A rotation about the three axes.
    #[wasm_bindgen(constructor)]
    pub fn new(latitude: &Angle, longitude: &Angle, revolution: &Angle) -> Self {
        Self(ooxml::SphereCoordinates {
            latitude: latitude.0,
            longitude: longitude.0,
            revolution: revolution.0,
        })
    }

    /// The rotation about the horizontal axis.
    #[wasm_bindgen(getter, js_name = "latitude")]
    pub fn latitude(&self) -> Angle {
        Angle(self.0.latitude)
    }

    /// The rotation about the vertical axis.
    #[wasm_bindgen(getter, js_name = "longitude")]
    pub fn longitude(&self) -> Angle {
        Angle(self.0.longitude)
    }

    /// The rotation about the view axis.
    #[wasm_bindgen(getter, js_name = "revolution")]
    pub fn revolution(&self) -> Angle {
        Angle(self.0.revolution)
    }
}

#[wasm_bindgen]
impl Camera {
    /// A camera: one of the sixty-two presets, optionally overridden.
    #[wasm_bindgen(constructor)]
    pub fn new(
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
    #[wasm_bindgen(getter, js_name = "preset")]
    pub fn preset(&self) -> Result<PresetCamera, JsValue> {
        PresetCamera::from_model(self.0.preset)
    }

    /// The field of view, when stated.
    #[wasm_bindgen(getter, js_name = "fieldOfView")]
    pub fn field_of_view(&self) -> Option<Angle> {
        self.0.field_of_view.map(Angle)
    }

    /// The zoom, when stated.
    #[wasm_bindgen(getter, js_name = "zoom")]
    pub fn zoom(&self) -> Option<Fraction> {
        self.0.zoom.map(Fraction)
    }

    /// The rotation, when stated.
    #[wasm_bindgen(getter, js_name = "rotation")]
    pub fn rotation(&self) -> Option<SphereCoordinates> {
        self.0.rotation.map(SphereCoordinates)
    }
}

#[wasm_bindgen]
impl LightRig {
    /// A light rig: which lighting, from which direction, optionally rotated.
    #[wasm_bindgen(constructor)]
    pub fn new(
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
    #[wasm_bindgen(getter, js_name = "rig")]
    pub fn rig(&self) -> Result<LightRigType, JsValue> {
        LightRigType::from_model(self.0.rig)
    }

    /// Which of the eight directions the light comes from.
    #[wasm_bindgen(getter, js_name = "direction")]
    pub fn direction(&self) -> Result<LightRigDirection, JsValue> {
        LightRigDirection::from_model(self.0.direction)
    }

    /// The rig's own rotation, when stated.
    #[wasm_bindgen(getter, js_name = "rotation")]
    pub fn rotation(&self) -> Option<SphereCoordinates> {
        self.0.rotation.map(SphereCoordinates)
    }
}

#[wasm_bindgen]
impl Backdrop {
    /// The plane a 3-D scene sits on: a point on it, and two directions that orient it.
    #[wasm_bindgen(constructor)]
    pub fn new(anchor: &Point3D, normal: &Vector3D, up: &Vector3D) -> Self {
        Self(ooxml::Backdrop {
            anchor: anchor.0,
            normal: normal.0,
            up: up.0,
        })
    }

    /// A point on the plane.
    #[wasm_bindgen(getter, js_name = "anchor")]
    pub fn anchor(&self) -> Point3D {
        Point3D(self.0.anchor)
    }

    /// The direction perpendicular to the plane.
    #[wasm_bindgen(getter, js_name = "normal")]
    pub fn normal(&self) -> Vector3D {
        Vector3D(self.0.normal)
    }

    /// The direction that is "up" on the plane.
    #[wasm_bindgen(getter, js_name = "up")]
    pub fn up(&self) -> Vector3D {
        Vector3D(self.0.up)
    }
}

#[wasm_bindgen]
impl Bevel {
    /// A bevel: how wide, how deep, and which of the twelve shapes.
    #[wasm_bindgen(constructor)]
    pub fn new(width: Option<Emu>, height: Option<Emu>, preset: Option<BevelPreset>) -> Self {
        Self(ooxml::Bevel {
            width: width.map(|value| value.0),
            height: height.map(|value| value.0),
            preset: preset.map(Into::into),
        })
    }

    /// The bevel's width, when stated.
    #[wasm_bindgen(getter, js_name = "width")]
    pub fn width(&self) -> Option<Emu> {
        self.0.width.map(Emu)
    }

    /// The bevel's height, when stated.
    #[wasm_bindgen(getter, js_name = "height")]
    pub fn height(&self) -> Option<Emu> {
        self.0.height.map(Emu)
    }

    /// Which of the twelve bevel shapes, when stated.
    #[wasm_bindgen(getter, js_name = "preset")]
    pub fn preset(&self) -> Result<Option<BevelPreset>, JsValue> {
        self.0.preset.map(BevelPreset::from_model).transpose()
    }
}

#[wasm_bindgen]
impl Scene3DSpec {
    /// A 3-D scene: where the viewer stands and where the light comes from.
    #[wasm_bindgen(constructor)]
    pub fn new(camera: &Camera, light_rig: &LightRig) -> Self {
        Self(ooxml::Scene3DSpec {
            camera: camera.0,
            light_rig: light_rig.0,
        })
    }

    /// The camera.
    #[wasm_bindgen(getter, js_name = "camera")]
    pub fn camera(&self) -> Camera {
        Camera(self.0.camera)
    }

    /// The light rig.
    #[wasm_bindgen(getter, js_name = "lightRig")]
    pub fn light_rig(&self) -> LightRig {
        LightRig(self.0.light_rig)
    }
}

#[wasm_bindgen]
impl Shape3DSpec {
    /// A shape's 3-D properties. Everything is optional; a shape that states none is flat.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
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
    #[wasm_bindgen(getter, js_name = "z")]
    pub fn z(&self) -> Option<Emu> {
        self.0.z.map(Emu)
    }

    /// How thick the extrusion is, when stated.
    #[wasm_bindgen(getter, js_name = "extrusionHeight")]
    pub fn extrusion_height(&self) -> Option<Emu> {
        self.0.extrusion_height.map(Emu)
    }

    /// How wide the contour is, when stated.
    #[wasm_bindgen(getter, js_name = "contourWidth")]
    pub fn contour_width(&self) -> Option<Emu> {
        self.0.contour_width.map(Emu)
    }

    /// Which of the fifteen surface materials, when stated.
    #[wasm_bindgen(getter, js_name = "material")]
    pub fn material(&self) -> Result<Option<PresetMaterial>, JsValue> {
        self.0.material.map(PresetMaterial::from_model).transpose()
    }

    /// The top bevel, when stated.
    #[wasm_bindgen(getter, js_name = "bevelTop")]
    pub fn bevel_top(&self) -> Option<Bevel> {
        self.0.bevel_top.map(Bevel)
    }

    /// The bottom bevel, when stated.
    #[wasm_bindgen(getter, js_name = "bevelBottom")]
    pub fn bevel_bottom(&self) -> Option<Bevel> {
        self.0.bevel_bottom.map(Bevel)
    }

    /// The extrusion's colour, when stated.
    #[wasm_bindgen(getter, js_name = "extrusionColor")]
    pub fn extrusion_color(&self) -> Option<ColorSpec> {
        self.0.extrusion_color.clone().map(ColorSpec)
    }

    /// The contour's colour, when stated.
    #[wasm_bindgen(getter, js_name = "contourColor")]
    pub fn contour_color(&self) -> Option<ColorSpec> {
        self.0.contour_color.clone().map(ColorSpec)
    }
}
