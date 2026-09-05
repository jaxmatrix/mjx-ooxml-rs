//! What happens to a part this crate does not understand — which, for MJXOFF-91, is *every* part.
//!
//! # The tier-1 contract, in Excel's terms
//!
//! This project's overriding promise is that opening a file, editing it and writing it back does not
//! corrupt what was not touched. At the package tier that promise has an exact shape: **a part that
//! nothing dirtied re-emits its decompressed bytes verbatim**, byte for byte, and the container's
//! entry set and order are unchanged. [`mjx_opc::Package`] is what implements it — parts stay raw
//! bytes until a first mutation, and a read never counts as one — and
//! `crates/mjx-xlsx/tests/roundtrip.rs` is what proves this crate does not break it.
//!
//! The failure this guards against is not exotic. A workbook is full of parts nobody here models:
//! pivot caches, external links, revision logs, a query table's connection, a printer's saved
//! configuration. If opening a workbook parsed and re-serialized them, every one would be at the
//! mercy of a writer that had never seen most of what they carry.
//!
//! # Classification is not a gate
//!
//! [`PartKind`] names the twenty-one part kinds this crate can *identify*. A part it cannot is not
//! an error and is not rejected: [`classify`] reports it as [`PartClassification::Unclassified`],
//! and it is carried through a save untouched. That is why a `.xlsm`'s macro-enabled workbook, a
//! custom XML mapping and an embedded image all round-trip through a crate that knows nothing about
//! any of them.

use mjx_opc::{Package, PartName};

use crate::parts::PartKind;

/// What this crate could work out about one part of a workbook package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartClassification {
    /// A part kind [`PartKind`] names — reachable through the part graph, and something a later
    /// Phase D child can model.
    Classified(PartKind),
    /// A part this crate does not classify. **Not an error**: it is preserved verbatim through a
    /// save, exactly as a classified part nothing dirtied is. See this module's own documentation.
    Unclassified,
}

impl PartClassification {
    /// The part kind, or `None` for an unclassified part.
    #[must_use]
    pub fn kind(self) -> Option<PartKind> {
        match self {
            Self::Classified(kind) => Some(kind),
            Self::Unclassified => None,
        }
    }
}

/// One row of [`crate::Workbook::part_inventory`]: a part, the content type
/// `[Content_Types].xml` gives it, and what this crate made of it.
///
/// The part name is owned (it is re-derived from a ZIP entry's name and exists nowhere else to
/// borrow from); the content type is borrowed straight out of the package's content-type map, so an
/// inventory of a hundred parts copies no content-type strings at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartInventoryEntry<'a> {
    /// The part's absolute name.
    pub part: PartName,
    /// The content type resolved for it — an `Override` naming it, or a `Default` for its
    /// extension. `None` only for a package that would already fail
    /// [`mjx_opc::Package::validate`]'s content-type coverage check.
    pub content_type: Option<&'a str>,
    /// What this crate made of it.
    pub classification: PartClassification,
}

/// Classifies one part by the content type the package gives it.
#[must_use]
pub fn classify(package: &Package, part: &PartName) -> PartClassification {
    package
        .content_type_of(part)
        .and_then(PartKind::from_content_type)
        .map_or(PartClassification::Unclassified, |kind| {
            PartClassification::Classified(kind)
        })
}

/// Every addressable part of `package`, in container order, with its content type and
/// classification.
///
/// `[Content_Types].xml` itself is not a part and is not listed, for the same reason
/// [`mjx_opc::Package::part_names`] skips it.
pub(crate) fn inventory(package: &Package) -> Vec<PartInventoryEntry<'_>> {
    package
        .part_names()
        .map(|part| {
            let content_type = package.content_type_of(&part);
            let classification = content_type
                .and_then(PartKind::from_content_type)
                .map_or(PartClassification::Unclassified, |kind| {
                    PartClassification::Classified(kind)
                });
            PartInventoryEntry {
                part,
                content_type,
                classification,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parts::{CONTENT_TYPE_STYLES, CONTENT_TYPE_WORKBOOK};

    /// A content type this crate knows classifies; one it does not is preserved, not rejected.
    ///
    /// Built on [`Package::empty`] rather than a fixture so the two answers are produced by the same
    /// call on the same package — a case that only ever saw classified parts could not tell the
    /// difference between "unclassified is preserved" and "unclassified never happens".
    #[test]
    fn an_unrecognised_content_type_is_preserved_rather_than_rejected() {
        let mut package = Package::empty();
        let workbook = PartName::new("/xl/workbook.xml").expect("a valid part name");
        let stranger = PartName::new("/xl/aliens/probe.xml").expect("a valid part name");
        package
            .insert_part(&workbook, CONTENT_TYPE_WORKBOOK, b"<workbook/>".to_vec())
            .expect("insert the workbook part");
        package
            .insert_part(
                &stranger,
                "application/x-nobody-models-this",
                b"<x/>".to_vec(),
            )
            .expect("insert a part with a content type this crate does not classify");

        assert_eq!(
            classify(&package, &workbook),
            PartClassification::Classified(PartKind::Workbook)
        );
        assert_eq!(
            classify(&package, &stranger),
            PartClassification::Unclassified
        );
        assert_eq!(classify(&package, &stranger).kind(), None);

        // …and the package still saves, with the stranger's bytes intact. That is the whole claim.
        let saved = package
            .save()
            .expect("an unclassified part does not block a save");
        let reopened = Package::open(&saved).expect("reopen");
        assert_eq!(
            reopened.part_bytes(&stranger),
            Some(b"<x/>".as_slice()),
            "the part this crate could not classify came back byte for byte"
        );
    }

    /// The inventory covers every addressable part exactly once and never `[Content_Types].xml`.
    #[test]
    fn the_inventory_lists_every_addressable_part_and_not_the_content_type_stream() {
        let mut package = Package::empty();
        for (name, content_type) in [
            ("/xl/workbook.xml", CONTENT_TYPE_WORKBOOK),
            ("/xl/styles.xml", CONTENT_TYPE_STYLES),
        ] {
            package
                .insert_part(
                    &PartName::new(name).expect("a valid part name"),
                    content_type,
                    b"<x/>".to_vec(),
                )
                .expect("insert");
        }

        let rows = inventory(&package);
        let names: Vec<&str> = rows.iter().map(|row| row.part.as_str()).collect();
        assert!(names.contains(&"/xl/workbook.xml"));
        assert!(names.contains(&"/xl/styles.xml"));
        assert!(
            !names.iter().any(|name| name.contains("[Content_Types]")),
            "the content-type stream is not a part: {names:?}"
        );
        assert_eq!(
            names.len(),
            rows.len(),
            "each part appears once in the inventory"
        );
        for row in &rows {
            assert!(
                row.content_type.is_some(),
                "{} has no content type",
                row.part.as_str()
            );
            if row.part.as_str() == "/xl/styles.xml" {
                assert_eq!(
                    row.classification,
                    PartClassification::Classified(PartKind::Styles)
                );
            }
        }
    }
}
