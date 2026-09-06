//! Resuming from a checkpoint produces **the same fragments**, and a wrong checkpoint is refused.
//!
//! # Why the equivalence has to be proved rather than assumed
//!
//! The whole point of a checkpoint is that a reader who drags a scrollbar to page 40 sees the same
//! page 40 as a reader who scrolled there. If resuming produced *nearly* the same page, nothing
//! would look broken and the two readers would silently disagree — and the disagreement compounds,
//! because page 41 resumes from page 40's checkpoint.
//!
//! So the test is an equality, not a smoke test: lay out pages 1..=*N* in order, then lay out page
//! *N* alone from *N−1*'s checkpoint, and compare the fragment trees. `FragmentTree` is `PartialEq`
//! all the way down to the shaped glyphs, so this compares glyph ids, advances, offsets, rectangles,
//! transforms and every `SourceRef` — not a summary of them.
//!
//! # And proved to be able to fail
//!
//! An equality between two things produced by the same code path is worth nothing unless a wrong
//! checkpoint gives a different answer. `perturbing_a_checkpoint_changes_the_page_it_produces` flips
//! one byte of the continuation state and asserts the page changes; the two refusal tests show that
//! a checkpoint from another box model and a checkpoint for another page are typed errors rather
//! than plausible-looking wrong pages.

mod support;

use mjx_layout::{
    BoxModel, Checkpoint, Constraints, LayoutError, LayoutSize, ModelSignature, PageIndex,
    MAXIMUM_CHECKPOINT_BYTES,
};
use mjx_ooxml_core::measure::Emu;
use std::sync::Arc;

use mjx_text::{FontFace, FontSize, GlyphRasteriser};

use support::plain_text::{PlainTextColumn, PlainTextDocument};

fn constraints() -> Constraints {
    Constraints::single_column(
        LayoutSize::new(Emu::from_inches(5.0), Emu::from_inches(2.5)),
        Emu::from_inches(0.75),
    )
}

fn content() -> PlainTextDocument {
    PlainTextDocument::from_paragraphs([
        "Flow layout is sequentially dependent, which is the whole reason a checkpoint has to \
         exist at all, and this paragraph is long enough to make that concrete rather than \
         theoretical.",
        "A second paragraph, so that a page boundary can fall inside one and between two, and so \
         that both cases are covered by the same walk over the document.",
        "A third and final paragraph, which carries on for long enough that the document reliably \
         runs to several pages at the size this suite lays it out at.",
    ])
}

/// A column over `face`.
///
/// The **same** `Arc<FontFace>` every time, deliberately. `GlyphRasteriser::register` is idempotent
/// by `Arc` identity, so parsing the file twice would produce two `FaceId`s and every fragment
/// comparison below would fail on the face rather than on the layout. That is a real property of the
/// contract and not a test artefact: a box model is handed the faces the rasteriser knows, and two
/// parses of one file are two faces.
fn column(rasteriser: &mut GlyphRasteriser, face: &Arc<FontFace>) -> PlainTextColumn {
    PlainTextColumn::new(rasteriser, Arc::clone(face), FontSize::from_points(12.0))
}

/// Lay out pages in order and collect `(tree, continuation)` for each.
fn sequential(
    model: &mut PlainTextColumn,
    content: &PlainTextDocument,
    constraints: &Constraints,
) -> Vec<(mjx_layout::FragmentTree, Option<Checkpoint>)> {
    let mut pages = Vec::new();
    let mut resume: Option<Checkpoint> = None;
    let mut page = PageIndex::FIRST;
    loop {
        let laid_out = model
            .layout_page(content, page, constraints, resume.as_ref())
            .expect("the column lays out");
        let (tree, continuation) = laid_out.into_parts();
        resume = continuation.clone();
        let finished = continuation.is_none();
        pages.push((tree, continuation));
        if finished {
            break;
        }
        page = page.next();
        assert!(page.number() < 100, "the column made no progress");
    }
    pages
}

#[test]
fn resuming_a_page_from_its_predecessors_checkpoint_produces_identical_fragments() {
    let mut rasteriser = GlyphRasteriser::new();
    let face = support::liberation_sans();
    let constraints = constraints();
    let content = content();

    let mut walker = column(&mut rasteriser, &face);
    let pages = sequential(&mut walker, &content, &constraints);
    assert!(
        pages.len() >= 3,
        "the fixture must produce at least three pages, or 'resume the last one' is 'resume the \
         second one': {} page(s)",
        pages.len()
    );

    for (number, (expected, _)) in pages.iter().enumerate().skip(1) {
        let page = PageIndex::new(u32::try_from(number).expect("a small document"));
        let previous = pages
            .get(number - 1)
            .and_then(|(_, continuation)| continuation.as_ref())
            .expect("a page that is not the last has a continuation");
        assert_eq!(
            previous.resumes_page(),
            page,
            "a checkpoint says which page it resumes"
        );

        // A *fresh* box model, so nothing carries over except the checkpoint itself. A model that
        // remembered where it was between calls would pass this test without the checkpoint doing
        // anything, which is exactly the vacuous version of it.
        let mut jumper = column(&mut rasteriser, &face);
        let (jumped, _) = jumper
            .layout_page(&content, page, &constraints, Some(previous))
            .expect("the column resumes")
            .into_parts();

        assert_eq!(
            &jumped,
            expected,
            "page {page} laid out from page {}'s checkpoint differs from page {page} laid out in \
             order",
            page.previous().expect("not the first page")
        );
    }
}

#[test]
fn perturbing_a_checkpoint_changes_the_page_it_produces() {
    // The proof that the equality above can fail. If a wrong continuation produced the same page,
    // the continuation would not be doing anything and the test above would be vacuous.
    let mut rasteriser = GlyphRasteriser::new();
    let face = support::liberation_sans();
    let constraints = constraints();
    let content = content();
    let mut walker = column(&mut rasteriser, &face);
    let pages = sequential(&mut walker, &content, &constraints);

    let honest = pages[0]
        .1
        .as_ref()
        .expect("page 1 has a continuation")
        .clone();
    assert_eq!(honest.byte_size(), 8, "this model writes eight bytes");
    assert!(honest.byte_size() <= MAXIMUM_CHECKPOINT_BYTES);

    // Move the byte offset on by a few characters — the smallest lie a continuation can tell.
    let mut state = honest.state().to_vec();
    state[4] = state[4].wrapping_add(7);
    let perturbed = Checkpoint::new(
        PlainTextColumn::SIGNATURE,
        honest.ends_page(),
        honest.position().clone(),
        state,
    )
    .expect("eight bytes is under the ceiling");

    let mut honest_model = column(&mut rasteriser, &face);
    let (honest_page, _) = honest_model
        .layout_page(&content, PageIndex::new(1), &constraints, Some(&honest))
        .expect("the honest checkpoint resumes")
        .into_parts();

    let mut lying_model = column(&mut rasteriser, &face);
    let (lying_page, _) = lying_model
        .layout_page(&content, PageIndex::new(1), &constraints, Some(&perturbed))
        .expect("the perturbed checkpoint still resumes, it just resumes elsewhere")
        .into_parts();

    assert_ne!(
        honest_page, lying_page,
        "a checkpoint whose offset moved by seven bytes must produce a different page; if it did \
         not, the equivalence test above would be proving nothing"
    );
}

#[test]
fn a_checkpoint_from_another_box_model_is_refused_rather_than_misread() {
    let mut rasteriser = GlyphRasteriser::new();
    let face = support::liberation_sans();
    let constraints = constraints();
    let content = content();
    let mut model = column(&mut rasteriser, &face);
    let pages = sequential(&mut model, &content, &constraints);
    let honest = pages[0].1.as_ref().expect("a continuation").clone();

    let foreign = Checkpoint::new(
        ModelSignature::new(0xDEAD_BEEF),
        honest.ends_page(),
        honest.position().clone(),
        honest.state().to_vec(),
    )
    .expect("under the ceiling");

    let error = foreign
        .state_for(PlainTextColumn::SIGNATURE, PageIndex::new(1))
        .expect_err("another model's checkpoint must be refused");
    assert!(matches!(error, LayoutError::ForeignCheckpoint { .. }));
    // The message names both, because a caller has to be able to tell which two.
    let text = error.to_string();
    assert!(text.contains("0x00000000deadbeef"), "{text}");
    assert!(
        text.contains(&PlainTextColumn::SIGNATURE.to_string()),
        "{text}"
    );

    // And the whole way through the trait, not only on the helper.
    let mut fresh = column(&mut rasteriser, &face);
    let refused = fresh.layout_page(&content, PageIndex::new(1), &constraints, Some(&foreign));
    assert!(refused.is_err());
}

#[test]
fn a_checkpoint_for_another_page_is_refused_rather_than_misread() {
    // The half that is usually forgotten. A checkpoint from the right model and the wrong page
    // produces a page that looks entirely plausible and holds the wrong content, which is worse than
    // a refusal because nothing about it looks wrong.
    let mut rasteriser = GlyphRasteriser::new();
    let face = support::liberation_sans();
    let constraints = constraints();
    let content = content();
    let mut model = column(&mut rasteriser, &face);
    let pages = sequential(&mut model, &content, &constraints);
    assert!(pages.len() >= 3);
    let first = pages[0].1.as_ref().expect("a continuation").clone();

    let error = first
        .state_for(PlainTextColumn::SIGNATURE, PageIndex::new(2))
        .expect_err("page 3 does not resume from page 1's checkpoint");
    match error {
        LayoutError::MisplacedCheckpoint {
            requested,
            ends_page,
            expected,
        } => {
            assert_eq!(requested, PageIndex::new(2));
            assert_eq!(ends_page, PageIndex::FIRST);
            assert_eq!(expected, PageIndex::new(1));
        }
        other => panic!("expected a misplaced checkpoint, got {other}"),
    }

    // The right page is accepted, so the refusal is discriminating rather than blanket.
    assert!(first
        .state_for(PlainTextColumn::SIGNATURE, PageIndex::new(1))
        .is_ok());
}

#[test]
fn a_continuation_larger_than_the_ceiling_is_refused() {
    // Checkpoints are kept for every page of a document that may be three hundred pages long, which
    // is only affordable while each is small. A box model that needs more is keeping page content in
    // its continuation.
    let position =
        mjx_layout::SourceRef::node(mjx_layout::PartId::PRIMARY, mjx_layout::SourcePath::root());
    let ceiling = Checkpoint::new(
        PlainTextColumn::SIGNATURE,
        PageIndex::FIRST,
        position.clone(),
        vec![0_u8; MAXIMUM_CHECKPOINT_BYTES],
    );
    assert!(ceiling.is_ok(), "exactly the ceiling is allowed");

    let error = Checkpoint::new(
        PlainTextColumn::SIGNATURE,
        PageIndex::FIRST,
        position,
        vec![0_u8; MAXIMUM_CHECKPOINT_BYTES + 1],
    )
    .expect_err("one byte past it is not");
    assert!(matches!(
        error,
        LayoutError::CheckpointTooLarge {
            bytes,
            ceiling: MAXIMUM_CHECKPOINT_BYTES,
        } if bytes == MAXIMUM_CHECKPOINT_BYTES + 1
    ));
}
