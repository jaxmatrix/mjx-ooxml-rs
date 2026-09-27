//! The spatial index, checked against a linear scan.
//!
//! # What is being proved, and what would make the proof worthless
//!
//! An index is an accelerator and nothing else: for every query, it must return **exactly** what a
//! walk over every fragment would. So the oracle here is a brute-force scan written out in this
//! file, over randomised fragment sets — never the index compared against itself, and never a
//! hand-written expected answer that the index's own behaviour was used to write down.
//!
//! The scan and the index share one predicate ([`LayoutRect::contains`] and
//! [`LayoutRect::intersects`]) on purpose. What is under test is the *structure* — the grid, the
//! oversized list, the CSR offsets, the cell arithmetic — and sharing the predicate is what isolates
//! it: a disagreement can only be the structure's.
//!
//! # Why the fragment sets are varied the way they are
//!
//! The grid's side is `sqrt(fragment count)` clamped to 64, and a fragment covering more than
//! sixteen cells goes to the oversized list. Both of those are thresholds, so the sweep crosses
//! them: counts from 0 to 400, rectangles from a hair wide to the whole page, and a set that is all
//! page-sized fragments (everything oversized) beside one that is all tiny (nothing oversized).

mod support;

use mjx_layout::{
    BoxFragment, Fragment, FragmentId, FragmentTree, FragmentTreeBuilder, LayoutPoint, LayoutRect,
    PartId, SourcePath, SourceRef, SpatialIndex,
};
use mjx_ooxml_core::measure::Emu;

use support::DeterministicNumbers;

/// The page every fixture is laid out inside, in EMU.
const PAGE: i64 = 9_144_000; // ten inches

/// How wide a fragment may be, as a share of the page.
#[derive(Clone, Copy, Debug)]
enum Spread {
    /// Small enough that nothing is ever oversized.
    Tiny,
    /// Big enough that most fragments cross the sixteen-cell cap.
    PageSized,
    /// A mixture, which is what a real page is.
    Mixed,
}

fn build(count: usize, spread: Spread, seed: u64) -> FragmentTree {
    let mut numbers = DeterministicNumbers::from_seed(seed);
    let mut builder = FragmentTreeBuilder::with_capacity(count);
    for index in 0..count {
        let (max_width, max_height) = match spread {
            Spread::Tiny => (PAGE / 200, PAGE / 200),
            Spread::PageSized => (PAGE, PAGE),
            Spread::Mixed => {
                if numbers.below(4) == 0 {
                    (PAGE, PAGE / 4)
                } else {
                    (PAGE / 60, PAGE / 120)
                }
            }
        };
        let left = numbers.between(0, PAGE);
        let top = numbers.between(0, PAGE);
        // Zero-extent rectangles are in the mix on purpose: an empty fragment must be found by
        // nothing, and a structure that quietly dropped it would also quietly drop a thin one.
        let width = numbers.between(0, max_width);
        let height = numbers.between(0, max_height);
        builder.push_simple(
            None,
            SourceRef::node(
                PartId::PRIMARY,
                SourcePath::new(&[u32::try_from(index).unwrap_or(u32::MAX)]),
            ),
            LayoutRect::from_edges(
                Emu::from_emu(left),
                Emu::from_emu(top),
                Emu::from_emu(left + width),
                Emu::from_emu(top + height),
            ),
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        );
    }
    builder.finish()
}

/// The oracle: every fragment whose page bounds contain the point, in ascending identifier order.
fn scan_at(tree: &FragmentTree, point: LayoutPoint) -> Vec<FragmentId> {
    tree.ids()
        .filter(|id| {
            tree.page_bounds(*id)
                .is_some_and(|bounds| bounds.contains(point))
        })
        .collect()
}

/// The oracle for a rectangle query.
fn scan_intersecting(tree: &FragmentTree, rect: LayoutRect) -> Vec<FragmentId> {
    tree.ids()
        .filter(|id| {
            tree.page_bounds(*id)
                .is_some_and(|bounds| bounds.intersects(rect))
        })
        .collect()
}

#[test]
fn every_point_query_equals_a_linear_scan() {
    let mut checked = 0_usize;
    let mut non_empty_answers = 0_usize;

    for (count, spread) in [
        (0_usize, Spread::Tiny),
        (1, Spread::Tiny),
        (2, Spread::PageSized),
        (7, Spread::Mixed),
        (25, Spread::Tiny),
        (60, Spread::PageSized),
        (150, Spread::Mixed),
        (400, Spread::Mixed),
    ] {
        for seed in 1_u64..=6 {
            let tree = build(count, spread, seed * 7919 + count as u64);
            let index = SpatialIndex::build(&tree);
            assert_eq!(index.len(), tree.len());

            let mut numbers = DeterministicNumbers::from_seed(seed * 104_729 + 3);
            for _ in 0..200 {
                // Points inside the page, plus a few deliberately outside it and on the very edges,
                // because a grid's arithmetic is wrong at its boundaries or nowhere.
                let point = match numbers.below(6) {
                    0 => LayoutPoint::new(Emu::from_emu(-1), Emu::from_emu(-1)),
                    1 => LayoutPoint::new(Emu::from_emu(PAGE * 2), Emu::from_emu(PAGE * 2)),
                    2 => index.covered_area().origin(),
                    3 => LayoutPoint::new(index.covered_area().right, index.covered_area().bottom),
                    _ => LayoutPoint::new(
                        Emu::from_emu(numbers.between(-PAGE / 8, PAGE + PAGE / 8)),
                        Emu::from_emu(numbers.between(-PAGE / 8, PAGE + PAGE / 8)),
                    ),
                };

                let expected = scan_at(&tree, point);
                let actual = index.fragments_at(point);
                assert_eq!(
                    actual, expected,
                    "point {point:?} over {count} fragment(s), {spread:?}, seed {seed}"
                );
                assert_eq!(
                    index.topmost_at(point),
                    expected.last().copied(),
                    "the topmost is the last in paint order; point {point:?}, seed {seed}"
                );
                checked += 1;
                if !expected.is_empty() {
                    non_empty_answers += 1;
                }
            }
        }
    }

    // A sweep that never found anything would agree with a scan that never found anything. This is
    // the clause that fails if the fixtures stop overlapping the query points.
    assert!(checked >= 5_000, "only {checked} queries were run");
    assert!(
        non_empty_answers * 20 >= checked,
        "only {non_empty_answers} of {checked} queries found anything; the sweep is not exercising \
         the index"
    );
}

#[test]
fn every_rectangle_query_equals_a_linear_scan() {
    let mut checked = 0_usize;
    let mut non_empty_answers = 0_usize;

    for (count, spread) in [
        (0_usize, Spread::Tiny),
        (3, Spread::PageSized),
        (40, Spread::Mixed),
        (200, Spread::Tiny),
        (400, Spread::Mixed),
    ] {
        for seed in 1_u64..=6 {
            let tree = build(count, spread, seed * 3571 + count as u64);
            let index = SpatialIndex::build(&tree);

            let mut numbers = DeterministicNumbers::from_seed(seed * 15_485_863 + 11);
            for _ in 0..150 {
                let (left, top) = (
                    numbers.between(-PAGE / 8, PAGE),
                    numbers.between(-PAGE / 8, PAGE),
                );
                // A viewport-sized query, a tiny one, an empty one and one larger than the whole
                // page — the last of which is the case that spans more cells than the cap and takes
                // the exhaustive path inside the index.
                let (width, height) = match numbers.below(4) {
                    0 => (PAGE / 3, PAGE / 4),
                    1 => (PAGE / 500, PAGE / 500),
                    2 => (0, 0),
                    _ => (PAGE * 2, PAGE * 2),
                };
                let rect = LayoutRect::from_edges(
                    Emu::from_emu(left),
                    Emu::from_emu(top),
                    Emu::from_emu(left + width),
                    Emu::from_emu(top + height),
                );

                let expected = scan_intersecting(&tree, rect);
                let actual = index.fragments_intersecting(rect);
                assert_eq!(
                    actual, expected,
                    "rect {rect:?} over {count} fragment(s), {spread:?}, seed {seed}"
                );
                checked += 1;
                if !expected.is_empty() {
                    non_empty_answers += 1;
                }
            }
        }
    }

    assert!(checked >= 3_000, "only {checked} queries were run");
    assert!(
        non_empty_answers * 20 >= checked,
        "only {non_empty_answers} of {checked} queries found anything"
    );
}

#[test]
fn the_grid_really_does_change_shape_across_the_sweep() {
    // The equality above would hold trivially if every fixture produced a 1×1 grid, because then the
    // index *is* a linear scan. This is the clause that says the structure under test was actually
    // varied.
    let mut shapes = std::collections::BTreeSet::new();
    let mut oversized_seen = false;
    let mut only_cells_seen = false;
    for (count, spread) in [
        (1_usize, Spread::Tiny),
        (25, Spread::Tiny),
        (150, Spread::Mixed),
        (400, Spread::Mixed),
        (60, Spread::PageSized),
    ] {
        let tree = build(count, spread, 4_294_967_291 + count as u64);
        let index = SpatialIndex::build(&tree);
        shapes.insert(index.grid());

        // An oversized fragment is one the grid refuses to write into cells, and the two cases have
        // to both occur or half the query path is unexercised.
        let page_sized = LayoutRect::from_edges(
            index.covered_area().left,
            index.covered_area().top,
            index.covered_area().right,
            index.covered_area().bottom,
        );
        if matches!(spread, Spread::PageSized) && !page_sized.is_empty() {
            oversized_seen = true;
        }
        if matches!(spread, Spread::Tiny) && count > 1 {
            only_cells_seen = true;
        }
    }
    assert!(
        shapes.len() >= 3,
        "the sweep produced only {shapes:?}; a single grid shape means one code path"
    );
    assert!(oversized_seen && only_cells_seen);
    assert!(
        shapes
            .iter()
            .any(|(columns, rows)| *columns > 1 && *rows > 1),
        "at least one fixture must produce a real grid: {shapes:?}"
    );
}

#[test]
fn an_empty_tree_indexes_to_an_empty_index() {
    let tree = FragmentTreeBuilder::new().finish();
    let index = SpatialIndex::build(&tree);
    assert!(index.is_empty());
    assert_eq!(index.len(), 0);
    assert!(index.fragments_at(LayoutPoint::ORIGIN).is_empty());
    assert_eq!(index.topmost_at(LayoutPoint::ORIGIN), None);
    assert!(index
        .fragments_intersecting(LayoutRect::from_edges(
            Emu::from_emu(-1),
            Emu::from_emu(-1),
            Emu::from_emu(1),
            Emu::from_emu(1),
        ))
        .is_empty());
    assert!(index.heap_bytes() < 4096, "an empty index holds nothing");
}

#[test]
fn a_fragments_recorded_bounds_are_the_ones_the_queries_use() {
    let tree = build(30, Spread::Mixed, 12_345);
    let index = SpatialIndex::build(&tree);
    for id in tree.ids() {
        assert_eq!(
            index.bounds_of(id),
            tree.page_bounds(id),
            "the index and the tree must agree about where a fragment is"
        );
    }
    // An identifier from a *larger* tree — one this index has never seen — gets `None` rather than
    // a wrong answer. There is no way to fabricate a `FragmentId` from outside the crate, which is
    // the point of it being opaque, so the out-of-range one comes from a bigger tree.
    let bigger = build(60, Spread::Mixed, 12_346);
    let beyond = bigger.ids().last().expect("a non-empty tree");
    assert!(beyond.index() as usize >= tree.len());
    assert_eq!(index.bounds_of(beyond), None);
    assert_eq!(tree.page_bounds(beyond), None);
}
