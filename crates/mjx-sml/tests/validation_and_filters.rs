//! **MJXOFF-123's markup gate.** Data validation, autofilters and sort state: the six filter kinds,
//! the extension slot beside them, the formulas that are never resolved, and the three things this
//! cluster must never do.
//!
//! # The fixture is authored to make specific wrong answers visible
//!
//! `tests/fixtures/validation_and_filters.xlsx` is not a file with one filter repeated. The ticket
//! names that trap directly — *"One filter kind repeated four times tests one code path"* — so the
//! fixture carries **every one of `CT_FilterColumn`'s six kinds, one per column, plus a seventh
//! column holding only the choice's `extLst`**:
//!
//! | document position | `@colId` | kind |
//! |---|---|---|
//! | 0 | **1** | `customFilters` — an `and="1"` pair, `>=500` and `<2000` |
//! | 1 | **0** | `filters` — two values, a `dateGroupItem`, `blank="1"`, `calendarType="hijri"` |
//! | 2 | **4** | `top10` — `top="0" percent="1" val="25" filterVal="317.5"` |
//! | 3 | **2** | `dynamicFilter` — `type="M3"` (wire `M3`, generated `March`) |
//! | 4 | **5** | `colorFilter` — `dxfId="1" cellColor="0"` |
//! | 5 | **3** | `iconFilter` — `iconSet="3Symbols"`, which is **`ThreeSymbolsCircled`** |
//! | 6 | **6** | *no filter at all* — only an `x14` `extLst` |
//!
//! Four further things are in it on purpose:
//!
//! * **the `@colId`s do not ascend** — `1, 0, 4, 2, 5, 3, 6` in document order — so a model that
//!   sorted the columns, or that assumed position *is* `colId`, produces a different file and a
//!   different list. [`the_filter_columns_keep_document_order_not_column_order`] pins it;
//! * **row 4 is hidden and row 5 is not**, and the filter above them is exactly the sort of filter
//!   that would hide row 5 if anything here evaluated one. [`no_row_hidden_flag_moves_for_a_filter`]
//!   pins both;
//! * **`dataValidations@count` says 9 and there are 4 rules**, so any write path that "corrected" the
//!   cache would change the file. [`the_stale_validation_count_is_left_exactly_as_the_file_wrote_it`]
//!   pins it;
//! * **two `list` validations sit side by side, one sourced from a range and one from a literal**, so
//!   the range→literal resolution the ticket forbids has something to be visibly absent from.
//!
//! # Why the write-path gate is not a byte-identity test, and how that was found out
//!
//! MJXOFF-123 asks for a mutation that *"resolves a list validation's range source into literal
//! values on write"* and for the **tier-1** byte-identity test to go red. **It cannot**, and
//! MJXOFF-120 proved the same thing one child earlier:
//!
//! * a part nobody edited is one `extend_from_slice` of its own buffer, so the model's writer is
//!   never reached; and
//! * after an edit *elsewhere*, every other slot still writes from **its own** stored bytes, so the
//!   model's writer is still never reached for the untouched `dataValidations`.
//!
//! So a resolution performed in `DataValidationSpec::build` or in `FormulaElement::as_raw_element`
//! is invisible to both byte-identity gates: they are green precisely because the code under test is
//! skipped, which is the epic's §7 signature failure.
//!
//! [`a_validation_re_emitted_from_the_model_keeps_its_range_source`] is the gate that can see it. It
//! goes through the **one door that drops a slot's verbatim bytes** — the `_mut` accessor — and then
//! asserts on what the model emits. `crates/mjx-sml/tests/conditional_formatting.rs` has the same
//! shape for the same reason.
//!
//! # Nothing here filters, sorts, or validates
//!
//! Every assertion below is about what the file *says*. None is about which rows a filter would
//! hide, what order a sort would produce, or whether a cell satisfies a validation — no call in this
//! workspace can answer any of the three.

use mjx_ooxml_types::spreadsheetml::{
    DataValidationErrorStyle, DataValidationImeMode, DataValidationOperator, DataValidationType,
    DateTimeGrouping, DynamicFilterType, FilterOperator, IconSetType, SortBy,
};
use mjx_opc::{Package, PartName};
use mjx_sml::{
    AutoFilter, AutoFilterSpec, CellRange, CellRangeList, CellReference, CustomFilterSpec,
    DataValidation, DataValidationSpec, FilterColumnSpec, FilterKind, FilterSpecKind,
    FormulaElement, SortConditionSpec, SortStateSpec, WorksheetPart,
};

/// The fixture this whole suite is written against.
const FIXTURE: &str = "validation_and_filters.xlsx";

/// The bytes of one part of the fixture.
fn part_bytes(part: &str) -> Vec<u8> {
    let bytes = mjx_fixtures::fixture(FIXTURE);
    let package = Package::open(&bytes).expect("the fixture opens");
    let name = PartName::new(part).expect("a part name");
    package
        .part_bytes(&name)
        .expect("the part is there")
        .to_vec()
}

/// The fixture's own worksheet bytes.
fn sheet_bytes() -> Vec<u8> {
    part_bytes("/xl/worksheets/sheet1.xml")
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

/// A range, or a panic naming it.
fn range(text: &str) -> CellRange {
    CellRange::parse(text).unwrap_or_else(|error| panic!("{text} is a range: {error}"))
}

/// A range list, or a panic naming it.
fn ranges(text: &str) -> CellRangeList {
    CellRangeList::parse(text).unwrap_or_else(|error| panic!("{text} is a sqref: {error}"))
}

/// The fixture's autofilter.
fn auto_filter(sheet: &WorksheetPart) -> &AutoFilter {
    sheet.auto_filter().expect("the sheet has an autoFilter")
}

// -----------------------------------------------------------------------------------------------
// The fixture itself
// -----------------------------------------------------------------------------------------------

/// The fixture must actually be in the corpus, or every byte-identity suite below it is vacuous.
#[test]
fn the_fixture_is_in_the_directory_derived_corpus() {
    assert!(
        mjx_fixtures::package_fixtures()
            .iter()
            .any(|f| f == FIXTURE),
        "{FIXTURE} is not in the directory-derived corpus, so the workspace byte-identity suites \
         never open it"
    );
}

/// **The fixture is discriminating, and this is what says so.**
///
/// It asserts this file's own documentation against the file before anything else runs: seven
/// filter columns, six distinct kinds among them, `@colId`s that do not ascend, a hidden row beside
/// a visible one, and a `@count` that disagrees with the rule count. An edit that quietly made the
/// fixture tidy would fail *here*, naming what changed, rather than turning the assertions below
/// into tautologies.
#[test]
fn the_fixture_really_does_carry_every_kind_out_of_order() {
    let sheet = sheet();
    let filter = auto_filter(&sheet);

    let column_ids: Vec<u32> = filter
        .columns()
        .map(|column| column.column_offset(sheet.interner()).expect("a colId"))
        .collect();
    assert_eq!(
        column_ids,
        vec![1, 0, 4, 2, 5, 3, 6],
        "the fixture's filter columns must be out of ascending order, or the ordering assertions \
         below cannot fail"
    );

    let kinds: Vec<Option<&'static str>> = filter
        .columns()
        .map(|column| column.filter().and_then(FilterKind::local))
        .collect();
    assert_eq!(
        kinds,
        vec![
            Some("customFilters"),
            Some("filters"),
            Some("top10"),
            Some("dynamicFilter"),
            Some("colorFilter"),
            Some("iconFilter"),
            // The seventh column carries only the choice's `extLst`.
            None,
        ],
        "the fixture must carry all six filter kinds and one extension-only column"
    );

    let hidden: Vec<(u32, bool)> = sheet
        .rows()
        .map(|row| (row.number().expect("a row number"), row.is_hidden()))
        .collect();
    assert_eq!(
        hidden,
        vec![
            (1, false),
            (2, false),
            (3, false),
            (4, true),
            (5, false),
            (20, false)
        ],
        "the fixture must hold one hidden row beside a visible one, or the never-hide gate proves \
         nothing"
    );

    let validations = sheet
        .data_validations()
        .expect("the dataValidations element");
    assert_eq!(validations.len(), 4, "four rules");
    assert_eq!(
        validations.count(sheet.interner()).expect("a @count"),
        Some(9),
        "the fixture's `@count` must be stale, or nothing can prove it is left alone"
    );
}

// -----------------------------------------------------------------------------------------------
// The gate: six kinds, six variants, and an extension slot that survives
// -----------------------------------------------------------------------------------------------

/// **One of the two gates this child is decided by.** Each of the six filter kinds reads back as its
/// own variant carrying its own attributes, and the seventh choice member — `extLst` — comes back
/// too rather than being dropped.
///
/// The expected values are spelled out rather than derived, so they cannot agree with the
/// implementation by construction. A model that collapsed the choice to its first matching variant
/// answers `Values` six times over and fails here.
#[test]
fn the_six_filter_kinds_read_back_as_distinct_variants() {
    let sheet = sheet();
    let interner = sheet.interner();
    let columns: Vec<_> = auto_filter(&sheet).columns().collect();
    assert_eq!(columns.len(), 7);

    // 0 — `customFilters`, an `and` pair.
    let FilterKind::Custom(custom) = columns[0].filter().expect("a filter") else {
        panic!("column 0 is a customFilters, not {:?}", columns[0].filter());
    };
    assert!(
        custom.requires_both(interner).expect("@and"),
        "`and=\"1\"` joins the pair with *And*"
    );
    let comparisons: Vec<(FilterOperator, String)> = custom
        .comparisons()
        .map(|filter| {
            (
                filter.operator(interner).expect("@operator"),
                filter
                    .value(interner)
                    .expect("@val")
                    .expect("a value")
                    .into_owned(),
            )
        })
        .collect();
    assert_eq!(
        comparisons,
        vec![
            (FilterOperator::GreaterThanOrEqual, "500".to_owned()),
            (FilterOperator::LessThan, "2000".to_owned()),
        ]
    );

    // 1 — `filters`: the values, the date group, **and the two attributes the ticket omits**.
    let FilterKind::Values(values) = columns[1].filter().expect("a filter") else {
        panic!("column 1 is a filters");
    };
    let literals: Vec<String> = values
        .values()
        .map(|filter| {
            filter
                .value(interner)
                .expect("@val")
                .expect("a value")
                .into_owned()
        })
        .collect();
    assert_eq!(
        literals,
        vec!["North".to_owned(), "South & East".to_owned()]
    );
    assert!(
        values.includes_blanks(interner).expect("@blank"),
        "`@blank` is (Blanks); a model that dropped it loses it from the drop-down"
    );
    assert_eq!(
        values.calendar(interner).expect("@calendarType"),
        mjx_ooxml_types::shared::CalendarType::Hijri,
        "`@calendarType` says which calendar the date groups are in"
    );
    let groups: Vec<(u16, Option<u16>, DateTimeGrouping)> = values
        .date_groups()
        .map(|item| {
            (
                item.year(interner).expect("@year"),
                item.month(interner).expect("@month"),
                item.grouping(interner).expect("@dateTimeGrouping"),
            )
        })
        .collect();
    assert_eq!(groups, vec![(2015, Some(3), DateTimeGrouping::Month)]);

    // 2 — `top10`, with the derived threshold Excel cached beside the count the user asked for.
    let FilterKind::Top10(top10) = columns[2].filter().expect("a filter") else {
        panic!("column 2 is a top10");
    };
    assert_eq!(top10.value(interner).expect("@val"), 25.0);
    assert!(!top10.takes_the_top(interner).expect("@top"), "bottom 25%");
    assert!(top10.is_percentage(interner).expect("@percent"));
    assert_eq!(
        top10.derived_threshold(interner).expect("@filterVal"),
        Some(317.5),
        "`@filterVal` is Excel's cache and is reported, never recomputed"
    );

    // 3 — `dynamicFilter`. The wire token is `M3`; the generated variant is `March`.
    let FilterKind::Dynamic(dynamic) = columns[3].filter().expect("a filter") else {
        panic!("column 3 is a dynamicFilter");
    };
    assert_eq!(
        dynamic.kind(interner).expect("@type"),
        DynamicFilterType::March
    );
    assert_eq!(dynamic.value(interner).expect("@val"), Some(42064.0));
    assert_eq!(
        dynamic.maximum_value(interner).expect("@maxVal"),
        Some(42094.0)
    );

    // 4 — `colorFilter`, which names a `dxf` by **position**.
    let FilterKind::Color(color) = columns[4].filter().expect("a filter") else {
        panic!("column 4 is a colorFilter");
    };
    assert_eq!(
        color.differential_format_index(interner).expect("@dxfId"),
        Some(1)
    );
    assert!(
        !color.is_cell_color(interner).expect("@cellColor"),
        "by font colour"
    );

    // 5 — `iconFilter`. **The naming trap**: the wire token `3Symbols` is `ThreeSymbolsCircled`,
    // and `3Symbols2` is the one called `ThreeSymbols`. Nothing here infers a name from a token.
    let FilterKind::Icon(icon) = columns[5].filter().expect("a filter") else {
        panic!("column 5 is an iconFilter");
    };
    assert_eq!(
        icon.icon_set(interner).expect("@iconSet"),
        IconSetType::ThreeSymbolsCircled
    );
    assert_eq!(icon.icon_index(interner).expect("@iconId"), Some(2));

    // 6 — the extension slot: no modelled filter, and the `extLst` still there.
    assert!(
        columns[6].filter().is_none(),
        "the seventh column carries no modelled filter kind"
    );
    assert!(
        columns[6]
            .content()
            .iter()
            .any(|item| matches!(item, FilterKind::Raw(_))),
        "the seventh member of the choice — `extLst` — must be held, not dropped"
    );
    assert!(columns[6]
        .hides_the_button(interner)
        .expect("@hiddenButton"));
    assert!(!columns[6].shows_the_button(interner).expect("@showButton"));
}

/// The columns come back in **document order**, which the fixture makes differ from `@colId` order.
#[test]
fn the_filter_columns_keep_document_order_not_column_order() {
    let sheet = sheet();
    let observed: Vec<u32> = auto_filter(&sheet)
        .columns()
        .map(|column| column.column_offset(sheet.interner()).expect("a colId"))
        .collect();
    assert_eq!(observed, vec![1, 0, 4, 2, 5, 3, 6]);
    assert_ne!(
        observed,
        {
            let mut sorted = observed.clone();
            sorted.sort_unstable();
            sorted
        },
        "the fixture's order must differ from the sorted one, or this test cannot fail"
    );
}

/// The two-condition sort state comes back as a record: the range, the flags, and both keys in
/// significance order.
#[test]
fn the_sort_state_reads_back_as_two_conditions_in_significance_order() {
    let sheet = sheet();
    let interner = sheet.interner();
    let state = auto_filter(&sheet).sort_state().expect("a sortState");

    assert_eq!(state.range(interner).expect("@ref"), range("A2:G20"));
    assert!(state.is_case_sensitive(interner).expect("@caseSensitive"));
    assert!(
        !state.sorts_columns(interner).expect("@columnSort"),
        "the default is a top-to-bottom sort"
    );
    assert_eq!(state.len(), 2);

    let conditions: Vec<(CellRange, bool, SortBy, Option<u32>)> = state
        .conditions()
        .map(|condition| {
            (
                condition.range(interner).expect("@ref"),
                condition.is_descending(interner).expect("@descending"),
                condition.sort_by(interner).expect("@sortBy"),
                condition
                    .differential_format_index(interner)
                    .expect("@dxfId"),
            )
        })
        .collect();
    assert_eq!(
        conditions,
        vec![
            (range("B2:B20"), true, SortBy::Value, None),
            (range("F2:F20"), false, SortBy::CellColor, Some(0)),
        ],
        "the first condition is the primary key; nothing reorders them and nothing performs them"
    );
    assert_eq!(
        state
            .conditions()
            .next()
            .expect("the primary key")
            .icon_set(interner)
            .expect("@iconSet"),
        IconSetType::ThreeArrows,
        "an absent `@iconSet` reads as the schema default `3Arrows`, spelled from the generated enum"
    );
}

// -----------------------------------------------------------------------------------------------
// Data validation
// -----------------------------------------------------------------------------------------------

/// Every attribute and both formulas of the four rules read back as the file wrote them — the
/// entity-spelled `custom` expression included.
#[test]
fn the_four_validations_read_back_with_their_formulas_as_text() {
    let sheet = sheet();
    let interner = sheet.interner();
    let rules: Vec<&DataValidation> = sheet.data_validation_rules().collect();
    assert_eq!(rules.len(), 4);

    // 0 — the list validation whose source is a **range reference**.
    assert_eq!(
        rules[0].kind(interner).expect("@type"),
        DataValidationType::List
    );
    assert_eq!(
        rules[0].first_formula().expect("formula1").text(),
        "$G$2:$G$4",
        "a range source stays a range source — this is the resolution the ticket forbids"
    );
    assert!(rules[0].second_formula().is_none());
    assert!(rules[0].allows_blank(interner).expect("@allowBlank"));
    assert_eq!(
        rules[0].ranges(interner).expect("@sqref").ranges(),
        ranges("G2:G5 G8").ranges()
    );
    assert_eq!(
        rules[0]
            .error_title(interner)
            .expect("@errorTitle")
            .expect("a title"),
        "Bad <value>",
        "an `ST_Xstring` decodes on read; the file's bytes are unchanged"
    );
    assert_eq!(
        rules[0]
            .error_message(interner)
            .expect("@error")
            .expect("a message"),
        "Use \"Low\", \"Medium\" or \"High\" & nothing else"
    );
    assert_eq!(
        rules[0]
            .prompt_title(interner)
            .expect("@promptTitle")
            .expect("a title"),
        "Grade"
    );

    // 1 — the list validation whose source is a **literal**, sitting beside the one above.
    assert_eq!(
        rules[1].first_formula().expect("formula1").text(),
        "\"Ada,Grace,Alan\"",
        "a quoted literal list stays quoted; the two spellings are never converted into each other"
    );
    assert!(rules[1].shows_drop_down(interner).expect("@showDropDown"));

    // 2 — the two-formula comparison.
    assert_eq!(
        rules[2].kind(interner).expect("@type"),
        DataValidationType::Decimal
    );
    assert_eq!(
        rules[2].operator(interner).expect("@operator"),
        DataValidationOperator::Between
    );
    assert_eq!(rules[2].first_formula().expect("formula1").text(), "0");
    assert_eq!(rules[2].second_formula().expect("formula2").text(), "10000");

    // 3 — the custom rule, whose expression is entity-spelled in the file.
    assert_eq!(
        rules[3].kind(interner).expect("@type"),
        DataValidationType::Custom
    );
    assert_eq!(
        rules[3].first_formula().expect("formula1").text(),
        "AND($E2>0,$E2<\"10000\")",
        "the text decodes; the bytes are replayed as written, which the round trip below proves"
    );
    assert_eq!(
        rules[3].error_style(interner).expect("@errorStyle"),
        DataValidationErrorStyle::Warning
    );
    assert_eq!(
        rules[3].input_method_mode(interner).expect("@imeMode"),
        DataValidationImeMode::FullWidthKatakana
    );
    assert_eq!(
        rules[3].operator(interner).expect("@operator"),
        DataValidationOperator::NotBetween,
        "a `@operator` on a `custom` rule is preserved and reported, because it is what the file says"
    );
}

/// A cell's validations are the rules whose `@sqref` covers it — the multi-range one included — and
/// nothing else.
#[test]
fn a_cells_validations_are_the_rules_whose_sqref_covers_it() {
    let sheet = sheet();
    let covering = |reference: &str| {
        sheet
            .data_validations_for(cell(reference))
            .expect("the rules resolve")
            .iter()
            .map(|rule| rule.kind(sheet.interner()).expect("@type"))
            .collect::<Vec<_>>()
    };

    assert_eq!(covering("G3"), vec![DataValidationType::List]);
    // `G8` is in the **second** range of the multi-range `sqref`, the half a single-range parser
    // would miss.
    assert_eq!(covering("G8"), vec![DataValidationType::List]);
    assert_eq!(covering("G6"), Vec::new(), "between the two ranges");
    assert_eq!(covering("B4"), vec![DataValidationType::Decimal]);
    assert_eq!(covering("A2"), Vec::new(), "no rule names column A");
}

/// The stale `@count` is left exactly as the file wrote it, on read and after an append.
///
/// `@count` is a producer's cache. Correcting one nobody asked about is the same class of
/// helpfulness this cluster refuses everywhere else.
#[test]
fn the_stale_validation_count_is_left_exactly_as_the_file_wrote_it() {
    let mut sheet = sheet();
    let interner_count = sheet
        .data_validations()
        .expect("the element")
        .count(sheet.interner())
        .expect("a @count");
    assert_eq!(interner_count, Some(9), "the file says 9 and holds 4");

    let prefix = sheet.element_prefix().map(str::to_owned);
    let rule = DataValidationSpec::list(ranges("D2:D5"), "\"a,b\"")
        .build(sheet.interner_mut(), prefix.as_deref());
    sheet.add_data_validation(rule);

    let block = sheet.data_validations().expect("the element");
    assert_eq!(block.len(), 5, "the rule was appended");
    assert_eq!(
        block.count(sheet.interner()).expect("a @count"),
        Some(9),
        "adding a rule does not rewrite the producer's cache"
    );
}

// -----------------------------------------------------------------------------------------------
// Never apply — the three faces of this child's own hazard
// -----------------------------------------------------------------------------------------------

/// **Reading, writing and removing a filter moves no row's `@hidden` flag.**
///
/// Row 4 is hidden in the file and row 5 is not, and the filters over them are exactly the sort that
/// would hide row 5 if anything here evaluated one. All three operations leave both rows as the file
/// wrote them.
#[test]
fn no_row_hidden_flag_moves_for_a_filter() {
    let flags = |sheet: &WorksheetPart| -> Vec<(u32, bool)> {
        sheet
            .rows()
            .map(|row| (row.number().expect("a row number"), row.is_hidden()))
            .collect()
    };
    let original = flags(&sheet());

    // Reading.
    let read_only = sheet();
    let _ = auto_filter(&read_only).columns().count();
    assert_eq!(
        flags(&read_only),
        original,
        "reading a filter hides nothing"
    );

    // Writing a new one over a range that covers every row.
    let mut sheet = sheet();
    let prefix = sheet.element_prefix().map(str::to_owned);
    let spec = AutoFilterSpec::over(range("A1:G20"))
        .with_column(FilterColumnSpec::new(0, FilterSpecKind::values(["North"])));
    let built = spec.build(sheet.interner_mut(), prefix.as_deref());
    sheet.set_auto_filter(Some(built));
    assert_eq!(
        flags(&sheet),
        original,
        "authoring a filter that matches one row of five hides none of the other four"
    );
    // And the same after a full round trip through the writer.
    assert_eq!(flags(&read(&sheet.to_markup())), original);

    // Removing it.
    sheet.set_auto_filter(None);
    assert_eq!(
        flags(&sheet),
        original,
        "removing a filter unhides nothing either — a hidden row is the file's statement"
    );
}

/// **A sort state is a record.** Reading or writing one leaves every row in the order the file wrote
/// it.
#[test]
fn no_row_moves_for_a_sort_state() {
    let order = |sheet: &WorksheetPart| -> Vec<u32> {
        sheet
            .rows()
            .map(|row| row.number().expect("a row number"))
            .collect()
    };
    let values = |sheet: &WorksheetPart| -> Vec<String> {
        sheet
            .cells()
            .map(|cell| cell.reference().text().as_str().to_owned())
            .collect()
    };
    let original_order = order(&sheet());
    let original_values = values(&sheet());

    let mut sheet = sheet();
    let prefix = sheet.element_prefix().map(str::to_owned);
    // A descending sort on the column whose values ascend down the sheet — the one that would
    // visibly reorder anything if it were applied.
    let spec = AutoFilterSpec::over(range("A1:G20")).with_sort_state(SortStateSpec::new(
        range("A2:G20"),
        vec![SortConditionSpec::descending(range("B2:B20"))],
    ));
    let built = spec.build(sheet.interner_mut(), prefix.as_deref());
    sheet.set_auto_filter(Some(built));

    assert_eq!(order(&sheet), original_order, "no row moved");
    assert_eq!(values(&sheet), original_values, "no cell moved");
    let reread = read(&sheet.to_markup());
    assert_eq!(order(&reread), original_order);
    assert_eq!(values(&reread), original_values);
}

/// **The gate the ticket's second mutation is really about.** A `list` validation authored from a
/// range source keeps that range source, at build time and after a round trip.
#[test]
fn an_authored_list_validation_keeps_its_range_source() {
    let mut sheet = worksheet("<x:sheetData/>");
    let prefix = sheet.element_prefix().map(str::to_owned);
    let spec = DataValidationSpec::list(ranges("A2:A10"), "Lookups!$A$1:$A$9");
    let rule = spec.build(sheet.interner_mut(), prefix.as_deref());
    sheet.add_data_validation(rule);

    let markup = String::from_utf8(sheet.to_markup()).expect("UTF-8");
    assert!(
        markup.contains("<x:formula1>Lookups!$A$1:$A$9</x:formula1>"),
        "the source must be written as the caller stated it; got {markup}"
    );
    assert!(
        !markup.contains("<x:formula1>\""),
        "nothing turned the range into a quoted literal list; got {markup}"
    );

    let reread = read(markup.as_bytes());
    assert_eq!(
        reread
            .data_validation_rules()
            .next()
            .expect("the rule")
            .first_formula()
            .expect("formula1")
            .text(),
        "Lookups!$A$1:$A$9"
    );
}

// -----------------------------------------------------------------------------------------------
// Fidelity
// -----------------------------------------------------------------------------------------------

/// The whole worksheet comes back byte for byte through the markup model.
#[test]
fn the_fixture_re_emits_byte_for_byte_through_the_markup_model() {
    assert_eq!(
        sheet().to_markup(),
        sheet_bytes(),
        "the worksheet did not come back byte-identical through the markup model"
    );
}

/// The negative for the assertion above: it is shown to fail when one byte of one `@colId` changes,
/// so a green run means the filters really were compared.
#[test]
fn one_changed_column_offset_byte_is_caught() {
    let mut mutated = sheet_bytes();
    let at = mutated
        .windows(13)
        .position(|window| window == b"colId=\"4\"><to")
        .expect("the top10 column's colId is in the fixture");
    mutated[at + 7] = b'9';
    assert_ne!(
        read(&mutated).to_markup(),
        sheet_bytes(),
        "a changed colId must change the bytes, or the round trip proves nothing"
    );
}

/// **A validation re-emitted from the model keeps its range source, its stale count and its entity
/// spellings.**
///
/// The two byte-identity assertions cannot see any of that: an unedited part is one `memcpy`, and
/// after an edit elsewhere each slot still writes from its own bytes. This goes through the **one
/// door** that drops the `dataValidations` slot's claim on the file — the `_mut` accessor — and then
/// asserts on what the model emits. See this file's own documentation.
#[test]
fn a_validation_re_emitted_from_the_model_keeps_its_range_source() {
    let mut sheet = sheet();
    // Reaching the slot mutably drops its verbatim bytes, so from here on the model writes it.
    let removed = sheet
        .data_validations_mut()
        .expect("the dataValidations element")
        .remove_rule(2)
        .expect("the decimal rule");
    assert_eq!(
        removed.first_formula().expect("formula1").text(),
        "0",
        "the rule removed is the two-formula comparison"
    );

    let markup = String::from_utf8(sheet.to_markup()).expect("UTF-8");
    assert!(
        markup.contains("<formula1>$G$2:$G$4</formula1>"),
        "the list validation's range source survived re-emission from the model; a resolver would \
         have written the three cell values instead. Got: {markup}"
    );
    assert!(
        markup.contains("<formula1>\"Ada,Grace,Alan\"</formula1>"),
        "the literal list survived unchanged too"
    );
    assert!(
        markup.contains("AND($E2&gt;0,$E2&lt;&quot;10000&quot;)"),
        "the custom expression kept the file's own entity spellings — a minimal re-escape would \
         have written a bare `\"` here"
    );
    assert!(
        markup.contains("count=\"9\""),
        "the stale `@count` survived a re-emission that removed a rule"
    );
    assert!(
        markup.contains("sqref=\"G2:G5  G8\""),
        "the multi-range `@sqref` kept its double space, which only a verbatim replay can do"
    );

    let reread = read(markup.as_bytes());
    let kinds: Vec<DataValidationType> = reread
        .data_validation_rules()
        .map(|rule| rule.kind(reread.interner()).expect("@type"))
        .collect();
    assert_eq!(
        kinds,
        vec![
            DataValidationType::List,
            DataValidationType::List,
            DataValidationType::Custom,
        ],
        "removing one rule leaves the other three exactly as they were"
    );
}

/// **The same door, on the autofilter.** A filter column re-emitted from the model keeps every kind,
/// the extension slot, and the sort state.
#[test]
fn an_autofilter_re_emitted_from_the_model_keeps_every_kind_and_its_extension() {
    let mut sheet = sheet();
    let removed = sheet
        .auto_filter_mut()
        .expect("the autoFilter")
        .remove_column(0)
        .expect("the customFilters column");
    assert_eq!(removed.column_offset(sheet.interner()).expect("a colId"), 1);

    let markup = String::from_utf8(sheet.to_markup()).expect("UTF-8");
    for fragment in [
        "<filters blank=\"1\" calendarType=\"hijri\">",
        "<filter val=\"South &amp; East\"/>",
        "<dateGroupItem year=\"2015\" month=\"3\" dateTimeGrouping=\"month\"/>",
        "<top10 top=\"0\" percent=\"1\" val=\"25\" filterVal=\"317.5\"/>",
        "<dynamicFilter type=\"M3\" val=\"42064\" maxVal=\"42094\"/>",
        "<colorFilter dxfId=\"1\" cellColor=\"0\"/>",
        "<iconFilter iconSet=\"3Symbols\" iconId=\"2\"/>",
        // The seventh member of the choice, prefix, `uri` and all.
        "<x14:filter val=\"pending\"/>",
        "<sortCondition ref=\"F2:F20\" sortBy=\"cellColor\" dxfId=\"0\"/>",
    ] {
        assert!(
            markup.contains(fragment),
            "re-emitting the autofilter from the model lost `{fragment}`. Got: {markup}"
        );
    }
    assert!(
        !markup.contains("customFilters"),
        "the one column that was removed is gone, so this test is not vacuous"
    );
}

/// A cell edit rewrites `sheetData` and leaves the autofilter, the validations, every row's
/// `@hidden` and the `x14` extension byte for byte where they were.
///
/// **This is the `x14` gate.** The extension namespace carries the cross-sheet validations, is not
/// modelled here, and must survive an unrelated edit with its prefix, its `uri` and its attribute
/// order intact.
#[test]
fn an_unrelated_edit_leaves_the_filters_the_validations_and_the_x14_extension_byte_identical() {
    let original = String::from_utf8(sheet_bytes()).expect("UTF-8");
    let mut sheet = sheet();
    sheet
        .set_cell_value(cell("C3"), mjx_sml::CellValue::Number(1234.5))
        .expect("the cell writes");
    let rebuilt = String::from_utf8(sheet.to_markup()).expect("UTF-8");
    assert_ne!(
        rebuilt, original,
        "the edit must actually have changed something"
    );

    for fragment in [
        // The whole autofilter, out-of-order colIds and all.
        "<autoFilter ref=\"A1:G20\"><filterColumn colId=\"1\"><customFilters and=\"1\">",
        "<filterColumn colId=\"6\" hiddenButton=\"1\" showButton=\"0\">",
        "<sortState ref=\"A2:G20\" caseSensitive=\"1\">",
        // The validations, the stale count and the odd whitespace in the sqref.
        "<dataValidations count=\"9\" xWindow=\"120\" yWindow=\"180\">",
        "sqref=\"G2:G5  G8\"",
        "<formula1>$G$2:$G$4</formula1>",
        "AND($E2&gt;0,$E2&lt;&quot;10000&quot;)",
        // The hidden row is untouched.
        "<row r=\"4\" hidden=\"1\" customHeight=\"1\" ht=\"0\">",
        // The worksheet-level `x14` extension, with the second foreign prefix `xm` inside it.
        "<x14:formula1><xm:f>Lookups!$A$1:$A$9</xm:f></x14:formula1>",
        "<xm:sqref>C2:C20</xm:sqref>",
    ] {
        assert!(
            rebuilt.contains(fragment),
            "an unrelated cell edit lost `{fragment}`"
        );
    }
}

// -----------------------------------------------------------------------------------------------
// Authoring
// -----------------------------------------------------------------------------------------------

/// Authoring an autofilter and a list validation into a bare worksheet puts both at their rank in
/// `CT_Worksheet`'s sequence — the autofilter at 10, the validations at 17 — and emits the markup
/// the schema declares.
#[test]
fn authoring_places_both_slots_at_their_schema_rank() {
    let mut sheet = worksheet(
        "<x:sheetData/><x:mergeCells count=\"1\"><x:mergeCell ref=\"A1:B1\"/></x:mergeCells>\
         <x:pageMargins left=\"0.7\" right=\"0.7\" top=\"0.75\" bottom=\"0.75\" header=\"0.3\" footer=\"0.3\"/>",
    );
    let prefix = sheet.element_prefix().map(str::to_owned);

    let filter = AutoFilterSpec::over(range("A1:C10"))
        .with_column(FilterColumnSpec::new(
            0,
            FilterSpecKind::values(["North", "South"]),
        ))
        .with_column(FilterColumnSpec::new(
            1,
            FilterSpecKind::Custom {
                comparisons: vec![CustomFilterSpec::new(FilterOperator::GreaterThan, "100")],
                requires_both: false,
            },
        ))
        .with_sort_state(SortStateSpec::new(
            range("A2:C10"),
            vec![SortConditionSpec::descending(range("B2:B10"))],
        ))
        .build(sheet.interner_mut(), prefix.as_deref());
    sheet.set_auto_filter(Some(filter));

    let rule = DataValidationSpec::list(ranges("C2:C10"), "$E$1:$E$4")
        .with_error("Bad value", "Pick from the list")
        .build(sheet.interner_mut(), prefix.as_deref());
    sheet.add_data_validation(rule);

    let locals: Vec<&str> = sheet.child_element_locals().collect();
    assert_eq!(
        locals,
        vec![
            "sheetData",       // 5
            "autoFilter",      // 10
            "mergeCells",      // 14
            "dataValidations", // 17
            "pageMargins",     // 20
        ],
        "both new slots landed at their rank, on the right side of the children already there"
    );

    let markup = String::from_utf8(sheet.to_markup()).expect("UTF-8");
    assert!(markup.contains(
        "<x:autoFilter ref=\"A1:C10\"><x:filterColumn colId=\"0\"><x:filters>\
         <x:filter val=\"North\"/><x:filter val=\"South\"/></x:filters></x:filterColumn>"
    ));
    assert!(markup.contains(
        "<x:filterColumn colId=\"1\"><x:customFilters>\
         <x:customFilter operator=\"greaterThan\" val=\"100\"/></x:customFilters></x:filterColumn>"
    ));
    assert!(markup.contains(
        "<x:sortState ref=\"A2:C10\"><x:sortCondition descending=\"true\" ref=\"B2:B10\"/></x:sortState>"
    ));
    assert!(markup.contains(
        "<x:dataValidations><x:dataValidation type=\"list\" showErrorMessage=\"true\" \
         errorTitle=\"Bad value\" error=\"Pick from the list\" sqref=\"C2:C10\">\
         <x:formula1>$E$1:$E$4</x:formula1></x:dataValidation></x:dataValidations>"
    ));
}

/// A filter column carries **one** kind: setting a second replaces the first in its position, which
/// is what the `xsd:choice` asks for.
#[test]
fn setting_a_second_filter_kind_replaces_the_first() {
    let mut sheet = worksheet("<x:sheetData/>");
    let prefix = sheet.element_prefix().map(str::to_owned);
    let interner = sheet.interner_mut();

    let mut column = FilterColumnSpec::new(0, FilterSpecKind::values(["North"]))
        .build(interner, prefix.as_deref());
    assert!(matches!(column.filter(), Some(FilterKind::Values(_))));
    assert_eq!(column.content().len(), 1);

    let top10 = FilterSpecKind::Top10 {
        count: 10.0,
        takes_the_top: true,
        is_percentage: false,
    }
    .build(interner, prefix.as_deref());
    column.set_filter(Some(top10));
    assert!(matches!(column.filter(), Some(FilterKind::Top10(_))));
    assert_eq!(
        column.content().len(),
        1,
        "the choice holds one member, not two"
    );

    column.set_filter(None);
    assert!(column.filter().is_none());
    assert!(column.content().is_empty());
}

/// A `FormulaElement` set by hand replaces its text and nothing else, and the three slots it serves
/// keep their own names.
///
/// The escaping is **minimal**, which is XML's own rule and `mjx_xml::text::escape_text`'s: `<` and
/// `&` become references and `>` does not have to. That is also why the type replays a file's own
/// character data rather than re-escaping it — see [`mjx_sml::FormulaElement`].
#[test]
fn the_shared_formula_element_serves_all_three_slots() {
    let mut sheet = worksheet("<x:sheetData/>");
    let prefix = sheet.element_prefix().map(str::to_owned);
    let interner = sheet.interner_mut();

    let mut rule = DataValidation::new(interner, prefix.as_deref());
    rule.set_kind(interner, Some(DataValidationType::Custom));
    rule.set_ranges(interner, ranges("A1:A5"));
    let mut formula = FormulaElement::new(interner, prefix.as_deref(), "formula1", "A1>0");
    formula.set_text("A1>0 & \"x\"");
    rule.set_first_formula(Some(formula));
    rule.set_second_formula(Some(FormulaElement::new(
        interner,
        prefix.as_deref(),
        "formula2",
        "A1<9",
    )));

    sheet.add_data_validation(rule);
    let markup = String::from_utf8(sheet.to_markup()).expect("UTF-8");
    assert!(
        markup
            .contains("<x:formula1>A1>0 &amp; \"x\"</x:formula1><x:formula2>A1&lt;9</x:formula2>"),
        "each slot keeps its own local name, and replaced text is escaped once. Got: {markup}"
    );
}

/// **`CT_Worksheet` has two `sortState` slots and they are different elements.** One is rank 1 of
/// `CT_AutoFilter`; the other is rank **11** of the worksheet itself, the autofilter's own sibling.
///
/// The fixture writes the first. This authors the second beside it and asserts both survive, in the
/// right order, with the right conditions — a model that confused the two would either lose one or
/// place it wrong.
#[test]
fn the_sheet_level_sort_state_is_a_second_slot_beside_the_autofilters_own() {
    let mut sheet = sheet();
    assert!(
        sheet.sort_state().is_none(),
        "the fixture writes no sheet-level sortState, only the autofilter's"
    );

    let prefix = sheet.element_prefix().map(str::to_owned);
    let state = SortStateSpec::new(
        range("A2:G20"),
        vec![SortConditionSpec::ascending(range("A2:A20"))],
    )
    .build(sheet.interner_mut(), prefix.as_deref());
    sheet.set_sort_state(Some(state));

    let locals: Vec<&str> = sheet.child_element_locals().collect();
    let at = |name: &str| locals.iter().position(|local| *local == name);
    assert!(
        at("autoFilter") < at("sortState"),
        "rank 10 comes before rank 11: {locals:?}"
    );
    assert!(
        at("sortState") < at("dataValidations"),
        "rank 11 comes before rank 17: {locals:?}"
    );

    let reread = read(&sheet.to_markup());
    assert_eq!(
        reread
            .sort_state()
            .expect("the sheet-level sortState")
            .range(reread.interner())
            .expect("@ref"),
        range("A2:G20")
    );
    assert_eq!(
        reread
            .auto_filter()
            .expect("the autoFilter")
            .sort_state()
            .expect("its own sortState")
            .len(),
        2,
        "the autofilter's own two-condition sort is untouched by the sheet-level one"
    );
}
