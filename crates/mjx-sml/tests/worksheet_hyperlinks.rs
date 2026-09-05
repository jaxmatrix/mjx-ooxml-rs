//! **MJXOFF-127's markup gate.** The hyperlink cluster, the object-anchor vocabulary three Phase E
//! children share, and the six small `CT_Worksheet` children nothing else in Phase D claimed.
//!
//! # The fixture is authored to make specific wrong answers visible
//!
//! `tests/fixtures/hyperlinks.xlsx` is not one hyperlink repeated. **One hyperlink kind tests one
//! branch**, and the ticket names the branch a tidy reader would collapse, so the sheet carries all
//! three shapes `CT_Hyperlink` can take:
//!
//! | `x:hyperlink` | `@ref` | `@r:id` | `@location` | why it is there |
//! |---|---|---|---|---|
//! | 1st | `B4:D6` — **multi-cell** | — | `Overview!A1` | the internal kind, over a range rather than a cell |
//! | 2nd | `A2` | `rId2` | — | the external kind |
//! | 3rd | `A8` | `rId1` | `Overview!B2` | **both at once** — the never-repair case |
//!
//! Nine further things are in it on purpose, each of which turns a plausible wrong answer red:
//!
//! * **the two external entries name `rId2` then `rId1`**, the reverse of the `.rels` order, so a
//!   reader answering from the relationships rather than from `x:hyperlinks` swaps the two targets.
//!   `crates/mjx-xlsx/tests/hyperlinks.rs` pins that half, which is where relationships exist;
//! * **the mailto target spells its query `%20` and `&amp;`**, so a reader that percent-decoded,
//!   re-escaped or otherwise normalised an untrusted URI produces a different string;
//! * **the second entry's `@display` is single-quoted**, which is XML the writer never emits by
//!   choice — so a re-emission that rebuilt the attribute would change the bytes;
//! * **`dataConsolidate@function` is `stdDev`**, whose generated variant is
//!   [`DataConsolidateFunction::SampleStandardDeviation`] and not `StdDev`. A test written from the
//!   wire token does not compile;
//! * **`dataRefs@count` is 5 against three entries**, and one `dataRef` states **no attribute at
//!   all** — so a write path that "corrected" the cache, or a reader that assumed a reference names
//!   something, changes or misreports the file;
//! * **`webPublishItems@count` is 2 against one item**, for the same reason;
//! * **the two `cellWatch`es are `D6` then `B2`**, out of address order, so anything that sorted
//!   would show;
//! * **`ignoredErrors` carries an `extLst` after its records**, which is the one type in this child
//!   whose sequence has two slots — an appended record landing after the `extLst` is a worksheet
//!   this library wrote out of `xsd:sequence` order;
//! * **the sheet carries a `customSheetViews` (rank 13) and a `phoneticPr` (rank 15)**, neither of
//!   which MJXOFF-127 models, so the modelled and unmodelled slots genuinely interleave.
//!
//! # Why the re-emission tests are not byte-identity tests
//!
//! MJXOFF-120, MJXOFF-123 and MJXOFF-125 each established it and it holds again here: **a part
//! nobody edited is one `extend_from_slice` of its own buffer, and after an edit elsewhere every
//! other slot still writes from its own stored bytes.** No byte-identity gate ever reaches this
//! model's writer. So [`every_new_model_survives_a_rebuild_from_the_model_alone`] forces each new
//! type to be rebuilt through [`ToXml::to_xml`] — an element carrying no verbatim range anywhere —
//! and asserts on what comes out. That is the door a defect in an `as_raw_element` can be seen
//! through, and the byte-identity suites cannot.
//!
//! # Nothing here repairs, resolves, follows or performs anything
//!
//! Every assertion below is about what the file *says*. None is about where a hyperlink would go,
//! whether a suppressed error is present, what a consolidation would compute, or what a recogniser
//! would find — no call in this workspace can answer any of the four.

use mjx_ooxml_core::{Interner, ToXml};
use mjx_ooxml_types::spreadsheetml::{DataConsolidateFunction, WebSourceType};
use mjx_opc::{Package, PartName};
use mjx_sml::{
    CellRange, CellRangeList, CellReference, Hyperlink, Hyperlinks, ObjectAnchor, ObjectProperties,
    WorksheetPart,
};

/// The fixture this whole suite is written against.
const FIXTURE: &str = "hyperlinks.xlsx";

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

/// The fixture's first worksheet, read.
fn sheet() -> WorksheetPart {
    WorksheetPart::read_part(&part_bytes("/xl/worksheets/sheet1.xml"))
        .expect("the worksheet reads")
        .expect("the root is an x:worksheet")
}

/// A range, or a panic naming it.
fn range(text: &str) -> CellRange {
    CellRange::parse(text).unwrap_or_else(|error| panic!("{text} is a range: {error}"))
}

/// A cell reference, or a panic naming it.
fn cell(text: &str) -> CellReference {
    CellReference::parse(text).unwrap_or_else(|error| panic!("{text} is a cell: {error}"))
}

/// A range list, or a panic naming it.
fn ranges(text: &str) -> CellRangeList {
    CellRangeList::parse(text).unwrap_or_else(|error| panic!("{text} is an sqref: {error}"))
}

/// One value serialized to bytes, **rebuilt from the model** — no verbatim source range anywhere.
///
/// `interner` has to be the one the value's own names were interned in: a `RawName` is a pair of
/// symbols and a symbol means nothing anywhere else. That is why the sheet-derived cases below clone
/// the child out before reaching [`WorksheetPart::interner_mut`] — the clone is independent, and the
/// symbols still resolve.
fn rebuilt<T: ToXml>(value: &T, interner: &mut Interner) -> String {
    let element = value.to_xml(interner);
    let mut out = Vec::new();
    mjx_xml::fidelity::serialize_element(&element, interner, None, &mut out);
    String::from_utf8(out).expect("UTF-8")
}

/// Parses `markup` into a model of type `T`, in a fresh interner.
fn read<T: mjx_ooxml_core::FromXml>(markup: &str) -> (Interner, T) {
    let document = mjx_xml::fidelity::parse(markup.as_bytes()).expect("the markup parses");
    let value = T::from_xml(&document.root, &document.interner).expect("the model reads");
    (document.interner, value)
}

// -------------------------------------------------------------------------------------------
// The three hyperlink kinds
// -------------------------------------------------------------------------------------------

/// All three shapes of `CT_Hyperlink` read, and they are not one shape three times.
///
/// The case the ticket names directly: *"One hyperlink kind tests one branch."* The three entries
/// differ in `@ref` arity, in whether they name a relationship, and in whether they name a location,
/// so a reader that answered any of the three from a single field reports at least one of them
/// wrongly.
#[test]
fn the_three_hyperlink_kinds_read_and_they_are_not_one_kind_three_times() {
    let sheet = sheet();
    let interner = sheet.interner();
    let prefix = sheet.relationship_prefix();
    assert_eq!(prefix, Some("r"), "the fixture binds the r prefix");

    let links = sheet.hyperlinks().expect("the sheet writes x:hyperlinks");
    assert_eq!(links.len(), 3);
    let entries: Vec<&Hyperlink> = links.links().collect();

    // 1 — internal, over a multi-cell range, no relationship at all.
    assert_eq!(
        entries[0].range(interner),
        Ok(range("B4:D6")),
        "`@ref` is an ST_Ref: a hyperlink covers a range, not a cell"
    );
    assert!(
        !entries[0].range(interner).unwrap().is_single_cell(),
        "the internal entry must be multi-cell or it tests the same thing the others do"
    );
    assert_eq!(entries[0].relationship_id(interner, prefix), Ok(None));
    assert_eq!(
        entries[0].location(interner).unwrap().as_deref(),
        Some("Overview!A1")
    );

    // 2 — external, one relationship, no location.
    assert_eq!(entries[1].range(interner), Ok(range("A2")));
    assert_eq!(
        entries[1].relationship_id(interner, prefix),
        Ok(Some("rId2".to_owned())),
        "the *second* entry names rId2 — the reverse of the .rels order"
    );
    assert_eq!(entries[1].location(interner).unwrap(), None);
    assert_eq!(
        entries[1].tooltip(interner).unwrap().as_deref(),
        Some("Ask sales")
    );

    // 3 — both at once. The never-repair case.
    assert_eq!(entries[2].range(interner), Ok(range("A8")));
    assert_eq!(
        entries[2].relationship_id(interner, prefix),
        Ok(Some("rId1".to_owned()))
    );
    assert_eq!(
        entries[2].location(interner).unwrap().as_deref(),
        Some("Overview!B2"),
        "an entry carrying BOTH an r:id and a location is a real file Excel writes"
    );
}

/// Reading an entry that carries both an `@r:id` and a `@location` changes neither.
///
/// The trap stated as an assertion. A model that treated the two as alternatives — dropping the
/// relationship reference because a location is present, or the other way round — would still pass
/// every test above; this one re-emits the entry **from the model** and requires both attributes to
/// come back.
#[test]
fn an_entry_with_both_an_r_id_and_a_location_is_never_normalised_into_one_kind() {
    let mut sheet = sheet();
    let entry = sheet
        .hyperlinks()
        .expect("x:hyperlinks")
        .links()
        .nth(2)
        .expect("the third entry")
        .clone();

    let emitted = rebuilt(&entry, sheet.interner_mut());
    assert!(
        emitted.contains(r#"r:id="rId1""#),
        "the relationship reference survives a location beside it: {emitted}"
    );
    assert!(
        emitted.contains(r#"location="Overview!B2""#),
        "the location survives a relationship beside it: {emitted}"
    );
}

/// An entry's `@display` keeps the single quotes the file wrote it with.
///
/// The attribute grammar never rebuilds the attribute vector, and this is what says so for a
/// character no writer here would choose. It fails for a model that decoded and re-encoded the
/// value, which is the shape of every "helpful" normalisation.
#[test]
fn an_attribute_keeps_the_quote_character_the_file_wrote() {
    let raw = String::from_utf8(part_bytes("/xl/worksheets/sheet1.xml")).expect("UTF-8");
    assert!(
        raw.contains("display='ECMA-376'"),
        "the fixture must carry a single-quoted attribute or this case proves nothing"
    );

    let mut sheet = sheet();
    let entry = sheet
        .hyperlinks()
        .expect("x:hyperlinks")
        .links()
        .nth(1)
        .expect("the second entry")
        .clone();
    assert!(
        rebuilt(&entry, sheet.interner_mut()).contains("display='ECMA-376'"),
        "a rebuild keeps the quote character"
    );
}

/// The lookup answers the first entry in **document order** that covers a cell, and covers every
/// cell of a multi-cell `@ref`.
#[test]
fn a_lookup_covers_every_cell_of_a_multi_cell_reference() {
    let sheet = sheet();
    for inside in ["B4", "C5", "D6"] {
        let found = sheet
            .hyperlink_covering(cell(inside))
            .unwrap_or_else(|| panic!("{inside} is inside B4:D6"));
        assert_eq!(
            found.location(sheet.interner()).unwrap().as_deref(),
            Some("Overview!A1")
        );
    }
    assert!(
        sheet.hyperlink_covering(cell("E7")).is_none(),
        "E7 is outside every @ref"
    );
    assert_eq!(sheet.hyperlink_position_covering(cell("A2")), Some(1));
    assert_eq!(sheet.hyperlink_position_covering(cell("A8")), Some(2));
}

/// Removing the last entry removes the whole `x:hyperlinks` element.
///
/// `hyperlink` is `minOccurs="1"`, so an empty `<hyperlinks/>` is markup the schema gate rejects —
/// which is exactly what would ship if this rule were missing.
#[test]
fn removing_the_last_entry_removes_the_element_the_schema_forbids_leaving_empty() {
    let mut sheet = sheet();
    assert!(sheet.remove_hyperlink(2).is_some());
    assert!(sheet.remove_hyperlink(1).is_some());
    assert_eq!(sheet.hyperlinks().map(Hyperlinks::len), Some(1));
    assert!(sheet.remove_hyperlink(0).is_some());
    assert!(
        sheet.hyperlinks().is_none(),
        "the element goes with its last entry"
    );
    assert!(
        !sheet
            .child_element_locals()
            .any(|local| local == "hyperlinks"),
        "and it is gone from the child list, not merely emptied"
    );
    assert!(sheet.remove_hyperlink(0).is_none());
}

/// An authored `x:hyperlinks` lands at rank 18, between `phoneticPr` (15) and `customProperties`
/// (25) — both of which this fixture carries and only one of which is modelled.
///
/// The placement case MJXOFF-117's `Slot::rank` fix exists for: an unmodelled `phoneticPr` that was
/// treated as unrankable would let this element be inserted before it.
#[test]
fn an_authored_hyperlinks_element_lands_at_its_rank_among_modelled_and_held_slots() {
    let mut sheet = sheet();
    // Take the element out, then put a fresh one back and see where it goes.
    sheet.set_hyperlinks(None);
    let before: Vec<String> = sheet.child_element_locals().map(str::to_owned).collect();
    assert!(!before.contains(&"hyperlinks".to_owned()));

    let prefix = sheet.element_prefix().map(str::to_owned);
    let mut link = Hyperlink::new(sheet.interner_mut(), prefix.as_deref());
    link.set_range(sheet.interner_mut(), range("A1"));
    sheet.add_hyperlink(link);

    let after: Vec<String> = sheet.child_element_locals().map(str::to_owned).collect();
    let at = after
        .iter()
        .position(|local| local == "hyperlinks")
        .expect("the element was created");
    let phonetic = after
        .iter()
        .position(|local| local == "phoneticPr")
        .expect("the fixture carries an unmodelled phoneticPr at rank 15");
    let custom = after
        .iter()
        .position(|local| local == "customProperties")
        .expect("rank 25");
    assert!(
        phonetic < at && at < custom,
        "rank 18 sits after the held phoneticPr and before customProperties: {after:?}"
    );
}

// -------------------------------------------------------------------------------------------
// The object-anchor vocabulary — MJXOFF-114 (E5) and MJXOFF-107 (E3) consume these
// -------------------------------------------------------------------------------------------

/// `CT_ObjectAnchor` reads its two flags and holds its two `xdr:` markers verbatim.
///
/// The markers are `dml-spreadsheetDrawing.xsd`'s `CT_Marker` and MJXOFF-107 (E3) models them; this
/// crate hands back the element the file wrote, prefix intact, and decodes nothing.
#[test]
fn an_object_anchor_reads_its_flags_and_holds_the_xdr_markers_verbatim() {
    let markup = concat!(
        r#"<anchor xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" "#,
        r#"xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" "#,
        r#"moveWithCells="1">"#,
        r#"<xdr:from><xdr:col>2</xdr:col><xdr:colOff>19050</xdr:colOff>"#,
        r#"<xdr:row>3</xdr:row><xdr:rowOff>9525</xdr:rowOff></xdr:from>"#,
        r#"<xdr:to><xdr:col>5</xdr:col><xdr:colOff>0</xdr:colOff>"#,
        r#"<xdr:row>8</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>"#,
        r#"</anchor>"#
    );
    let (mut interner, anchor): (_, ObjectAnchor) = read(markup);

    assert_eq!(
        anchor.moves_with_cells(&interner),
        Ok(true),
        "@moveWithCells is written and true"
    );
    assert_eq!(
        anchor.sizes_with_cells(&interner),
        Ok(false),
        "@sizeWithCells is absent, and its schema default is false"
    );

    let from = anchor.from_marker(&interner).expect("xdr:from");
    assert_eq!(interner.resolve(from.name.local), "from");
    assert_eq!(
        from.name.prefix.map(|s| interner.resolve(s)),
        Some("xdr"),
        "the marker keeps the prefix the file bound"
    );
    assert_eq!(
        from.children.len(),
        4,
        "col, colOff, row and rowOff are held, not decoded"
    );
    assert!(anchor.to_marker(&interner).is_some());

    let emitted = rebuilt(&anchor, &mut interner);
    assert!(emitted.contains("<xdr:from>"), "{emitted}");
    assert!(emitted.contains("<xdr:to>"), "{emitted}");
}

/// Placing a marker uses the generated `OBJECT_ANCHOR` table, which knows `xdr:from` precedes
/// `xdr:to` **across a namespace boundary** no `sml` local name reveals.
///
/// Written `to` first on purpose: a placement that appended would leave the two the wrong way round,
/// and a hand-rolled table keyed on `sml` names could not have ranked either of them.
#[test]
fn placing_the_two_markers_puts_them_in_the_order_the_generated_table_gives() {
    let markup = concat!(
        r#"<anchor xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" "#,
        r#"xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"/>"#
    );
    let (mut interner, mut anchor): (_, ObjectAnchor) = read(markup);

    let to = mjx_xml::fidelity::parse(
        br#"<xdr:to xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"/>"#,
    )
    .expect("parses");
    let from = mjx_xml::fidelity::parse(
        br#"<xdr:from xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"/>"#,
    )
    .expect("parses");

    // The interners differ, so re-intern the two roots into the anchor's own.
    let adopt = |element: &mjx_ooxml_core::RawElement,
                 source: &Interner,
                 into: &mut Interner|
     -> mjx_ooxml_core::RawElement {
        mjx_ooxml_core::RawElement::rebuilt(
            mjx_ooxml_core::RawName {
                prefix: element.name.prefix.map(|s| into.intern(source.resolve(s))),
                local: into.intern(source.resolve(element.name.local)),
                namespace: element
                    .name
                    .namespace
                    .map(|s| into.intern(source.resolve(s))),
            },
            Vec::new(),
            Vec::new(),
            true,
        )
    };
    let to = adopt(&to.root, &to.interner, &mut interner);
    let from = adopt(&from.root, &from.interner, &mut interner);

    anchor.set_to_marker(&interner, to);
    anchor.set_from_marker(&interner, from);

    let emitted = rebuilt(&anchor, &mut interner);
    let from_at = emitted.find("<xdr:from").expect("from is written");
    let to_at = emitted.find("<xdr:to").expect("to is written");
    assert!(
        from_at < to_at,
        "xdr:from is rank 0 and xdr:to rank 1, whichever order they were set in: {emitted}"
    );
}

/// `CT_ObjectPr`'s eight-of-nine `true` defaults are the schema's, not `false`.
///
/// The trap this type carries: every other flag family in `sml.xsd` defaults to `false`, and a
/// reader that assumed the same here reports six of `CT_ObjectPr`'s flags backwards for an element
/// that writes none.
#[test]
fn object_properties_report_the_schema_defaults_and_not_a_family_wide_false() {
    let markup = concat!(
        r#"<objectPr xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" "#,
        r#"xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" "#,
        r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" "#,
        r#"defaultSize="0" altText="A chart of Q1" r:id="rId9">"#,
        r#"<anchor moveWithCells="1"><xdr:from/><xdr:to/></anchor>"#,
        r#"</objectPr>"#
    );
    let (interner, properties): (_, ObjectProperties) = read(markup);

    for (name, actual) in [
        ("locked", properties.is_locked(&interner)),
        ("print", properties.is_printed(&interner)),
        ("autoFill", properties.fills_automatically(&interner)),
        ("autoLine", properties.outlines_automatically(&interner)),
        (
            "autoPict",
            properties.scales_picture_automatically(&interner),
        ),
    ] {
        assert_eq!(actual, Ok(true), "@{name} is absent and defaults to true");
    }
    assert_eq!(
        properties.has_default_size(&interner),
        Ok(false),
        "@defaultSize is written 0, which beats the true default"
    );
    for (name, actual) in [
        ("disabled", properties.is_disabled(&interner)),
        ("uiObject", properties.is_user_interface_object(&interner)),
        ("dde", properties.is_dynamic_data_exchange(&interner)),
    ] {
        assert_eq!(actual, Ok(false), "@{name} is absent and defaults to false");
    }

    assert_eq!(
        properties.alternative_text(&interner).unwrap().as_deref(),
        Some("A chart of Q1")
    );
    assert_eq!(
        properties.relationship_id(&interner, Some("r")),
        Ok(Some("rId9".to_owned())),
        "the embedded object part is named, and never resolved here"
    );
    assert!(
        properties
            .anchor()
            .expect("the anchor")
            .moves_with_cells(&interner)
            == Ok(true),
        "the anchor is reached through the same type E3 and E5 will use"
    );
}

// -------------------------------------------------------------------------------------------
// The six small worksheet children
// -------------------------------------------------------------------------------------------

/// `x:dataConsolidate` reads, and its `@function` is the **generated** variant rather than the wire
/// token.
#[test]
fn a_data_consolidation_reads_and_never_performs_one() {
    let sheet = sheet();
    let interner = sheet.interner();
    let consolidation = sheet
        .data_consolidation()
        .expect("the sheet writes x:dataConsolidate at rank 12");

    assert_eq!(
        consolidation.function(interner),
        Ok(DataConsolidateFunction::SampleStandardDeviation),
        "`stdDev` is SampleStandardDeviation; the generator renames it and the wire token does not"
    );
    assert_eq!(consolidation.uses_left_labels(interner), Ok(true));
    assert_eq!(
        consolidation.uses_top_labels(interner),
        Ok(false),
        "absent, and the schema default is false"
    );
    assert_eq!(
        consolidation.is_linked(interner),
        Ok(true),
        "@link is reported and never honoured — nothing here refreshes anything"
    );

    let refs = consolidation.references().expect("x:dataRefs");
    assert_eq!(refs.len(), 3);
    assert_eq!(
        refs.declared_count(interner),
        Ok(Some(5)),
        "@count is stale at 5 against three entries, and reading one never corrects it"
    );

    let entries: Vec<_> = refs.references().collect();
    assert_eq!(entries[0].range(interner), Ok(Some(range("A1:B4"))));
    assert_eq!(
        entries[0].sheet_name(interner).unwrap().as_deref(),
        Some("Overview"),
        "the sheet is named and never resolved to a tab"
    );
    assert_eq!(
        entries[1].name(interner).unwrap().as_deref(),
        Some("LastYear"),
        "a defined name, never resolved"
    );
    assert_eq!(
        entries[2].range(interner),
        Ok(None),
        "every attribute of CT_DataRef is optional and this one states none"
    );
    assert_eq!(entries[2].name(interner).unwrap(), None);
}

/// `x:customProperties` reads, and holds the `r:id` as text.
#[test]
fn custom_properties_read_and_hold_the_relationship_as_text() {
    let sheet = sheet();
    let interner = sheet.interner();
    let properties = sheet
        .custom_properties()
        .expect("the sheet writes x:customProperties at rank 25");
    assert_eq!(properties.len(), 1);

    let entry = properties.properties().next().expect("one customPr");
    assert_eq!(entry.name(interner).as_deref(), Ok("Provenance"));
    assert_eq!(
        entry.relationship_id(interner, sheet.relationship_prefix()),
        Ok(Some("rId3".to_owned())),
        "the property's value is in a part of its own, and this is the edge to it"
    );
}

/// `x:cellWatches` reads, in the order the file wrote them.
#[test]
fn cell_watches_read_in_document_order_and_nothing_sorts_them() {
    let sheet = sheet();
    let interner = sheet.interner();
    let watches = sheet
        .cell_watches()
        .expect("the sheet writes x:cellWatches at rank 26");
    let seen: Vec<_> = watches
        .watches()
        .map(|watch| watch.reference(interner).expect("@r"))
        .collect();
    assert_eq!(
        seen,
        vec![cell("D6"), cell("B2")],
        "D6 comes first in the file and comes first here"
    );
}

/// `x:ignoredErrors` reads its multi-range `@sqref` and its nine independent flags.
#[test]
fn ignored_errors_read_their_ranges_and_flags_and_suppress_nothing() {
    let sheet = sheet();
    let interner = sheet.interner();
    let block = sheet
        .ignored_errors()
        .expect("the sheet writes x:ignoredErrors at rank 27");
    assert_eq!(block.len(), 2);

    let records: Vec<_> = block.records().collect();
    assert_eq!(
        records[0].ranges(interner),
        Ok(ranges("C3:C5 D7")),
        "an ST_Sqref: one record can cover several disjoint ranges"
    );
    assert_eq!(records[0].ignores_number_stored_as_text(interner), Ok(true));
    assert_eq!(records[0].ignores_evaluation_error(interner), Ok(false));

    assert_eq!(records[1].ranges(interner), Ok(ranges("B2")));
    assert_eq!(
        records[1].ignores_evaluation_error(interner),
        Ok(true),
        "the flags are not mutually exclusive and this record sets two"
    );
    assert_eq!(records[1].ignores_unlocked_formula(interner), Ok(true));
    assert_eq!(records[1].ignores_calculated_column(interner), Ok(false));
}

/// Appending an `x:ignoredError` lands **before** the `extLst`, through the generated table.
///
/// `CT_IgnoredErrors` is the one type in this child whose sequence has two slots. A `push` that
/// appended would put the record after the `extLst`, which is a worksheet in schema-invalid order
/// written by this library — the failure `xsd:sequence` ordering exists to prevent.
#[test]
fn appending_an_ignored_error_lands_before_the_extension_list() {
    let mut sheet = sheet();
    let mut block = sheet.ignored_errors().expect("x:ignoredErrors").clone();
    assert!(
        block.content().iter().any(|item| matches!(
            item,
            mjx_sml::IgnoredErrorsContent::Raw(mjx_ooxml_core::RawNode::Element(_))
        )),
        "the fixture must carry an extLst here or this case proves nothing"
    );

    let interner = sheet.interner_mut();
    let mut record = mjx_sml::IgnoredError::new(interner, None);
    record.set_ranges(interner, ranges("Z9"));
    block.push(record);

    let emitted = rebuilt(&block, interner);
    let added = emitted
        .find(r#"sqref="Z9""#)
        .expect("the record is written");
    let ext = emitted.find("extLst").expect("the extLst is still there");
    assert!(
        added < ext,
        "rank 0 precedes rank 1 however the record was appended: {emitted}"
    );
}

/// `x:smartTags` reads three levels deep, and is not the workbook's `smartTagTypes`.
#[test]
fn worksheet_smart_tags_read_three_levels_deep() {
    let sheet = sheet();
    let interner = sheet.interner();
    let tags = sheet
        .smart_tags()
        .expect("the sheet writes x:smartTags at rank 28");
    assert_eq!(tags.len(), 1);

    let cell_tags = tags.cells().next().expect("one cellSmartTags");
    assert_eq!(
        cell_tags.reference(interner),
        Ok(cell("A4")),
        "the address is on the middle level, not on the tag"
    );

    let tag = cell_tags.tags().next().expect("one cellSmartTag");
    assert_eq!(
        tag.type_index(interner),
        Ok(1),
        "@type is an index into the workbook's smartTagTypes, and is never resolved here"
    );
    assert_eq!(tag.smart_tag_was_deleted(interner), Ok(true));
    assert_eq!(tag.is_xml_based(interner), Ok(false));

    let properties: Vec<_> = tag
        .properties()
        .map(|property| {
            (
                property.key(interner).expect("@key").into_owned(),
                property.value(interner).expect("@val").into_owned(),
            )
        })
        .collect();
    assert_eq!(
        properties,
        vec![
            ("Ticker".to_owned(), "MSFT".to_owned()),
            ("Exchange".to_owned(), "NASDAQ".to_owned()),
        ]
    );
}

/// `x:webPublishItems` reads, and its destination path is carried exactly.
#[test]
fn web_publish_items_read_and_the_destination_is_never_resolved() {
    let sheet = sheet();
    let interner = sheet.interner();
    let items = sheet
        .web_publish_items()
        .expect("the sheet writes x:webPublishItems at rank 36");
    assert_eq!(items.len(), 1);
    assert_eq!(
        items.declared_count(interner),
        Ok(Some(2)),
        "@count is stale at 2 against one item, and reading never corrects it"
    );

    let item = items.items().next().expect("one webPublishItem");
    assert_eq!(item.id(interner), Ok(4));
    assert_eq!(item.division_id(interner).as_deref(), Ok("links_q1"));
    assert_eq!(item.source_type(interner), Ok(WebSourceType::Range));
    assert_eq!(item.source_range(interner), Ok(Some(range("A1:D8"))));
    assert_eq!(
        item.destination_file(interner).as_deref(),
        Ok(r"\\fileserver\web\Q1%20numbers.htm"),
        "an untrusted path, carried exactly: not resolved, not decoded, not opened"
    );
    assert_eq!(item.republishes_automatically(interner), Ok(true));
}

// -------------------------------------------------------------------------------------------
// The slots nobody modelled, and the whole part
// -------------------------------------------------------------------------------------------

/// The two slots MJXOFF-127 deliberately left held still round-trip, in position.
///
/// `customSheetViews` (rank 13) is MJXOFF-129 (D17)'s — it embeds a `pageMargins`, which is D17's
/// own print block — and `phoneticPr` (rank 15) is nobody's yet. Both are *explicitly preserved*,
/// which is a claim this case checks rather than a hope.
#[test]
fn the_two_slots_this_child_did_not_model_are_still_held_in_position() {
    let sheet = sheet();
    let locals: Vec<&str> = sheet.child_element_locals().collect();
    assert_eq!(
        locals,
        vec![
            "dimension",
            "sheetData",
            "dataConsolidate",
            "customSheetViews",
            "phoneticPr",
            "hyperlinks",
            "customProperties",
            "cellWatches",
            "ignoredErrors",
            "smartTags",
            "webPublishItems",
        ],
        "every child, modelled or held, in the order the file wrote them"
    );

    // Held as raw nodes, not as modelled slots — which is what "explicitly preserved" means.
    let held: Vec<&str> = sheet
        .children()
        .filter_map(|child| match child {
            mjx_sml::WorksheetContent::Raw(mjx_ooxml_core::RawNode::Element(element)) => {
                Some(sheet.interner().resolve(element.name.local))
            }
            _ => None,
        })
        .collect();
    assert_eq!(held, vec!["customSheetViews", "phoneticPr"]);
}

/// Every new model survives a rebuild **from the model alone**, with no verbatim range anywhere.
///
/// The door the byte-identity suites cannot reach: an unedited part is one `memcpy`, and after an
/// edit elsewhere each slot still writes from its own stored bytes, so no tier-1 case ever executes
/// an `as_raw_element`. This one does, for all seven new worksheet slots at once.
#[test]
fn every_new_model_survives_a_rebuild_from_the_model_alone() {
    let mut sheet = sheet();
    // Cloned out first: `interner_mut` needs the part mutably and every child below borrows it.
    let hyperlinks_model = sheet.hyperlinks().expect("x:hyperlinks").clone();
    let consolidation_model = sheet
        .data_consolidation()
        .expect("x:dataConsolidate")
        .clone();
    let custom_model = sheet
        .custom_properties()
        .expect("x:customProperties")
        .clone();
    let watches_model = sheet.cell_watches().expect("x:cellWatches").clone();
    let ignored_model = sheet.ignored_errors().expect("x:ignoredErrors").clone();
    let tags_model = sheet.smart_tags().expect("x:smartTags").clone();
    let published_model = sheet
        .web_publish_items()
        .expect("x:webPublishItems")
        .clone();
    let out = sheet.interner_mut();

    let hyperlinks = rebuilt(&hyperlinks_model, out);
    assert!(hyperlinks.contains(r#"ref="B4:D6""#), "{hyperlinks}");
    assert!(hyperlinks.contains(r#"r:id="rId2""#), "{hyperlinks}");

    let consolidation = rebuilt(&consolidation_model, out);
    assert!(
        consolidation.contains(r#"function="stdDev""#),
        "{consolidation}"
    );
    assert!(
        consolidation.contains(r#"count="5""#),
        "the stale cache is re-emitted stale: {consolidation}"
    );
    assert!(consolidation.contains("<dataRef/>"), "{consolidation}");

    let custom = rebuilt(&custom_model, out);
    assert!(custom.contains(r#"name="Provenance""#), "{custom}");

    let watches = rebuilt(&watches_model, out);
    assert!(
        watches.find(r#"r="D6""#) < watches.find(r#"r="B2""#),
        "document order survives the rebuild: {watches}"
    );

    let ignored = rebuilt(&ignored_model, out);
    assert!(ignored.contains(r#"sqref="C3:C5 D7""#), "{ignored}");
    assert!(
        ignored.contains("extLst"),
        "the unmodelled child comes back too: {ignored}"
    );

    let tags = rebuilt(&tags_model, out);
    assert!(tags.contains(r#"key="Exchange""#), "{tags}");

    let published = rebuilt(&published_model, out);
    assert!(
        published.contains(r"\\fileserver\web\Q1%20numbers.htm"),
        "the untrusted path is re-emitted byte for byte: {published}"
    );
}

/// A worksheet nobody edited writes back the bytes it was read from.
///
/// Tier 1 at the markup tier: the whole part is one `extend_from_slice`, every new slot included.
#[test]
fn an_untouched_worksheet_writes_back_its_own_bytes() {
    let bytes = part_bytes("/xl/worksheets/sheet1.xml");
    let sheet = sheet();
    assert!(sheet.is_verbatim());
    assert_eq!(sheet.to_markup(), bytes);
}

/// Editing one slot leaves every **other** slot byte-identical, the six new ones included.
///
/// Tier 3 at the markup tier. The sheet is edited through the hyperlink surface — the only slot this
/// case touches — and the assertion is over the bytes of the untouched neighbours rather than over a
/// second run of the writer.
#[test]
fn editing_the_hyperlinks_slot_leaves_every_other_slot_byte_identical() {
    let original = String::from_utf8(part_bytes("/xl/worksheets/sheet1.xml")).expect("UTF-8");
    let mut sheet = sheet();
    assert!(sheet.remove_hyperlink(0).is_some());
    let edited = String::from_utf8(sheet.to_markup()).expect("UTF-8");

    assert!(
        !edited.contains(r#"ref="B4:D6""#),
        "the entry that was removed is gone"
    );
    for untouched in [
        r#"<dataConsolidate function="stdDev" leftLabels="1" link="1">"#,
        r#"<dataRefs count="5">"#,
        r#"<customSheetView guid="{5A2C8B04-1F3E-4C7D-9B61-0E4A7D2C8F35}" scale="85" showPageBreaks="1" showGridLines="0">"#,
        r#"<phoneticPr fontId="1" type="noConversion"/>"#,
        r#"<customPr name="Provenance" r:id="rId3"/>"#,
        r#"<cellWatch r="D6"/>"#,
        r#"<ignoredError sqref="C3:C5 D7" numberStoredAsText="1"/>"#,
        r#"<cellSmartTagPr key="Exchange" val="NASDAQ"/>"#,
        r#"<webPublishItems count="2">"#,
        r#"<c r="C3" t="inlineStr"><is><t>450</t></is></c>"#,
    ] {
        assert!(
            original.contains(untouched),
            "the fixture must carry {untouched} or this case proves nothing"
        );
        assert!(
            edited.contains(untouched),
            "an edit to x:hyperlinks must leave {untouched} byte-identical"
        );
    }
}
