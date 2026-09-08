//! The fidelity contract, unchanged by residency.
//!
//! # What batched commit moved, and what it did not
//!
//! `CLAUDE.md` stated copy-on-write as *on first edit, serialize from the model and drop raw bytes.*
//! MJXOFF-167 split that into two moments — the bytes are dropped and the part marked dirty at the
//! first edit; the XML is written at the commit — and this suite is what says the split cost
//! nothing. The round-trip guarantee is **per-part decompressed-payload byte identity**, and an
//! untouched part is never marked dirty, so it is still re-emitted from the container's own bytes.
//!
//! Only the timing moved. These gates check the thing that did not.

#![cfg(feature = "ooxml")]

use mjx_pptx::{Package, Presentation};
use mjx_session::ooxml::{PresentationSession, SpreadsheetSession, WordSession};
use mjx_session::{
    CommitPolicy, ManualClock, MemoryDocument, MemoryJournal, Operation, Session, Value,
};

#[path = "support/mod.rs"]
mod support;

use support::{
    cell_address, first_run_after_the_first_paragraph, first_run_in_deck, first_run_in_document,
    fixture,
};

/// Every entry of a container, by name, with its decompressed payload.
fn payloads(container: &[u8]) -> Vec<(String, Vec<u8>)> {
    let package = Package::open(container).expect("the container opens");
    package
        .entries()
        .iter()
        .map(|entry| {
            (
                entry.name.clone(),
                entry
                    .bytes()
                    .expect("a freshly opened package holds every part's bytes")
                    .to_vec(),
            )
        })
        .collect()
}

/// Which entries differ between two containers.
fn differing(before: &[u8], after: &[u8]) -> Vec<String> {
    let before = payloads(before);
    let after = payloads(after);
    assert_eq!(
        before.iter().map(|it| &it.0).collect::<Vec<_>>(),
        after.iter().map(|it| &it.0).collect::<Vec<_>>(),
        "structural container identity: the same entries in the same order"
    );
    before
        .iter()
        .zip(after.iter())
        .filter(|(one, two)| one.1 != two.1)
        .map(|(one, _)| one.0.clone())
        .collect()
}

#[test]
fn a_session_that_edits_nothing_commits_a_container_identical_part_for_part() {
    // Tier 3 of `CONTRIBUTING.md`, through a session: residency changes nothing about what a save
    // writes for a part nobody touched.
    let deck = fixture("sample.pptx");
    let sink = MemoryDocument::new();
    Session::new(
        PresentationSession::open(&deck).expect("it opens"),
        ManualClock::new(),
        MemoryJournal::new(),
        sink.clone(),
    )
    .with_commit_policy(CommitPolicy::manual())
    .save()
    .expect("a save");
    assert!(
        differing(&deck, &sink.contents().expect("a container")).is_empty(),
        "a `.pptx` opened and committed changed a part nobody edited"
    );

    let document = fixture("sample.docx");
    let sink = MemoryDocument::new();
    Session::new(
        WordSession::open(&document).expect("it opens"),
        ManualClock::new(),
        MemoryJournal::new(),
        sink.clone(),
    )
    .with_commit_policy(CommitPolicy::manual())
    .save()
    .expect("a save");
    assert!(
        differing(&document, &sink.contents().expect("a container")).is_empty(),
        "a `.docx` opened and committed changed a part nobody edited"
    );

    let workbook = fixture("sample.xlsx");
    let sink = MemoryDocument::new();
    Session::new(
        SpreadsheetSession::open(&workbook).expect("it opens"),
        ManualClock::new(),
        MemoryJournal::new(),
        sink.clone(),
    )
    .with_commit_policy(CommitPolicy::manual())
    .save()
    .expect("a save");
    assert!(
        differing(&workbook, &sink.contents().expect("a container")).is_empty(),
        "an `.xlsx` opened and committed changed a part nobody edited"
    );
}

#[test]
fn an_edited_slide_is_the_only_part_a_commit_changes() {
    let bytes = fixture("sample.pptx");
    let mut deck = Presentation::open(&bytes).expect("it opens");
    let address = first_run_in_deck(&mut deck).expect("a run with text");
    let committed = MemoryDocument::new();
    let mut session = Session::new(
        PresentationSession::new(deck),
        ManualClock::new(),
        MemoryJournal::new(),
        committed.clone(),
    )
    .with_commit_policy(CommitPolicy::manual());

    for round in 0..5 {
        session
            .edit(Operation::set_value(
                address.clone(),
                Value::text(format!("edited {round}")),
            ))
            .expect("an edit");
        session.clock().advance(40);
    }
    session.save().expect("a save");

    let written = committed.contents().expect("a committed container");
    let changed = differing(&bytes, &written);
    assert_eq!(
        changed.len(),
        1,
        "five edits to one run changed these parts: {changed:?}"
    );
    assert!(
        changed[0].contains("slide"),
        "the part that changed is the slide, not {}",
        changed[0]
    );

    // And the edit is really there — a byte-identity gate that passed because nothing was written
    // would be the worst possible pass.
    let mut reopened = Presentation::open(&written).expect("the committed deck opens");
    let segments = address.path().segments();
    let shape: Vec<usize> = segments[1..segments.len() - 2]
        .iter()
        .map(|&it| it as usize)
        .collect();
    assert_eq!(
        reopened
            .run_text(
                segments[0] as usize,
                shape,
                segments[segments.len() - 2] as usize,
                segments[segments.len() - 1] as usize
            )
            .expect("the run reads back"),
        "edited 4"
    );
}

#[test]
fn an_edited_word_run_is_the_only_part_a_commit_changes() {
    let bytes = fixture("sample.docx");
    let mut document = mjx_docx::Document::open(&bytes).expect("it opens");
    let address = first_run_in_document(&mut document).expect("a run with text");
    let committed = MemoryDocument::new();
    let mut session = Session::new(
        WordSession::new(document),
        ManualClock::new(),
        MemoryJournal::new(),
        committed.clone(),
    )
    .with_commit_policy(CommitPolicy::manual());

    session
        .edit(Operation::set_value(address, Value::text("edited")))
        .expect("an edit");
    session.save().expect("a save");

    let written = committed.contents().expect("a committed container");
    let changed = differing(&bytes, &written);
    assert_eq!(changed, vec!["word/document.xml".to_owned()]);
}

#[test]
fn an_edited_worksheet_is_the_only_part_a_commit_changes() {
    let bytes = fixture("sample.xlsx");
    let committed = MemoryDocument::new();
    let mut session = Session::new(
        SpreadsheetSession::open(&bytes).expect("it opens"),
        ManualClock::new(),
        MemoryJournal::new(),
        committed.clone(),
    )
    .with_commit_policy(CommitPolicy::manual());

    for row in 200..210_u32 {
        session
            .edit(Operation::set_value(
                cell_address(0, row, 1),
                Value::Number(f64::from(row) / 4.0),
            ))
            .expect("an edit");
    }
    session.save().expect("a save");

    let written = committed.contents().expect("a committed container");
    let changed = differing(&bytes, &written);
    assert_eq!(
        changed.len(),
        1,
        "ten cell edits on one sheet changed these parts: {changed:?}"
    );
    assert!(changed[0].contains("sheet"), "changed {}", changed[0]);
}

#[test]
fn an_undo_puts_a_slide_run_back_to_the_bytes_it_arrived_with() {
    // The strongest statement of exactness this vocabulary can make: edit, undo, commit, and the
    // container is what it was.
    let bytes = fixture("sample.pptx");
    let mut deck = Presentation::open(&bytes).expect("it opens");
    let address = first_run_in_deck(&mut deck).expect("a run with text");
    let committed = MemoryDocument::new();
    let mut session = Session::new(
        PresentationSession::new(deck),
        ManualClock::new(),
        MemoryJournal::new(),
        committed.clone(),
    )
    .with_commit_policy(CommitPolicy::manual());

    session
        .edit(Operation::set_value(address, Value::text("something else")))
        .expect("an edit");
    session.undo().expect("an undo").expect("a unit");
    session.save().expect("a save");

    let written = committed.contents().expect("a committed container");
    let changed = differing(&bytes, &written);
    assert!(
        changed.is_empty(),
        "an edit and its undo left these parts different: {changed:?}"
    );
}

#[test]
fn a_cell_that_cannot_be_put_back_exactly_is_refused_rather_than_approximated() {
    // `shared_strings_rich_text.xlsx` is in the corpus precisely because its strings are rich. A
    // residency that edited one of those would restore plain text over formatting on the first undo,
    // which is this library authoring over a user's document.
    let bytes = fixture("formulas.xlsx");
    let mut session = SpreadsheetSession::open(&bytes).expect("it opens");
    let mut refusals = 0;
    let mut edits = 0;
    for row in 0..12_u32 {
        for column in 0..6_u32 {
            match session.cell_value(0, row, u16::try_from(column).expect("a small column")) {
                Ok(_) => edits += 1,
                Err(mjx_session::SessionError::NoExactInverse { .. }) => refusals += 1,
                Err(other) => panic!("unexpected refusal: {other}"),
            }
        }
    }
    assert!(
        refusals > 0,
        "a workbook of formulas should have cells this vocabulary refuses; {edits} were readable \
         and none were refused, which means the exactness check never ran"
    );
}

#[test]
fn a_run_in_a_later_paragraph_is_the_run_that_changes() {
    // The identity-value trap, named. `mjx-pptx` counts runs **per paragraph** when reading and
    // **flattened over the whole shape** when writing; `PresentationSession` converts between the
    // two, and for paragraph zero that conversion is the identity. Every other gate in this crate
    // addresses `sample.pptx`'s only run, which is paragraph zero's, so all of them would pass with
    // the conversion deleted — and the wrong run would be silently rewritten the first time a real
    // editor touched the second line of a bullet list.
    //
    // `text_levels.pptx` is the one fixture in the corpus with a shape of five paragraphs.
    //
    // Proved by mutation: replacing `flattened_run_index`'s body with a bare `run` — the value
    // it has for paragraph zero — turns this red (*"the edit did not land in paragraph 1"*) and
    // leaves every other test in this crate green.
    let bytes = fixture("text_levels.pptx");
    let mut deck = Presentation::open(&bytes).expect("it opens");
    let address = first_run_after_the_first_paragraph(&mut deck)
        .expect("`text_levels.pptx` has a shape whose later paragraphs carry runs");
    let segments: Vec<u32> = address.path().segments().to_vec();
    let (slide, shape, paragraph, run) = (
        segments[0] as usize,
        segments[1] as usize,
        segments[2] as usize,
        segments[3] as usize,
    );
    assert!(paragraph >= 1, "the probe found paragraph {paragraph}");

    let count = deck
        .paragraph_count(slide, shape)
        .expect("the paragraph count");
    let before: Vec<String> = (0..count)
        .map(|index| {
            deck.paragraph_text(slide, shape, index)
                .expect("a paragraph reads")
        })
        .collect();

    let committed = MemoryDocument::new();
    let mut session = Session::new(
        PresentationSession::new(deck),
        ManualClock::new(),
        MemoryJournal::new(),
        committed.clone(),
    )
    .with_commit_policy(CommitPolicy::manual());
    session
        .edit(Operation::set_value(
            address,
            Value::text("REWRITTEN BY THE SESSION"),
        ))
        .expect("an edit");
    session.save().expect("a save");

    let mut reopened =
        Presentation::open(&committed.contents().expect("a container")).expect("it reopens");
    let after: Vec<String> = (0..reopened
        .paragraph_count(slide, shape)
        .expect("the paragraph count"))
        .map(|index| {
            reopened
                .paragraph_text(slide, shape, index)
                .expect("a paragraph reads")
        })
        .collect();

    assert_eq!(
        before.len(),
        after.len(),
        "the edit changed how many paragraphs the shape has"
    );
    assert_eq!(
        reopened
            .run_text(slide, shape, paragraph, run)
            .expect("the run reads back"),
        "REWRITTEN BY THE SESSION",
        "the edit did not land in paragraph {paragraph}"
    );
    for (index, (was, is)) in before.iter().zip(after.iter()).enumerate() {
        if index == paragraph {
            continue;
        }
        assert_eq!(
            was, is,
            "paragraph {index} changed and nobody edited it — the flattened run index is wrong"
        );
    }
}
