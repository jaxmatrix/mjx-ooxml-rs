//! Integration tests for SmartArt diagrams (MJX-140): the four parts a diagram is made of, the
//! relationship graph between them and the frame, authoring one, and replacing one part without
//! disturbing the rest.
//!
//! No fixture is committed: a diagram is something this library now *writes*, so every test builds
//! one and reads it back — which proves the writer and the reader agree, and keeps the binary
//! fixtures to the ones we could not otherwise obtain.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_opc::{Package, PartName, Relationship, TargetMode};
use mjx_pptx::{
    DiagramContent, DiagramPartKind, GraphicFrameKind, PptxError, Presentation, ShapeBounds,
};

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
    ShapeBounds::from_inches(1.0, 1.0, 4.0, 3.0)
}

/// A deck with one authored diagram on slide 1, and that diagram's shape index.
fn deck_with_a_diagram() -> (Presentation, usize) {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres
        .add_diagram(
            0,
            &DiagramContent::vertical_list(&["Plan", "Build"]),
            bounds(),
        )
        .expect("add diagram");
    (pres, shape)
}

// ---------------------------------------------------------------------------------------------
// Reading — the relationship graph
// ---------------------------------------------------------------------------------------------

#[test]
fn an_added_diagram_is_a_diagram_frame() {
    let (mut pres, shape) = deck_with_a_diagram();
    assert_eq!(
        pres.graphic_frame_kind(0, shape).expect("kind"),
        Some(GraphicFrameKind::Diagram),
        "the authored frame declares the diagram graphic URI"
    );
    // The title text box is not a graphic frame at all.
    assert_eq!(pres.graphic_frame_kind(0, 0).expect("kind"), None);
}

#[test]
fn a_diagram_frame_names_four_distinct_relationships() {
    let (mut pres, shape) = deck_with_a_diagram();
    let ids = pres
        .diagram_relationship_ids(0, shape)
        .expect("ids")
        .expect("the frame carries a dgm:relIds");

    let all = [&ids.data, &ids.layout, &ids.style, &ids.colors];
    for (label, id) in ["dm", "lo", "qs", "cs"].into_iter().zip(all) {
        assert!(id.is_some(), "r:{label} must be present");
    }
    let mut distinct: Vec<&String> = all.into_iter().flatten().collect();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), 4, "the four ids are distinct: {ids:?}");

    // A shape that frames no diagram answers None rather than an empty record.
    assert_eq!(pres.diagram_relationship_ids(0, 0).expect("ids"), None);
}

#[test]
fn the_four_relationships_resolve_to_the_four_parts() {
    let (mut pres, shape) = deck_with_a_diagram();
    let parts = pres
        .diagram_parts(0, shape)
        .expect("parts")
        .expect("the frame frames a diagram");

    assert_eq!(parts.data, Some(part("/ppt/diagrams/data1.xml")));
    assert_eq!(parts.layout, Some(part("/ppt/diagrams/layout1.xml")));
    assert_eq!(parts.style, Some(part("/ppt/diagrams/quickStyle1.xml")));
    assert_eq!(parts.colors, Some(part("/ppt/diagrams/colors1.xml")));
    assert_eq!(
        parts.drawing, None,
        "an authored diagram writes no cached drawing — PowerPoint regenerates it"
    );
    assert_eq!(parts.all().len(), 4);

    // Each part is really there, with the bytes that were written.
    let data = parts.data.clone().expect("data part");
    let bytes = pres.diagram_part_bytes(&data).expect("data bytes");
    assert!(String::from_utf8_lossy(bytes).contains("<a:t>Plan</a:t>"));
}

#[test]
fn the_cached_drawing_is_found_through_the_data_part_not_the_frame() {
    // The `diagramDrawing` relationship hangs off the *data* part, which is the one thing about the
    // graph a caller cannot guess from the frame. Wire one up by hand and prove it is followed.
    let (pres, shape) = deck_with_a_diagram();
    let saved = pres.save().expect("save");

    let mut pkg = Package::open(&saved).expect("open");
    let drawing = part("/ppt/diagrams/drawing1.xml");
    pkg.insert_part(
        &drawing,
        "application/vnd.ms-office.drawingml.diagramDrawing+xml",
        br#"<dsp:drawing xmlns:dsp="http://schemas.microsoft.com/office/drawing/2008/diagram"/>"#
            .to_vec(),
    )
    .expect("insert drawing");
    pkg.add_relationship(
        Some(&part("/ppt/diagrams/data1.xml")),
        Relationship {
            id: "rId1".to_owned(),
            rel_type: "http://schemas.microsoft.com/office/2007/relationships/diagramDrawing"
                .to_owned(),
            target: "drawing1.xml".to_owned(),
            mode: TargetMode::Internal,
        },
    )
    .expect("relate");

    let mut pres = Presentation::open(&pkg.save().expect("save")).expect("reopen");
    let parts = pres
        .diagram_parts(0, shape)
        .expect("parts")
        .expect("diagram");
    assert_eq!(
        parts.drawing,
        Some(drawing),
        "the cached drawing resolves through the data part's own relationships"
    );
    assert_eq!(parts.all().len(), 5);
}

#[test]
fn reading_a_diagram_dirties_nothing() {
    let (pres, shape) = deck_with_a_diagram();
    let saved = pres.save().expect("save");
    let original = byte_map(&Package::open(&saved).expect("baseline"));

    let mut pres = Presentation::open(&saved).expect("reopen");
    let parts = pres
        .diagram_parts(0, shape)
        .expect("parts")
        .expect("diagram");
    for name in parts.all() {
        pres.diagram_part_bytes(&name).expect("bytes");
    }
    pres.diagram_relationship_ids(0, shape).expect("ids");

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(reopened, original, "reading a diagram must dirty nothing");
}

// ---------------------------------------------------------------------------------------------
// Authoring
// ---------------------------------------------------------------------------------------------

#[test]
fn a_second_diagram_gets_its_own_numbered_set_of_parts() {
    let (mut pres, _) = deck_with_a_diagram();
    let second = pres
        .add_diagram(0, &DiagramContent::vertical_list(&["Ship"]), bounds())
        .expect("add second");

    let parts = pres
        .diagram_parts(0, second)
        .expect("parts")
        .expect("diagram");
    assert_eq!(parts.data, Some(part("/ppt/diagrams/data2.xml")));
    assert_eq!(parts.layout, Some(part("/ppt/diagrams/layout2.xml")));
    assert_eq!(parts.style, Some(part("/ppt/diagrams/quickStyle2.xml")));
    assert_eq!(parts.colors, Some(part("/ppt/diagrams/colors2.xml")));

    // The first diagram still names its own set — the two do not cross.
    let first = pres
        .diagram_parts(0, second - 1)
        .expect("parts")
        .expect("diagram");
    assert_eq!(first.data, Some(part("/ppt/diagrams/data1.xml")));
}

#[test]
fn a_caller_supplied_diagram_is_stored_verbatim() {
    let data = br#"<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"><dgm:ptLst/></dgm:dataModel>"#;
    let content = DiagramContent::from_parts(
        data.to_vec(),
        b"<layout/>".to_vec(),
        b"<style/>".to_vec(),
        b"<colors/>".to_vec(),
    );
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres.add_diagram(0, &content, bounds()).expect("add");

    let parts = pres
        .diagram_parts(0, shape)
        .expect("parts")
        .expect("diagram");
    let stored = pres
        .diagram_part_bytes(&parts.data.clone().expect("data"))
        .expect("bytes");
    assert_eq!(
        stored, data,
        "the caller's bytes are what the package carries"
    );
}

#[test]
fn a_diagram_is_a_shape_and_can_be_moved_and_removed() {
    let (mut pres, shape) = deck_with_a_diagram();
    let moved = ShapeBounds::from_inches(2.0, 2.0, 1.0, 1.0);
    pres.set_shape_bounds(0, shape, moved).expect("move");
    assert_eq!(pres.shape_bounds(0, shape).expect("bounds"), Some(moved));

    let before = pres.shape_count(0).expect("count");
    pres.remove_shape(0, shape).expect("remove");
    assert_eq!(pres.shape_count(0).expect("count"), before - 1);
}

// ---------------------------------------------------------------------------------------------
// Editing
// ---------------------------------------------------------------------------------------------

#[test]
fn replacing_one_diagram_part_leaves_every_other_part_byte_identical() {
    let (pres, shape) = deck_with_a_diagram();
    let saved = pres.save().expect("save");
    let baseline = byte_map(&Package::open(&saved).expect("baseline"));

    let mut pres = Presentation::open(&saved).expect("reopen");
    let relabelled = DiagramContent::vertical_list(&["Plan", "Build", "Ship"]).data;
    pres.set_diagram_part(0, shape, DiagramPartKind::Data, relabelled.clone())
        .expect("replace data");
    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    assert_eq!(
        after.get("ppt/diagrams/data1.xml").map(Vec::as_slice),
        Some(relabelled.as_slice()),
        "the data part carries the new labels"
    );
    for (name, bytes) in &baseline {
        if name == "ppt/diagrams/data1.xml" {
            continue;
        }
        assert_eq!(
            after.get(name),
            Some(bytes),
            "{name} must be byte-identical after a diagram data edit — the slide included"
        );
    }
    assert_eq!(after.len(), baseline.len(), "no part was added or removed");
}

#[test]
fn each_diagram_part_kind_addresses_its_own_part() {
    let (mut pres, shape) = deck_with_a_diagram();
    for (kind, name) in [
        (DiagramPartKind::Data, "ppt/diagrams/data1.xml"),
        (DiagramPartKind::Layout, "ppt/diagrams/layout1.xml"),
        (DiagramPartKind::Style, "ppt/diagrams/quickStyle1.xml"),
        (DiagramPartKind::Colors, "ppt/diagrams/colors1.xml"),
    ] {
        let marker = format!("<marker for=\"{name}\"/>").into_bytes();
        pres.set_diagram_part(0, shape, kind, marker.clone())
            .expect("replace");
        let saved = pres.save().expect("save");
        let map = byte_map(&Package::open(&saved).expect("reopen"));
        assert_eq!(
            map.get(name).map(Vec::as_slice),
            Some(marker.as_slice()),
            "{kind:?} must address {name} and nothing else"
        );
    }
}

#[test]
fn replacing_a_part_the_diagram_does_not_have_is_refused() {
    let (mut pres, shape) = deck_with_a_diagram();
    assert!(matches!(
        pres.set_diagram_part(0, shape, DiagramPartKind::Drawing, b"<x/>".to_vec()),
        Err(PptxError::DiagramPartMissing {
            kind: DiagramPartKind::Drawing
        })
    ));
}

#[test]
fn diagram_methods_refuse_a_shape_that_frames_no_diagram() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert_eq!(pres.diagram_parts(0, 0).expect("parts"), None);
    assert!(matches!(
        pres.set_diagram_part(0, 0, DiagramPartKind::Data, b"<x/>".to_vec()),
        Err(PptxError::ShapeIsNotADiagram)
    ));
}
