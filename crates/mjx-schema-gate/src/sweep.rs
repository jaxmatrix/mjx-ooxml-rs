//! The systemic meta-gate: two assertions over the *whole* sweep rather than over one package.
//!
//! A per-package assertion cannot see the shape of the failure this project keeps repeating. A gate
//! phrased "X is covered and green" is green precisely when X is skipped, and three times now the
//! thing that would have caught it was a statement about the sweep as a whole:
//!
//! * **Every arm of the schema table is exercised.** An arm nothing reaches is an arm nobody would
//!   notice breaking — and an arm that is *never* reached is indistinguishable from one that does
//!   not exist, which is how "the schema gate covers Word" was true and empty at the same time.
//! * **Every entry of the category-2 allowlist is reached.** A dead allowlist entry is an unproven
//!   claim about markup nothing in the corpus contains.
//!
//! The other half of the pinned-skip rule — *a namespace on no list fails, naming it* — is enforced
//! per part, at the moment it is met, by [`PartOutcome::Uncategorised`]. Stating it there rather
//! than only here means the failure names the part as well as the namespace, and means it fires in
//! the suite that reads the part rather than only in the whole-workspace sweep.
//!
//! [`PartOutcome::Uncategorised`]: crate::PartOutcome

use std::collections::BTreeSet;

use mjx_opc::Package;
use mjx_xml::fidelity;

use crate::categories::{
    categorise, ForeignMarkupKey, NamespaceCategory, MODELED_SCHEMAS, PRESERVED_FOREIGN_MARKUP,
};
use crate::inspect::{PartOutcome, PartRow};

/// What the sweep saw, accumulated across every package it inspected.
#[derive(Debug, Default)]
pub struct Sweep {
    /// Where each observation came from, for the failure messages.
    sources: Vec<String>,
    /// The XSD file names actually validated against.
    schemas_exercised: BTreeSet<&'static str>,
    /// The category-2 entries actually reached, by label.
    foreign_reached: BTreeSet<&'static str>,
    /// Every namespace reported as preserved-foreign, printed with the pinned list.
    namespaces_skipped: BTreeSet<String>,
}

impl Sweep {
    /// An empty sweep.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Folds one package's rows in.
    pub fn record(&mut self, label: &str, rows: &[PartRow]) {
        self.sources.push(label.to_owned());
        for row in rows {
            if let Some(schema) = row.outcome.validated_against() {
                self.schemas_exercised.insert(schema);
            }
            if let PartOutcome::SkippedPreservedForeign {
                namespace, label, ..
            } = &row.outcome
            {
                self.foreign_reached.insert(label);
                self.namespaces_skipped.insert(
                    namespace
                        .clone()
                        .unwrap_or_else(|| "«no namespace»".to_owned()),
                );
            }
        }
    }

    /// The namespaces this sweep reported as preserved-foreign, for the report.
    #[must_use]
    pub fn namespaces_skipped(&self) -> Vec<&str> {
        self.namespaces_skipped.iter().map(String::as_str).collect()
    }

    /// Every arm of the schema table must have been exercised by at least one part.
    ///
    /// # Panics
    /// Naming every arm nothing reached.
    pub fn assert_every_modeled_schema_was_exercised(&self) {
        let unexercised: Vec<String> = MODELED_SCHEMAS
            .iter()
            .filter(|schema| !self.schemas_exercised.contains(schema.schema.file))
            .map(|schema| format!("{} ({})", schema.label, schema.schema.file))
            .collect();
        assert!(
            unexercised.is_empty(),
            "these schema arms were reached by nothing in the sweep: {}.\nThe sweep covered {} \
             package(s): {:?}.\nAn arm nothing exercises is an arm nobody would notice breaking — \
             either a fixture or an authoring case must reach it, or the arm does not belong in \
             MODELED_SCHEMAS",
            unexercised.join(", "),
            self.sources.len(),
            self.sources
        );
    }

    /// Every entry of the category-2 allowlist must have been reached.
    ///
    /// The converse — a namespace reached that is on no list — is a hard failure at the part, so a
    /// new namespace can never join this set silently.
    ///
    /// # Panics
    /// Naming every allowlist entry nothing reached.
    pub fn assert_pinned_skips(&self) {
        println!(
            "namespaces skipped as preserved foreign markup across {} package(s): {:?}",
            self.sources.len(),
            self.namespaces_skipped
        );
        let dead: Vec<&str> = PRESERVED_FOREIGN_MARKUP
            .iter()
            .filter(|entry| !self.foreign_reached.contains(entry.label))
            .map(|entry| entry.label)
            .collect();
        assert!(
            dead.is_empty(),
            "these entries of PRESERVED_FOREIGN_MARKUP were reached by nothing in the sweep: \
             {dead:?}.\nAn allowlist entry nothing reaches is an unproven claim: either a fixture \
             must carry that markup, or the entry must go — otherwise the list stops describing \
             what the corpus contains and starts excusing what it might"
        );
    }
}

/// The provenance-exact form of the category rule, for a caller that still holds a live package.
///
/// [`Package::authored_xml_parts`] reports the parts whose bytes **this library produced** — they
/// were inserted, replaced, built by `Package::empty`, or edited through the tree. Provenance does
/// not survive a save (that is what fidelity means: re-opened bytes are all `FromContainer`), so the
/// byte-level sweep applies the same rule to every part instead, which is strictly stricter. This
/// function is the exact statement, and it is what a format crate holding its own `Package` should
/// call.
///
/// # Panics
/// Naming every authored part whose root namespace is in neither category.
pub fn assert_authored_parts_are_categorised(label: &str, package: &Package) {
    let mut uncategorised = Vec::new();
    for (part, entry) in package.authored_xml_parts() {
        let Some(bytes) = entry.bytes() else {
            // An edited part has no materialized bytes; its tree is the source of truth and the
            // save path is what the byte-level sweep then inspects.
            continue;
        };
        let Ok(document) = fidelity::parse(bytes) else {
            continue;
        };
        let namespace = document
            .root
            .name
            .namespace
            .map(|symbol| document.interner.resolve(symbol).to_owned());
        if let NamespaceCategory::Uncategorised = categorise(namespace.as_deref()) {
            uncategorised.push(format!(
                "{} (root {} in {})",
                part.as_str(),
                document.interner.resolve(document.root.name.local),
                namespace.as_deref().unwrap_or("no namespace")
            ));
        }
    }
    assert!(
        uncategorised.is_empty(),
        "{label}: these authored parts are in a namespace that belongs to no category: {}.\nAn \
         authored part must be markup we model (add it to MODELED_SCHEMAS with its XSD) or foreign \
         markup we only preserve (add it to PRESERVED_FOREIGN_MARKUP with the reason). There is no \
         third answer for markup this library writes",
        uncategorised.join(", ")
    );
}

/// The allowlist keys, for a report that wants to print the rule rather than a run's output.
#[must_use]
pub fn preserved_foreign_keys() -> Vec<String> {
    PRESERVED_FOREIGN_MARKUP
        .iter()
        .map(|entry| match entry.key {
            ForeignMarkupKey::Namespace(namespace) => namespace.to_owned(),
            ForeignMarkupKey::NoNamespace => "«no namespace»".to_owned(),
        })
        .collect()
}
