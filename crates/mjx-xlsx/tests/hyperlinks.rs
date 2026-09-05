//! **MJXOFF-127's package gate.** The half `mjx-sml` cannot see: the relationship, and the rule that
//! it and the entry are one thing.
//!
//! `crates/mjx-sml/tests/worksheet_hyperlinks.rs` covers the markup — the three kinds, the six small
//! worksheet children, the object-anchor vocabulary — and its module documentation lists every
//! deliberate oddity in `tests/fixtures/hyperlinks.xlsx`. What is here is what needs a *package*:
//!
//! * the sheet lists its two external entries as `rId2` then `rId1`, **the reverse of the `.rels`
//!   order**, so a reader that answered from the relationships rather than from `x:hyperlinks`
//!   reports each target against the wrong cell;
//! * the mailto target carries `%20` and an `&amp;`-escaped query, so a resolver that normalised or
//!   re-escaped an untrusted URI produces a different string;
//! * the sheet also carries a `customProperty` relationship, which is **not** a hyperlink and which
//!   no markup in the sheet names by `r:id`… except that `customPr` does. Both directions are
//!   asserted, because the orphan check must fault a hyperlink nothing names and must *not* fault a
//!   relationship of any other type — a `comments` relationship is found by type and named by
//!   nothing, and a rule that faulted it would fail every commented worksheet in existence.
//!
//! # What the two mutations proved
//!
//! The ticket names both, and each is recorded on the case that caught it:
//!
//! * dropping the relationship when removing a hyperlink turns
//!   [`removing_a_hyperlink_removes_its_relationship_and_the_package_still_validates`] red **on
//!   `validate`**, not on an arithmetic assertion before it — MJXOFF-125 went red for the wrong
//!   reason first, so `validate()` is called before anything is asserted about ids;
//! * treating an internal `@location` link as external turns
//!   [`the_two_kinds_are_written_differently_and_only_one_adds_a_relationship`] red on the
//!   relationship count.

use mjx_opc::{Package, PartName, Relationship, TargetMode};
use mjx_sml::{CellRange, CellReference};
use mjx_xlsx::{HyperlinkKind, HyperlinkTarget, Workbook};

/// The fixture this whole suite is written against.
const FIXTURE: &str = "hyperlinks.xlsx";

/// The fixture, opened.
fn workbook() -> Workbook {
    Workbook::open(&mjx_fixtures::fixture(FIXTURE)).expect("the fixture opens")
}

/// A range, or a panic naming it.
fn range(text: &str) -> CellRange {
    CellRange::parse(text).unwrap_or_else(|error| panic!("{text} is a range: {error}"))
}

/// A cell reference, or a panic naming it.
fn cell(text: &str) -> CellReference {
    CellReference::parse(text).unwrap_or_else(|error| panic!("{text} is a cell: {error}"))
}

/// The worksheet part behind the first tab.
fn sheet_part() -> PartName {
    PartName::new("/xl/worksheets/sheet1.xml").expect("a part name")
}

/// How many `hyperlink` relationships `part` declares in `bytes`.
fn hyperlink_relationship_count(bytes: &[u8], part: &PartName) -> usize {
    let package = Package::open(bytes).expect("the container opens");
    package
        .relationships_for(Some(part))
        .map(|rels| {
            rels.iter()
                .filter(|rel| rel.rel_type == mjx_xlsx::parts::REL_HYPERLINK)
                .count()
        })
        .unwrap_or(0)
}

// -------------------------------------------------------------------------------------------
// Reading: the entry resolved against the sheet's own relationships
// -------------------------------------------------------------------------------------------

/// All three entries resolve, and each is reported as the kind it actually is.
///
/// The `ExternalWithLocation` row is the one that matters: an entry carrying both an `@r:id` and a
/// `@location` is a real file Excel writes, and reporting it as either of the other two would be
/// repairing it in the report.
#[test]
fn every_entry_resolves_and_the_third_kind_is_reported_rather_than_repaired() {
    let workbook = workbook();
    let links = workbook.sheet_hyperlinks(0).expect("the sheet reads");
    assert_eq!(links.len(), 3);

    assert_eq!(links[0].range, range("B4:D6"));
    assert_eq!(links[0].kind(), HyperlinkKind::Internal);
    assert_eq!(links[0].relationship_id, None);
    assert_eq!(links[0].target, None);
    assert_eq!(links[0].location.as_deref(), Some("Overview!A1"));

    assert_eq!(links[1].range, range("A2"));
    assert_eq!(links[1].kind(), HyperlinkKind::External);
    assert_eq!(
        links[1].relationship_id.as_deref(),
        Some("rId2"),
        "the sheet lists rId2 before rId1 — the reverse of the .rels order"
    );
    assert_eq!(
        links[1].target.as_deref(),
        Some("mailto:sales@example.invalid?subject=Q1%20numbers&body=see%20attached"),
        "an untrusted URI, carried exactly: the %20 is not decoded and the & is not re-escaped"
    );
    assert_eq!(links[1].target_mode, Some(TargetMode::External));

    assert_eq!(links[2].range, range("A8"));
    assert_eq!(
        links[2].kind(),
        HyperlinkKind::ExternalWithLocation,
        "both halves at once is a fourth report, not one of the other two"
    );
    assert_eq!(links[2].relationship_id.as_deref(), Some("rId1"));
    assert_eq!(
        links[2].target.as_deref(),
        Some("https://www.ecma-international.org/publications-and-standards/standards/ecma-376/")
    );
    assert_eq!(links[2].location.as_deref(), Some("Overview!B2"));
}

/// The per-cell lookup covers every cell of a multi-cell `@ref`, and nothing else.
#[test]
fn the_per_cell_lookup_covers_a_whole_range() {
    let workbook = workbook();
    for inside in ["B4", "C5", "D6"] {
        let found = workbook
            .cell_hyperlink(0, cell(inside))
            .expect("the sheet reads")
            .unwrap_or_else(|| panic!("{inside} is inside B4:D6"));
        assert_eq!(found.location.as_deref(), Some("Overview!A1"));
    }
    assert!(workbook
        .cell_hyperlink(0, cell("E7"))
        .expect("the sheet reads")
        .is_none());
    // The second tab has no hyperlinks at all, and that is an empty answer rather than an error.
    assert!(workbook
        .sheet_hyperlinks(1)
        .expect("sheet 2 reads")
        .is_empty());
}

/// Reading hyperlinks does not dirty the package: the container comes back byte-identical.
///
/// Tier 1. The whole point of `worksheet_markup` being a read is that asking a question can never
/// change what `save` writes.
#[test]
fn reading_hyperlinks_leaves_the_container_byte_identical() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let workbook = workbook();
    let _ = workbook.sheet_hyperlinks(0).expect("read");
    let _ = workbook.cell_hyperlink(0, cell("A2")).expect("read");

    let saved = workbook.save().expect("it saves");
    let before = Package::open(&original).expect("open");
    let after = Package::open(&saved).expect("open");
    for name in before.part_names() {
        assert_eq!(
            before.part_bytes(&name),
            after.part_bytes(&name),
            "{} changed, and nothing was edited",
            name.as_str()
        );
    }
}

// -------------------------------------------------------------------------------------------
// Writing: the two halves
// -------------------------------------------------------------------------------------------

/// The two authorable kinds are written differently, and **only one of them adds a relationship**.
///
/// This is the case the second mutation is aimed at: treat an internal `@location` link as external
/// — give it a relationship — and the count below goes to three.
#[test]
fn the_two_kinds_are_written_differently_and_only_one_adds_a_relationship() {
    let part = sheet_part();
    let before = hyperlink_relationship_count(&mjx_fixtures::fixture(FIXTURE), &part);
    assert_eq!(
        before, 2,
        "the fixture starts with two hyperlink relationships"
    );

    // An internal jump: `@location`, and nothing in the `.rels`.
    let mut with_location = workbook();
    with_location
        .set_cell_hyperlink(
            0,
            range("C2"),
            &HyperlinkTarget::Location("Overview!A2".to_owned()),
        )
        .expect("the internal link is written");
    let saved = with_location.save().expect("it saves");
    assert_eq!(
        hyperlink_relationship_count(&saved, &part),
        before,
        "an internal link adds no relationship — there is no part to reach"
    );
    let reopened = Workbook::open(&saved).expect("reopen");
    let written = reopened
        .cell_hyperlink(0, cell("C2"))
        .expect("read")
        .expect("the entry is there");
    assert_eq!(written.kind(), HyperlinkKind::Internal);
    assert_eq!(written.location.as_deref(), Some("Overview!A2"));
    assert_eq!(written.relationship_id, None);

    // An external target: one new `External` relationship, and the entry names it.
    let mut with_url = workbook();
    with_url
        .set_cell_hyperlink(
            0,
            range("D2"),
            &HyperlinkTarget::Url("https://example.invalid/a%20b?x=1&y=2".to_owned()),
        )
        .expect("the external link is written");
    let saved = with_url.save().expect("it saves");
    assert_eq!(
        hyperlink_relationship_count(&saved, &part),
        before + 1,
        "an external link adds exactly one relationship"
    );
    let reopened = Workbook::open(&saved).expect("reopen");
    let written = reopened
        .cell_hyperlink(0, cell("D2"))
        .expect("read")
        .expect("the entry is there");
    assert_eq!(written.kind(), HyperlinkKind::External);
    assert_eq!(written.target_mode, Some(TargetMode::External));
    assert_eq!(
        written.target.as_deref(),
        Some("https://example.invalid/a%20b?x=1&y=2"),
        "the URI a caller gave is written byte for byte — never normalised or resolved"
    );
}

/// Removing a hyperlink removes its relationship, and the package still validates.
///
/// **The ticket's first mutation lands here.** `validate()` is called *before* anything is asserted
/// about relationship counts, because MJXOFF-125's equivalent case first went red on arithmetic
/// rather than on the package invariant the ticket names — and a mutation that goes red for the
/// wrong reason has proved nothing.
#[test]
fn removing_a_hyperlink_removes_its_relationship_and_the_package_still_validates() {
    let part = sheet_part();
    let mut workbook = workbook();

    assert!(
        workbook
            .remove_cell_hyperlink(0, cell("A2"))
            .expect("the removal runs"),
        "A2 carried the external entry that names rId2"
    );

    // The invariant first. A relationship left behind fails here and nowhere else.
    workbook
        .validate()
        .expect("removing both halves together leaves a valid package");

    let saved = workbook.save().expect("it saves");
    assert_eq!(
        hyperlink_relationship_count(&saved, &part),
        1,
        "the relationship went with the entry"
    );

    let reopened = Workbook::open(&saved).expect("reopen");
    let links = reopened.sheet_hyperlinks(0).expect("read");
    assert_eq!(links.len(), 2);
    assert!(
        links
            .iter()
            .all(|link| link.relationship_id.as_deref() != Some("rId2")),
        "nothing still names the removed relationship"
    );
    assert!(
        reopened
            .cell_hyperlink(0, cell("A2"))
            .expect("read")
            .is_none(),
        "and the cell has no link"
    );
}

/// A relationship left behind fails `validate`, and it names the part, the id and the target.
///
/// The other side of the same rule, and what makes the mutation above meaningful: this case shows
/// the check *can* fire, by constructing exactly the state a defective `remove_cell_hyperlink` would
/// leave. Without it, "validate passes" would be satisfied by a validator that checks nothing.
#[test]
fn a_relationship_left_behind_is_a_defect_that_names_itself() {
    let part = sheet_part();
    let mut workbook = workbook();

    // Remove the entry through the markup tier only — which cannot see the relationship — and write
    // the sheet back. That is precisely the half-done removal the rule exists to catch.
    let mut markup = workbook
        .worksheet_markup(0)
        .expect("the sheet reads")
        .expect("it is a worksheet");
    let at = markup
        .hyperlink_position_covering(cell("A2"))
        .expect("A2 is linked");
    markup.remove_hyperlink(at).expect("the entry is removed");
    workbook
        .write_worksheet_markup(0, &markup)
        .expect("the sheet is written back");

    let error = workbook
        .validate()
        .expect_err("an orphaned hyperlink relationship is a defect");
    let text = error.to_string();
    assert!(
        text.contains("rId2"),
        "the defect names the relationship: {text}"
    );
    assert!(
        text.contains(part.as_str()),
        "and the part it is declared on: {text}"
    );
    assert!(
        text.contains("mailto:sales@example.invalid"),
        "and the target, so a caller can fix it without re-deriving it: {text}"
    );

    // `save` runs the same check, so this cannot be shipped by going round `validate`.
    assert!(
        workbook.save().is_err(),
        "save runs validate, so the half-done removal cannot reach a container"
    );
}

/// The orphan rule is about **hyperlink** relationships and no others.
///
/// A `comments` relationship is found by type and named by no markup at all, so a general
/// "unreferenced relationship" rule would fault every commented worksheet in existence — which is
/// exactly why `mjx-opc` refuses to state one. This case adds a non-hyperlink relationship that
/// nothing names and requires the package to stay valid.
#[test]
fn a_non_hyperlink_relationship_that_nothing_names_is_not_a_defect() {
    let part = sheet_part();

    // Put the extra relationship on the sheet at the container tier, where `Workbook` deliberately
    // offers no door: the point is to reach a package state a caller could arrive with, not one this
    // crate's surface can produce.
    let mut package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("open");
    package
        .add_relationship(
            Some(&part),
            Relationship {
                id: "rId900".to_owned(),
                rel_type: mjx_xlsx::parts::REL_PRINTER_SETTINGS.to_owned(),
                target: "https://example.invalid/printer".to_owned(),
                mode: TargetMode::External,
            },
        )
        .expect("the relationship is added");
    let bytes = package.save().expect("the container saves");

    let mut workbook = Workbook::open(&bytes).expect("reopen");
    // An edit brings the sheet into `authored_xml_parts`, which is the check's whole scope.
    workbook
        .set_cell_hyperlink(
            0,
            range("C2"),
            &HyperlinkTarget::Location("Overview!A2".to_owned()),
        )
        .expect("the edit is written");

    workbook
        .validate()
        .expect("a relationship of another type that nothing names is legal");
}

/// Two cells sharing one link: removing the first leaves the relationship for the second.
///
/// The rule `mjx_pptx::Presentation::remove_hyperlink_rel_if_unreferenced` states, restated here.
/// A removal that took the relationship out unconditionally would leave the surviving entry with a
/// dangling `@r:id`, which `mjx-opc` then reports.
#[test]
fn a_relationship_two_entries_share_survives_the_first_removal() {
    let mut workbook = workbook();
    let part = sheet_part();

    // Point a second cell at the same relationship, through the markup tier, so that exactly one
    // relationship has two users.
    let mut markup = workbook
        .worksheet_markup(0)
        .expect("read")
        .expect("a worksheet");
    let prefix = markup.relationship_prefix().map(str::to_owned).expect("r");
    let element_prefix = markup.element_prefix().map(str::to_owned);
    {
        let interner = markup.interner_mut();
        let mut entry = mjx_sml::Hyperlink::new(interner, element_prefix.as_deref());
        entry.set_range(interner, range("C7"));
        entry.set_relationship_id(interner, &prefix, "rId2");
        markup.add_hyperlink(entry);
    }
    workbook.write_worksheet_markup(0, &markup).expect("write");
    workbook
        .validate()
        .expect("two users of one relationship is valid");

    assert!(workbook
        .remove_cell_hyperlink(0, cell("A2"))
        .expect("the first removal runs"));
    workbook
        .validate()
        .expect("the surviving entry still names a declared relationship");
    let saved = workbook.save().expect("it saves");
    assert_eq!(
        hyperlink_relationship_count(&saved, &part),
        2,
        "the shared relationship survived its first user"
    );

    let mut workbook = Workbook::open(&saved).expect("reopen");
    assert!(workbook
        .remove_cell_hyperlink(0, cell("C7"))
        .expect("the second removal runs"));
    workbook.validate().expect("still valid");
    let saved = workbook.save().expect("it saves");
    assert_eq!(
        hyperlink_relationship_count(&saved, &part),
        1,
        "and went with its last user"
    );
}

/// Replacing a cell's link removes the relationship the old one named.
#[test]
fn replacing_an_external_link_with_an_internal_one_takes_the_relationship_with_it() {
    let part = sheet_part();
    let mut workbook = workbook();
    workbook
        .set_cell_hyperlink(
            0,
            range("A2"),
            &HyperlinkTarget::Location("Overview!A1".to_owned()),
        )
        .expect("the replacement is written");
    workbook.validate().expect("both halves moved together");

    let saved = workbook.save().expect("it saves");
    assert_eq!(
        hyperlink_relationship_count(&saved, &part),
        1,
        "the external relationship the old entry named is gone"
    );
    let reopened = Workbook::open(&saved).expect("reopen");
    let link = reopened
        .cell_hyperlink(0, cell("A2"))
        .expect("read")
        .expect("A2 is still linked");
    assert_eq!(link.kind(), HyperlinkKind::Internal);
    assert_eq!(
        reopened.sheet_hyperlinks(0).expect("read").len(),
        3,
        "one entry was replaced, not added beside the old one"
    );
}

/// Adding a hyperlink to a sheet that binds no relationship prefix declares one.
///
/// A worksheet this library authored declares only the SpreadsheetML namespace, and an `@r:id`
/// cannot be spelled without a binding. Refusing would mean a sheet built from nothing could never
/// gain a link.
#[test]
fn a_sheet_that_binds_no_relationship_prefix_gains_one() {
    let mut workbook = Workbook::blank().expect("a blank workbook");
    let markup = workbook
        .worksheet_markup(0)
        .expect("read")
        .expect("a worksheet");
    assert_eq!(
        markup.relationship_prefix(),
        None,
        "a blank sheet binds nothing but SpreadsheetML, or this case proves nothing"
    );

    workbook
        .set_cell_hyperlink(
            0,
            range("A1"),
            &HyperlinkTarget::Url("https://example.invalid/".to_owned()),
        )
        .expect("the link is written");
    workbook.validate().expect("the package is valid");

    let saved = workbook.save().expect("it saves");
    let reopened = Workbook::open(&saved).expect("reopen");
    let link = reopened
        .cell_hyperlink(0, cell("A1"))
        .expect("read")
        .expect("A1 is linked");
    assert_eq!(link.target.as_deref(), Some("https://example.invalid/"));
}

// -------------------------------------------------------------------------------------------
// Fidelity
// -------------------------------------------------------------------------------------------

/// Editing the hyperlinks of one sheet leaves every other part byte-identical.
///
/// Tier 3. The assertion is over the fixture's own bytes rather than over a second run of this
/// crate's writer, so a writer that was wrong in the same way twice does not pass.
#[test]
fn editing_one_sheets_hyperlinks_leaves_every_other_part_byte_identical() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let mut workbook = workbook();
    workbook
        .set_cell_hyperlink(
            0,
            range("C2"),
            &HyperlinkTarget::Location("Overview!A2".to_owned()),
        )
        .expect("the link is written");
    let saved = workbook.save().expect("it saves");

    let before = Package::open(&original).expect("open");
    let after = Package::open(&saved).expect("open");
    let edited = sheet_part();
    let mut changed = Vec::new();
    for name in before.part_names() {
        if before.part_bytes(&name) != after.part_bytes(&name) {
            changed.push(name.as_str().to_owned());
        }
    }
    assert_eq!(
        changed,
        vec![edited.as_str().to_owned()],
        "only the edited worksheet changed"
    );

    // Inside the edited part, everything but `x:hyperlinks` is still the file's own bytes.
    let sheet =
        String::from_utf8(after.part_bytes(&edited).expect("bytes").to_vec()).expect("UTF-8");
    for untouched in [
        r#"<dataConsolidate function="stdDev" leftLabels="1" link="1">"#,
        r#"<customSheetView guid="{5A2C8B04-1F3E-4C7D-9B61-0E4A7D2C8F35}" scale="85" showPageBreaks="1" showGridLines="0">"#,
        r#"<phoneticPr fontId="1" type="noConversion"/>"#,
        r#"<cellSmartTagPr key="Exchange" val="NASDAQ"/>"#,
        r#"<webPublishItems count="2">"#,
    ] {
        assert!(
            sheet.contains(untouched),
            "an edit to x:hyperlinks must leave {untouched} byte-identical"
        );
    }
}

/// The fixture opens, saves untouched, and comes back byte-identical part for part.
///
/// Tier 1 at the package tier. `mjx-opc`'s own suites sweep the whole corpus for this; the case is
/// repeated here so that the file this child added is covered by an assertion that names it.
#[test]
fn the_fixture_round_trips_untouched() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let workbook = workbook();
    let saved = workbook.save().expect("it saves");

    let before = Package::open(&original).expect("open");
    let after = Package::open(&saved).expect("open");
    let names: Vec<String> = before.part_names().map(|n| n.as_str().to_owned()).collect();
    assert!(
        names.contains(&"/customProperty1.bin".to_owned()),
        "the custom-property part is in the corpus"
    );
    for name in before.part_names() {
        assert_eq!(
            before.part_bytes(&name),
            after.part_bytes(&name),
            "{} is not byte-identical",
            name.as_str()
        );
    }
}
