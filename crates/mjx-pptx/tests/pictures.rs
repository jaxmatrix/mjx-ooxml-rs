//! Integration tests for picture shapes (`p:pic`): adding one, addressing it in the slide's single
//! shape index space, editing it through the shared `p:spPr` surface, and reading or replacing the
//! image it shows.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{ColorSpec, LineSpec, LineWidth};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_opc::{Package, PartName};
use mjx_pptx::{PptxError, Presentation, ShapeBounds, ShapeKind};

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

fn part(name: &str) -> PartName {
    PartName::new(name).expect("valid part name")
}

fn bounds() -> ShapeBounds {
    ShapeBounds::from_inches(1.0, 1.0, 3.0, 2.0)
}

/// A valid 2×2 truecolour PNG (76 bytes), inlined so no binary fixture is committed.
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x02, 0x00, 0x00, 0x00, 0xFD, 0xD4, 0x9A,
    0x73, 0x00, 0x00, 0x00, 0x13, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0x78, 0x60, 0x60, 0x60,
    0x90, 0xF0, 0x80, 0x01, 0x88, 0x81, 0x2C, 0x00, 0x25, 0xAE, 0x05, 0x61, 0x56, 0x69, 0x41, 0x72,
    0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// A second, different valid 2×2 PNG.
const OTHER_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x02, 0x00, 0x00, 0x00, 0xFD, 0xD4, 0x9A,
    0x73, 0x00, 0x00, 0x00, 0x13, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0x50, 0x58, 0xE0, 0xF0,
    0xE1, 0x81, 0x00, 0x03, 0x10, 0x03, 0x59, 0x00, 0x29, 0xCE, 0x05, 0xC1, 0x82, 0x11, 0xDA, 0x8B,
    0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[test]
fn a_picture_is_a_shape_in_the_one_index_space() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let before = pres.shape_count(0).expect("shape count");

    let picture = pres
        .add_picture(0, TINY_PNG, bounds())
        .expect("add picture");

    assert_eq!(
        picture, before,
        "the picture appends after the existing shapes"
    );
    assert_eq!(pres.shape_count(0).expect("shape count"), before + 1);
    assert_eq!(
        pres.shape_kind(0, picture).expect("shape kind"),
        ShapeKind::Picture
    );
    assert_eq!(
        pres.shape_kind(0, 0).expect("shape kind"),
        ShapeKind::Shape,
        "the pre-existing shapes keep their kind and index"
    );
}

#[test]
fn pictures_and_shapes_interleave_by_document_order() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let first_picture = pres.add_picture(0, TINY_PNG, bounds()).expect("picture");
    let shape = pres
        .add_shape(0, PresetShapeType::Rectangle, bounds())
        .expect("shape");
    let second_picture = pres.add_picture(0, OTHER_PNG, bounds()).expect("picture");

    assert_eq!(
        (shape, second_picture),
        (first_picture + 1, first_picture + 2)
    );
    for (idx, expected) in [
        (first_picture, ShapeKind::Picture),
        (shape, ShapeKind::Shape),
        (second_picture, ShapeKind::Picture),
    ] {
        assert_eq!(pres.shape_kind(0, idx).expect("shape kind"), expected);
    }
}

#[test]
fn a_picture_survives_a_save_and_reopen() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let picture = pres
        .add_picture(0, TINY_PNG, bounds())
        .expect("add picture");
    let saved = pres.save().expect("save");

    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened.shape_kind(0, picture).expect("shape kind"),
        ShapeKind::Picture
    );
    assert_eq!(
        reopened
            .picture_image_bytes(0, picture)
            .expect("image bytes"),
        Some(TINY_PNG),
        "the picture still shows the bytes it was given"
    );
}

#[test]
fn adding_a_picture_touches_only_the_parts_it_must() {
    let bytes = fixture("sample.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.add_picture(0, TINY_PNG, bounds())
        .expect("add picture");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    let allowed = [
        "[Content_Types].xml",
        "ppt/slides/slide1.xml",
        "ppt/slides/_rels/slide1.xml.rels",
    ];
    for (name, orig) in &original {
        if allowed.contains(&name.as_str()) {
            continue;
        }
        assert_eq!(reopened.get(name), Some(orig), "part {name} changed");
    }
    assert_eq!(
        reopened.get("ppt/media/image1.png").map(Vec::as_slice),
        Some(TINY_PNG)
    );
}

#[test]
fn the_same_image_in_two_pictures_is_stored_once() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_picture(0, TINY_PNG, bounds()).expect("picture");
    pres.add_picture(0, TINY_PNG, bounds()).expect("picture");

    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        pkg.part_names()
            .filter(|p| p.as_str().starts_with("/ppt/media/"))
            .count(),
        1
    );
}

#[test]
fn a_picture_takes_the_shared_sp_pr_surface() {
    // The payoff of the one index space: outline and effects apply to a picture with no picture-
    // specific API at all.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let picture = pres
        .add_picture(0, TINY_PNG, bounds())
        .expect("add picture");

    let outline = LineSpec {
        fill: Some(mjx_dml::FillSpec::solid(ColorSpec::Srgb("203864".into()))),
        width: Some(LineWidth::from_points(3.0)),
        ..LineSpec::new()
    };
    pres.set_shape_outline(0, picture, &outline)
        .expect("outline a picture");

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened.shape_outline(0, picture).expect("read outline"),
        Some(outline),
        "the outline must round-trip on a picture just as on a shape"
    );
}

#[test]
fn picture_image_rel_id_and_bytes_resolve_to_the_stored_part() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let picture = pres
        .add_picture(0, TINY_PNG, bounds())
        .expect("add picture");

    let rel_id = pres
        .picture_image_rel_id(0, picture)
        .expect("rel id")
        .expect("the picture embeds an image");
    assert_eq!(
        pres.picture_image_bytes(0, picture).expect("bytes"),
        Some(TINY_PNG)
    );

    // The id really is the slide's relationship to the media part.
    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    let rels = pkg
        .relationships_for(Some(&part("/ppt/slides/slide1.xml")))
        .expect("slide relationships");
    assert_eq!(
        rels.by_id(&rel_id).expect("relationship").target,
        "../media/image1.png"
    );
}

#[test]
fn setting_a_picture_image_swaps_the_embed_and_keeps_the_old_part() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let picture = pres
        .add_picture(0, TINY_PNG, bounds())
        .expect("add picture");
    let first_rel = pres.picture_image_rel_id(0, picture).expect("rel").unwrap();

    pres.set_picture_image(0, picture, OTHER_PNG)
        .expect("replace the image");

    let second_rel = pres.picture_image_rel_id(0, picture).expect("rel").unwrap();
    assert_ne!(
        first_rel, second_rel,
        "the embed must point at the new image"
    );
    assert_eq!(
        pres.picture_image_bytes(0, picture).expect("bytes"),
        Some(OTHER_PNG)
    );

    // The replaced part stays in the package — another shape may still reference it.
    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        pkg.part_bytes(&part("/ppt/media/image1.png")),
        Some(TINY_PNG)
    );
    assert_eq!(
        pkg.part_bytes(&part("/ppt/media/image2.png")),
        Some(OTHER_PNG)
    );
}

#[test]
fn setting_the_same_image_again_is_stable() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let picture = pres
        .add_picture(0, TINY_PNG, bounds())
        .expect("add picture");
    let rel = pres.picture_image_rel_id(0, picture).expect("rel").unwrap();

    pres.set_picture_image(0, picture, TINY_PNG)
        .expect("set the same image");

    assert_eq!(
        pres.picture_image_rel_id(0, picture).expect("rel").unwrap(),
        rel,
        "identical bytes reuse the part and the relationship"
    );
    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        pkg.part_names()
            .filter(|p| p.as_str().starts_with("/ppt/media/"))
            .count(),
        1
    );
}

#[test]
fn the_image_apis_reject_a_shape_that_is_not_a_picture() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres
        .add_shape(0, PresetShapeType::Rectangle, bounds())
        .expect("add shape");

    let err = pres
        .picture_image_rel_id(0, shape)
        .expect_err("not a picture");
    assert!(matches!(err, PptxError::ShapeIsNotAPicture), "{err:?}");
    let err = pres
        .set_picture_image(0, shape, TINY_PNG)
        .expect_err("not a picture");
    assert!(matches!(err, PptxError::ShapeIsNotAPicture), "{err:?}");
}

#[test]
fn a_picture_of_unrecognized_bytes_is_rejected_and_changes_nothing() {
    let bytes = fixture("sample.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");

    let err = pres
        .add_picture(0, b"not an image", bounds())
        .expect_err("unknown bytes must be rejected");
    assert!(matches!(err, PptxError::UnrecognizedImageFormat), "{err:?}");

    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        original,
        "no shape and no part may be left behind"
    );
}
