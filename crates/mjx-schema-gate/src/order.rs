//! The child-order half of the gate: no element carries a child out of its complex type's
//! `xsd:sequence`.
//!
//! This is the half that needs no external tool. `xmllint` catches an ordering fault only for the
//! shape some case happens to author, and only where `References/` exists; this walks **every
//! element of every part** whose root the generated tables know, on every run.
//!
//! # Why the old assertion was vacuous, and what replaced it
//!
//! The previous form asserted `!audited.is_empty()` — that *at least one part* had been audited.
//! `root_element` maps `a:theme`, so a `.docx` "covered" by that assertion meant
//! `word/theme/theme1.xml` was audited and nothing else was. [`TreeAudit::elements_visited`] was
//! already returned per part and thrown away, and `child_order.rs` says outright that an audit which
//! visits nothing passes vacuously.
//!
//! [`assert_deck_is_in_schema_order`] now asserts three separate facts:
//!
//! 1. **Coverage**, stated over the *category* table rather than over the audit's own lookup — see
//!    [`parts_that_must_be_audited`] for why that distinction is the difference between an
//!    assertion and a tautology. It is the assertion that grows itself: the moment MJXOFF-90 adds
//!    the `wml` rows and flips its `OrderingCoverage`, every `word/*.xml` part becomes required
//!    here with no edit to this file.
//! 2. **Non-vacuity, per part.** Each audited part descended into whatever structure its root had
//!    — see [`MINIMUM_ELEMENTS_VISITED`].
//! 3. **Something was audited at all.**
//!
//! # The markup it walks is the markup the schema arm validates (MJXOFF-272)
//!
//! Both arms are pointed at the same part, and until MJXOFF-272 they did not see the same markup:
//! [`crate::inspect`] resolves markup compatibility first, and this walked the raw tree. The
//! generated tables name no `mc:AlternateContent` slot, so `child_order::audit_tree` stepped over
//! every such element without entering it — and what that costs depends only on where the element
//! sits:
//!
//! * **as a root's only child**, the walk recognises nothing and visits one element, which
//!   [`MINIMUM_ELEMENTS_VISITED`] catches. A red is a working alarm;
//! * **as one root child among several**, the walk descends into the siblings, reports a count that
//!   looks exactly like a healthy one, and audits the `mc:` subtree not at all. **No floor can see
//!   this** — the number it reads is the number a healthy audit of the siblings produces.
//!
//! So this module resolves too, through the one function that produces that view, and the case that
//! holds it up is `tests/ordering_under_markup_compatibility.rs` — written against *what was
//! visited* rather than against a total, because a total is precisely what the quiet shape leaves
//! looking right.
//!
//! It is the **same** view down to the [wildcard slot](crate::wildcard_slots) rule, not a second one
//! shaped for walking. That rule drops an element resolution emptied, and a walk does not need it
//! the way `xmllint` does — but two views assembled for two arms is the arrangement MJXOFF-196
//! refused, and an emptied slot has no children left to put in any order, so nothing is given up by
//! sharing. What the arms report about a part is a fact about one piece of markup.
//!
//! # Emptied is recorded, not reported (MJXOFF-273)
//!
//! Resolving first bought a second way for a root to arrive with no element children, and the two
//! are not the same fact. `charts.pptx`'s `/ppt/tableStyles.xml` is a genuinely empty
//! `a:tblStyleLst`: there was nothing there and the audit saw all of it.
//! `legacy_form_control.xlsx`'s `/xl/drawings/drawing1.xml` is an `xdr:wsDr` whose only child is an
//! `mc:AlternateContent` holding one `mc:Choice Requires="a14"` and **no** `mc:Fallback`, so
//! resolution drops the subtree and the audit saw *nothing of what the file contains*. Both then
//! report `root_child_elements = 0`, `elements_visited = 1`, floor 1, clean.
//!
//! [`AuditedPart::raw_root_child_elements`] is what tells them apart, and it changes **no verdict**.
//! It is not a defect and must not be made to look like one: auditing the losing choice would mean
//! faulting a producer's extension markup against schemas that do not describe it, which is exactly
//! what `child_order.rs` refuses to do, and flagging every emptied root as a finding would turn that
//! deliberate, correct refusal into a red on every Office-authored drawing. So the arm *records* the
//! distinction — a reader of an [`OrderAudit`], and `xtask validation-artefacts --ingest`, can say
//! which of the two they are looking at — while [`assert_deck_is_in_schema_order`] goes on asserting
//! only what it asserted before.
//!
//! [`TreeAudit::elements_visited`]: mjx_ooxml_types::child_order::TreeAudit::elements_visited

use mjx_ooxml_types::child_order;
use mjx_opc::{Package, PartName};

// The rule for "this content type names an XML payload" is [`crate::inspect`]'s, *called* rather
// than restated. It stood here as a second copy of the same three lines until MJXOFF-221 found
// both copies matching `vmlDrawing` case-sensitively; two copies of a string-literal guard are two
// places for the next one to go wrong.
use crate::inspect::{
    carries_markup_compatibility, is_xml_content_type, markup_compatibility_resolved_tree,
};

use crate::categories::{categorise, NamespaceCategory, OrderingCoverage};

/// The least number of elements an audited part must have visited, **when its root has element
/// children at all**.
///
/// **Two, and the number is not arbitrary.** The walk visits an element only when the tables name
/// its complex type, so a count of **one** means exactly this: the tables knew the part's *root*
/// type and recognised **none** of its children — the vacuous audit `child_order.rs` warns about,
/// and the shape a `.docx` took when `a:theme` alone satisfied the old `!audited.is_empty()`.
///
/// The qualifier is not a loophole; it is a measured fact. `charts.pptx` ships a legitimately empty
/// `/ppt/tableStyles.xml` — an `a:tblStyleLst` with no children — and a walk that visits one element
/// there is complete, not vacuous. So the floor is *one* for a root with no element children and
/// *two* otherwise: the assertion is "the walk descended into whatever structure was there", which
/// is what non-vacuity means, rather than "the part was big enough".
///
/// A higher floor would be a guess about part size rather than a statement about the walk. Cases
/// that want a specific depth pin it themselves — `mjx-pptx`'s coverage case asserts more than five
/// on each of the five parts a filled blank deck writes.
pub const MINIMUM_ELEMENTS_VISITED: usize = 2;

/// One audited part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditedPart {
    /// The part name, e.g. `/ppt/slides/slide1.xml`.
    pub name: String,
    /// How many elements the walk actually checked.
    pub elements_visited: usize,
    /// How many element children the root has — the structure the walk had available to descend
    /// into, which is what makes [`MINIMUM_ELEMENTS_VISITED`] a statement about the walk rather
    /// than about the part's size.
    ///
    /// Counted on the **resolved** root, like everything else here: a part whose whole content sat
    /// inside a losing `mc:Choice` really does present a conforming consumer with an empty root, and
    /// reporting the raw child it no longer has would make the floor read a structure the walk was
    /// never going to enter.
    pub root_child_elements: usize,
    /// How many element children the root had **before** markup compatibility was resolved.
    ///
    /// This decides nothing — the floor reads [`root_child_elements`], because the resolved tree is
    /// the only structure the walk was ever going to enter. It is here so a reader can tell an
    /// empty part from an emptied one (MJXOFF-273): both arrive with `root_child_elements = 0` and
    /// a complete one-element audit, and only this number says whether the file had content the
    /// conforming view discards.
    ///
    /// It is **not** an upper bound on [`root_child_elements`], and reading it as one would be
    /// wrong: resolution replaces an `mc:AlternateContent` with the children of the winning branch,
    /// so a root loses children when the branch is smaller, gains them when it is larger, and keeps
    /// the count when it is one for one. A part carrying no markup compatibility at all reports the
    /// two equal, which is nearly every part of nearly every package.
    ///
    /// [`root_child_elements`]: AuditedPart::root_child_elements
    pub raw_root_child_elements: usize,
}

impl AuditedPart {
    /// The least number of elements this particular part's audit must have visited.
    #[must_use]
    pub fn floor(&self) -> usize {
        if self.root_child_elements == 0 {
            1
        } else {
            MINIMUM_ELEMENTS_VISITED
        }
    }

    /// Whether this root arrived with element children and markup compatibility resolution left it
    /// with none — so the audit below is complete over *nothing the file contains*.
    ///
    /// **Not a defect**, and no assertion here treats it as one; see the module documentation. It
    /// separates `legacy_form_control.xlsx`'s `xdr:wsDr` (`true`) from `charts.pptx`'s empty
    /// `a:tblStyleLst` (`false`), which are otherwise the same row.
    #[must_use]
    pub fn emptied_by_markup_compatibility_resolution(&self) -> bool {
        self.root_child_elements == 0 && self.raw_root_child_elements > 0
    }
}

/// What one child-order audit found, **without panicking on any of it**.
///
/// [`audit_deck_order`] and [`assert_deck_is_in_schema_order`] are both written on top of this, so
/// there is one walk rather than two spellings of it. The reporting form exists because a *reporter*
/// — `xtask validation-artefacts --ingest`, which hands an Office-authored file back with every check
/// answered — must print the round-trip and package verdicts too, and a panic on the first ordering
/// defect would take the rest of the report with it (MJXOFF-130).
#[derive(Debug, Default)]
pub struct OrderAudit {
    /// One entry per part the walk actually audited.
    pub audited: Vec<AuditedPart>,
    /// Every ordering defect found, each already phrased as the message the assertion raises.
    /// **All of them**, not the first: a reporter wants the whole list.
    pub defects: Vec<String>,
}

impl OrderAudit {
    /// The parts that *had* to be audited and were not — a codegen gap rather than a permitted skip.
    /// `required` comes from [`parts_that_must_be_audited`], which is deliberately derived from the
    /// category table rather than from the audit's own lookup.
    #[must_use]
    pub fn missed<'a>(&self, required: &'a [String]) -> Vec<&'a String> {
        required
            .iter()
            .filter(|part| !self.audited.iter().any(|entry| &entry.name == *part))
            .collect()
    }

    /// The audited parts whose walk visited less than their [`AuditedPart::floor`] — a vacuous audit.
    #[must_use]
    pub fn vacuous(&self) -> Vec<&AuditedPart> {
        self.audited
            .iter()
            .filter(|part| part.elements_visited < part.floor())
            .collect()
    }

    /// The audited parts whose root markup compatibility resolution emptied — a **complete audit
    /// over nothing the file contains**, which is not the same fact as an empty part and is not a
    /// defect either (MJXOFF-273).
    ///
    /// Kept separate from [`vacuous`](OrderAudit::vacuous) deliberately: a vacuous audit is a
    /// codegen gap to chase, this is markup the gate correctly declines to describe.
    #[must_use]
    pub fn emptied_by_markup_compatibility_resolution(&self) -> Vec<&AuditedPart> {
        self.audited
            .iter()
            .filter(|part| part.emptied_by_markup_compatibility_resolution())
            .collect()
    }
}

/// Runs the child-order audit over every part of `bytes` whose root element the generated tables
/// name, reporting every defect instead of panicking on the first.
///
/// A package that cannot be opened — the outer one or an embedded workbook — is itself reported as a
/// defect rather than raised, for the same reason: the caller is a reporter.
#[must_use]
pub fn audit_order_report(label: &str, bytes: &[u8]) -> OrderAudit {
    let mut report = OrderAudit::default();
    audit_package_order(label, bytes, "", &mut report);
    report
}

/// Runs the child-order audit over every part of `bytes` whose root element the generated tables
/// name, panicking on the first defect. Returns one [`AuditedPart`] per audited part, so a caller
/// can prove the walk is not passing vacuously.
///
/// # Panics
/// If the package cannot be opened, or a part carries a child out of its `xsd:sequence`.
#[must_use]
pub fn audit_deck_order(label: &str, bytes: &[u8]) -> Vec<AuditedPart> {
    let report = audit_order_report(label, bytes);
    if let Some(defect) = report.defects.first() {
        panic!("{defect}");
    }
    report.audited
}

/// The content type of an embedded Office package — a chart's workbook. Restated from
/// `inspect.rs`'s own list rather than shared, for the reason that module's copy gives: each half of
/// the gate states the fact it acts on.
const EMBEDDED_PACKAGE_CONTENT_TYPES: [&str; 1] =
    ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"];

/// Audits one package, appending to `audited`, and descends into any package embedded in it.
///
/// `prefix` names where the package sits: empty for the document itself, and
/// `/word/embeddings/Microsoft_Excel_Sheet1.xlsx!` for a chart's workbook — the **same naming the
/// validation half uses** (`inspect_package`), so a part appears under one name in both halves of
/// the gate's report.
///
/// # Why the descent exists (MJXOFF-103)
///
/// Until this child the ordering audit walked the outer package only, while the validation half had
/// descended into an embedded workbook since A5. That asymmetry meant **no chart's embedded
/// workbook was ever audited for child order, in any format** — `xmllint` checked its SpreadsheetML
/// and the generated `sml` tables checked nothing, even though `sml` has been in
/// `CHILD_ORDER_SCHEMAS` since MJXOFF-132 and `mjx-sml`'s writer is what composes those parts. It
/// was found by a Word case asserting the nested worksheet was audited and discovering it was not
/// in the list at all; the hole was never Word-specific, and closing it here closes it for
/// `mjx-pptx` in the same commit.
fn audit_package_order(label: &str, bytes: &[u8], prefix: &str, report: &mut OrderAudit) {
    let mut package = match Package::open(bytes) {
        Ok(package) => package,
        Err(e) => {
            report
                .defects
                .push(format!("{label}: opening package: {e}"));
            return;
        }
    };
    let parts: Vec<PartName> = package.part_names().collect();
    for part in parts {
        let Some(content_type) = package.content_type_of(&part).map(str::to_owned) else {
            continue;
        };
        if EMBEDDED_PACKAGE_CONTENT_TYPES.contains(&content_type.as_str()) {
            let Some(payload) = package
                .part_payload(&part)
                .map(std::borrow::Cow::into_owned)
            else {
                continue;
            };
            let nested = format!("{prefix}{}!", part.as_str());
            audit_package_order(label, &payload, &nested, report);
            continue;
        }
        if !is_xml_content_type(&content_type) {
            continue;
        }
        let Ok(document) = package.part_tree(&part) else {
            continue;
        };
        // Read off the tree the package holds, before the line below rebinds `document` to the
        // resolved view: it is the only place the raw shape is still in hand, and it is what
        // separates an emptied root from an empty one (MJXOFF-273).
        let raw_root_child_elements = element_children(&document.root);
        // The **same view [`crate::inspect`] validates** (MJXOFF-272). A part is re-serialized only
        // when it really carries markup compatibility, so the common path still walks the tree the
        // package holds.
        let resolved = if carries_markup_compatibility(&document.root, &document.interner) {
            match markup_compatibility_resolved_tree(document) {
                Ok(resolved) => Some(resolved),
                Err(error) => {
                    report.defects.push(format!(
                        "{label}: {prefix}{} carries markup compatibility that will not resolve, \
                         so its child order could not be audited at all — {error}",
                        part.as_str()
                    ));
                    continue;
                }
            }
        } else {
            None
        };
        let document = resolved.as_ref().unwrap_or(document);
        let interner = &document.interner;
        let root = &document.root;
        let Some(namespace) = root.name.namespace.map(|symbol| interner.resolve(symbol)) else {
            continue;
        };
        let Some(order) = child_order::root_element(namespace, interner.resolve(root.name.local))
        else {
            // A part whose root the tables do not name. Which parts those may be is not left open:
            // `assert_deck_is_in_schema_order` re-derives the set from the category tables and
            // fails if one of them *should* have been audited.
            continue;
        };
        let audit = child_order::audit_tree(order, root, interner);
        if let Some(defect) = audit.defect {
            report.defects.push(format!(
                "{label}: {prefix}{} is out of schema order — {defect}",
                part.as_str()
            ));
            continue;
        }
        report.audited.push(AuditedPart {
            name: format!("{prefix}{}", part.as_str()),
            elements_visited: audit.elements_visited,
            root_child_elements: element_children(root),
            raw_root_child_elements,
        });
    }
}

/// How many of an element's children are elements.
///
/// Called twice on the same part — once on the raw root and once on the resolved one — which is the
/// whole reason it is a function rather than the inline `filter().count()` it replaced: two
/// spellings of one count are two places for the next change to reach only one.
fn element_children(element: &mjx_ooxml_core::RawElement) -> usize {
    element
        .children
        .iter()
        .filter(|child| matches!(child, mjx_ooxml_core::RawNode::Element(_)))
        .count()
}

/// Every part of `bytes` the ordering audit **must** have reached: one whose root namespace is a
/// modelled schema whose child-order table is [`Generated`](crate::OrderingCoverage::Generated).
///
/// This is deliberately derived from the *category* table rather than from `root_element`. Asking
/// `root_element` which parts it knows and then asserting the audit reached exactly those would be
/// a tautology — the audit uses the same lookup. Asking the category table instead states something
/// the audit cannot make true by itself: *this schema is generated, so every part rooted in it is
/// audited*. A new global element the generator missed fails here, naming the part.
///
/// A schema whose coverage is `Pending` is not required — and the requirement appears on its own
/// the moment its owner flips the entry. There is no such schema left: WordprocessingML flipped with
/// MJXOFF-90, DrawingML diagrams with MJXOFF-148 and SpreadsheetML with MJXOFF-132, so every
/// modelled markup namespace this gate categorises is now audited. `Pending` stays because the enum
/// is how a *future* gap gets a named owner, not because one is open.
///
/// # Panics
/// If the package cannot be opened.
#[must_use]
pub fn parts_that_must_be_audited(label: &str, bytes: &[u8]) -> Vec<String> {
    let mut expected = Vec::new();
    let mut package =
        Package::open(bytes).unwrap_or_else(|e| panic!("{label}: opening package: {e}"));
    let parts: Vec<PartName> = package.part_names().collect();
    for part in parts {
        let Some(content_type) = package.content_type_of(&part).map(str::to_owned) else {
            continue;
        };
        if !is_xml_content_type(&content_type) {
            continue;
        }
        let Ok(document) = package.part_tree(&part) else {
            continue;
        };
        let namespace = document
            .root
            .name
            .namespace
            .map(|symbol| document.interner.resolve(symbol).to_owned());
        let local_name = document
            .interner
            .resolve(document.root.name.local)
            .to_owned();
        if let NamespaceCategory::Modeled(schema) = categorise(namespace.as_deref(), &local_name) {
            if schema.ordering == OrderingCoverage::Generated {
                expected.push(part.as_str().to_owned());
            }
        }
    }
    expected
}

/// Asserts that no element of any part of `bytes` carries a child out of its complex type's
/// `xsd:sequence`, that every part the tables *could* audit was audited, and that no audit passed
/// vacuously.
///
/// # Why this may be pointed at a whole deck
///
/// Some of these decks are a committed fixture opened, edited and saved, so they carry parts this
/// library did not write. That is deliberate and safe: the fixtures are themselves schema-valid (the
/// `*_fixture_is_schema_valid` cases prove it with `xmllint`), so a fault this raises is markup
/// **this** library placed. The library never re-orders what it reads — placement only ever runs on
/// a child a caller asked to write — and the byte-identity suites in `mjx-opc` hold that line.
///
/// # Panics
/// On an ordering defect, a part the tables could audit but the walk missed, a vacuous audit, or a
/// deck in which nothing at all was audited.
pub fn assert_deck_is_in_schema_order(label: &str, bytes: &[u8]) {
    let report = audit_order_report(label, bytes);
    if let Some(defect) = report.defects.first() {
        panic!("{defect}");
    }
    let audited = report.audited;
    let expected = parts_that_must_be_audited(label, bytes);

    let missed: Vec<&String> = expected
        .iter()
        .filter(|part| !audited.iter().any(|entry| &entry.name == *part))
        .collect();
    assert!(
        missed.is_empty(),
        "{label}: {missed:?} are rooted in a schema whose child-order table is generated, but the \
         walk did not audit them — the generated `root_element` map does not name their root, which \
         is a codegen gap, not a permitted skip"
    );
    assert!(
        !audited.is_empty(),
        "{label}: not one part was audited for child order — an ordering gate that reaches nothing \
         is the vacuous pass this assertion exists to prevent"
    );
    for part in &audited {
        assert!(
            part.elements_visited >= part.floor(),
            "{label}: {} visited {} element(s) though its root has {} element child(ren); the walk \
             knew the root's complex type and recognised none of its children, which is a vacuous \
             audit rather than a clean one",
            part.name,
            part.elements_visited,
            part.root_child_elements
        );
    }
}
