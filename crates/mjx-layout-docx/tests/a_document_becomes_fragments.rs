//! A document becomes a [`FragmentTree`](mjx_layout::FragmentTree): the shape of the tree, the
//! address every fragment carries, and the hit test that address makes possible.
//!
//! # The shape, and why it is three levels
//!
//! ```text
//! page          the column, one box
//!  └─ paragraph one box per paragraph on the page, carrying its decoration handle
//!      └─ line  one per line, carrying its baseline and its direction
//!          └─ glyph run  one per shaped stretch, at its own origin
//! ```
//!
//! A line is a fragment rather than a property of its runs because everything that acts on a line
//! acts on all of it at once — clicking past its end, extending a selection by a line, an underline
//! that spans it, vertical caret movement, a screen reader's line granularity. `mjx-layout`'s own
//! documentation says so; this is what makes it true for a document.

mod support;

use mjx_layout::{Fragment, LayoutPoint};
use support::{constraints, one_page, paragraph};

#[test]
fn the_tree_is_a_page_of_paragraphs_of_lines_of_runs() {
    let tree = one_page(
        &[paragraph("", "First paragraph."), paragraph("", "Second.")],
        &constraints(6.5, 11.0),
    );
    assert_eq!(tree.roots().len(), 1, "one page fragment");

    let page = tree.roots()[0];
    assert!(matches!(
        tree.node(page).map(mjx_layout::FragmentNode::fragment),
        Some(Fragment::Box(_))
    ));
    let paragraphs: Vec<_> = tree.children(page).collect();
    assert_eq!(paragraphs.len(), 2, "one box per paragraph");

    for (index, block) in paragraphs.iter().enumerate() {
        let node = tree.node(*block).expect("a paragraph box");
        assert_eq!(
            node.source().path().segments(),
            &[u32::try_from(index).expect("an index")],
            "a paragraph's address is its index"
        );
        let lines: Vec<_> = tree.children(*block).collect();
        assert_eq!(lines.len(), 1, "each fixture paragraph is one line");
        for line in lines {
            let line_node = tree.node(line).expect("a line");
            assert!(matches!(line_node.fragment(), Fragment::Line(_)));
            assert_eq!(line_node.source().path().depth(), 2);
            let runs: Vec<_> = tree.children(line).collect();
            assert!(!runs.is_empty(), "a line of text has glyph runs on it");
            for run in runs {
                let run_node = tree.node(run).expect("a run");
                assert!(matches!(run_node.fragment(), Fragment::GlyphRun(_)));
                assert_eq!(run_node.source().path().depth(), 3);
            }
        }
    }
}

/// A line's baseline is where the glyphs sit, and it is **relative to the line's own rectangle** —
/// which is what lets justification, vertical alignment and a page break each move a line with one
/// edit and nothing else.
#[test]
fn a_line_carries_its_baseline_relative_to_its_own_rectangle() {
    let tree = one_page(&[paragraph("", "One line.")], &constraints(6.5, 11.0));
    let (id, node) = tree
        .nodes()
        .find(|(_, node)| matches!(node.fragment(), Fragment::Line(_)))
        .expect("a line");
    let Fragment::Line(line) = node.fragment() else {
        unreachable!("filtered above")
    };
    assert!(line.baseline > mjx_ooxml_core::measure::Emu::ZERO);
    assert!(
        line.baseline <= node.rect().height(),
        "the baseline is inside the line box"
    );
    assert!(line.ascent + line.descent <= node.rect().height());

    // The glyph runs on it sit **on** that baseline, in page coordinates.
    let baseline_y = node.rect().top + line.baseline;
    for run in tree.children(id) {
        let run_node = tree.node(run).expect("a run");
        let Fragment::GlyphRun(glyphs) = run_node.fragment() else {
            continue;
        };
        assert_eq!(glyphs.origin.y, baseline_y);
    }
}

/// The address a fragment carries is what makes a hit test answer *where in the document*, which is
/// the whole reason a `SourceRef` is not a rectangle.
#[test]
fn a_point_finds_the_paragraph_it_is_in() {
    let constraints = constraints(6.5, 11.0);
    let mut document = support::document(&[
        paragraph("", "First paragraph."),
        paragraph("", "Second paragraph."),
    ]);
    let flow = support::flow(&mut document);
    let mut model = support::model();
    let page = mjx_layout::BoxModel::layout_page(
        &mut model,
        &flow,
        mjx_layout::PageIndex::FIRST,
        &constraints,
        None,
    )
    .expect("a page");

    // A point on the second paragraph's line.
    let second = page
        .fragments()
        .nodes()
        .find(|(_, node)| {
            matches!(node.fragment(), Fragment::Line(_))
                && node.source().path().segments().first() == Some(&1)
        })
        .map(|(_, node)| node.rect())
        .expect("the second paragraph's line");
    let point = LayoutPoint::new(
        second.left + second.width().divided_by(10),
        second.top + second.height().divided_by(2),
    );

    let hits = page.index().fragments_at(point);
    assert!(!hits.is_empty(), "a point on a line must hit something");
    let addressed = hits
        .iter()
        .filter_map(|id| page.fragments().node(*id))
        .any(|node| node.source().path().segments().first() == Some(&1));
    assert!(
        addressed,
        "and what it hits must say which paragraph it is in"
    );
}

/// Every fragment's rectangle is inside the page's, which is what says nothing was placed off the
/// paper.
#[test]
fn nothing_is_placed_outside_the_page() {
    let constraints = constraints(6.5, 11.0);
    let tree = one_page(
        &[
            paragraph("", "First."),
            paragraph(r#"<w:jc w:val="right"/>"#, "Second."),
        ],
        &constraints,
    );
    let page = tree
        .node(tree.roots()[0])
        .expect("the page fragment")
        .rect();
    for (_, node) in tree.nodes() {
        let rect = node.rect();
        assert!(
            rect.left >= page.left && rect.top >= page.top,
            "{:?} starts outside the page {page:?}",
            rect
        );
        assert!(
            rect.right <= page.right + mjx_ooxml_core::measure::Emu::from_emu(2),
            "{:?} runs past the page's right edge {page:?}",
            rect
        );
    }
}
