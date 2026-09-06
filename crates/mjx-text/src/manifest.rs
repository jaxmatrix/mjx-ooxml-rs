//! The per-document record of which substitutions were made.
//!
//! `docs/UI_PLATFORM_PLAN.md` §10 ends on this: *"a per-document manifest recording which
//! substitutions were made — surfaced in the UI, because a user is entitled to know that what they
//! are seeing is not what the author sent."* U08's font picker renders it.
//!
//! It is populated by [`crate::FontResolver`] as a side effect of resolving, so it cannot drift from
//! what was actually drawn: there is no second pass that might be skipped, and nothing to remember
//! to call.
//!
//! Family names are interned through `mjx-ooxml-core`'s [`Interner`]. A deck with two thousand runs
//! names the same handful of families two thousand times, and a record holds the four-byte
//! [`Symbol`] rather than a `String` it would otherwise clone once per run.

use mjx_ooxml_core::{Interner, Symbol};

use crate::compatibility::MetricCompatibility;
use crate::index::ResolutionTier;

/// What happened to one family the document asked for.
#[derive(Clone, PartialEq, Debug)]
pub struct SubstitutionRecord {
    /// The family the document named.
    pub requested_family: Symbol,
    /// The family that was actually used, or `None` when nothing could serve it and the resolution
    /// was a fetch plan or a failure.
    pub resolved_family: Option<Symbol>,
    /// Which tier answered.
    pub tier: ResolutionTier,
    /// Whether the swap moves a line break — see [`MetricCompatibility`].
    pub metric_compatibility: MetricCompatibility,
    /// How many times the document asked for this family. A substitution that affects one run and
    /// one that affects the whole body are different problems, and a user interface that showed
    /// them the same way would be misleading.
    pub occurrences: u32,
}

impl SubstitutionRecord {
    /// Whether the face used differs from the face asked for.
    #[must_use]
    pub fn is_substitution(&self) -> bool {
        self.resolved_family != Some(self.requested_family)
    }
}

/// Every substitution a document needed, queryable.
///
/// Records are held in a `Vec`, in the order the document first named each family, and looked up by
/// a linear scan. That is not a compromise: a document names a handful of families however many
/// runs it has — a deck with two thousand runs still names four or five — so a scan over a `Vec`
/// touches one cache line where a map would chase a pointer, and the order it preserves is the one
/// a reader expects to see in U08's font picker.
#[derive(Debug, Default)]
pub struct SubstitutionManifest {
    records: Vec<SubstitutionRecord>,
}

impl SubstitutionManifest {
    /// An empty manifest.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one resolution. A second resolution of the same family increments its count and
    /// leaves the rest alone, because the resolver is deterministic and would reach the same face.
    pub(crate) fn record(
        &mut self,
        requested_family: Symbol,
        resolved_family: Option<Symbol>,
        tier: ResolutionTier,
        metric_compatibility: MetricCompatibility,
    ) {
        if let Some(existing) = self
            .records
            .iter_mut()
            .find(|record| record.requested_family == requested_family)
        {
            existing.occurrences = existing.occurrences.saturating_add(1);
            return;
        }
        self.records.push(SubstitutionRecord {
            requested_family,
            resolved_family,
            tier,
            metric_compatibility,
            occurrences: 1,
        });
    }

    /// Every family the document asked for, whether or not it was substituted.
    pub fn records(&self) -> impl Iterator<Item = &SubstitutionRecord> {
        self.records.iter()
    }

    /// Only the families that were actually replaced by a different one.
    pub fn substitutions(&self) -> impl Iterator<Item = &SubstitutionRecord> {
        self.records
            .iter()
            .filter(|record| record.is_substitution())
    }

    /// Only the substitutions that will move a line break — the ones worth warning about.
    pub fn metrically_divergent_substitutions(&self) -> impl Iterator<Item = &SubstitutionRecord> {
        self.records
            .iter()
            .filter(|record| record.metric_compatibility.is_divergent())
    }

    /// The record for `family`, if the document asked for it.
    #[must_use]
    pub fn record_for_family(
        &self,
        interner: &Interner,
        family: &str,
    ) -> Option<&SubstitutionRecord> {
        let symbol = interner.get(family)?;
        self.records
            .iter()
            .find(|record| record.requested_family == symbol)
    }

    /// How many distinct families the document asked for.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the document asked for no family at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// How many distinct families were replaced.
    #[must_use]
    pub fn substitution_count(&self) -> usize {
        self.substitutions().count()
    }

    /// A line per substitution, resolving the interned names, for a log or a user interface that
    /// wants text rather than structure.
    #[must_use]
    pub fn describe(&self, interner: &Interner) -> Vec<String> {
        self.substitutions()
            .map(|record| {
                let requested = interner.resolve(record.requested_family);
                match record.resolved_family {
                    Some(resolved) => format!(
                        "`{requested}` is not available; {} was drawn in `{}` ({}, {} occurrence(s))",
                        if record.metric_compatibility.preserves_line_breaks() {
                            "it"
                        } else {
                            "it — with different line breaks —"
                        },
                        interner.resolve(resolved),
                        record.tier.label(),
                        record.occurrences,
                    ),
                    None => format!(
                        "`{requested}` is not available and nothing on this device can stand in \
                         for it ({} occurrence(s))",
                        record.occurrences
                    ),
                }
            })
            .collect()
    }
}
