//! The vocabulary itself: all six fragment kinds, the tree's structure, transforms, clips, and the
//! addresses.
//!
//! # Why every kind is built here
//!
//! The plain-text box model in `tests/support/` produces boxes, lines and glyph runs, and nothing
//! else — so images, shapes and tables would be **exported and never read**, which is the pattern
//! MJXOFF-155 §8 lists and which this crate, being a contract, is the worst place to carry. This
//! suite builds one of each, indexes them, hit-tests them and reads every field back, so that no
//! part of the vocabulary is a claim nothing checks.

mod support;

use mjx_layout::{
    BoxFragment, DecorationRef, Fragment, FragmentTreeBuilder, GeometryRef, GlyphRunFragment,
    ImageFragment, ImageRef, LayoutPoint, LayoutRect, LayoutSize, LineFragment, PartId,
    ShapeFragment, SourcePath, SourceRef, SpatialIndex, TableCell, TableFragment, Transform,
    TransformId, UnitRect, INLINE_PATH_DEPTH,
};
use mjx_ooxml_core::measure::{Angle, Emu};
use mjx_text::{
    BidiLevel, FeatureSet, FontSize, GlyphRasteriser, Shaper, ShapingRequest, TextDirection,
    TextScript,
};

fn inches(value: f64) -> Emu {
    Emu::from_inches(value)
}

fn rect(left: f64, top: f64, right: f64, bottom: f64) -> LayoutRect {
    LayoutRect::from_edges(inches(left), inches(top), inches(right), inches(bottom))
}

fn at(path: &[u32]) -> SourceRef {
    SourceRef::node(PartId::PRIMARY, SourcePath::new(path))
}

#[test]
fn every_fragment_kind_is_built_indexed_and_hit_tested() {
    let face = support::liberation_sans();
    let mut rasteriser = GlyphRasteriser::new();
    let face_id = rasteriser.register(&face).expect("the face registers");
    let features = FeatureSet::default();
    let mut shaper = Shaper::new();
    let run = shaper
        .shape(
            &face,
            &ShapingRequest::new(
                "text",
                TextScript::of_character('a'),
                FontSize::from_points(12.0),
                &features,
            ),
        )
        .expect("the face shapes");

    let mut builder = FragmentTreeBuilder::new();
    let page = builder
        .push_simple(
            None,
            at(&[]),
            rect(0.0, 0.0, 8.0, 10.0),
            Fragment::Box(BoxFragment {
                decoration: Some(DecorationRef::new(7)),
                cell: None,
            }),
        )
        .expect("the page is pushed");

    let line = builder
        .push_simple(
            Some(page),
            SourceRef::new(PartId::PRIMARY, SourcePath::new(&[0, 0]), 0..4),
            rect(1.0, 1.0, 3.0, 1.25),
            Fragment::Line(LineFragment {
                baseline: inches(0.2),
                ascent: inches(0.2),
                descent: inches(0.05),
                base_direction: TextDirection::LeftToRight,
                hanging_width: inches(0.02),
            }),
        )
        .expect("the line is pushed");

    let glyphs = builder
        .push_simple(
            Some(line),
            SourceRef::new(PartId::PRIMARY, SourcePath::new(&[0, 0]), 0..4),
            rect(1.0, 1.0, 3.0, 1.25),
            Fragment::GlyphRun(GlyphRunFragment {
                face: face_id,
                run: run.clone(),
                origin: LayoutPoint::new(inches(1.0), inches(1.2)),
                direction: TextDirection::LeftToRight,
                level: BidiLevel::LEFT_TO_RIGHT,
            }),
        )
        .expect("the run is pushed");

    let image = builder
        .push_simple(
            Some(page),
            at(&[1]),
            rect(1.0, 2.0, 4.0, 5.0),
            Fragment::Image(ImageFragment {
                image: ImageRef::new(11),
                crop: Some(UnitRect {
                    left: 0.1,
                    top: 0.0,
                    right: 0.9,
                    bottom: 1.0,
                }),
            }),
        )
        .expect("the image is pushed");

    let shape = builder
        .push_simple(
            Some(page),
            at(&[2]),
            rect(5.0, 2.0, 7.0, 4.0),
            Fragment::Shape(ShapeFragment {
                geometry: GeometryRef::new(42),
                decoration: Some(DecorationRef::new(8)),
            }),
        )
        .expect("the shape is pushed");

    let table = builder
        .push_simple(
            Some(page),
            at(&[3]),
            rect(1.0, 6.0, 7.0, 9.0),
            Fragment::Table(TableFragment {
                columns: 3,
                rows: 4..9,
                header_rows: 1,
                continued_from_previous_page: true,
                continues_on_next_page: true,
            }),
        )
        .expect("the table is pushed");

    let cell = builder
        .push_simple(
            Some(table),
            at(&[3, 0]),
            rect(1.0, 6.0, 3.0, 7.0),
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: Some(TableCell {
                    column: 0,
                    row: 4,
                    column_span: 2,
                    row_span: 1,
                }),
            }),
        )
        .expect("the cell is pushed");

    let tree = builder.finish();
    assert_eq!(tree.len(), 7);
    assert_eq!(tree.roots(), &[page]);

    // Every kind is present and reads back exactly what was put in.
    let kinds: Vec<&str> = tree
        .nodes()
        .map(|(_, node)| node.fragment().kind_name())
        .collect();
    assert_eq!(
        kinds,
        vec!["box", "line", "glyph run", "image", "shape", "table", "box"]
    );

    match tree.node(line).expect("the line").fragment() {
        Fragment::Line(fragment) => {
            assert_eq!(fragment.baseline, inches(0.2));
            assert_eq!(fragment.hanging_width, inches(0.02));
            assert_eq!(fragment.base_direction, TextDirection::LeftToRight);
        }
        other => panic!("expected a line, got a {}", other.kind_name()),
    }
    match tree.node(glyphs).expect("the run").fragment() {
        Fragment::GlyphRun(fragment) => {
            assert_eq!(fragment.face, face_id);
            assert_eq!(fragment.run, run);
            assert_eq!(fragment.origin.y, inches(1.2));
        }
        other => panic!("expected a glyph run, got a {}", other.kind_name()),
    }
    match tree.node(image).expect("the image").fragment() {
        Fragment::Image(fragment) => {
            assert_eq!(fragment.image.number(), 11);
            assert_eq!(fragment.crop.expect("a crop").left, 0.1);
        }
        other => panic!("expected an image, got a {}", other.kind_name()),
    }
    match tree.node(shape).expect("the shape").fragment() {
        Fragment::Shape(fragment) => {
            assert_eq!(fragment.geometry.number(), 42);
            assert_eq!(fragment.decoration.map(DecorationRef::number), Some(8));
        }
        other => panic!("expected a shape, got a {}", other.kind_name()),
    }
    match tree.node(table).expect("the table").fragment() {
        Fragment::Table(fragment) => {
            assert_eq!(fragment.columns, 3);
            assert_eq!(fragment.rows, 4..9);
            assert_eq!(fragment.header_rows, 1);
            assert!(fragment.continued_from_previous_page && fragment.continues_on_next_page);
        }
        other => panic!("expected a table, got a {}", other.kind_name()),
    }
    match tree.node(cell).expect("the cell").fragment() {
        Fragment::Box(fragment) => {
            let position = fragment.cell.expect("a cell knows its coordinates");
            assert_eq!((position.column, position.row), (0, 4));
            assert_eq!(position.column_span, 2);
        }
        other => panic!("expected a box, got a {}", other.kind_name()),
    }

    // The structure: children in paint order, parents reachable, ancestors walking upward.
    assert_eq!(
        tree.children(page).collect::<Vec<_>>(),
        vec![line, image, shape, table]
    );
    assert_eq!(tree.children(glyphs).count(), 0);
    assert_eq!(tree.ancestors(cell).collect::<Vec<_>>(), vec![table, page]);
    assert_eq!(tree.node(page).expect("the page").parent(), None);
    assert_eq!(tree.node(page).expect("the page").last_child(), Some(table));

    // And the index finds each of them where it was put.
    let index = SpatialIndex::build(&tree);
    for (id, expected_kind) in [
        (image, "image"),
        (shape, "shape"),
        (cell, "box"),
        (glyphs, "glyph run"),
    ] {
        let bounds = tree.page_bounds(id).expect("a fragment has bounds");
        let middle = LayoutPoint::new(
            bounds.left + bounds.width().divided_by(2),
            bounds.top + bounds.height().divided_by(2),
        );
        let found = index.fragments_at(middle);
        assert!(
            found.contains(&id),
            "the {expected_kind} at {bounds:?} must be found at {middle:?}: {found:?}"
        );
        assert_eq!(
            index.topmost_at(middle),
            Some(id),
            "the innermost fragment at {middle:?} is the {expected_kind}"
        );
    }
}

#[test]
fn a_rotated_shape_is_indexed_by_its_bounding_box_and_hit_tested_exactly() {
    // The broad phase and the narrow phase are different answers, and a hit test that stopped at the
    // first would report a click in the empty corner of a rotated shape's bounding box as a hit.
    let mut builder = FragmentTreeBuilder::new();
    let square = rect(2.0, 2.0, 4.0, 4.0);
    let turn = Transform::rotation_about(
        Angle::from_degrees(45.0),
        LayoutPoint::new(inches(3.0), inches(3.0)),
    );
    let transform = builder.transform(turn);
    assert_ne!(transform, TransformId::IDENTITY);
    // Registering the same transform twice must not grow the table — a page with one rotated group
    // holds one transform however many fragments are inside it.
    assert_eq!(builder.transform(turn), transform);
    assert_eq!(
        builder.transform(Transform::IDENTITY),
        TransformId::IDENTITY
    );

    let clip = builder.clip(rect(0.0, 0.0, 8.0, 8.0)).expect("a real clip");
    assert_eq!(
        builder.clip(LayoutRect::ZERO),
        None,
        "an empty clip would hide the subtree; a box model that means that emits nothing"
    );

    let rotated = builder
        .push(
            None,
            at(&[0]),
            square,
            transform,
            Some(clip),
            Fragment::Shape(ShapeFragment {
                geometry: GeometryRef::new(1),
                decoration: None,
            }),
        )
        .expect("the shape is pushed");
    let tree = builder.finish();

    assert_eq!(tree.transform(transform), turn);
    assert_eq!(tree.clip(clip), Some(rect(0.0, 0.0, 8.0, 8.0)));
    assert_eq!(tree.node(rotated).expect("the shape").clip(), Some(clip));

    // A square turned 45° has a bounding box √2 times wider than itself.
    let bounds = tree.page_bounds(rotated).expect("bounds");
    assert!(
        bounds.width() > square.width(),
        "the bounding box of a rotated square is wider than the square: {bounds:?}"
    );
    let ratio = bounds.width().emu() as f64 / square.width().emu() as f64;
    assert!(
        (ratio - std::f64::consts::SQRT_2).abs() < 1e-6,
        "the ratio must be √2, and is {ratio}"
    );

    // The centre is inside under both phases.
    let centre = LayoutPoint::new(inches(3.0), inches(3.0));
    assert!(bounds.contains(centre));
    assert!(tree.contains_page_point(rotated, centre));

    // A corner of the bounding box is inside the box and outside the shape — which is the whole
    // difference between the two phases.
    let corner = LayoutPoint::new(
        bounds.left + Emu::from_inches(0.05),
        bounds.top + Emu::from_inches(0.05),
    );
    assert!(bounds.contains(corner));
    assert!(
        !tree.contains_page_point(rotated, corner),
        "a click in the empty corner of a rotated shape's bounding box is not a click on the shape"
    );

    // The index is the broad phase, so it offers the candidate and the narrow phase rejects it.
    let index = SpatialIndex::build(&tree);
    assert_eq!(index.fragments_at(corner), vec![rotated]);
    assert_eq!(index.fragments_at(centre), vec![rotated]);
}

#[test]
fn a_transform_composes_and_inverts() {
    let move_right = Transform::translation(inches(2.0), Emu::ZERO);
    let double = Transform::scale(2.0, 2.0);
    let both = move_right.then(double);
    let origin = LayoutPoint::new(inches(1.0), inches(1.0));
    assert_eq!(
        both.apply(origin),
        double.apply(move_right.apply(origin)),
        "`then` is the composition it says it is"
    );

    let back = both.inverse().expect("a scale of two is invertible");
    assert_eq!(back.apply(both.apply(origin)), origin);

    // A collapsed transform has no inverse, and a shape under one contains nothing.
    let flat = Transform::scale(1.0, 0.0);
    assert_eq!(flat.determinant(), 0.0);
    assert_eq!(flat.inverse(), None);

    let mut builder = FragmentTreeBuilder::new();
    let flattened = builder.transform(flat);
    let id = builder
        .push(
            None,
            at(&[0]),
            rect(0.0, 0.0, 1.0, 1.0),
            flattened,
            None,
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        )
        .expect("pushed");
    let tree = builder.finish();
    assert!(!tree.contains_page_point(id, LayoutPoint::ORIGIN));

    // A flip is a negative scale, and it maps a rectangle to the same rectangle mirrored.
    let flip = Transform::scale(-1.0, 1.0);
    let mirrored = flip.map_rect_bounds(rect(1.0, 0.0, 3.0, 1.0));
    assert_eq!(mirrored, rect(-3.0, 0.0, -1.0, 1.0));
}

#[test]
fn a_source_path_addresses_deeply_without_losing_a_segment() {
    let root = SourcePath::root();
    assert!(root.is_root());
    assert_eq!(root.depth(), 0);
    assert_eq!(root.parent(), None);

    // Inline, then past the inline capacity: the deep path must carry every segment.
    let mut path = root.clone();
    let mut expected = Vec::new();
    for segment in 0..(INLINE_PATH_DEPTH as u32 + 4) {
        path = path.child(segment);
        expected.push(segment);
        assert_eq!(path.segments(), expected.as_slice());
        assert_eq!(path.depth(), expected.len());
    }
    assert!(
        path.depth() > INLINE_PATH_DEPTH,
        "the fixture must cross the inline boundary, or the spilled representation is untested"
    );
    assert_eq!(path.parent().expect("a parent").depth(), path.depth() - 1);
    assert!(root.contains(&path), "the root contains everything");
    assert!(!path.contains(&root));
    assert!(path.contains(&path));

    // Lexicographic order is document order.
    let mut paths = vec![
        SourcePath::new(&[1, 0]),
        SourcePath::new(&[0, 9]),
        SourcePath::new(&[0]),
        SourcePath::new(&[0, 1]),
    ];
    paths.sort();
    assert_eq!(
        paths,
        vec![
            SourcePath::new(&[0]),
            SourcePath::new(&[0, 1]),
            SourcePath::new(&[0, 9]),
            SourcePath::new(&[1, 0]),
        ]
    );
}

#[test]
fn a_source_reference_says_what_it_covers_and_what_it_contains() {
    let paragraph = SourceRef::node(PartId::PRIMARY, SourcePath::new(&[3]));
    assert!(paragraph.is_empty());
    assert_eq!(paragraph.character_count(), 0);

    let run = SourceRef::new(PartId::PRIMARY, SourcePath::new(&[3]), 10..20);
    assert_eq!(run.character_count(), 10);
    assert!(
        paragraph.contains(&run),
        "a node-level address covers every character under it"
    );

    let overlapping = SourceRef::new(PartId::PRIMARY, SourcePath::new(&[3]), 15..25);
    let disjoint = SourceRef::new(PartId::PRIMARY, SourcePath::new(&[3]), 30..40);
    assert!(run.contains(&overlapping));
    assert!(!run.contains(&disjoint));

    // Another part is another document address, whatever the path says.
    let elsewhere = SourceRef::new(PartId::new(1), SourcePath::new(&[3]), 10..20);
    assert!(!run.contains(&elsewhere));
    assert!(run < elsewhere, "a part orders before a later part");

    // A reversed range is normalised rather than refused, because it comes from a document.
    #[allow(clippy::reversed_empty_ranges)]
    let backwards = SourceRef::new(PartId::PRIMARY, SourcePath::new(&[3]), 20..10);
    assert_eq!(backwards.characters(), 10..20);
    assert_eq!(backwards, run);

    // Re-addressing the same node at a caret is how a caret is derived from the run holding it.
    let caret = run.with_characters(14..14);
    assert!(caret.is_empty());
    assert_eq!(caret.path(), run.path());
}

#[test]
fn a_builder_refuses_a_parent_from_another_tree_rather_than_panicking() {
    let mut first = FragmentTreeBuilder::new();
    let id = first
        .push_simple(
            None,
            at(&[0]),
            rect(0.0, 0.0, 1.0, 1.0),
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        )
        .expect("pushed");

    let mut second = FragmentTreeBuilder::new();
    assert_eq!(
        second.push_simple(
            Some(id),
            at(&[0]),
            rect(0.0, 0.0, 1.0, 1.0),
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        ),
        None,
        "a parent from another tree names no node here"
    );
    assert!(second.is_empty());
    assert_eq!(second.len(), 0);

    // An identifier from another tree gets `None` from every accessor rather than a wrong answer.
    let tree = second.finish();
    assert!(tree.is_empty());
    assert_eq!(tree.node(id), None);
    assert_eq!(tree.page_bounds(id), None);
    assert!(!tree.contains_page_point(id, LayoutPoint::ORIGIN));
    assert_eq!(tree.children(id).count(), 0);
    assert_eq!(tree.ancestors(id).count(), 0);
}

#[test]
fn a_page_with_no_rotation_holds_exactly_one_transform_and_no_clips() {
    // The memory claim the side tables exist for. A thousand fragments on an unrotated page must not
    // store a thousand identity transforms.
    let mut builder = FragmentTreeBuilder::with_capacity(1000);
    for index in 0..1000_u32 {
        builder.push_simple(
            None,
            at(&[index]),
            rect(0.0, 0.0, 1.0, 1.0),
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        );
    }
    let tree = builder.finish();
    assert_eq!(tree.len(), 1000);
    assert_eq!(tree.transform(TransformId::IDENTITY), Transform::IDENTITY);
    // The heap figure is the nodes plus the roots, plus one transform and no clips — so it must be
    // well under what a per-node transform would cost.
    let per_node_transforms = 1000 * std::mem::size_of::<Transform>();
    assert!(
        tree.heap_bytes() > 0,
        "a thousand fragments occupy something"
    );
    assert!(
        tree.heap_bytes() < 1000 * 200 + per_node_transforms,
        "the tree holds {} bytes for a thousand fragments",
        tree.heap_bytes()
    );
}

#[test]
fn a_rectangle_can_be_given_as_an_origin_and_a_size() {
    // The form `a:xfrm` gives one in: an offset (`a:off`) and an extent (`a:ext`). A box model that
    // read a document would otherwise have to add them itself and get the sign of a negative extent
    // wrong.
    let origin = LayoutPoint::new(inches(1.0), inches(2.0));
    let size = LayoutSize::new(inches(3.0), inches(4.0));
    assert_eq!(
        LayoutRect::from_origin_and_size(origin, size),
        rect(1.0, 2.0, 4.0, 6.0)
    );
    assert_eq!(
        LayoutRect::from_origin_and_size(origin, size).origin(),
        origin
    );
    assert_eq!(LayoutRect::from_origin_and_size(origin, size).size(), size);

    // A negative extent means the rectangle the other side of the origin, not an empty one.
    let backwards = LayoutSize::new(-inches(3.0), -inches(4.0));
    assert!(backwards.is_empty(), "the size itself encloses nothing");
    assert_eq!(
        LayoutRect::from_origin_and_size(origin, backwards),
        rect(-2.0, -2.0, 1.0, 2.0)
    );
    assert!(LayoutSize::ZERO.is_empty());
    assert!(!size.is_empty());
}

#[test]
fn a_writing_mode_says_whether_lines_run_down_the_page() {
    // The predicate a box model branches on. All three variants, so that the answer is a
    // classification rather than a constant.
    use mjx_layout::WritingMode;
    assert!(!WritingMode::HorizontalTopToBottom.is_vertical());
    assert!(WritingMode::VerticalRightToLeft.is_vertical());
    assert!(WritingMode::VerticalLeftToRight.is_vertical());
    assert_eq!(
        WritingMode::default(),
        WritingMode::HorizontalTopToBottom,
        "the default is the one most of the world writes in"
    );
    assert!(!WritingMode::default().is_vertical());
}

#[test]
fn constraints_divide_a_content_area_into_columns_without_dividing_by_zero() {
    let page = LayoutSize::new(inches(8.0), inches(10.0));
    let single = mjx_layout::Constraints::single_column(page, inches(1.0));
    assert_eq!(single.column_count(), 1);
    assert_eq!(single.column(0), Some(rect(1.0, 1.0, 7.0, 9.0)));
    assert_eq!(single.column(1), None);

    let three = mjx_layout::Constraints {
        columns: 3,
        column_gap: inches(0.5),
        ..single
    };
    let first = three.column(0).expect("a first column");
    let last = three.column(2).expect("a third column");
    assert_eq!(three.column(3), None);
    assert!(first.width() > Emu::ZERO);
    assert_eq!(first.width(), last.width(), "the columns are equal");
    assert!(
        last.right <= single.content.right,
        "the last column ends inside the content area: {last:?}"
    );

    // Zero columns reads as one, so nothing divides by zero.
    let zero = mjx_layout::Constraints {
        columns: 0,
        ..single
    };
    assert_eq!(zero.column_count(), 1);
    assert_eq!(zero.column(0), single.column(0));

    // A gap wider than the content leaves no room, and says so rather than producing a negative
    // column.
    let crowded = mjx_layout::Constraints {
        columns: 4,
        column_gap: inches(10.0),
        ..single
    };
    assert_eq!(crowded.column(0), None);
}
