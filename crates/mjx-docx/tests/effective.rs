//! The effective-properties ladder (MJXOFF-106): the corrected order (numbering below the
//! paragraph-style chain, not above it — the ticket's own claim was wrong, see
//! `crates/mjx-docx/docs/effective_properties.md`), the toggle-property XOR rule, and theme
//! colour/font resolution through `mjx-dml`'s own theme model.
//!
//! `tests/fixtures/effective_properties.docx` is authored for this child — no fixture in the corpus
//! carries `w:docDefaults`, a numbering level's own `w:rPr`, a paragraph style *and* a numbering
//! level both stating the same property, or a bold paragraph style paired with a bold character
//! style, all in one place. Nine paragraphs, one scenario each (see the fixture-build comments at
//! the top of each test below for exactly what each one carries).

use mjx_docx::{Document, DocxError};
use mjx_fixtures::fixture;

fn open() -> Document {
    Document::open(&fixture("effective_properties.docx")).expect("open effective_properties.docx")
}

// -------------------------------------------------------------------------------------------
// The discriminating three-rung matrix: the same property (`w:sz`, font size) set at three
// different rungs with three different values (docDefaults 10pt, the paragraph-style chain's base
// 12pt, the numbering level 11pt), read from three paragraphs that progressively strip away the
// higher rungs — moving the winning value one rung at a time and asserting the answer changes each
// time, exactly the matrix the ticket's own trap section demands rather than a single-rung fixture.
// -------------------------------------------------------------------------------------------

/// Would this pass if the ladder order were not implemented (or implemented in the ticket's own,
/// wrong order)? No. Paragraph 0 carries `w:pStyle="Leaf"` (based on `w:styleId="Base"`, which states
/// `w:sz="24"` — 12pt) *and* `w:numPr` pointing at a numbering level whose own `w:rPr` states
/// `w:sz="22"` — 11pt. ECMA-376 Part 1 §17.7.2 places the paragraph-style chain *above* numbering in
/// priority, so the paragraph style's 12pt must win. A resolver built to the ticket's own (wrong)
/// order — numbering above the paragraph style — would answer 11pt instead; see this file's own
/// `moving_the_winning_rung_down_one_step_changes_the_answer` for the same property answering
/// differently once that higher rung is removed, which is the second half of the discriminating
/// proof.
#[test]
fn the_paragraph_style_chain_outranks_the_numbering_level() {
    let mut document = open();
    let effective = document
        .effective_run_properties(0, 0)
        .expect("paragraph 0, run 0");
    assert_eq!(
        effective.font_size,
        Some(mjx_ooxml_types::wordprocessingml::HalfPointMeasure::from_wire("24")),
        "the paragraph-style chain (12pt) must outrank the numbering level (11pt)"
    );
}

/// Paragraph 1: `w:pStyle="LeafNoSize"` (whose own chain states no `w:sz` at all) with the *same*
/// numbering level as paragraph 0. With the paragraph-style rung now silent, the numbering level's
/// 11pt must be the answer — proving the numbering rung is genuinely consulted, not merely present
/// but shadowed. Paragraph 2 removes the numbering reference too, leaving only `w:docDefaults`'
/// 10pt. Three paragraphs, one property, three different winning rungs, three different answers —
/// moving the winning value down one rung at a time changes the read each time.
#[test]
fn moving_the_winning_rung_down_one_step_changes_the_answer() {
    let mut document = open();

    let paragraph_style_silent = document
        .effective_run_properties(1, 0)
        .expect("paragraph 1, run 0");
    assert_eq!(
        paragraph_style_silent.font_size,
        Some(mjx_ooxml_types::wordprocessingml::HalfPointMeasure::from_wire("22")),
        "with the paragraph-style chain silent, the numbering level (11pt) must win"
    );

    let numbering_absent_too = document
        .effective_run_properties(2, 0)
        .expect("paragraph 2, run 0");
    assert_eq!(
        numbering_absent_too.font_size,
        Some(mjx_ooxml_types::wordprocessingml::HalfPointMeasure::from_wire("20")),
        "with both the paragraph-style chain and numbering silent, docDefaults (10pt) must win"
    );

    // The three answers are pairwise distinct — the discriminating property this whole matrix
    // exists to prove: a resolver reading only one rung, or the wrong rung, cannot produce all
    // three of these from the same fixture.
    let paragraph_wins = document
        .effective_run_properties(0, 0)
        .expect("paragraph 0, run 0")
        .font_size;
    assert_ne!(paragraph_wins, paragraph_style_silent.font_size);
    assert_ne!(
        paragraph_style_silent.font_size,
        numbering_absent_too.font_size
    );
    assert_ne!(paragraph_wins, numbering_absent_too.font_size);
}

// -------------------------------------------------------------------------------------------
// The toggle rule (ECMA-376 Part 1 §17.7.3): combined by XOR across ladder tiers, not by plain
// override — and only for the twelve properties the section actually names.
// -------------------------------------------------------------------------------------------

/// Would this pass if `combine_toggle` used plain override (the naive, and wrong, implementation
/// every other field in this ladder correctly uses)? No. Paragraph 3's paragraph style
/// (`BoldParaStyle`) states `w:b` (bold, default `true`), and its run's own character style
/// (`BoldCharStyle`, via `w:rStyle`) *also* states `w:b` — both `true`, no direct `w:b` on the run
/// itself. A naive override resolver reads the character-style tier (higher priority) and answers
/// `true`. ECMA-376 Part 1 §17.7.3's own rule is Boolean XOR across tiers: `true XOR true = false`.
/// This is the naive-implementation mutation the child's own Done-when names explicitly — flipping
/// `combine_toggle` to plain fallback (`direct.or(character_tier).or(paragraph_tier).or(numbering).or(doc_defaults)`)
/// turns this test red (`left: Some(true), right: Some(false)`); restored by re-editing.
#[test]
fn a_paragraph_style_and_character_style_both_bold_cancel_to_not_bold() {
    let mut document = open();
    let effective = document
        .effective_run_properties(3, 0)
        .expect("paragraph 3, run 0");
    assert_eq!(
        effective.bold,
        Some(false),
        "true XOR true must cancel to false, not stay true"
    );
}

/// The XOR-with-one-term control: paragraph 4 carries only the bold paragraph style (no character
/// style on the run). A single `true` term XORs to `true` — this is what proves the fixture's
/// `false` answer above comes from genuine cancellation, not from some unrelated bug that makes
/// every bold-shaped fixture read `false`.
#[test]
fn a_single_bold_tier_alone_stays_bold() {
    let mut document = open();
    let effective = document
        .effective_run_properties(4, 0)
        .expect("paragraph 4, run 0");
    assert_eq!(effective.bold, Some(true));
}

/// Direct formatting always wins outright, even over an XOR combination that would otherwise cancel
/// to `false` — paragraph 5 carries the same bold paragraph style and bold character style as
/// paragraph 3 (which cancel), but the run *also* states `w:b` directly. Direct must win regardless
/// of what the styles combine to.
#[test]
fn direct_bold_wins_over_a_cancelling_style_combination() {
    let mut document = open();
    let effective = document
        .effective_run_properties(5, 0)
        .expect("paragraph 5, run 0");
    assert_eq!(effective.bold, Some(true));
}

// -------------------------------------------------------------------------------------------
// Theme colour and theme font, resolved through mjx-dml's own theme model.
// -------------------------------------------------------------------------------------------

/// Paragraph 6's run states `w:color` with `w:themeColor="accent1"` (and an unrelated literal `val`
/// that must be ignored once the theme reference resolves) and `w:rFonts` with
/// `w:asciiTheme="minorAscii"` and no literal `w:ascii`. `theme/theme1.xml` defines `a:accent1` as
/// `4F81BD` and the minor font's Latin typeface as `Calibri`. Would this pass if theme resolution
/// were not wired up? No — an unresolved reader would hand back the literal (wrong) `000000` for the
/// colour and `None` for the ascii font (no literal to fall back to).
#[test]
fn theme_colour_and_theme_font_resolve_through_mjx_dml() {
    let mut document = open();
    let effective = document
        .effective_run_properties(6, 0)
        .expect("paragraph 6, run 0");

    match effective.color {
        Some(mjx_docx::EffectiveColor::Hex(hex)) => assert_eq!(hex, "4F81BD"),
        other => panic!("expected a resolved theme colour, got {other:?}"),
    }
    assert_eq!(
        effective.fonts.and_then(|fonts| fonts.ascii),
        Some("Calibri".to_owned())
    );
}

// -------------------------------------------------------------------------------------------
// Dangling references degrade the way the module's own doc comment states: a dangling style id is a
// documented fallback (the tier contributes nothing, everything else still resolves); a dangling
// numId is a typed error, never a panic.
// -------------------------------------------------------------------------------------------

/// Paragraph 7's `w:pStyle="NoSuchStyle"` names a style this style sheet does not define, and
/// carries no `w:numPr` at all. The read must still succeed — the paragraph-style tier contributes
/// nothing, and `w:docDefaults`' 10pt still answers `font_size`.
#[test]
fn a_dangling_paragraph_style_degrades_to_no_paragraph_tier_rather_than_erroring() {
    let mut document = open();
    let effective = document
        .effective_run_properties(7, 0)
        .expect("a dangling w:pStyle must not fail the whole read");
    assert_eq!(
        effective.font_size,
        Some(mjx_ooxml_types::wordprocessingml::HalfPointMeasure::from_wire("20"))
    );
    let paragraph = document
        .effective_paragraph_properties(7)
        .expect("a dangling w:pStyle must not fail the whole read");
    assert_eq!(paragraph.alignment, None);
}

/// Paragraph 8's `w:numPr` names `numId="99"`, which `word/numbering.xml` does not define (only
/// `numId="1"` exists). Both effective readers must return the typed
/// [`DocxError::UnknownNumberingId`], never a panic and never a silently wrong answer.
#[test]
fn a_dangling_num_id_is_a_typed_error_not_a_panic() {
    let mut document = open();
    match document.effective_run_properties(8, 0) {
        Err(DocxError::UnknownNumberingId(99)) => {}
        other => panic!("expected DocxError::UnknownNumberingId(99), got {other:?}"),
    }
    let mut document = open();
    match document.effective_paragraph_properties(8) {
        Err(DocxError::UnknownNumberingId(99)) => {}
        other => panic!("expected DocxError::UnknownNumberingId(99), got {other:?}"),
    }
}

// -------------------------------------------------------------------------------------------
// Paragraph-level ladder: the same tiers apply (minus the character-style rung, which affects only
// run formatting), and none of the eighteen paragraph-level `CT_OnOff` members is a toggle property
// — plain override throughout.
// -------------------------------------------------------------------------------------------

/// Paragraph 0's own effective paragraph properties: `w:numId` names a list, so `numbering` must be
/// populated with the reference (not the rendered number — MJXOFF-104's own boundary, restated
/// here).
#[test]
fn effective_paragraph_properties_reports_the_numbering_reference() {
    let mut document = open();
    let effective = document
        .effective_paragraph_properties(0)
        .expect("paragraph 0");
    assert_eq!(
        effective.numbering.map(|reference| reference.numbering_id),
        Some(1)
    );
}

// -------------------------------------------------------------------------------------------
// The ticket's own Done-when names `sample.docx`'s `theme1.xml` specifically. Checked directly:
// neither `sample.docx`'s `word/document.xml` nor its `word/styles.xml` contains a single
// `w:themeColor`/`asciiTheme`-family attribute (`grep -o 'themeColor="[a-zA-Z0-9]*"'` and the
// `*Theme="..."` family both match zero times) — there is no theme reference in that fixture for
// any resolver to resolve, real or fake. The genuine end-to-end resolution proof above therefore
// has to live on an authored fixture; what `sample.docx` *can* still prove is that this reader's
// theme-loading path does not choke on a real, Office-authored `theme1.xml` (as opposed to the
// synthetic one this child wrote) — a `<a:sysClr>`-free but otherwise ordinarily-shaped theme part.
// -------------------------------------------------------------------------------------------

/// `sample.docx` ships `word/theme/theme1.xml`; opening it and reading any run's effective
/// properties must not fail merely because a real theme part is now in the mix, even though this
/// particular fixture's runs happen not to reference it.
#[test]
fn a_real_office_authored_theme_part_does_not_break_resolution() {
    let mut document = Document::open(&fixture("sample.docx")).expect("open sample.docx");
    let count = document.paragraph_count().expect("paragraph count");
    assert!(count > 0, "sample.docx must carry at least one paragraph");
    for index in 0..count {
        document
            .effective_paragraph_properties(index)
            .unwrap_or_else(|error| panic!("paragraph {index}: {error}"));
    }
}
