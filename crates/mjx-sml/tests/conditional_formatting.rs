//! **MJXOFF-120's markup gate.** Conditional formatting: the rule kinds, the cross-block priority
//! order, the `dxf` layer beside a cell's base format, and the `x14` extensions that must survive
//! untouched.
//!
//! # The fixture is authored to make one specific wrong answer visible
//!
//! `tests/fixtures/conditional_formatting.xlsx` carries **four `conditionalFormatting` blocks whose
//! priorities interleave**. That is the whole point of it, and it is the trap the ticket named: *"a
//! fixture with one block per priority range tests nothing"*, because a per-block sort and a
//! cross-block sort produce the same list whenever each block's priorities are already contiguous.
//!
//! | block | `@sqref` | rules, in document order |
//! |---|---|---|
//! | 0 | `B2:B10 D2:D10` — a **multi-range** `ST_Sqref` | priority **1** (`cellIs greaterThan`, `dxfId="0"`), priority **4** (`expression`, `dxfId="1"`) |
//! | 1 | `B2:D5` | priority **2** (`colorScale`, three stops) |
//! | 2 | `A1:D20` | priority **3** (`dataBar`, `stopIfTrue="1"`, and an `x14` `extLst`) |
//! | 3 | `B2:B3` | priority **2** again (`cellIs lessThan`), priority **7** (`iconSet 4Rating`) |
//!
//! `B2` is covered by all four. The correct chain is
//!
//! ```text
//! priority 1 (block 0, rule 0)   priority 3 (block 2, rule 0)
//! priority 2 (block 1, rule 0)   priority 4 (block 0, rule 1)
//! priority 2 (block 3, rule 0)   priority 7 (block 3, rule 1)
//! ```
//!
//! — an order in which **consecutive rules come from different blocks, and one block appears at both
//! ends**. A sort performed per block and then concatenated produces `1, 4, 2, 3, 2, 7`, which is a
//! different list; [`the_interleaved_blocks_report_one_chain_ordered_across_all_of_them`] is written
//! against the numbers, so it goes red on that mutation and is shown to.
//!
//! Three further things are in the fixture on purpose:
//!
//! * **a duplicate priority** — `2` appears in block 1 and again in block 3 — so the stable sort has
//!   something to be stable about; and
//! * **a gap** — nothing has priority 5 or 6 — so a write path that renumbered densely would change
//!   the file and be caught by the byte-identity assertion; and
//! * **an `x14` `extLst` in two places**: inside the `dataBar` rule, and at the worksheet's own rank
//!   38. Neither is modelled and both must come back byte for byte, prefix included.
//!
//! # Nothing here evaluates a condition
//!
//! Every assertion below is about *which rules apply* and *what each states*. None is about whether
//! a rule is true, because no call in this workspace can answer that. `stopIfTrue` is asserted as a
//! reported **position**, never as a truncation.

use mjx_ooxml_core::RawDocument;
use mjx_ooxml_types::spreadsheetml::{
    ConditionalFormatType, ConditionalFormatValueObjectType, ConditionalFormattingOperator,
    IconSetType,
};
use mjx_opc::{Package, PartName};
use mjx_sml::styles::effective::CellFormatResolver;
use mjx_sml::{
    CellRangeList, CellReference, ColorScaleSpec, ConditionalFormatting,
    ConditionalFormattingFormula, ConditionalFormattingRule, ConditionalRuleSpec,
    ConditionalRuleSpecKind, ConditionalValueObjectSpec, DataBarSpec, DifferentialFormat,
    IconSetSpec, PatternFillSpec, SmlError, StylesheetPart, WorksheetPart,
};

/// The fixture this whole suite is written against.
const FIXTURE: &str = "conditional_formatting.xlsx";

/// The bytes of one part of one committed fixture.
fn part_bytes(fixture: &str, part: &str) -> Vec<u8> {
    let bytes = mjx_fixtures::fixture(fixture);
    let package = Package::open(&bytes).expect("the fixture opens");
    let name = PartName::new(part).expect("a part name");
    package
        .part_bytes(&name)
        .expect("the part is there")
        .to_vec()
}

/// The fixture's own worksheet bytes.
fn sheet_bytes() -> Vec<u8> {
    part_bytes(FIXTURE, "/xl/worksheets/sheet1.xml")
}

/// Reads a worksheet part, insisting that it is one.
fn read(bytes: &[u8]) -> WorksheetPart {
    WorksheetPart::read_part(bytes)
        .expect("the worksheet reads")
        .expect("the root is an x:worksheet")
}

/// The fixture's worksheet, read.
fn sheet() -> WorksheetPart {
    read(&sheet_bytes())
}

/// The fixture's styles part, parsed and modelled.
fn stylesheet() -> (RawDocument, StylesheetPart) {
    let bytes = part_bytes(FIXTURE, "/xl/styles.xml");
    let document = mjx_xml::fidelity::parse(&bytes).expect("the styles part parses");
    let part = StylesheetPart::read_part(&document)
        .expect("the part reads")
        .expect("the root is an x:styleSheet");
    (document, part)
}

/// A worksheet part around `body`, in the SpreadsheetML namespace under the `x` prefix.
fn worksheet(body: &str) -> WorksheetPart {
    let markup = format!(
        "<x:worksheet xmlns:x=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
         {body}</x:worksheet>"
    );
    read(markup.as_bytes())
}

/// A cell reference, or a panic naming it.
fn cell(text: &str) -> CellReference {
    CellReference::parse(text).unwrap_or_else(|error| panic!("{text} is a cell reference: {error}"))
}

/// The chain for one cell as `(block index, rule index, priority)`, which is what every ordering
/// assertion here is written against.
fn chain_of(sheet: &WorksheetPart, reference: &str) -> Vec<(usize, usize, i32)> {
    sheet
        .conditional_rules_for(cell(reference))
        .expect("the chain resolves")
        .rules()
        .iter()
        .map(|applied| {
            (
                applied.block_index(),
                applied.rule_index(),
                applied.priority(),
            )
        })
        .collect()
}

// -----------------------------------------------------------------------------------------------
// The fixture itself
// -----------------------------------------------------------------------------------------------

/// The fixture must actually be in the corpus, or every byte-identity suite below it is vacuous.
#[test]
fn the_fixture_is_in_the_directory_derived_corpus() {
    assert!(
        mjx_fixtures::package_fixtures_with_extension("xlsx").contains(&FIXTURE.to_owned()),
        "{FIXTURE} is not in the committed corpus, so the byte-identity tiers never see it"
    );
}

/// **The fixture is discriminating, and this is what says so.**
///
/// It asserts the table in this file's own documentation against the file, before anything else
/// runs: four blocks, and priorities that interleave rather than falling in one contiguous run per
/// block. An edit that quietly gave each block its own priority range would fail *here*, naming the
/// block, rather than turning the ordering assertions below into tautologies.
#[test]
fn the_fixtures_priorities_really_do_interleave_across_its_blocks() {
    let sheet = sheet();
    let interner = sheet.interner();
    let per_block: Vec<Vec<i32>> = sheet
        .conditional_formatting_blocks()
        .map(|block| {
            block
                .rules()
                .map(|rule| {
                    rule.priority(interner)
                        .expect("every rule states a priority")
                })
                .collect()
        })
        .collect();

    assert_eq!(
        per_block,
        vec![vec![1, 4], vec![2], vec![3], vec![2, 7]],
        "the fixture no longer interleaves; a per-block sort would now agree with a cross-block one"
    );

    // The two properties the table above claims, stated as assertions rather than as prose.
    assert!(
        per_block[0][0] < per_block[1][0] && per_block[0][1] > per_block[2][0],
        "block 0 must straddle the other blocks' priorities, or nothing distinguishes the sorts"
    );
    assert!(
        per_block[1][0] == per_block[3][0],
        "a duplicate priority is what gives the stable sort something to be stable about"
    );
}

// -----------------------------------------------------------------------------------------------
// The gate: one chain, ordered across every block
// -----------------------------------------------------------------------------------------------

/// **The gate this child is decided by.** A cell covered by all four blocks reports one chain, in
/// priority order, whose consecutive entries come from different blocks.
///
/// The expected list is spelled out in full rather than derived, so that it cannot agree with the
/// implementation by construction. Sorting per block and concatenating produces
/// `1, 4, 2, 3, 2, 7`; this asserts `1, 2, 2, 3, 4, 7`.
#[test]
fn the_interleaved_blocks_report_one_chain_ordered_across_all_of_them() {
    assert_eq!(
        chain_of(&sheet(), "B2"),
        vec![
            (0, 0, 1), // block 0's first rule
            (1, 0, 2), // block 1's only rule
            (3, 0, 2), // block 3's first rule — the duplicate priority, read later
            (2, 0, 3), // block 2's only rule
            (0, 1, 4), // block 0 again, at the other end of the chain
            (3, 1, 7), // block 3's second rule
        ],
        "the chain must be ordered across every block, not block by block"
    );
}

/// A cell only some blocks cover gets only those blocks' rules — which is what makes the `@sqref`
/// filter load-bearing rather than decorative.
///
/// `D10` is in block 0's **second** range (`D2:D10`, the half of the multi-range `sqref` that a
/// single-range parser would miss) and in block 2's, and in neither of the others.
#[test]
fn a_cell_only_some_blocks_cover_gets_only_those_blocks_rules() {
    assert_eq!(
        chain_of(&sheet(), "D10"),
        vec![(0, 0, 1), (2, 0, 3), (0, 1, 4)],
        "only the blocks whose sqref covers D10 may contribute"
    );
}

/// A cell no block covers has an empty chain — not an error, and not the whole sheet's rules.
#[test]
fn a_cell_no_block_covers_has_an_empty_chain() {
    let sheet = sheet();
    let chain = sheet
        .conditional_rules_for(cell("F30"))
        .expect("an uncovered cell is a question, not a failure");
    assert!(chain.is_empty());
    assert_eq!(chain.len(), 0);
    assert_eq!(chain.first_stopping_rule(), None);
}

/// Rules sharing a priority keep the order they were read in: block by block, then rule by rule.
///
/// The file states no other order for them, so any reordering would be this library choosing which
/// of two equal rules wins.
#[test]
fn rules_sharing_a_priority_keep_document_order() {
    let chain = chain_of(&sheet(), "B2");
    let twos: Vec<(usize, usize)> = chain
        .iter()
        .filter(|(_, _, priority)| *priority == 2)
        .map(|(block, rule, _)| (*block, *rule))
        .collect();
    assert_eq!(
        twos,
        vec![(1, 0), (3, 0)],
        "block 1's rule was read before block 3's and must stay in front of it"
    );
}

/// The priorities come back exactly as the file wrote them — the gap between 4 and 7 included.
///
/// Nothing renumbers. §18.3.1.10 says lower wins; it does not say the numbers are dense, and Excel's
/// own files are not.
#[test]
fn the_priorities_come_back_with_their_gap_and_their_duplicate_intact() {
    let priorities: Vec<i32> = chain_of(&sheet(), "B2")
        .into_iter()
        .map(|(_, _, priority)| priority)
        .collect();
    assert_eq!(priorities, vec![1, 2, 2, 3, 4, 7]);
}

/// `stopIfTrue` is reported as a **position**, and the chain is not truncated at it.
///
/// §18.3.1.10 makes the stop conditional on the rule evaluating to true, and nothing here evaluates
/// anything — so every applicable rule is still listed, and the position says where a consumer that
/// *can* evaluate would first consider stopping.
#[test]
fn the_stop_is_reported_as_a_position_and_the_chain_is_not_truncated() {
    let sheet = sheet();
    let chain = sheet
        .conditional_rules_for(cell("B2"))
        .expect("the chain resolves");
    assert_eq!(
        chain.first_stopping_rule(),
        Some(3),
        "the dataBar rule at priority 3 is the first carrying stopIfTrue"
    );
    assert_eq!(
        chain.len(),
        6,
        "the chain lists every applicable rule; truncating at the stop would assert it fired"
    );
    let interner = sheet.interner();
    let stops: Vec<bool> = chain
        .rules()
        .iter()
        .map(|applied| {
            applied
                .stops_lower_priority_rules(interner)
                .expect("the flag reads")
        })
        .collect();
    assert_eq!(stops, vec![false, false, false, true, false, false]);
}

// -----------------------------------------------------------------------------------------------
// The rule kinds
// -----------------------------------------------------------------------------------------------

/// Every modelled rule kind reads back as the file wrote it — attributes, formulas and children.
#[test]
fn every_rule_kind_reads_back_as_the_file_wrote_it() {
    let sheet = sheet();
    let interner = sheet.interner();
    let blocks: Vec<&ConditionalFormatting> = sheet.conditional_formatting_blocks().collect();
    assert_eq!(blocks.len(), 4);

    // Block 0, rule 0 — cellIs, with an operator and one formula.
    let comparison = blocks[0].rules().next().expect("block 0 has a rule");
    assert_eq!(
        comparison.kind(interner).expect("the type reads"),
        Some(ConditionalFormatType::CellIs)
    );
    assert_eq!(
        comparison.operator(interner).expect("the operator reads"),
        Some(ConditionalFormattingOperator::GreaterThan)
    );
    assert_eq!(
        comparison
            .differential_format_index(interner)
            .expect("the dxfId reads"),
        Some(0)
    );
    let operands: Vec<&str> = comparison
        .formulas()
        .map(ConditionalFormattingFormula::text)
        .collect();
    assert_eq!(operands, vec!["1000"]);

    // Block 0, rule 1 — expression, whose formula the file wrote with entity references.
    let expression = blocks[0].rules().nth(1).expect("block 0 has a second rule");
    assert_eq!(
        expression
            .formulas()
            .next()
            .map(ConditionalFormattingFormula::text),
        Some("AND($B2>0,$B2<=\"500\")"),
        "the decoded text is what the accessor answers; the bytes are another matter entirely"
    );

    // Block 1 — a three-stop colour scale, one of whose colours is a theme position with a tint.
    let scale = blocks[1]
        .rules()
        .next()
        .expect("block 1 has a rule")
        .color_scale()
        .expect("it is a colorScale rule");
    assert!(scale.is_balanced(), "three stops and three colours");
    let stops: Vec<(ConditionalFormatValueObjectType, Option<String>)> = scale
        .thresholds()
        .map(|threshold| {
            (
                threshold.value_kind(interner).expect("the type reads"),
                threshold
                    .value(interner)
                    .expect("the value reads")
                    .map(|value| value.into_owned()),
            )
        })
        .collect();
    assert_eq!(
        stops,
        vec![
            (
                ConditionalFormatValueObjectType::Minimum,
                Some("0".to_owned())
            ),
            (
                ConditionalFormatValueObjectType::Percentile,
                Some("50".to_owned())
            ),
            (
                ConditionalFormatValueObjectType::Maximum,
                Some("0".to_owned())
            ),
        ]
    );
    let middle = scale
        .colors()
        .nth(1)
        .expect("three colours")
        .color(interner);
    assert_eq!(
        (middle.theme, middle.tint, middle.rgb.as_deref()),
        (Some(5), Some(0.399_975_585_192_419_2), None),
        "SpreadsheetML's theme is a position and its tint has no DrawingML representation at all"
    );

    // Block 2 — a data bar, whose three attributes the file states away from their defaults.
    let bar = blocks[2]
        .rules()
        .next()
        .expect("block 2 has a rule")
        .data_bar()
        .expect("it is a dataBar rule");
    assert_eq!(bar.minimum_length(interner).expect("minLength reads"), 5);
    assert_eq!(bar.maximum_length(interner).expect("maxLength reads"), 95);
    assert!(!bar.shows_cell_value(interner).expect("showValue reads"));
    assert_eq!(bar.thresholds().count(), 2);
    assert!(bar.color().is_some());

    // Block 3, rule 1 — an icon set, with a non-default `@iconSet` and a `@gte="0"` threshold.
    let icons = blocks[3]
        .rules()
        .nth(1)
        .expect("block 3 has a second rule")
        .icon_set()
        .expect("it is an iconSet rule");
    assert_eq!(
        icons.icons(interner).expect("the iconSet reads"),
        IconSetType::FourRatings
    );
    assert!(icons.icons_are_reversed(interner).expect("reverse reads"));
    let inclusive: Vec<bool> = icons
        .thresholds()
        .map(|threshold| {
            threshold
                .is_greater_than_or_equal(interner)
                .expect("gte reads")
        })
        .collect();
    assert_eq!(
        inclusive,
        vec![true, false, true, true],
        "gte defaults to true and the second threshold states it false"
    );
}

/// The generated `ST_IconSetType` really is what carries the digit-leading wire tokens, and the
/// schema's own default among them.
///
/// Nothing in `mjx-sml` writes a table of icon-set names; this is the assertion that says the
/// generated one is what a rule is read and written through.
#[test]
fn the_icon_set_default_is_the_generated_variant_for_the_digit_leading_token() {
    assert_eq!(IconSetType::ThreeTrafficLights.to_wire(), "3TrafficLights1");
    assert_eq!(
        IconSetType::from_wire("3TrafficLights1"),
        Some(IconSetType::ThreeTrafficLights)
    );
    // An `iconSet` writing no `@iconSet` means the schema default, and the accessor says so.
    let sheet = worksheet(
        "<x:conditionalFormatting sqref=\"A1\"><x:cfRule type=\"iconSet\" priority=\"1\">\
         <x:iconSet><x:cfvo type=\"percent\" val=\"0\"/><x:cfvo type=\"percent\" val=\"50\"/>\
         </x:iconSet></x:cfRule></x:conditionalFormatting>",
    );
    let icons = sheet
        .conditional_formatting_blocks()
        .next()
        .and_then(|block| block.rules().next())
        .and_then(ConditionalFormattingRule::icon_set)
        .expect("the icon set reads");
    assert_eq!(
        icons.icons(sheet.interner()).expect("the default applies"),
        IconSetType::ThreeTrafficLights
    );
}

// -----------------------------------------------------------------------------------------------
// Byte identity, at the markup tier
// -----------------------------------------------------------------------------------------------

/// The markup tier's round trip: read the whole part, write it straight back, get the file's bytes.
///
/// This is the tier the package one cannot stand in for. `mjx-opc` re-emits a stored part without
/// looking inside it, so it is green for a worksheet this crate never parsed; here every rule,
/// every threshold and every `x14` extension has been through the model.
#[test]
fn the_fixture_re_emits_byte_for_byte_through_the_markup_model() {
    assert_eq!(
        sheet().to_markup(),
        sheet_bytes(),
        "the worksheet did not come back byte-identical through the markup model"
    );
}

/// The negative for the assertion above: it is shown to fail when one byte of one priority changes,
/// so a green run means the rules really were compared.
#[test]
fn one_changed_priority_byte_is_caught() {
    let mut mutated = sheet_bytes();
    let at = mutated
        .windows(13)
        .position(|window| window == b"priority=\"7\"\x20")
        .or_else(|| {
            mutated
                .windows(12)
                .position(|window| window == b"priority=\"7\"")
        })
        .expect("the icon-set rule's priority is in the fixture");
    mutated[at + 10] = b'8';
    assert_ne!(
        read(&mutated).to_markup(),
        sheet_bytes(),
        "a changed priority must change the bytes, or the round trip proves nothing"
    );
}

/// A cell edit rewrites `sheetData` and leaves every conditional-formatting block — and both `x14`
/// extensions — byte for byte where they were.
///
/// **This is the `x14` gate.** The extension namespace carries the modern conditional formats, is
/// not modelled here, and must survive an unrelated edit with its prefix, its `uri`, its attribute
/// order and its GUIDs intact.
#[test]
fn an_unrelated_edit_leaves_the_blocks_and_both_x14_extensions_byte_identical() {
    let original = sheet_bytes();
    let mut sheet = sheet();
    sheet
        .set_cell_value(cell("C3"), mjx_sml::CellValue::Number(1234.5))
        .expect("the cell writes");
    let rebuilt = sheet.to_markup();
    assert_ne!(
        rebuilt, original,
        "the edit must actually have changed something"
    );

    for fragment in [
        // The whole of the rule-level `x14` extension, prefix and GUIDs included.
        "<extLst><ext uri=\"{B025F937-C7B1-47D3-B67F-A62EFF666E3E}\" \
         xmlns:x14=\"http://schemas.microsoft.com/office/spreadsheetml/2009/9/main\">\
         <x14:id>{0E7C1A5B-8B44-4E2C-9D3F-1F5A0C6D2B71}</x14:id></ext></extLst>",
        // The worksheet-level one, at rank 38, with the second foreign prefix `xm` inside it.
        "<x14:negativeFillColor rgb=\"FFFF0000\"/>",
        "<xm:sqref>A1:D20</xm:sqref>",
        // And every block, verbatim.
        "<conditionalFormatting sqref=\"B2:B10 D2:D10\">",
        "<iconSet iconSet=\"4Rating\" reverse=\"1\">",
        "<cfvo type=\"percent\" val=\"25\" gte=\"0\"/>",
    ] {
        assert!(
            String::from_utf8_lossy(&rebuilt).contains(fragment),
            "an unrelated edit lost or rewrote: {fragment}"
        );
    }

    // And, precisely: everything from the first block to the end of the part is untouched.
    let tail = |bytes: &[u8]| {
        let text = String::from_utf8_lossy(bytes).into_owned();
        let at = text
            .find("<conditionalFormatting")
            .expect("the part has a conditionalFormatting block");
        text[at..].to_owned()
    };
    assert_eq!(
        tail(&rebuilt),
        tail(&original),
        "every byte after sheetData must be the file's own"
    );
}

// -----------------------------------------------------------------------------------------------
// Placement — rank 16, between held slots
// -----------------------------------------------------------------------------------------------

/// A new block lands at rank 16: **after** a held `phoneticPr` (15) and **before** a held
/// `dataValidations` (17), neither of which this crate models.
///
/// This is the defect MJXOFF-117 fixed, restated for the slot that sits deepest inside the held
/// range. A frame that reported `None` for an unmodelled child's rank would put the new block at the
/// wrong end of both.
#[test]
fn a_new_block_lands_at_rank_sixteen_between_two_held_slots() {
    let mut sheet = worksheet(
        "<x:sheetData/><x:phoneticPr fontId=\"0\"/>\
         <x:dataValidations count=\"1\"><x:dataValidation sqref=\"A1\"/></x:dataValidations>\
         <x:pageMargins left=\"0.7\" right=\"0.7\" top=\"0.75\" bottom=\"0.75\" header=\"0.3\" \
         footer=\"0.3\"/>",
    );
    let prefix = sheet.element_prefix().map(str::to_owned);
    let block = {
        let interner = sheet.interner_mut();
        let mut block = ConditionalFormatting::new(interner, prefix.as_deref());
        block.set_ranges(
            interner,
            Some(CellRangeList::parse("A1:B2").expect("a sqref")),
        );
        let mut rule = ConditionalFormattingRule::new(interner, prefix.as_deref());
        rule.set_kind(interner, Some(ConditionalFormatType::ContainsBlanks));
        rule.set_priority(interner, 1);
        block.push_rule(rule);
        block
    };
    sheet.push_conditional_formatting(block);

    assert_eq!(
        sheet.child_element_locals().collect::<Vec<_>>(),
        vec![
            "sheetData",
            "phoneticPr",
            "conditionalFormatting",
            "dataValidations",
            "pageMargins",
        ],
        "rank 16 sits between two slots this crate holds rather than models"
    );
}

/// A second block appends **after** the first, because the slot is `maxOccurs="unbounded"` and the
/// order of the blocks is part of the file.
#[test]
fn a_second_block_appends_after_the_first() {
    let mut sheet = sheet();
    let prefix = sheet.element_prefix().map(str::to_owned);
    let block = {
        let interner = sheet.interner_mut();
        let mut block = ConditionalFormatting::new(interner, prefix.as_deref());
        block.set_ranges(
            interner,
            Some(CellRangeList::parse("Z1:Z9").expect("a sqref")),
        );
        let mut rule = ConditionalFormattingRule::new(interner, prefix.as_deref());
        rule.set_kind(interner, Some(ConditionalFormatType::UniqueValues));
        rule.set_priority(interner, 9);
        block.push_rule(rule);
        block
    };
    sheet.push_conditional_formatting(block);

    assert_eq!(sheet.conditional_formatting_block_count(), 5);
    let last = sheet
        .conditional_formatting_blocks()
        .last()
        .expect("five blocks");
    assert_eq!(
        last.ranges(sheet.interner())
            .expect("the sqref reads")
            .map(|list| list.to_string()),
        Some("Z1:Z9".to_owned())
    );
    // And the sheet's element order is still schema order.
    let locals: Vec<&str> = sheet.child_element_locals().collect();
    let first = locals
        .iter()
        .position(|local| *local == "conditionalFormatting")
        .expect("a block");
    assert_eq!(&locals[first..first + 5], &["conditionalFormatting"; 5]);
    assert_eq!(locals[first + 5], "pageMargins");
}

// -----------------------------------------------------------------------------------------------
// Authoring
// -----------------------------------------------------------------------------------------------

/// Authoring a rule appends a `dxf` and **changes no existing `dxfId` anywhere**.
///
/// The fixture's `dxfs` holds two entries and its rules name both by index. Appending a third must
/// leave those two exactly where they were, or every rule in the workbook silently repoints.
#[test]
fn authoring_a_rule_appends_a_dxf_and_changes_no_existing_index() {
    let (document, mut stylesheet) = stylesheet();
    let mut interner = document.interner;

    let before: Vec<String> = stylesheet
        .differential_formats()
        .expect("the fixture writes a dxfs table")
        .formats()
        .map(|format| format!("{format:?}"))
        .collect();
    assert_eq!(before.len(), 2);

    let mut appended = DifferentialFormat::new(&mut interner, None);
    appended.set_fill(Some(
        PatternFillSpec::solid("C6EFCE").build(&mut interner, None),
    ));
    let index = stylesheet.append_differential_format(&mut interner, None, appended);

    assert_eq!(
        index, 2,
        "the new dxf takes the next index, never an existing one"
    );
    let after: Vec<String> = stylesheet
        .differential_formats()
        .expect("the table is still there")
        .formats()
        .map(|format| format!("{format:?}"))
        .collect();
    assert_eq!(after.len(), 3);
    assert_eq!(
        &after[..2],
        &before[..],
        "appending must leave every existing dxf at the index it already had"
    );
    assert!(
        stylesheet
            .differential_formats()
            .and_then(|table| table.get(2))
            .is_some(),
        "and the appended dxf is reachable at the index it was given"
    );
}

/// A `cellIs` rule, a colour scale and a data bar authored onto a range read back as what was asked
/// for — and the part they were written into is still in schema order.
#[test]
fn authoring_the_three_rule_kinds_writes_markup_that_reads_back() {
    let mut sheet = worksheet("<x:sheetData/>");
    let prefix = sheet.element_prefix().map(str::to_owned);
    let specs = [
        ConditionalRuleSpec {
            differential_format_index: Some(2),
            ..ConditionalRuleSpec::cell_is(
                ConditionalFormattingOperator::Between,
                ["10".to_owned(), "20".to_owned()],
                1,
            )
        },
        ConditionalRuleSpec {
            kind: ConditionalRuleSpecKind::ColorScale(ColorScaleSpec::two_color(
                "FFF8696B", "FF63BE7B",
            )),
            priority: 2,
            stops_lower_priority_rules: None,
            differential_format_index: None,
        },
        ConditionalRuleSpec {
            kind: ConditionalRuleSpecKind::DataBar(DataBarSpec::spanning_the_range("638EC6")),
            priority: 3,
            stops_lower_priority_rules: Some(true),
            differential_format_index: None,
        },
        ConditionalRuleSpec {
            kind: ConditionalRuleSpecKind::IconSet(IconSetSpec {
                icons: Some(IconSetType::ThreeArrows),
                thresholds: vec![
                    ConditionalValueObjectSpec::with_value(
                        ConditionalFormatValueObjectType::Percent,
                        "0",
                    ),
                    ConditionalValueObjectSpec::with_value(
                        ConditionalFormatValueObjectType::Percent,
                        "33",
                    ),
                    ConditionalValueObjectSpec::with_value(
                        ConditionalFormatValueObjectType::Percent,
                        "67",
                    ),
                ],
                ..IconSetSpec::default()
            }),
            priority: 4,
            stops_lower_priority_rules: None,
            differential_format_index: None,
        },
    ];

    let block = {
        let interner = sheet.interner_mut();
        let mut block = ConditionalFormatting::new(interner, prefix.as_deref());
        block.set_ranges(
            interner,
            Some(CellRangeList::parse("A1:A5 C1:C5").expect("a sqref")),
        );
        for spec in &specs {
            let rule = spec.build(interner, prefix.as_deref());
            block.push_rule(rule);
        }
        block
    };
    sheet.push_conditional_formatting(block);

    // Read the authored markup back through a fresh parse: what matters is what the bytes say.
    let markup = sheet.to_markup();
    let reread = read(&markup);
    let interner = reread.interner();
    let block = reread
        .conditional_formatting_blocks()
        .next()
        .expect("the authored block is there");
    assert_eq!(
        block
            .ranges(interner)
            .expect("the sqref reads")
            .map(|list| list.to_string()),
        Some("A1:A5 C1:C5".to_owned())
    );
    let rules: Vec<&ConditionalFormattingRule> = block.rules().collect();
    assert_eq!(rules.len(), 4);

    assert_eq!(
        rules[0].kind(interner).expect("a type"),
        Some(ConditionalFormatType::CellIs)
    );
    assert_eq!(
        rules[0].operator(interner).expect("an operator"),
        Some(ConditionalFormattingOperator::Between)
    );
    assert_eq!(
        rules[0]
            .formulas()
            .map(ConditionalFormattingFormula::text)
            .collect::<Vec<_>>(),
        vec!["10", "20"],
        "`between` writes two operands, in order"
    );
    assert_eq!(
        rules[0]
            .differential_format_index(interner)
            .expect("a dxfId"),
        Some(2)
    );

    let scale = rules[1].color_scale().expect("a colour scale");
    assert_eq!(scale.thresholds().count(), 2);
    assert_eq!(scale.colors().count(), 2);
    assert!(scale.is_balanced());

    let bar = rules[2].data_bar().expect("a data bar");
    assert_eq!(bar.thresholds().count(), 2);
    assert!(bar.color().is_some());
    assert!(rules[2]
        .stops_lower_priority_rules(interner)
        .expect("stopIfTrue reads"));

    let icons = rules[3].icon_set().expect("an icon set");
    assert_eq!(
        icons.icons(interner).expect("an iconSet"),
        IconSetType::ThreeArrows
    );
    assert_eq!(icons.thresholds().count(), 3);

    // And the authored markup is in `CT_CfRule`'s own child order: formulas first, then the graded
    // child, which is what the generated table is consulted for.
    let text = String::from_utf8_lossy(&markup).into_owned();
    let formula_at = text
        .find("<x:formula>10</x:formula>")
        .expect("the first operand");
    let scale_at = text.find("<x:colorScale>").expect("the colour scale");
    assert!(formula_at < scale_at);
}

/// An authored block round-trips: writing it out and reading it back gives the same bytes again.
#[test]
fn authored_markup_round_trips_byte_for_byte() {
    let mut sheet = worksheet("<x:sheetData/>");
    let prefix = sheet.element_prefix().map(str::to_owned);
    let block = {
        let interner = sheet.interner_mut();
        let mut block = ConditionalFormatting::new(interner, prefix.as_deref());
        block.set_ranges(
            interner,
            Some(CellRangeList::parse("A1:A5").expect("a sqref")),
        );
        let rule = ConditionalRuleSpec::cell_is(
            ConditionalFormattingOperator::GreaterThan,
            ["\"a & b\"".to_owned()],
            1,
        )
        .build(interner, prefix.as_deref());
        block.push_rule(rule);
        block
    };
    sheet.push_conditional_formatting(block);

    let once = sheet.to_markup();
    let twice = read(&once).to_markup();
    assert_eq!(
        once, twice,
        "an authored block must survive its own round trip"
    );
    // Character data is escaped **minimally** — `&` and `<`, never `"` — which is what
    // `mjx_xml::text::escape_text` does and what a second round trip has to reproduce exactly.
    assert!(
        String::from_utf8_lossy(&once).contains("<x:formula>\"a &amp; b\"</x:formula>"),
        "an authored formula is escaped once, and escaped minimally"
    );
}

// -----------------------------------------------------------------------------------------------
// The `dxf` layer, beside the base format
// -----------------------------------------------------------------------------------------------

/// **The conditional layer is reported alongside the base format, never folded into it.**
///
/// `B2` carries `s="1"` — `cellXfs[1]`, which states `numFmtId="2"` — and is covered by six rules,
/// two of which name a `dxf`. The base format is exactly what the resolver answers with no
/// conditional formatting in the picture; the layer is the six candidates beside it; and there is no
/// call that merges the two, because whether any of the six fires is unknown here.
#[test]
fn the_conditional_layer_is_reported_beside_the_base_format_and_never_folded_in() {
    let (document, stylesheet) = stylesheet();
    let resolver =
        CellFormatResolver::new(&stylesheet, &document.interner).expect("the resolver builds");
    let sheet = sheet();

    let layered = resolver
        .conditional_cell_format(&sheet, cell("B2"), None)
        .expect("the cell resolves in both layers");

    // The base is untouched by conditional formatting: the same value the plain resolver gives.
    let plain = resolver
        .effective_cell_format(
            sheet.cell(cell("B2")).as_ref(),
            sheet.sheet_data().and_then(|cells| cells.row(2)).as_ref(),
            None,
        )
        .expect("the plain resolution succeeds");
    assert_eq!(layered.base(), plain);
    assert_eq!(layered.base().style_index(), 1);
    assert_eq!(
        layered.base().number_format().resource_index,
        Some(2),
        "cellXfs[1] states numFmtId=2, and the conditional layer must not have changed it"
    );

    // The layer is the six candidates, in the same priority order the chain reports.
    let layer: Vec<(i32, Option<u32>, bool)> = layered
        .layer()
        .iter()
        .map(|entry| {
            (
                entry.priority(),
                entry.differential_format_index(),
                entry.differential_format().is_some(),
            )
        })
        .collect();
    assert_eq!(
        layer,
        vec![
            (1, Some(0), true),
            (2, None, false),   // the colour scale imposes no dxf
            (2, Some(0), true), // the duplicate-priority cellIs names dxf 0 as well
            (3, None, false),   // the data bar imposes none
            (4, Some(1), true),
            (7, None, false), // nor does the icon set
        ]
    );
    assert_eq!(layered.first_stopping_rule(), Some(3));

    // And the two `dxf`s really are the fixture's, reached by index — absent members mean
    // *inherited*, which is why a fold would be wrong even if a rule were known to fire.
    let first = layered.layer()[0]
        .differential_format()
        .expect("dxf 0 is there");
    assert!(first.font().is_some(), "dxf 0 states a font");
    assert!(first.fill().is_some(), "dxf 0 states a fill");
    assert!(
        first.border().is_none() && first.number_format().is_none(),
        "and inherits the border and the number format, which a folded answer would have lost"
    );

    let second = layered.layer()[4]
        .differential_format()
        .expect("dxf 1 is there");
    assert!(second.fill().is_some() && second.font().is_none());
}

/// A cell no rule covers still resolves: a base format, and an empty layer.
#[test]
fn an_uncovered_cell_has_a_base_format_and_no_layer() {
    let (document, stylesheet) = stylesheet();
    let resolver =
        CellFormatResolver::new(&stylesheet, &document.interner).expect("the resolver builds");
    let sheet = sheet();
    let layered = resolver
        .conditional_cell_format(&sheet, cell("F30"), None)
        .expect("an uncovered cell resolves");
    assert!(layered.layer().is_empty());
    assert_eq!(layered.base().style_index(), 0);
}

/// A `@dxfId` naming no record is reported as an absence rather than repaired or refused.
#[test]
fn a_dangling_dxf_id_reports_no_differential_format() {
    let (document, stylesheet) = stylesheet();
    let resolver =
        CellFormatResolver::new(&stylesheet, &document.interner).expect("the resolver builds");
    assert_eq!(resolver.differential_format_count(), 2);
    assert!(resolver.differential_format(0).is_some());
    assert!(
        resolver.differential_format(9).is_none(),
        "an index past the table is the file's defect, reported and not invented around"
    );
}

// -----------------------------------------------------------------------------------------------
// What is refused, and what is not
// -----------------------------------------------------------------------------------------------

/// A block with no `@sqref` cannot say which cells it covers, so a query over it is a typed error
/// naming the block — not a silently shortened chain.
#[test]
fn a_block_with_no_sqref_is_a_typed_error_naming_it() {
    let sheet = worksheet(
        "<x:conditionalFormatting sqref=\"A1\"><x:cfRule type=\"expression\" priority=\"1\"/>\
         </x:conditionalFormatting>\
         <x:conditionalFormatting><x:cfRule type=\"expression\" priority=\"2\"/>\
         </x:conditionalFormatting>",
    );
    let error = sheet
        .conditional_rules_for(cell("A1"))
        .expect_err("a block with no sqref cannot be answered around");
    assert!(matches!(
        error,
        SmlError::ConditionalFormattingBlockHasNoRange { block: 1 }
    ));
    // But the markup is preserved, not refused: reading and re-emitting is unaffected.
    assert_eq!(sheet.conditional_formatting_block_count(), 2);
}

/// A rule with no `@priority` — which the schema declares required — is a typed error naming the
/// block and the rule, rather than a made-up number that would decide which rule wins.
#[test]
fn a_rule_with_no_priority_is_a_typed_error_naming_it() {
    let sheet = worksheet(
        "<x:conditionalFormatting sqref=\"A1:B2\">\
         <x:cfRule type=\"expression\" priority=\"1\"/>\
         <x:cfRule type=\"expression\"/></x:conditionalFormatting>",
    );
    let error = sheet
        .conditional_rules_for(cell("A1"))
        .expect_err("a rule with no priority has no place in the order");
    assert!(matches!(
        error,
        SmlError::ConditionalFormattingRuleHasNoPriority { block: 0, rule: 1 }
    ));
}

/// An unbalanced colour scale is described, never repaired: `pairs` yields what exists and
/// `is_balanced` says the file is wrong.
#[test]
fn an_unbalanced_colour_scale_is_described_rather_than_repaired() {
    let sheet = worksheet(
        "<x:conditionalFormatting sqref=\"A1\"><x:cfRule type=\"colorScale\" priority=\"1\">\
         <x:colorScale><x:cfvo type=\"min\"/><x:cfvo type=\"percent\" val=\"50\"/>\
         <x:cfvo type=\"max\"/><x:color rgb=\"FF000000\"/><x:color rgb=\"FFFFFFFF\"/>\
         </x:colorScale></x:cfRule></x:conditionalFormatting>",
    );
    let scale = sheet
        .conditional_formatting_blocks()
        .next()
        .and_then(|block| block.rules().next())
        .and_then(ConditionalFormattingRule::color_scale)
        .expect("the scale reads");
    assert_eq!(scale.thresholds().count(), 3);
    assert_eq!(scale.colors().count(), 2);
    assert!(!scale.is_balanced());
    assert_eq!(
        scale.pairs().count(),
        2,
        "pairing stops at the shorter list"
    );
    // And it re-emits exactly as it stands.
    assert_eq!(
        String::from_utf8_lossy(&sheet.to_markup())
            .matches("<x:cfvo")
            .count(),
        3
    );
}

/// Removing a rule and removing a block take out exactly what was asked for and nothing beside it.
#[test]
fn removing_a_rule_and_a_block_takes_out_only_what_was_named() {
    let mut sheet = sheet();
    let removed = sheet
        .conditional_formatting_block_mut(0)
        .expect("block 0")
        .remove_rule(1)
        .expect("its second rule");
    assert_eq!(removed.priority(sheet.interner()).expect("its priority"), 4);
    assert_eq!(
        chain_of(&sheet, "B2"),
        vec![(0, 0, 1), (1, 0, 2), (3, 0, 2), (2, 0, 3), (3, 1, 7),]
    );

    let block = sheet
        .remove_conditional_formatting(3)
        .expect("the fourth block");
    assert_eq!(block.len(), 2);
    assert_eq!(sheet.conditional_formatting_block_count(), 3);
    assert_eq!(
        chain_of(&sheet, "B2"),
        vec![(0, 0, 1), (1, 0, 2), (2, 0, 3)]
    );
}
