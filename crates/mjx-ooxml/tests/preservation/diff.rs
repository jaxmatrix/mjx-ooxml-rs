//! What one call did to a package, stated as a set of **classified** differences.
//!
//! A part's *class* is its content type, taken from the package rather than written down here — so
//! this module has no table to drift. Three names are not content types and are spelled out:
//! [`CONTENT_TYPES`] for `[Content_Types].xml`, which is not a part; [`RELATIONSHIPS`] for a `.rels`
//! stream, whose declared type is the same for every one of them; and [`NO_CONTENT_TYPE`] for an
//! entry the map does not reach, which is a defect in its own right and must never be silently
//! grouped with something else.
//!
//! # Why an embedded package is walked into
//!
//! A chart's embedded workbook is a whole `.xlsx` stored as one part. Comparing it as opaque bytes
//! answers only *did it change*, and the regression this gate exists to hold down (MJXOFF-208) is
//! **what** changed inside it: the producer's sheets, styles and document properties were dropped
//! while the part legitimately changed. So [`snapshot`] descends into an embedded package and
//! reports its parts too, classed with the [`EMBEDDED_PREFIX`] and keyed by
//! `container!inner` — the same shape `mjx_schema_gate::order` uses for the same reason.

use std::collections::BTreeMap;

use mjx_opc::{Package, PartName};

/// The class of `[Content_Types].xml`, which is a ZIP item but not a part and therefore has no
/// content type of its own.
pub(crate) const CONTENT_TYPES: &str = "«content types»";

/// The class of every `.rels` stream. They all declare
/// `application/vnd.openxmlformats-package.relationships+xml`, so the content type would not
/// distinguish them from one another anyway; naming the class makes a declaration read as what it
/// is.
pub(crate) const RELATIONSHIPS: &str = "«relationships»";

/// The class of an entry the content-type map does not reach. No declaration may name it: an
/// unclassifiable part is a packaging defect, and grouping it with anything else would hide one.
pub(crate) const NO_CONTENT_TYPE: &str = "«no content type»";

/// Prefixed onto the class of every part found *inside* an embedded package.
pub(crate) const EMBEDDED_PREFIX: &str = "embedded:";

/// A part that is itself an Office package — a chart's embedded workbook.
pub(crate) const EMBEDDED_PACKAGE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

/// Which side of the comparison a part landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Change {
    /// The part is in the saved package and was not in the original.
    Added,
    /// The part is in both and its decompressed payload differs.
    Changed,
    /// The part was in the original and is not in the saved package.
    Removed,
}

impl Change {
    /// The word a failure message uses.
    pub(crate) const fn verb(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Changed => "changed",
            Self::Removed => "removed",
        }
    }
}

/// One classified difference.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DiffEntry {
    /// What happened to it.
    pub(crate) change: Change,
    /// Its class — a content type, or one of the three spelled-out names above.
    pub(crate) class: String,
    /// The ZIP entry name, prefixed `container!inner` for a part inside an embedded package.
    pub(crate) part: String,
}

impl std::fmt::Display for DiffEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} [{}]", self.change.verb(), self.part, self.class)
    }
}

/// Every classified difference between two saved packages, sorted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PackageDiff {
    /// The differences, sorted by (change, part).
    pub(crate) entries: Vec<DiffEntry>,
}

impl PackageDiff {
    /// Whether the two packages are identical part for part.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The differences, one per line, for a failure message.
    pub(crate) fn describe(&self) -> String {
        self.entries
            .iter()
            .map(|entry| format!("      {entry}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// One flattened package: entry name → (class, decompressed payload).
type Snapshot = BTreeMap<String, (String, Vec<u8>)>;

/// Flattens a package — and every package inside it — into `out`.
///
/// An entry whose body has no materialized bytes is skipped rather than guessed at. `Package::open`
/// on a saved container materializes every entry, so in this suite that never happens; the guard is
/// here because a silent `Vec::new()` would compare as a difference and blame the wrong call.
fn flatten(bytes: &[u8], prefix: &str, depth: usize, out: &mut Snapshot) -> Result<(), String> {
    let package = Package::open(bytes).map_err(|e| format!("opening {prefix}: {e}"))?;
    for entry in package.entries() {
        let Some(payload) = entry.bytes() else {
            continue;
        };
        let class = class_of(&package, &entry.name, depth);
        let key = format!("{prefix}{}", entry.name);
        let is_embedded_package = class.ends_with(EMBEDDED_PACKAGE);
        out.insert(key.clone(), (class, payload.to_vec()));
        if is_embedded_package {
            // An embedded package that will not open is reported as its own difference by the
            // caller comparing opaque bytes; refusing here would make an unrelated call fail.
            let _ = flatten(payload, &format!("{key}!"), depth + 1, out);
        }
    }
    Ok(())
}

/// The class of one ZIP entry.
fn class_of(package: &Package, zip_name: &str, depth: usize) -> String {
    let bare = if zip_name == "[Content_Types].xml" {
        CONTENT_TYPES.to_owned()
    } else if zip_name.ends_with(".rels") {
        RELATIONSHIPS.to_owned()
    } else {
        PartName::from_zip_name(zip_name)
            .ok()
            .and_then(|part| package.content_type_of(&part).map(str::to_owned))
            .unwrap_or_else(|| NO_CONTENT_TYPE.to_owned())
    };
    if depth == 0 {
        bare
    } else {
        format!("{EMBEDDED_PREFIX}{bare}")
    }
}

/// Every classified difference between `before` and `after`.
///
/// # Errors
/// If either container does not open.
pub(crate) fn diff(before: &[u8], after: &[u8]) -> Result<PackageDiff, String> {
    let mut left = Snapshot::new();
    let mut right = Snapshot::new();
    flatten(before, "", 0, &mut left)?;
    flatten(after, "", 0, &mut right)?;

    let mut entries = Vec::new();
    for (name, (class, payload)) in &left {
        match right.get(name) {
            None => entries.push(DiffEntry {
                change: Change::Removed,
                class: class.clone(),
                part: name.clone(),
            }),
            Some((after_class, after_payload)) => {
                if payload != after_payload {
                    entries.push(DiffEntry {
                        change: Change::Changed,
                        // The class a declaration is checked against is the one the *saved* package
                        // states: a part whose content type was rewritten is a change in its own
                        // right, and blaming it on the old class would name the wrong thing.
                        class: after_class.clone(),
                        part: name.clone(),
                    });
                }
            }
        }
    }
    for (name, (class, _)) in &right {
        if !left.contains_key(name) {
            entries.push(DiffEntry {
                change: Change::Added,
                class: class.clone(),
                part: name.clone(),
            });
        }
    }
    entries.sort();
    Ok(PackageDiff { entries })
}
