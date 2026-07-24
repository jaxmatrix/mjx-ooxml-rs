//! Unit tests for the DrawingML 3-D model (`a:scene3d` / `a:sp3d`), through the public API only.
//!
//! Every byte-identity assertion is paired with a typed one, so a round-trip can't pass by dumping
//! everything into an opaque bucket. The two properties the model exists for get the most attention:
//! **absent is not the schema default** (an unstated `@w` / `@fov` reads `None`, not `76200` / `0`),
//! and **the opaque children survive** (`a:backdrop`, `extLst` re-emit verbatim).

use mjx_dml::{
    BevelPreset, ColorSpec, LightRigDirection, LightRigType, PresetCamera, PresetMaterial, Scene3D,
    Scene3DSpec, Shape3D, Shape3DSpec,
};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, ToXml};
use mjx_xml::fidelity;

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

fn parse_typed<T: FromXml>(fragment: &[u8]) -> (T, RawDocument) {
    let doc = fidelity::parse(fragment).expect("fragment parses");
    let typed = T::from_xml(&doc.root, &doc.interner).expect("from_xml");
    (typed, doc)
}

#[track_caller]
fn assert_round_trips<T: ToXml>(typed: &T, mut doc: RawDocument, expected: &[u8]) {
    doc.root = typed.to_xml(&mut doc.interner);
    let out = fidelity::serialize_to_vec(&doc);
    assert_eq!(
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(expected),
        "round-trip byte mismatch"
    );
}

fn serialize_built<T: ToXml>(mut interner: Interner, typed: &T) -> String {
    let root = typed.to_xml(&mut interner);
    let doc = RawDocument {
        interner,
        bom: false,
        prologue: Vec::new(),
        root,
        epilogue: Vec::new(),
    };
    String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8")
}

// ---------------------------------------------------------------------------------------------
// Every enum maps to its exact XSD token, the overridden ones especially
// ---------------------------------------------------------------------------------------------

#[test]
fn the_preset_enums_round_trip_through_their_wire_tokens() {
    // Auto-expanded tokens.
    assert_eq!(
        BevelPreset::from_wire("relaxedInset"),
        Some(BevelPreset::RelaxedInset)
    );
    assert_eq!(BevelPreset::Circle.to_wire(), "circle");
    assert_eq!(
        PresetCamera::from_wire("isometricTopUp"),
        Some(PresetCamera::IsometricTopUp)
    );
    assert_eq!(PresetMaterial::WarmMatte.to_wire(), "warmMatte");

    // The tokens that needed a value override are pinned to their exact wire form.
    assert_eq!(
        LightRigDirection::from_wire("tl"),
        Some(LightRigDirection::TopLeft)
    );
    assert_eq!(LightRigDirection::BottomRight.to_wire(), "br");
    assert_eq!(
        LightRigType::from_wire("threePt"),
        Some(LightRigType::ThreePoint)
    );
    assert_eq!(LightRigType::TwoPoint.to_wire(), "twoPt");
    assert_eq!(
        PresetMaterial::from_wire("dkEdge"),
        Some(PresetMaterial::DarkEdge)
    );
    assert_eq!(PresetMaterial::SoftMetal.to_wire(), "softmetal");
}

// ---------------------------------------------------------------------------------------------
// Scene3D — the camera and light rig read typed; the rest stays opaque
// ---------------------------------------------------------------------------------------------

const SCENE: &str = concat!(
    r#"<a:scene3d xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">"#,
    r#"<a:camera prst="orthographicFront" fov="600000" zoom="150000">"#,
    r#"<a:rot lat="1200000" lon="0" rev="5400000"/></a:camera>"#,
    r#"<a:lightRig rig="threePt" dir="t"><a:rot lat="0" lon="600000" rev="0"/></a:lightRig>"#,
    r#"<a:backdrop><a:anchor x="1" y="2" z="3"/><a:norm dx="0" dy="0" dz="1"/>"#,
    r#"<a:up dx="0" dy="1" dz="0"/></a:backdrop>"#,
    r#"</a:scene3d>"#,
);

#[test]
fn a_scene_reads_its_camera_and_light_rig() {
    let (scene, doc) = parse_typed::<Scene3D>(SCENE.as_bytes());

    let camera = scene.camera(&doc.interner).expect("a camera");
    assert_eq!(camera.preset, PresetCamera::OrthographicFront);
    assert_eq!(
        camera.field_of_view.expect("fov").degrees().round() as i64,
        10
    );
    assert!((camera.zoom.expect("zoom").ratio() - 1.5).abs() < 1e-9);
    let rot = camera.rotation.expect("camera rotation");
    assert_eq!(rot.latitude.degrees().round() as i64, 20);
    assert_eq!(rot.revolution.degrees().round() as i64, 90);

    let rig = scene.light_rig(&doc.interner).expect("a light rig");
    assert_eq!(rig.rig, LightRigType::ThreePoint);
    assert_eq!(rig.direction, LightRigDirection::Top);
    assert_eq!(
        rig.rotation
            .expect("rig rotation")
            .longitude
            .degrees()
            .round() as i64,
        10
    );
}

#[test]
fn a_scene_round_trips_byte_for_byte_with_its_backdrop_opaque() {
    let (scene, doc) = parse_typed::<Scene3D>(SCENE.as_bytes());
    // No typed edit: the fidelity wrapper must re-emit everything, backdrop included.
    assert_round_trips(&scene, doc, SCENE.as_bytes());
}

#[test]
fn a_scene_spec_needs_both_required_parts() {
    // A well-formed scene yields a spec.
    let (scene, doc) = parse_typed::<Scene3D>(SCENE.as_bytes());
    assert!(scene.spec(&doc.interner).is_some());

    // A scene missing its light rig cannot be described.
    let broken =
        format!(r#"<a:scene3d xmlns:a="{A}"><a:camera prst="orthographicFront"/></a:scene3d>"#);
    let (scene, doc) = parse_typed::<Scene3D>(broken.as_bytes());
    assert!(scene.camera(&doc.interner).is_some());
    assert!(scene.spec(&doc.interner).is_none());
}

#[test]
fn a_scene_spec_rebuilds_the_camera_and_light_rig() {
    let (scene, doc) = parse_typed::<Scene3D>(SCENE.as_bytes());
    let spec = scene.spec(&doc.interner).expect("spec");

    // Rebuilding from the spec drops the opaque backdrop but keeps every modeled facet.
    let mut interner = Interner::new();
    let scene = spec.to_scene_3d(&mut interner);
    let built = serialize_built(interner, &scene);
    assert!(built.contains(r#"<a:camera prst="orthographicFront" fov="600000" zoom="150000">"#));
    assert!(built.contains(r#"<a:lightRig rig="threePt" dir="t">"#));
    assert!(
        !built.contains("backdrop"),
        "the opaque backdrop is not part of the spec: {built}"
    );
}

#[test]
fn a_scene_spec_is_interner_free_and_round_trips_a_value() {
    let (scene, doc) = parse_typed::<Scene3D>(SCENE.as_bytes());
    let mut spec: Scene3DSpec = scene.spec(&doc.interner).expect("spec");
    // Edit an interner-free field, write it back, read it again.
    spec.camera.preset = PresetCamera::IsometricLeftDown;
    spec.light_rig.direction = LightRigDirection::BottomRight;

    let mut interner = Interner::new();
    let rebuilt = spec.to_scene_3d(&mut interner);
    let read_back = rebuilt.spec(&interner).expect("spec");
    assert_eq!(read_back.camera.preset, PresetCamera::IsometricLeftDown);
    assert_eq!(
        read_back.light_rig.direction,
        LightRigDirection::BottomRight
    );
}

// ---------------------------------------------------------------------------------------------
// Shape3D — extrusion, bevels, colors, material
// ---------------------------------------------------------------------------------------------

const SP3D: &str = concat!(
    r#"<a:sp3d xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#,
    r#" z="12700" extrusionH="63500" contourW="6350" prstMaterial="metal">"#,
    r#"<a:bevelT w="88900" h="88900" prst="coolSlant"/>"#,
    r#"<a:bevelB w="50800" h="25400" prst="angle"/>"#,
    r#"<a:extrusionClr><a:srgbClr val="FF0000"/></a:extrusionClr>"#,
    r#"<a:contourClr><a:schemeClr val="accent1"/></a:contourClr>"#,
    r#"<a:extLst><a:ext uri="{X}"/></a:extLst>"#,
    r#"</a:sp3d>"#,
);

#[test]
fn a_shape_3d_reads_its_extrusion_bevels_and_colors() {
    let (sp3d, doc) = parse_typed::<Shape3D>(SP3D.as_bytes());

    assert_eq!(sp3d.z(&doc.interner).expect("z").emu(), 12_700);
    assert_eq!(
        sp3d.extrusion_height(&doc.interner).expect("h").emu(),
        63_500
    );
    assert_eq!(sp3d.contour_width(&doc.interner).expect("w").emu(), 6_350);
    assert_eq!(sp3d.material(&doc.interner), Some(PresetMaterial::Metal));

    let top = sp3d.bevel_top(&doc.interner).expect("bevelT");
    assert_eq!(top.width.expect("w").emu(), 88_900);
    assert_eq!(top.preset, Some(BevelPreset::CoolSlant));
    let bottom = sp3d.bevel_bottom(&doc.interner).expect("bevelB");
    assert_eq!(bottom.preset, Some(BevelPreset::Angle));

    assert_eq!(
        sp3d.extrusion_color(&doc.interner),
        Some(ColorSpec::Srgb("FF0000".to_owned()))
    );
    assert!(matches!(
        sp3d.contour_color(&doc.interner),
        Some(ColorSpec::Scheme(_))
    ));
}

#[test]
fn a_shape_3d_round_trips_byte_for_byte_with_its_ext_lst_opaque() {
    let (sp3d, doc) = parse_typed::<Shape3D>(SP3D.as_bytes());
    assert_round_trips(&sp3d, doc, SP3D.as_bytes());
}

#[test]
fn a_shape_3d_spec_is_non_destructive_of_the_facets_it_does_not_touch() {
    let (sp3d, doc) = parse_typed::<Shape3D>(SP3D.as_bytes());
    let mut spec: Shape3DSpec = sp3d.spec(&doc.interner);
    // Change only the material.
    spec.material = Some(PresetMaterial::Plastic);

    let mut interner = Interner::new();
    let rebuilt = spec.to_shape_3d(&mut interner);
    let read = rebuilt.spec(&interner);
    // The edited facet took, and the ones the spec did not name are exactly as they were.
    assert_eq!(read.material, Some(PresetMaterial::Plastic));
    assert_eq!(read.z, Some(mjx_dml::Emu::from_emu(12_700)));
    assert_eq!(
        read.bevel_top.and_then(|b| b.preset),
        Some(BevelPreset::CoolSlant)
    );
    assert_eq!(
        read.extrusion_color,
        Some(ColorSpec::Srgb("FF0000".to_owned()))
    );
}

// ---------------------------------------------------------------------------------------------
// Absent is not the schema default
// ---------------------------------------------------------------------------------------------

#[test]
fn an_unstated_measure_reads_none_not_the_schema_default() {
    // A bare bevel and a bare sp3d state nothing — `None`, not 76200 / 0 / warmMatte.
    let sp3d_xml = format!(r#"<a:sp3d xmlns:a="{A}"><a:bevelT/></a:sp3d>"#);
    let (sp3d, doc) = parse_typed::<Shape3D>(sp3d_xml.as_bytes());
    assert_eq!(sp3d.z(&doc.interner), None);
    assert_eq!(sp3d.extrusion_height(&doc.interner), None);
    assert_eq!(sp3d.material(&doc.interner), None);
    let bevel = sp3d.bevel_top(&doc.interner).expect("bevelT present");
    assert_eq!(bevel.width, None);
    assert_eq!(bevel.height, None);
    assert_eq!(bevel.preset, None);

    // A camera states only its required preset; fov/zoom/rot are absent.
    let scene_xml = format!(r#"<a:scene3d xmlns:a="{A}"><a:camera prst="orthographicFront"/>"#,);
    let scene_xml = format!(r#"{scene_xml}<a:lightRig rig="threePt" dir="t"/></a:scene3d>"#);
    let (scene, doc) = parse_typed::<Scene3D>(scene_xml.as_bytes());
    let camera = scene.camera(&doc.interner).expect("camera");
    assert_eq!(camera.field_of_view, None);
    assert_eq!(camera.zoom, None);
    assert_eq!(camera.rotation, None);
    assert_eq!(scene.light_rig(&doc.interner).expect("rig").rotation, None);
}
