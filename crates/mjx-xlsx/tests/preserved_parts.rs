//! **MJXOFF-133's gate.** *Preserved does not mean ignored.*
//!
//! Half of `sml.xsd` — 184 of its 367 complex types — describes features this library deliberately
//! does not model. The guarantee it makes about them is that **their bytes come back**, and a
//! guarantee nothing asserts is a guarantee that quietly stops being true: `mjx-vml` sat at 69
//! lines and `mjx-omml` at 13 for three phases on the strength of a "later phase" note with no
//! owner. This file is that note given a test.
//!
//! # The fixture, and what each choice makes visible
//!
//! `tests/fixtures/preserved_parts.xlsx` carries **one part of every cluster at once** — a pivot
//! table with its cache definition and cache records, an external link, a connections part, a query
//! table, an XML map, a cell metadata part, a volatile dependencies part, a single-cell table
//! definitions part, the three shared-workbook parts, and a Custom Property part. Fourteen preserved
//! parts. Splitting them across fixtures would let a part disappear in a file no case opened.
//!
//! Three choices in it are load-bearing:
//!
//! * **The pivot table hangs off `Report` (sheet 1), and its cache reads `Data` (sheet 2).** So an
//!   edit can be made *near* the preserved part or *far* from it, and
//!   [`every_preserved_part_survives_an_edit_on_the_pivot_tables_own_sheet`] and
//!   [`every_preserved_part_survives_an_edit_on_the_other_sheet`] are genuinely different cases.
//! * **`Report` carries a `x:customProperties` naming its Custom Property part.** Editing a cell on
//!   `Report` re-emits that worksheet **from the model**, so the `customPr@r:id` goes through
//!   `WorksheetContent::Raw`'s writer rather than through a `memcpy` — which is the one shape a
//!   byte-identity gate over an *untouched* part can never reach (MJXOFF-120's finding, running the
//!   other way).
//! * **`xl/xmlMaps.xml` is registered as `application/xml` and the Custom Property part as
//!   `…spreadsheetml.customProperty`.** Neither content type identifies its part on its own, so
//!   [`a_part_no_content_type_identifies_is_classified_by_the_edge_that_reaches_it`] fails for a
//!   classifier that only ever looked at `[Content_Types].xml`.
//!
//! # Why this cuts the other way from MJXOFF-120
//!
//! MJXOFF-120 found that a byte-identity gate cannot see a model-writer defect, because an untouched
//! part is one `memcpy`. **The deliverable here *is* byte identity**, so the same fact is a hazard
//! in the opposite direction: a preserved part coming back unchanged may prove only that nothing
//! went near it. Every survival case therefore asserts that the edit **really happened** — the
//! edited worksheet's bytes must differ — before comparing the preserved parts. A `save()` that did
//! nothing at all would pass the comparison and fail that assertion.
//!
//! # What each mutation would break
//!
//! * Dropping the pivot cache part on save turns
//!   [`every_preserved_part_survives_an_edit_on_the_pivot_tables_own_sheet`] red, naming the part.
//! * Widening `mjx-xlsx`'s schema-gate rule to swallow a pivot part without naming it turns
//!   `the_rule_still_rejects_every_shape_of_false_green` red in `tests/schema_gate.rs`.
//! * Classifying only by content type turns
//!   [`a_part_no_content_type_identifies_is_classified_by_the_edge_that_reaches_it`] red.
//! * Answering a pivot table's range from a re-parse of `workbook.xml` rather than from its own
//!   `x:location` turns [`the_surface_reports_the_pivot_tables_sheet_its_range_and_its_cache`] red.

use mjx_ooxml_core::{Interner, RawElement};
use mjx_opc::{Package, PartName, Relationship, TargetMode};
use mjx_sml::{
    CellRange, CellReference, CellValue, ExternalLinkTarget, PivotCacheSource, PivotTableIdentity,
};
use mjx_xlsx::{PartClassification, PartKind, SpreadsheetDefect, Workbook, XlsxError};

/// The fixture this suite is written against.
const FIXTURE: &str = "preserved_parts.xlsx";

/// The fixture, opened.
fn workbook() -> Workbook {
    Workbook::open(&mjx_fixtures::fixture(FIXTURE)).expect("the fixture opens")
}

/// A part name, or a panic naming it.
fn part(name: &str) -> PartName {
    PartName::new(name).expect("a valid part name")
}

/// Every preserved part of the fixture, as `(part name, bytes)`, read out of a fresh package.
fn preserved_bytes(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let workbook = Workbook::open(bytes).expect("the workbook opens");
    let package = Package::open(bytes).expect("the package opens");
    workbook
        .preserved_parts()
        .expect("the preserved part graph resolves")
        .all()
        .into_iter()
        .map(|(kind, part)| {
            let payload = package
                .part_bytes(&part)
                .unwrap_or_else(|| panic!("{kind:?} names {} but it is not there", part.as_str()))
                .to_vec();
            (part.as_str().to_owned(), payload)
        })
        .collect()
}

/// Opens the fixture, sets `B2` on the tab at `sheet_index`, saves, and returns the saved bytes.
fn edited(sheet_index: usize, value: f64) -> Vec<u8> {
    let mut workbook = workbook();
    workbook
        .set_cell_value(
            sheet_index,
            CellReference::parse("B2").expect("B2 parses"),
            CellValue::Number(value),
        )
        .expect("the cell is set");
    workbook.save().expect("the edited workbook saves")
}

/// The shared body of the two survival cases: the edit must have landed, and every preserved part
/// must be byte-identical to the one that went in.
fn assert_preserved_parts_survive(edited_sheet_part: &str, saved: &[u8]) {
    let original = mjx_fixtures::fixture(FIXTURE);
    let before = Package::open(&original).expect("the fixture opens");
    let after = Package::open(saved).expect("the saved workbook opens");

    // The edit really happened. Without this the comparison below is satisfied by a `save()` that
    // did nothing, which is exactly the vacuous pass a byte-identity gate invites.
    let edited_part = part(edited_sheet_part);
    assert_ne!(
        before.part_bytes(&edited_part),
        after.part_bytes(&edited_part),
        "{edited_sheet_part} was supposed to be edited and its bytes are unchanged — the rest of \
         this case would pass for a save that wrote nothing"
    );

    let before_preserved = preserved_bytes(&original);
    let after_preserved = preserved_bytes(saved);
    assert!(
        before_preserved.len() >= 14,
        "the fixture is supposed to carry fourteen preserved parts; it reports {}",
        before_preserved.len()
    );
    assert_eq!(
        before_preserved.iter().map(|(n, _)| n).collect::<Vec<_>>(),
        after_preserved.iter().map(|(n, _)| n).collect::<Vec<_>>(),
        "the set of preserved parts changed across the save"
    );
    for ((name, before), (_, after)) in before_preserved.iter().zip(&after_preserved) {
        assert_eq!(
            before,
            after,
            "{name} is preserved and must come back byte for byte; it is {} bytes and came back {}",
            before.len(),
            after.len()
        );
    }
}

/// The fixture really carries one part of every cluster — the non-discriminating-fixture guard.
///
/// Every later case in this file rests on this one: a fixture missing (say) the volatile
/// dependencies part would let every survival assertion pass while saying nothing about it.
#[test]
fn the_fixture_carries_one_part_of_every_preserved_cluster() {
    let workbook = workbook();
    let preserved = workbook.preserved_parts().expect("the graph resolves");

    assert_eq!(
        preserved.pivot_tables,
        vec![part("/xl/pivotTables/pivotTable1.xml")]
    );
    assert_eq!(
        preserved.pivot_cache_definitions,
        vec![part("/xl/pivotCache/pivotCacheDefinition1.xml")]
    );
    assert_eq!(
        preserved.pivot_cache_records,
        vec![part("/xl/pivotCache/pivotCacheRecords1.xml")]
    );
    assert_eq!(
        preserved.external_links,
        vec![part("/xl/externalLinks/externalLink1.xml")]
    );
    assert_eq!(preserved.connections, Some(part("/xl/connections.xml")));
    assert_eq!(
        preserved.query_tables,
        vec![part("/xl/queryTables/queryTable1.xml")]
    );
    assert_eq!(preserved.metadata, Some(part("/xl/metadata.xml")));
    assert_eq!(
        preserved.volatile_dependencies,
        Some(part("/xl/volatileDependencies.xml"))
    );
    assert_eq!(
        preserved.custom_xml_mappings,
        Some(part("/xl/xmlMaps.xml")),
        "xl/xmlMaps.xml is the part MJXOFF-88 §9 recorded as unowned; MJXOFF-133 claims it"
    );
    assert_eq!(
        preserved.single_cell_table_definitions,
        vec![part("/xl/tables/tableSingleCells1.xml")]
    );
    assert_eq!(
        preserved.revision_headers,
        Some(part("/xl/revisions/revisionHeaders.xml"))
    );
    assert_eq!(
        preserved.revision_logs,
        vec![part("/xl/revisions/revisionLog1.xml")]
    );
    assert_eq!(
        preserved.shared_workbook_user_data,
        Some(part("/xl/revisions/userNames.xml"))
    );
    assert_eq!(
        preserved.custom_properties,
        vec![part("/xl/customProperty1.bin")]
    );

    let kinds: Vec<PartKind> = preserved.all().into_iter().map(|(kind, _)| kind).collect();
    for expected in [
        PartKind::PivotTable,
        PartKind::PivotCacheDefinition,
        PartKind::PivotCacheRecords,
        PartKind::ExternalLink,
        PartKind::QueryTable,
        PartKind::SingleCellTableDefinitions,
        PartKind::RevisionLog,
        PartKind::CustomProperty,
        PartKind::Connections,
        PartKind::Metadata,
        PartKind::VolatileDependencies,
        PartKind::CustomXmlMappings,
        PartKind::RevisionHeaders,
        PartKind::SharedWorkbookUserData,
    ] {
        assert!(
            kinds.contains(&expected),
            "the fixture carries no {expected:?}, so nothing in this file asserts anything about it"
        );
    }
    assert_eq!(
        preserved.all().len(),
        14,
        "fourteen parts, kinds: {kinds:?}"
    );
    assert!(!preserved.is_empty());
}

/// **The deliverable.** Every preserved part comes back byte for byte after a cell is edited on the
/// very sheet the pivot table sits on.
///
/// *Near*, not merely elsewhere. Editing `Report!B2` makes `WorksheetPart` re-emit that whole
/// worksheet from its model, so the sheet's own `x:customProperties` — the `customPr@r:id` naming
/// the Custom Property part — is written by the slot writer rather than copied. Every relationship
/// in that sheet's `.rels`, and every part on the other end of one, has to come through that
/// unchanged.
#[test]
fn every_preserved_part_survives_an_edit_on_the_pivot_tables_own_sheet() {
    let saved = edited(0, 99.0);
    assert_preserved_parts_survive("/xl/worksheets/sheet1.xml", &saved);

    // …and the edited sheet still names its Custom Property part, so the surviving bytes are not
    // surviving as an orphan.
    let after = Workbook::open(&saved).expect("the saved workbook opens");
    assert_eq!(
        after.preserved_parts().expect("resolves").custom_properties,
        vec![part("/xl/customProperty1.bin")],
        "editing a cell re-emitted the worksheet from its model and lost the customPr edge"
    );
    assert_eq!(
        after
            .cell_text(0, CellReference::parse("B2").expect("B2"))
            .expect("the cell reads")
            .as_deref(),
        Some("99"),
        "the edit itself must have landed in the saved workbook"
    );
}

/// The same, for an edit on the *other* sheet — the one the pivot cache reads.
///
/// The far case, kept beside the near one because they exercise different machinery: this one never
/// re-emits `sheet1.xml` at all, so it is the pure part-level copy-on-write claim.
#[test]
fn every_preserved_part_survives_an_edit_on_the_other_sheet() {
    let saved = edited(1, 77.0);
    assert_preserved_parts_survive("/xl/worksheets/sheet2.xml", &saved);
}

/// Opening and saving with no edit at all leaves every part of the container alone — the tier-1
/// contract, restated for this fixture.
#[test]
fn an_untouched_workbook_re_emits_every_part_verbatim() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let saved = workbook().save().expect("an untouched workbook saves");
    let before = Package::open(&original).expect("open");
    let after = Package::open(&saved).expect("open");
    let names: Vec<String> = before.part_names().map(|p| p.as_str().to_owned()).collect();
    assert!(names.len() >= 26, "the fixture has {} parts", names.len());
    for name in &names {
        let part = part(name);
        assert_eq!(
            before.part_bytes(&part),
            after.part_bytes(&part),
            "{name} changed across an untouched round trip"
        );
    }
}

/// The identification surface answers the pivot table's name, sheet, range and cache — without a
/// model of the ninety-seven-type cluster behind it.
#[test]
fn the_surface_reports_the_pivot_tables_sheet_its_range_and_its_cache() {
    let workbook = workbook();
    let tables = workbook.pivot_tables().expect("the pivot tables read");
    assert_eq!(tables.len(), 1);
    let table = &tables[0];

    assert_eq!(table.part, part("/xl/pivotTables/pivotTable1.xml"));
    assert_eq!(table.sheet_index, 0);
    assert_eq!(table.sheet_name, "Report");
    assert_eq!(table.sheet_part, part("/xl/worksheets/sheet1.xml"));
    assert_eq!(table.identity.name, "RegionTotals");
    assert_eq!(table.identity.cache_id, 7);
    assert_eq!(table.identity.data_caption, "Values");
    assert_eq!(table.identity.location, "D3:E6");
    assert_eq!(
        table.identity.range(),
        Some(CellRange::parse("D3:E6").expect("D3:E6 parses"))
    );

    assert_eq!(
        table.cache_definition_part,
        Some(part("/xl/pivotCache/pivotCacheDefinition1.xml"))
    );
    assert_eq!(
        table.cache_records_part,
        Some(part("/xl/pivotCache/pivotCacheRecords1.xml"))
    );
    let cache = table.cache.as_ref().expect("the cache definition reads");
    assert_eq!(cache.record_count, Some(2));
    assert_eq!(cache.refreshed_by.as_deref(), Some("mjx-ooxml-rs"));
    assert_eq!(
        cache.source,
        PivotCacheSource::Worksheet {
            sheet: Some("Data".to_owned()),
            range: Some("A1:B3".to_owned()),
            name: None,
            relationship_id: None,
        },
        "the cache reads the other sheet, which is what makes the two survival cases differ"
    );

    // The workbook's own `pivotCache@cacheId` agrees with the table's `@cacheId` — the one
    // cross-part invariant this surface can state without a calculation model.
    let cache_id = table.identity.cache_id;
    let mut workbook = workbook;
    let listed = workbook
        .workbook_markup(|markup, interner| {
            markup.pivot_caches().map_or_else(Vec::new, |caches| {
                caches
                    .caches()
                    .map(|cache| cache.cache_id(interner))
                    .collect::<Vec<_>>()
            })
        })
        .expect("the workbook part reads");
    let listed: Vec<u32> = listed
        .into_iter()
        .map(|id| id.expect("pivotCache@cacheId is required"))
        .collect();
    assert!(
        listed.contains(&cache_id),
        "the table names cache {cache_id} and the workbook lists {listed:?}"
    );
}

/// The external link reports the book it names, its cached sheet names, and the URI its own
/// relationship carries.
#[test]
fn the_surface_reports_the_external_links_book_its_sheets_and_its_target() {
    let workbook = workbook();
    let links = workbook.external_links().expect("the external links read");
    assert_eq!(links.len(), 1);
    let link = &links[0];

    assert_eq!(link.part, part("/xl/externalLinks/externalLink1.xml"));
    assert_eq!(link.relationship_id.as_deref(), Some("rIdExternalLink"));
    assert_eq!(
        link.reference_index,
        Some(1),
        "the index comes from x:externalReferences, which is what a formula's [1] means"
    );
    assert_eq!(
        link.target.as_deref(),
        Some("file:///prices.xlsx"),
        "an untrusted URI, reported and never opened"
    );
    let ExternalLinkTarget::Workbook {
        relationship_id,
        sheet_names,
        defined_name_count,
    } = &link.identity.target
    else {
        panic!("the fixture's link is an externalBook: {:?}", link.identity);
    };
    assert_eq!(relationship_id.as_deref(), Some("rIdExternalBook"));
    assert_eq!(sheet_names, &["Prices".to_owned(), "Rates".to_owned()]);
    assert_eq!(*defined_name_count, 1);
}

/// The connections part reports each connection's name and where it says its data comes from.
#[test]
fn the_surface_reports_the_connections_name_and_source() {
    let workbook = workbook();
    let connections = workbook.connections().expect("the connections read");
    assert_eq!(connections.len(), 1);
    let connection = &connections[0];
    assert_eq!(connection.part, part("/xl/connections.xml"));
    assert_eq!(connection.identity.id, 1);
    assert_eq!(
        connection.identity.name.as_deref(),
        Some("Quarterly figures")
    );
    assert_eq!(
        connection.identity.description.as_deref(),
        Some("A text import nobody runs")
    );
    assert_eq!(
        connection.identity.source_file.as_deref(),
        Some("figures.txt"),
        "an untrusted path, reported and never opened"
    );
    assert_eq!(connection.identity.connection_type, Some(6));
}

/// The query table names the connection it reads, and its `refreshOnLoad` is reported rather than
/// obeyed.
#[test]
fn the_query_table_names_the_connection_it_reads_and_is_never_refreshed() {
    let workbook = workbook();
    let query_tables = workbook.query_tables().expect("the query tables read");
    assert_eq!(query_tables.len(), 1);
    let query_table = &query_tables[0];
    assert_eq!(query_table.part, part("/xl/queryTables/queryTable1.xml"));
    assert_eq!(query_table.sheet_index, 0);
    assert_eq!(query_table.sheet_name, "Report");
    assert_eq!(query_table.identity.name, "ExternalData_1");
    assert!(query_table.identity.refresh_on_load);

    let connections = workbook.connections().expect("the connections read");
    assert!(
        connections
            .iter()
            .any(|connection| connection.identity.id == query_table.identity.connection_id),
        "the query table reads connection {} and the workbook declares {:?}",
        query_table.identity.connection_id,
        connections
            .iter()
            .map(|connection| connection.identity.id)
            .collect::<Vec<_>>()
    );

    // Reported, never obeyed: nothing about asking for a refresh changes a byte of the part.
    let saved = workbook.save().expect("saves");
    let before = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("open");
    let after = Package::open(&saved).expect("open");
    let part = part("/xl/queryTables/queryTable1.xml");
    assert_eq!(before.part_bytes(&part), after.part_bytes(&part));
}

/// The XML map reports its schema, its root element and the namespaces its XPaths run against —
/// the §9 item this child claims.
///
/// `xl/xmlMaps.xml` was recorded in MJXOFF-88 §9 as a part no unit owned: nothing named its
/// relationship type, its content type, or its markup. This is the case that closes it.
#[test]
fn the_xml_map_reports_its_schema_and_root_element() {
    let workbook = workbook();
    let maps = workbook
        .xml_maps()
        .expect("the mappings part reads")
        .expect("the fixture has one");
    assert_eq!(maps.part, part("/xl/xmlMaps.xml"));
    assert_eq!(
        maps.identity.selection_namespaces,
        "xmlns:ns1='urn:example:orders'"
    );
    assert_eq!(maps.identity.schema_ids, vec!["Schema1".to_owned()]);
    assert_eq!(maps.identity.maps.len(), 1);
    let map = &maps.identity.maps[0];
    assert_eq!(map.id, 1);
    assert_eq!(map.name, "order_Map");
    assert_eq!(map.root_element, "order");
    assert_eq!(map.schema_id, "Schema1");
    assert!(
        maps.identity.schema_ids.contains(&map.schema_id),
        "a map names a schema the same part declares"
    );
}

/// The workbook reports that it is shared, who shares it, and how many editing sessions it has —
/// without modelling a single revision record.
#[test]
fn the_workbook_reports_shared_mode_and_who_shares_it() {
    let workbook = workbook();
    let state = workbook.revision_state().expect("the revision state reads");
    assert!(state.is_shared);
    assert_eq!(
        state.headers_part,
        Some(part("/xl/revisions/revisionHeaders.xml"))
    );
    assert_eq!(
        state.log_parts,
        vec![part("/xl/revisions/revisionLog1.xml")]
    );
    let headers = state.headers.as_ref().expect("the headers read");
    assert_eq!(headers.guid, "{1FA7B2B1-1111-4A4A-9E9E-0F0F0F0F0F01}");
    assert!(headers.shared);
    assert!(headers.tracks_revisions);
    assert_eq!(headers.preserve_history_days, 30, "the schema default");
    assert_eq!(headers.sessions.len(), 1);
    assert_eq!(headers.sessions[0].user_name, "Jai Shukla");
    assert_eq!(
        headers.sessions[0].relationship_id.as_deref(),
        Some("rIdRevisionLog1")
    );

    let users = state.users.as_ref().expect("the user data reads");
    assert_eq!(users.stated_count, Some(1));
    assert_eq!(users.users.len(), 1);
    assert_eq!(users.users[0].name, "Jai Shukla");
    assert_eq!(
        users.users[0].id, -1,
        "CT_SharedUser@id is xsd:int and a negative one is reported as written"
    );
}

/// A workbook with none of these parts says so, and every accessor answers empty rather than
/// erroring.
///
/// The negative half of every case above: without it, an accessor that always answered empty would
/// pass nothing here and fail nothing either.
#[test]
fn a_workbook_with_no_preserved_parts_reports_none_of_them() {
    let workbook = Workbook::open(&mjx_fixtures::fixture("sample.xlsx")).expect("opens");
    let preserved = workbook.preserved_parts().expect("resolves");
    assert!(preserved.is_empty(), "sample.xlsx: {preserved:?}");
    assert!(workbook.pivot_tables().expect("reads").is_empty());
    assert!(workbook.external_links().expect("reads").is_empty());
    assert!(workbook.connections().expect("reads").is_empty());
    assert!(workbook.query_tables().expect("reads").is_empty());
    assert_eq!(workbook.xml_maps().expect("reads"), None);
    let state = workbook.revision_state().expect("reads");
    assert!(!state.is_shared);
    assert_eq!(state.headers, None);
    assert!(state.log_parts.is_empty());
}

/// A part whose content type identifies nothing is still classified — by the relationship that
/// reaches it.
///
/// `xl/xmlMaps.xml` is registered as `application/xml`, which §12.3.6 gives it and which is also
/// what `[Content_Types].xml` most often makes the `Default` for the `xml` extension; classifying
/// from it would misidentify every unregistered part in a real package. The Custom Property part
/// carries "any content, support for which is application-defined" (§12.3.5). Both are identified by
/// their edge instead. This case fails for a classifier that only ever looked at content types.
#[test]
fn a_part_no_content_type_identifies_is_classified_by_the_edge_that_reaches_it() {
    let package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("opens");

    assert_eq!(
        PartKind::from_content_type("application/xml"),
        None,
        "application/xml identifies nothing on its own"
    );
    assert_eq!(
        mjx_xlsx::preserve::classify(&package, &part("/xl/xmlMaps.xml")),
        PartClassification::Classified(PartKind::CustomXmlMappings)
    );
    assert_eq!(
        mjx_xlsx::preserve::classify(&package, &part("/xl/customProperty1.bin")),
        PartClassification::Classified(PartKind::CustomProperty)
    );

    // …and the inventory, which builds the same answer for every part in one pass, agrees.
    let workbook = workbook();
    let inventory = workbook.part_inventory();
    for (name, kind) in [
        ("/xl/xmlMaps.xml", PartKind::CustomXmlMappings),
        ("/xl/customProperty1.bin", PartKind::CustomProperty),
        (
            "/xl/revisions/revisionLog1.xml",
            PartKind::RevisionLog, // this one *is* identified by its content type
        ),
    ] {
        let row = inventory
            .iter()
            .find(|row| row.part.as_str() == name)
            .unwrap_or_else(|| panic!("{name} is not in the inventory"));
        assert_eq!(
            row.classification,
            PartClassification::Classified(kind),
            "{name} classified as {:?}",
            row.classification
        );
    }
    // Every part *under `xl/`* is classified — which before MJXOFF-133 was false for two of them.
    // The relationship items and `docProps/` are outside `PartKind` by design: a `.rels` is not a
    // part `PartKind` names, and document properties are `mjx-opc`'s and `mjx-docx`'s vocabulary.
    let unclassified: Vec<String> = inventory
        .iter()
        .filter(|row| {
            row.part.as_str().starts_with("/xl/") && !row.part.as_str().ends_with(".rels")
        })
        .filter(|row| row.classification == PartClassification::Unclassified)
        .map(|row| row.part.as_str().to_owned())
        .collect();
    assert!(
        unclassified.is_empty(),
        "every part under xl/ is now classified; these are not: {unclassified:?}"
    );
}

/// A pivot table whose `location/@ref` is not a range is **reported**, not repaired and not
/// panicked on.
///
/// Authored here rather than committed as a fixture: the corpus is swept by the schema gate and a
/// deliberately invalid `ST_Ref` would have to be tolerated there for no gain. What matters is that
/// the identification surface survives it, which this asks of the reader directly.
#[test]
fn a_pivot_table_with_a_malformed_location_is_reported_rather_than_panicked_on() {
    let markup = concat!(
        r#"<pivotTableDefinition xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" "#,
        r#"name="Broken" cacheId="7" dataCaption="Values">"#,
        r#"<location ref="not a range" firstHeaderRow="1" firstDataRow="1" firstDataCol="1"/>"#,
        r#"</pivotTableDefinition>"#,
    );
    let document = mjx_xml::fidelity::parse(markup.as_bytes()).expect("well-formed");
    let identity = read_pivot_table(&document.root, &document.interner)
        .expect("a malformed ref is not an error")
        .expect("the root is a pivotTableDefinition");
    assert_eq!(identity.location, "not a range", "reported as written");
    assert_eq!(
        identity.range(),
        None,
        "and never repaired into a rectangle"
    );

    // A *missing* required attribute is different, and is an error rather than a substitution.
    let missing = concat!(
        r#"<pivotTableDefinition xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" "#,
        r#"cacheId="7" dataCaption="Values">"#,
        r#"<location ref="A1:B2" firstHeaderRow="1" firstDataRow="1" firstDataCol="1"/>"#,
        r#"</pivotTableDefinition>"#,
    );
    let document = mjx_xml::fidelity::parse(missing.as_bytes()).expect("well-formed");
    let error = read_pivot_table(&document.root, &document.interner)
        .expect_err("an absent use=\"required\" @name has no default to substitute");
    assert!(
        error.to_string().contains("name"),
        "the error must name the attribute: {error}"
    );

    // …and one with no `x:location` at all, which `CT_pivotTableDefinition` makes mandatory.
    let no_location = concat!(
        r#"<pivotTableDefinition xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" "#,
        r#"name="Broken" cacheId="7" dataCaption="Values"/>"#,
    );
    let document = mjx_xml::fidelity::parse(no_location.as_bytes()).expect("well-formed");
    read_pivot_table(&document.root, &document.interner)
        .expect_err("a pivot table with no x:location is reported");
}

/// [`PivotTableIdentity::read_root`] with the error type this suite asserts on.
fn read_pivot_table(
    root: &RawElement,
    interner: &Interner,
) -> Result<Option<PivotTableIdentity>, mjx_sml::SmlError> {
    PivotTableIdentity::read_root(root, interner)
}

/// A `pivotCache@r:id` leading to a part that is not a pivot cache definition is a defect the
/// workbook refuses to save. The same for an `externalReference@r:id`.
///
/// The direction packaging cannot see. `mjx-opc` reports an `r:id` that names *no* relationship;
/// only SpreadsheetML knows what kind of part the one it does name has to be.
///
/// **The entry is repointed in the markup, not the relationship in the `.rels`.** Retargeting the
/// relationship instead would leave the link part unreachable and trip
/// [`SpreadsheetDefect::UnreachableSpreadsheetPart`] first — a different defect, correctly reported,
/// and not the one this case is about. Repointing the entry at `rIdStyles` leaves every relationship
/// and every part exactly where it was, so the *only* thing wrong with the package is the kind of
/// part the entry names.
#[test]
fn a_workbook_reference_leading_to_the_wrong_kind_of_part_is_refused() {
    for (element, expected_target) in [
        ("pivotCache", "/xl/styles.xml"),
        ("externalReference", "/xl/styles.xml"),
    ] {
        let mut package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("opens");
        let workbook_part = part("/xl/workbook.xml");
        {
            let document = package
                .part_tree_mut(&workbook_part)
                .expect("the workbook part parses");
            let mjx_ooxml_core::RawDocument { interner, root, .. } = document;
            assert!(
                repoint_first(root, interner, element, "rIdStyles"),
                "the fixture has no x:{element} to repoint"
            );
        }

        let workbook = Workbook::from_package(package).expect("still opens");
        let error = workbook
            .save()
            .expect_err("an entry leading to the styles part is a defect");
        let XlsxError::InvalidWorkbook(defect) = &error else {
            panic!("expected an InvalidWorkbook, got: {error}");
        };
        let SpreadsheetDefect::WorkbookReferenceTargetIsWrongKind {
            element: reported,
            relationship_id,
            target_part,
            expected_content_type,
            ..
        } = &**defect
        else {
            panic!("expected a WorkbookReferenceTargetIsWrongKind, got: {defect}");
        };
        assert_eq!(*reported, element);
        assert_eq!(relationship_id, "rIdStyles");
        assert_eq!(target_part, expected_target);
        assert!(
            expected_content_type.contains(if element == "pivotCache" {
                "pivotCacheDefinition"
            } else {
                "externalLink"
            }),
            "the defect must say what the entry was supposed to reach: {expected_content_type}"
        );
    }
}

/// Rewrites the `r:id` of the first SpreadsheetML element named `local`, depth first.
fn repoint_first(
    element: &mut RawElement,
    interner: &mut Interner,
    local: &str,
    relationship_id: &str,
) -> bool {
    let matches = element
        .name
        .namespace
        .is_some_and(|ns| interner.resolve(ns) == SML_NS)
        && interner.resolve(element.name.local) == local;
    if matches {
        mjx_xml::attribute::set(
            &mut element.attributes,
            interner,
            Some("r"),
            "id",
            relationship_id,
        );
        return true;
    }
    for child in &mut element.children {
        if let mjx_ooxml_core::RawNode::Element(child) = child {
            if repoint_first(child, interner, local, relationship_id) {
                return true;
            }
        }
    }
    false
}

/// The SpreadsheetML namespace, as `sml.xsd` declares it.
const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// The same relationships, left alone, do **not** trip the check.
///
/// Without this the case above would pass for a check that faulted every workbook with a pivot
/// cache at all.
#[test]
fn a_workbook_whose_reference_targets_are_right_still_saves_after_an_edit() {
    let mut workbook = workbook();
    workbook
        .rename_sheet(0, "Renamed")
        .expect("the rename lands");
    let saved = workbook.save().expect("a correct workbook still saves");
    assert_preserved_parts_survive("/xl/workbook.xml", &saved);
}

/// Reading the identification surface does not dirty a single part.
///
/// The claim every accessor's documentation makes, asserted once: ask every question, then save, and
/// compare the whole container.
#[test]
fn reading_the_identification_surface_dirties_nothing() {
    let workbook = workbook();
    let _ = workbook.preserved_parts().expect("reads");
    let _ = workbook.pivot_tables().expect("reads");
    let _ = workbook.external_links().expect("reads");
    let _ = workbook.connections().expect("reads");
    let _ = workbook.query_tables().expect("reads");
    let _ = workbook.xml_maps().expect("reads");
    let _ = workbook.revision_state().expect("reads");

    let original = mjx_fixtures::fixture(FIXTURE);
    let saved = workbook.save().expect("saves");
    let before = Package::open(&original).expect("open");
    let after = Package::open(&saved).expect("open");
    for name in before.part_names() {
        assert_eq!(
            before.part_bytes(&name),
            after.part_bytes(&name),
            "{} changed after nothing but reads",
            name.as_str()
        );
    }
}

/// A relationship that resolves to nothing yields a `None` part rather than an error.
///
/// `mjx-opc` reports a dangling reference; restating it here would be a second, drifting
/// implementation of one rule. Built on an authored package so the shape is reachable at all.
#[test]
fn a_pivot_table_whose_cache_relationship_is_missing_is_reported_with_no_cache() {
    let mut package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("opens");
    package
        .remove_relationship(
            Some(&part("/xl/pivotTables/pivotTable1.xml")),
            "rIdCacheDefinition",
        )
        .expect("the pivot table's cache relationship is removed");
    let bytes = package.save_unchecked().expect("saves anyway");

    let workbook = Workbook::open(&bytes).expect("opens");
    let tables = workbook
        .pivot_tables()
        .expect("the pivot tables still read");
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].cache_definition_part, None);
    assert_eq!(tables[0].cache_records_part, None);
    assert_eq!(tables[0].cache, None);
    assert_eq!(
        tables[0].identity.name, "RegionTotals",
        "the table is still identified; only its cache edge is gone"
    );
}

/// An external-link relationship the workbook's `x:externalReferences` never lists is reported with
/// no index rather than renumbered.
#[test]
fn an_unlisted_external_link_keeps_its_part_and_loses_its_index() {
    let mut package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("opens");
    let workbook_part = part("/xl/workbook.xml");
    package
        .add_relationship(
            Some(&workbook_part),
            Relationship {
                id: "rIdSecondLink".to_owned(),
                rel_type: mjx_xlsx::parts::REL_EXTERNAL_LINK.to_owned(),
                target: "externalLinks/externalLink1.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("a second relationship to the same link part");
    let bytes = package.save_unchecked().expect("saves");

    let workbook = Workbook::open(&bytes).expect("opens");
    let links = workbook.external_links().expect("reads");
    assert_eq!(
        links.len(),
        2,
        "both relationships reach a link part, so both are reported"
    );
    assert!(
        links.iter().any(|link| link.reference_index == Some(1)),
        "the listed one keeps its [1] index"
    );
}
