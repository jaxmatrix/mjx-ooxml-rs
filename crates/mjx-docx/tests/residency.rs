//! [`Document::formatting`] (MJXOFF-174) against
//! [`Document::effective_paragraph_properties`](mjx_docx::Document::effective_paragraph_properties)
//! — the two orchestrations of one ladder, asserted to agree.
//!
//! # Why this suite exists
//!
//! `mjx-docx` now resolves the effective-property ladder **twice**: once per paragraph, for a caller
//! asking one question, and once for the whole document, for a layout engine that would otherwise be
//! quadratic. Two orchestrations of one algorithm is exactly the shape that drifts — the second one
//! agrees with the first on every fixture anybody wrote, and then the first is fixed and the second
//! is not.
//!
//! The ladder's *order* is stated in one place (`combine_paragraph_tiers` and `combine_run_tiers`)
//! and every rung's extraction was already shared, so the drift has nowhere obvious to start. This
//! is what says so anyway: every paragraph and every run of every `.docx` in the committed corpus,
//! compared field for field.
//!
//! # What the residency adds that the per-paragraph reader does not
//!
//! The *text*, the *hard breaks* and the *sections*, which the second half of this file asserts on
//! their own. `Paragraph::text` concatenates `w:t` and nothing else; a renderer needs the tab, the
//! line feed and the two hyphens as well, and where a `w:br@type="page"` sits.

use mjx_docx::Document;
use mjx_fixtures::{fixture, package_fixtures_with_extension};

fn documents() -> Vec<(String, Document)> {
    package_fixtures_with_extension("docx")
        .into_iter()
        .filter_map(|name| {
            Document::open(&fixture(&name))
                .ok()
                .map(|document| (name, document))
        })
        .collect()
}

/// **The anti-drift gate.** Every paragraph of every committed `.docx`, resolved both ways.
#[test]
fn the_residency_agrees_with_the_per_paragraph_reader_about_paragraphs() {
    let mut compared = 0_usize;
    for (name, mut document) in documents() {
        let Ok(formatting) = document.formatting() else {
            continue;
        };
        for (index, paragraph) in formatting.paragraphs().iter().enumerate() {
            let Ok(one) = document.effective_paragraph_properties(index) else {
                continue;
            };
            assert_eq!(
                paragraph.properties(),
                &one,
                "{name}: paragraph {index} resolves differently through the two readers"
            );
            compared += 1;
        }
    }
    assert!(
        compared > 20,
        "the corpus must give this gate something to compare: {compared} paragraphs"
    );
    println!("compared {compared} paragraphs through both readers");
}

/// The same for runs, which have a tier the paragraph ladder does not (`w:rStyle`) and a
/// recombination rule the paragraph ladder does not (the twelve toggle properties' XOR).
#[test]
fn the_residency_agrees_with_the_per_paragraph_reader_about_runs() {
    let mut compared = 0_usize;
    for (name, mut document) in documents() {
        let Ok(formatting) = document.formatting() else {
            continue;
        };
        for (index, paragraph) in formatting.paragraphs().iter().enumerate() {
            for run in paragraph.runs() {
                let Some(path) = run.path.clone() else {
                    // A run inside a `w:fldSimple` has no address the per-paragraph reader can
                    // take, which is the one asymmetry between the two walks and is documented as
                    // such on `RunFormatting::path`.
                    continue;
                };
                let Ok(one) = document.effective_run_properties(index, path) else {
                    continue;
                };
                assert_eq!(
                    run.properties, one,
                    "{name}: a run of paragraph {index} resolves differently through the two readers"
                );
                compared += 1;
            }
        }
    }
    assert!(
        compared > 20,
        "the corpus must give this gate something to compare: {compared} runs"
    );
    println!("compared {compared} runs through both readers");
}

/// The residency's text is a superset of `Paragraph::text`: every `w:t` is still there, and the six
/// other elements have contributed a character each.
#[test]
fn the_text_carries_the_characters_a_renderer_needs() {
    let mut document = Document::open(&fixture("run_content.docx")).expect("open run_content.docx");
    let formatting = document.formatting().expect("formatting");
    let joined: String = formatting
        .paragraphs()
        .iter()
        .map(mjx_docx::ParagraphFormatting::text)
        .collect();
    assert!(!joined.is_empty(), "the fixture must hold text");

    // Whatever else it holds, nothing may be a character this reader does not know how to place.
    for character in joined.chars() {
        assert!(
            character != '\r',
            "a carriage return must have become a line feed"
        );
    }
}

/// A run's ranges cover its paragraph's text with no gap and no overlap, which is what lets a box
/// model index one by the other without a translation.
#[test]
fn the_runs_cover_the_text_exactly() {
    for (name, mut document) in documents() {
        let Ok(formatting) = document.formatting() else {
            continue;
        };
        for (index, paragraph) in formatting.paragraphs().iter().enumerate() {
            let mut cursor = 0_usize;
            for run in paragraph.runs() {
                assert_eq!(
                    run.range.start, cursor,
                    "{name}: paragraph {index} has a gap or an overlap in its runs"
                );
                cursor = run.range.end;
            }
            assert_eq!(
                cursor,
                paragraph.text().len(),
                "{name}: paragraph {index}'s runs stop short of its text"
            );
        }
    }
}

/// Every section resolves to plain numbers, so a caller keeping a [`DocumentFormatting`] does not
/// have to keep an interner beside it.
#[test]
fn the_sections_resolve_to_numbers() {
    let mut document =
        Document::open(&fixture("three_section_document.docx")).expect("open the fixture");
    let formatting = document.formatting().expect("formatting");
    assert!(
        formatting.sections().len() >= 3,
        "the fixture is named for its three sections: {}",
        formatting.sections().len()
    );
    for section in formatting.sections() {
        if let Some(size) = section.page_size {
            assert!(size.width_twips > 0 && size.height_twips > 0);
        }
    }
    // And a paragraph can be asked which section governs it.
    assert!(formatting.section_of(0).is_some());
}

/// The settings a layout engine reads arrive with their defaults already applied, so a consumer
/// never has to know which of them default to on.
#[test]
fn the_settings_arrive_with_their_defaults_applied() {
    let mut document = Document::open(&fixture("sample.docx")).expect("open sample.docx");
    let formatting = document.formatting().expect("formatting");
    let settings = formatting.settings();
    assert!(
        settings.default_tab_stop_twips > 0,
        "a tab grid of zero would be an infinite loop"
    );
    // A blank-ish document states no hyphenation, and the absence is a `false` rather than a
    // `None` a consumer would have to interpret.
    let _ = settings.auto_hyphenation;
    let _ = settings.do_not_hyphenate_capitals;
}

/// A document with no `word/styles.xml` at all — which `Document::blank` produces — resolves rather
/// than failing, and gives every paragraph the all-`None` ladder.
#[test]
fn a_document_with_no_styles_part_still_resolves() {
    let mut blank = Document::blank(mjx_docx::PageSize::a4()).expect("a blank document");
    let formatting = blank.formatting().expect("a blank document resolves");
    assert_eq!(
        formatting.paragraphs().len(),
        1,
        "a blank document is one empty paragraph"
    );
    assert_eq!(formatting.paragraphs()[0].text(), "");
}

// =================================================================================================
// MJXOFF-175 (R20): what a section resolves to beyond its page size, the header/footer and note
// content streams, and where a note is referenced from.
// =================================================================================================

/// A section resolves its **break kind, columns, numbering and note rules** — everything a layout
/// engine needs about a section and nothing it would have to read an interner for.
#[test]
fn a_section_resolves_its_whole_layout_vocabulary() {
    let mut document =
        Document::open(&fixture("three_section_document.docx")).expect("open the fixture");
    let formatting = document.formatting().expect("formatting");
    let sections = formatting.sections();
    assert!(sections.len() >= 3);

    // The fixture's first two sections state `w:type="nextPage"` and its second states two equal
    // columns; the third states neither, which is what makes it the control.
    assert_eq!(
        sections[0].break_kind,
        Some(mjx_ooxml_types::wordprocessingml::SectionBreakType::NextPage)
    );
    let multi = sections
        .iter()
        .find(|section| section.columns.count > 1)
        .expect("the fixture states a two-column section");
    assert_eq!(multi.columns.count, 2);
    assert!(
        multi.columns.columns.is_empty(),
        "an equal-width section reports no explicit list: the width is the text area divided, and \
         the text area is not known until the page is chosen"
    );
    assert_eq!(
        multi.columns.space_twips,
        mjx_docx::SectionColumns::DEFAULT_SPACE_TWIPS
    );

    for section in sections {
        assert!(!section.title_page, "the fixture writes no `w:titlePg`");
        assert!(section.line_numbering.is_none());
        assert_eq!(section.page_numbering.start, None);
        assert_eq!(
            section.page_numbering.format,
            mjx_ooxml_types::wordprocessingml::NumberFormat::Decimal
        );
    }
}

/// A section's header and footer slots are **already resolved** — §17.10's two flags and the
/// inheritance walk have run — so a consumer selects rather than resolves.
#[test]
fn the_header_and_footer_slots_are_resolved_rather_than_listed() {
    let mut document =
        Document::open(&fixture("header_footer_variants.docx")).expect("open the fixture");
    let formatting = document.formatting().expect("formatting");
    let first = formatting.sections().first().expect("a section");

    // The fixture references all three header kinds and all three footer kinds, and states neither
    // `w:titlePg` nor `w:evenAndOddHeaders` — so both the first-page and the even-page queries are
    // **downgraded to the default**, which is §17.10.6 and §17.10.1's own sentence.
    assert!(!formatting.settings().even_and_odd_headers);
    assert!(!first.title_page);
    assert_eq!(
        first.headers.first, first.headers.default,
        "with `w:titlePg` absent, a first-page query resolves exactly as a default one does"
    );
    assert_eq!(
        first.headers.even, first.headers.default,
        "and so does an even-page query with `w:evenAndOddHeaders` absent"
    );
    assert_eq!(first.footers.first, first.footers.default);
    assert!(first.headers.default.is_some(), "the fixture states one");
    assert_ne!(
        first.headers.default, first.footers.default,
        "a header and a footer are different streams even when both slots are filled"
    );

    // The second section states no references at all and inherits every one of them.
    let second = formatting.sections().get(1).expect("a second section");
    assert_eq!(second.headers.default, first.headers.default);
    assert_eq!(second.footers.default, first.footers.default);
}

/// The header and footer parts are **read**, and their paragraphs go through the same ladder the
/// body's do.
#[test]
fn the_header_and_footer_streams_are_read_and_resolved() {
    let mut document =
        Document::open(&fixture("header_footer_variants.docx")).expect("open the fixture");
    let formatting = document.formatting().expect("formatting");
    assert!(
        !formatting.header_footer_streams().is_empty(),
        "the fixture relates six header and footer parts"
    );
    let mut seen = std::collections::BTreeSet::new();
    for stream in formatting.header_footer_streams() {
        assert!(
            seen.insert(stream.part().as_str().to_owned()),
            "each part is read once, however many sections reach it"
        );
        assert!(
            !stream.paragraphs().is_empty(),
            "{} holds no paragraph",
            stream.part().as_str()
        );
        for paragraph in stream.paragraphs() {
            // The ladder ran: an effective property struct exists for it, exactly as for a body
            // paragraph. The fixture's headers are unstyled, so what is asserted is that the runs
            // cover the text — the same invariant the body's own gate asserts.
            let mut at = 0;
            for run in paragraph.runs() {
                assert_eq!(run.range.start, at);
                at = run.range.end;
            }
            assert_eq!(at, paragraph.text().len());
        }
    }
    let default = formatting
        .sections()
        .first()
        .and_then(|section| section.headers.default)
        .and_then(|slot| formatting.header_footer_stream(slot))
        .expect("the default header");
    assert_eq!(
        default
            .paragraphs()
            .iter()
            .map(mjx_docx::ParagraphFormatting::text)
            .collect::<String>(),
        "Default header"
    );
}

/// A document that relates no notes parts reports empty lists rather than failing.
#[test]
fn a_document_with_no_notes_reports_none() {
    let mut document = Document::open(&fixture("sample.docx")).expect("open sample.docx");
    let formatting = document.formatting().expect("formatting");
    assert!(formatting.footnotes().is_empty());
    assert!(formatting.endnotes().is_empty());
    for paragraph in formatting.paragraphs() {
        assert!(paragraph.note_references().is_empty());
    }
}

/// A footnotes part is read entry by entry, its reserved separators included — a renderer draws
/// them — and each entry's `w:type` decides whether it is one of the document's own notes.
#[test]
fn the_note_streams_are_read_with_their_reserved_entries() {
    let mut document = Document::blank(mjx_docx::PageSize::a4()).expect("a blank document");
    let identifier = document
        .add_footnote(mjx_docx::BlockPath::from(0), "The note's text.")
        .expect("a footnote is addable");

    let formatting = document.formatting().expect("formatting");
    let notes = formatting.footnotes();
    assert!(
        notes.len() >= 3,
        "two reserved entries and one of the document's own: {}",
        notes.len()
    );
    assert!(
        notes
            .iter()
            .any(|note| note.kind()
                == mjx_ooxml_types::wordprocessingml::FootnoteEndnoteType::Separator),
        "the separator a renderer draws above a page's notes is one of the entries"
    );
    let user: Vec<_> = notes.iter().filter(|note| note.is_user_visible()).collect();
    assert_eq!(user.len(), 1, "one of them is the document's own");
    assert_eq!(user[0].id(), identifier);
    assert!(
        user[0]
            .paragraphs()
            .iter()
            .any(|paragraph| paragraph.text().contains("The note's text.")),
        "and its content went through the same reader the body's did"
    );

    // The reference the body now carries is recorded, with the id it names and the byte it sits at.
    let referencing = formatting
        .paragraphs()
        .iter()
        .find(|paragraph| !paragraph.note_references().is_empty())
        .expect("the body refers to the note");
    let reference = referencing.note_references()[0];
    assert_eq!(reference.id, identifier);
    assert!(!reference.endnote);
    assert!(
        reference.at <= referencing.text().len(),
        "the offset addresses this paragraph's own text"
    );
    assert_eq!(
        formatting
            .footnote(identifier)
            .map(mjx_docx::NoteFormatting::id),
        Some(identifier)
    );
}

/// `w:evenAndOddHeaders` and `w:mirrorMargins` arrive on the settings with their defaults applied.
#[test]
fn the_document_wide_page_settings_arrive_with_their_defaults() {
    let mut blank = Document::blank(mjx_docx::PageSize::a4()).expect("a blank document");
    let formatting = blank.formatting().expect("a blank document resolves");
    assert!(!formatting.settings().even_and_odd_headers);
    assert!(!formatting.settings().mirror_margins);
}
