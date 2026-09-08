//! The `SourceRef` round trip: a point inside a laid-out run hit-tests back to the exact shape and
//! the exact character.
//!
//! This is what R11's selection and loop 2's editing depend on, and it is the one assertion that
//! makes `SourceRef` more than a decorative field. A fragment tree whose addresses were plausible
//! but wrong would pass every geometry test in this crate and fail here.
//!
//! # The whole trip, not half of it
//!
//! Laying out and then reading a fragment's own address back proves nothing — it is the same value
//! twice. So each test below goes **out of the crate and back**: the address is handed to
//! `mjx-pptx`, which answers with the run's text, and that text is compared against what was
//! authored. A path off by one names a different run and the text differs.

mod support;

use mjx_layout::{Fragment, LayoutPoint, PageFragments};
use mjx_layout_pptx::{constraints_for, SlideBoxModel, SlideDeck, TextHit};
use mjx_ooxml_core::measure::Emu;
use mjx_pptx::{Presentation, ShapeBounds};

use support::{blank_deck, fixture, model, resolver, text_box};

/// Lays out slide `index` and keeps the page, so the spatial index is available.
fn page(model: &mut SlideBoxModel, deck: &mut Presentation, index: usize) -> PageFragments {
    use mjx_layout::{BoxModel, PageIndex};
    let read = SlideDeck::read(deck).expect("read");
    model
        .layout_page(
            &read,
            PageIndex::new(u32::try_from(index).expect("small")),
            &constraints_for(&read),
            None,
        )
        .expect("lays out")
}

/// The topmost glyph run at `point`, as a hit.
fn hit_at(page: &PageFragments, model: &SlideBoxModel, point: LayoutPoint) -> Option<TextHit> {
    let candidates = page.index().fragments_at(point);
    // Last is topmost, and a glyph run is always above the line and the boxes that hold it.
    for id in candidates.into_iter().rev() {
        let node = page.fragments().node(id)?;
        if !matches!(node.fragment(), Fragment::GlyphRun(_)) {
            continue;
        }
        let depth = model.catalogue().shape_depth(node.source().path())?;
        return TextHit::from_source(node.source(), depth);
    }
    None
}

#[test]
fn a_point_inside_a_run_names_the_shape_the_paragraph_and_the_run() {
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        "Hit me",
        ShapeBounds::from_inches(1.0, 1.0, 4.0, 1.0),
    );
    let mut model = model();
    let laid = page(&mut model, &mut deck, slide);

    // A point a little inside the first glyph run's own rectangle.
    let (_, run) = laid
        .fragments()
        .nodes()
        .find(|(_, node)| {
            matches!(node.fragment(), Fragment::GlyphRun(_)) && node.source().path().depth() >= 4
        })
        .expect("a glyph run of the text");
    let rect = run.rect();
    let point = LayoutPoint::new(
        rect.left + rect.width().divided_by(4),
        rect.top + rect.height().divided_by(2),
    );

    let hit = hit_at(&laid, &model, point).expect("the point is on a run");
    assert_eq!(hit.surface_index as usize, slide);
    assert_eq!(hit.shape_indices(), vec![shape]);
    assert_eq!(hit.paragraph, Some(0));
    assert_eq!(hit.run, Some(0));

    // Out of the crate and back: `mjx-pptx` is asked what run that address names.
    let text = deck
        .run_text(slide, hit.shape_indices(), 0, 0)
        .expect("the run the hit names");
    assert_eq!(text, "Hit me");
}

#[test]
fn a_point_on_the_second_of_two_runs_in_one_paragraph_names_the_second() {
    // The identity-value trap applied to the run index: a scheme that always answered run `0` is
    // *right* for every paragraph with one run, which is nearly all of them. So this paragraph is
    // split into two runs — formatting half of it is what splits it, exactly as it does in
    // PowerPoint — and both halves are hit.
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        "AAAAA BBBBB",
        ShapeBounds::from_inches(1.0, 1.0, 6.0, 1.0),
    );
    deck.set_text_range_properties(
        slide,
        shape,
        0,
        6..11,
        &mjx_dml::CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("formatting half the paragraph splits it into two runs");
    assert_eq!(
        deck.run_count(slide, vec![shape], 0).expect("run count"),
        2,
        "the paragraph now holds two runs"
    );

    let mut model = model();
    let laid = page(&mut model, &mut deck, slide);
    let runs: Vec<_> = laid
        .fragments()
        .nodes()
        .filter(|(_, node)| {
            matches!(node.fragment(), Fragment::GlyphRun(_)) && node.source().path().depth() >= 4
        })
        .map(|(_, node)| node.rect())
        .collect();
    assert!(runs.len() >= 2, "two runs, two glyph runs: {runs:?}");

    let mut seen = std::collections::BTreeSet::new();
    for rect in &runs {
        let point = LayoutPoint::new(
            rect.left + rect.width().divided_by(2),
            rect.top + rect.height().divided_by(2),
        );
        let hit = hit_at(&laid, &model, point).expect("the point is on a run");
        let index = hit.run.expect("a run index");
        seen.insert(index);
        let text = deck
            .run_text(slide, hit.shape_indices(), 0, index)
            .expect("the run the hit names");
        assert!(
            !text.is_empty(),
            "run {index} of the paragraph has text: {text:?}"
        );
    }
    assert!(
        seen.contains(&1),
        "a point on the second run must name run 1, not run 0: {seen:?}"
    );
}

#[test]
fn a_hit_on_a_second_paragraph_names_the_second_paragraph() {
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        "Alpha\nBravo",
        ShapeBounds::from_inches(1.0, 1.0, 6.0, 2.0),
    );
    let mut model = model();
    let laid = page(&mut model, &mut deck, slide);

    let second = laid
        .fragments()
        .nodes()
        .filter(|(_, node)| {
            matches!(node.fragment(), Fragment::GlyphRun(_)) && node.source().path().depth() >= 4
        })
        .map(|(_, node)| node.rect())
        .last()
        .expect("a second run");
    let point = LayoutPoint::new(
        second.left + Emu::from_emu(1),
        second.top + second.height().divided_by(2),
    );
    let hit = hit_at(&laid, &model, point).expect("a hit");
    assert_eq!(hit.paragraph, Some(1));
    assert_eq!(
        deck.run_text(slide, hit.shape_indices(), 1, hit.run.expect("a run"))
            .expect("the run"),
        "Bravo"
    );
    let _ = shape;
}

#[test]
fn a_hit_reports_a_character_offset_inside_the_runs_own_text() {
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        "Wrapping across several lines makes the offsets interesting",
        ShapeBounds::from_inches(1.0, 1.0, 2.0, 3.0),
    );
    let mut model = model();
    let laid = page(&mut model, &mut deck, slide);

    let runs: Vec<_> = laid
        .fragments()
        .nodes()
        .filter(|(_, node)| {
            matches!(node.fragment(), Fragment::GlyphRun(_)) && node.source().path().depth() >= 4
        })
        .map(|(_, node)| (node.rect(), node.source().clone()))
        .collect();
    assert!(runs.len() > 1, "the fixture wraps");

    let whole = deck
        .run_text(slide, vec![shape], 0, 0)
        .expect("the run's own text");
    for (rect, source) in &runs {
        let point = LayoutPoint::new(
            rect.left + Emu::from_emu(1),
            rect.top + rect.height().divided_by(2),
        );
        let hit = hit_at(&laid, &model, point).expect("a hit");
        let offset = hit.offset.expect("an offset");
        assert!(
            offset <= whole.len(),
            "the offset is inside the run's own text: {offset} of {}",
            whole.len()
        );
        assert!(
            whole.is_char_boundary(offset),
            "an offset that is not a character boundary cannot hold a caret"
        );
        assert_eq!(offset, source.characters().start as usize);
    }
}

#[test]
fn a_point_on_no_shape_finds_nothing() {
    let (mut deck, slide) = blank_deck();
    text_box(
        &mut deck,
        slide,
        "Somewhere else",
        ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0),
    );
    let mut model = model();
    let laid = page(&mut model, &mut deck, slide);
    let far = LayoutPoint::new(Emu::from_inches(9.0), Emu::from_inches(6.0));
    assert!(hit_at(&laid, &model, far).is_none());
}

#[test]
fn a_hit_on_a_real_deck_round_trips_through_mjx_pptx() {
    let mut deck = Presentation::open(&fixture("text_levels.pptx")).expect("open");
    let mut model = SlideBoxModel::new(resolver());
    let laid = page(&mut model, &mut deck, 0);

    let mut checked = 0_usize;
    let runs: Vec<_> = laid
        .fragments()
        .nodes()
        .filter(|(_, node)| {
            matches!(node.fragment(), Fragment::GlyphRun(_)) && node.source().path().depth() >= 4
        })
        .map(|(_, node)| node.rect())
        .collect();
    for rect in runs {
        let point = LayoutPoint::new(
            rect.left + Emu::from_emu(1),
            rect.top + rect.height().divided_by(2),
        );
        let Some(hit) = hit_at(&laid, &model, point) else {
            continue;
        };
        let (Some(paragraph), Some(run)) = (hit.paragraph, hit.run) else {
            continue;
        };
        let text = deck
            .run_text(0, hit.shape_indices(), paragraph, run)
            .unwrap_or_else(|error| {
                panic!(
                    "the address {:?} names no run: {error}",
                    hit.shape_indices()
                )
            });
        assert!(
            !text.is_empty(),
            "a laid-out run addresses a run that has text"
        );
        checked += 1;
    }
    assert!(checked > 0, "the fixture has runs to hit");
}
