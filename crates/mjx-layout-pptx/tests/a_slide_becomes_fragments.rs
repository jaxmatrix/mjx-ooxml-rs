//! The end of the thirteen children before this one: a real `.pptx` becomes a real
//! [`FragmentTree`](mjx_layout::FragmentTree).
//!
//! Every assertion here is about **text and position**, and that is deliberate. The
//! `GeometryProvider` is still the placeholder (MJXOFF-202 defers real preset paths), so a gate
//! about how a shape *looks* would pass whatever this crate did; a gate about where a line's
//! baseline is would not.

mod support;

use mjx_layout::{BoxModel, Constraints, ExtentPrecision, Fragment, PageIndex};
use mjx_layout_pptx::{constraints_for, SlideBoxModel, SlideDeck};
use mjx_ooxml_core::measure::Emu;
use mjx_pptx::{Presentation, ShapeBounds};

use support::{blank_deck, fixture, glyph_runs, lay_out, lines, model, shapes, text_box};

#[test]
fn a_committed_deck_lays_out_every_slide() {
    for name in [
        "sample.pptx",
        "layouts.pptx",
        "text_levels.pptx",
        "notes.pptx",
    ] {
        let mut presentation = Presentation::open(&fixture(name)).expect("open");
        let deck = SlideDeck::read(&mut presentation).expect("read");
        let constraints = constraints_for(&deck);
        let mut model = model();

        let mut resume = None;
        let mut pages = 0_usize;
        for index in 0..deck.slide_count() {
            let page = model
                .layout_page(
                    &deck,
                    PageIndex::new(u32::try_from(index).expect("a small deck")),
                    &constraints,
                    resume.as_ref(),
                )
                .unwrap_or_else(|error| panic!("{name} slide {index}: {error}"));
            assert_eq!(page.page().number() as usize, index);
            resume = page.continuation().cloned();
            pages += 1;
        }
        assert_eq!(pages, deck.slide_count(), "{name}");
        assert!(resume.is_none(), "{name}: the last slide ends the content");
    }
}

#[test]
fn a_deck_with_text_produces_glyph_runs_addressed_to_their_runs() {
    let mut presentation = Presentation::open(&fixture("text_levels.pptx")).expect("open");
    let mut model = model();
    let tree = lay_out(&mut model, &mut presentation, 0);

    let runs = glyph_runs(&tree);
    assert!(!runs.is_empty(), "text_levels.pptx's first slide has text");

    // Two kinds of glyph run, and the *path length* is what tells them apart. A run of the
    // paragraph's own text names a run — slide, shape, paragraph, run — while a **bullet** belongs
    // to the paragraph and not to any run in it, so it names one segment fewer. That is the answer
    // a hit test on a bullet should give ("the marker of paragraph 0"), so it is asserted rather
    // than smoothed over.
    let text_runs = runs.iter().filter(|(path, _, _)| path.len() >= 4).count();
    let markers = runs.iter().filter(|(path, _, _)| path.len() == 3).count();
    assert!(text_runs > 0, "the slide's text is addressed to its runs");
    assert!(
        markers > 0,
        "text_levels.pptx's paragraphs are bulleted, so some run is a marker"
    );
    for (path, range, glyphs) in &runs {
        assert!(
            path.len() >= 3,
            "every run names at least a paragraph: {path:?}"
        );
        assert!(*glyphs > 0, "a run with no glyphs is not a run");
        assert!(range.end >= range.start);
    }
}

#[test]
fn the_extent_is_exact_and_equals_the_slide_count() {
    let mut presentation = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let deck = SlideDeck::read(&mut presentation).expect("read");
    let model = model();
    let extent = model.estimate_extent(&deck, &constraints_for(&deck));
    assert_eq!(extent.pages as usize, deck.slide_count());
    assert_eq!(
        extent.precision,
        ExtentPrecision::Exact,
        "a deck knows how many slides it has without laying one out"
    );
    assert_eq!(extent.page_size, deck.page());
}

#[test]
fn laying_out_a_page_alone_gives_the_same_fragments_as_laying_out_the_deck_in_order() {
    // The equivalence `BoxModel::layout_page` promises. It is free for an absolute box model, which
    // is exactly why it has to be asserted: "free" is a property of this implementation and not of
    // the trait, and the first refactor that carries state between pages would break it silently.
    let mut presentation = Presentation::open(&fixture("charts.pptx")).expect("open");
    let deck = SlideDeck::read(&mut presentation).expect("read");
    assert!(
        deck.slide_count() >= 2,
        "the fixture has more than one slide"
    );
    let constraints = constraints_for(&deck);

    let mut sequential = model();
    let mut resume = None;
    let mut in_order = Vec::new();
    for index in 0..deck.slide_count() {
        let page = sequential
            .layout_page(
                &deck,
                PageIndex::new(u32::try_from(index).expect("small")),
                &constraints,
                resume.as_ref(),
            )
            .expect("lays out");
        resume = page.continuation().cloned();
        in_order.push(page.fragments().clone());
    }

    // The second slide, reached by handing the first slide's checkpoint to a *fresh* box model.
    let mut alone = model();
    let first = alone
        .layout_page(&deck, PageIndex::FIRST, &constraints, None)
        .expect("lays out");
    let second = alone
        .layout_page(&deck, PageIndex::new(1), &constraints, first.continuation())
        .expect("lays out");
    assert_eq!(second.fragments(), &in_order[1]);
}

#[test]
fn a_shape_becomes_a_shape_fragment_at_its_effective_bounds() {
    let (mut deck, slide) = blank_deck();
    let bounds = ShapeBounds::from_inches(1.0, 2.0, 3.0, 0.5);
    let shape = text_box(&mut deck, slide, "hello", bounds);
    let mut model = model();
    let tree = lay_out(&mut model, &mut deck, slide);

    let ids = shapes(&tree);
    assert_eq!(ids.len(), 1, "one shape on the slide");
    let node = tree.node(ids[0]).expect("the shape node");
    assert_eq!(node.rect().left, Emu::from_emu(bounds.offset_x_emu));
    assert_eq!(node.rect().top, Emu::from_emu(bounds.offset_y_emu));
    assert_eq!(node.rect().width(), Emu::from_emu(bounds.width_emu));
    assert_eq!(node.rect().height(), Emu::from_emu(bounds.height_emu));
    assert_eq!(
        node.source().path().segments(),
        &[
            u32::try_from(slide).expect("small"),
            u32::try_from(shape).expect("small")
        ]
    );
}

#[test]
fn a_line_sits_inside_its_shapes_content_box() {
    let (mut deck, slide) = blank_deck();
    let bounds = ShapeBounds::from_inches(1.0, 1.0, 4.0, 1.0);
    text_box(&mut deck, slide, "hello world", bounds);
    let mut model = model();
    let tree = lay_out(&mut model, &mut deck, slide);

    let line_ids = lines(&tree);
    assert_eq!(line_ids.len(), 1, "one line");
    let line = tree.node(line_ids[0]).expect("the line node");
    // The schema's default insets are 0.1" horizontally and 0.05" vertically.
    assert!(line.rect().left >= Emu::from_emu(bounds.offset_x_emu + 91_440));
    assert!(line.rect().top >= Emu::from_emu(bounds.offset_y_emu + 45_720));
    assert!(
        line.rect().right <= Emu::from_emu(bounds.offset_x_emu + bounds.width_emu),
        "the line does not leave the shape"
    );
    let Fragment::Line(fragment) = line.fragment() else {
        panic!("it is a line");
    };
    assert!(fragment.ascent > Emu::ZERO, "a line has an ascent");
    assert!(
        fragment.baseline > Emu::ZERO && fragment.baseline <= line.rect().height(),
        "the baseline is inside the line's own box"
    );
}

#[test]
fn a_page_past_the_last_slide_is_refused_rather_than_answered() {
    let mut presentation = Presentation::open(&fixture("sample.pptx")).expect("open");
    let deck = SlideDeck::read(&mut presentation).expect("read");
    let constraints = constraints_for(&deck);
    let mut model = model();
    let past = u32::try_from(deck.slide_count()).expect("small");
    let error = model
        .layout_page(&deck, PageIndex::new(past), &constraints, None)
        .expect_err("there is no such slide");
    assert!(error.to_string().contains("was asked for"), "{error}");
}

#[test]
fn a_checkpoint_from_another_box_model_is_refused() {
    use mjx_layout::{Checkpoint, ModelSignature, PartId, SourcePath, SourceRef};

    let mut presentation = Presentation::open(&fixture("charts.pptx")).expect("open");
    let deck = SlideDeck::read(&mut presentation).expect("read");
    let constraints = constraints_for(&deck);
    let mut model = model();

    let foreign = Checkpoint::new(
        ModelSignature::new(0xDEAD_BEEF),
        PageIndex::FIRST,
        SourceRef::node(PartId::PRIMARY, SourcePath::new(&[1])),
        vec![1, 0, 0, 0],
    )
    .expect("a small checkpoint");
    let error = model
        .layout_page(&deck, PageIndex::new(1), &constraints, Some(&foreign))
        .expect_err("a foreign checkpoint is refused");
    assert!(error.to_string().contains("box model"), "{error}");
}

#[test]
fn a_checkpoint_whose_state_is_the_wrong_shape_is_refused_rather_than_misread() {
    use mjx_layout::{Checkpoint, PartId, SourcePath, SourceRef};

    let mut presentation = Presentation::open(&fixture("charts.pptx")).expect("open");
    let deck = SlideDeck::read(&mut presentation).expect("read");
    let constraints = constraints_for(&deck);
    let mut model = model();

    let malformed = Checkpoint::new(
        SlideBoxModel::SIGNATURE,
        PageIndex::FIRST,
        SourceRef::node(PartId::PRIMARY, SourcePath::new(&[1])),
        vec![1, 0, 0],
    )
    .expect("a small checkpoint");
    let error = model
        .layout_page(&deck, PageIndex::new(1), &constraints, Some(&malformed))
        .expect_err("three bytes are not four");
    assert!(error.to_string().contains("exactly four"), "{error}");
}

#[test]
fn a_slide_laid_out_at_a_size_it_was_not_authored_at_still_places_its_shapes() {
    // `Constraints` is the caller's, not the document's: a preview pane may ask for a slide at a
    // different page size. The shapes keep their own absolute coordinates, which is what makes an
    // absolute box model absolute.
    let mut presentation = Presentation::open(&fixture("sample.pptx")).expect("open");
    let deck = SlideDeck::read(&mut presentation).expect("read");
    let mut model = model();

    let authored = constraints_for(&deck);
    let halved = Constraints {
        page: mjx_layout::LayoutSize::new(
            deck.page().width.divided_by(2),
            deck.page().height.divided_by(2),
        ),
        ..authored
    };
    let at_authored = model
        .layout_page(&deck, PageIndex::FIRST, &authored, None)
        .expect("lays out");
    let at_halved = model
        .layout_page(&deck, PageIndex::FIRST, &halved, None)
        .expect("lays out");

    let shapes_at_authored = shapes(at_authored.fragments()).len();
    assert_eq!(shapes(at_halved.fragments()).len(), shapes_at_authored);
    // Only the page-level box changes size; a shape is where the file says it is.
    let root_authored = at_authored
        .fragments()
        .node(at_authored.fragments().roots()[0])
        .expect("a root");
    let root_halved = at_halved
        .fragments()
        .node(at_halved.fragments().roots()[0])
        .expect("a root");
    assert_ne!(root_authored.rect(), root_halved.rect());
}
