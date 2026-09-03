//! The three-category rule: what happens to a part, decided by the namespace of its root element.
//!
//! Every XML part of every package this gate inspects falls into exactly one category, and the
//! category decides the verdict. There is deliberately no fourth branch and no "skip anything we
//! have no arm for" fallback — that fallback is the silent skip this gate exists to close.
//!
//! 1. **[Markup we model](ModeledSchema).** PresentationML, DrawingML main / chart / diagram,
//!    SpreadsheetML, WordprocessingML, and the two OPC control streams `mjx-opc` rewrites on every
//!    save. A part rooted here is **validated** against the XSD named, and — once the schema has a
//!    generated child-order table — every element of it is audited for `xsd:sequence` order.
//! 2. **[Foreign markup we preserve](PreservedForeignMarkup).** VML, InkML, ActiveX and the two
//!    `docProps` streams. A named list, one written reason per entry. A part rooted here is
//!    **skipped**, and the skip prints that reason.
//! 3. **Anything else.** A **hard failure** naming the namespace and the part.
//!
//! Category 3 is the whole point. A namespace nobody has thought about cannot arrive quietly: it
//! arrives as a red test that names it, and the only way to make the suite green is to put it in
//! category 1 (with a schema arm) or category 2 (with a reason). That is what "the allowlist is the
//! thing a new namespace must be added to" means in mechanical terms.
//!
//! # Why this is stated over namespaces rather than over [`mjx_opc::PartProvenance`]
//!
//! [`PartProvenance::Authored`](mjx_opc::PartProvenance::Authored) is a property of a *live*
//! package: it says the bytes were inserted, replaced, built by `Package::empty`, or edited through
//! the tree. It does not survive a save — that is exactly what fidelity means, and re-opening saved
//! bytes reports every part as [`FromContainer`](mjx_opc::PartProvenance::FromContainer). The gate
//! inspects saved bytes, so it cannot read provenance there.
//!
//! What it does instead is **stricter**, not weaker: the category rule is applied to every part of
//! every package, authored or preserved. Anything an authored part would have been faulted for, a
//! preserved part is faulted for too. The asymmetry the rule needs is kept where it belongs — in
//! category 2, which is precisely "markup we never write and must not be asked to validate".
//!
//! [`assert_authored_parts_are_categorised`](crate::assert_authored_parts_are_categorised) is the
//! provenance-exact form of the same rule, for a caller that still holds a live
//! [`Package`](mjx_opc::Package).

use mjx_ooxml_types::child_order;
use mjx_ooxml_types::namespaces::{self, SchemaNamespace};

/// Which schema tree an XSD lives in. The markup schemas are ECMA-376 Part 4 (Transitional); the
/// packaging schemas are ECMA-376 Part 2 (OPC) and ship in a separate archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaSet {
    /// ECMA-376 Part 4, `OfficeOpenXML-XMLSchema-Transitional`.
    Markup,
    /// ECMA-376 Part 2, `OpenPackagingConventions-XMLSchema`.
    Packaging,
}

/// The schema governing one namespace: which tree it lives in and its file name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaRef {
    /// Which of the two schema trees the file is in.
    pub set: SchemaSet,
    /// The XSD's file name, e.g. `wml.xsd`.
    pub file: &'static str,
}

/// Whether a modelled schema has a generated `child_order` table, and if not, why not.
///
/// `xtask`'s `CHILD_ORDER_SCHEMAS` is the list of schemas whose complex-type child orders are
/// generated. A schema joins it when a crate starts authoring its markup. This enum is how the gate
/// says which schemas are already there, which are owed a row by a named work item, and which will
/// never have one — so that a *pending* row cannot quietly become a permanent hole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderingCoverage {
    /// The schema is in `CHILD_ORDER_SCHEMAS`; every part rooted here is audited for child order.
    Generated,
    /// No table yet. `owner` is the work item that adds it — this is a declared, owned gap. The
    /// unit test `the_ordering_gaps_are_exactly_the_declared_ones` in this module fails the moment
    /// the gap closes without this entry being updated, so it cannot rot into a silent skip.
    Pending {
        /// The work item that owns closing the gap.
        owner: &'static str,
        /// Why the row is not this gate's to add.
        reason: &'static str,
    },
    /// The schema is outside the child-order generator's remit for good, with the reason stated.
    NotGenerated {
        /// Why no table will ever exist for this schema.
        reason: &'static str,
    },
}

/// Category 1: markup this project models, so a part rooted here must validate.
#[derive(Debug, Clone, Copy)]
pub struct ModeledSchema {
    /// The Transitional namespace URI of the schema's target namespace.
    pub namespace: &'static str,
    /// A human name for reports, e.g. `WordprocessingML`.
    pub label: &'static str,
    /// The XSD a part rooted in this namespace is validated against.
    pub schema: SchemaRef,
    /// Whether the generated child-order tables cover this schema.
    pub ordering: OrderingCoverage,
    /// The local name of a **global element** the schema declares, used to ask
    /// [`child_order::root_element`] whether the generated tables know this schema at all. It is
    /// what makes [`OrderingCoverage`] a checked fact rather than a comment.
    pub probe_root_element: &'static str,
}

/// The OPC relationships stream (`_rels/*.rels`), rewritten by `mjx-opc` on every save.
pub const OPC_RELATIONSHIPS_NS: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships";
/// The OPC content-types stream (`[Content_Types].xml`), rewritten by `mjx-opc` on every save.
pub const OPC_CONTENT_TYPES_NS: &str =
    "http://schemas.openxmlformats.org/package/2006/content-types";
/// The OPC core-properties stream (`docProps/core.xml`), ECMA-376 Part 2.
pub const OPC_CORE_PROPERTIES_NS: &str =
    "http://schemas.openxmlformats.org/package/2006/metadata/core-properties";
/// `inkml:` — the W3C Ink Markup Language, the payload of a `p:contentPart`.
pub const INKML_NS: &str = "http://www.w3.org/2003/InkML";
/// `ax:` — the Microsoft ActiveX control markup an `activeX*.xml` part carries.
pub const ACTIVEX_NS: &str = "http://schemas.microsoft.com/office/2006/activeX";

/// Every namespace in category 1, with the schema it is validated against.
///
/// SpreadsheetML is here because an authored chart embeds a whole `.xlsx` workbook: without this
/// entry every part of that workbook would be reported skipped-as-foreign, which is the difference
/// between the gate covering the workbook and only looking as though it does. DrawingML-diagram is
/// here because `add_diagram` writes four `dgm:` parts. WordprocessingML is here so Phase C inherits
/// a gate rather than building one, which is the whole reason this crate exists.
pub const MODELED_SCHEMAS: &[ModeledSchema] = &[
    ModeledSchema {
        namespace: namespaces::PML.transitional,
        label: "PresentationML",
        schema: SchemaRef {
            set: SchemaSet::Markup,
            file: "pml.xsd",
        },
        ordering: OrderingCoverage::Generated,
        probe_root_element: "presentation",
    },
    ModeledSchema {
        namespace: namespaces::DML_MAIN.transitional,
        label: "DrawingML",
        schema: SchemaRef {
            set: SchemaSet::Markup,
            file: "dml-main.xsd",
        },
        ordering: OrderingCoverage::Generated,
        probe_root_element: "theme",
    },
    ModeledSchema {
        namespace: namespaces::DML_CHART.transitional,
        label: "DrawingML charts",
        schema: SchemaRef {
            set: SchemaSet::Markup,
            file: "dml-chart.xsd",
        },
        ordering: OrderingCoverage::Generated,
        probe_root_element: "chartSpace",
    },
    ModeledSchema {
        namespace: namespaces::DML_DIAGRAM.transitional,
        label: "DrawingML diagrams",
        schema: SchemaRef {
            set: SchemaSet::Markup,
            file: "dml-diagram.xsd",
        },
        // `add_diagram` authors four `dgm:` parts, so this schema *is* markup we write and cannot be
        // foreign-allowlisted. The row is not this gate's to add: the child-order tables are claimed
        // by the child that models the markup, and adding 58 unmodelled complex types here would put
        // a generated table in the tree with no writer using it.
        ordering: OrderingCoverage::Pending {
            owner: "MJXOFF-148",
            reason: "the DrawingML diagram model, and with it the `dml-diagram` row in \
                     `CHILD_ORDER_SCHEMAS`, is MJXOFF-148's deliverable; `add_diagram` writes the \
                     four parts from fixed templates today, which `xmllint` validates in full",
        },
        probe_root_element: "dataModel",
    },
    ModeledSchema {
        namespace: namespaces::SML.transitional,
        label: "SpreadsheetML",
        schema: SchemaRef {
            set: SchemaSet::Markup,
            file: "sml.xsd",
        },
        ordering: OrderingCoverage::Pending {
            owner: "MJXOFF-132",
            reason: "the `sml` row in `CHILD_ORDER_SCHEMAS` belongs to the Excel crate spine, which \
                     is the child that starts placing SpreadsheetML children; `mjx-chart`'s embedded \
                     workbook is written whole from a fixed template today",
        },
        probe_root_element: "workbook",
    },
    ModeledSchema {
        namespace: namespaces::WML.transitional,
        label: "WordprocessingML",
        schema: SchemaRef {
            set: SchemaSet::Markup,
            file: "wml.xsd",
        },
        ordering: OrderingCoverage::Pending {
            owner: "MJXOFF-90",
            reason: "the `wml` row in `CHILD_ORDER_SCHEMAS` belongs to the Word crate spine, the \
                     child that starts placing WordprocessingML children; nothing in this workspace \
                     authors `w:` markup yet",
        },
        probe_root_element: "document",
    },
    ModeledSchema {
        namespace: OPC_RELATIONSHIPS_NS,
        label: "OPC relationships",
        schema: SchemaRef {
            set: SchemaSet::Packaging,
            file: "opc-relationships.xsd",
        },
        ordering: OrderingCoverage::NotGenerated {
            reason: "ECMA-376 Part 2, which `xtask codegen` does not read — it parses the Part 4 \
                     markup schemas. `mjx-opc` writes the whole stream from its own model on every \
                     save and never places a child into an existing one, so there is no insertion \
                     point for an ordering table to govern",
        },
        probe_root_element: "Relationships",
    },
    ModeledSchema {
        namespace: OPC_CONTENT_TYPES_NS,
        label: "OPC content types",
        schema: SchemaRef {
            set: SchemaSet::Packaging,
            file: "opc-contentTypes.xsd",
        },
        ordering: OrderingCoverage::NotGenerated {
            reason: "ECMA-376 Part 2, as for the relationships stream: written whole on every save, \
                     never edited in place",
        },
        probe_root_element: "Types",
    },
];

/// How a part in category 2 is keyed. VML is the reason this is not simply a namespace: a `.vml`
/// part's root is a bare `<xml>` wrapper in **no namespace at all**, which is still a fact about the
/// markup and must still be answerable for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForeignMarkupKey {
    /// The root element's namespace URI.
    Namespace(&'static str),
    /// The root element is in no namespace — a VML drawing part's `<xml>` wrapper.
    NoNamespace,
}

/// Category 2: markup this project preserves verbatim and never writes, with the reason per entry.
#[derive(Debug, Clone, Copy)]
pub struct PreservedForeignMarkup {
    /// What identifies a part as this entry.
    pub key: ForeignMarkupKey,
    /// A human name for reports.
    pub label: &'static str,
    /// Why validating it against a schema would report noise rather than defects.
    pub reason: &'static str,
}

/// The complete category-2 allowlist. Every entry is a claim that the bytes came from somewhere
/// else and that this project only ever stores and re-emits them.
///
/// Adding an entry is a deliberate act: [`the_allowlist_has_no_dead_entries`] fails on an entry
/// nothing in the corpus reaches, and the sweep fails on a namespace that is on no list at all.
///
/// [`the_allowlist_has_no_dead_entries`]: crate::sweep::Sweep::assert_pinned_skips
pub const PRESERVED_FOREIGN_MARKUP: &[PreservedForeignMarkup] = &[
    PreservedForeignMarkup {
        key: ForeignMarkupKey::NoNamespace,
        label: "a VML drawing part",
        reason: "a `.vml` part's root is a bare `<xml>` wrapper the VML schemas declare no global \
                 element for, and `vml-main.xsd` cannot compile without an `xml.xsd` the \
                 Transitional set does not ship. VML is a Microsoft vocabulary this project stores \
                 and re-emits; `add_vml_drawing` takes the caller's bytes and writes them verbatim",
    },
    PreservedForeignMarkup {
        key: ForeignMarkupKey::Namespace(INKML_NS),
        label: "InkML",
        reason: "a W3C vocabulary, not an OOXML one. `add_ink` stores the caller's InkML document \
                 byte for byte; nothing in this workspace generates a stroke",
    },
    PreservedForeignMarkup {
        key: ForeignMarkupKey::Namespace(ACTIVEX_NS),
        label: "ActiveX control markup",
        reason: "a Microsoft vocabulary describing a COM control's persisted state. \
                 `add_activex_control` writes the caller's class id and state through; the payload \
                 is opaque to this project",
    },
    PreservedForeignMarkup {
        key: ForeignMarkupKey::Namespace(
            namespaces::SHARED_DOCUMENT_PROPERTIES_EXTENDED.transitional,
        ),
        label: "docProps/app.xml (extended properties)",
        reason: "document properties are preserved and never authored: `Package::empty` writes \
                 none, `Presentation::blank` ships none, and every one in the corpus arrived in a \
                 committed fixture. MJXOFF-149 owns the decision to author them — the moment it \
                 does, this entry moves to `MODELED_SCHEMAS` with \
                 `shared-documentPropertiesExtended.xsd`",
    },
    PreservedForeignMarkup {
        key: ForeignMarkupKey::Namespace(OPC_CORE_PROPERTIES_NS),
        label: "docProps/core.xml (core properties)",
        reason:
            "the Part 2 core-properties stream, preserved and never authored, on the same terms \
                 as the extended properties beside it. MJXOFF-149 owns the decision to author it",
    },
];

/// Which category a root element's namespace falls into.
#[derive(Debug, Clone, Copy)]
pub enum NamespaceCategory {
    /// Category 1: validate it against [`ModeledSchema::schema`].
    Modeled(&'static ModeledSchema),
    /// Category 2: skip it, printing [`PreservedForeignMarkup::reason`].
    PreservedForeign(&'static PreservedForeignMarkup),
    /// Category 3: on no list — a hard failure.
    Uncategorised,
}

/// Classifies a root element's namespace. `None` means the root is in no namespace at all.
#[must_use]
pub fn categorise(namespace: Option<&str>) -> NamespaceCategory {
    if let Some(namespace) = namespace {
        if let Some(modeled) = MODELED_SCHEMAS
            .iter()
            .find(|entry| entry.namespace == namespace)
        {
            return NamespaceCategory::Modeled(modeled);
        }
    }
    let key = match namespace {
        Some(namespace) => PRESERVED_FOREIGN_MARKUP.iter().find(
            |entry| matches!(entry.key, ForeignMarkupKey::Namespace(listed) if listed == namespace),
        ),
        None => PRESERVED_FOREIGN_MARKUP
            .iter()
            .find(|entry| entry.key == ForeignMarkupKey::NoNamespace),
    };
    match key {
        Some(entry) => NamespaceCategory::PreservedForeign(entry),
        None => NamespaceCategory::Uncategorised,
    }
}

/// The schema governing a namespace, or `None` when the namespace is not category 1.
///
/// This is the single arm table the whole gate reads: `MODELED_SCHEMAS` *is*
/// `schema_for_namespace`, so an arm cannot exist without the reason, the ordering verdict and the
/// probe element that go with it.
#[must_use]
pub fn schema_for_namespace(namespace: &str) -> Option<SchemaRef> {
    MODELED_SCHEMAS
        .iter()
        .find(|entry| entry.namespace == namespace)
        .map(|entry| entry.schema)
}

/// Every namespace ECMA-376 defines, in both conformance worlds — the set a consumer of these
/// formats "understands" for the purpose of resolving markup compatibility.
///
/// A `mc:Choice` requiring a namespace outside this set loses to its `mc:Fallback`, and an element
/// or attribute in an ignorable namespace outside it is dropped. That is exactly the view the base
/// Transitional schemas describe, which is what makes the resolved markup validatable at all.
#[must_use]
pub fn ecma_376_namespaces() -> Vec<&'static str> {
    let mut uris = Vec::with_capacity(namespaces::ALL.len() * 2);
    for SchemaNamespace {
        strict,
        transitional,
    } in namespaces::ALL
    {
        uris.push(*transitional);
        if let Some(strict) = strict {
            uris.push(*strict);
        }
    }
    // The Part 2 packaging namespaces are not in the generated Part 4 table.
    uris.push(OPC_RELATIONSHIPS_NS);
    uris.push(OPC_CONTENT_TYPES_NS);
    uris.push(OPC_CORE_PROPERTIES_NS);
    uris
}

/// Whether the generated child-order tables know this schema at all, asked through the schema's own
/// global element rather than by reading `CHILD_ORDER_SCHEMAS` from a comment.
#[must_use]
pub fn child_order_tables_cover(schema: &ModeledSchema) -> bool {
    child_order::root_element(schema.namespace, schema.probe_root_element).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The declared ordering gaps must be exactly the real ones. When MJXOFF-90 adds the `wml` row,
    /// this fails until its entry is flipped to `Generated` — so a closed gap cannot leave a stale
    /// `Pending` behind, and an *opened* gap cannot appear without a named owner.
    #[test]
    fn the_ordering_gaps_are_exactly_the_declared_ones() {
        for schema in MODELED_SCHEMAS {
            let covered = child_order_tables_cover(schema);
            match schema.ordering {
                OrderingCoverage::Generated => assert!(
                    covered,
                    "{} claims a generated child-order table, but the tables do not know its \
                     global element `{}` — either the codegen list lost the schema or this entry \
                     is wrong",
                    schema.label, schema.probe_root_element
                ),
                OrderingCoverage::Pending { owner, .. } => assert!(
                    !covered,
                    "{} is declared a pending ordering gap owned by {owner}, but the generated \
                     tables now cover it — close the gap here by making it `Generated`, or the \
                     ordering audit stays switched off for markup that is now audited",
                    schema.label
                ),
                OrderingCoverage::NotGenerated { .. } => assert!(
                    !covered,
                    "{} is declared permanently outside the child-order generator, but the tables \
                     cover it",
                    schema.label
                ),
            }
        }
    }

    /// `COVERAGE.md` is generated by `xtask` and names an owner for every schema whose child-order
    /// row is still pending; this enum names one for every schema *the gate categorises*. Two
    /// statements of the same fact drift, so this checks them against each other rather than
    /// letting each be right on its own.
    ///
    /// The document is read at compile time from the committed artifact — the gate does not
    /// regenerate it, it only refuses to disagree with it.
    #[test]
    fn the_declared_owners_agree_with_the_generated_coverage_document() {
        const COVERAGE: &str = include_str!("../../mjx-ooxml-types/COVERAGE.md");

        for schema in MODELED_SCHEMAS {
            // The packaging schemas are ECMA-376 Part 2 and are not in the markup set the
            // document reports on.
            if schema.schema.set != SchemaSet::Markup {
                continue;
            }
            let stem = schema
                .schema
                .file
                .strip_suffix(".xsd")
                .expect("a schema file name ends in .xsd");
            let row = COVERAGE
                .lines()
                .rfind(|line| line.starts_with(&format!("| {stem} | ")))
                .unwrap_or_else(|| panic!("COVERAGE.md has no child-order row for `{stem}`"));
            match schema.ordering {
                OrderingCoverage::Generated => assert!(
                    row.contains("generated —"),
                    "`{stem}` is declared `Generated` here, but COVERAGE.md says: {row}"
                ),
                OrderingCoverage::Pending { owner, .. } => assert!(
                    row.contains(owner),
                    "`{stem}` is declared a pending gap owned by {owner}, but COVERAGE.md names a \
                     different owner: {row}"
                ),
                OrderingCoverage::NotGenerated { .. } => {}
            }
        }
    }

    #[test]
    fn every_namespace_lands_in_exactly_one_category() {
        for schema in MODELED_SCHEMAS {
            assert!(
                matches!(
                    categorise(Some(schema.namespace)),
                    NamespaceCategory::Modeled(_)
                ),
                "{} must be category 1",
                schema.label
            );
            assert!(
                !PRESERVED_FOREIGN_MARKUP
                    .iter()
                    .any(|foreign| foreign.key == ForeignMarkupKey::Namespace(schema.namespace)),
                "{} is on both lists",
                schema.label
            );
            assert_eq!(schema_for_namespace(schema.namespace), Some(schema.schema));
        }
        for foreign in PRESERVED_FOREIGN_MARKUP {
            let namespace = match foreign.key {
                ForeignMarkupKey::Namespace(namespace) => Some(namespace),
                ForeignMarkupKey::NoNamespace => None,
            };
            assert!(
                matches!(
                    categorise(namespace),
                    NamespaceCategory::PreservedForeign(_)
                ),
                "{} must be category 2",
                foreign.label
            );
        }
        assert!(matches!(
            categorise(Some("urn:example:invented-for-this-test")),
            NamespaceCategory::Uncategorised
        ));
    }

    #[test]
    fn the_understood_set_holds_the_formats_and_not_the_vendor_extensions() {
        let understood = ecma_376_namespaces();
        for expected in [
            namespaces::WML.transitional,
            namespaces::SML.transitional,
            namespaces::PML.transitional,
            namespaces::VML_MAIN.transitional,
            OPC_RELATIONSHIPS_NS,
        ] {
            assert!(understood.contains(&expected), "missing {expected}");
        }
        // The `w14`/`p14` families are Microsoft extensions ECMA-376 does not define; a `mc:Choice`
        // requiring one must lose to its fallback, which is what makes the remainder validatable.
        for extension in [
            "http://schemas.microsoft.com/office/word/2010/wordml",
            "http://schemas.microsoft.com/office/powerpoint/2010/main",
        ] {
            assert!(
                !understood.contains(&extension),
                "{extension} must not be understood"
            );
        }
    }
}
