//! The third category, proved: an authored part in a namespace on neither list **fails**.
//!
//! This is the assertion that makes the other two categories mean anything. Without it the rule
//! degenerates into "skip anything we have no arm for", which is the silent skip the whole crate
//! exists to close — and which is precisely how `sample.docx`'s four WordprocessingML parts sat
//! outside every gate for the entire first phase of this project while the suite reported green.
//!
//! Two forms of the same rule are proved here, because they are enforced in two places:
//!
//! * **byte level**, over a saved package — [`mjx_schema_gate::inspect_deck`] reports
//!   [`PartOutcome::Uncategorised`] and [`mjx_schema_gate::assert_rows_are_valid`] fails on it;
//! * **provenance-exact**, over a live package — [`assert_authored_parts_are_categorised`] reads
//!   `Package::authored_xml_parts`, which is the form a format crate holding its own package should
//!   call. Provenance does not survive a save, which is why the byte-level form applies the rule to
//!   every part instead of only to authored ones: strictly stricter, never weaker.

use mjx_opc::{Package, PartName};
use mjx_schema_gate::{
    assert_authored_parts_are_categorised, assert_rows_are_valid, harness, inspect_deck,
    PartOutcome,
};

/// A namespace no OOXML schema defines and no allowlist entry names.
const INVENTED_NS: &str = "urn:example:a-namespace-on-no-list";

/// A package holding one authored part rooted in [`INVENTED_NS`].
fn package_with_an_uncategorised_authored_part() -> Package {
    let mut package = Package::empty();
    package
        .insert_part(
            &PartName::new("/parts/invented.xml").expect("a valid part name"),
            "application/xml",
            format!(r#"<thing xmlns="{INVENTED_NS}"/>"#).into_bytes(),
        )
        .expect("insert the part");
    package
}

#[test]
fn a_live_authored_part_in_an_uncategorised_namespace_is_named_and_refused() {
    let package = package_with_an_uncategorised_authored_part();
    let panic = std::panic::catch_unwind(|| {
        assert_authored_parts_are_categorised("an invented namespace", &package);
    })
    .expect_err("an authored part in an uncategorised namespace must fail");
    let message = panic
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_else(|| "<non-string panic>".to_owned());
    assert!(
        message.contains(INVENTED_NS) && message.contains("/parts/invented.xml"),
        "the failure must name both the namespace and the part; it said:\n{message}"
    );
    println!("the provenance-exact category rule, proved live:\n{message}");
}

#[test]
fn a_saved_part_in_an_uncategorised_namespace_is_named_and_refused() {
    let Some(harness) = harness() else { return };
    let saved = package_with_an_uncategorised_authored_part()
        .save()
        .expect("save");
    let rows = inspect_deck(&harness, "an invented namespace", &saved, &[]);

    let row = rows
        .iter()
        .find(|row| row.name == "/parts/invented.xml")
        .expect("the invented part is in the sweep");
    assert!(
        matches!(row.outcome, PartOutcome::Uncategorised { .. }),
        "it reported: {}",
        row.outcome.describe()
    );
    assert!(row.outcome.is_failure());

    let panic = std::panic::catch_unwind(|| {
        assert_rows_are_valid("an invented namespace", &rows);
    })
    .expect_err("an uncategorised part must fail the sweep");
    let message = panic
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_else(|| "<non-string panic>".to_owned());
    assert!(
        message.contains(INVENTED_NS),
        "the failure must name the namespace; it said:\n{message}"
    );
    println!("the byte-level category rule, proved live:\n{message}");
}

#[test]
fn a_part_in_a_category_two_namespace_is_skipped_rather_than_refused() {
    // The counterpart: the rule is not "fail on anything without a schema arm". An allowlisted
    // namespace is skipped with its written reason, and the same package would otherwise be
    // indistinguishable from the one above.
    let Some(harness) = harness() else { return };
    let mut package = Package::empty();
    package
        .insert_part(
            &PartName::new("/parts/strokes.xml").expect("a valid part name"),
            "application/inkml+xml",
            br#"<inkml:ink xmlns:inkml="http://www.w3.org/2003/InkML"/>"#.to_vec(),
        )
        .expect("insert the part");
    let saved = package.save().expect("save");

    let rows = inspect_deck(&harness, "an allowlisted namespace", &saved, &[]);
    let row = rows
        .iter()
        .find(|row| row.name == "/parts/strokes.xml")
        .expect("the ink part is in the sweep");
    assert!(
        matches!(row.outcome, PartOutcome::SkippedPreservedForeign { .. }),
        "it reported: {}",
        row.outcome.describe()
    );
    assert!(!row.outcome.is_failure());
    assert_authored_parts_are_categorised("an allowlisted namespace", &package);
}
