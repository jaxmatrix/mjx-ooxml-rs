//! **Endnotes are flow, not an area** — and `w:pos` decides where in the flow.
//!
//! # The one-sentence difference between an endnote and a footnote
//!
//! A footnote competes with the body for the space on *its own* page, which is why
//! [`mjx_layout_docx::notes`] exists and why it is a fixed point. An endnote does not compete with
//! anything: §17.11.3's `sectEnd`/`docEnd` are positions in the **flow**, so an endnote's content is
//! laid out as ordinary body content at the end of its scope, on the sheet that scope uses.
//!
//! That is why this suite asserts on **paragraph count and page count** rather than on a note area:
//! an endnote adds content to the document, and content is the thing that moves.
//!
//! # The trap
//!
//! `docEnd` is what an absent `w:pos` means, and it is also what a build that ignored `w:pos`
//! entirely would produce. So the two positions are asserted **against each other**, on a document
//! whose two sections use different sheets — where `sectEnd` puts the first section's notes on the
//! first section's paper and `docEnd` puts them all on the last section's.

mod support;

use mjx_docx::Document;
use mjx_layout::{BoxModel, PageIndex};
use mjx_layout_docx::{FlowOrigin, PageReport};
use mjx_ooxml_core::measure::Emu;
use support::{constraints, flow, model};

/// One `w:endnote` of `lines` paragraphs — [`support::note_entry`], named for this suite.
fn note(id: i64, kind: Option<&str>, lines: &[&str]) -> String {
    support::note_entry("endnote", id, kind, lines)
}

/// A document of `paragraphs` whose `word/endnotes.xml` holds `notes`.
fn document_with_endnotes(paragraphs: &[String], section: &str, notes: &[String]) -> Document {
    support::document_with_notes(paragraphs, section, false, notes)
}

/// A paragraph whose single run carries an endnote reference after its text.
fn referencing(id: i64, text: &str) -> String {
    support::referencing_note(false, id, text)
}

/// A sheet that holds a handful of lines.
fn small_page() -> String {
    support::page_geometry(6.5, 1.5, 0.25)
}

/// Every page's report.
fn reports(mut document: Document, limit: usize) -> Vec<PageReport> {
    let flow = flow(&mut document);
    let mut model = model();
    let area = constraints(6.5, 1.5);
    let mut out = Vec::new();
    let mut resume = None;
    for number in 0..limit {
        let Ok(page) = model.layout_page(
            &flow,
            PageIndex::new(u32::try_from(number).expect("a page number")),
            &area,
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

/// An endnote's paragraphs really are in the flow, and are reported as coming from the endnotes
/// part rather than from the body.
#[test]
fn an_endnote_becomes_flow_and_says_where_it_came_from() {
    let notes = vec![note(2, None, &["The endnote's own text."])];
    let paragraphs = vec![
        referencing(2, "Body paragraph zero."),
        support::paragraph("", "Body paragraph one."),
    ];
    let mut document = document_with_endnotes(&paragraphs, &small_page(), &notes);
    let read = flow(&mut document);
    assert_eq!(read.paragraphs().len(), 2, "the body has two paragraphs");
    assert_eq!(
        read.paragraph_count(),
        3,
        "and the flow has three: the endnote's is spliced in"
    );
    assert_eq!(read.origin(0), Some(FlowOrigin::Body(0)));
    assert_eq!(read.origin(1), Some(FlowOrigin::Body(1)));
    assert_eq!(
        read.origin(2),
        Some(FlowOrigin::Endnote {
            note: 2,
            paragraph: 0
        }),
        "the third came from `word/endnotes.xml`, and its index there is the entry's — the two \
         reserved separators are entries 0 and 1"
    );
}

/// An endnote that is never referenced is never flowed.
#[test]
fn an_unreferenced_endnote_is_not_laid_out() {
    let notes = vec![
        note(2, None, &["Referenced."]),
        note(3, None, &["Never referenced."]),
    ];
    let paragraphs = vec![referencing(2, "Body paragraph zero.")];
    let mut document = document_with_endnotes(&paragraphs, &small_page(), &notes);
    let read = flow(&mut document);
    assert_eq!(
        read.paragraph_count(),
        2,
        "one body paragraph and one endnote, not two endnotes"
    );
}

/// **`sectEnd` and `docEnd` put the notes on different sheets.** The pair is the assertion: a build
/// that ignored `w:pos` produces the `docEnd` answer for both.
///
/// The page *count* is the same either way — the same content is in the document — so what is
/// asserted is the **width of the sheet the endnote lands on**, which is the narrow section's under
/// `sectEnd` and the wide one's under `docEnd`.
#[test]
fn sect_end_and_doc_end_put_the_notes_on_different_sheets() {
    let narrow = support::page_geometry(3.0, 1.5, 0.25);
    let wide = support::page_geometry(6.5, 1.5, 0.25);
    let notes = vec![note(2, None, &["The first section's endnote."])];
    let of = |position: &str| {
        let paragraphs = vec![
            referencing(2, "Body paragraph zero."),
            support::paragraph(
                &format!(
                    r#"<w:sectPr>{narrow}<w:endnotePr><w:pos w:val="{position}"/></w:endnotePr></w:sectPr>"#
                ),
                "End of the narrow section.",
            ),
            support::paragraph("", "A paragraph of the wide section."),
        ];
        document_with_endnotes(&paragraphs, &wide, &notes)
    };

    assert_eq!(
        sheet_of_the_endnote(of("sectEnd")),
        Emu::from_inches(2.5),
        "`sectEnd` lays the note out on its own section's paper, before the wide section starts"
    );
    assert_eq!(
        sheet_of_the_endnote(of("docEnd")),
        Emu::from_inches(6.0),
        "and `docEnd` waits for the end of the document, which is the wide section's paper"
    );
}

/// The text-area width of the page the document's one endnote landed on.
///
/// The page is found by **part**, not by paragraph index: an endnote's fragments are addressed in
/// `word/endnotes.xml`'s own space (`[note, paragraph, …]`), which is exactly the separation
/// `crate::address` exists to make — a header's third paragraph and the body's third paragraph must
/// not sort together, and neither must an endnote's.
fn sheet_of_the_endnote(mut document: Document) -> Emu {
    let read = flow(&mut document);
    assert!(
        (0..read.paragraph_count())
            .any(|index| matches!(read.origin(index), Some(FlowOrigin::Endnote { .. }))),
        "the endnote is in the flow"
    );
    let mut model = model();
    let area = constraints(6.5, 1.5);
    let mut resume = None;
    for number in 0..40 {
        let Ok(page) = model.layout_page(&read, PageIndex::new(number), &area, resume.as_ref())
        else {
            break;
        };
        resume = page.continuation().cloned();
        let width = model.last_page().body.width();
        let carries = page
            .fragments()
            .nodes()
            .any(|(_, node)| node.source().part() == mjx_layout_docx::address::ENDNOTES);
        if carries {
            return width;
        }
        if resume.is_none() {
            break;
        }
    }
    panic!("the endnote must be laid out somewhere");
}

/// An endnote is not a footnote: it takes no note area and does not shrink the page it is
/// referenced from.
#[test]
fn an_endnote_takes_no_note_area() {
    let notes = vec![note(2, None, &["The endnote."])];
    let paragraphs = vec![
        referencing(2, "Body paragraph zero."),
        support::paragraph("", "Body paragraph one."),
    ];
    let pages = reports(
        document_with_endnotes(&paragraphs, &small_page(), &notes),
        20,
    );
    assert!(
        pages
            .iter()
            .all(|page| page.note_area_height == Emu::ZERO && page.notes.is_empty()),
        "an endnote never opens a footnote area: {pages:?}"
    );
}
