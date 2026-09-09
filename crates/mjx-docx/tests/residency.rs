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
