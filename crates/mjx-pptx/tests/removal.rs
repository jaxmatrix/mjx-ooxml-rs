//! Integration tests for removal: taking a shape back off a slide, a layout or a master, and taking
//! a slide back out of a deck.
//!
//! Removal is the half of the API that construction has always been missing. What is tested here is
//! that it closes the gap in the one shape index space, that the part still parses afterwards, and
//! that it touches nothing else — and, for a slide, that the whole
//! `p:sldIdLst` → relationship → part chain is unwired consistently.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_opc::{Package, PartName};
use mjx_pptx::{Hyperlink, PptxError, Presentation, ShapeBounds, Surface};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

const BOUNDS: ShapeBounds = ShapeBounds {
    offset_x_emu: 914_400,
    offset_y_emu: 914_400,
    width_emu: 3_657_600,
    height_emu: 1_828_800,
};

/// A slide carrying the fixture's title plus three labelled text boxes.
fn deck_with_three_boxes() -> Presentation {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    for label in ["one", "two", "three"] {
        pres.add_text_box(0, label, BOUNDS).expect("add");
    }
    pres
}

#[test]
fn removing_a_shape_closes_the_gap_in_the_index_space() {
    let mut pres = deck_with_three_boxes();
    assert_eq!(pres.shape_count(0).expect("count"), 4); // the fixture title + three boxes

    pres.remove_shape(0, 1).expect("remove the first box");

    assert_eq!(pres.shape_count(0).expect("count"), 3);
    assert_eq!(pres.shape_text(0, 0).expect("text"), "Hello OOXML");
    assert_eq!(
        pres.shape_text(0, 1).expect("text"),
        "two",
        "the shapes after the removed one move down one index"
    );
    assert_eq!(pres.shape_text(0, 2).expect("text"), "three");
}

#[test]
fn the_slide_still_parses_after_a_removal() {
    let mut pres = deck_with_three_boxes();
    pres.remove_shape(0, 2).expect("remove the middle box");

    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(reopened.shape_count(0).expect("count"), 3);
    let texts: Vec<String> = (0..3)
        .map(|idx| reopened.shape_text(0, idx).expect("text"))
        .collect();
    assert_eq!(texts, vec!["Hello OOXML", "one", "three"]);
}

#[test]
fn every_shape_can_be_removed_leaving_an_empty_shape_tree() {
    let mut pres = deck_with_three_boxes();
    for _ in 0..4 {
        pres.remove_shape(0, 0).expect("remove");
    }
    assert_eq!(pres.shape_count(0).expect("count"), 0);

    // An empty shape tree still round-trips through the reader.
    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(reopened.shape_count(0).expect("count"), 0);
}

#[test]
fn a_picture_is_removed_like_any_other_shape_but_keeps_its_image() {
    const PNG: &[u8] = &[
        0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0x0d, b'I', b'H', b'D', b'R', 0,
        0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0,
    ];
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let picture = pres.add_picture(0, PNG, BOUNDS).expect("add picture");
    pres.remove_shape(0, picture).expect("remove the picture");

    let saved = pres.save().expect("save");
    let pkg = Package::open(&saved).expect("reopen");
    assert!(
        pkg.part_names()
            .any(|p| p.as_str().starts_with("/ppt/media/")),
        "the image part stays: removing a shape is not a package garbage collection"
    );
    let mut reopened = Presentation::open(&saved).expect("reopen presentation");
    assert_eq!(reopened.shape_count(0).expect("count"), 1);
}

#[test]
fn a_layouts_shape_can_be_removed_too() {
    // Removal is Surface-addressed like every other shape call: this drops the layout's footer slot,
    // and with it the footer every slide on that layout was inheriting.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    assert_eq!(pres.shape_count(Surface::Layout(1)).expect("count"), 5);

    pres.remove_shape(Surface::Layout(1), 3).expect("remove");

    assert_eq!(pres.shape_count(Surface::Layout(1)).expect("count"), 4);
    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reopened.shape_count(Surface::Layout(1)).expect("count"),
        4,
        "the layout part still parses"
    );
}

#[test]
fn an_out_of_range_shape_is_rejected_and_names_its_surface() {
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let err = pres
        .remove_shape(Surface::Layout(1), 9)
        .expect_err("no such shape");
    match err {
        PptxError::ShapeIndexOutOfRange {
            surface,
            path,
            count: 5,
        } => {
            assert_eq!(surface.to_string(), "layout 1");
            assert_eq!(path.indices(), [9]);
        }
        other => panic!("unexpected error: {other:?}"),
    }
    assert_eq!(
        pres.shape_count(Surface::Layout(1)).expect("count"),
        5,
        "a rejected removal changes nothing"
    );
}

#[test]
fn removing_a_shape_leaves_every_other_part_byte_identical() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.remove_shape(0, 0).expect("remove the title");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    assert_eq!(
        snapshot.keys().collect::<Vec<_>>(),
        reopened.keys().collect::<Vec<_>>(),
        "removing a shape adds and removes no parts"
    );
    const SLIDE: &str = "ppt/slides/slide1.xml";
    assert_ne!(
        reopened.get(SLIDE),
        snapshot.get(SLIDE),
        "the edited slide should differ"
    );
    for (name, original) in &snapshot {
        if name == SLIDE {
            continue;
        }
        assert_eq!(
            reopened.get(name),
            Some(original),
            "part {name} must be byte-identical"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Slide removal. `layouts.pptx` has two slides on different layouts, so a removal can be seen to
// take the right one.
// ---------------------------------------------------------------------------------------------

#[test]
fn removing_a_slide_takes_the_right_one_and_shifts_the_rest() {
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    assert_eq!(pres.slide_count(), 2);
    let survivor_text = pres.shape_text(1, 0).expect("text of slide 1");
    let survivor_layout = pres.slide_layout(1).expect("layout of slide 1");

    pres.remove_slide(0).expect("remove slide 0");

    assert_eq!(pres.slide_count(), 1);
    assert_eq!(
        pres.shape_text(0, 0).expect("text"),
        survivor_text,
        "the surviving slide moved down to index 0, unchanged"
    );
    assert_eq!(pres.slide_layout(0).expect("layout"), survivor_layout);
}

#[test]
fn a_deck_with_a_slide_removed_reopens_consistently() {
    // The real proof: the p:sldIdLst, the relationships and the part set we rewrote must agree well
    // enough for a fresh `open` to resolve the deck.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let removed_part = pres.slide_part(0).expect("slide 0").clone();
    let survivor_text = pres.shape_text(1, 0).expect("text");

    pres.remove_slide(0).expect("remove");
    let saved = pres.save().expect("save");

    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(reopened.slide_count(), 1);
    assert_eq!(reopened.shape_text(0, 0).expect("text"), survivor_text);

    let pkg = Package::open(&saved).expect("reopen package");
    assert!(
        !pkg.part_names().any(|p| p == removed_part),
        "the slide part survived"
    );
    assert!(
        !pkg.entries()
            .iter()
            .any(|e| e.name == "ppt/slides/_rels/slide1.xml.rels"),
        "the slide's own .rels survived"
    );
}

#[test]
fn removing_a_slide_takes_the_images_only_it_showed() {
    const PNG: &[u8] = &[
        0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0x0d, b'I', b'H', b'D', b'R', 0,
        0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0,
    ];
    const GIF: &[u8] = b"GIF89a\x01\x00\x01\x00\x00\x00\x00;";

    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    // The shared image is on both slides; the exclusive one only on slide 0.
    pres.add_picture(0, PNG, BOUNDS).expect("shared on slide 0");
    pres.add_picture(1, PNG, BOUNDS).expect("shared on slide 1");
    pres.add_picture(0, GIF, BOUNDS).expect("exclusive");

    pres.remove_slide(0).expect("remove");
    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");

    let media: Vec<String> = pkg
        .part_names()
        .filter(|p| p.as_str().starts_with("/ppt/media/"))
        .map(|p| p.as_str().to_owned())
        .collect();
    assert!(
        media.iter().any(|m| m.ends_with(".png")),
        "the image the surviving slide still shows was deleted: {media:?}"
    );
    assert!(
        !media.iter().any(|m| m.ends_with(".gif")),
        "the image only the removed slide showed survived: {media:?}"
    );
}

#[test]
fn a_removed_slides_part_name_is_not_recycled() {
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    pres.remove_slide(0).expect("remove slide1.xml");
    let added = pres.add_slide_from_layout(2).expect("add");

    assert_eq!(
        pres.slide_part(added).expect("new slide").as_str(),
        "/ppt/slides/slide3.xml",
        "a new slide is numbered past every part, never into a freed name"
    );
    let reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(reopened.slide_count(), 2);
}

#[test]
fn an_out_of_range_slide_is_rejected() {
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let err = pres.remove_slide(9).expect_err("no such slide");
    assert!(
        matches!(err, PptxError::SlideIndexOutOfRange { index: 9, count: 2 }),
        "{err:?}"
    );
    assert_eq!(pres.slide_count(), 2, "a rejected removal changes nothing");
}

#[test]
fn removing_a_slide_touches_only_the_deck_wiring() {
    let bytes = fixture("layouts.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.remove_slide(0).expect("remove");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    // Exactly the deck wiring changes: the slide list, the relationship to the slide, and the
    // content-type override. The slide and its .rels are gone; nothing else moves.
    let rewritten = [
        "ppt/presentation.xml",
        "ppt/_rels/presentation.xml.rels",
        "[Content_Types].xml",
    ];
    let deleted = ["ppt/slides/slide1.xml", "ppt/slides/_rels/slide1.xml.rels"];
    for (name, original) in &snapshot {
        if rewritten.contains(&name.as_str()) {
            continue;
        }
        if deleted.contains(&name.as_str()) {
            assert!(!reopened.contains_key(name), "part {name} should be gone");
            continue;
        }
        assert_eq!(
            reopened.get(name),
            Some(original),
            "part {name} must be byte-identical"
        );
    }
    assert!(
        reopened.keys().all(|name| snapshot.contains_key(name)),
        "removing a slide added a part"
    );
}

// ---------------------------------------------------------------------------------------------
// References *to* the removed slide (MJXOFF-212).
//
// A slide is pointed at by more than `p:sldIdLst`: another slide can hyperlink to it and a custom
// show can list it. Each such reference is a relationship *plus* an element naming it, and both go
// with the slide — anything less leaves a package `save()` refuses, which is the defect this
// section locks down. `hyperlinks.pptx` ships the shape-level case; the run-level one is authored
// through the public API and the custom show is stitched into a copy of the fixture.
// ---------------------------------------------------------------------------------------------

/// The presentation part of every deck this file builds.
fn presentation_part() -> PartName {
    PartName::new("/ppt/presentation.xml").expect("a valid part name")
}

/// `bytes` with `part`'s markup put through `edit`, re-saved. The edit is textual because the point
/// is to hand `Presentation` markup it did not author.
fn with_part_text(bytes: &[u8], part: &PartName, edit: impl FnOnce(&str) -> String) -> Vec<u8> {
    let mut pkg = Package::open(bytes).expect("open");
    let original = std::str::from_utf8(pkg.part_bytes(part).expect("the part's bytes"))
        .expect("the part is UTF-8");
    let edited = edit(original);
    pkg.replace_part_bytes(part, edited.into_bytes())
        .expect("replace");
    pkg.save().expect("save")
}

#[test]
fn removing_a_slide_a_shape_jumps_to_leaves_a_package_that_saves() {
    let mut pres = Presentation::open(&fixture("hyperlinks.pptx")).expect("open");
    // Slide 0's rectangle carries `p:cNvPr > a:hlinkClick` jumping to slide 1, through `rId3` of
    // `slide1.xml.rels`.
    assert_eq!(
        pres.shape_hyperlink(0, 1).expect("link"),
        Some(Hyperlink::Slide(1))
    );

    pres.remove_slide(1).expect("remove");
    // Before MJXOFF-212 this was `Err`, permanently: the file could never be written back.
    let saved = pres.save().expect("the package must still be writable");
    let map = byte_map(&Package::open(&saved).expect("reopen"));

    let rels = String::from_utf8(map["ppt/slides/_rels/slide1.xml.rels"].clone()).expect("utf8");
    assert!(
        !rels.contains("rId3"),
        "the relationship to the removed slide is gone: {rels}"
    );
    assert!(
        rels.contains("https://example.com/"),
        "the run's external link is untouched: {rels}"
    );

    let slide = String::from_utf8(map["ppt/slides/slide1.xml"].clone()).expect("utf8");
    assert!(
        !slide.contains("hlinksldjump"),
        "the jump goes with the slide it jumped to: {slide}"
    );
    assert!(
        slide.contains("Go to slide 2"),
        "the shape keeps its text: {slide}"
    );
    assert!(
        slide.contains(r#"hlinkClick r:id="rId2""#),
        "the run's own hyperlink is left alone: {slide}"
    );

    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(reopened.slide_count(), 1);
    assert_eq!(
        reopened.shape_hyperlink(0, 1).expect("link"),
        None,
        "the shape is still there, and no longer a link"
    );
}

#[test]
fn removing_a_slide_a_run_links_to_keeps_the_text_and_drops_the_link() {
    let mut pres = Presentation::open(&fixture("hyperlinks.pptx")).expect("open");
    // Slide 1's text box has no link; give its run a jump back to slide 0, then delete slide 0.
    pres.set_run_hyperlink(1, 0, 0, 0, &Hyperlink::Slide(0))
        .expect("set");
    pres.remove_slide(0).expect("remove");
    let saved = pres.save().expect("the package must still be writable");
    let map = byte_map(&Package::open(&saved).expect("reopen"));

    let slide = String::from_utf8(map["ppt/slides/slide2.xml"].clone()).expect("utf8");
    assert!(
        slide.contains("Second slide"),
        "the run keeps its text: {slide}"
    );
    assert!(!slide.contains("hlinkClick"), "and loses its link: {slide}");
    let rels = String::from_utf8(map["ppt/slides/_rels/slide2.xml.rels"].clone()).expect("utf8");
    assert!(
        !rels.contains("relationships/slide\""),
        "the relationship it named goes too: {rels}"
    );
}

#[test]
fn removing_a_slide_a_custom_show_lists_drops_the_entry_and_keeps_the_show() {
    // `p:custShow > p:sldLst > p:sld` names the *presentation's* own slide relationships, so a
    // custom show is the second way a package can be left unwritable by a slide removal.
    let bytes = with_part_text(
        &fixture("hyperlinks.pptx"),
        &presentation_part(),
        |presentation| {
            presentation.replace(
                "</p:presentation>",
                concat!(
                    "  <p:custShowLst><p:custShow name=\"Short\" id=\"0\"><p:sldLst>",
                    "<p:sld r:id=\"rId2\"/><p:sld r:id=\"rId3\"/>",
                    "</p:sldLst></p:custShow></p:custShowLst>\n</p:presentation>",
                ),
            )
        },
    );

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.remove_slide(1).expect("remove");
    let saved = pres.save().expect("the package must still be writable");
    let map = byte_map(&Package::open(&saved).expect("reopen"));

    let presentation = String::from_utf8(map["ppt/presentation.xml"].clone()).expect("utf8");
    assert!(
        !presentation.contains(r#"r:id="rId3""#),
        "every reference to the removed slide is gone: {presentation}"
    );
    assert!(
        presentation.contains(r#"<p:sld r:id="rId2"/>"#),
        "the show still lists the slide that stayed: {presentation}"
    );
    assert!(
        presentation.contains("p:custShow"),
        "the show itself is not deleted with its entry: {presentation}"
    );
}

#[test]
fn an_element_outside_the_schema_naming_the_slide_goes_with_it() {
    // The rule is keyed on the *reference*, not on a list of element names: an element this build
    // has never heard of that names the removed slide is removed just the same. Keyed on names
    // instead, this markup would keep an `r:id` naming a relationship nothing declares, and
    // `Package::validate` refuses that as `UndeclaredRelationshipReference`. The element is not
    // PresentationML and the package is never schema-validated — it exists to state the rule.
    let bytes = with_part_text(
        &fixture("hyperlinks.pptx"),
        &PartName::new("/ppt/slides/slide1.xml").expect("a valid part name"),
        |slide| {
            slide.replace(
                "</p:sld>",
                "  <z:jump xmlns:z=\"urn:example:mjx-test\" r:id=\"rId3\"/>\n</p:sld>",
            )
        },
    );

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.remove_slide(1).expect("remove");
    let saved = pres.save().expect("the package must still be writable");
    let map = byte_map(&Package::open(&saved).expect("reopen"));

    let slide = String::from_utf8(map["ppt/slides/slide1.xml"].clone()).expect("utf8");
    assert!(
        !slide.contains("z:jump"),
        "an unknown element naming the removed slide stayed behind: {slide}"
    );
}

#[test]
fn a_part_that_holds_the_relationship_but_names_it_nowhere_is_left_alone() {
    // **The bound on MJXOFF-212's blast radius.** The sweep visits every part holding a relationship
    // to the removed slide, and a part can hold one it names nowhere in markup — an unreferenced
    // relationship is valid OOXML, which is why `remove_shape` documents that it leaves
    // relationships alone. The relationship still has to go, because its target is about to vanish.
    // The part itself must not be touched.
    //
    // The state is reached through the shipped API, not a hand-built package: `set_shape_hyperlink`
    // adds the relationship and the `a:hlinkClick` naming it, and `remove_shape` takes the shape —
    // and with it the markup — leaving the relationship behind.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    pres.set_shape_hyperlink(1, 0, &Hyperlink::Slide(0))
        .expect("link slide 1's shape 0 to slide 0");
    pres.remove_shape(1, 0)
        .expect("remove the shape, which keeps the relationship");
    let staged = pres.save().expect("save");

    // Then give the surviving slide something it *arrived* with and this library would refuse to
    // author: two shapes sharing a `p:cNvPr@id`. This is what makes "left alone" observable at all.
    // Byte equality cannot say it — the fidelity serializer reproduces an unmutated tree byte for
    // byte, so "kept its original bytes" and "re-serialized without changing anything" are the same
    // bytes. What differs is **provenance**, and provenance decides scope: `Package::validate` and
    // `Presentation::validate` walk the parts this library authored and spare the ones it did not,
    // so dirtying an untouched part drags a file the caller never edited into our own checks. A deck
    // that opened and saved a moment ago would stop saving because a slide it was not asked about
    // was rewritten.
    let survivor = PartName::new("/ppt/slides/slide2.xml").expect("a valid part name");
    let arrived = with_part_text(&staged, &survivor, |slide| {
        let edited = slide.replace(
            r#"<p:cNvPr id="7" name="Table 6"/>"#,
            r#"<p:cNvPr id="3" name="Table 6"/>"#,
        );
        assert_ne!(edited, slide, "the duplicate-id edit matched nothing");
        edited
    });

    let before = byte_map(&Package::open(&arrived).expect("reopen"));
    let staged_rels =
        String::from_utf8(before["ppt/slides/_rels/slide2.xml.rels"].clone()).expect("utf8");
    assert!(
        staged_rels.contains("relationships/slide\""),
        "the setup must leave a slide relationship behind: {staged_rels}"
    );
    let staged_slide = String::from_utf8(before["ppt/slides/slide2.xml"].clone()).expect("utf8");
    assert!(
        !staged_slide.contains("hlinkClick"),
        "and must leave no markup naming it: {staged_slide}"
    );
    Presentation::open(&arrived)
        .expect("open")
        .save()
        .expect("the premise: as it arrived, this deck saves");

    let mut pres = Presentation::open(&arrived).expect("reopen");
    pres.remove_slide(0).expect("remove");
    let saved = pres
        .save()
        .expect("a slide the removal never named was rewritten, and is now faulted for what it arrived with");
    let after = byte_map(&Package::open(&saved).expect("reopen"));

    assert_eq!(
        after["ppt/slides/slide2.xml"], before["ppt/slides/slide2.xml"],
        "the part came back changed"
    );
    let after_rels =
        String::from_utf8(after["ppt/slides/_rels/slide2.xml.rels"].clone()).expect("utf8");
    assert!(
        !after_rels.contains("relationships/slide\""),
        "the dangling relationship still had to go: {after_rels}"
    );
}
