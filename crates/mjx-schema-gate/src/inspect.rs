//! Classifying and validating every part of a package.
//!
//! Every ZIP entry is classified and reported; nothing is dropped without a printed reason, and the
//! reasons are the three categories of [`crate::categories`] rather than an open-ended list of
//! exceptions.
//!
//! # Markup compatibility is resolved, not skipped
//!
//! ECMA-376 Part 3 markup — `mc:AlternateContent`, `mc:Ignorable` and friends — lives *outside* the
//! base schemas by design, so a part carrying it cannot be validated as written. The obvious answer
//! is to skip such a part, and that is what this gate used to do; it is also why `sample.docx`'s
//! `word/document.xml` and `word/styles.xml` could never be validated, because LibreOffice writes
//! `mc:Ignorable` on both roots.
//!
//! Skipping would have re-created the hole this crate exists to close, so the gate **resolves**
//! instead: [`mjx_mce::resolve`] produces the view a conforming consumer sees — the winning
//! `mc:Choice` selected, ignorable markup in namespaces ECMA-376 does not define dropped — and the
//! gate validates *that*. No MCE logic is written here; the resolution is the existing crate's, and
//! only the re-serialization of its view is new.
//!
//! Resolution runs **only** on parts that actually carry markup compatibility. Every other part is
//! validated as the exact bytes the package holds, so the common path is never re-serialized.

use mjx_mce::{
    resolve, NamespaceScope, ResolveError, UnderstoodNamespaces, MARKUP_COMPATIBILITY_2006,
};
use mjx_ooxml_core::{Interner, RawDocument, RawElement, RawName, RawNode};
use mjx_opc::{Package, PartName, CONTENT_TYPES_ZIP_NAME};
use mjx_xml::fidelity;

use crate::categories::{categorise, NamespaceCategory, SchemaRef};
use crate::harness::{Harness, WorkDir};
use crate::tolerances::ToleratedDeviation;

/// What became of one part.
#[derive(Debug)]
pub enum PartOutcome {
    /// Validated clean against the named schema.
    Validated(&'static str),
    /// Failed only with errors covered by a [`ToleratedDeviation`].
    Tolerated {
        /// The XSD the part was validated against.
        schema: &'static str,
        /// Why the deviation is not ours to fix.
        reason: &'static str,
    },
    /// Not XML at all (an image, an OLE object, a printer-settings blob).
    SkippedBinary(String),
    /// Category 2: foreign markup this project preserves and never writes.
    SkippedPreservedForeign {
        /// The root element's namespace, or `None` when it is in no namespace at all.
        namespace: Option<String>,
        /// What the allowlist calls it.
        label: &'static str,
        /// The allowlist's written reason.
        reason: &'static str,
    },
    /// Category 3: the root element's namespace is on no list — always a failure.
    Uncategorised {
        /// The namespace nobody has answered for, or `None` for a root in no namespace.
        namespace: Option<String>,
    },
    /// Markup compatibility could not be resolved, so the part could not be reduced to base markup.
    UnresolvableMarkupCompatibility(String),
    /// Failed validation.
    Failed {
        /// The XSD the part was validated against.
        schema: &'static str,
        /// The validator's report, with the part named in place of the temporary file.
        report: String,
    },
}

impl PartOutcome {
    /// The one-line report entry, always printed, so no skip is ever silent.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Validated(schema) => format!("valid ({schema})"),
            Self::Tolerated { schema, reason } => {
                format!("tolerated deviation ({schema}) — {reason}")
            }
            Self::SkippedBinary(content_type) => {
                format!("skipped — not XML (content type {content_type})")
            }
            Self::SkippedPreservedForeign {
                namespace,
                label,
                reason,
            } => format!(
                "skipped — {label} ({}), preserved and never authored: {reason}",
                namespace.as_deref().unwrap_or("no namespace")
            ),
            Self::Uncategorised { namespace } => format!(
                "UNCATEGORISED — the root element is in {}, which is in neither MODELED_SCHEMAS \
                 nor PRESERVED_FOREIGN_MARKUP",
                namespace.as_deref().unwrap_or("no namespace")
            ),
            Self::UnresolvableMarkupCompatibility(error) => {
                format!("UNRESOLVABLE markup compatibility — {error}")
            }
            Self::Failed { schema, report } => format!("INVALID ({schema})\n{report}"),
        }
    }

    /// Whether this outcome fails the suite.
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::Failed { .. }
                | Self::Uncategorised { .. }
                | Self::UnresolvableMarkupCompatibility(_)
        )
    }

    /// The XSD this part was actually validated against, if any — what proves an arm is exercised.
    #[must_use]
    pub fn validated_against(&self) -> Option<&'static str> {
        match self {
            Self::Validated(schema) | Self::Tolerated { schema, .. } => Some(schema),
            _ => None,
        }
    }
}

/// One part's row in the sweep: its name, its root element as written, its namespace, its verdict.
#[derive(Debug)]
pub struct PartRow {
    /// The part name, prefixed by the enclosing package when it is an embedded one.
    pub name: String,
    /// The root element as written, e.g. `w:document`. `None` for a non-XML payload.
    pub root_element: Option<String>,
    /// The root element's namespace URI. `None` for a non-XML payload or a root in no namespace.
    pub namespace: Option<String>,
    /// What became of it.
    pub outcome: PartOutcome,
}

/// The content type of an embedded Office package — a chart's workbook. Its payload is a whole OPC
/// container, so the gate opens it and validates the markup *inside* rather than skipping it.
const EMBEDDED_PACKAGE_CONTENT_TYPES: [&str; 1] =
    ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"];

/// Whether a content type names an XML payload. `vmlDrawing` is XML despite the content type not
/// saying so; it is classified as preserved foreign markup a step later, which is the truthful
/// reason.
fn is_xml_content_type(content_type: &str) -> bool {
    content_type.ends_with("+xml")
        || content_type.ends_with("/xml")
        || content_type.ends_with("vmlDrawing")
}

/// Whether any element or attribute anywhere in the subtree is in the markup-compatibility
/// namespace. Attribute names carry no resolved namespace (the fidelity reader records the literal
/// prefix), so prefixes are resolved through [`NamespaceScope`] exactly as `mjx-mce` does.
fn carries_markup_compatibility(element: &RawElement, interner: &Interner) -> bool {
    fn walk(element: &RawElement, interner: &Interner, scope: &mut NamespaceScope) -> bool {
        scope.push_element(element, interner);
        let found = element
            .name
            .namespace
            .is_some_and(|ns| interner.resolve(ns) == MARKUP_COMPATIBILITY_2006)
            || element.attributes.iter().any(|attr| {
                attr.name.prefix.is_some_and(|prefix| {
                    let prefix = interner.resolve(prefix);
                    prefix != "xmlns"
                        && scope.resolve_prefix(prefix) == Some(MARKUP_COMPATIBILITY_2006)
                })
            })
            || element.children.iter().any(|child| match child {
                RawNode::Element(child) => walk(child, interner, scope),
                _ => false,
            });
        scope.pop();
        found
    }
    walk(element, interner, &mut NamespaceScope::new())
}

/// Re-serializes a document as the view a conforming consumer sees: the winning `mc:Choice`
/// selected, ignorable markup in namespaces ECMA-376 does not define dropped, every `mc:*`
/// attribute and the `xmlns:mc` binding removed.
///
/// The resolution itself is [`mjx_mce::resolve`]; this only rebuilds an owned tree from its
/// borrowed view so the existing fidelity writer can emit it. Comments and processing instructions
/// do not survive, which is correct — the result is validated, never written back into a package.
///
/// # Errors
/// Propagates an unsatisfied `mc:MustUnderstand` or a malformed `mc:AlternateContent`.
pub fn markup_compatibility_resolved(document: &RawDocument) -> Result<Vec<u8>, ResolveError> {
    let understood = UnderstoodNamespaces::from_uris(crate::categories::ecma_376_namespaces());
    let resolved = resolve(document, &understood)?;

    let mut interner = Interner::new();
    let root = rebuild(&resolved, &document.interner, &mut interner);
    let rebuilt = RawDocument::new(interner, document.bom, Vec::new(), root, Vec::new());
    Ok(fidelity::serialize_to_vec(&rebuilt))
}

/// Copies one resolved element into an owned [`RawElement`], re-interning its names.
fn rebuild(
    resolved: &mjx_mce::ResolvedElement<'_>,
    source: &Interner,
    interner: &mut Interner,
) -> RawElement {
    let attributes = resolved
        .attributes
        .iter()
        .map(|attr| mjx_ooxml_core::RawAttribute {
            name: rename(&attr.name, source, interner),
            value: attr.value.clone(),
            quote: attr.quote,
        })
        .collect();
    let children: Vec<RawNode> = resolved
        .children
        .iter()
        .map(|child| match child {
            mjx_mce::ResolvedNode::Element(child) => {
                RawNode::Element(rebuild(child, source, interner))
            }
            mjx_mce::ResolvedNode::Text(bytes) => RawNode::Text((*bytes).into()),
            mjx_mce::ResolvedNode::CData(bytes) => RawNode::CData((*bytes).into()),
        })
        .collect();
    let empty = resolved.source.empty && children.is_empty();
    RawElement::new(
        rename(resolved.name(), source, interner),
        attributes,
        children,
        empty,
    )
}

/// Re-interns a name into a fresh interner.
fn rename(name: &RawName, source: &Interner, interner: &mut Interner) -> RawName {
    RawName {
        prefix: name.prefix.map(|s| interner.intern(source.resolve(s))),
        local: interner.intern(source.resolve(name.local)),
        namespace: name.namespace.map(|s| interner.intern(source.resolve(s))),
    }
}

/// Classifies and validates every part of one package.
///
/// `tolerances` is empty for a deck this library authors: nothing it writes is ever excused.
///
/// # Panics
/// If the package cannot be opened, or a part declared XML does not parse — both are harness faults
/// rather than schema deviations.
#[must_use]
pub fn inspect_deck(
    harness: &Harness,
    label: &str,
    bytes: &[u8],
    tolerances: &[&ToleratedDeviation],
) -> Vec<PartRow> {
    let mut rows = Vec::new();
    inspect_package(harness, label, bytes, tolerances, "", &mut rows);
    rows
}

/// Classifies and validates every part of one package, appending to `rows`.
///
/// `prefix` names where the package sits: empty for the deck itself, and
/// `/ppt/embeddings/Microsoft_Excel_Sheet1.xlsx!` for a package **embedded inside** it — a chart's
/// workbook, which this library authors and whose SpreadsheetML must therefore be validated rather
/// than skipped as a binary blob.
fn inspect_package(
    harness: &Harness,
    label: &str,
    bytes: &[u8],
    tolerances: &[&ToleratedDeviation],
    prefix: &str,
    rows: &mut Vec<PartRow>,
) {
    let package = Package::open(bytes).unwrap_or_else(|e| panic!("{label}: opening package: {e}"));
    let work = WorkDir::new(&format!("{label}{prefix}").replace(['.', '/', ' ', '!'], "_"));

    // Every ZIP entry, not just the addressable parts: `[Content_Types].xml` is markup `mjx-opc`
    // writes on every save and is exactly the kind of stream a bug would break silently.
    for entry in package.entries() {
        let name = format!("{prefix}/{}", entry.name);
        let Some(payload) = entry.bytes() else {
            panic!("{label}: {name} has no materialized bytes in a freshly opened package");
        };
        // The content-types stream describes every other part and has no content type of its own.
        let content_type = PartName::from_zip_name(&entry.name)
            .ok()
            .and_then(|part| package.content_type_of(&part).map(str::to_owned));

        if let Some(content_type) = content_type {
            if EMBEDDED_PACKAGE_CONTENT_TYPES.contains(&content_type.as_str()) {
                let nested = format!("{name}!");
                inspect_package(harness, label, payload, tolerances, &nested, rows);
                continue;
            }
            if !is_xml_content_type(&content_type) {
                rows.push(PartRow {
                    name,
                    root_element: None,
                    namespace: None,
                    outcome: PartOutcome::SkippedBinary(content_type),
                });
                continue;
            }
        } else if entry.name != CONTENT_TYPES_ZIP_NAME {
            panic!("{label}: no content type for {name}");
        }

        let document = fidelity::parse(payload)
            .unwrap_or_else(|e| panic!("{label}: {name} is declared XML but does not parse: {e}"));
        let interner = &document.interner;
        let root_element = Some(qualified_name(&document.root, interner));
        let namespace = document
            .root
            .name
            .namespace
            .map(|ns| interner.resolve(ns).to_owned());

        let schema = match categorise(namespace.as_deref()) {
            NamespaceCategory::Modeled(modeled) => modeled.schema,
            NamespaceCategory::PreservedForeign(foreign) => {
                rows.push(PartRow {
                    name,
                    root_element,
                    namespace: namespace.clone(),
                    outcome: PartOutcome::SkippedPreservedForeign {
                        namespace,
                        label: foreign.label,
                        reason: foreign.reason,
                    },
                });
                continue;
            }
            NamespaceCategory::Uncategorised => {
                rows.push(PartRow {
                    name,
                    root_element,
                    namespace: namespace.clone(),
                    outcome: PartOutcome::Uncategorised { namespace },
                });
                continue;
            }
        };

        // Only a part that really carries markup compatibility is re-serialized; every other part is
        // validated as the exact bytes the package holds.
        let validated_bytes = if carries_markup_compatibility(&document.root, interner) {
            match markup_compatibility_resolved(&document) {
                Ok(bytes) => bytes,
                Err(error) => {
                    rows.push(PartRow {
                        name,
                        root_element,
                        namespace,
                        outcome: PartOutcome::UnresolvableMarkupCompatibility(error.to_string()),
                    });
                    continue;
                }
            }
        } else {
            payload.to_vec()
        };

        let file = work.path().join(
            name.trim_start_matches('/')
                .replace(['/', '[', ']', '!'], "_"),
        );
        std::fs::write(&file, &validated_bytes).expect("write part for validation");
        let outcome = validate_one(
            harness,
            schema,
            namespace.as_deref(),
            &file,
            &name,
            tolerances,
        );
        rows.push(PartRow {
            name,
            root_element,
            namespace,
            outcome,
        });
    }
}

/// Runs the validator over one already-written part and turns its report into an outcome.
fn validate_one(
    harness: &Harness,
    schema: SchemaRef,
    namespace: Option<&str>,
    file: &std::path::Path,
    part: &str,
    tolerances: &[&ToleratedDeviation],
) -> PartOutcome {
    let namespace = namespace.unwrap_or_default();
    match harness.validate(schema, namespace, file) {
        None => PartOutcome::Validated(schema.file),
        Some(report) => {
            let tolerance = tolerances.iter().find(|tolerance| {
                tolerance.part == part
                    && report.lines().all(|line| {
                        line.trim().is_empty() || line.contains(tolerance.error_contains)
                    })
            });
            match tolerance {
                Some(tolerance) => PartOutcome::Tolerated {
                    schema: schema.file,
                    reason: tolerance.reason,
                },
                None => PartOutcome::Failed {
                    schema: schema.file,
                    report: readable_report(&report, file, part),
                },
            }
        }
    }
}

/// Rewrites a validator report so each line names the part rather than the temporary file, and
/// strips the `Schemas validity error :` boilerplate.
fn readable_report(report: &str, temp_file: &std::path::Path, part: &str) -> String {
    let prefix = temp_file.display().to_string();
    report
        .lines()
        .map(|line| {
            let line = line.replace(&prefix, part);
            line.replace("Schemas validity error : ", "")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// An element's name as written, e.g. `w:document`.
fn qualified_name(element: &RawElement, interner: &Interner) -> String {
    match element.name.prefix {
        Some(prefix) => format!(
            "{}:{}",
            interner.resolve(prefix),
            interner.resolve(element.name.local)
        ),
        None => interner.resolve(element.name.local).to_owned(),
    }
}

/// Prints the per-part report and fails on any part the category rule or the validator faults.
///
/// Also fails when *nothing* was validated: a classification bug that skipped every part would
/// otherwise let invalid markup through as a silent pass.
///
/// # Panics
/// On any failing part, or when no part was validated at all.
pub fn assert_rows_are_valid(label: &str, rows: &[PartRow]) {
    let mut validated = 0usize;
    let mut failures = Vec::new();
    let mut lines = Vec::new();
    for row in rows {
        if row.outcome.validated_against().is_some() {
            validated += 1;
        }
        if row.outcome.is_failure() {
            failures.push(format!("{}: {}", row.name, row.outcome.describe()));
        }
        lines.push(format!("  {}: {}", row.name, row.outcome.describe()));
    }
    println!("schema validity — {label}\n{}", lines.join("\n"));

    assert!(
        failures.is_empty(),
        "{label}: {} part(s) do not meet the schema gate:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        validated > 0,
        "{label}: not one part was validated — every part was classified away, which would let \
         invalid markup pass unnoticed"
    );
}

/// The per-part table this gate prints for a package, one row per part.
///
/// Used by the report and by the `.docx`/`.xlsx` cases, which assert on its rows rather than on a
/// count: "some part validated" is true of a package whose every format-specific part was skipped.
#[must_use]
pub fn outcome_table(label: &str, rows: &[PartRow]) -> String {
    let mut out = format!("per-part outcomes — {label}\n");
    for row in rows {
        out.push_str(&format!(
            "  {:<48} {:<18} {:<66} {}\n",
            row.name,
            row.root_element.as_deref().unwrap_or("—"),
            row.namespace.as_deref().unwrap_or("—"),
            row.outcome.describe().lines().next().unwrap_or_default()
        ));
    }
    out
}
