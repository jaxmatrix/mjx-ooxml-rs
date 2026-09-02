//! Every PresentationML invariant `Presentation::save` enforces, proved by a deck that breaks
//! exactly one of them.
//!
//! Each case starts from a deck this library authored end to end — so every other invariant holds by
//! construction — rewrites **one** part so that a single invariant is broken, and then makes an
//! ordinary edit to that part. The edit matters: the checks are scoped to markup this library will
//! write, so a deck that is merely opened and saved is never faulted for what it arrived with. Both
//! halves of that rule are pinned below.
//!
//! Breaking one thing at a time is the point. A deck broken twice would fail for the wrong reason and
//! prove nothing about the check under test.

use mjx_ooxml_types::presentationml::SlideSizeKind;
use mjx_opc::{Package, PartName};
use mjx_pptx::{
    default_placeholder_ole, OleObjectSpec, PptxError, Presentation, PresentationDefect,
    ShapeBounds, SlideSize,
};

const PRESENTATION_PART: &str = "/ppt/presentation.xml";
const FIRST_SLIDE_PART: &str = "/ppt/slides/slide1.xml";

/// A one-pixel PNG, for the OLE snapshot.
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

/// A deck authored from nothing: one master, one layout, two slides. Valid in every respect — the
/// baseline each mutation below departs from by exactly one step.
fn authored_deck() -> Vec<u8> {
    let mut deck = Presentation::blank(SlideSize {
        width_emu: 12_192_000,
        height_emu: 6_858_000,
        kind: SlideSizeKind::Screen16X9,
    })
    .expect("blank");
    deck.add_slide_from_layout(0).expect("slide 1");
    deck.add_slide_from_layout(0).expect("slide 2");
    deck.save().expect("a deck this library authored is valid")
}

/// Rewrites one part of a deck, returning the container bytes.
///
/// Written with `save_unchecked`, because a broken deck is exactly what `save` refuses to produce —
/// which is the whole point of the tests below.
fn with_mutated_part(bytes: &[u8], part: &str, mutate: impl FnOnce(String) -> String) -> Vec<u8> {
    let mut package = Package::open(bytes).expect("open");
    let name = PartName::new(part).expect("valid part name");
    let text = String::from_utf8(
        package
            .part_bytes(&name)
            .expect("the part is present")
            .to_vec(),
    )
    .expect("utf-8 markup");
    let mutated = mutate(text);
    package
        .replace_part_bytes(&name, mutated.into_bytes())
        .expect("replace");
    package
        .save_unchecked()
        .expect("the escape hatch writes the broken deck")
}

/// Replaces exactly one occurrence of `from`, asserting it was there — a mutation that silently did
/// nothing would leave a valid deck and a test that proves nothing.
fn replace_once(text: &str, from: &str, to: &str) -> String {
    assert_eq!(
        text.matches(from).count(),
        1,
        "the mutation target {from:?} must appear exactly once"
    );
    text.replacen(from, to, 1)
}

/// The defect a save was refused for, or a panic naming what happened instead.
fn defect(result: Result<Vec<u8>, PptxError>) -> PresentationDefect {
    match result {
        Ok(_) => panic!("the deck saved, but it violates an invariant"),
        Err(PptxError::InvalidPresentation(defect)) => *defect,
        Err(other) => panic!("expected a PresentationML invariant failure, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------
// One invariant per test
// ---------------------------------------------------------------------------------------------

/// Two shapes in one slide sharing a `p:cNvPr@id`.
#[test]
fn a_duplicate_shape_id_is_refused() {
    let broken = with_mutated_part(&authored_deck(), FIRST_SLIDE_PART, |text| {
        replace_once(
            &text,
            r#"<p:cNvPr id="3" name="Text Placeholder 2"/>"#,
            r#"<p:cNvPr id="2" name="Text Placeholder 2"/>"#,
        )
    });

    let mut deck = Presentation::open(&broken).expect("open");
    // An ordinary edit to the slide: this library now owns the bytes it will write for it.
    deck.set_shape_text_content(0, 0, "Title").expect("edit");

    match defect(deck.save()) {
        PresentationDefect::DuplicateShapeId { part, shape_id } => {
            assert_eq!(part, FIRST_SLIDE_PART);
            assert_eq!(shape_id, "2");
        }
        other => panic!("wrong defect: {other:?}"),
    }
}

/// Two `p:sldId` entries sharing an `@id`.
#[test]
fn a_duplicate_slide_entry_id_is_refused() {
    let broken = with_mutated_part(&authored_deck(), PRESENTATION_PART, |text| {
        replace_once(&text, r#"<p:sldId id="257""#, r#"<p:sldId id="256""#)
    });

    let mut deck = Presentation::open(&broken).expect("open");
    // An ordinary edit to `presentation.xml`; the new entry takes a free id of its own.
    deck.add_slide_from_layout(0).expect("add a slide");

    match defect(deck.save()) {
        PresentationDefect::DuplicateListEntryId {
            part,
            list,
            entry_id,
        } => {
            assert_eq!(part, PRESENTATION_PART);
            assert_eq!(list, "p:sldIdLst");
            assert_eq!(entry_id, "256");
        }
        other => panic!("wrong defect: {other:?}"),
    }
}

/// Two `p:sldId` entries naming the same relationship — one slide listed twice.
#[test]
fn a_slide_listed_twice_is_refused() {
    let broken = with_mutated_part(&authored_deck(), PRESENTATION_PART, |text| {
        replace_once(
            &text,
            r#"<p:sldId id="257" r:id="rId4"/>"#,
            r#"<p:sldId id="257" r:id="rId4"/><p:sldId id="258" r:id="rId3"/>"#,
        )
    });

    let mut deck = Presentation::open(&broken).expect("open");
    deck.add_slide_from_layout(0).expect("add a slide");

    match defect(deck.save()) {
        PresentationDefect::DuplicateListEntryReference {
            part,
            list,
            relationship_id,
        } => {
            assert_eq!(part, PRESENTATION_PART);
            assert_eq!(list, "p:sldIdLst");
            assert_eq!(relationship_id, "rId3");
        }
        other => panic!("wrong defect: {other:?}"),
    }
}

/// A `p:sldId` naming a relationship that leads somewhere that is not a slide — here the theme.
#[test]
fn a_slide_entry_pointing_at_something_that_is_not_a_slide_is_refused() {
    let broken = with_mutated_part(&authored_deck(), PRESENTATION_PART, |text| {
        replace_once(
            &text,
            r#"<p:sldId id="257" r:id="rId4"/>"#,
            r#"<p:sldId id="257" r:id="rId4"/><p:sldId id="258" r:id="rId2"/>"#,
        )
    });

    let mut deck = Presentation::open(&broken).expect("open");
    deck.add_slide_from_layout(0).expect("add a slide");

    match defect(deck.save()) {
        PresentationDefect::ListEntryTargetHasWrongContentType {
            part,
            list,
            relationship_id,
            target_part,
            expected_content_type,
            actual_content_type,
        } => {
            assert_eq!(part, PRESENTATION_PART);
            assert_eq!(list, "p:sldIdLst");
            assert_eq!(relationship_id, "rId2");
            assert_eq!(target_part, "/ppt/theme/theme1.xml");
            assert!(expected_content_type.ends_with("presentationml.slide+xml"));
            assert!(actual_content_type.ends_with("theme+xml"));
        }
        other => panic!("wrong defect: {other:?}"),
    }
}

/// A slide the deck relates to that `p:sldIdLst` never lists — the reverse direction, which the
/// packaging layer cannot see because every relationship and every part is perfectly in order.
#[test]
fn a_slide_relationship_no_entry_lists_is_refused() {
    let broken = with_mutated_part(&authored_deck(), PRESENTATION_PART, |text| {
        replace_once(&text, r#"<p:sldId id="257" r:id="rId4"/>"#, "")
    });

    let mut deck = Presentation::open(&broken).expect("open");
    deck.add_slide_from_layout(0).expect("add a slide");

    match defect(deck.save()) {
        PresentationDefect::UnlistedRelationship {
            part,
            list,
            relationship_id,
            target_part,
        } => {
            assert_eq!(part, PRESENTATION_PART);
            assert_eq!(list, "p:sldIdLst");
            assert_eq!(relationship_id, "rId4");
            assert_eq!(target_part, "/ppt/slides/slide2.xml");
        }
        other => panic!("wrong defect: {other:?}"),
    }
}

/// A `p:sldId` whose `r:id` nothing declares is a *packaging* defect, not a PresentationML one — the
/// rule lives one layer down, and this pins that it is not restated (and not lost) here.
///
/// `Presentation::open` refuses such a deck outright (`SlideRelNotFound`), so the check is exercised
/// where a deck in that state can still reach a save: at the packaging layer, with the presentation
/// part edited so its bytes are this library's to write.
#[test]
fn a_slide_entry_naming_an_undeclared_relationship_is_refused_by_the_packaging_layer() {
    let broken = with_mutated_part(&authored_deck(), PRESENTATION_PART, |text| {
        replace_once(
            &text,
            r#"r:id="rId4"/></p:sldIdLst>"#,
            r#"r:id="rId9"/></p:sldIdLst>"#,
        )
    });
    assert!(
        matches!(
            Presentation::open(&broken),
            Err(PptxError::SlideRelNotFound { .. })
        ),
        "the deck reader rejects it on open"
    );

    let mut package = Package::open(&broken).expect("open as a package");
    package
        .part_tree_mut(&PartName::new(PRESENTATION_PART).expect("part"))
        .expect("edit the presentation part");

    match package.save() {
        Err(mjx_opc::OpcError::Invalid(
            mjx_opc::PackageDefect::UndeclaredRelationshipReference {
                part,
                element,
                attribute,
                relationship_id,
            },
        )) => {
            assert_eq!(part, PRESENTATION_PART);
            assert_eq!(element, "p:sldId");
            assert_eq!(attribute, "r:id");
            assert_eq!(relationship_id, "rId9");
        }
        other => panic!("expected a dangling relationship reference, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------
// The other half: what validation must *not* do
// ---------------------------------------------------------------------------------------------

/// A deck that arrived with a duplicate shape id is written back untouched rather than refused.
/// Refusing would mean a file this library can open and not save, which is the opposite of the
/// promise. The moment an edit makes those bytes ours, the same deck is refused — that is
/// `a_duplicate_shape_id_is_refused` above, and the two together are the whole scope rule.
#[test]
fn a_deck_is_not_faulted_for_markup_it_arrived_with() {
    let broken = with_mutated_part(&authored_deck(), FIRST_SLIDE_PART, |text| {
        replace_once(
            &text,
            r#"<p:cNvPr id="3" name="Text Placeholder 2"/>"#,
            r#"<p:cNvPr id="2" name="Text Placeholder 2"/>"#,
        )
    });

    let deck = Presentation::open(&broken).expect("open");
    deck.validate().expect("untouched markup is not ours");
    deck.save().expect("and it saves");
}

/// Reading a slide is not editing it: the verdict must not depend on what the caller looked at.
#[test]
fn reading_a_slide_does_not_change_the_verdict() {
    let broken = with_mutated_part(&authored_deck(), FIRST_SLIDE_PART, |text| {
        replace_once(
            &text,
            r#"<p:cNvPr id="3" name="Text Placeholder 2"/>"#,
            r#"<p:cNvPr id="2" name="Text Placeholder 2"/>"#,
        )
    });

    let mut deck = Presentation::open(&broken).expect("open");
    let _ = deck.shape_count(0).expect("read the slide");
    deck.save().expect("reading is not editing");
}

/// `save_unchecked` is the escape hatch, and it writes what `save` refuses.
#[test]
fn save_unchecked_writes_what_save_refuses() {
    let broken = with_mutated_part(&authored_deck(), FIRST_SLIDE_PART, |text| {
        replace_once(
            &text,
            r#"<p:cNvPr id="3" name="Text Placeholder 2"/>"#,
            r#"<p:cNvPr id="2" name="Text Placeholder 2"/>"#,
        )
    });

    let mut deck = Presentation::open(&broken).expect("open");
    deck.set_shape_text_content(0, 0, "Title").expect("edit");
    assert!(matches!(
        defect(deck.save()),
        PresentationDefect::DuplicateShapeId { .. }
    ));
    let bytes = deck.save_unchecked().expect("the escape hatch writes it");
    Presentation::open(&bytes).expect("and the container is a deck");
}

/// Two OLE objects on one slide must not both write a snapshot picture with the same non-visual id.
///
/// This is the defect the validator found the day it was written: the frame took an allocated id and
/// its snapshot picture took a hard-coded `0`, so a second OLE object duplicated it. The test is the
/// regression pin — it asserts the ids, not merely that the save succeeded, so a future writer that
/// re-introduces a constant fails here rather than only inside the validator.
#[test]
fn two_ole_objects_on_one_slide_get_distinct_shape_ids() {
    let mut deck = Presentation::open(&authored_deck()).expect("open");
    let payload = default_placeholder_ole();
    for _ in 0..2 {
        deck.add_ole_object(
            0,
            &OleObjectSpec::embedded_stream("Excel.Sheet.12", &payload, TINY_PNG),
            ShapeBounds::from_inches(1.0, 1.0, 3.0, 2.0),
        )
        .expect("add OLE object");
    }
    let saved = deck.save().expect("distinct ids, so the deck saves");

    let package = Package::open(&saved).expect("reopen");
    let markup = String::from_utf8(
        package
            .part_bytes(&PartName::new(FIRST_SLIDE_PART).expect("part"))
            .expect("slide bytes")
            .to_vec(),
    )
    .expect("utf-8");
    let mut ids: Vec<&str> = markup
        .match_indices(r#"<p:cNvPr id=""#)
        .map(|(at, marker)| {
            let rest = &markup[at + marker.len()..];
            &rest[..rest.find('"').expect("quoted id")]
        })
        .collect();
    let count = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), count, "shape ids repeat: {ids:?}");
}

/// A deck built entirely through the authoring API validates. Without this, every test above would
/// be satisfied by a validator that rejected everything.
#[test]
fn a_deck_this_library_authored_validates() {
    let mut deck = Presentation::blank(SlideSize {
        width_emu: 12_192_000,
        height_emu: 6_858_000,
        kind: SlideSizeKind::Screen16X9,
    })
    .expect("blank");
    deck.validate().expect("a blank deck is valid");
    let slide = deck.add_slide_from_layout(0).expect("slide");
    deck.set_shape_text_content(slide, 0, "Hello")
        .expect("text");
    deck.add_slide().expect("empty slide");
    deck.validate().expect("still valid");
    deck.save().expect("and it saves");
}
