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
