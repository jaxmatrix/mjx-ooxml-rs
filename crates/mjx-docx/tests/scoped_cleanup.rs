//! An edit cleans up after itself and nothing more (MJXOFF-209).
//!
//! Three `Document` edits used to finish by calling `Package::remove_unreferenced_parts`, the
//! **package-wide** sweep: `remove_header`/`remove_footer`, removing the last comment, and
//! `remove_drawing`. That sweep deletes every part it cannot reach from the package root — including
//! one the *producer* left unreferenced in the file they handed us. Removing a header would take an
//! unrelated image with it, which is an editing library changing what it was not asked to change.
//!
//! `mjx-pptx` never had the problem: its sweep is the opt-in `Presentation::remove_unused_parts`,
//! and these three were the only automatic callers in the workspace. The fix is not to stop cleaning
//! up — the three doc comments promise it and callers rely on it — but to scope the clean-up to the
//! part the edit itself orphaned, through `Package::remove_part_if_unreferenced`.
//!
//! Every test here therefore asserts **both** halves: the part the edit stranded is gone, and the
//! part it never touched is still there. Asserting only the first is what let the sweep sit here.

use mjx_docx::{Document, HeaderFooterType, PageSize, SectionLocation};
use mjx_fixtures::fixture;
use mjx_opc::{Package, PartName};

fn part(name: &str) -> PartName {
    PartName::new(name).expect("a valid part name")
}

/// The part every test plants: a real entry, with a content type, that no relationship names —
/// exactly the shape a producer leaves behind after their own editing session.
const PRODUCER_ORPHAN: &str = "/word/media/producer-orphan.png";

/// Re-opens `bytes` as a package, plants [`PRODUCER_ORPHAN`], and hands back a `Document` over the
/// result. `save_unchecked` is right here: an unreferenced part is legal, but writing it through
/// `Document` would be a second edit, and the point is that the orphan predates every edit below.
fn with_a_producer_orphan(bytes: &[u8]) -> Document {
    let mut package = Package::open(bytes).expect("the package opens");
    package
        .insert_part(&part(PRODUCER_ORPHAN), "image/png", b"orphan".to_vec())
        .expect("the orphan is planted");
    let planted = package.save_unchecked().expect("it saves");
    assert!(
        Package::open(&planted)
            .expect("it reopens")
            .part_bytes(&part(PRODUCER_ORPHAN))
            .is_some(),
        "the fixture for this test must actually carry the orphan"
    );
    Document::open(&planted).expect("the document opens")
}

fn part_names(bytes: &[u8]) -> Vec<String> {
    Package::open(bytes)
        .expect("it reopens")
        .part_names()
        .map(|p| p.as_str().to_owned())
        .collect()
}

fn assert_orphan_survived(bytes: &[u8]) {
    assert!(
        part_names(bytes).contains(&PRODUCER_ORPHAN.to_owned()),
        "an edit deleted a part the producer left and this call never mentioned: {:?}",
        part_names(bytes)
    );
}

#[test]
fn removing_a_header_takes_its_part_and_leaves_the_producers_orphan() {
    let blank = {
        let mut document = Document::blank(PageSize::a4()).expect("a blank document");
        document
            .create_header(SectionLocation::Body, HeaderFooterType::Default)
            .expect("the header part is created");
        document.save().expect("it saves")
    };
    let header = part("/word/header1.xml");
    assert!(part_names(&blank).contains(&header.as_str().to_owned()));

    let mut document = with_a_producer_orphan(&blank);
    document
        .remove_header(SectionLocation::Body, HeaderFooterType::Default)
        .expect("the header is removed");
    let saved = document.save().expect("it saves");

    assert!(
        !part_names(&saved).contains(&header.as_str().to_owned()),
        "the part this call orphaned must still go"
    );
    assert_orphan_survived(&saved);
}

#[test]
fn removing_the_last_comment_takes_the_comments_part_and_leaves_the_producers_orphan() {
    let blank = {
        let mut document = Document::blank(PageSize::a4()).expect("a blank document");
        document
            .add_comment(0, "Jane Doe", None, "a comment")
            .expect("the comment is added");
        document.save().expect("it saves")
    };
    let comments = part("/word/comments.xml");
    assert!(part_names(&blank).contains(&comments.as_str().to_owned()));

    let mut document = with_a_producer_orphan(&blank);
    let id = document
        .comments(|comments, interner| {
            comments
                .comments()
                .map(|comment| comment.id(interner).expect("an id"))
                .collect::<Vec<_>>()
        })
        .expect("the comments part is readable")
        .expect("it exists");
    document
        .remove_comment(id[0])
        .expect("the comment is removed");
    let saved = document.save().expect("it saves");

    assert!(
        !part_names(&saved).contains(&comments.as_str().to_owned()),
        "the part this call orphaned must still go"
    );
    assert_orphan_survived(&saved);
}

#[test]
fn removing_a_drawing_takes_its_image_and_leaves_the_producers_orphan() {
    let (blank, drawing_id) = {
        let mut document = Document::blank(PageSize::a4()).expect("a blank document");
        document
            .insert_run(0, 0, "A picture.")
            .expect("some text to hang it on");
        let id = document
            .add_inline_picture(
                0usize,
                png(),
                "image/png",
                "png",
                914_400,
                914_400,
                "Picture 1",
            )
            .expect("the picture is added");
        (document.save().expect("it saves"), id)
    };
    let image = part("/word/media/image1.png");
    assert!(part_names(&blank).contains(&image.as_str().to_owned()));

    let mut document = with_a_producer_orphan(&blank);
    assert!(document
        .remove_drawing(drawing_id)
        .expect("the drawing is removed"));
    let saved = document.save().expect("it saves");

    assert!(
        !part_names(&saved).contains(&image.as_str().to_owned()),
        "the part this call orphaned must still go"
    );
    assert_orphan_survived(&saved);
}

#[test]
fn an_edit_to_a_package_with_encoded_targets_keeps_every_other_part() {
    // The two halves of MJXOFF-209 meeting: `percent_encoded_targets.docx` addresses five parts
    // through an escape, and its header part is one of them. Removing that header must take exactly
    // the header — before the decode landed, the sweep could reach none of the five and deleted all
    // of them; before the scoping landed, it would still have deleted anything else left over.
    let mut document =
        Document::open(&fixture("percent_encoded_targets.docx")).expect("the fixture opens");
    document
        .remove_header(SectionLocation::Body, HeaderFooterType::Default)
        .expect("the header is removed");
    let saved = document.save().expect("it saves");
    let after = part_names(&saved);

    assert!(
        !after.contains(&"/word/my header.xml".to_owned()),
        "the header this call orphaned must go: {after:?}"
    );
    for surviving in [
        "/word/media/logo one.png",
        "/word/media/plain%20name.png",
        "/word/media/lower_case.png",
        "/word/media/upper.png",
    ] {
        assert!(
            after.contains(&surviving.to_owned()),
            "{surviving} is referenced through an escape and must survive an edit about the \
             header: {after:?}"
        );
    }
}

/// A tiny, real 1×1 PNG — the media bytes this crate stores verbatim.
fn png() -> Vec<u8> {
    vec![
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, b'I', b'H', b'D',
        b'R', 0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00,
        0x0C, b'I', b'D', b'A', b'T', 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01,
        0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00, 0x00, b'I', b'E', b'N', b'D', 0xAE,
        0x42, 0x60, 0x82,
    ]
}
