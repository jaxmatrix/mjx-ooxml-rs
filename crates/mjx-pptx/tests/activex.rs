//! Integration tests for ActiveX form controls (MJX-135, tier MJX-137): recognizing the `p:control`s a
//! slide carries (`p:cSld > p:controls`), resolving `p:control@r:id` to the ActiveX control part, its
//! binary blob across the two-hop chain, and its fallback snapshot image — without modeling the control,
//! and with fidelity (the deck round-trips byte-identically and editing a slide leaves the ActiveX parts
//! untouched).
//!
//! The fixture `activex.pptx` is `sample.pptx` plus, on slide 1, a `p:controls > p:control` (`rId2`,
//! type `control`) referencing `ppt/activeX/activeX1.xml`, which relates to `ppt/activeX/activeX1.bin`
//! (`activeXControlBinary`), with a fallback snapshot `ppt/media/image1.png` (`rId3`). Hand-crafted
//! (see MJX-140 for producer-authentic validation).

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_opc::{Package, PartName};
use mjx_pptx::{ActiveXControlSpec, ActiveXPersistence, PptxError, Presentation, ShapeBounds};

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

/// The ActiveX control is on slide 1 (surface 0), at control index 0.
const ACTIVEX_SURFACE: usize = 0;
const ACTIVEX_CONTROL: usize = 0;

/// Every part tied to the ActiveX control that must survive an edit made elsewhere byte-for-byte.
const ACTIVEX_PARTS: &[&str] = &[
    "ppt/activeX/activeX1.xml",
    "ppt/activeX/activeX1.bin",
    "ppt/activeX/_rels/activeX1.xml.rels",
    "ppt/media/image1.png",
    "ppt/slides/_rels/slide1.xml.rels",
];

#[test]
fn a_slide_reports_its_activex_controls() {
    let mut pres = Presentation::open(&fixture("activex.pptx")).expect("open");
    assert_eq!(
        pres.activex_control_count(ACTIVEX_SURFACE).expect("count"),
        1,
        "slide 1 carries one ActiveX control"
    );
    // A deck with no controls reports zero.
    let mut plain = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert_eq!(plain.activex_control_count(0).expect("count"), 0);
}

#[test]
fn activex_control_rel_id_and_name_resolve() {
    let mut pres = Presentation::open(&fixture("activex.pptx")).expect("open");
    assert_eq!(
        pres.activex_control_rel_id(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
            .expect("rel id"),
        Some("rId2".to_owned()),
        "the control names its control-part relationship"
    );
    assert_eq!(
        pres.activex_control_name(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
            .expect("name"),
        Some("CommandButton1".to_owned()),
    );
    // An out-of-range control index answers None, not an error.
    assert_eq!(
        pres.activex_control_rel_id(ACTIVEX_SURFACE, 1)
            .expect("read"),
        None
    );
    assert_eq!(
        pres.activex_control_name(ACTIVEX_SURFACE, 1).expect("read"),
        None
    );
}

#[test]
fn activex_part_bytes_resolves_to_the_verbatim_ocx() {
    let bytes = fixture("activex.pptx");
    let baseline = Package::open(&bytes).expect("baseline");
    let ocx = baseline
        .part_bytes(&part("/ppt/activeX/activeX1.xml"))
        .expect("fixture has a control part")
        .to_vec();

    let mut pres = Presentation::open(&bytes).expect("open");
    assert_eq!(
        pres.activex_part_bytes(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
            .expect("read"),
        Some(ocx.as_slice()),
        "the resolved bytes are exactly the package's ActiveX control part"
    );
    // An out-of-range control index answers None.
    assert_eq!(
        pres.activex_part_bytes(ACTIVEX_SURFACE, 1).expect("read"),
        None
    );
}

#[test]
fn activex_binary_bytes_resolves_across_the_two_hop_chain() {
    let bytes = fixture("activex.pptx");
    let baseline = Package::open(&bytes).expect("baseline");
    let blob = baseline
        .part_bytes(&part("/ppt/activeX/activeX1.bin"))
        .expect("fixture has a control binary")
        .to_vec();

    let mut pres = Presentation::open(&bytes).expect("open");
    assert_eq!(
        pres.activex_binary_bytes(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
            .expect("read"),
        Some(blob.as_slice()),
        "the resolved bytes are exactly the package's ActiveX binary blob"
    );
}

#[test]
fn activex_snapshot_image_bytes_resolves_to_the_verbatim_snapshot() {
    let bytes = fixture("activex.pptx");
    let baseline = Package::open(&bytes).expect("baseline");
    let snapshot = baseline
        .part_bytes(&part("/ppt/media/image1.png"))
        .expect("fixture has a snapshot image")
        .to_vec();

    let mut pres = Presentation::open(&bytes).expect("open");
    assert_eq!(
        pres.activex_snapshot_rel_id(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
            .expect("rel id"),
        Some("rId3".to_owned()),
    );
    assert_eq!(
        pres.activex_snapshot_image_bytes(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
            .expect("read"),
        Some(snapshot.as_slice()),
        "the resolved bytes are exactly the package's snapshot image"
    );
}

#[test]
fn reading_activex_leaves_every_part_byte_identical() {
    let bytes = fixture("activex.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Exercise every read accessor; none may dirty a part.
    pres.activex_control_count(ACTIVEX_SURFACE).expect("count");
    pres.activex_control_rel_id(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
        .expect("rel id");
    pres.activex_control_name(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
        .expect("name");
    pres.activex_part_bytes(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
        .expect("ocx bytes");
    pres.activex_binary_bytes(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
        .expect("binary bytes");
    pres.activex_snapshot_image_bytes(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
        .expect("snapshot bytes");

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(reopened, original, "reading ActiveX must dirty nothing");
}

#[test]
fn editing_a_slide_leaves_the_activex_parts_byte_identical() {
    let bytes = fixture("activex.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Edit the title text — the slide XML changes, but the separate ActiveX parts must not.
    pres.set_shape_text(ACTIVEX_SURFACE, 0, 0, "Edited")
        .expect("set text");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    for &name in ACTIVEX_PARTS {
        assert_eq!(
            reopened.get(name),
            original.get(name),
            "ActiveX part {name} must be untouched by an edit elsewhere"
        );
    }
    assert_ne!(
        reopened.get("ppt/slides/slide1.xml"),
        original.get("ppt/slides/slide1.xml"),
        "the edited slide should have changed"
    );
}

// ---------------------------------------------------------------------------------------------
// MJX-140 — authoring and editing an ActiveX control
// ---------------------------------------------------------------------------------------------

/// A valid 2×2 truecolour PNG (76 bytes), inlined so no binary fixture is committed.
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x02, 0x00, 0x00, 0x00, 0xFD, 0xD4, 0x9A,
    0x73, 0x00, 0x00, 0x00, 0x13, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0x78, 0x60, 0x60, 0x60,
    0x90, 0xF0, 0x80, 0x01, 0x88, 0x81, 0x2C, 0x00, 0x25, 0xAE, 0x05, 0x61, 0x56, 0x69, 0x41, 0x72,
    0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// The Forms 2.0 command button, the control PowerPoint's toolbox inserts most often.
const COMMAND_BUTTON: &str = "{D7053240-CE69-11CD-A777-00DD01143C57}";

fn control_bounds() -> ShapeBounds {
    ShapeBounds::from_inches(1.0, 1.0, 2.0, 0.5)
}

#[test]
fn an_authored_control_reads_back_through_every_accessor() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert_eq!(pres.activex_control_count(0).expect("count"), 0);

    let state = b"persisted control state".as_slice();
    let idx = pres
        .add_activex_control(
            0,
            &ActiveXControlSpec::new("CommandButton1", COMMAND_BUTTON, state, TINY_PNG),
            control_bounds(),
        )
        .expect("add control");

    assert_eq!(idx, 0, "the first control on the surface");
    assert_eq!(pres.activex_control_count(0).expect("count"), 1);
    assert_eq!(
        pres.activex_control_name(0, idx).expect("name").as_deref(),
        Some("CommandButton1")
    );
    assert_eq!(
        pres.activex_class_id(0, idx).expect("class id").as_deref(),
        Some(COMMAND_BUTTON)
    );
    assert_eq!(
        pres.activex_persistence(0, idx).expect("persistence"),
        Some(ActiveXPersistence::Storage)
    );
    assert_eq!(
        pres.activex_binary_bytes(0, idx).expect("state"),
        Some(state),
        "the two-hop chain to the .bin resolves"
    );
    assert_eq!(
        pres.activex_snapshot_image_bytes(0, idx).expect("snapshot"),
        Some(TINY_PNG)
    );

    // Nothing about the control is a shape.
    assert_eq!(
        pres.shape_count(0).expect("count"),
        1,
        "a p:control is a sibling of the shape tree, not a member of it"
    );

    // The graph survives a save/reopen.
    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(reopened.activex_control_count(0).expect("count"), 1);
    assert_eq!(
        reopened.activex_binary_bytes(0, 0).expect("state"),
        Some(state)
    );
}

#[test]
fn a_control_that_persists_nothing_writes_no_binary_and_no_rels_part() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_activex_control(
            0,
            &ActiveXControlSpec {
                name: "Label1",
                class_id: COMMAND_BUTTON,
                persistence: ActiveXPersistence::PropertyBag,
                state: None,
                snapshot_image: TINY_PNG,
            },
            control_bounds(),
        )
        .expect("add control");

    assert_eq!(pres.activex_binary_bytes(0, idx).expect("state"), None);
    assert_eq!(
        pres.activex_persistence(0, idx).expect("persistence"),
        Some(ActiveXPersistence::PropertyBag)
    );
    let map = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert!(
        !map.contains_key("ppt/activeX/activeX1.bin"),
        "no state, no .bin"
    );
    assert!(
        !map.contains_key("ppt/activeX/_rels/activeX1.xml.rels"),
        "and no relationship part to hold a reference that does not exist"
    );
}

#[test]
fn a_second_control_gets_its_own_numbered_parts_and_index() {
    let mut pres = Presentation::open(&fixture("activex.pptx")).expect("open");
    let idx = pres
        .add_activex_control(
            0,
            &ActiveXControlSpec::new("CommandButton2", COMMAND_BUTTON, b"state", TINY_PNG),
            control_bounds(),
        )
        .expect("add control");

    assert_eq!(idx, 1, "it follows the control the fixture already had");
    assert_eq!(pres.activex_control_count(0).expect("count"), 2);
    let map = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert!(map.contains_key("ppt/activeX/activeX2.xml"));
    assert!(map.contains_key("ppt/activeX/activeX2.bin"));
    // The control the fixture already had is untouched.
    assert_eq!(
        pres.activex_control_name(0, 0).expect("name").as_deref(),
        Some("CommandButton1")
    );
}

#[test]
fn a_control_with_an_unrecognizable_snapshot_is_refused_before_anything_is_written() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let original = byte_map(&Package::open(&fixture("sample.pptx")).expect("baseline"));

    assert!(matches!(
        pres.add_activex_control(
            0,
            &ActiveXControlSpec::new("Button", COMMAND_BUTTON, b"state", b"not an image"),
            control_bounds()
        ),
        Err(PptxError::UnrecognizedImageFormat)
    ));

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(
        reopened, original,
        "a refused add must leave the package exactly as it was"
    );
}

#[test]
fn renaming_a_control_rewrites_only_that_attribute() {
    let bytes = fixture("activex.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");

    pres.set_activex_control_name(0, 0, "OkButton")
        .expect("rename");
    let saved = pres.save().expect("save");
    let after = byte_map(&Package::open(&saved).expect("reopen"));

    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened
            .activex_control_name(0, 0)
            .expect("name")
            .as_deref(),
        Some("OkButton")
    );
    for &name in ACTIVEX_PARTS {
        assert_eq!(
            after.get(name),
            original.get(name),
            "{name} must be untouched"
        );
    }
    let slide = String::from_utf8_lossy(after.get("ppt/slides/slide1.xml").expect("slide"));
    assert!(
        slide.contains(r#"spid="_x0000_s1026""#) && slide.contains(r#"imgW="2540000""#),
        "the control's other attributes survive the rewrite, got:\n{slide}"
    );

    assert!(matches!(
        reopened.set_activex_control_name(0, 7, "Nope"),
        Err(PptxError::ActiveXControlOutOfRange { index: 7, count: 1 })
    ));
}

#[test]
fn replacing_the_persisted_state_changes_only_the_binary_part() {
    let bytes = fixture("activex.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");

    pres.set_activex_state(0, 0, b"new state")
        .expect("set state");
    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    assert_eq!(
        after.get("ppt/activeX/activeX1.bin").map(Vec::as_slice),
        Some(b"new state".as_slice())
    );
    for (name, bytes) in &original {
        if name == "ppt/activeX/activeX1.bin" {
            continue;
        }
        assert_eq!(
            after.get(name),
            Some(bytes),
            "{name} must be byte-identical after a control-state edit — the ax:ocx included"
        );
    }
    assert_eq!(after.len(), original.len(), "no part was added or removed");
}

#[test]
fn replacing_the_state_of_a_control_that_persists_none_is_refused() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_activex_control(
        0,
        &ActiveXControlSpec {
            name: "Label1",
            class_id: COMMAND_BUTTON,
            persistence: ActiveXPersistence::PropertyBag,
            state: None,
            snapshot_image: TINY_PNG,
        },
        control_bounds(),
    )
    .expect("add control");
    assert!(matches!(
        pres.set_activex_state(0, 0, b"state"),
        Err(PptxError::ActiveXControlOutOfRange { .. })
    ));
}

#[test]
fn replacing_a_controls_snapshot_leaves_the_slide_markup_alone() {
    let bytes = fixture("activex.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");

    pres.set_activex_snapshot_image(0, 0, TINY_PNG)
        .expect("replace snapshot");
    let saved = pres.save().expect("save");
    let after = byte_map(&Package::open(&saved).expect("reopen"));

    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened
            .activex_snapshot_image_bytes(0, 0)
            .expect("snapshot"),
        Some(TINY_PNG)
    );
    assert_eq!(
        after.get("ppt/slides/slide1.xml"),
        original.get("ppt/slides/slide1.xml"),
        "the p:control markup is untouched — only its relationship moved"
    );
    assert_eq!(
        after.get("ppt/activeX/activeX1.bin"),
        original.get("ppt/activeX/activeX1.bin")
    );
}

#[test]
fn removing_a_control_closes_the_gap_in_the_control_index_space() {
    let mut pres = Presentation::open(&fixture("activex.pptx")).expect("open");
    pres.add_activex_control(
        0,
        &ActiveXControlSpec::new("CommandButton2", COMMAND_BUTTON, b"state", TINY_PNG),
        control_bounds(),
    )
    .expect("add control");
    assert_eq!(pres.activex_control_count(0).expect("count"), 2);

    pres.remove_activex_control(0, 0).expect("remove");
    assert_eq!(pres.activex_control_count(0).expect("count"), 1);
    assert_eq!(
        pres.activex_control_name(0, 0).expect("name").as_deref(),
        Some("CommandButton2"),
        "the second control moved down to index 0"
    );

    assert!(matches!(
        pres.remove_activex_control(0, 5),
        Err(PptxError::ActiveXControlOutOfRange { index: 5, count: 1 })
    ));
}

#[test]
fn the_legacy_shape_id_a_control_names_is_readable() {
    let mut pres = Presentation::open(&fixture("activex.pptx")).expect("open");
    assert_eq!(
        pres.activex_control_shape_id(0, 0)
            .expect("spid")
            .as_deref(),
        Some("_x0000_s1026")
    );
    assert_eq!(pres.activex_control_shape_id(0, 9).expect("spid"), None);
}

#[test]
fn an_authored_control_can_be_bound_to_its_legacy_fallback() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_activex_control(
            0,
            &ActiveXControlSpec::new("CommandButton1", COMMAND_BUTTON, b"state", TINY_PNG),
            control_bounds(),
        )
        .expect("add control");
    assert_eq!(pres.activex_control_shape_id(0, idx).expect("spid"), None);

    pres.set_activex_control_shape_id(0, idx, "_x0000_s1026")
        .expect("bind");
    assert_eq!(
        pres.activex_control_shape_id(0, idx)
            .expect("spid")
            .as_deref(),
        Some("_x0000_s1026")
    );

    assert!(matches!(
        pres.set_activex_control_shape_id(0, 4, "_x0000_s1"),
        Err(PptxError::ActiveXControlOutOfRange { index: 4, count: 1 })
    ));
}
