//! `word/settings.xml`, `word/webSettings.xml`, `word/fontTable.xml` and `word/recipients.xml`
//! (MJXOFF-136).
//!
//! `settings_document_configuration.docx` is the adversarial fixture the ticket's own trap names:
//! `word/settings.xml` carries `w14:`/`w15:` extension elements **interleaved between** modelled
//! ones, two `w:compatSetting` entries inside `w:compat`, and a `w:docVars` block — a model that
//! silently drops what it does not know still passes "zoom reads as 100"; it does not survive
//! [`unknown_bucket_order_is_exactly_preserved`] below. The same package also carries
//! `word/webSettings.xml` (a `w:divs` tree), `word/fontTable.xml` (one embedded font, obfuscation
//! key included) and `word/recipients.xml` (two rows) — every part `wml.xsd` gives this child, all
//! four in one package so the byte-identical round-trip proof below covers all four at once.

use mjx_docx::Document;
use mjx_fixtures::fixture;
use mjx_opc::{Package, PartName};

fn document() -> Document {
    Document::open(&fixture("settings_document_configuration.docx"))
        .expect("open settings_document_configuration.docx")
}

fn part_bytes<'a>(pkg: &'a Package, name: &str) -> &'a [u8] {
    pkg.part_bytes(&PartName::new(name).expect("valid part name"))
        .unwrap_or_else(|| panic!("{name} is missing from the package"))
}

// -------------------------------------------------------------------------------------------
// Byte-identical round trip, all four parts, forced through the typed model with a no-op edit —
// the same forcing step `tests/styles.rs`'s own
// `sample_docx_styles_xml_round_trips_byte_identically_when_untouched` uses: mjx-opc's part-level
// copy-on-write already reproduces an untouched part verbatim before a line of this crate runs, so
// a bare open/save would prove nothing about `write_back`.
// -------------------------------------------------------------------------------------------

#[test]
fn all_four_parts_round_trip_byte_identically_through_the_typed_model() {
    let original = fixture("settings_document_configuration.docx");
    let mut doc = Document::open(&original).expect("open fixture");

    doc.edit_document_settings(|_settings, _interner| {})
        .expect("materialize word/settings.xml through DocumentSettings with a no-op edit");
    doc.edit_web_settings(|_settings, _interner| {})
        .expect("materialize word/webSettings.xml through WebSettings with a no-op edit");
    doc.edit_font_table(|_table, _interner| {})
        .expect("materialize word/fontTable.xml through FontTable with a no-op edit");
    doc.edit_recipients(|_recipients, _interner| {})
        .expect("materialize word/recipients.xml through Recipients with a no-op edit");

    let saved = doc.save().expect("save");
    let original_pkg = Package::open(&original).expect("open original");
    let saved_pkg = Package::open(&saved).expect("open saved");

    for part in [
        "/word/settings.xml",
        "/word/webSettings.xml",
        "/word/fontTable.xml",
        "/word/recipients.xml",
    ] {
        assert_eq!(
            part_bytes(&original_pkg, part),
            part_bytes(&saved_pkg, part),
            "{part} must round-trip byte-identically through the typed model when untouched"
        );
    }
}

/// The embedded font's own binary payload and its relationship survive a save that never touches
/// `word/fontTable.xml` at all — proof the OPC part/relationship machinery, not this crate, is what
/// carries the payload, exactly as the module's own doc comment says.
#[test]
fn the_embedded_fonts_binary_payload_and_relationship_survive_an_unrelated_save() {
    let original = fixture("settings_document_configuration.docx");
    let mut doc = Document::open(&original).expect("open fixture");
    // An edit to a wholly different part (settings.xml) must not perturb fontTable.xml's own
    // binary relationship target.
    doc.edit_document_settings(|settings, interner| {
        settings.set_do_not_embed_smart_tags(interner, Some(true));
    })
    .expect("edit settings.xml");
    let saved = doc.save().expect("save");

    let original_pkg = Package::open(&original).expect("open original");
    let saved_pkg = Package::open(&saved).expect("open saved");
    let font_part = PartName::new("/word/fonts/font1.fntdata").expect("valid part name");
    assert_eq!(
        original_pkg.part_bytes(&font_part),
        saved_pkg.part_bytes(&font_part),
        "the embedded font's binary payload must survive an unrelated edit untouched"
    );
    assert_eq!(
        part_bytes(&original_pkg, "/word/fontTable.xml"),
        part_bytes(&saved_pkg, "/word/fontTable.xml"),
        "word/fontTable.xml itself (the fontKey obfuscation key included) must be untouched"
    );
}

// -------------------------------------------------------------------------------------------
// The unknown-bucket trap: interleaved w14:/w15: elements, two w:compatSetting entries and a
// w:docVars block, all read back through the typed model and asserted in place.
// -------------------------------------------------------------------------------------------

/// Reads `word/settings.xml` back through [`Document::document_settings`] and confirms every
/// known setting the fixture carries is reachable **and** that the raw content order still
/// interleaves the `w14:`/`w15:` extensions between them exactly where the fixture put them —
/// the assertion a model that silently drops unknown elements cannot pass, because dropping one
/// changes both the count and the relative positions asserted here.
#[test]
fn unknown_bucket_order_is_exactly_preserved() {
    let mut doc = document();
    doc.document_settings(|settings, interner| {
        // Known settings the fixture states, each reachable through its own accessor.
        let zoom = settings.zoom().expect("w:zoom is present");
        assert_eq!(zoom.percent(interner).unwrap().to_wire(), "100");
        assert_eq!(settings.mirror_margins(interner).unwrap(), Some(true));
        assert_eq!(settings.even_and_odd_headers(interner).unwrap(), Some(true));
        assert!(settings.document_protection().is_some());
        let compat = settings.compat().expect("w:compat is present");
        assert_eq!(compat.settings().count(), 2, "both w:compatSetting entries must be reachable");
        assert!(settings.doc_vars().is_some());
        assert!(settings.rsids().is_some());

        // The interleaving itself: walk `content()` in document order and record which entries
        // are known (`Some(local)`) vs. unknown (`None`, `SettingsContent::Raw`).
        let shape: Vec<bool> = settings
            .content()
            .iter()
            .map(|item| !matches!(item, mjx_docx::SettingsContent::Raw(_)))
            .collect();
        // The fixture, in document order: zoom(known), w14:docId(unknown), mirrorMargins(known),
        // w15:chartTrackingRefBased(unknown), evenAndOddHeaders(known), documentProtection(known),
        // defaultTabStop(known), autoHyphenation(known), compat(known, 2 compatSettings inside —
        // not visible at this top level), docVars(known), rsids(known).
        assert_eq!(
            shape,
            vec![true, false, true, false, true, true, true, true, true, true, true],
            "the known/unknown shape of word/settings.xml's own top-level content must match the \
             fixture exactly, in order: {shape:?}"
        );

        // Zoom in on the two unknown positions specifically: they must be the w14:/w15: elements,
        // not e.g. two copies of one or the other, and they must sit exactly where the fixture
        // put them (immediately after zoom, and immediately after mirrorMargins).
        let raw_locals: Vec<String> = settings
            .content()
            .iter()
            .filter_map(|item| match item {
                mjx_docx::SettingsContent::Raw(mjx_ooxml_core::RawNode::Element(element)) => {
                    Some(interner.resolve(element.name.local).to_owned())
                }
                _ => None,
            })
            .collect();
        assert_eq!(raw_locals, vec!["docId", "chartTrackingRefBased"]);
    })
    .expect("read word/settings.xml")
    .expect("fixture has word/settings.xml");
}

/// [`unknown_bucket_order_is_exactly_preserved`]'s own negative control. Proved once, by hand,
/// not automated here: `DocumentSettings::content` (`settings.rs`) was temporarily changed to
/// `.filter(|item| !matches!(item, SettingsContent::Raw(_)))` — simulating a model that silently
/// drops what it does not know — which turned `unknown_bucket_order_is_exactly_preserved` red:
///
/// ```text
/// thread 'unknown_bucket_order_is_exactly_preserved' panicked:
/// assertion `left == right` failed: the known/unknown shape of word/settings.xml's own top-level
/// content must match the fixture exactly, in order: [true, true, true, true, true, true, true,
/// true, true]
///   left: [true, true, true, true, true, true, true, true, true]
///  right: [true, false, true, false, true, true, true, true, true, true, true]
/// ```
///
/// Reverted by re-editing (never `git checkout --`) once the red run was captured; this crate's
/// own suite is green again. This function stays `#[ignore]` in shipped code: it is not a test
/// itself, it is the harness `regenerate_fixtures`-style functions in this crate's other test
/// files use to document *how* a claim was checked, not to check it on every run.
#[test]
#[ignore = "documents the mutation-proof captured above; not a test to run"]
fn mutation_proof_is_documented_not_automated() {}

// -------------------------------------------------------------------------------------------
// The four parts' own typed reads: web settings, font table, recipients.
// -------------------------------------------------------------------------------------------

#[test]
fn web_settings_divs_and_flags_read_back() {
    let mut doc = document();
    doc.web_settings(|settings, interner| {
        assert_eq!(
            settings.optimize_for_browser().unwrap().value(interner).unwrap(),
            Some(true)
        );
        assert_eq!(settings.allow_png(interner).unwrap(), Some(true));
        let divs = settings.divs().expect("w:divs is present");
        let div = divs.by_id(interner, 1_234_567_890).expect("div id 1234567890");
        assert_eq!(
            div.margin_left()
                .unwrap()
                .twentieths_of_a_point(interner)
                .unwrap()
                .to_wire(),
            "0"
        );
        let target = settings.target_screen_size().unwrap();
        assert_eq!(target.size(interner).unwrap().to_string(), "1024x768");
    })
    .expect("read word/webSettings.xml")
    .expect("fixture has word/webSettings.xml");
}

#[test]
fn font_table_resolves_the_embedded_font_and_its_obfuscation_key() {
    let mut doc = document();
    doc.font_table(|table, interner| {
        let font = table
            .font(interner, "EmbeddedSample")
            .expect("EmbeddedSample is in the font table");
        let embed = font.embed_regular().expect("w:embedRegular is present");
        assert_eq!(
            embed.relationship_id(interner).unwrap(),
            "rIdFont1"
        );
        assert_eq!(
            embed.font_key(interner).unwrap().as_deref(),
            Some("{12345678-1234-1234-1234-123456789ABC}"),
            "the obfuscation key must be preserved exactly, never decoded"
        );
    })
    .expect("read word/fontTable.xml")
    .expect("fixture has word/fontTable.xml");
}

#[test]
fn recipients_rows_read_back_in_order() {
    let mut doc = document();
    doc.recipients(|recipients, interner| {
        let rows: Vec<_> = recipients.rows().collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].active(interner).unwrap(),
            Some(true),
            "a present <w:active/> with no val defaults to true, per CT_OnOff"
        );
        assert_eq!(rows[0].column().value(interner).unwrap(), 0);
        assert_eq!(rows[1].active(interner).unwrap(), Some(false));
        assert_eq!(rows[1].column().value(interner).unwrap(), 1);
    })
    .expect("read word/recipients.xml")
    .expect("fixture has word/recipients.xml");
}

// -------------------------------------------------------------------------------------------
// The document-protection password hash: preserved exactly, never recomputed.
// -------------------------------------------------------------------------------------------

#[test]
fn document_protection_hash_is_preserved_exactly() {
    let mut doc = document();
    let (hash, salt) = doc
        .document_settings(|settings, interner| {
            let protection = settings
                .document_protection()
                .expect("w:documentProtection is present");
            (
                protection.hash_value(interner).unwrap().map(|s| s.into_owned()),
                protection.salt_value(interner).unwrap().map(|s| s.into_owned()),
            )
        })
        .expect("read word/settings.xml")
        .expect("fixture has word/settings.xml");
    assert_eq!(
        hash.as_deref(),
        Some("MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5MDEyMw==")
    );
    assert_eq!(
        salt.as_deref(),
        Some("c2FsdC12YWx1ZS1ieXRlcy0wMTIzNDU2Nzg5YWI=")
    );

    // Editing an unrelated flag on the same w:settings must not touch the hash — the "never
    // recomputed" half of the claim.
    doc.edit_document_settings(|settings, interner| {
        settings.set_mirror_margins(interner, Some(false));
    })
    .expect("edit an unrelated flag");
    let saved = doc.save().expect("save");
    let mut reopened = Document::open(&saved).expect("reopen");
    let hash_after = reopened
        .document_settings(|settings, interner| {
            settings
                .document_protection()
                .and_then(|p| p.hash_value(interner).unwrap())
                .map(|s| s.into_owned())
        })
        .expect("read word/settings.xml")
        .expect("fixture has word/settings.xml");
    assert_eq!(
        hash_after.as_deref(),
        Some("MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5MDEyMw=="),
        "the password hash must survive an edit to an unrelated setting untouched"
    );
}

// -------------------------------------------------------------------------------------------
// MJXOFF-113's ad-hoc read, replaced: even_and_odd_headers now goes through DocumentSettings.
// -------------------------------------------------------------------------------------------

#[test]
fn even_and_odd_headers_reads_through_the_typed_model() {
    let mut doc = document();
    assert!(doc.even_and_odd_headers().expect("read the flag"));

    let mut blank = mjx_docx::Document::blank(mjx_docx::PageSize::a4()).expect("blank");
    assert!(
        !blank.even_and_odd_headers().expect("read the flag"),
        "a document with no word/settings.xml at all must read false, not error"
    );
}
