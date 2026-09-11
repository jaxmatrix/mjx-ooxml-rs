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
//!
//! ## …and a slot resolution empties goes with the content it held (MJXOFF-196)
//!
//! Resolution removes an ignorable element **together with its content**, which is what ECMA-376
//! Part 3 says and what [`mjx_mce::resolve`] correctly does. Composed with the base schemas it
//! leaves a hole: `sml.xsd`'s and `dml-chart.xsd`'s `CT_Extension` declare their whole content model
//! as a bare `<xsd:any processContents="lax"/>`, whose `minOccurs` defaults to **1**, so an `<ext>`
//! whose only child was ignorable is rejected — *Missing child element(s)* — on every conformant
//! file Office has written since 2010. So the gate drops such an element as well: an emptied
//! [wildcard slot](crate::wildcard_slots) existed only to carry the extension that was ignored, and
//! ignoring the extension without ignoring the slot is half a resolution.
//!
//! **Exactly one view is validated, and it is always this one.** There is no second attempt and no
//! fall-back: a gate that reports only what fails in every view it tries is a gate that goes quiet,
//! which is MJXOFF-88 §7's shape. Three properties keep the rule from quieting anything real:
//!
//! * It fires only on the elements [`crate::wildcard_slots::WILDCARD_SLOTS`] names, and that table is
//!   *derived from the pinned XSDs by test* rather than written by hand.
//! * Such an element's content model is wildcards and nothing else, so dropping it can never hide a
//!   missing **named** child — the shape a defect of ours would take.
//! * It fires only when the source element **had** element children. An `<ext/>` we authored empty
//!   is still a failure, and `an_extension_slot_we_author_empty_is_still_a_failure` holds that.
//!
//! What it gives up is stated in [`crate::wildcard_slots`] and in `docs/validation/06-the-office-pass.md`:
//! the gate no longer says anything about markup *inside* an ignorable extension. It never did.
//! `CT_Extension`'s wildcard is `processContents="lax"` and no schema for such a namespace is
//! loaded, so keeping the content would have had the validator accept it unread — the ticket's
//! option 1 measured exactly that and called it "validates".

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
    /// A ZIP **directory entry** — a container name ending in `/`, which is not a part at all.
    ///
    /// `zip -r` without `-D` writes one for every folder it walks, which is a legal archive real
    /// producers emit; OPC simply has no content type for a directory. This is a *skip*, never a
    /// failure, and it carries its own row so the skip is not silent (MJXOFF-284).
    SkippedDirectoryEntry,
    /// A **file** entry that no content type covers — a genuine package defect, always a failure.
    ///
    /// ECMA-376 Part 2 §10.1.2 gives every part exactly one content type, so an entry that is not a
    /// directory and that neither an `<Override>` names nor a `<Default>` extension rule covers is
    /// markup nothing can be resolved for. This is the half a fix that skipped every untyped entry
    /// would have lost, which is why it is a separate variant rather than a second spelling of
    /// [`SkippedDirectoryEntry`](Self::SkippedDirectoryEntry).
    WithoutContentType {
        /// Why nothing types it: either the entry name is not a valid OPC part name, or it is and
        /// the content-types stream still says nothing about it.
        reason: String,
    },
    /// A part whose content type declares XML and whose bytes do not parse as XML.
    ///
    /// A deck this library authors never produces one — but `validation-artefacts --ingest` is
    /// pointed at files this library did not write, and refusing one of those with a bare panic is
    /// the failure shape MJXOFF-284 closed.
    NotWellFormedXml(String),
    /// The package — the outer one, or one embedded in it — could not be opened at all.
    ///
    /// The outer case is unreachable through `--ingest`, which opens the file before it reports on
    /// it; an **embedded** workbook whose bytes are not a container is not, and
    /// [`crate::audit_order_report`] already reports that rather than raising it.
    PackageWouldNotOpen(String),
    /// Category 1b: every child of a [wrapper root](crate::categories::WrapperRoot) validated clean
    /// against the named schema. The wrapper itself is asserted nothing about, which is why the
    /// count is carried: a wrapper the splitter found no children in is
    /// [`WrapperHeldNothing`](Self::WrapperHeldNothing), not a pass.
    ValidatedPerChild {
        /// The XSD each child was validated against.
        schema: &'static str,
        /// The root element as written, e.g. `xml`.
        root: String,
        /// How many element children were validated.
        children: usize,
    },
    /// Category 1b, and the splitter matched nothing: a wrapper with no element children was
    /// handed to no validator at all, which would otherwise be reported as a clean part.
    WrapperHeldNothing {
        /// The root element as written.
        root: String,
    },
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
            Self::ValidatedPerChild {
                schema,
                root,
                children,
            } => format!(
                "valid ({schema}) — {children} child element(s) of <{root}> validated separately; \
                 no schema declares the wrapper itself"
            ),
            Self::WrapperHeldNothing { root } => format!(
                "WRAPPER HELD NOTHING — <{root}> has no element child, so the per-child validator \
                 was handed nothing to check"
            ),
            Self::Tolerated { schema, reason } => {
                format!("tolerated deviation ({schema}) — {reason}")
            }
            Self::SkippedBinary(content_type) => {
                format!("skipped — not XML (content type {content_type})")
            }
            Self::SkippedDirectoryEntry => {
                "skipped — a ZIP directory entry, which is not a part and has no content type"
                    .to_owned()
            }
            Self::WithoutContentType { reason } => format!(
                "NO CONTENT TYPE — this entry is a file, not a directory, and nothing types it: \
                 {reason}"
            ),
            Self::NotWellFormedXml(error) => {
                format!("NOT WELL-FORMED — declared XML by its content type, but: {error}")
            }
            Self::PackageWouldNotOpen(error) => {
                format!("WOULD NOT OPEN — the package could not be read at all: {error}")
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
                | Self::WrapperHeldNothing { .. }
                // A directory entry is deliberately **not** here: it is a legal container feature,
                // not a defect. Everything else this list gained with MJXOFF-284 is one.
                | Self::WithoutContentType { .. }
                | Self::NotWellFormedXml(_)
                | Self::PackageWouldNotOpen(_)
        )
    }

    /// The XSD this part was actually validated against, if any — what proves an arm is exercised.
    #[must_use]
    pub fn validated_against(&self) -> Option<&'static str> {
        match self {
            Self::Validated(schema)
            | Self::ValidatedPerChild { schema, .. }
            | Self::Tolerated { schema, .. } => Some(schema),
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
///
/// **The comparison folds case, and that is load-bearing rather than tidy.** ECMA-376 Part 2
/// §10.1.2.3 compares a content type case-insensitively, and this predicate decides whether the gate
/// *looks at a part at all* — a spelling it does not recognise is reported as a non-XML payload and
/// skipped, which is MJXOFF-88 §7's shape exactly: the gate goes green because the part was never
/// inspected. MJXOFF-114 had already paid for it once, in `mjx-opc`'s own exception list; MJXOFF-221
/// found the same exact match here and in [`crate::order`], where it was a second copy of this
/// function rather than a call to it.
pub(crate) fn is_xml_content_type(content_type: &str) -> bool {
    let base = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase();
    base.ends_with("+xml") || base.ends_with("/xml") || base.ends_with("vmldrawing")
}

/// Whether any element or attribute anywhere in the subtree is in the markup-compatibility
/// namespace. Attribute names carry no resolved namespace (the fidelity reader records the literal
/// prefix), so prefixes are resolved through [`NamespaceScope`] exactly as `mjx-mce` does.
pub(crate) fn carries_markup_compatibility(element: &RawElement, interner: &Interner) -> bool {
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
    Ok(fidelity::serialize_to_vec(
        &markup_compatibility_resolved_tree(document)?,
    ))
}

/// The same view as a **tree**, for the half of the gate that walks it instead of handing it to
/// `xmllint`.
///
/// [`crate::order`] audits child order over exactly this, which is the whole of MJXOFF-272: the two
/// arms used to disagree about which markup they were looking at, so a part whose content sat inside
/// an `mc:AlternateContent` was validated in its resolved form and *ordered* in its raw one — where
/// the tables name no `mc:AlternateContent` slot, so the walk stepped over the subtree without
/// entering it. One function produces the view and both arms call it; a second resolution here
/// would be a second thing to keep in step.
///
/// # Errors
/// Propagates an unsatisfied `mc:MustUnderstand` or a malformed `mc:AlternateContent`.
pub(crate) fn markup_compatibility_resolved_tree(
    document: &RawDocument,
) -> Result<RawDocument, ResolveError> {
    let understood = UnderstoodNamespaces::from_uris(crate::categories::ecma_376_namespaces());
    let resolved = resolve(document, &understood)?;

    let mut interner = Interner::new();
    let root = rebuild(&resolved, &document.interner, &mut interner);
    Ok(RawDocument::new(
        interner,
        document.bom,
        Vec::new(),
        root,
        Vec::new(),
    ))
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
        .filter(|child| match child {
            mjx_mce::ResolvedNode::Element(child) => {
                !resolution_emptied_a_wildcard_slot(child, source)
            }
            _ => true,
        })
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

/// Whether markup-compatibility resolution emptied a [wildcard slot](crate::wildcard_slots) — an
/// element the schema declares as holding one foreign child and nothing else.
///
/// All three conditions are load-bearing; the module documentation says why each one is there.
/// A slot whose source was **already** childless answers `false`, so an `<ext/>` this library
/// authored empty is still handed to the validator and still fails.
///
/// Applied by the parent, so a part *root* is never dropped. No slot in
/// [`crate::wildcard_slots::WILDCARD_SLOTS`] is a part root — `x:ext`, `c:ext`, `x:Schema`,
/// `x:DataBinding` and `o:equationxml` are all nested — and a part with no root element is not a
/// part.
fn resolution_emptied_a_wildcard_slot(
    resolved: &mjx_mce::ResolvedElement<'_>,
    interner: &Interner,
) -> bool {
    let name = resolved.name();
    let namespace = name.namespace.map(|ns| interner.resolve(ns));
    if !crate::wildcard_slots::is_wildcard_slot(namespace, interner.resolve(name.local)) {
        return false;
    }
    let empty_now = !resolved
        .children
        .iter()
        .any(|child| matches!(child, mjx_mce::ResolvedNode::Element(_)));
    let held_something = resolved
        .source
        .children
        .iter()
        .any(|child| matches!(child, RawNode::Element(_)));
    empty_now && held_something
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
/// # This reports; it does not raise
///
/// Every refusal is a [`PartRow`], including the ones that used to be a `panic!`: a package that
/// will not open, an entry nothing types, a part declared XML whose bytes are not. `inspect_deck` is
/// reached from `validation-artefacts --ingest`, which a person points at an arbitrary file they
/// have just saved out of Office, and a bare stack trace there costs the reviewer the report the
/// pass exists to produce (MJXOFF-284). A test suite that wants those rows to *fail* calls
/// [`assert_rows_are_valid`], which they do — see [`PartOutcome::is_failure`].
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
    let package = match Package::open(bytes) {
        Ok(package) => package,
        Err(error) => {
            // The embedded case is the live one: a chart workbook whose bytes are not a container
            // arrives here from a file this library did not write. `audit_order_report` reports the
            // same fact the same way, and the two halves of the gate must not disagree about it.
            rows.push(PartRow {
                name: if prefix.is_empty() {
                    label.to_owned()
                } else {
                    prefix.to_owned()
                },
                root_element: None,
                namespace: None,
                outcome: PartOutcome::PackageWouldNotOpen(error.to_string()),
            });
            return;
        }
    };
    let work = WorkDir::new(&format!("{label}{prefix}").replace(['.', '/', ' ', '!'], "_"));

    // Every ZIP entry, not just the addressable parts: `[Content_Types].xml` is markup `mjx-opc`
    // writes on every save and is exactly the kind of stream a bug would break silently.
    for entry in package.entries() {
        let name = format!("{prefix}/{}", entry.name);

        // A **directory entry** — the container's marker for a folder, written by `zip -r` without
        // `-D` and by real producers. It is not a part, so OPC has no content type for it and none
        // is missing. The discriminator is the trailing `/`, and it is not this crate's invention:
        // `mjx_opc::PartName::new` refuses such a name outright ("part name must not end with
        // '/'"), which is exactly why `Package::part_names`, `Package::validate`'s content-type
        // check and `authored_xml_parts` all pass it over. Testing the name rather than the payload
        // matters: a directory entry is empty, but so is a zero-byte *part*, and only one of the
        // two is excused.
        if entry.name.ends_with('/') {
            rows.push(PartRow {
                name,
                root_element: None,
                namespace: None,
                outcome: PartOutcome::SkippedDirectoryEntry,
            });
            continue;
        }

        // Unreachable rather than tolerated: `bytes` was opened above, so every body is `Raw` and
        // only an `Edited` one — which needs a mutation this function never performs — yields
        // `None`. It stays an assertion because a `None` here would mean the copy-on-write
        // invariant had broken, which is a fault in this harness and not a verdict about the file.
        let Some(payload) = entry.bytes() else {
            panic!("{label}: {name} has no materialized bytes in a freshly opened package");
        };

        // The content-types stream describes every other part and has no content type of its own.
        // Everything else that reaches this point is a file, so "nothing types it" is a defect —
        // whether because the name cannot be addressed as a part at all, or because it can and the
        // stream is silent about it. Those two are distinguished in the reason, never in the
        // verdict.
        let content_type = match PartName::from_zip_name(&entry.name) {
            Ok(part) => package.content_type_of(&part).map(str::to_owned),
            Err(error) => {
                rows.push(PartRow {
                    name,
                    root_element: None,
                    namespace: None,
                    outcome: PartOutcome::WithoutContentType {
                        reason: format!("its name is not a valid OPC part name — {error}"),
                    },
                });
                continue;
            }
        };

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
            rows.push(PartRow {
                name,
                root_element: None,
                namespace: None,
                outcome: PartOutcome::WithoutContentType {
                    // The *rule* is not restated: `content_type_of` above is mjx-opc's own answer,
                    // the same one `Package::validate` reads to raise `PartWithoutContentType`.
                    // Only the sentence explaining it is written twice, and it is written to match.
                    reason: "no <Override> names it and no <Default> covers its extension"
                        .to_owned(),
                },
            });
            continue;
        }

        let document = match fidelity::parse(payload) {
            Ok(document) => document,
            Err(error) => {
                rows.push(PartRow {
                    name,
                    root_element: None,
                    namespace: None,
                    outcome: PartOutcome::NotWellFormedXml(error.to_string()),
                });
                continue;
            }
        };
        let interner = &document.interner;
        let root_element = Some(qualified_name(&document.root, interner));
        let namespace = document
            .root
            .name
            .namespace
            .map(|ns| interner.resolve(ns).to_owned());

        let root_local = interner.resolve(document.root.name.local).to_owned();
        let category = categorise(namespace.as_deref(), &root_local);
        let schema = match category {
            NamespaceCategory::Modeled(modeled) => modeled.schema,
            // Category 1b is validated after markup-compatibility resolution, exactly as category 1
            // is, so the schema is carried here and the split happens below.
            NamespaceCategory::Wrapper(wrapper) => wrapper.schema,
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
        let outcome = if let NamespaceCategory::Wrapper(wrapper) = category {
            validate_each_child(harness, wrapper, &validated_bytes, &file, &name)
        } else {
            std::fs::write(&file, &validated_bytes).expect("write part for validation");
            validate_one(
                harness,
                schema,
                namespace.as_deref(),
                &file,
                &name,
                tolerances,
            )
        };
        rows.push(PartRow {
            name,
            root_element,
            namespace,
            outcome,
        });
    }
}

/// Validates every element child of a [wrapper root](crate::categories::WrapperRoot) separately.
///
/// A `.vml` part is `<xml>…</xml>` in no namespace, and no VML schema declares a global element for
/// it — `xmllint` pointed at the document reports *No matching global declaration available for the
/// validation root* and stops there, which is the whole of what is left of the reason such a part
/// used to be skipped for (MJXOFF-245). Its children are a different matter: `v:shape`,
/// `v:shapetype`, `o:shapelayout`, `o:lock` and the rest are global elements of the VML family, so
/// each one validates on its own against a driver over `vml-main.xsd`.
///
/// Each child is re-serialized as a standalone document carrying the namespace declarations the
/// wrapper held, because a `v:` prefix means nothing once its `xmlns:v` is left behind on a root
/// that is not being written. Declarations the child makes for itself win — a producer may rebind a
/// prefix on the child, and the file's own binding is the one to keep.
///
/// `bytes` is the same markup-compatibility-resolved view category 1 validates, re-parsed rather
/// than threaded through: one code path produces the view, and a `.vml` part is small.
///
/// # Panics
/// If the resolved view does not parse, or a child cannot be written for validation — both harness
/// faults rather than schema deviations.
fn validate_each_child(
    harness: &Harness,
    wrapper: &'static crate::categories::WrapperRoot,
    bytes: &[u8],
    file: &std::path::Path,
    part: &str,
) -> PartOutcome {
    let document = fidelity::parse(bytes).unwrap_or_else(|e| {
        panic!("{part}: the resolved view of a wrapper part does not parse: {e}")
    });
    let interner = &document.interner;
    let root = qualified_name(&document.root, interner);

    let mut validated = 0usize;
    let mut reports = Vec::new();
    for child in document.root.children.iter().filter_map(|node| match node {
        RawNode::Element(child) => Some(child),
        _ => None,
    }) {
        let child_file = file.with_extension(format!("child{validated}.xml"));
        std::fs::write(
            &child_file,
            wrapper_child_document(&document.root, child, interner),
        )
        .expect("write a wrapper's child for validation");
        let named = format!("{part} → <{}>", qualified_name(child, interner));
        if let Some(report) = harness.validate(wrapper.schema, wrapper.namespace, &child_file) {
            reports.push(readable_report(&report, &child_file, &named));
        }
        validated += 1;
    }

    if validated == 0 {
        return PartOutcome::WrapperHeldNothing { root };
    }
    if reports.is_empty() {
        return PartOutcome::ValidatedPerChild {
            schema: wrapper.schema.file,
            root,
            children: validated,
        };
    }
    PartOutcome::Failed {
        schema: wrapper.schema.file,
        report: reports.join("\n"),
    }
}

/// One child of a wrapper root, serialized as a standalone document under the wrapper's own
/// namespace declarations.
///
/// No source range is passed to the writer: the element is being written with attributes it did not
/// have, so a verbatim byte range would emit the original and silently drop the declarations that
/// make the child readable at all.
fn wrapper_child_document(
    wrapper: &RawElement,
    child: &RawElement,
    interner: &Interner,
) -> Vec<u8> {
    // Names are compared by symbol rather than by string: both elements came out of the same
    // document and therefore the same interner, so two equal symbols are the same name.
    let is_namespace_declaration = |name: &RawName| match name.prefix {
        Some(prefix) => interner.resolve(prefix) == "xmlns",
        None => interner.resolve(name.local) == "xmlns",
    };
    let child_declares = |name: &RawName| child.attributes.iter().any(|attr| attr.name == *name);

    let mut attributes: Vec<mjx_ooxml_core::RawAttribute> = wrapper
        .attributes
        .iter()
        .filter(|attr| is_namespace_declaration(&attr.name) && !child_declares(&attr.name))
        .cloned()
        .collect();
    attributes.extend(child.attributes.iter().cloned());

    // `RawElement::new` rather than a clone: the child's own source range describes bytes that do
    // not carry the declarations just added to it.
    let standalone = RawElement::new(child.name, attributes, child.children.clone(), child.empty);
    let mut out = Vec::new();
    fidelity::serialize_element(&standalone, interner, None, &mut out);
    out
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

#[cfg(test)]
mod tests {
    use super::is_xml_content_type;

    /// The gate looks at a part only when this says the part is XML, so a spelling it misses is a
    /// part nobody validates — the false green MJXOFF-110 exists to close, arriving through a string
    /// literal rather than through a missing table row.
    ///
    /// Every spelling below is the same media type under ECMA-376 Part 2 §10.1.2.3. Before
    /// MJXOFF-221 the test was `content_type.ends_with("vmlDrawing")` and the last three answered
    /// `false`.
    #[test]
    fn a_vml_drawing_is_xml_in_every_casing_a_producer_writes() {
        for spelling in [
            "application/vnd.openxmlformats-officedocument.vmlDrawing",
            "application/vnd.openxmlformats-officedocument.vmldrawing",
            "APPLICATION/VND.OPENXMLFORMATS-OFFICEDOCUMENT.VMLDRAWING",
            "application/vnd.openxmlformats-officedocument.vmlDrawing; charset=utf-8",
        ] {
            assert!(is_xml_content_type(spelling), "{spelling} names XML");
        }
    }

    /// The `+xml` suffix and the two generic types fold too, and a binary payload still does not
    /// match.
    #[test]
    fn the_suffix_rule_folds_and_a_binary_payload_still_does_not_match() {
        for spelling in [
            "application/vnd.openxmlformats-officedocument.presentationml.slide+xml",
            "APPLICATION/VND.OPENXMLFORMATS-OFFICEDOCUMENT.THEME+XML",
            "text/xml",
            "TEXT/XML",
            "application/xml; charset=utf-8",
        ] {
            assert!(is_xml_content_type(spelling), "{spelling} names XML");
        }
        for spelling in [
            "image/png",
            "application/vnd.openxmlformats-officedocument.oleObject",
            "",
        ] {
            assert!(!is_xml_content_type(spelling), "{spelling} is not XML");
        }
    }
}
