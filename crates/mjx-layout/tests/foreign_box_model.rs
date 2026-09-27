//! The contract, exercised by a box model that has never heard of OOXML.
//!
//! `tests/support/plain_text.rs` reflows a `Vec<String>` into a fixed-width column. It shares no
//! ancestry with the three real box models, and the whole point of it is that writing it should not
//! have required bending anything in `mjx-layout` — because an interface that only the
//! implementation which shaped it can satisfy is not a contract, it is that implementation's shape
//! written down twice.
//!
//! What it proves here: a real [`FragmentTree`] with real [`SourceRef`]s, pagination, resumption
//! from a [`Checkpoint`], a hit test through the spatial index, an extent estimate and an
//! invalidation. That is every method of the trait and every type in the vocabulary.

mod support;

use mjx_layout::{
    BoxModel, ChangeKind, ChangeSet, Constraints, ContentChange, DirtyPages, ExtentPrecision,
    Fragment, LayoutPoint, LayoutSize, PageIndex, PartId, SourcePath, SourceRef,
};
use mjx_ooxml_core::measure::Emu;
use mjx_text::{FontSize, GlyphRasteriser};

use support::plain_text::{PlainTextColumn, PlainTextDocument};

/// A page the size of a slide, with a one-inch margin.
fn page() -> Constraints {
    Constraints::single_column(
        LayoutSize::new(Emu::from_inches(6.0), Emu::from_inches(3.0)),
        Emu::from_inches(1.0),
    )
}

fn prose() -> PlainTextDocument {
    PlainTextDocument::from_paragraphs([
        "The quick brown fox jumps over the lazy dog, and then it does the whole thing again \
         because that is what a fox in a test paragraph is for.",
        "A second paragraph, shorter than the first, but still long enough that it has to break \
         across more than one line of a narrow column.",
        "",
        "And a third, so that the page boundary has somewhere to fall that is not the end of the \
         document, and so that resuming has something to resume into.",
    ])
}

fn column(rasteriser: &mut GlyphRasteriser) -> PlainTextColumn {
    PlainTextColumn::new(
        rasteriser,
        support::liberation_sans(),
        FontSize::from_points(14.0),
    )
}

/// Lay out every page, in order, and hand back the pages.
fn all_pages(
    model: &mut PlainTextColumn,
    content: &PlainTextDocument,
    constraints: &Constraints,
) -> Vec<mjx_layout::PageFragments> {
    let mut pages = Vec::new();
    let mut resume = None;
    let mut page = PageIndex::FIRST;
    loop {
        let laid_out = model
            .layout_page(content, page, constraints, resume.as_ref())
            .expect("the plain-text column lays out");
        resume = laid_out.continuation().cloned();
        let finished = laid_out.is_last();
        pages.push(laid_out);
        if finished {
            break;
        }
        page = page.next();
        assert!(
            page.number() < 100,
            "the column made no progress; a box model that never finishes is the failure this \
             guard exists for"
        );
    }
    pages
}

#[test]
fn a_box_model_that_has_never_heard_of_ooxml_produces_a_real_fragment_tree() {
    let mut rasteriser = GlyphRasteriser::new();
    let mut model = column(&mut rasteriser);
    let content = prose();
    let constraints = page();

    let pages = all_pages(&mut model, &content, &constraints);
    assert!(
        pages.len() >= 2,
        "the fixture must paginate, or nothing about resumption can be tested: {} page(s)",
        pages.len()
    );

    let first = &pages[0];
    let tree = first.fragments();
    assert!(!tree.is_empty());
    assert_eq!(tree.roots().len(), 1, "one page body");

    // Every kind the model produces is present, and every fragment carries an address.
    let mut lines = 0;
    let mut glyph_runs = 0;
    let mut boxes = 0;
    for (id, node) in tree.nodes() {
        assert_eq!(
            node.source().part(),
            PartId::PRIMARY,
            "every fragment names a part"
        );
        match node.fragment() {
            Fragment::Box(_) => boxes += 1,
            Fragment::Line(line) => {
                lines += 1;
                assert!(
                    line.ascent > Emu::ZERO,
                    "a line with no ascent would draw on top of the line above it"
                );
                assert_eq!(
                    line.baseline, line.ascent,
                    "the baseline sits one ascent below the top of the line's own box"
                );
            }
            Fragment::GlyphRun(run) => {
                glyph_runs += 1;
                assert!(!run.run.is_empty(), "a run with no glyphs draws nothing");
                assert!(
                    !node.source().is_empty(),
                    "a glyph run must name the characters it drew, or a caret cannot be placed \
                     in it"
                );
                let parent = node.parent().and_then(|parent| tree.node(parent));
                assert!(
                    matches!(
                        parent.map(mjx_layout::FragmentNode::fragment),
                        Some(Fragment::Line(_))
                    ),
                    "a glyph run hangs under a line"
                );
                assert!(
                    tree.page_bounds(id)
                        .is_some_and(|bounds| !bounds.is_empty()),
                    "a run that occupies no area cannot be hit-tested"
                );
            }
            other => panic!(
                "the plain-text column does not produce a {}",
                other.kind_name()
            ),
        }
    }
    assert!(boxes >= 2, "a page body and at least one paragraph");
    assert!(lines >= 2, "the column must have wrapped: {lines} line(s)");
    assert_eq!(
        glyph_runs, lines,
        "one run per line, because the model sets everything in one face"
    );

    // The addresses are in document order, which is what makes a caret walk and an invalidation
    // binary search possible.
    let addresses: Vec<SourceRef> = tree
        .nodes()
        .filter(|(_, node)| matches!(node.fragment(), Fragment::Line(_)))
        .map(|(_, node)| node.source().clone())
        .collect();
    let mut sorted = addresses.clone();
    sorted.sort();
    assert_eq!(addresses, sorted, "lines come out in document order");
}

#[test]
fn a_hit_test_finds_the_glyph_run_a_point_is_inside() {
    let mut rasteriser = GlyphRasteriser::new();
    let mut model = column(&mut rasteriser);
    let content = prose();
    let constraints = page();
    let pages = all_pages(&mut model, &content, &constraints);
    let first = &pages[0];

    // Take a real glyph run and ask what is at a point a little way inside it.
    let (run_id, run_bounds) = first
        .fragments()
        .nodes()
        .find(|(_, node)| matches!(node.fragment(), Fragment::GlyphRun(_)))
        .map(|(id, _)| {
            (
                id,
                first.fragments().page_bounds(id).expect("a run has bounds"),
            )
        })
        .expect("the page has a glyph run");

    let inside = LayoutPoint::new(
        run_bounds.left + run_bounds.width().divided_by(2),
        run_bounds.top + run_bounds.height().divided_by(2),
    );
    let found = first.index().fragments_at(inside);
    assert!(
        found.contains(&run_id),
        "the run at {run_bounds:?} must be found at {inside:?}: {found:?}"
    );
    assert_eq!(
        first.index().topmost_at(inside),
        Some(run_id),
        "the innermost, last-painted fragment is the one a click is about"
    );

    // And the answer carries the document address, which is the whole reason a hit test is useful.
    let node = first
        .fragments()
        .node(run_id)
        .expect("the run is in the tree");
    assert_eq!(node.source().part(), PartId::PRIMARY);
    assert!(!node.source().is_empty());

    // A point off the page finds nothing, rather than the nearest thing.
    let outside = LayoutPoint::new(Emu::from_inches(-5.0), Emu::from_inches(-5.0));
    assert!(first.index().fragments_at(outside).is_empty());
    assert_eq!(first.index().topmost_at(outside), None);
}

#[test]
fn the_extent_estimate_is_marked_as_an_estimate_and_is_the_right_order_of_magnitude() {
    let mut rasteriser = GlyphRasteriser::new();
    let mut model = column(&mut rasteriser);
    let content = prose();
    let constraints = page();

    let estimate = model.estimate_extent(&content, &constraints);
    assert_eq!(estimate.precision, ExtentPrecision::Estimated);
    assert_eq!(estimate.page_size, constraints.page);
    assert!(estimate.pages >= 1);

    let real = all_pages(&mut model, &content, &constraints).len() as u32;
    // An estimate is allowed to be wrong; it is not allowed to be wrong by an order of magnitude,
    // because a scrollbar drawn from it would be unusable.
    assert!(
        estimate.pages * 4 >= real && real * 4 >= estimate.pages,
        "estimate {estimate:?} against {real} real page(s)"
    );
}

#[test]
fn an_edit_dirties_the_page_it_lands_on_and_every_page_after_it() {
    let mut rasteriser = GlyphRasteriser::new();
    let mut model = column(&mut rasteriser);
    let content = prose();
    let constraints = page();
    let pages = all_pages(&mut model, &content, &constraints);
    assert!(pages.len() >= 2);

    // An empty change set dirties nothing — which is the clause that fails if `invalidate` simply
    // answered `All`.
    assert_eq!(model.invalidate(&ChangeSet::new()), DirtyPages::None);
    assert!(ChangeSet::new().is_empty());

    // A change in the last paragraph does not dirty the first page.
    let last_paragraph = u32::try_from(content.paragraphs.len() - 1).expect("a small document");
    let mut late = ChangeSet::new();
    late.record(ContentChange {
        source: SourceRef::new(PartId::PRIMARY, SourcePath::new(&[last_paragraph]), 0..1),
        kind: ChangeKind::Inserted,
    });
    let dirty = model.invalidate(&late);
    assert!(
        !dirty.contains(PageIndex::FIRST),
        "an edit on the last page must not reflow the first: {dirty:?}"
    );
    assert!(dirty.contains(PageIndex::new(pages.len() as u32 - 1)));

    // A change in the first paragraph dirties everything, because the content flows.
    let mut early = ChangeSet::new();
    early.record(ContentChange {
        source: SourceRef::new(PartId::PRIMARY, SourcePath::new(&[0]), 0..1),
        kind: ChangeKind::Removed,
    });
    let dirty = model.invalidate(&early);
    for page in 0..pages.len() as u32 {
        assert!(
            dirty.contains(PageIndex::new(page)),
            "an edit on page 1 of flowing content dirties page {}: {dirty:?}",
            page + 1
        );
    }
    assert!(!dirty.is_empty());
}

#[test]
fn a_continuation_page_does_not_indent_its_first_line() {
    // The observable consequence of resumption: a paragraph's first line is indented and its
    // continuation lines are not, so a page that begins mid-paragraph must not indent.
    let mut rasteriser = GlyphRasteriser::new();
    let mut model = column(&mut rasteriser);
    let content = PlainTextDocument::from_paragraphs([
        "One very long paragraph, long enough that it certainly runs past the bottom of a short \
         page and has to carry on onto the next one, which is the case this test is about, and it \
         keeps going for a while yet so that there is no doubt at all about the pagination.",
    ]);
    let constraints = page();
    let pages = all_pages(&mut model, &content, &constraints);
    assert!(pages.len() >= 2, "the fixture must paginate");

    let left_edge_of_first_line = |page: &mjx_layout::PageFragments| {
        page.fragments()
            .nodes()
            .find(|(_, node)| matches!(node.fragment(), Fragment::Line(_)))
            .map(|(_, node)| node.rect().left)
            .expect("every page has a line")
    };

    let first = left_edge_of_first_line(&pages[0]);
    let second = left_edge_of_first_line(&pages[1]);
    assert!(
        first > second,
        "page 1 opens a paragraph and is indented; page 2 continues one and is not: {first:?} \
         against {second:?}"
    );
    assert_eq!(
        second, constraints.content.left,
        "a continuation line starts at the column's own edge"
    );
}
