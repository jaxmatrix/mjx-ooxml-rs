//! Integration tests for image parts: adding a picture to a package, deduplicating identical bytes,
//! and using the resulting relationship as a shape's picture fill — with fidelity (only the parts an
//! added image must touch change).

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{BlipFillMode, FillSpec};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_opc::{Package, PartName};
use mjx_pptx::{PptxError, Presentation, ShapeBounds};

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

/// A valid 2×2 truecolour PNG (76 bytes) — small enough to inline, so no binary fixture is committed
/// and the sample deck (which has no media parts) stays as it is.
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
fn add_image_inserts_a_media_part_and_a_slide_relationship() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let rel_id = pres.add_image(0, TINY_PNG).expect("add image");

    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    let media = part("/ppt/media/image1.png");
    assert_eq!(
        pkg.part_bytes(&media),
        Some(TINY_PNG),
        "the image bytes must be stored verbatim"
    );
    assert_eq!(pkg.content_type_of(&media), Some("image/png"));

    let rels = pkg
        .relationships_for(Some(&part("/ppt/slides/slide1.xml")))
        .expect("slide has relationships");
    let rel = rels
        .by_id(&rel_id)
        .expect("the returned id is in the .rels");
    assert_eq!(
        rel.rel_type,
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
    );
    assert_eq!(rel.target, "../media/image1.png");
}

#[test]
fn add_image_registers_a_default_and_no_override() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_image(0, TINY_PNG).expect("add image");

    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    assert!(
        pkg.content_types()
            .defaults()
            .iter()
            .any(|d| d.extension == "png" && d.content_type == "image/png"),
        "a Default rule for png should carry the media part's content type"
    );
    assert!(
        !pkg.content_types()
            .overrides()
            .iter()
            .any(|o| o.part_name.as_str().starts_with("/ppt/media/")),
        "no per-part Override should be needed"
    );
}

#[test]
fn add_image_leaves_every_untouched_part_byte_identical() {
    let bytes = fixture("sample.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.add_image(0, TINY_PNG).expect("add image");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    // Only the content types and the slide's own .rels may change; nothing else.
    let allowed = ["[Content_Types].xml", "ppt/slides/_rels/slide1.xml.rels"];
    for (name, orig) in &original {
        if allowed.contains(&name.as_str()) {
            continue;
        }
        assert_eq!(reopened.get(name), Some(orig), "part {name} changed");
    }
    assert!(
        reopened.contains_key("ppt/media/image1.png"),
        "the media part is missing"
    );
}

#[test]
fn identical_bytes_are_stored_once_and_share_a_relationship() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let first = pres.add_image(0, TINY_PNG).expect("add image");
    let second = pres
        .add_image(0, TINY_PNG)
        .expect("add the same image again");
    assert_eq!(first, second, "the same bytes on one slide reuse the id");

    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    let media: Vec<String> = pkg
        .part_names()
        .filter(|p| p.as_str().starts_with("/ppt/media/"))
        .map(|p| p.as_str().to_owned())
        .collect();
    assert_eq!(media, vec!["/ppt/media/image1.png".to_owned()]);
}

#[test]
fn different_images_get_successive_part_names() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let first = pres.add_image(0, TINY_PNG).expect("add image");
    let second = pres.add_image(0, OTHER_PNG).expect("add another image");
    assert_ne!(first, second);

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
fn the_same_image_on_two_slides_shares_the_part_but_not_the_relationship() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let slide2 = pres.add_slide().expect("add slide");
    pres.add_image(0, TINY_PNG).expect("add to slide 1");
    pres.add_image(slide2, TINY_PNG).expect("add to slide 2");

    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        pkg.part_names()
            .filter(|p| p.as_str().starts_with("/ppt/media/"))
            .count(),
        1,
        "the image should be stored once"
    );
    for slide in ["/ppt/slides/slide1.xml", "/ppt/slides/slide2.xml"] {
        let rels = pkg
            .relationships_for(Some(&part(slide)))
            .unwrap_or_else(|| panic!("{slide} has relationships"));
        assert_eq!(
            rels.by_type(
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
            )
            .count(),
            1,
            "{slide} should relate to the image itself"
        );
    }
}

#[test]
fn unrecognized_bytes_are_rejected_and_change_nothing() {
    let bytes = fixture("sample.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");

    let err = pres
        .add_image(0, b"this is not an image")
        .expect_err("unknown bytes must be rejected");
    assert!(matches!(err, PptxError::UnrecognizedImageFormat), "{err:?}");

    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        original,
        "a rejected image must leave the package untouched"
    );
}

#[test]
fn out_of_range_slide_is_rejected() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let err = pres.add_image(99, TINY_PNG).expect_err("no such slide");
    assert!(
        matches!(err, PptxError::SlideIndexOutOfRange { index: 99, .. }),
        "{err:?}"
    );
}

#[test]
fn an_added_image_fills_a_shape_end_to_end() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 1.0, 3.0, 2.0),
        )
        .expect("add shape");
    let rel_id = pres.add_image(0, TINY_PNG).expect("add image");
    pres.set_shape_fill(
        0,
        shape,
        &FillSpec::Blip {
            rel_id: rel_id.clone(),
            mode: BlipFillMode::Stretch,
        },
    )
    .expect("set picture fill");

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened.shape_fill(0, shape).expect("shape fill"),
        Some(FillSpec::Blip {
            rel_id: rel_id.clone(),
            mode: BlipFillMode::Stretch,
        }),
        "the picture fill should survive a save/open round trip"
    );

    // And the relationship still resolves to a media part holding the original bytes.
    let pkg = Package::open(&saved).expect("reopen package");
    let rels = pkg
        .relationships_for(Some(&part("/ppt/slides/slide1.xml")))
        .expect("slide relationships");
    let target = &rels.by_id(&rel_id).expect("relationship present").target;
    assert_eq!(target, "../media/image1.png");
    assert_eq!(
        pkg.part_bytes(&part("/ppt/media/image1.png")),
        Some(TINY_PNG)
    );
}
