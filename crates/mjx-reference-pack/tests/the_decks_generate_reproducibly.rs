//! The four artefacts: the same bytes every time, and packages a real reader opens.
//!
//! # Why byte reproducibility is a requirement and not a nicety
//!
//! The person doing the sitting may generate the pack on one machine and export it on another, days
//! apart. *"Is this the pack the harness expects?"* then has to be answerable by comparing bytes,
//! because the alternative is trusting a memory about which build produced which file — and a plate
//! map that is off by one shape turns every row of the report into a lie about a different preset.
//!
//! It also catches a whole class of authoring defect for free: a timestamp, a random id, an
//! iteration over a hash map. None of those would fail any other test here.
//!
//! # What "opens" means
//!
//! Every artefact is opened again through the crate that wrote it, and its plate count, shape count
//! and page count are asserted against the tables the generator was driven by. That is stronger than
//! *"it saved"*: a deck that wrote 186 shapes and a table that says 187 would save perfectly well.

use std::collections::BTreeMap;

use mjx_docx::Document;
use mjx_opc::Package;
use mjx_pptx::Presentation;
use mjx_reference_pack::hanging::{hanging_document, paragraphs, HangingRole};
use mjx_reference_pack::layout::{pages_for, PLATES_PER_SLIDE};
use mjx_reference_pack::pack::{artefacts, instructions};
use mjx_reference_pack::plates::{plates, PresetDeck};
use mjx_reference_pack::typography::{page_count, probes, specimen_deck, swatches};
use mjx_reference_pack::ARTEFACTS;

#[test]
fn every_artefact_is_the_same_bytes_twice() {
    let first = artefacts().expect("the pack authors");
    let second = artefacts().expect("the pack authors again");
    assert_eq!(first.len(), ARTEFACTS.len());
    for (left, right) in first.iter().zip(&second) {
        assert_eq!(left.name, right.name);
        assert_eq!(
            left.bytes,
            right.bytes,
            "`{}` came out differently the second time it was authored ({} bytes against {}); a \
             pack whose bytes move cannot be identified after the fact",
            left.name,
            left.bytes.len(),
            right.bytes.len()
        );
        assert!(
            left.bytes.len() > 1_000,
            "`{}` is {} bytes, which is not a document",
            left.name,
            left.bytes.len()
        );
    }
    assert_eq!(
        instructions(),
        instructions(),
        "the instructions are not reproducible either"
    );
}

#[test]
fn the_preset_decks_carry_every_plate_the_table_names() {
    for which in PresetDeck::ALL {
        let table = plates(which);
        assert_eq!(
            table.len(),
            187,
            "{} has {} plates; `PresetShapeType` declares 187",
            which.file_name(),
            table.len()
        );
        let bytes = mjx_reference_pack::deck::preset_deck(which).expect("the deck authors");
        let mut deck = Presentation::open(&bytes).expect("the deck opens");
        let pages = pages_for(table.len());
        assert_eq!(deck.slide_count(), pages, "{}", which.file_name());

        let mut seen = 0usize;
        for page in 0..pages {
            let on_this_page = table.iter().filter(|plate| plate.page() == page).count();
            // One heading, then a shape and a caption for each plate.
            let expected = 1 + on_this_page * 2;
            let count = deck.shape_count(page).expect("the slide reads");
            assert_eq!(
                count,
                expected,
                "{} slide {page} has {count} shapes; {on_this_page} plates filled, stroked and \
                 captioned under one heading are {expected}",
                which.file_name()
            );
            seen += on_this_page;
        }
        assert_eq!(seen, table.len());
        assert!(
            pages * PLATES_PER_SLIDE >= table.len(),
            "the grid does not hold the table"
        );
    }
}

#[test]
fn every_plates_caption_names_its_own_preset() {
    // The caption is how a person holding a printed sheet knows which plate is which, so a caption
    // that named the wrong shape would make the whole artefact useless in a way no pixel check
    // would notice.
    let table = plates(PresetDeck::AtTheirDefaults);
    let bytes = mjx_reference_pack::deck::preset_deck(PresetDeck::AtTheirDefaults)
        .expect("the deck authors");
    let mut deck = Presentation::open(&bytes).expect("the deck opens");
    let mut checked = 0usize;
    for plate in &table {
        // Shape 0 is the heading; then plate, caption, plate, caption...
        let position = plate.index % PLATES_PER_SLIDE;
        let caption = 1 + position * 2 + 1;
        let text = deck
            .shape_text(plate.page(), caption)
            .expect("the caption reads");
        assert!(
            text.contains(plate.token),
            "the caption at slide {} shape {caption} says {text:?} and the plate there is `{}`",
            plate.page(),
            plate.token
        );
        checked += 1;
    }
    assert_eq!(checked, 187);
}

#[test]
fn the_specimen_deck_carries_every_probe_and_every_hatch() {
    let bytes = specimen_deck().expect("the deck authors");
    let mut deck = Presentation::open(&bytes).expect("the deck opens");
    assert_eq!(deck.slide_count(), page_count());

    let probes = probes();
    let swatches = swatches();
    assert_eq!(
        probes.len(),
        92 * 5,
        "92 characters across five families is 460 probes, not {}",
        probes.len()
    );
    assert_eq!(swatches.len(), 54);

    let mut shapes = 0usize;
    for page in 0..deck.slide_count() {
        shapes += deck.shape_count(page).expect("the slide reads");
    }
    // A heading per page; two text boxes per probe; one `HH` baseline box per family; one text box
    // per line-pitch specimen; a swatch and a caption per hatch.
    let baselines = mjx_reference_pack::typography::baselines();
    assert_eq!(baselines.len(), 5, "one baseline box per family");
    let expected = page_count() + probes.len() * 2 + baselines.len() + 5 + swatches.len() * 2;
    assert_eq!(
        shapes, expected,
        "the specimen deck has {shapes} shapes and its tables describe {expected}"
    );
}

#[test]
fn the_hanging_document_carries_its_three_paragraphs() {
    let bytes = hanging_document().expect("the document authors");
    let mut document = Document::open(&bytes).expect("the document opens");
    assert_eq!(document.paragraph_count().expect("count"), 3);

    for paragraph in paragraphs() {
        let text = document
            .paragraph_text(paragraph.index)
            .expect("the paragraph reads");
        for candidate in mjx_reference_pack::hanging::HANGABLE {
            assert!(
                text.contains(candidate),
                "paragraph {} ({}) does not carry `{candidate}`, so the sitting cannot answer \
                 whether Word hangs it",
                paragraph.index,
                paragraph.role.label()
            );
        }
    }

    // And the setting the whole artefact turns on, in the markup rather than in a promise.
    let package = Package::open(&bytes).expect("the package opens");
    let part = package
        .entries()
        .iter()
        .find(|entry| entry.name == "word/document.xml")
        .and_then(|entry| entry.bytes())
        .expect("the main part");
    let markup = String::from_utf8_lossy(part);
    assert_eq!(
        markup.matches("<w:overflowPunct").count(),
        3,
        "three paragraphs state `w:overflowPunct`, and the differential needs all three"
    );
    assert!(
        markup.contains("w:val=\"false\"") || markup.contains("w:val=\"0\""),
        "one paragraph must state `w:overflowPunct` as *off*, or there is no control"
    );
    assert_eq!(HangingRole::ALL.len(), 3);
}

#[test]
fn every_artefact_is_a_package_and_its_parts_are_distinct() {
    let mut sizes: BTreeMap<&str, usize> = BTreeMap::new();
    for artefact in artefacts().expect("the pack authors") {
        let package = Package::open(&artefact.bytes)
            .unwrap_or_else(|error| panic!("`{}` is not a package: {error}", artefact.name));
        let parts = package.entries().len();
        // A blank `.docx` is five parts — content types, the two relationship parts, the main
        // document and the styles — and a `.pptx` is far more; the floor is the smaller of the two.
        assert!(
            parts >= 5,
            "`{}` has {parts} parts, which is not an Office document",
            artefact.name
        );
        sizes.insert(artefact.name, artefact.bytes.len());
    }
    // Four different artefacts, not one written four times — which a `for` loop over a list that
    // quietly collapsed would produce.
    let distinct: std::collections::BTreeSet<usize> = sizes.values().copied().collect();
    assert_eq!(
        distinct.len(),
        ARTEFACTS.len(),
        "the four artefacts have {} distinct sizes: {sizes:?}",
        distinct.len()
    );
}
