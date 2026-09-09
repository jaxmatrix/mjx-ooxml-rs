//! **Where every expected value in this crate came from**, declared row by row and printed on every
//! run.
//!
//! # Nobody ran Word
//!
//! Not once, anywhere in this child. So a green suite must not be readable as *"this matches
//! Word"*, and the only way to stop it being read that way is to say, for each expectation, what
//! kind of thing it is:
//!
//! * **`SpecCode`** — the value is stated in ECMA-376 Part 1, or is a schema default. The strongest
//!   kind here, and it still says nothing about what Word *renders*: the specification defines
//!   `w:widowControl`'s default and not what a renderer does with a widow.
//! * **`DocumentedBehaviour`** — the value has an external, checkable definition somewhere other
//!   than this repository: a Unicode line-breaking class, a JIS standard, an arithmetic identity.
//!   **These are the rows that are actually evidence.**
//! * **`EngineDerived`** — the expectation was read off this engine. It is a **change detector and
//!   not evidence about Word**, and every one of them is a candidate for the Windows sitting.
//!
//! MJXOFF-172 (R17) split 31 / 110 / 52 and MJXOFF-173 (R18) repeated it. This is the third run of
//! the same instrument on a third format, and the split is printed rather than described so that a
//! reader of a green run sees the shape of the evidence rather than the fact of a pass.

use std::collections::BTreeMap;

/// Where one expected value came from.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Provenance {
    /// ECMA-376 Part 1, or a schema default.
    SpecCode,
    /// An external, checkable definition that is not this repository's.
    DocumentedBehaviour,
    /// Read off this engine. A change detector, not evidence.
    EngineDerived,
}

/// One expectation a suite in this crate asserts.
struct Row {
    /// Which suite asserts it.
    suite: &'static str,
    /// What it is about.
    subject: &'static str,
    /// Where the number or the behaviour came from.
    provenance: Provenance,
    /// The citation, for a `SpecCode` or `DocumentedBehaviour` row, or the reason the value is only
    /// this engine's for an `EngineDerived` one.
    because: &'static str,
}

/// Every expectation this crate's suites rest on.
///
/// It is hand-maintained, deliberately: a table generated from the tests would say what the tests
/// say and could not say where the tests got it. Adding an assertion without adding a row here is a
/// silent claim about Word.
const LEDGER: &[Row] = &[
    // ---------------------------------------------------------------------------------------
    // SpecCode — the specification says the value.
    // ---------------------------------------------------------------------------------------
    Row {
        suite: "each_constraint_moves_a_paragraph",
        subject: "`w:widowControl` defaults to on",
        provenance: Provenance::SpecCode,
        because:
            "§17.3.1.44's own default; reading it as off turns the rule off for every document \
                  that does not write it, which is most of them",
    },
    Row {
        suite: "spacing_tabs_and_indents",
        subject: "`w:line` is 240ths of a line under `w:lineRule=\"auto\"`",
        provenance: Provenance::SpecCode,
        because: "§17.3.1.33; the one place a `ST_SignedTwipsMeasure` is not a measure",
    },
    Row {
        suite: "spacing_tabs_and_indents",
        subject: "`w:lineRule=\"exact\"` and `\"atLeast\"` are lengths in twips",
        provenance: Provenance::SpecCode,
        because: "§17.3.1.33",
    },
    Row {
        suite: "spacing_tabs_and_indents",
        subject: "`w:hanging` outranks `w:firstLine`",
        provenance: Provenance::SpecCode,
        because: "§17.3.1.12 declares them mutually exclusive",
    },
    Row {
        suite: "spacing_tabs_and_indents",
        subject: "`w:val=\"clear\"` removes a tab stop rather than adding one",
        provenance: Provenance::SpecCode,
        because: "§17.3.1.37",
    },
    Row {
        suite: "spacing_tabs_and_indents",
        subject: "the nine members of `ST_TabJc` and the six of `ST_TabTlc`",
        provenance: Provenance::SpecCode,
        because: "`wml.xsd`, through `mjx_ooxml_types::wordprocessingml`",
    },
    Row {
        suite: "a_justified_line_has_positions",
        subject: "the twelve members of `ST_Jc`",
        provenance: Provenance::SpecCode,
        because: "`wml.xsd`",
    },
    Row {
        suite: "a_justified_line_has_positions",
        subject: "`left`/`right` are physical and `start`/`end` follow the direction",
        provenance: Provenance::SpecCode,
        because: "§17.18.44 keeps all four, which it would not need to if they were synonyms",
    },
    Row {
        suite: "no_rule_rounds_to_nothing",
        subject: "a border's `w:sz` is in eighths of a point",
        provenance: Provenance::SpecCode,
        because: "`ST_EighthPointMeasure`",
    },
    Row {
        suite: "no_rule_rounds_to_nothing",
        subject: "`w:sz=\"0\"` is a line and `w:val=\"none\"` is its absence",
        provenance: Provenance::SpecCode,
        because: "they are different attributes; §17.3.4 gives `none` its own value",
    },
    Row {
        suite: "spacing_tabs_and_indents",
        subject: "a twip is a twentieth of a point and an EMU is 1/914,400 inch",
        provenance: Provenance::SpecCode,
        because: "ECMA-376 Part 1 §22.9.2.14 and §20.1.2.1",
    },
    Row {
        suite: "break_opportunities_that_matter",
        subject: "`w:br` ends a line whatever the measure says",
        provenance: Provenance::SpecCode,
        because: "§17.3.3.1",
    },
    // ---------------------------------------------------------------------------------------
    // DocumentedBehaviour — an external, checkable definition. The rows that are evidence.
    // ---------------------------------------------------------------------------------------
    Row {
        suite: "break_opportunities_that_matter",
        subject: "a line may break after `U+002D HYPHEN-MINUS`",
        provenance: Provenance::DocumentedBehaviour,
        because: "UAX #14 class `HY`",
    },
    Row {
        suite: "break_opportunities_that_matter",
        subject: "a line may **not** break at `U+00A0 NO-BREAK SPACE`",
        provenance: Provenance::DocumentedBehaviour,
        because: "UAX #14 class `GL`",
    },
    Row {
        suite: "break_opportunities_that_matter",
        subject: "`w:noBreakHyphen` is `U+2011`, which does not break",
        provenance: Provenance::DocumentedBehaviour,
        because: "UAX #14 class `GL`; the element's own name says the rest",
    },
    Row {
        suite: "break_opportunities_that_matter",
        subject: "`w:softHyphen` is `U+00AD`, which does break",
        provenance: Provenance::DocumentedBehaviour,
        because: "UAX #14 class `BA`",
    },
    Row {
        suite: "break_opportunities_that_matter",
        subject: "nearly every CJK character is a break opportunity",
        provenance: Provenance::DocumentedBehaviour,
        because: "UAX #14 class `ID`",
    },
    Row {
        suite: "break_opportunities_that_matter",
        subject: "kinsoku removes an opportunity plain UAX #14 offers",
        provenance: Provenance::DocumentedBehaviour,
        because: "JIS X 4051's 行頭禁則 set, which `mjx_text::KinsokuRules` holds",
    },
    Row {
        suite: "break_opportunities_that_matter",
        subject: "a URL breaks at its slashes rather than overflowing",
        provenance: Provenance::DocumentedBehaviour,
        because: "UAX #14 class `SY` after a solidus",
    },
    Row {
        suite: "a_justified_line_has_positions",
        subject: "a Japanese line is stretched between its characters and a Latin word is not",
        provenance: Provenance::DocumentedBehaviour,
        because: "JIS X 4051 §4.4 and the Unicode blocks the character set is read from; **the \
                  character set itself is a GUESS**",
    },
    Row {
        suite: "a_justified_line_has_positions",
        subject: "the slack is shared equally between the gaps",
        provenance: Provenance::DocumentedBehaviour,
        because: "arithmetic: the definition of filling a measure by widening n gaps",
    },
    Row {
        suite: "a_justified_line_has_positions",
        subject: "right alignment moves a line twice as far as centring",
        provenance: Provenance::DocumentedBehaviour,
        because: "arithmetic: the slack and half the slack",
    },
    Row {
        suite: "spacing_tabs_and_indents",
        subject: "`w:line=\"480\"` under `auto` is exactly double `w:line=\"240\"`",
        provenance: Provenance::DocumentedBehaviour,
        because: "arithmetic, given §17.3.1.33's unit",
    },
    Row {
        suite: "a_checkpoint_is_work_not_output",
        subject: "a resumed page and a walked page are the same page",
        provenance: Provenance::DocumentedBehaviour,
        because: "`mjx_layout::BoxModel::layout_page`'s own stated contract",
    },
    Row {
        suite: "a_long_document_paginates_consistently",
        subject: "every line appears exactly once across the document",
        provenance: Provenance::DocumentedBehaviour,
        because:
            "the definition of pagination; a line drawn twice or dropped is a defect under any \
                  reading",
    },
    Row {
        suite: "termination",
        subject: "every page places at least one line",
        provenance: Provenance::DocumentedBehaviour,
        because: "the termination argument: a page that places nothing repeats itself for ever",
    },
    Row {
        suite: "no_rule_rounds_to_nothing",
        subject: "half of a one-EMU stroke must not be nothing",
        provenance: Provenance::DocumentedBehaviour,
        because: "arithmetic, and `mjx-layout-xlsx`'s own `border::half_of`, which found it first",
    },
    // ---------------------------------------------------------------------------------------
    // EngineDerived — change detectors. NOT evidence about Word.
    // ---------------------------------------------------------------------------------------
    Row {
        suite: "each_constraint_moves_a_paragraph",
        subject: "a widow is refused by sending a second line over, an orphan by moving the \
                  paragraph whole",
        provenance: Provenance::EngineDerived,
        because: "ECMA-376 names `w:widowControl` and does not define what a renderer does; the \
                  two-line minimum is this engine's reading",
    },
    Row {
        suite: "each_constraint_moves_a_paragraph",
        subject: "`w:pageBreakBefore` is ignored on the first paragraph of the document",
        provenance: Provenance::EngineDerived,
        because: "GUESS: honouring it would open every such document with a blank page",
    },
    Row {
        suite: "each_constraint_moves_a_paragraph",
        subject: "a `w:keepNext` chain is broken where breaking it would empty the page",
        provenance: Provenance::EngineDerived,
        because: "an unsatisfiable chain has no correct answer; where Word breaks it is unknown",
    },
    Row {
        suite: "each_constraint_moves_a_paragraph",
        subject: "a `w:keepLines` paragraph taller than a page is placed anyway",
        provenance: Provenance::EngineDerived,
        because: "the same: the constraint cannot be satisfied and something must be drawn",
    },
    Row {
        suite: "spacing_tabs_and_indents",
        subject: "a tab stop's `w:pos` is measured from the column edge, before indents",
        provenance: Provenance::EngineDerived,
        because:
            "GUESS: §17.3.1.37 says \"the current text margin\" and does not define whether an \
                  indent moves it",
    },
    Row {
        suite: "spacing_tabs_and_indents",
        subject: "the default grid starts past a `bar` stop",
        provenance: Provenance::EngineDerived,
        because:
            "GUESS: a bar stop is a custom stop for the grid's purposes but not for the pen's; \
                  the other reading is two inches away on the page",
    },
    Row {
        suite: "spacing_tabs_and_indents",
        subject: "a decimal tab with no separator behaves as a trailing tab",
        provenance: Provenance::EngineDerived,
        because: "GUESS: it is what lines a column of whole numbers up with a column of decimals",
    },
    Row {
        suite: "spacing_tabs_and_indents",
        subject: "the five leader characters",
        provenance: Provenance::EngineDerived,
        because: "GUESS: ECMA-376 names the styles and not the glyphs; `heavy` is drawn as an \
                  ordinary underscore and is visibly lighter than Word's",
    },
    Row {
        suite: "spacing_tabs_and_indents",
        subject: "the extra leading of a multiple or `atLeast` line goes above the baseline",
        provenance: Provenance::EngineDerived,
        because: "GUESS: putting it below would move every baseline on the page",
    },
    Row {
        suite: "spacing_tabs_and_indents",
        subject: "`w:defaultTabStop` falls back to 720 twips",
        provenance: Provenance::EngineDerived,
        because: "a fact about Word's `Normal.dotm` rather than about the schema, which gives the \
                  element no default at all",
    },
    Row {
        suite: "a_long_document_paginates_consistently",
        subject: "the space above a paragraph is suppressed at the top of a page",
        provenance: Provenance::EngineDerived,
        because: "GUESS: the alternative is a band of white space above the first line of every \
                  page",
    },
    Row {
        suite: "a_long_document_paginates_consistently",
        subject: "the page count of the fixture",
        provenance: Provenance::EngineDerived,
        because:
            "**nobody has opened it in Word**; what is asserted is internal consistency, which \
                  is a change detector",
    },
    Row {
        suite: "a_justified_line_has_positions",
        subject: "a line with a tab on it is not aligned any further",
        provenance: Provenance::EngineDerived,
        because:
            "GUESS: a tab stop is an absolute position, so translating the line moves the text \
                  off it",
    },
    Row {
        suite: "a_justified_line_has_positions",
        subject: "the three kashida alignments and `numTab` fall back to ordinary justification",
        provenance: Provenance::EngineDerived,
        because: "GUESS: kashida elongation needs a shaper feature nothing here drives, and \
                  `numTab` needs a list number, which is R22's",
    },
    Row {
        suite: "break_opportunities_that_matter",
        subject: "a hyphenated line's hyphen is `U+2010` in the paragraph's first run's face",
        provenance: Provenance::EngineDerived,
        because: "GUESS: Word takes the face of the word it splits, which differs in a paragraph \
                  that changes font mid-word",
    },
    Row {
        suite: "no_rule_rounds_to_nothing",
        subject: "the hairline is a quarter point",
        provenance: Provenance::EngineDerived,
        because:
            "GUESS: `w:sz=\"2\"` is a quarter point and the specification says nothing about a \
                  narrower one",
    },
    Row {
        suite: "the_fragments_match_their_baselines",
        subject: "every number in every committed baseline",
        provenance: Provenance::EngineDerived,
        because: "a golden file generated by the code under test always matches the code under \
                  test; each one states `approver = generator`, which records that no person has \
                  looked at it",
    },
    Row {
        suite: "spacing_tabs_and_indents",
        subject: "`w:contextualSpacing` suppresses the gap between two paragraphs of one style",
        provenance: Provenance::SpecCode,
        because:
            "§17.3.1.9 states the rule; what it does not state is whether two paragraphs that \
                  name no style are the same style, which this engine reads as yes",
    },
    Row {
        suite: "the_public_surface_is_reachable",
        subject: "an unstated font size is ten points",
        provenance: Provenance::EngineDerived,
        because: "GUESS: `w:sz` has no schema default; a run that inherits nothing at all is a \
                  document Word never wrote",
    },
    Row {
        suite: "the_public_surface_is_reachable",
        subject: "the **ASCII** slot of `w:rFonts` is what is asked for",
        provenance: Provenance::EngineDerived,
        because:
            "GUESS: `w:hint` decides among four slots and `mjx-text` already does per-character \
                  fallback from one request; a mixed Japanese paragraph will fall back rather than \
                  use its stated East Asian face",
    },
];

fn split() -> BTreeMap<Provenance, usize> {
    let mut counts = BTreeMap::new();
    for row in LEDGER {
        *counts.entry(row.provenance).or_insert(0) += 1;
    }
    counts
}

/// Prints the ledger on **every** run, so a green suite cannot be read as a claim about Word.
#[test]
fn the_split_is_printed_and_asserted_in_both_directions() {
    let counts = split();
    println!("\nProvenance of every expected value in `mjx-layout-docx`:\n");
    for (provenance, count) in &counts {
        println!("  {provenance:>20?}  {count:>3}");
    }
    println!("  {:>20}  {:>3}\n", "total", LEDGER.len());
    println!("  Nobody ran Microsoft Word. `EngineDerived` rows are change detectors and are not");
    println!("  evidence about Word; every one is a candidate for the Windows sitting.\n");

    let spec = counts.get(&Provenance::SpecCode).copied().unwrap_or(0);
    let documented = counts
        .get(&Provenance::DocumentedBehaviour)
        .copied()
        .unwrap_or(0);
    let engine = counts.get(&Provenance::EngineDerived).copied().unwrap_or(0);

    // **Both directions.** A ledger that only had a floor would pass for a build that had quietly
    // relabelled everything as `SpecCode`; a ledger that only had a ceiling would pass for one that
    // had stopped asserting anything at all.
    assert_eq!(spec + documented + engine, LEDGER.len());
    assert!(
        spec >= 10,
        "the specification really does state this many of them: {spec}"
    );
    assert!(
        documented >= 12,
        "these are the rows that are evidence, and there must be some: {documented}"
    );
    assert!(
        engine >= 12,
        "and this many are only this engine agreeing with itself — a count that *fell* would mean \
         somebody had relabelled a guess: {engine}"
    );
    assert!(
        engine * 2 >= documented,
        "if the guesses ever stopped outnumbering the evidence it would be because somebody had \
         run Word, and this assertion is the place to record that: {engine} against {documented}"
    );
}

/// Every suite named in the ledger exists, so a row cannot outlive the assertion it describes.
#[test]
fn every_suite_the_ledger_names_exists() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    for row in LEDGER {
        let path = directory.join(format!("{}.rs", row.suite));
        assert!(
            path.exists(),
            "the ledger names `{}`, which is not a suite in this crate",
            row.suite
        );
    }
}

/// Every `EngineDerived` row says *why* it is only this engine's, which is what makes the sitting's
/// list writable from this file alone.
#[test]
fn every_engine_derived_row_says_why() {
    for row in LEDGER {
        if row.provenance != Provenance::EngineDerived {
            continue;
        }
        assert!(
            row.because.len() > 30,
            "`{}` is a change detector and the reason must be usable by whoever runs Word",
            row.subject
        );
    }
}
