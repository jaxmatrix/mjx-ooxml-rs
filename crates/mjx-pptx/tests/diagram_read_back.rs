//! A diagram `add_diagram` writes reads back as a point-and-connection graph (MJXOFF-148) —
//! `mjx-dml::diagram`'s independent reader parsing the bytes `mjx-pptx`'s independent writer
//! produced, which is a genuine cross-check: the two were written by different code, in different
//! crates, for different reasons (`legacy.rs` hand-assembles a fixed template; `mjx-dml::diagram`
//! reads the schema generically). A round-trip test of one against itself could not catch a
//! disagreement between them; this one is built to.
//!
//! `crates/mjx-dml/tests/in_context_roundtrip.rs` already proves the reader against hand-written
//! markup that disagrees with anything this project writes. This file is the other half: proving the
//! reader against what the writer actually produces.

use mjx_dml::diagram::{
    ColorTransform, ConnectionType, DataModel, LayoutDefinition, PointType, StyleDefinition,
};
use mjx_ooxml_core::{FromXml, Interner};
use mjx_pptx::{DiagramContent, Presentation, ShapeBounds};
use mjx_xml::fidelity;

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

/// Parses `bytes` (a whole part) as `T`, returning it with the [`Interner`] that parsed it — every
/// accessor on the result needs that same interner to resolve the names and values it read.
fn parse_part<T: FromXml>(bytes: &[u8]) -> (T, Interner) {
    let document = fidelity::parse(bytes).expect("the part is well-formed XML");
    let typed = T::from_xml(&document.root, &document.interner)
        .expect("the part is this type's root element");
    (typed, document.interner)
}

#[test]
fn an_authored_diagrams_data_part_reads_back_as_a_point_and_connection_graph() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres
        .add_diagram(
            0,
            &DiagramContent::vertical_list(&["Plan", "Build", "Ship"]),
            ShapeBounds::from_inches(1.0, 1.0, 4.0, 3.0),
        )
        .expect("add diagram");
    let parts = pres
        .diagram_parts(0, shape)
        .expect("parts")
        .expect("the frame frames a diagram");
    let data_bytes = pres
        .diagram_part_bytes(&parts.data.clone().expect("data part"))
        .expect("data bytes")
        .to_vec();

    let (model, interner) = parse_part::<DataModel>(&data_bytes);

    // The graph shape `vertical_list` documents: one document-root point, and per label a node plus
    // the parent/sibling transition points PowerPoint expects, joined to the root by a `parOf`
    // connection.
    let points: Vec<_> = model.points().expect("dgm:ptLst").points().collect();
    assert_eq!(
        points.len(),
        1 + 3 * 3,
        "one root, three labels of three points each"
    );

    let root = model
        .points()
        .expect("dgm:ptLst")
        .point_by_id(&interner, "1")
        .expect("the document point (modelId 1)");
    assert_eq!(root.point_type(&interner), Ok(PointType::Document));

    // The three labelled nodes, in document order — `point_type` defaults to `node` per the schema,
    // which `vertical_list`'s writer relies on by never writing `@type` on them at all.
    let labels: Vec<String> = points
        .iter()
        .filter(|pt| pt.point_type(&interner) == Ok(PointType::Node))
        .filter_map(|pt| pt.text_content())
        .collect();
    assert_eq!(
        labels,
        vec!["Plan".to_owned(), "Build".to_owned(), "Ship".to_owned()],
        "the three node points carry the three labels, in order, as their text"
    );

    // The graph's edges: one `parOf` connection per label, from the document root to that label's
    // node — the point-and-connection *graph*, not just a flat list of typed elements.
    let connections: Vec<_> = model
        .connections()
        .expect("dgm:cxnLst")
        .connections()
        .collect();
    assert_eq!(connections.len(), 3, "one connection per label");
    for connection in &connections {
        assert_eq!(
            connection.connection_type(&interner),
            Ok(ConnectionType::ParentOf),
            "@type defaults to parOf, which the writer relies on by never writing it"
        );
        assert_eq!(
            connection
                .source_id(&interner)
                .expect("required @srcId")
                .as_ref(),
            "1",
            "every edge starts at the document root"
        );
    }

    // Walking the graph from the root reaches exactly the three node points that carry the labels —
    // proving `connections_from` is real adjacency, not just a list this crate happens to also hold.
    let destination_labels: Vec<String> = model
        .connections()
        .expect("dgm:cxnLst")
        .connections_from(&interner, "1")
        .filter_map(|cxn| cxn.destination_id(&interner).ok())
        .filter_map(|id| {
            model
                .points()
                .expect("dgm:ptLst")
                .point_by_id(&interner, &id)
        })
        .filter_map(|pt| pt.text_content())
        .collect();
    assert_eq!(
        destination_labels,
        vec!["Plan".to_owned(), "Build".to_owned(), "Ship".to_owned()],
        "walking the root's outgoing edges reaches the three labelled nodes, in order"
    );
}

#[test]
fn an_authored_diagrams_other_three_parts_parse_as_their_typed_roots() {
    // The data part is the load-bearing one, proved above; this pins that the other three are not
    // opaque byte blobs either — each parses as the type its own root element names.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres
        .add_diagram(
            0,
            &DiagramContent::vertical_list(&["Plan"]),
            ShapeBounds::from_inches(1.0, 1.0, 4.0, 3.0),
        )
        .expect("add diagram");
    let parts = pres
        .diagram_parts(0, shape)
        .expect("parts")
        .expect("diagram");

    let layout_bytes = pres
        .diagram_part_bytes(&parts.layout.clone().expect("layout part"))
        .expect("layout bytes")
        .to_vec();
    let (layout, _) = parse_part::<LayoutDefinition>(&layout_bytes);
    assert!(layout.root().is_some(), "the layout's algorithm tree root");

    let style_bytes = pres
        .diagram_part_bytes(&parts.style.clone().expect("style part"))
        .expect("style bytes")
        .to_vec();
    let (style, _) = parse_part::<StyleDefinition>(&style_bytes);
    assert_eq!(style.style_labels().count(), 1, "one quick-style label");

    let colors_bytes = pres
        .diagram_part_bytes(&parts.colors.clone().expect("colors part"))
        .expect("colors bytes")
        .to_vec();
    let (colors, _) = parse_part::<ColorTransform>(&colors_bytes);
    assert_eq!(
        colors.style_labels().count(),
        1,
        "one colour-transform label"
    );
}
