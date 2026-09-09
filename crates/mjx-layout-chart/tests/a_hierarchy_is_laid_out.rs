//! SmartArt: a real hierarchy, with node positions asserted.
//!
//! # Why the fixture branches unevenly
//!
//! A single-node diagram lays out identically under every algorithm, and so does a perfectly
//! balanced tree: three children each with two grandchildren would place identically whether the
//! engine split a parent's span equally between its children or in proportion to how many leaves each
//! carried. Only an **uneven** tree tells the two apart.
//!
//! So the fixture is three levels with uneven branching — a root, three children, and 3/1/2
//! grandchildren — and [`a_wide_branch_gets_more_room_than_a_narrow_one`] asserts the difference an
//! equal split would not produce.

use mjx_dml::diagram::data::DataModel;
use mjx_dml::diagram::LayoutDefinition;
use mjx_layout::LayoutRect;
use mjx_layout_chart::{lay_out_diagram, ChartLayoutError};
use mjx_ooxml_core::{measure::Emu, FromXml};
use mjx_ooxml_types::diagram::AlgorithmType;

/// The frame every diagram assertion is made in: ten inches by six.
fn frame() -> LayoutRect {
    LayoutRect::from_edges(
        Emu::ZERO,
        Emu::ZERO,
        Emu::from_inches(10.0),
        Emu::from_inches(6.0),
    )
}

/// A `dgm:dataModel` of `points` and `connections`.
fn data_model(points: &str, connections: &str) -> (DataModel, mjx_ooxml_core::Interner) {
    let xml = format!(
        r#"<?xml version="1.0"?>
<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <dgm:ptLst>{points}</dgm:ptLst><dgm:cxnLst>{connections}</dgm:cxnLst>
</dgm:dataModel>"#
    );
    let document = mjx_xml::fidelity::parse(xml.as_bytes()).expect("well-formed");
    let model = DataModel::from_xml(&document.root, &document.interner).expect("a data model");
    (model, document.interner)
}

/// A `dgm:pt` of `id` carrying `text`.
fn point(id: &str, text: &str) -> String {
    format!(
        r#"<dgm:pt modelId="{id}" type="node"><dgm:t><a:bodyPr/><a:p><a:r><a:t>{text}</a:t></a:r></a:p></dgm:t></dgm:pt>"#
    )
}

/// A `parOf` `dgm:cxn` from `parent` to `child`, ranked `order` among its siblings.
fn parent_of(id: &str, parent: &str, child: &str, order: u32) -> String {
    format!(
        r#"<dgm:cxn modelId="{id}" type="parOf" srcId="{parent}" destId="{child}" srcOrd="{order}" destOrd="0"/>"#
    )
}

/// The uneven three-level tree: a root, three children, and 3 / 1 / 2 grandchildren.
fn uneven_tree() -> (DataModel, mjx_ooxml_core::Interner) {
    let mut points = point("root", "Board");
    let mut connections = String::new();
    let branches = [("a", 3), ("b", 1), ("c", 2)];
    for (order, (branch, children)) in branches.iter().enumerate() {
        points.push_str(&point(branch, branch));
        connections.push_str(&parent_of(
            &format!("cx-{branch}"),
            "root",
            branch,
            u32::try_from(order).unwrap_or(0),
        ));
        for child in 0..*children {
            let id = format!("{branch}{child}");
            points.push_str(&point(&id, &id));
            connections.push_str(&parent_of(&format!("cx-{id}"), branch, &id, child));
        }
    }
    data_model(&points, &connections)
}

/// A `dgm:layoutDef` whose root node runs `algorithm`.
fn definition(algorithm: &str) -> (LayoutDefinition, mjx_ooxml_core::Interner) {
    let xml = format!(
        r#"<?xml version="1.0"?>
<dgm:layoutDef xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" uniqueId="urn:test">
  <dgm:layoutNode name="root"><dgm:alg type="{algorithm}"/></dgm:layoutNode>
</dgm:layoutDef>"#
    );
    let document = mjx_xml::fidelity::parse(xml.as_bytes()).expect("well-formed");
    let definition =
        LayoutDefinition::from_xml(&document.root, &document.interner).expect("a definition");
    (definition, document.interner)
}

#[test]
fn a_three_level_tree_gets_one_row_per_level() {
    let (data, interner) = uneven_tree();
    let layout = lay_out_diagram(&data, None, &interner, frame()).expect("the hierarchy lays out");
    assert_eq!(layout.algorithm, AlgorithmType::HierarchyRoot);
    assert_eq!(
        layout.nodes.len(),
        1 + 3 + 6,
        "root, three branches, six leaves"
    );

    assert_eq!(layout.at_depth(0).count(), 1);
    assert_eq!(layout.at_depth(1).count(), 3);
    assert_eq!(layout.at_depth(2).count(), 6);

    // Each level occupies its own horizontal band, and the bands do not overlap.
    let row_of = |depth: usize| {
        let node = layout.at_depth(depth).next().expect("a node at this depth");
        (node.rect.top, node.rect.bottom)
    };
    let (top0, bottom0) = row_of(0);
    let (top1, bottom1) = row_of(1);
    let (top2, bottom2) = row_of(2);
    assert!(
        bottom0 <= top1,
        "the root's row must sit above its children's"
    );
    assert!(bottom1 <= top2);
    assert!(top0 >= frame().top && bottom2 <= frame().bottom);

    // The root spans the whole frame; every node sits inside it.
    let root = layout.node("root").expect("the root");
    for node in &layout.nodes {
        assert!(node.rect.left >= frame().left && node.rect.right <= frame().right);
        assert!(node.rect.right > node.rect.left, "a node must have width");
    }
    assert!(
        (root.rect.right - root.rect.left).emu() as f64
            > (frame().right - frame().left).emu() as f64 * 0.7,
        "the root spans nearly the whole frame"
    );
}

#[test]
fn a_wide_branch_gets_more_room_than_a_narrow_one() {
    let (data, interner) = uneven_tree();
    let layout = lay_out_diagram(&data, None, &interner, frame()).expect("lays out");
    let width = |id: &str| {
        let node = layout.node(id).unwrap_or_else(|| panic!("no node {id}"));
        (node.rect.right - node.rect.left).emu() as f64
    };
    let (a, b, c) = (width("a"), width("b"), width("c"));
    assert!(
        a > c && c > b,
        "the branch with three leaves must be widest and the one with one narrowest; got \
         a={a}, b={b}, c={c} — an equal split would have made all three the same"
    );
    // Three leaves against one: a is close to three times b.
    assert!(
        (a / b - 3.0).abs() < 0.3,
        "the widths must follow the leaf counts; a/b was {}",
        a / b
    );
}

#[test]
fn siblings_are_placed_in_order_and_do_not_overlap() {
    let (data, interner) = uneven_tree();
    let layout = lay_out_diagram(&data, None, &interner, frame()).expect("lays out");
    let level: Vec<_> = layout.at_depth(1).collect();
    assert_eq!(level.len(), 3);
    for window in level.windows(2) {
        assert!(
            window[0].rect.right <= window[1].rect.left,
            "two siblings overlap: {:?} and {:?}",
            window[0].rect,
            window[1].rect
        );
    }
    // A child sits under its own parent's span, not under a sibling's.
    let a = layout.node("a").expect("branch a");
    for child in ["a0", "a1", "a2"] {
        let node = layout.node(child).expect(child);
        assert!(
            node.rect.left >= a.rect.left - Emu::from_inches(1.0)
                && node.rect.right <= a.rect.right + Emu::from_inches(1.0),
            "{child} at {:?} is not under its parent's span {:?}",
            node.rect,
            a.rect
        );
    }
}

#[test]
fn the_edges_are_the_parent_of_connections() {
    let (data, interner) = uneven_tree();
    let layout = lay_out_diagram(&data, None, &interner, frame()).expect("lays out");
    assert_eq!(layout.edges.len(), 9, "three branches and six leaves");
    for edge in &layout.edges {
        let parent = &layout.nodes[edge.from];
        let child = &layout.nodes[edge.to];
        assert_eq!(child.depth, parent.depth + 1);
    }
}

#[test]
fn the_text_of_each_point_reaches_the_caller() {
    let (data, interner) = uneven_tree();
    let layout = lay_out_diagram(&data, None, &interner, frame()).expect("lays out");
    assert_eq!(
        layout
            .node("root")
            .and_then(|node| node.text.clone())
            .as_deref(),
        Some("Board")
    );
    assert_eq!(
        layout
            .node("a0")
            .and_then(|node| node.text.clone())
            .as_deref(),
        Some("a0")
    );
}

#[test]
fn a_linear_definition_lays_out_a_row() {
    let (data, interner) = uneven_tree();
    let (definition, _) = definition("lin");
    let layout = lay_out_diagram(&data, Some(&definition), &interner, frame())
        .expect("a linear diagram lays out");
    assert_eq!(layout.algorithm, AlgorithmType::Linear);
    // Every node in one row, in order, none overlapping.
    let tops: Vec<_> = layout.nodes.iter().map(|node| node.rect.top).collect();
    assert!(
        tops.windows(2).all(|pair| pair[0] == pair[1]),
        "a row is one row"
    );
    for window in layout.nodes.windows(2) {
        assert!(window[0].rect.right <= window[1].rect.left);
    }
}

#[test]
fn a_cycle_definition_places_the_first_node_at_twelve_oclock() {
    let (data, interner) = uneven_tree();
    let (definition, _) = definition("cycle");
    let layout = lay_out_diagram(&data, Some(&definition), &interner, frame())
        .expect("a cycle diagram lays out");
    assert_eq!(layout.algorithm, AlgorithmType::Cycle);
    let first = &layout.nodes[0];
    let centre_x = (frame().left + frame().right).divided_by(2);
    let centre_y = (frame().top + frame().bottom).divided_by(2);
    let first_centre_x = (first.rect.left + first.rect.right).divided_by(2);
    let first_centre_y = (first.rect.top + first.rect.bottom).divided_by(2);
    assert_eq!(
        first_centre_x, centre_x,
        "the first node is straight up from the centre"
    );
    assert!(first_centre_y < centre_y, "and above it");
}

#[test]
fn the_seven_algorithms_this_engine_does_not_evaluate_are_refused_by_name() {
    let (data, interner) = uneven_tree();
    for algorithm in ["pyra", "snake", "composite", "sp", "conn"] {
        let (definition, _) = definition(algorithm);
        let result = lay_out_diagram(&data, Some(&definition), &interner, frame());
        match result {
            Err(ChartLayoutError::DiagramAlgorithmNotEvaluated { algorithm: named }) => {
                assert_eq!(
                    named, algorithm,
                    "the refusal must name the algorithm the file asked for"
                );
            }
            other => panic!(
                "`{algorithm}` must be refused rather than approximated with a row; got {other:?}"
            ),
        }
    }
}

#[test]
fn a_data_model_with_no_drawable_point_is_an_error() {
    // A `pres` point is the presentation graph the layout definition builds, not a shape to draw.
    let (data, interner) = data_model(r#"<dgm:pt modelId="p" type="pres"/>"#, "");
    assert!(matches!(
        lay_out_diagram(&data, None, &interner, frame()),
        Err(ChartLayoutError::DiagramHasNoPoints)
    ));
}

#[test]
fn a_cyclic_parent_graph_terminates() {
    // Two points each claiming to be the other's parent. A file that has been through a repair tool
    // can carry one, and a naive walk over it never returns.
    let (data, interner) = data_model(
        &format!("{}{}", point("x", "X"), point("y", "Y")),
        &format!(
            "{}{}",
            parent_of("c1", "x", "y", 0),
            parent_of("c2", "y", "x", 0)
        ),
    );
    let layout = lay_out_diagram(&data, None, &interner, frame()).expect("it terminates");
    assert_eq!(layout.nodes.len(), 2);
}
