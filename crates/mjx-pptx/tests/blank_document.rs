//! Creating a document from nothing: `mjx_opc::Package::empty` and `Presentation::blank`.
//!
//! The three tiers of the fidelity contract all apply to a deck the library *authored*, not just to
//! one it read:
//!
//! 1. every part re-serializes to the same decompressed bytes through save → reopen → save;
//! 2. the package reopens and resolves through the same code path a real `.pptx` takes;
//! 3. adding a slide leaves every other part byte-identical.
//!
//! Schema validity is asserted by `schema_validity.rs` and openability by `office_open.rs`; this
//! file is about structure, addressability and fidelity.

use std::collections::BTreeMap;

use mjx_dml::{ColorSchemeSlot, FontSlot};
use mjx_ooxml_types::presentationml::{PlaceholderType, SlideLayoutKind, SlideSizeKind};
use mjx_opc::{Package, PartName, TargetMode};
use mjx_pptx::{constants, PptxError, Presentation, ShapeBounds, SlideSize};

/// PowerPoint's default widescreen extent, 13⅓ × 7½ inches.
fn widescreen() -> SlideSize {
    SlideSize {
        width_emu: 12_192_000,
        height_emu: 6_858_000,
        kind: SlideSizeKind::Screen16X9,
    }
}

/// The classic 4:3 extent, 10 × 7½ inches — deliberately a *different* shape from the reference the
/// placeholder geometry was measured against, so a test using it proves the scaling ran.
fn four_by_three() -> SlideSize {
    SlideSize {
        width_emu: 9_144_000,
        height_emu: 6_858_000,
        kind: SlideSizeKind::Screen4X3,
    }
}

/// A name → decompressed-bytes map of every entry that currently has materialized bytes.
fn byte_map(package: &Package) -> BTreeMap<String, Vec<u8>> {
    package
        .entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

fn part(name: &str) -> PartName {
    PartName::new(name).expect("valid part name")
}

/// One `<Relationship>` as the graph assertions compare it: id, type URI, target.
type RelationshipRow<'a> = (&'a str, &'a str, &'a str);

// -------------------------------------------------------------------------------------------
// mjx-opc: the empty package
// -------------------------------------------------------------------------------------------

#[test]
fn an_empty_package_carries_the_two_content_type_defaults_and_a_root_rels() {
    let package = Package::empty();

    let names: Vec<&str> = package.entries().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["[Content_Types].xml", "_rels/.rels"]);

    let defaults: Vec<(&str, &str)> = package
        .content_types()
        .defaults()
        .iter()
        .map(|d| (d.extension.as_str(), d.content_type.as_str()))
        .collect();
    assert_eq!(
        defaults,
        [
            ("rels", mjx_opc::CONTENT_TYPE_RELATIONSHIPS),
            ("xml", mjx_opc::CONTENT_TYPE_XML),
        ]
    );
    assert!(package.content_types().overrides().is_empty());

    // The root relationship part exists and is empty — which is what makes it a *package* rather
    // than a bag of bytes, and what `add_relationship(None, …)` then appends to.
    let root = package
        .relationships_for(None)
        .expect("package-root relationships");
    assert_eq!(root.len(), 0);

    // A `.rels` part resolves its content type through the Default, never an Override.
    assert_eq!(
        package.content_type_of(&part("/_rels/.rels")),
        Some(mjx_opc::CONTENT_TYPE_RELATIONSHIPS)
    );
}

#[test]
fn an_empty_package_saves_and_reopens_byte_identically() {
    let package = Package::empty();
    let saved = package.save().expect("save");
    let reopened = Package::open(&saved).expect("reopen");

    assert_eq!(byte_map(&package), byte_map(&reopened));
    assert_eq!(saved, reopened.save().expect("re-save"));
}

// -------------------------------------------------------------------------------------------
// mjx-pptx: the blank presentation
// -------------------------------------------------------------------------------------------

#[test]
fn a_blank_deck_has_one_master_one_layout_and_no_slides() {
    let mut deck = Presentation::blank(widescreen()).expect("blank");

    assert_eq!(deck.slide_count(), 0);
    assert_eq!(deck.master_count(), 1);
    assert_eq!(deck.layout_count(), 1);
    assert_eq!(deck.layout_master(0), Some(0));

    assert_eq!(deck.presentation_part().as_str(), "/ppt/presentation.xml");
    assert_eq!(
        deck.master_part(0).expect("master").as_str(),
        "/ppt/slideMasters/slideMaster1.xml"
    );
    assert_eq!(
        deck.layout_part(0).expect("layout").as_str(),
        "/ppt/slideLayouts/slideLayout1.xml"
    );

    assert_eq!(
        deck.master_name(0).expect("master name").as_deref(),
        Some("Office Theme")
    );
    assert_eq!(
        deck.layout_name(0).expect("layout name").as_deref(),
        Some("Title and Text")
    );
    assert_eq!(
        deck.layout_kind(0).expect("layout kind"),
        SlideLayoutKind::Text
    );
}

#[test]
fn a_blank_deck_states_the_slide_size_it_was_asked_for() {
    for size in [widescreen(), four_by_three()] {
        let mut deck = Presentation::blank(size).expect("blank");
        assert_eq!(deck.slide_size().expect("slide size"), size);
    }
}

#[test]
fn a_slide_size_outside_what_p_sld_sz_can_express_is_refused() {
    // `ST_SlideSizeCoordinate` is bounded to 914400..=51206400 EMU. Writing anything else would
    // produce a presentation.xml no conforming consumer accepts, so it is refused up front.
    for size in [
        SlideSize {
            width_emu: 914_399,
            height_emu: 6_858_000,
            kind: SlideSizeKind::Custom,
        },
        SlideSize {
            width_emu: 12_192_000,
            height_emu: 51_206_401,
            kind: SlideSizeKind::Custom,
        },
        SlideSize {
            width_emu: 0,
            height_emu: 0,
            kind: SlideSizeKind::Custom,
        },
        SlideSize {
            width_emu: -12_192_000,
            height_emu: 6_858_000,
            kind: SlideSizeKind::Custom,
        },
    ] {
        assert!(
            matches!(
                Presentation::blank(size),
                Err(PptxError::InvalidSlideSize { .. })
            ),
            "{size:?} should be refused"
        );
    }
    // The exact boundaries are legal.
    for extent in [914_400_i64, 51_206_400] {
        assert!(Presentation::blank(SlideSize {
            width_emu: extent,
            height_emu: extent,
            kind: SlideSizeKind::Custom,
        })
        .is_ok());
    }
}

#[test]
fn a_blank_deck_ships_exactly_the_parts_it_declares_and_wires_them_together() {
    let deck = Presentation::blank(widescreen()).expect("blank");
    let bytes = deck.save().expect("save");
    let package = Package::open(&bytes).expect("reopen as a package");

    let mut names: Vec<&str> = package.entries().iter().map(|e| e.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "[Content_Types].xml",
            "_rels/.rels",
            "ppt/_rels/presentation.xml.rels",
            "ppt/presentation.xml",
            "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
            "ppt/slideLayouts/slideLayout1.xml",
            "ppt/slideMasters/_rels/slideMaster1.xml.rels",
            "ppt/slideMasters/slideMaster1.xml",
            "ppt/theme/theme1.xml",
        ]
    );

    // Every addressable part resolves to a content type, and the four XML parts to their own
    // per-part Override rather than the generic `xml` Default.
    for name in package.part_names() {
        assert!(
            package.content_type_of(&name).is_some(),
            "no content type for {}",
            name.as_str()
        );
    }
    for (name, content_type) in [
        (
            "/ppt/presentation.xml",
            constants::CONTENT_TYPE_PRESENTATION,
        ),
        (
            "/ppt/slideMasters/slideMaster1.xml",
            constants::CONTENT_TYPE_SLIDE_MASTER,
        ),
        (
            "/ppt/slideLayouts/slideLayout1.xml",
            constants::CONTENT_TYPE_SLIDE_LAYOUT,
        ),
        ("/ppt/theme/theme1.xml", constants::CONTENT_TYPE_THEME),
    ] {
        assert_eq!(
            package.content_type_of(&part(name)),
            Some(content_type),
            "content type of {name}"
        );
    }

    // The relationship graph, source by source. Every target is internal and resolves to a part
    // that is actually in the package.
    let expected: &[(Option<&str>, &[RelationshipRow])] = &[
        (
            None,
            &[(
                "rId1",
                constants::REL_OFFICE_DOCUMENT,
                "ppt/presentation.xml",
            )],
        ),
        (
            Some("/ppt/presentation.xml"),
            &[
                (
                    "rId1",
                    constants::REL_SLIDE_MASTER,
                    "slideMasters/slideMaster1.xml",
                ),
                ("rId2", constants::REL_THEME, "theme/theme1.xml"),
            ],
        ),
        (
            Some("/ppt/slideMasters/slideMaster1.xml"),
            &[
                (
                    "rId1",
                    constants::REL_SLIDE_LAYOUT,
                    "../slideLayouts/slideLayout1.xml",
                ),
                ("rId2", constants::REL_THEME, "../theme/theme1.xml"),
            ],
        ),
        (
            Some("/ppt/slideLayouts/slideLayout1.xml"),
            &[(
                "rId1",
                constants::REL_SLIDE_MASTER,
                "../slideMasters/slideMaster1.xml",
            )],
        ),
    ];
    for (source, wanted) in expected {
        let source_part = source.map(part);
        let rels = package
            .relationships_for(source_part.as_ref())
            .unwrap_or_else(|| panic!("relationships for {source:?}"));
        let actual: Vec<RelationshipRow> = rels
            .iter()
            .map(|r| (r.id.as_str(), r.rel_type.as_str(), r.target.as_str()))
            .collect();
        assert_eq!(&actual, wanted, "relationships of {source:?}");
        assert!(rels.iter().all(|r| r.mode == TargetMode::Internal));
    }

    // Nothing is orphaned: everything is reachable from the package root.
    let mut package = Package::open(&bytes).expect("reopen");
    assert_eq!(
        package.remove_unreferenced_parts().expect("sweep"),
        Vec::<PartName>::new()
    );
}

#[test]
fn a_blank_deck_round_trips_byte_identically_through_save_and_reopen() {
    let deck = Presentation::blank(widescreen()).expect("blank");
    let first = deck.save().expect("save");

    let reopened = Presentation::open(&first).expect("reopen");
    let second = reopened.save().expect("re-save");

    // Tier 1: per-part decompressed-payload byte identity.
    let before = Package::open(&first).expect("open first");
    let after = Package::open(&second).expect("open second");
    assert_eq!(byte_map(&before), byte_map(&after));

    // Structural container identity: the same entries, in the same order.
    let names_before: Vec<&str> = before.entries().iter().map(|e| e.name.as_str()).collect();
    let names_after: Vec<&str> = after.entries().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names_before, names_after);
}

#[test]
fn blank_is_deterministic() {
    // Two blank decks of the same size are byte-for-byte the same document. A constructor that
    // reached for a clock, a random id or a hash iteration order would fail here.
    let a = Presentation::blank(widescreen()).expect("blank");
    let b = Presentation::blank(widescreen()).expect("blank");
    assert_eq!(
        byte_map(&Package::open(&a.save().expect("save")).expect("open")),
        byte_map(&Package::open(&b.save().expect("save")).expect("open"))
    );
}

#[test]
fn a_slide_built_from_the_blank_layout_carries_a_title_and_a_body_placeholder() {
    let mut deck = Presentation::blank(widescreen()).expect("blank");
    let slide = deck
        .add_slide_from_layout(0)
        .expect("add slide from layout");

    assert_eq!(deck.slide_count(), 1);
    assert_eq!(slide, 0);
    assert_eq!(deck.slide_layout(slide).expect("slide layout"), Some(0));
    assert_eq!(
        deck.slide_part(slide).expect("slide part").as_str(),
        "/ppt/slides/slide1.xml"
    );

    assert_eq!(deck.shape_count(slide).expect("shape count"), 2);
    let kinds: Vec<PlaceholderType> = (0..2)
        .map(|shape| {
            deck.shape_placeholder(slide, shape)
                .expect("placeholder")
                .expect("is a placeholder")
                .kind
        })
        .collect();
    assert_eq!(kinds, [PlaceholderType::Title, PlaceholderType::Body]);

    deck.set_shape_text_content(slide, 0, "Quarterly results")
        .expect("set title");
    deck.set_shape_text_content(slide, 1, "Revenue up 14%")
        .expect("set body");
    assert_eq!(
        deck.shape_text(slide, 0).expect("title"),
        "Quarterly results"
    );
    assert_eq!(deck.shape_text(slide, 1).expect("body"), "Revenue up 14%");
}

#[test]
fn a_placeholder_on_a_blank_deck_inherits_the_master_it_was_authored_with() {
    // Nothing on the slide states a size, a font or a position: all three come from the master and
    // the theme this constructor wrote. These are the exact numbers `blank.rs` puts there, so the
    // assertions can only pass if the whole inheritance chain — slide → layout → master → theme —
    // resolves through markup we authored.
    let mut deck = Presentation::blank(widescreen()).expect("blank");
    let slide = deck.add_slide_from_layout(0).expect("add slide");
    deck.set_shape_text_content(slide, 0, "Title")
        .expect("set title");
    deck.set_shape_text_content(slide, 1, "Body")
        .expect("set body");

    let title = deck
        .effective_run_properties(slide, 0, 0, 0)
        .expect("effective title run properties");
    assert_eq!(title.size_points(), Some(44.0));

    let body = deck
        .effective_run_properties(slide, 1, 0, 0)
        .expect("effective body run properties");
    assert_eq!(body.size_points(), Some(28.0));

    // Position is stated only on the master; the slide and the layout both leave it out.
    let bounds = deck
        .effective_shape_bounds(slide, 0)
        .expect("effective bounds")
        .expect("the master states them");
    assert_eq!(bounds.offset_x_emu, 838_200);
    assert_eq!(bounds.offset_y_emu, 365_125);
    assert_eq!(bounds.width_emu, 10_515_600);
    assert_eq!(bounds.height_emu, 1_325_563);
}

#[test]
fn placeholder_geometry_follows_a_narrower_slide() {
    // The 4:3 deck is 3/4 as wide, so the title's horizontal measurements scale with it while its
    // vertical ones (the two decks are the same height) do not. A constructor that hard-coded the
    // widescreen numbers would put the title 1.7 inches off the right edge.
    let mut deck = Presentation::blank(four_by_three()).expect("blank");
    let slide = deck.add_slide_from_layout(0).expect("add slide");
    let bounds = deck
        .effective_shape_bounds(slide, 0)
        .expect("effective bounds")
        .expect("the master states them");

    assert_eq!(bounds.offset_x_emu, 838_200 * 3 / 4);
    assert_eq!(bounds.width_emu, 10_515_600 * 3 / 4);
    assert_eq!(bounds.offset_y_emu, 365_125);
    assert!(
        bounds.offset_x_emu + bounds.width_emu <= 9_144_000,
        "the title must fit inside the slide"
    );
}

#[test]
fn adding_a_slide_touches_only_the_presentation_part() {
    // Tier 3, edit isolation: on a deck we authored just as on one we opened, adding a slide must
    // leave every pre-existing part byte-identical. Only `presentation.xml` (a new `p:sldId`) and
    // its `.rels` may change.
    let blank = Presentation::blank(widescreen()).expect("blank");
    let before = Package::open(&blank.save().expect("save")).expect("open");
    let before_bytes = byte_map(&before);

    let mut deck = Presentation::blank(widescreen()).expect("blank");
    deck.add_slide_with_text("Hello", ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0))
        .expect("add slide with text");
    let after = Package::open(&deck.save().expect("save")).expect("open");
    let after_bytes = byte_map(&after);

    let changed: Vec<&String> = before_bytes
        .iter()
        .filter(|(name, bytes)| after_bytes.get(*name) != Some(*bytes))
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        changed,
        [
            "[Content_Types].xml",
            "ppt/_rels/presentation.xml.rels",
            "ppt/presentation.xml"
        ],
        "adding a slide changed more than the parts that name it"
    );

    // And the new parts are exactly the slide and its relationships.
    let added: Vec<&String> = after_bytes
        .keys()
        .filter(|name| !before_bytes.contains_key(*name))
        .collect();
    assert_eq!(
        added,
        ["ppt/slides/_rels/slide1.xml.rels", "ppt/slides/slide1.xml"]
    );
}

#[test]
fn add_slide_works_on_a_deck_that_has_no_slide_to_inherit_from() {
    // `add_slide` normally copies slide 0's layout relationship. A blank deck has no slide 0, so it
    // has to fall back to the deck's own first layout — otherwise `blank()` would hand back a deck
    // nothing could be put on.
    let mut deck = Presentation::blank(widescreen()).expect("blank");
    let first = deck.add_slide().expect("add the first slide");
    assert_eq!(first, 0);
    assert_eq!(deck.slide_layout(first).expect("layout"), Some(0));

    // The second slide takes the ordinary path (inherit from slide 0) and lands on the same layout.
    let second = deck.add_slide().expect("add a second slide");
    assert_eq!(second, 1);
    assert_eq!(deck.slide_layout(second).expect("layout"), Some(0));
    assert_eq!(
        deck.slide_part(second).expect("part").as_str(),
        "/ppt/slides/slide2.xml"
    );
}

#[test]
fn a_deck_built_from_nothing_reopens_and_reads_back_what_was_written() {
    // The end-to-end claim: build, save, and then read the result with the *reader*, not the
    // builder's own view of it.
    let mut deck = Presentation::blank(four_by_three()).expect("blank");
    let slide = deck.add_slide_from_layout(0).expect("add slide");
    deck.set_shape_text_content(slide, 0, "Built from nothing")
        .expect("set title");
    deck.add_text_box(
        slide,
        "…and a text box",
        ShapeBounds::from_inches(1.0, 4.0, 4.0, 1.0),
    )
    .expect("add text box");
    let bytes = deck.save().expect("save");

    let mut reopened = Presentation::open(&bytes).expect("reopen");
    assert_eq!(reopened.slide_count(), 1);
    assert_eq!(reopened.master_count(), 1);
    assert_eq!(reopened.layout_count(), 1);
    assert_eq!(reopened.slide_size().expect("slide size"), four_by_three());
    assert_eq!(reopened.shape_count(0).expect("shapes"), 3);
    assert_eq!(
        reopened.shape_text(0, 0).expect("title"),
        "Built from nothing"
    );
    assert_eq!(
        reopened.shape_text(0, 2).expect("text box"),
        "…and a text box"
    );

    // The theme travelled with it, and it is the theme this constructor wrote: the Office palette,
    // the Calibri pair, and the three fill styles `a:fillStyleLst` demands.
    let theme = reopened.theme(0).expect("theme").expect("a theme");
    let fonts = theme.font_scheme().expect("a font scheme");
    assert_eq!(
        fonts
            .major()
            .font(FontSlot::Latin)
            .map(|f| f.typeface.as_str()),
        Some("Calibri Light")
    );
    assert_eq!(
        fonts
            .minor()
            .font(FontSlot::Latin)
            .map(|f| f.typeface.as_str()),
        Some("Calibri")
    );
    assert_eq!(theme.fill_styles().len(), 3);
    assert_eq!(theme.line_styles().len(), 3);
    assert!(theme.color(ColorSchemeSlot::Accent1).is_some());
}
