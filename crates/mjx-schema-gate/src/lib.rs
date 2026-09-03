//! The ECMA-376 schema and child-order gate every format crate is held to.
//!
//! **Test-only.** Nothing shipped depends on this crate; it is a `dev-dependency` of `mjx-pptx`,
//! `mjx-docx` and `mjx-xlsx` and of nothing else. It exists because an integration test compiles
//! only into its own crate, so the harness that used to live in `mjx-pptx/tests/schema_validity.rs`
//! was unreachable from the two crates Phases C and D are about to fill.
//!
//! # What it asserts
//!
//! Two independent halves, and only the first needs an external tool:
//!
//! * **Child order** ([`assert_deck_is_in_schema_order`]) — no element carries a child out of its
//!   complex type's `xsd:sequence`, walked from the generated `mjx-ooxml-types::child_order` tables.
//!   Runs on every machine, with or without `References/`.
//! * **Schema validity** ([`assert_fixture_is_schema_valid`],
//!   [`assert_authored_deck_is_schema_valid`]) — `xmllint --noout --schema` against the ECMA-376
//!   Part 4 **Transitional** and Part 2 **OPC** schemas. `xmllint` is a C tool, which is fine: only
//!   *shipped* crates are pure Rust; C tooling is sanctioned for CI and tests.
//!
//! # The line it draws
//!
//! [`categories`] holds the three-category rule and is the only place the line is drawn. Markup we
//! model is validated; markup we merely preserve is skipped with a written reason; a namespace on
//! neither list is a **hard failure naming it**. There is no "skip anything we have no arm for"
//! fallback, because that fallback is the silent skip this crate exists to close.
//!
//! # Skipping
//!
//! `References/` is git-ignored, so the schema half skips — printing a notice and passing — when the
//! trees or `xmllint` are absent. `MJX_REQUIRE_SCHEMA=1` turns any absence into a hard failure, and
//! CI sets it. See the [`harness`](mod@self::harness) module.

pub mod categories;
pub mod harness;
pub mod inspect;
pub mod order;
pub mod sweep;
pub mod tolerances;

pub use categories::{
    categorise, child_order_tables_cover, ecma_376_namespaces, schema_for_namespace,
    ForeignMarkupKey, ModeledSchema, NamespaceCategory, OrderingCoverage, PreservedForeignMarkup,
    SchemaRef, SchemaSet, MODELED_SCHEMAS, PRESERVED_FOREIGN_MARKUP,
};
pub use harness::{harness, Harness, WorkDir, XML_NAMESPACE_SCHEMA};
pub use inspect::{
    assert_rows_are_valid, inspect_deck, markup_compatibility_resolved, outcome_table, PartOutcome,
    PartRow,
};
/// The committed fixture corpus, re-exported so a suite needs one dependency rather than two.
pub use mjx_fixtures as fixtures;
pub use mjx_fixtures::{
    assert_every_fixture_has_a_known_kind, fixture, fixtures_dir, package_fixtures,
    package_fixtures_with_extension,
};
pub use order::{
    assert_deck_is_in_schema_order, audit_deck_order, parts_that_must_be_audited, AuditedPart,
    MINIMUM_ELEMENTS_VISITED,
};
pub use sweep::{assert_authored_parts_are_categorised, Sweep};
pub use tolerances::{tolerances_for, ToleratedDeviation, TOLERATED_DEVIATIONS};

/// Validates a committed fixture, allowing only the deviations [`TOLERATED_DEVIATIONS`] records for
/// it, and returns its per-part rows so a caller can assert on them.
///
/// Returns an empty vector when the harness is unavailable and the gate skipped.
#[must_use]
pub fn inspect_fixture(name: &str) -> Vec<PartRow> {
    let Some(harness) = harness() else {
        return Vec::new();
    };
    let tolerances = tolerances_for(name);
    inspect_deck(&harness, name, &fixture(name), &tolerances)
}

/// Validates a committed fixture's markup, when the schemas are present.
///
/// The child-order audit is deliberately **not** run here. `child_order`'s own documentation draws
/// that line: the audit "is for verifying *authored* markup — running it over a document a caller
/// supplied would only report what that caller's application wrote, which is not ours to fault".
/// A fixture reaches the ordering gate the moment this library opens, edits and saves it, which is
/// what [`assert_authored_deck_is_schema_valid`] covers.
///
/// # Panics
/// On an untolerated schema deviation, or a part in an uncategorised namespace.
pub fn assert_fixture_is_schema_valid(name: &str) {
    let rows = inspect_fixture(name);
    if rows.is_empty() {
        return;
    }
    assert_rows_are_valid(name, &rows);
}

/// Validates a deck this library **authored**. No deviation is tolerated: everything in a deck we
/// write is ours.
///
/// Two gates run here, and only the second needs `References/`:
///
/// 1. [`assert_deck_is_in_schema_order`] — child order, from the committed tables. It runs
///    **always**, so the ordering guarantee does not evaporate on a machine with no schemas.
/// 2. `xmllint` against the XSDs, when the harness is available.
///
/// # Panics
/// On an ordering defect, any schema deviation, or a part in an uncategorised namespace.
pub fn assert_authored_deck_is_schema_valid(label: &str, bytes: &[u8]) {
    assert_deck_is_in_schema_order(label, bytes);
    let Some(harness) = harness() else { return };
    let rows = inspect_deck(&harness, label, bytes, &[]);
    assert_rows_are_valid(label, &rows);
}
