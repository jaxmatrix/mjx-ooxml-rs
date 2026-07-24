//! Integration tests for the shape 3-D surface: read a shape's [`Scene3DSpec`] / [`Shape3DSpec`],
//! set and clear them, and save — with fidelity (only the edited slide changes), correct `p:spPr`
//! placement (`a:scene3d` after effects, `a:sp3d` last before any `a:extLst`), and parity between the
//! flat setters and the fluent [`mjx_pptx::ShapeCursor`].

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{
    Angle, Bevel, BevelPreset, Camera, ColorSpec, Emu, FillSpec, Fraction, LightRig,
    LightRigDirection, LightRigType, LineSpec, LineWidth, PresetCamera, PresetMaterial,
    Scene3DSpec, Shape3DSpec, SphereCoordinates,
};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_opc::Package;
use mjx_pptx::{Presentation, ShapeBounds};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

fn bounds() -> ShapeBounds {
    ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0)
}

/// Adds a fresh autoshape and returns its shape index.
fn added_shape(pres: &mut Presentation) -> usize {
    pres.add_shape(0, PresetShapeType::RoundedRectangle, bounds())
        .expect("add shape")
}

/// A fully-populated 3-D scene: a camera (preset + fov + zoom + rotation) and a light rig (rig +
/// direction + rotation). Every stated value is a fixed point of `to_scene_3d → spec`, so it
/// round-trips exactly through a save/reopen.
fn scene() -> Scene3DSpec {
    Scene3DSpec {
        camera: Camera {
            preset: PresetCamera::OrthographicFront,
            field_of_view: Some(Angle::from_degrees(45.0)),
            zoom: Some(Fraction::from_ratio(1.5)),
            rotation: Some(SphereCoordinates {
                latitude: Angle::from_degrees(20.0),
                longitude: Angle::from_degrees(30.0),
                revolution: Angle::from_degrees(0.0),
            }),
        },
        light_rig: LightRig {
            rig: LightRigType::ThreePoint,
            direction: LightRigDirection::TopLeft,
            rotation: None,
        },
    }
}

/// A rich set of 3-D shape properties: a stand-off, an extrusion, a material, a top and bottom bevel,
/// and both edge colors. `contour_width` is deliberately left unset to prove `None` survives (an
/// unstated attribute reads `None`, not the schema default).
fn properties() -> Shape3DSpec {
    Shape3DSpec {
        z: Some(Emu::from_emu(12_700)),
        extrusion_height: Some(Emu::from_emu(190_500)),
        contour_width: None,
        material: Some(PresetMaterial::Metal),
        bevel_top: Some(Bevel {
            width: Some(Emu::from_emu(76_200)),
            height: Some(Emu::from_emu(38_100)),
            preset: Some(BevelPreset::Circle),
        }),
        bevel_bottom: Some(Bevel {
            width: Some(Emu::from_emu(50_800)),
            height: None,
            preset: Some(BevelPreset::Slope),
        }),
        extrusion_color: Some(ColorSpec::Srgb("FF0000".to_owned())),
        contour_color: Some(ColorSpec::Srgb("00FF00".to_owned())),
    }
}

/// The decompressed bytes of the deck's first slide part, as UTF-8 text.
fn slide1_xml(bytes: &[u8]) -> String {
    let pkg = Package::open(bytes).expect("open package");
    let (_, xml) = byte_map(&pkg)
        .into_iter()
        .find(|(name, _)| name.ends_with("slide1.xml"))
        .expect("slide1.xml present");
    String::from_utf8(xml).expect("slide1 is UTF-8")
}

#[test]
fn fresh_shape_has_no_3d() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    assert_eq!(pres.shape_scene_3d(0, idx).expect("scene"), None);
    assert_eq!(pres.shape_3d_properties(0, idx).expect("props"), None);
}

#[test]
fn set_scene_3d_reads_back_and_persists() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_scene_3d(0, idx, &scene())
        .expect("set scene");

    assert_eq!(pres.shape_scene_3d(0, idx).expect("scene"), Some(scene()));

    // Survives save/reopen, and the shape's geometry is intact (the scene didn't clobber spPr).
    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(reread.shape_scene_3d(0, idx).expect("scene"), Some(scene()));
    assert!(matches!(
        reread.shape_geometry(0, idx).expect("geometry"),
        mjx_dml::ShapeGeometry::RoundedRectangle { .. }
    ));
}

#[test]
fn set_shape_3d_properties_reads_back_and_persists() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_3d_properties(0, idx, &properties())
        .expect("set props");

    assert_eq!(
        pres.shape_3d_properties(0, idx).expect("props"),
        Some(properties())
    );

    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    let read = reread.shape_3d_properties(0, idx).expect("props");
    assert_eq!(read, Some(properties()));
    // The deliberately-unstated attribute stays `None`, not the schema default.
    assert_eq!(read.expect("some").contour_width, None);
}

#[test]
fn set_scene_3d_replaces_in_place() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_scene_3d(0, idx, &scene())
        .expect("set first scene");
    // A second set replaces the a:scene3d rather than adding a second one.
    let second = Scene3DSpec {
        light_rig: LightRig {
            rig: LightRigType::Balanced,
            direction: LightRigDirection::Top,
            rotation: None,
        },
        ..scene()
    };
    pres.set_shape_scene_3d(0, idx, &second)
        .expect("set second scene");

    let xml = slide1_xml(&pres.save().expect("save"));
    assert_eq!(
        xml.matches("<a:scene3d").count(),
        1,
        "exactly one a:scene3d"
    );

    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(reread.shape_scene_3d(0, idx).expect("scene"), Some(second));
}

#[test]
fn clear_removes_scene_and_props() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_scene_3d(0, idx, &scene())
        .expect("set scene");
    pres.set_shape_3d_properties(0, idx, &properties())
        .expect("set props");

    pres.clear_shape_scene_3d(0, idx).expect("clear scene");
    pres.clear_shape_3d_properties(0, idx).expect("clear props");

    assert_eq!(pres.shape_scene_3d(0, idx).expect("scene"), None);
    assert_eq!(pres.shape_3d_properties(0, idx).expect("props"), None);

    // Clearing is byte-level removal: no empty element is left behind.
    let xml = slide1_xml(&pres.save().expect("save"));
    assert!(!xml.contains("scene3d"), "no a:scene3d remains");
    assert!(!xml.contains("sp3d"), "no a:sp3d remains");
}

#[test]
fn clear_when_absent_is_a_no_op() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    // Nothing to remove — both clears succeed and leave the shape flat.
    pres.clear_shape_scene_3d(0, idx).expect("clear scene");
    pres.clear_shape_3d_properties(0, idx).expect("clear props");
    assert_eq!(pres.shape_scene_3d(0, idx).expect("scene"), None);
    assert_eq!(pres.shape_3d_properties(0, idx).expect("props"), None);
}

#[test]
fn three_d_coexists_with_fill_outline_effects_and_is_ordered() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_fill(
        0,
        idx,
        &FillSpec::solid(ColorSpec::Srgb("FFFF00".to_owned())),
    )
    .expect("fill");
    pres.set_shape_outline(
        0,
        idx,
        &LineSpec::solid(
            LineWidth::from_points(1.5),
            ColorSpec::Srgb("FF0000".to_owned()),
        ),
    )
    .expect("outline");
    pres.set_shape_no_effects(0, idx).expect("effects");
    pres.set_shape_scene_3d(0, idx, &scene()).expect("scene");
    pres.set_shape_3d_properties(0, idx, &properties())
        .expect("props");

    // All slots read back, so the spPr children don't collide.
    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert!(reread.shape_fill(0, idx).expect("fill").is_some());
    assert!(reread.shape_outline(0, idx).expect("outline").is_some());
    assert_eq!(reread.shape_scene_3d(0, idx).expect("scene"), Some(scene()));
    assert_eq!(
        reread.shape_3d_properties(0, idx).expect("props"),
        Some(properties())
    );

    // The `CT_ShapeProperties` content order is respected: ln → effectLst → scene3d → sp3d.
    let xml = slide1_xml(&pres.save().expect("save"));
    let at = |needle: &str| {
        xml.find(needle)
            .unwrap_or_else(|| panic!("{needle} present"))
    };
    assert!(at("<a:ln") < at("<a:effectLst"), "ln before effectLst");
    assert!(
        at("<a:effectLst") < at("<a:scene3d"),
        "effectLst before scene3d"
    );
    assert!(at("<a:scene3d") < at("<a:sp3d"), "scene3d before sp3d");
}

#[test]
fn set_3d_keeps_other_parts_byte_identical() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_scene_3d(0, idx, &scene()).expect("scene");
    pres.set_shape_3d_properties(0, idx, &properties())
        .expect("props");
    let saved = pres.save().expect("save");

    let reopened = byte_map(&Package::open(&saved).expect("reopen package"));
    for (name, original) in &snapshot {
        if name.ends_with("slide1.xml") {
            continue; // the one part we edited
        }
        assert_eq!(
            reopened.get(name),
            Some(original),
            "3-D edit dirtied unrelated part {name}"
        );
    }
}

#[test]
fn cursor_matches_the_flat_setters() {
    // The fluent cursor records the same edits and produces the same read-back as the flat API.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.shape(0, idx)
        .expect("cursor")
        .scene_3d(scene())
        .shape_3d_properties(properties())
        .apply()
        .expect("apply");

    assert_eq!(pres.shape_scene_3d(0, idx).expect("scene"), Some(scene()));
    assert_eq!(
        pres.shape_3d_properties(0, idx).expect("props"),
        Some(properties())
    );

    // And the cursor can clear what it set.
    pres.shape(0, idx)
        .expect("cursor")
        .clear_scene_3d()
        .clear_shape_3d_properties()
        .apply()
        .expect("apply clears");
    assert_eq!(pres.shape_scene_3d(0, idx).expect("scene"), None);
    assert_eq!(pres.shape_3d_properties(0, idx).expect("props"), None);
}

#[test]
fn set_3d_on_a_group_member() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let a = added_shape(&mut pres);
    let b = added_shape(&mut pres);
    let group = pres
        .group_shapes(0, &[a.into(), b.into()])
        .expect("group shapes");
    let member = group.child(0);

    pres.set_shape_scene_3d(0, member.clone(), &scene())
        .expect("set scene on member");
    assert_eq!(
        pres.shape_scene_3d(0, member).expect("scene"),
        Some(scene())
    );
}
