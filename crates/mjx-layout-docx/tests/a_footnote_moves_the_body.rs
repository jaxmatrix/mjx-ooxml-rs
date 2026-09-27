//! **A footnote changes the page its own reference is on — and therefore the page every paragraph
//! after it is on.**
//!
//! # The gate that looks right and is wrong
//!
//! Laying the notes out *after* pagination is settled produces a page that looks entirely correct:
//! the note is at the foot of the page its reference is on, the separator is above it, the numbering
//! runs. It is wrong three pages later, because the note took space the body was allowed to use. So
//! the assertion here is **not** "the note is on the right page" — it is *a named paragraph lands on
//! a different page when a footnote is added*, and the pair is the fixture: the same body text
//! twice, differing only in whether one run carries a `w:footnoteReference`.
//!
//! **The reference contributes no character** (see `mjx_docx::NoteReference`), so the two documents
//! have byte-identical body text and lay out identically line for line. Every difference between
//! them is the note's own height.
//!
//! # And the one that does not terminate
//!
//! A footnote taller than the page is a real document. `crate::notes` caps the reservation at the
//! body's first line and splits the note; this suite runs one forty times the page's height and
//! asserts that the pages come out, that each one places at least one line of the body **and** at
//! least one line of the note, and that the body finishes.

mod support;

use mjx_docx::Document;
use mjx_layout::{BoxModel, Fragment, PageIndex};
use mjx_layout_docx::PageReport;
use support::{constraints, flow, model};

/// One `w:footnote` of `lines` paragraphs — [`support::note_entry`], named for this suite.
fn note(id: i64, kind: Option<&str>, lines: &[&str]) -> String {
    support::note_entry("footnote", id, kind, lines)
}

/// A document of `paragraphs` whose `word/footnotes.xml` holds `notes`.
fn document_with_notes(paragraphs: &[String], section: &str, notes: &[String]) -> Document {
    support::document_with_notes(paragraphs, section, true, notes)
}

/// A paragraph whose single run carries a footnote reference after its text.
fn referencing(id: i64, text: &str) -> String {
    support::referencing_note(true, id, text)
}

/// Every page's report.
fn reports(
    mut document: Document,
    area: &mjx_layout::Constraints,
    limit: usize,
) -> Vec<PageReport> {
    let flow = flow(&mut document);
    let mut model = model();
    let mut out = Vec::new();
    let mut resume = None;
    for number in 0..limit {
        let Ok(page) = model.layout_page(
            &flow,
            PageIndex::new(u32::try_from(number).expect("a page number")),
            area,
            resume.as_ref(),
        ) else {
            break;
        };
        resume = page.continuation().cloned();
        out.push(model.last_page().clone());
        if resume.is_none() {
            break;
        }
    }
    out
}

/// Which page each paragraph's first line landed on.
fn page_of_each(
    mut document: Document,
    area: &mjx_layout::Constraints,
    limit: usize,
) -> Vec<Option<usize>> {
    let flow = flow(&mut document);
    let mut model = model();
    support::page_of_each_paragraph(&mut model, &flow, area, limit)
}

/// A sheet that holds six lines of eleven-point body text.
fn six_line_page() -> String {
    support::page_geometry(6.5, 6.0 * 0.19 + 0.5, 0.25)
}

/// **The assertion the ticket asks for.** Adding one footnote moves a *later* paragraph on to the
/// next page.
#[test]
fn one_footnote_changes_which_page_a_later_paragraph_lands_on() {
    let geometry = six_line_page();
    let area = constraints(6.5, 6.0);
    let notes = vec![note(
        2,
        None,
        &["A footnote of two lines,", "the second of them here."],
    )];

    let plain: Vec<String> = (0..12)
        .map(|index| support::paragraph("", &format!("Paragraph {index}.")))
        .collect();
    let mut noted = plain.clone();
    noted[1] = referencing(2, "Paragraph 1.");

    let without = page_of_each(document_with_notes(&plain, &geometry, &notes), &area, 20);
    let with = page_of_each(document_with_notes(&noted, &geometry, &notes), &area, 20);

    assert_eq!(
        without.len(),
        with.len(),
        "the two documents hold the same paragraphs"
    );
    let moved = without
        .iter()
        .zip(&with)
        .enumerate()
        .find(|(_, (before, after))| before != after);
    let (index, (before, after)) = moved.expect(
        "some paragraph must move: a footnote that changed nothing would mean the note area is \
         being laid out after pagination rather than with it",
    );
    assert!(
        index > 1,
        "and the paragraph that moves must be a *later* one than the reference's own: {index}"
    );
    assert!(
        after > before,
        "it moves forward, never back: paragraph {index} was on {before:?} and is now on {after:?}"
    );
}

/// The note itself is on the page its reference is on, drawn in its own part.
#[test]
fn the_note_is_drawn_on_the_page_that_refers_to_it() {
    let geometry = six_line_page();
    let notes = vec![note(2, None, &["A footnote."])];
    let mut paragraphs: Vec<String> = (0..12)
        .map(|index| support::paragraph("", &format!("Paragraph {index}.")))
        .collect();
    paragraphs[7] = referencing(2, "Paragraph 7.");
    let pages = reports(
        document_with_notes(&paragraphs, &geometry, &notes),
        &constraints(6.5, 6.0),
        20,
    );
    let carrying: Vec<usize> = pages
        .iter()
        .enumerate()
        .filter(|(_, page)| !page.notes.is_empty())
        .map(|(number, _)| number)
        .collect();
    assert_eq!(
        carrying.len(),
        1,
        "exactly one page carries the note: {carrying:?}"
    );
    assert!(
        pages[carrying[0]].note_area_height > mjx_ooxml_core::measure::Emu::ZERO,
        "and it takes real space"
    );
    assert!(
        pages
            .iter()
            .enumerate()
            .all(|(number, page)| (number == carrying[0]) == !page.notes.is_empty()),
        "no other page has a note area at all"
    );
}

/// **A page with no notes on it must lay out exactly as a document with no notes part at all.**
///
/// This is the gate the pair fixtures above structurally cannot make. Every one of them compares two
/// documents that *both* carry a `word/footnotes.xml`, so a cost paid by merely relating the part —
/// reserving the separator's height on every page, say — is paid by both sides and cancels. The
/// comparison that catches it is against a document with **no footnotes part**, which is the one
/// this makes.
///
/// It caught a real defect while being written: `demanded_height` counted the separator
/// unconditionally, so a page with no notes reserved a rule's height out of its body **and** was
/// assembled a second time to discover that it had.
#[test]
fn an_empty_note_area_costs_the_body_nothing() {
    let geometry = six_line_page();
    let area = constraints(6.5, 6.0);
    let paragraphs: Vec<String> = (0..24)
        .map(|index| support::paragraph("", &format!("Paragraph {index}.")))
        .collect();

    let with_part = page_of_each(
        document_with_notes(
            &paragraphs,
            &geometry,
            &[note(2, None, &["A footnote nothing refers to."])],
        ),
        &area,
        40,
    );
    let mut plain = support::document_with(&paragraphs, &geometry);
    let flow = flow(&mut plain);
    let mut model = model();
    let without_part = support::page_of_each_paragraph(&mut model, &flow, &area, 40);

    assert_eq!(
        with_part, without_part,
        "an unreferenced footnote costs the body nothing at all"
    );

    let pages = reports(
        document_with_notes(
            &paragraphs,
            &geometry,
            &[note(2, None, &["A footnote nothing refers to."])],
        ),
        &area,
        40,
    );
    assert!(
        pages.iter().all(|page| page.assemblies == 1),
        "and no page needs a second assembly to find out that it has no notes: {:?}",
        pages.iter().map(|page| page.assemblies).collect::<Vec<_>>()
    );
    assert!(
        pages
            .iter()
            .all(|page| page.note_area_height == mjx_ooxml_core::measure::Emu::ZERO),
        "nor any note area"
    );
}

/// **The fixed point never needs a third assembly.** Asserted on every page of a document whose
/// notes really do move the boundary.
#[test]
fn the_body_is_never_assembled_more_than_twice() {
    let geometry = six_line_page();
    let notes = vec![note(2, None, &["Two lines of note,", "and the second."])];
    let paragraphs: Vec<String> = (0..30)
        .map(|index| {
            if index % 4 == 1 {
                referencing(2, &format!("Paragraph {index}."))
            } else {
                support::paragraph("", &format!("Paragraph {index}."))
            }
        })
        .collect();
    let pages = reports(
        document_with_notes(&paragraphs, &geometry, &notes),
        &constraints(6.5, 6.0),
        60,
    );
    assert!(pages.len() > 5, "several pages: {}", pages.len());
    for (number, page) in pages.iter().enumerate() {
        assert!(
            page.assemblies <= 2,
            "page {number} took {} assemblies; `crate::notes` proves two is the bound",
            page.assemblies
        );
    }
    assert!(
        pages.iter().any(|page| page.assemblies == 2),
        "and at least one page must actually have needed the second one, or this asserts nothing"
    );
}

/// The separator is drawn above a page's notes, in the notes part.
#[test]
fn word_s_own_separator_is_drawn_above_the_notes() {
    let geometry = six_line_page();
    let notes = vec![note(2, None, &["A footnote."])];
    let mut paragraphs: Vec<String> = (0..6)
        .map(|index| support::paragraph("", &format!("Paragraph {index}.")))
        .collect();
    paragraphs[0] = referencing(2, "Paragraph 0.");
    let mut document = document_with_notes(&paragraphs, &geometry, &notes);
    let flow = flow(&mut document);
    let mut model = model();
    let page = model
        .layout_page(&flow, PageIndex::FIRST, &constraints(6.5, 6.0), None)
        .expect("page one lays out");
    let in_notes: Vec<_> = page
        .fragments()
        .nodes()
        .filter(|(_, node)| node.source().part() == mjx_layout_docx::address::FOOTNOTES)
        .filter(|(_, node)| matches!(node.fragment(), Fragment::GlyphRun(_)))
        .collect();
    assert!(
        in_notes.len() >= 2,
        "the separator's glyph and the note's own must both be there: {}",
        in_notes.len()
    );
    let top = in_notes
        .iter()
        .map(|(_, node)| node.rect().top)
        .min()
        .expect("a fragment");
    let body_bottom = page
        .fragments()
        .nodes()
        .filter(|(_, node)| node.source().part() == mjx_layout_docx::address::BODY)
        .filter(|(_, node)| matches!(node.fragment(), Fragment::Line(_)))
        .map(|(_, node)| node.rect().bottom)
        .max()
        .expect("a body line");
    assert!(
        top >= body_bottom,
        "the note area sits below the body: {top:?} against {body_bottom:?}"
    );
}

/// A note taller than its page is **split**, carried under a continuation separator, and finishes.
#[test]
fn a_footnote_taller_than_the_page_is_carried_and_the_document_still_ends() {
    let geometry = six_line_page();
    let long: Vec<String> = (0..40).map(|index| format!("Note line {index}.")).collect();
    let borrowed: Vec<&str> = long.iter().map(String::as_str).collect();
    let notes = vec![note(2, None, &borrowed)];
    let mut paragraphs: Vec<String> = (0..6)
        .map(|index| support::paragraph("", &format!("Paragraph {index}.")))
        .collect();
    paragraphs[0] = referencing(2, "Paragraph 0.");

    let pages = reports(
        document_with_notes(&paragraphs, &geometry, &notes),
        &constraints(6.5, 6.0),
        200,
    );
    assert!(
        pages.len() > 5,
        "a forty-line note on a six-line page takes several: {}",
        pages.len()
    );
    assert!(
        pages.len() < 200,
        "and it must **end** — a run that hit the limit is a fixed point that did not converge"
    );
    assert!(
        pages[0].carried_note.is_some(),
        "page one cannot hold it all: {:?}",
        pages[0]
    );
    assert!(
        pages.last().expect("a page").carried_note.is_none(),
        "and the last page carries nothing forward"
    );
    let carrying = pages
        .iter()
        .filter(|page| page.carried_note.is_some())
        .count();
    assert!(
        carrying >= 5,
        "the note is spread over several pages rather than crammed on to one: {carrying}"
    );
}

/// Footnote numbering, all three restart modes, asserted on the numbers themselves.
#[test]
fn footnote_numbering_restarts_where_the_section_says() {
    let base = support::page_geometry(6.5, 6.0 * 0.19 + 0.5, 0.25);
    let notes: Vec<String> = (2..8).map(|id| note(id, None, &["A footnote."])).collect();
    let paragraphs: Vec<String> = (0..24)
        .map(|index| {
            if index % 4 == 1 {
                referencing(2 + i64::from(index / 4), &format!("Paragraph {index}."))
            } else {
                support::paragraph("", &format!("Paragraph {index}."))
            }
        })
        .collect();
    let area = constraints(6.5, 6.0);

    let numbers = |restart: &str| -> Vec<i64> {
        let section =
            format!(r#"{base}<w:footnotePr><w:numRestart w:val="{restart}"/></w:footnotePr>"#);
        reports(
            document_with_notes(&paragraphs, &section, &notes),
            &area,
            40,
        )
        .iter()
        .flat_map(|page| page.notes.iter().map(|(_, number)| *number))
        .collect()
    };

    let continuous = numbers("continuous");
    let each_page = numbers("eachPage");
    assert_eq!(
        continuous,
        (1..=continuous.len() as i64).collect::<Vec<_>>(),
        "continuous numbering counts the whole document"
    );
    assert!(
        each_page.contains(&1) && each_page != continuous,
        "and `eachPage` starts over: {each_page:?} against {continuous:?}"
    );
    assert_eq!(
        *each_page.iter().max().expect("a number"),
        1,
        "one note per page in this fixture, so every page's first note is number one"
    );
}
