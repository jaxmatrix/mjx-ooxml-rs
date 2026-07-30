//! Integration tests for OLE object graphic frames (MJX-135, tier MJX-136): recognizing a
//! `p:graphicFrame` that frames a legacy OLE object, resolving `p:oleObj@r:id` to the embedded object
//! part and its fallback snapshot image, and reading both verbatim — without modeling the embedded
//! object, and with fidelity (the deck round-trips byte-identically and editing a slide leaves the OLE
//! parts untouched).
//!
//! The fixture `ole.pptx` is `sample.pptx` plus, on slide 1, a `p:graphicFrame` whose `p:oleObj`
//! (wrapped in `mc:AlternateContent`) references `ppt/embeddings/oleObject1.bin` (`rId2`) and a
//! fallback snapshot `ppt/media/image1.png` (`rId3`). Hand-crafted (see MJX-140 for producer-authentic
//! validation).

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_opc::{Package, PartName, TargetMode};
use mjx_pptx::{default_placeholder_ole, GraphicFrameKind, OleObject, PptxError, Presentation};

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

/// The OLE frame is shape 1 on slide 1 (shape 0 is the title text box).
const OLE_SURFACE: usize = 0;
const OLE_SHAPE: usize = 1;

/// Every part tied to the OLE object that must survive an edit made elsewhere byte-for-byte.
const OLE_PARTS: &[&str] = &[
    "ppt/embeddings/oleObject1.bin",
    "ppt/media/image1.png",
    "ppt/slides/_rels/slide1.xml.rels",
];

#[test]
fn an_ole_frame_reads_as_an_ole_object() {
    let mut pres = Presentation::open(&fixture("ole.pptx")).expect("open");
    assert_eq!(
        pres.graphic_frame_kind(OLE_SURFACE, OLE_SHAPE)
            .expect("kind"),
        Some(GraphicFrameKind::OleObject),
        "slide 1 shape 1 frames an OLE object"
    );
    // The title text box is not a graphic frame at all.
    assert_eq!(
        pres.graphic_frame_kind(OLE_SURFACE, 0).expect("kind"),
        None,
        "a text box is not a graphic frame"
    );
}

#[test]
fn ole_object_rel_id_and_prog_id_name_the_embedded_object() {
    let mut pres = Presentation::open(&fixture("ole.pptx")).expect("open");
    assert_eq!(
        pres.ole_object_rel_id(OLE_SURFACE, OLE_SHAPE)
            .expect("rel id"),
        Some("rId2".to_owned()),
        "the OLE frame names its embedded-object relationship"
    );
    assert_eq!(
        pres.ole_prog_id(OLE_SURFACE, OLE_SHAPE).expect("prog id"),
        Some("Excel.Sheet.12".to_owned()),
    );
    // A shape that frames no OLE object answers None, not an error.
    assert_eq!(pres.ole_object_rel_id(OLE_SURFACE, 0).expect("read"), None);
    assert_eq!(pres.ole_prog_id(OLE_SURFACE, 0).expect("read"), None);
}

#[test]
fn ole_object_part_bytes_resolves_to_the_verbatim_embedded_part() {
    let bytes = fixture("ole.pptx");
    let baseline = Package::open(&bytes).expect("baseline");
    let embedded = baseline
        .part_bytes(&part("/ppt/embeddings/oleObject1.bin"))
        .expect("fixture has an embedded object part")
        .to_vec();

    let mut pres = Presentation::open(&bytes).expect("open");
    assert_eq!(
        pres.ole_object_part_bytes(OLE_SURFACE, OLE_SHAPE)
            .expect("read"),
        Some(embedded.as_slice()),
        "the resolved bytes are exactly the package's embedded object part"
    );
    // A non-OLE shape answers None.
    assert_eq!(
        pres.ole_object_part_bytes(OLE_SURFACE, 0).expect("read"),
        None
    );
}

#[test]
fn ole_snapshot_image_bytes_resolves_to_the_verbatim_snapshot() {
    let bytes = fixture("ole.pptx");
    let baseline = Package::open(&bytes).expect("baseline");
    let snapshot = baseline
        .part_bytes(&part("/ppt/media/image1.png"))
        .expect("fixture has a snapshot image")
        .to_vec();

    let mut pres = Presentation::open(&bytes).expect("open");
    assert_eq!(
        pres.ole_snapshot_rel_id(OLE_SURFACE, OLE_SHAPE)
            .expect("rel id"),
        Some("rId3".to_owned()),
    );
    assert_eq!(
        pres.ole_snapshot_image_bytes(OLE_SURFACE, OLE_SHAPE)
            .expect("read"),
        Some(snapshot.as_slice()),
        "the resolved bytes are exactly the package's snapshot image"
    );
}

#[test]
fn reading_ole_leaves_every_part_byte_identical() {
    let bytes = fixture("ole.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Exercise every read accessor; none may dirty a part.
    pres.graphic_frame_kind(OLE_SURFACE, OLE_SHAPE)
        .expect("kind");
    pres.ole_object_rel_id(OLE_SURFACE, OLE_SHAPE)
        .expect("rel id");
    pres.ole_object_part_bytes(OLE_SURFACE, OLE_SHAPE)
        .expect("bytes");
    pres.ole_snapshot_rel_id(OLE_SURFACE, OLE_SHAPE)
        .expect("snapshot rel id");
    pres.ole_snapshot_image_bytes(OLE_SURFACE, OLE_SHAPE)
        .expect("snapshot bytes");
    pres.ole_prog_id(OLE_SURFACE, OLE_SHAPE).expect("prog id");

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(
        reopened, original,
        "reading an OLE object must dirty nothing"
    );
}

#[test]
fn editing_a_slide_leaves_the_ole_parts_byte_identical() {
    let bytes = fixture("ole.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Edit the title text — the slide XML changes, but the separate OLE parts must not.
    pres.set_shape_text(OLE_SURFACE, 0, 0, "Edited")
        .expect("set text");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    for &name in OLE_PARTS {
        assert_eq!(
            reopened.get(name),
            original.get(name),
            "OLE part {name} must be untouched by an edit elsewhere"
        );
    }
    assert_ne!(
        reopened.get("ppt/slides/slide1.xml"),
        original.get("ppt/slides/slide1.xml"),
        "the edited slide should have changed"
    );
}

// -------------------------------------------------------------------------------------------------
// MJX-201 P3 — replacing an inaccessible OLE object with a placeholder
// -------------------------------------------------------------------------------------------------

/// Flips the fixture's embedded-object relationship (`rId2`) to an external link, returning the
/// reopened presentation — the caller-decides-inaccessible case, which the fixture cannot provide.
fn ole_with_external_object() -> Presentation {
    let mut pkg = Package::open(&fixture("ole.pptx")).expect("open");
    let slide = part("/ppt/slides/slide1.xml");
    assert!(
        pkg.retarget_relationship(
            Some(&slide),
            "rId2",
            "file:///C:/data/Worksheet.xlsx",
            TargetMode::External,
        )
        .expect("retarget"),
        "the object relationship exists"
    );
    Presentation::open(&pkg.save().expect("save")).expect("reopen")
}

#[test]
fn default_placeholder_ole_is_a_structurally_valid_compound_file() {
    let cfb = default_placeholder_ole();
    assert_eq!(cfb.len(), 1536, "header + FAT sector + directory sector");
    assert_eq!(
        &cfb[0..8],
        &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
        "CFB signature"
    );
    assert_eq!(&cfb[26..28], &0x0003u16.to_le_bytes(), "major version 3");
    assert_eq!(&cfb[30..32], &0x0009u16.to_le_bytes(), "512-byte sectors");
    // FAT sector (sector 0) begins at byte 512: entry 0 = FATSECT, entry 1 = ENDOFCHAIN.
    assert_eq!(
        &cfb[512..516],
        &0xFFFF_FFFDu32.to_le_bytes(),
        "FAT[0]=FATSECT"
    );
    assert_eq!(
        &cfb[516..520],
        &0xFFFF_FFFEu32.to_le_bytes(),
        "FAT[1]=ENDOFCHAIN"
    );
    // Directory sector (sector 1) begins at byte 1024: the root entry names "Root Entry".
    let root_name: Vec<u16> = (0..10)
        .map(|i| u16::from_le_bytes([cfb[1024 + i * 2], cfb[1024 + i * 2 + 1]]))
        .collect();
    assert_eq!(String::from_utf16(&root_name).unwrap(), "Root Entry");
    assert_eq!(cfb[1024 + 66], 0x05, "root entry is a root storage");
}

#[test]
fn ole_objects_reports_the_embedded_object() {
    let mut pres = Presentation::open(&fixture("ole.pptx")).expect("open");
    assert_eq!(
        pres.ole_objects(OLE_SURFACE).expect("ole objects"),
        vec![OleObject {
            shape_index: OLE_SHAPE,
            target: "../embeddings/oleObject1.bin".to_owned(),
            external: false,
            prog_id: Some("Excel.Sheet.12".to_owned()),
        }]
    );
}

#[test]
fn ole_objects_flags_an_external_object() {
    let mut pres = ole_with_external_object();
    let objects = pres.ole_objects(OLE_SURFACE).expect("ole objects");
    assert_eq!(objects.len(), 1);
    assert!(objects[0].external, "the linked object is flagged external");
    assert_eq!(objects[0].target, "file:///C:/data/Worksheet.xlsx");
    // An external object's bytes are unreachable today.
    assert!(matches!(
        pres.ole_object_part_bytes(OLE_SURFACE, OLE_SHAPE),
        Err(PptxError::ExternalTarget { .. })
    ));
}

#[test]
fn replacing_an_external_ole_object_resolves_it_in_package() {
    let mut pres = ole_with_external_object();
    pres.replace_ole_object_with_placeholder(OLE_SURFACE, OLE_SHAPE, None)
        .expect("replace");

    // No longer external, and the object now resolves to the placeholder compound file.
    assert!(pres
        .ole_objects(OLE_SURFACE)
        .expect("ole objects")
        .iter()
        .all(|o| !o.external));
    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reopened
            .ole_object_part_bytes(OLE_SURFACE, OLE_SHAPE)
            .expect("bytes")
            .map(<[u8]>::to_vec),
        Some(default_placeholder_ole()),
        "the object data is the default placeholder"
    );
    // The frame still reads as an OLE object.
    assert_eq!(
        reopened
            .graphic_frame_kind(OLE_SURFACE, OLE_SHAPE)
            .expect("kind"),
        Some(GraphicFrameKind::OleObject)
    );
}

#[test]
fn replacing_an_ole_object_can_use_caller_supplied_bytes() {
    let mut pres = ole_with_external_object();
    let custom = b"my own compound file".to_vec();
    pres.replace_ole_object_with_placeholder(OLE_SURFACE, OLE_SHAPE, Some(&custom))
        .expect("replace");
    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reopened
            .ole_object_part_bytes(OLE_SURFACE, OLE_SHAPE)
            .expect("bytes")
            .map(<[u8]>::to_vec),
        Some(custom)
    );
}

#[test]
fn replacing_an_embedded_ole_object_leaves_the_old_part_sweepable() {
    let mut pres = Presentation::open(&fixture("ole.pptx")).expect("open");
    pres.replace_ole_object_with_placeholder(OLE_SURFACE, OLE_SHAPE, None)
        .expect("replace");

    let mut pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    let removed = pkg.remove_unreferenced_parts().expect("sweep");
    assert!(
        removed
            .iter()
            .any(|p| p.as_str() == "/ppt/embeddings/oleObject1.bin"),
        "the replaced embedded object should be swept: {removed:?}"
    );
}

#[test]
fn replacing_the_object_of_a_non_ole_shape_is_rejected() {
    let mut pres = Presentation::open(&fixture("ole.pptx")).expect("open");
    // Shape 0 is the title text box.
    assert!(matches!(
        pres.replace_ole_object_with_placeholder(OLE_SURFACE, 0, None),
        Err(PptxError::ShapeIsNotAnOleObject)
    ));
    assert!(pres.ole_objects(OLE_SURFACE).expect("ole objects").len() == 1);
}
