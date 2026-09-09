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
//! MJXOFF-172 (R17) split 31 / 110 / 52, MJXOFF-173 (R18) repeated it, and MJXOFF-174 (R19) opened
//! this crate at 13 / 15 / 19. MJXOFF-175 (R20) adds sections, columns, headers and notes, and
//! MJXOFF-176 (R21) adds tables and floating objects. The split is printed rather than described so
//! that a reader of a green run sees the shape of the evidence rather than the fact of a pass.
//!
//! **R21's evidence is weaker again, and for a nameable reason: there is no external standard for
//! text wrapping at all.** UAX #14 defines what a line breaker consumes and nothing defines what a
//! renderer does with a `wp:wrapPolygon`; ECMA-376 names the five wrap elements and the two table
//! layout algorithms and defines neither algorithm. So this child's `DocumentedBehaviour` rows are
//! arithmetic and an algorithm borrowed from HTML, its `SpecCode` rows are attributes and defaults,
//! and every question about *where a line actually goes beside an object* is `EngineDerived`.
//!
//! **The evidence got weaker, and that is the honest report.** R19's `DocumentedBehaviour` rows were
//! unusually strong for a layout engine, because UAX #14 is an external, checkable definition of
//! exactly what a line breaker consumes. There is no equivalent for *where a footnote area's gap
//! goes*, *what an absent `w:type` means*, or *which number decides that a page is even*: ECMA-376
//! defines the attributes and is silent on the rendering. So this child's rows are mostly
//! `EngineDerived`, and the ratio moving in that direction is a fact about the subject rather than
//! about the care taken.

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

    // =======================================================================================
    // MJXOFF-175 (R20) — sections, columns, headers and footers, footnotes and endnotes.
    //
    // The same instrument on the same crate, one child later. **Nobody ran Word for these either**,
    // and the shape of the evidence is worse here than it was for R19's line breaking: UAX #14 gave
    // that child an external definition of exactly what it consumed, and there is no equivalent
    // external definition of *where a footnote area's gap goes* or *what an absent `w:type` means*.
    // Those are the rows the Windows sitting is for.
    // =======================================================================================

    // ---------------------------------------------------------------------------------------
    // SpecCode — the specification says the value.
    // ---------------------------------------------------------------------------------------
    Row {
        suite: "a_section_changes_the_page",
        subject: "a `w:sectPr` inside a paragraph ENDS the section that paragraph belongs to",
        provenance: Provenance::SpecCode,
        because: "§17.6.17 and `mjx-docx`'s own `sections.rs`, which quotes it; the body-level one \
                  governs whatever is left",
    },
    Row {
        suite: "a_section_changes_the_page",
        subject: "the five members of `ST_SectionMark`",
        provenance: Provenance::SpecCode,
        because: "`wml.xsd`, through `mjx_ooxml_types::wordprocessingml::SectionBreakType`",
    },
    Row {
        suite: "a_section_changes_the_page",
        subject: "`w:type/@val` carries no schema default at all",
        provenance: Provenance::SpecCode,
        because: "`wml.xsd`; `mjx-docx` therefore reports `None` rather than asserting one, which \
                  is why the reading is a layout decision and appears below as a guess",
    },
    Row {
        suite: "a_section_changes_the_page",
        subject: "`w:pgNumType@fmt` defaults to `decimal`",
        provenance: Provenance::SpecCode,
        because: "`wml.xsd`'s own default on `CT_PageNumber`",
    },
    Row {
        suite: "a_section_changes_the_page",
        subject: "`w:mirrorMargins` swaps the left and right margins on an even page",
        provenance: Provenance::SpecCode,
        because: "§17.15.1.71 — the binding margin is always on the inside edge",
    },
    Row {
        suite: "columns_balance_at_a_continuous_break",
        subject: "`w:equalWidth=\"true\"` outranks an explicit `w:col` list",
        provenance: Provenance::SpecCode,
        because: "§17.6.4's own prose and worked example, quoted in `mjx-docx`'s `sections.rs`: \
                  the `w:col` children are described as ignored",
    },
    Row {
        suite: "columns_balance_at_a_continuous_break",
        subject: "`w:cols@space` defaults to 720 twips and `w:cols@num` to one",
        provenance: Provenance::SpecCode,
        because: "`wml.xsd`'s own defaults on `CT_Columns`",
    },
    Row {
        suite: "columns_balance_at_a_continuous_break",
        subject: "a `w:col`'s own `w:space` is the gap to the column after it",
        provenance: Provenance::SpecCode,
        because: "§17.6.3 — which is why an explicit list's gaps are not the `w:cols@space`",
    },
    Row {
        suite: "the_headers_differ_by_page",
        subject: "`w:titlePg` off makes a first-page reference be ignored",
        provenance: Provenance::SpecCode,
        because: "§17.10.6, quoted in `mjx-docx`'s `headers.rs`: \"it shall be ignored and only the \
                  odd page header/footer shall be displayed\"",
    },
    Row {
        suite: "the_headers_differ_by_page",
        subject: "`w:evenAndOddHeaders` off makes an even-page reference be ignored",
        provenance: Provenance::SpecCode,
        because: "§17.10.1, the same sentence for the even variant",
    },
    Row {
        suite: "the_headers_differ_by_page",
        subject: "a variant a section does not state is inherited from the nearest preceding one",
        provenance: Provenance::SpecCode,
        because: "§17.10.5 and §17.10.2, identical prose, and stated per variant independently",
    },
    Row {
        suite: "the_headers_differ_by_page",
        subject: "`w:pgMar@header` is measured from the page's own top edge",
        provenance: Provenance::SpecCode,
        because: "§17.6.11 — not from the text margin, which is what makes the body's top a \
                  comparison rather than an addition",
    },
    Row {
        suite: "a_footnote_moves_the_body",
        subject: "`ST_FtnEdn`'s four members, and that an absent `w:type` is `normal`",
        provenance: Provenance::SpecCode,
        because: "§17.11.10's own attribute table: \"it shall be considered to be of style normal\"",
    },
    Row {
        suite: "a_footnote_moves_the_body",
        subject: "a footnote that is not `normal` is never referenced from the main story",
        provenance: Provenance::SpecCode,
        because: "§17.11.10 states it directly, which is why the separators are excluded from the \
                  demand by `w:type` and not by an id range",
    },
    Row {
        suite: "a_footnote_moves_the_body",
        subject: "`w:numRestart`'s three values, and `continuous` when it is absent",
        provenance: Provenance::SpecCode,
        because: "`ST_RestartNumber`, through `mjx_ooxml_types::wordprocessingml`",
    },
    Row {
        suite: "endnotes_flow_at_the_end_of_their_scope",
        subject: "`w:endnotePr/w:pos`'s two values, `sectEnd` and `docEnd`",
        provenance: Provenance::SpecCode,
        because: "§17.11.3 — and they are positions in the flow, which is what makes an endnote not \
                  a second area",
    },
    Row {
        suite: "line_numbers_restart_per_mode",
        subject: "`w:lnNumType@start` defaults to one and `@restart` to `newPage`",
        provenance: Provenance::SpecCode,
        because: "`wml.xsd`'s own defaults on `CT_LineNumber`",
    },
    Row {
        suite: "line_numbers_restart_per_mode",
        subject: "the three members of `ST_LineNumberRestart`",
        provenance: Provenance::SpecCode,
        because: "`wml.xsd`, through `mjx_ooxml_types::wordprocessingml::LineNumberRestart`",
    },

    // ---------------------------------------------------------------------------------------
    // DocumentedBehaviour — checkable somewhere other than this repository.
    // ---------------------------------------------------------------------------------------
    Row {
        suite: "columns_balance_at_a_continuous_break",
        subject: "bisection finds the least point at which a monotone predicate holds",
        provenance: Provenance::DocumentedBehaviour,
        because: "an arithmetic identity, and the reason column balancing is a search rather than a \
                  division; the predicate's monotonicity is argued in `crate::paginate`",
    },
    Row {
        suite: "a_footnote_moves_the_body",
        subject: "a non-increasing map reaches its fixed point in one step from below",
        provenance: Provenance::DocumentedBehaviour,
        because: "arithmetic: if N is non-increasing and R₁ = N(R₀) > R₀ then N(R₁) ≤ N(R₀) = R₁, \
                  which is the whole two-assembly bound in `crate::notes`",
    },
    Row {
        suite: "a_section_changes_the_page",
        subject: "Roman numerals are additive with subtractive pairs, up to 3999",
        provenance: Provenance::DocumentedBehaviour,
        because: "the numeral system itself, which is an external definition; above 3999 there is \
                  no notation without the overline and this writes decimal instead",
    },
    Row {
        suite: "a_section_changes_the_page",
        subject: "`upperLetter` repeats a letter rather than counting in base twenty-six",
        provenance: Provenance::DocumentedBehaviour,
        because: "Microsoft's own documented numbering for `ST_NumberFormat`: the twenty-seventh is \
                  `AA` and the fifty-third `AAA`, which base twenty-six would write `AB` and `BA`",
    },
    Row {
        suite: "termination",
        subject: "a loop whose variant strictly decreases and is bounded below terminates",
        provenance: Provenance::DocumentedBehaviour,
        because: "the standard loop-variant argument, applied to two flows at once: every page \
                  consumes at least one line of the body and at least one of any carried note",
    },

    // ---------------------------------------------------------------------------------------
    // EngineDerived — read off this engine. Change detectors, and the Windows sitting's list.
    // ---------------------------------------------------------------------------------------
    Row {
        suite: "a_section_changes_the_page",
        subject: "an absent `w:type` starts a page",
        provenance: Provenance::EngineDerived,
        because: "`wml.xsd` gives `w:type/@val` no default and the prose does not say; the other \
                  reading — treating it as `continuous` — runs two sections' page geometry together \
                  on one sheet, which is a visibly different document",
    },
    Row {
        suite: "a_section_changes_the_page",
        subject: "`evenPage`/`oddPage` parity is tested against the ASSIGNED page number",
        provenance: Provenance::EngineDerived,
        because: "the alternative is the physical sheet index, and the two differ in exactly the \
                  documents that use the feature: one whose sections restart their numbering. Which \
                  one Word uses decides whether the blank page is there at all",
    },
    Row {
        suite: "a_section_changes_the_page",
        subject: "an `evenPage`/`oddPage` break inserts at most one blank page",
        provenance: Provenance::EngineDerived,
        because: "one flip is enough to reach the wanted parity, so a second would be a defect; \
                  Word's own behaviour when the parity is already right is not stated anywhere",
    },
    Row {
        suite: "a_section_changes_the_page",
        subject: "a blank page still shows its section's header and footer",
        provenance: Provenance::EngineDerived,
        because: "the page is printed, so something must be on it, but nothing says whether Word \
                  treats an inserted blank as a page of the old section or of the new one",
    },
    Row {
        suite: "a_section_changes_the_page",
        subject: "`w:vAlign=\"both\"` falls back to `top`",
        provenance: Provenance::EngineDerived,
        because: "§17.6.23 says the text is justified vertically without saying between what, and \
                  distributing the slack between paragraphs moves every baseline on the page",
    },
    Row {
        suite: "a_section_changes_the_page",
        subject: "`w:rtlGutter` is not honoured and the gutter is always on the left of an odd page",
        provenance: Provenance::EngineDerived,
        because: "a document that sets it has its binding space on the wrong edge here, which is a \
                  visible half-inch on every page; reported rather than hidden",
    },
    Row {
        suite: "a_section_changes_the_page",
        subject: "a section that governs no paragraph is skipped rather than filled",
        provenance: Provenance::EngineDerived,
        because: "two `w:sectPr`s with nothing between them; the alternative — letting the empty \
                  section take the next one's content — is what a naive walk does, and nothing says \
                  which page geometry Word uses for a section with no content of its own",
    },
    Row {
        suite: "a_section_changes_the_page",
        subject: "a section that states no page size inherits the caller's constraints",
        provenance: Provenance::EngineDerived,
        because: "there is no document to defer to, so the fallback is ours; Word would use its own \
                  template's default, which is a different number on a different machine",
    },
    Row {
        suite: "columns_balance_at_a_continuous_break",
        subject: "balancing happens at a `continuous` break and NOT at the end of a document",
        provenance: Provenance::EngineDerived,
        because: "adding a trailing continuous break is the well-known trick for balancing the last \
                  columns, which only makes sense if the document's own end does not balance them — \
                  an inference from a habit rather than from a specification",
    },
    Row {
        suite: "columns_balance_at_a_continuous_break",
        subject: "a `continuous` break whose section changes the sheet starts a page anyway",
        provenance: Provenance::EngineDerived,
        because: "two page sizes cannot share one sheet, so something must give; whether Word \
                  breaks the page or ignores the new geometry is not stated",
    },
    Row {
        suite: "columns_balance_at_a_continuous_break",
        subject: "a `w:col` with no `w:w` states a width of zero rather than an equal share",
        provenance: Provenance::EngineDerived,
        because: "the attribute is optional and the schema names no default; sharing the remainder \
                  out would be indistinguishable from a stated width, and this way the file's own \
                  silence stays visible",
    },
    Row {
        suite: "the_headers_differ_by_page",
        subject: "\"an even page\" is a page whose DISPLAYED number is even",
        provenance: Provenance::EngineDerived,
        because: "§17.10.1 does not say which number decides, and a section that restarts its \
                  numbering has its even and odd headers swapped under the other reading — a \
                  visible difference on every page of that section",
    },
    Row {
        suite: "the_headers_differ_by_page",
        subject: "a header taller than the top margin pushes the body down",
        provenance: Provenance::EngineDerived,
        because: "the alternative is text printed on top of text, so this is the only sane reading \
                  — but by how much, and whether Word clips instead, is not stated anywhere",
    },
    Row {
        suite: "the_headers_differ_by_page",
        subject: "a section with NO header does not have its top margin grown at all",
        provenance: Provenance::EngineDerived,
        because: "`w:pgMar@header` is where a header would start, not a second top margin; reading \
                  it the other way puts half an inch of white space at the top of every page of \
                  every document that has no header, which is most of them",
    },
    Row {
        suite: "a_footnote_moves_the_body",
        subject: "the note area's reservation is capped at the body's first line",
        provenance: Provenance::EngineDerived,
        because: "it is what makes a footnote taller than the page terminate, and Word's own \
                  division of a page between an enormous note and the body is not documented; the \
                  cap decides how much body text a reader sees on such a page",
    },
    Row {
        suite: "a_footnote_moves_the_body",
        subject: "the body is not re-expanded when the settled note area is shorter than reserved",
        provenance: Provenance::EngineDerived,
        because: "expanding it could only re-admit the line that was just excluded, which re-adds \
                  the note; the cost is a few EMU of white space above the rule, and whether Word \
                  leaves the same gap is exactly the sort of thing only Word can settle",
    },
    Row {
        suite: "a_footnote_moves_the_body",
        subject: "the area sits flush with the foot of the text area, with no gap above it",
        provenance: Provenance::EngineDerived,
        because: "§17.11.16 names `pageBottom` and `beneathText` and says nothing about the space \
                  either leaves, which is the number a reader would notice first",
    },
    Row {
        suite: "a_footnote_moves_the_body",
        subject: "a carried note's continuation is placed before any new note on the page",
        provenance: Provenance::EngineDerived,
        because: "it is the only order that terminates — a new note that pushed the carry off would \
                  carry it for ever — but nothing states that Word orders them this way",
    },
    Row {
        suite: "a_footnote_moves_the_body",
        subject: "a `continuationSeparator` is drawn when a note is carried in and `separator` \
                  otherwise",
        provenance: Provenance::EngineDerived,
        because: "the two reserved entries exist for exactly this and the longer rule is the \
                  continuation one, but which page each belongs on is a convention rather than a \
                  stated rule",
    },
    Row {
        suite: "a_footnote_moves_the_body",
        subject: "a `continuationNotice` is placed in the room left over and dropped when there \
                  is none",
        provenance: Provenance::EngineDerived,
        because: "including its height in the demand would reserve space on every page for a notice \
                  most pages do not need, and reserving it conditionally would make the demand \
                  depend on whether a carry happens — which the demand decides; whether Word makes \
                  room for it instead is exactly the sort of thing only Word can settle",
    },
    Row {
        suite: "columns_balance_at_a_continuous_break",
        subject: "`w:cols@sep` is reported and NOT drawn",
        provenance: Provenance::EngineDerived,
        because: "a rule is a paint and this crate never paints; `w:pBdr/w:between` and a `bar` tab \
                  stop already carry the same decision, and `mjx-scene-docx` (MJXOFF-255) is what \
                  draws all three. Until it exists a reader sees no separator at all",
    },
    Row {
        suite: "a_footnote_moves_the_body",
        subject: "a footnote's reference mark contributes no character and no advance",
        provenance: Provenance::EngineDerived,
        because: "the mark is generated from the note's numbering rather than held in the run \
                  stream, so a line containing one is measured a superscript numeral too narrow \
                  here — R22's field rendering is where that is fixed",
    },
    Row {
        suite: "endnotes_flow_at_the_end_of_their_scope",
        subject: "document-end endnotes are laid out on the LAST section's sheet",
        provenance: Provenance::EngineDerived,
        because: "they come after the last section's content, so they inherit its paper, and a \
                  reader expects the endnote page of a landscape document to be landscape — but \
                  §17.11.3 says only where in the flow they go, not on what",
    },
    Row {
        suite: "endnotes_flow_at_the_end_of_their_scope",
        subject: "an endnote appears once however many times it is referenced",
        provenance: Provenance::EngineDerived,
        because: "a second copy would be absurd, but which reference decides its position when two \
                  sections both name it is not stated; the first one wins here",
    },
    Row {
        suite: "line_numbers_restart_per_mode",
        subject: "`w:countBy` prints the lines whose own number is a multiple of the interval",
        provenance: Provenance::EngineDerived,
        because: "§17.6.10 calls it the increment and does not say what it is measured from; a \
                  section starting at 3 and counting by 5 prints 5 and 10 here and 3, 8, 13 under \
                  the other reading, which is a different set of numbers in the margin",
    },
    Row {
        suite: "line_numbers_restart_per_mode",
        subject: "a line number sits 360 twips from the text when `w:distance` is absent",
        provenance: Provenance::EngineDerived,
        because: "§17.6.10 says an absent value means the numbers are placed automatically, without \
                  saying where automatic is; a quarter of an inch is a guess at Word's own",
    },
    Row {
        suite: "line_numbers_restart_per_mode",
        subject: "`w:suppressLineNumbers` skips a paragraph WITHOUT advancing the count",
        provenance: Provenance::EngineDerived,
        because: "§17.3.1.34 says the lines are not numbered and does not say whether they are \
                  counted; the two readings differ by one on every line after the suppressed \
                  paragraph, which is every number on the rest of the page",
    },
    Row {
        suite: "line_numbers_restart_per_mode",
        subject: "a line number is drawn in the body's own face at the body's own size",
        provenance: Provenance::EngineDerived,
        because: "Word draws it in a style of its own (`LineNumber`), which this crate does not \
                  resolve; the number is therefore the right number in the wrong face until a \
                  style tier for generated marks exists",
    },
    Row {
        suite: "a_section_changes_the_page",
        subject: "every `ST_NumberFormat` outside the five implemented falls back to decimal",
        provenance: Provenance::EngineDerived,
        because: "sixty-three members and no committed face for the East Asian, Hebrew, Thai or \
                  Vietnamese digits; a document asking for `ideographDigital` gets `3` rather than \
                  三, which is wrong and at least legible",
    },


    // ---------------------------------------------------------------------------------------
    // MJXOFF-176 (R21) — tables and floating objects.
    //
    // **The weakest body of evidence in this crate**, and that is a fact about the subject. There is
    // no external standard for text wrapping at all: ECMA-376 names the five `wp:wrap*` elements and
    // the four `ST_WrapText` values and says almost nothing about what a renderer does with any of
    // them, and the two table layout algorithms are named in a sentence each and defined nowhere. So
    // the `SpecCode` rows below are attributes and defaults, the `DocumentedBehaviour` ones are
    // arithmetic or an algorithm defined outside this repository, and everything about *where a
    // line actually goes beside an object* is `EngineDerived`.
    // ---------------------------------------------------------------------------------------
    Row {
        suite: "a_table_splits_across_a_page",
        subject: "`w:tblHeader` repeats a row at the top of every page the table spans",
        provenance: Provenance::SpecCode,
        because: "§17.4.19 states exactly that, and it is invisible unless the table splits",
    },
    Row {
        suite: "a_table_splits_across_a_page",
        subject: "`w:cantSplit` keeps a row's content on one page",
        provenance: Provenance::SpecCode,
        because: "§17.4.6; the row moves whole rather than breaking",
    },
    Row {
        suite: "a_table_grid_is_solved",
        subject: "`w:trHeight@hRule=\"atLeast\"` raises a short row and `\"exact\"` fixes it",
        provenance: Provenance::SpecCode,
        because: "§17.4.80 and `ST_HeightRule`'s own three values",
    },
    Row {
        suite: "a_table_grid_is_solved",
        subject: "`w:gridSpan` covers that many grid columns",
        provenance: Provenance::SpecCode,
        because: "§17.4.17, whose own default is one",
    },
    Row {
        suite: "a_table_grid_is_solved",
        subject: "an absent `w:tblLayout` is auto-fit",
        provenance: Provenance::SpecCode,
        because: "§17.4.52 states `autofit` as the default for `ST_TblLayoutType`",
    },
    Row {
        suite: "a_table_grid_is_solved",
        subject: "`w:tblW@type=\"pct\"` is in fiftieths of a percent",
        provenance: Provenance::SpecCode,
        because: "§17.4.86; the one place in WordprocessingML a percentage is not thousandths",
    },
    Row {
        suite: "text_wraps_around_a_float",
        subject: "`wp:wrapNone` displaces no text",
        provenance: Provenance::SpecCode,
        because: "§20.4.2.10: the object is behind or in front of the text and text flows over it",
    },
    Row {
        suite: "text_wraps_around_a_float",
        subject: "`wp:wrapTopAndBottom` leaves no text beside the object",
        provenance: Provenance::SpecCode,
        because: "§20.4.2.11 states that text is above and below the object and never beside it",
    },
    Row {
        suite: "text_wraps_around_a_float",
        subject: "`wp:wrapThrough` admits text into the polygon and `wp:wrapTight` does not",
        provenance: Provenance::SpecCode,
        because: "§20.4.2.15 against §20.4.2.17 — the two elements share a content model and \
                  differ in exactly that sentence",
    },
    Row {
        suite: "text_wraps_around_a_float",
        subject: "an even-odd scanline of a polygon is exact between its vertex rows",
        provenance: Provenance::DocumentedBehaviour,
        because: "a polygon's edges are straight, so a crossing's x is linear in y and its extremes \
                  on an interval are at the interval's ends — which is why sampling the band's own \
                  edges and every vertex inside it finds every extreme there is",
    },
    Row {
        suite: "a_table_grid_is_solved",
        subject: "auto-fit distributes the slack in proportion to each column's own range",
        provenance: Provenance::DocumentedBehaviour,
        because: "the automatic table layout algorithm HTML defines, whose distribution is the \
                  unique assignment putting every column the same fraction of the way from its \
                  minimum to its maximum",
    },
    Row {
        suite: "a_table_grid_is_solved",
        subject: "the solved columns sum exactly to the table's stated width",
        provenance: Provenance::DocumentedBehaviour,
        because: "arithmetic: integer division loses a few EMU per column and the remainder has to \
                  go somewhere, or every table is a hairline narrow at its right rule",
    },
    Row {
        suite: "text_wraps_around_a_float",
        subject: "a `wp:wrapPolygon`'s coordinates are 21600ths of the object's extent",
        provenance: Provenance::EngineDerived,
        because: "§20.4.2.16 types the points as `a:CT_Point2D`, which is EMU, and Word writes the \
                  shape's own 0..21600 drawing space instead; 21600 EMU is 0.06 mm, so the EMU \
                  reading gives a wrap outline nobody could have authored. Read as relative when \
                  every coordinate is inside that range and the object is larger than it. **The \
                  first thing a sitting should check.**",
    },
    Row {
        suite: "text_wraps_around_a_float",
        subject: "`wrapText=\"largest\"` gives a tie to the left run",
        provenance: Provenance::EngineDerived,
        because: "`ST_WrapText` names the value and says nothing about a tie; a tie means the object \
                  is centred, and Word's own dialog labels that case \"left only\" — a reading, not \
                  a fact",
    },
    Row {
        suite: "text_wraps_around_a_float",
        subject: "a line is composed against one free run and never jumps a float",
        provenance: Provenance::EngineDerived,
        because: "no part of ECMA-376 says whether text may continue on the far side of an object \
                  on the same line; it does not in any renderer anyone has looked at, but that is \
                  observation rather than specification",
    },
    Row {
        suite: "text_wraps_around_a_float",
        subject: "a line's band is composed at most twice before it is accepted",
        provenance: Provenance::EngineDerived,
        because: "the height/width cycle is real and Word's own iteration count is not documented; \
                  two is the smallest bound that gets the common case exactly right, and the error \
                  it leaves is one line's height and is visible in `LaidOutLine::measure`",
    },
    Row {
        suite: "text_wraps_around_a_float",
        subject: "a float pushed past by a `topAndBottom` band grows the line box above the text",
        provenance: Provenance::EngineDerived,
        because: "the alternative is a block of empty space between two lines, which paginates \
                  differently at a page boundary; nothing in the specification chooses between them",
    },
    Row {
        suite: "text_wraps_around_a_float",
        subject: "`relativeFrom=\"character\"` is read as the paragraph's own text start",
        provenance: Provenance::EngineDerived,
        because: "a run's x is not knowable before the line it lands on is composed, and that line \
                  depends on this object — a genuine cycle, cut at the paragraph because that is \
                  the nearest position that is settled",
    },
    Row {
        suite: "a_table_grid_is_solved",
        subject: "a spanned cell's width demand is shared equally across the columns it covers",
        provenance: Provenance::EngineDerived,
        because: "ECMA-376 says nothing about how a `w:gridSpan` contributes to an auto-fit solve; \
                  equal shares is the only distribution that does not depend on an order the file \
                  never states",
    },
    Row {
        suite: "a_table_splits_across_a_page",
        subject: "a row splits at the union of its cells' line bottoms",
        provenance: Provenance::EngineDerived,
        because: "no part of the specification says where inside a row a page break may fall; \
                  cutting at a height every cell has a whole line above is the only rule that never \
                  clips text, and Word's own answer is unchecked",
    },
    Row {
        suite: "a_table_splits_across_a_page",
        subject: "a vertically merged cell's height deficit goes to the last row of its group",
        provenance: Provenance::EngineDerived,
        because: "ECMA-376 is entirely silent; putting it on the anchor's own row makes one deep \
                  row followed by thin ones, and distributing it evenly moves every rule in the \
                  group",
    },
    Row {
        suite: "a_table_grid_is_solved",
        subject: "an absent `w:tblCellMar` is 0 / 0 / 115 / 115 twips",
        provenance: Provenance::EngineDerived,
        because: "no default is stated anywhere; 115 twips is what every table Word creates writes \
                  explicitly, and no side margin at all sets text against the cell's own rules",
    },
    Row {
        suite: "a_table_grid_is_solved",
        subject: "`w:tblInd` and `w:jc` do not compose",
        provenance: Provenance::EngineDerived,
        because: "§17.4.64 calls `w:tblInd` an indent from the leading margin and does not say \
                  whether a centred table is then indented as well; adding them moves a centred, \
                  indented table twice",
    },
    Row {
        suite: "a_table_splits_across_a_page",
        subject: "a table states no space before or after itself",
        provenance: Provenance::EngineDerived,
        because: "`w:tbl` has nothing analogous to `w:spacing`, so the gap above a table is whatever \
                  the paragraph before it states; whether Word adds anything of its own is unchecked",
    },
    Row {
        suite: "a_table_grid_is_solved",
        subject: "`w:vAlign` shifts a split row's content by the same amount on every page",
        provenance: Provenance::EngineDerived,
        because: "§17.4.83 states the three values and says nothing about a row split across a                   page; shifting each slice independently would move the text at the boundary,                   which is the one thing a reader of a split table would notice",
    },
    Row {
        suite: "a_table_grid_is_solved",
        subject: "`w:vAlign=\"both\"` is read as `top`",
        provenance: Provenance::EngineDerived,
        because: "`both` justifies a cell's paragraphs vertically, which distributes the space                   *between* them rather than above them; reading it as `top` until something                   distributes it keeps the text where the file's first line puts it",
    },
    Row {
        suite: "a_table_splits_across_a_page",
        subject: "`w:widowControl` does not apply to a table's rows",
        provenance: Provenance::EngineDerived,
        because: "the rule is defined for lines of a paragraph and a row is not a line; refusing to \
                  leave one row at a page foot is `w:cantSplit`'s job and the author says which rows",
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
        spec >= 34,
        "the specification really does state this many of them: {spec}"
    );
    assert!(
        documented >= 21,
        "these are the rows that are evidence, and there must be some: {documented}"
    );
    assert!(
        engine >= 53,
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
