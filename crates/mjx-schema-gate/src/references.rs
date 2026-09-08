//! The reference-resolution half of the gate: **every reference a package's own content makes
//! resolves to something the package contains.**
//!
//! # The hole this closes (MJXOFF-200, MJXOFF-198 §5)
//!
//! Every other machine check in this repository asks whether *the bytes we wrote are the bytes we
//! meant*. Schema validation asks whether each part is well-typed. [`crate::order`] asks whether
//! each element's children are in `xsd:sequence` order. `Package::validate` asks whether the
//! packaging invariants hold. Byte identity asks whether an untouched part came back unchanged.
//!
//! **None of them asks whether a part we did not write should have existed.** An absent optional
//! part is invisible to all four, and that is exactly how a `.docx` this library authored shipped
//! for months with charts that painted a title, axes, category labels, legend text, a plot frame —
//! and no bars at all. The chart series carried no `c:spPr`, which is correct and deliberate: a
//! series with no shape properties takes its fill from the theme's `accent1…accent6`, so the host
//! document's brand wins. There was no theme part, so `accent1` resolved to nothing, and nothing in
//! the workspace could tell.
//!
//! A test asserting *"the package has a theme part"* would close that instance and nothing else. It
//! would also pass the day someone wrote an **empty** theme, at which point the bars vanish again.
//! So this module states the class instead: a package is finished not when it is valid but when
//! every reference its content makes has something to resolve against.
//!
//! # What counts as a reference here — and where the line is drawn
//!
//! **In scope**, because each names something that must be *present in the package* and is cheap to
//! resolve exactly:
//!
//! | Reference | Resolves against |
//! |---|---|
//! | A relationship id (`r:id`, `r:embed`, `r:link`, …) | that part's own `.rels` |
//! | An internal relationship target | a part of the package |
//! | `a:schemeClr@val` (DrawingML scheme colour) | the theme's `a:clrScheme` slot |
//! | A `c:ser` with **no** `c:spPr` — an *implicit* scheme colour | the theme's `accent1…accent6` |
//! | `+mj-…` / `+mn-…` typefaces (DrawingML theme fonts) | the theme's `a:fontScheme` collection |
//! | `w:*@w:themeColor` / `@w:themeFill` | the theme's `a:clrScheme` slot |
//! | `w:rFonts@w:asciiTheme` and its three siblings | the theme's `a:fontScheme` collection |
//! | SpreadsheetML `@theme` on a colour | the theme's colour-scheme slot at that index |
//! | SpreadsheetML `<scheme val="major\|minor"/>` | the theme's `a:fontScheme` collection |
//! | `w:pStyle` / `w:rStyle` / `w:tblStyle` / `w:basedOn` / `w:next` / `w:link` | a `w:style@w:styleId` in `styles.xml` |
//! | `w:numPr > w:numId` | a `w:num@w:numId` in `numbering.xml` |
//! | `a:tableStyleId` | an `a:tblStyle@styleId` in `tableStyles.xml` |
//! | SpreadsheetML `c@s` / `row@s` / `col@style` / `xf@xfId` | a record of the named `styles.xml` table |
//! | SpreadsheetML `xf@fontId` / `@fillId` / `@borderId` | a record of the named `styles.xml` table |
//!
//! **Out of scope, deliberately:**
//!
//! * **Whether a value that resolves is the *right* value.** `accent1` resolving to blue rather than
//!   to green is a design question, not a broken reference.
//! * **Inheritance.** Whether a paragraph ends up with the formatting a person expected, walking
//!   `w:basedOn` chains or a placeholder's layout/master ancestry, is `effective_*`'s job and has
//!   its own suites.
//! * **Anything needing the ECMA-376 XSDs.** That is the other half of this crate, and it skips
//!   without `References/`; this half runs on every machine, like [`crate::order`].
//! * **References that leave the package.** An `External` relationship target is a URL by
//!   definition; nothing here fetches one.
//! * **Markup we merely preserve.** Every markup check below is keyed on the element's namespace
//!   being DrawingML, WordprocessingML or SpreadsheetML, so a reference inside VML, InkML, an
//!   ActiveX control or any other namespace this project copies verbatim is never resolved: it is
//!   not ours, and reading it would be guessing. Relationship ids are the deliberate exception and
//!   are checked in **every** namespace, because a `.rels` entry is packaging rather than markup —
//!   a VML drawing's `o:relid` still has to name a real relationship of its own part.
//! * **`numFmtId` below 164.** Those are the built-in number formats; ECMA-376 §18.8.30 defines
//!   them and no `numFmt` record is written for one.
//!
//! # Two of the rules are not new coverage, and say so
//!
//! `mjx_opc::Package::validate` — which `Package::save` runs on every save — already refuses an
//! `r:id` that names no relationship (`UndeclaredRelationshipReference`) and an internal
//! relationship target with no part behind it (`RelationshipTargetMissing`). The two relationship
//! rules here therefore add nothing for a package **we** write; they are included so the audit
//! states the whole class in one place, and so it still holds over bytes that never went through
//! our writer. `tests/reference_resolution.rs` writes those two negative controls with
//! `save_unchecked` for exactly that reason, and asserts that `save` refuses them.
//!
//! # What this is asserted over
//!
//! [`assert_authored_package_resolves_every_reference`] is applied to **packages this library
//! authors**, which is the population the ticket names: a producer's file may contain whatever it
//! contains, and repairing one is not this project's business, but a package we wrote and cannot
//! resolve is our defect. It descends into an embedded package (a chart's workbook) exactly as
//! [`crate::order`] does, because that is a package we author too.

use std::collections::{BTreeMap, BTreeSet};

use mjx_ooxml_core::{Interner, RawElement, RawNode};
use mjx_ooxml_types::namespaces;
use mjx_opc::{Package, PartName, TargetMode};

/// The content type of a theme part (DrawingML, ECMA-376 Part 1 §14.2.7) — the same string in all
/// three formats.
const CONTENT_TYPE_THEME: &str = "application/vnd.openxmlformats-officedocument.theme+xml";

/// The content type of a WordprocessingML styles part (§11.3.12).
const CONTENT_TYPE_WORD_STYLES: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml";

/// The content type of a WordprocessingML numbering part (§11.3.11).
const CONTENT_TYPE_WORD_NUMBERING: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml";

/// The content type of a PresentationML table-styles part (§13.3.10).
const CONTENT_TYPE_TABLE_STYLES: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml";

/// The content type of a SpreadsheetML styles part (§12.3.20).
const CONTENT_TYPE_SHEET_STYLES: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml";

/// The content type of an embedded Office package — a chart's workbook. Restated here rather than
/// shared, for the reason [`crate::order`]'s own copy gives: each half of the gate states the fact
/// it acts on.
const EMBEDDED_PACKAGE_CONTENT_TYPES: [&str; 1] =
    ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"];

/// The lowest `numFmtId` a file has to define for itself. Everything below it is one of ECMA-376
/// §18.8.30's built-in formats, which no `numFmt` record accompanies.
const FIRST_CUSTOM_NUMBER_FORMAT_ID: u32 = 164;

/// One reference that resolves to nothing.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DanglingReference {
    /// The part that makes the reference, prefixed for an embedded package exactly as
    /// [`crate::order`] prefixes it.
    pub part: String,
    /// Where in that part — the element and attribute, e.g. `a:schemeClr@val`.
    pub site: String,
    /// What the reference names, verbatim.
    pub reference: String,
    /// Why it does not resolve, in a sentence a reader can act on.
    pub reason: String,
}

impl std::fmt::Display for DanglingReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} names {:?} — {}",
            self.part, self.site, self.reference, self.reason
        )
    }
}

/// What one reference audit found, **without panicking on any of it**.
///
/// [`audit_package_references`] and [`assert_authored_package_resolves_every_reference`] are both
/// written on top of this, so there is one walk rather than two spellings of it — the same shape
/// [`crate::order::audit_deck_order`] takes, and for the same reason: a *reporter*
/// (`xtask validation-artefacts --ingest`) must print every verdict rather than stop at the first.
#[derive(Debug, Clone, Default)]
pub struct ReferenceAudit {
    /// Every reference that resolved to nothing.
    pub dangling: Vec<DanglingReference>,
    /// How many references were checked and did resolve — the number that makes a green verdict a
    /// statement rather than a tautology.
    pub resolved: usize,
    /// The parts whose content was walked, prefixed as in [`DanglingReference::part`].
    pub parts_walked: Vec<String>,
    /// Anything that stopped the walk itself: a package that would not open, a part that would not
    /// parse. Never silently swallowed, because a walk that visited nothing must not read as proof.
    pub errors: Vec<String>,
}

impl ReferenceAudit {
    /// Whether every reference resolved and nothing stopped the walk.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.dangling.is_empty() && self.errors.is_empty()
    }

    /// The report a failing assertion prints.
    #[must_use]
    pub fn report(&self) -> String {
        let mut out = String::new();
        for error in &self.errors {
            out.push_str("  error: ");
            out.push_str(error);
            out.push('\n');
        }
        for dangling in &self.dangling {
            out.push_str("  ");
            out.push_str(&dangling.to_string());
            out.push('\n');
        }
        out
    }
}

/// Audits `bytes` as a package and answers every reference in it that resolves to nothing.
///
/// See the [module documentation](self) for what counts as a reference and where the line is drawn.
#[must_use]
pub fn audit_package_references(bytes: &[u8]) -> ReferenceAudit {
    let mut audit = ReferenceAudit::default();
    audit_package(bytes, "", &mut audit);
    audit
}

/// Asserts that a package **this library authored** resolves every reference its content makes.
///
/// # Panics
/// With the full report if any reference dangles, or if anything stopped the walk. It also panics
/// when the walk checked **nothing at all**: an audit that visits no reference passes vacuously,
/// which is the failure mode MJXOFF-88 §7 names and the one this whole crate exists to close.
pub fn assert_authored_package_resolves_every_reference(label: &str, bytes: &[u8]) {
    let audit = audit_package_references(bytes);
    assert!(
        audit.is_clean(),
        "{label}: a package we authored makes {} reference(s) that resolve to nothing:\n{}",
        audit.dangling.len(),
        audit.report()
    );
    assert!(
        audit.resolved > 0,
        "{label}: the reference audit resolved nothing at all, so it proves nothing. \
         Parts walked: {:?}",
        audit.parts_walked
    );
}

// -------------------------------------------------------------------------------------------
// The walk
// -------------------------------------------------------------------------------------------

/// Audits one package, appending to `audit`, and descends into any package embedded in it.
///
/// `prefix` names where the package sits: empty for the document itself, and
/// `/word/embeddings/Microsoft_Excel_Sheet1.xlsx!` for a chart's workbook — the same naming both
/// other halves of the gate use, so a part appears under one name in every report.
fn audit_package(bytes: &[u8], prefix: &str, audit: &mut ReferenceAudit) {
    let mut package = match Package::open(bytes) {
        Ok(package) => package,
        Err(e) => {
            audit.errors.push(format!("{prefix}opening package: {e}"));
            return;
        }
    };

    let facts = PackageFacts::gather(&mut package, prefix, audit);
    check_relationship_targets(&package, prefix, audit);

    let parts: Vec<PartName> = package.part_names().collect();
    for part in parts {
        let Some(content_type) = package.content_type_of(&part).map(str::to_owned) else {
            continue;
        };
        if EMBEDDED_PACKAGE_CONTENT_TYPES.contains(&content_type.as_str()) {
            if let Some(payload) = package
                .part_payload(&part)
                .map(std::borrow::Cow::into_owned)
            {
                let nested = format!("{prefix}{}!", part.as_str());
                audit_package(&payload, &nested, audit);
            }
            continue;
        }
        if !is_walkable_xml(&content_type) {
            continue;
        }
        let name = format!("{prefix}{}", part.as_str());
        let relationship_ids = relationship_ids_of(&package, &part);
        let document = match package.part_tree(&part) {
            Ok(document) => document,
            Err(e) => {
                audit.errors.push(format!("{name}: parsing: {e}"));
                continue;
            }
        };
        audit.parts_walked.push(name.clone());
        let mut walker = PartWalker {
            part: &name,
            interner: &document.interner,
            facts: &facts,
            relationship_ids: &relationship_ids,
            audit,
            namespace_scope: Vec::new(),
            implicit_series_count: 0,
        };
        walker.element(&document.root);
    }
}

/// Whether a content type names markup this module walks.
///
/// The `.rels` parts are deliberately excluded: their `Id` attributes *declare* relationship ids
/// rather than reference them, and their targets are checked separately by
/// [`check_relationship_targets`], against the package rather than against themselves.
fn is_walkable_xml(content_type: &str) -> bool {
    if content_type == "application/vnd.openxmlformats-package.relationships+xml" {
        return false;
    }
    content_type.ends_with("+xml") || content_type.ends_with("/xml")
}

/// The relationship ids declared for one part, plus those of the package root when `part` is the
/// root's own.
fn relationship_ids_of(package: &Package, part: &PartName) -> BTreeSet<String> {
    package
        .relationships_for(Some(part))
        .into_iter()
        .flat_map(mjx_opc::Relationships::iter)
        .map(|rel| rel.id.clone())
        .collect()
}

/// Every internal relationship target resolves to a part the package holds.
///
/// This is the one check made from the packaging side rather than from the markup, because a
/// relationship is where a part *names another part* — the coarsest reference in the format, and
/// the one whose absence removes a whole part rather than a colour.
fn check_relationship_targets(package: &Package, prefix: &str, audit: &mut ReferenceAudit) {
    let present: BTreeSet<String> = package
        .part_names()
        .map(|name| name.as_str().to_owned())
        .collect();
    let sources: Vec<Option<PartName>> = std::iter::once(None)
        .chain(package.part_names().map(Some))
        .collect();
    for source in sources {
        let Some(relationships) = package.relationships_for(source.as_ref()) else {
            continue;
        };
        let from = source
            .as_ref()
            .map_or_else(|| "/".to_owned(), |part| part.as_str().to_owned());
        for relationship in relationships.iter() {
            if relationship.mode == TargetMode::External {
                continue;
            }
            let resolved = match source.as_ref() {
                Some(part) => part.resolve(&relationship.target),
                None => PartName::resolve_from_root(&relationship.target),
            };
            let Ok(target) = resolved else {
                audit.dangling.push(DanglingReference {
                    part: format!("{prefix}{from}"),
                    site: format!("Relationship@Target ({})", relationship.id),
                    reference: relationship.target.clone(),
                    reason: "the target is not a part name this package could resolve".to_owned(),
                });
                continue;
            };
            if present.contains(target.as_str()) {
                audit.resolved += 1;
            } else {
                audit.dangling.push(DanglingReference {
                    part: format!("{prefix}{from}"),
                    site: format!("Relationship@Target ({})", relationship.id),
                    reference: relationship.target.clone(),
                    reason: format!("no part {} in the package", target.as_str()),
                });
            }
        }
    }
}

// -------------------------------------------------------------------------------------------
// What the package offers a reference to resolve against
// -------------------------------------------------------------------------------------------

/// The colour-scheme slot names ECMA-376 gives a theme, in the order a SpreadsheetML `@theme`
/// index counts them (§18.8.3: the index is into `a:clrScheme` as written, and every producer
/// writes the twelve in schema order).
const THEME_SLOTS: [&str; 12] = [
    "dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6",
    "hlink", "folHlink",
];

/// Everything a reference in this package can resolve against.
#[derive(Debug, Default)]
struct PackageFacts {
    /// The theme, if the package carries one.
    theme: Option<ThemeFacts>,
    /// `w:style@w:styleId` values, or `None` when the package carries no `styles.xml` at all — a
    /// distinction that matters, because "no styles part" and "a styles part without that id" are
    /// different sentences to print.
    word_style_ids: Option<BTreeSet<String>>,
    /// `w:num@w:numId` values, or `None` when there is no numbering part.
    word_numbering_ids: Option<BTreeSet<String>>,
    /// `a:tblStyle@styleId` values, or `None` when there is no table-styles part.
    table_style_ids: Option<BTreeSet<String>>,
    /// How many records each `styles.xml` table holds, keyed by element name, or `None` when there
    /// is no SpreadsheetML styles part.
    sheet_style_counts: Option<BTreeMap<String, usize>>,
    /// The custom `numFmt@numFmtId` values the SpreadsheetML styles part defines.
    sheet_number_format_ids: BTreeSet<u32>,
}

/// What a theme offers.
#[derive(Debug, Default)]
struct ThemeFacts {
    /// The `a:clrScheme` slots it defines, by local name.
    color_slots: BTreeSet<String>,
    /// Whether `a:fontScheme > a:majorFont` is there.
    has_major_font: bool,
    /// Whether `a:fontScheme > a:minorFont` is there.
    has_minor_font: bool,
}

impl PackageFacts {
    /// Reads every part a reference can resolve against, once, before the walk.
    fn gather(package: &mut Package, prefix: &str, audit: &mut ReferenceAudit) -> Self {
        let mut facts = Self::default();
        let parts: Vec<PartName> = package.part_names().collect();
        for part in parts {
            let Some(content_type) = package.content_type_of(&part).map(str::to_owned) else {
                continue;
            };
            let known = matches!(
                content_type.as_str(),
                CONTENT_TYPE_THEME
                    | CONTENT_TYPE_WORD_STYLES
                    | CONTENT_TYPE_WORD_NUMBERING
                    | CONTENT_TYPE_TABLE_STYLES
                    | CONTENT_TYPE_SHEET_STYLES
            );
            if !known {
                continue;
            }
            let document = match package.part_tree(&part) {
                Ok(document) => document,
                Err(e) => {
                    audit
                        .errors
                        .push(format!("{prefix}{}: parsing: {e}", part.as_str()));
                    continue;
                }
            };
            let interner = &document.interner;
            let root = &document.root;
            match content_type.as_str() {
                CONTENT_TYPE_THEME => facts.theme = Some(ThemeFacts::read(root, interner)),
                CONTENT_TYPE_WORD_STYLES => {
                    facts.word_style_ids =
                        Some(attribute_values(root, interner, "style", "styleId"));
                }
                CONTENT_TYPE_WORD_NUMBERING => {
                    facts.word_numbering_ids =
                        Some(attribute_values(root, interner, "num", "numId"));
                }
                CONTENT_TYPE_TABLE_STYLES => {
                    facts.table_style_ids =
                        Some(attribute_values(root, interner, "tblStyle", "styleId"));
                }
                CONTENT_TYPE_SHEET_STYLES => {
                    facts.sheet_style_counts = Some(table_counts(root, interner));
                    facts.sheet_number_format_ids =
                        attribute_values(root, interner, "numFmt", "numFmtId")
                            .iter()
                            .filter_map(|value| value.parse::<u32>().ok())
                            .collect();
                }
                _ => {}
            }
        }
        facts
    }
}

impl ThemeFacts {
    /// Reads the two things a reference can name in a theme: the colour slots and the two font
    /// collections.
    fn read(root: &RawElement, interner: &Interner) -> Self {
        let mut facts = Self::default();
        let Some(elements) = child_named(root, interner, "themeElements") else {
            return facts;
        };
        if let Some(scheme) = child_named(elements, interner, "clrScheme") {
            for node in &scheme.children {
                if let RawNode::Element(child) = node {
                    facts
                        .color_slots
                        .insert(interner.resolve(child.name.local).to_owned());
                }
            }
        }
        if let Some(fonts) = child_named(elements, interner, "fontScheme") {
            facts.has_major_font = child_named(fonts, interner, "majorFont").is_some();
            facts.has_minor_font = child_named(fonts, interner, "minorFont").is_some();
        }
        facts
    }

    /// Whether the theme defines `slot`, following the `bg1`/`tx1` colour-map spellings back to the
    /// `dk1`/`lt1` pair a `p:clrMap` maps them onto.
    ///
    /// The mapping direction is a slide master's choice, so both members of each pair are required
    /// rather than guessed at: a theme that defines only one of `dk1`/`lt1` cannot answer `tx1`
    /// under every colour map.
    fn defines_color(&self, slot: &str) -> bool {
        match slot {
            "bg1" | "tx1" | "bg2" | "tx2" => {
                let pair: [&str; 2] = if slot.ends_with('1') {
                    ["dk1", "lt1"]
                } else {
                    ["dk2", "lt2"]
                };
                pair.iter().all(|name| self.color_slots.contains(*name))
            }
            // WordprocessingML spells the same four slots out in full.
            "text1" | "background1" => ["dk1", "lt1"]
                .iter()
                .all(|name| self.color_slots.contains(*name)),
            "text2" | "background2" => ["dk2", "lt2"]
                .iter()
                .all(|name| self.color_slots.contains(*name)),
            "dark1" | "light1" => ["dk1", "lt1"]
                .iter()
                .all(|name| self.color_slots.contains(*name)),
            "dark2" | "light2" => ["dk2", "lt2"]
                .iter()
                .all(|name| self.color_slots.contains(*name)),
            "hyperlink" => self.color_slots.contains("hlink"),
            "followedHyperlink" => self.color_slots.contains("folHlink"),
            other => self.color_slots.contains(other),
        }
    }

    /// Whether the theme defines the colour slot at `index`, as a SpreadsheetML `@theme` counts.
    fn defines_color_at(&self, index: usize) -> bool {
        THEME_SLOTS
            .get(index)
            .is_some_and(|slot| self.color_slots.contains(*slot))
    }
}

/// The first child of `element` with this local name, at any prefix.
fn child_named<'a>(
    element: &'a RawElement,
    interner: &Interner,
    local: &str,
) -> Option<&'a RawElement> {
    element.children.iter().find_map(|node| match node {
        RawNode::Element(child) if interner.resolve(child.name.local) == local => Some(child),
        _ => None,
    })
}

/// Every `attribute` on every descendant element named `local`, as strings.
fn attribute_values(
    root: &RawElement,
    interner: &Interner,
    local: &str,
    attribute: &str,
) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    collect_attribute_values(root, interner, local, attribute, &mut found);
    found
}

fn collect_attribute_values(
    element: &RawElement,
    interner: &Interner,
    local: &str,
    attribute: &str,
    found: &mut BTreeSet<String>,
) {
    if interner.resolve(element.name.local) == local {
        if let Some(value) = attribute_named(element, interner, attribute) {
            found.insert(value);
        }
    }
    for node in &element.children {
        if let RawNode::Element(child) = node {
            collect_attribute_values(child, interner, local, attribute, found);
        }
    }
}

/// The value of `element`'s attribute with this local name, at any prefix and unescaped enough for
/// comparison (no entity in an id, a slot name or an index needs decoding).
fn attribute_named(element: &RawElement, interner: &Interner, local: &str) -> Option<String> {
    element
        .attributes
        .iter()
        .find(|attribute| {
            interner.resolve(attribute.name.local) == local
                && interner.resolve(attribute.name.prefix.unwrap_or(attribute.name.local))
                    != "xmlns"
        })
        .map(|attribute| String::from_utf8_lossy(&attribute.value).into_owned())
}

/// How many records each of `styles.xml`'s tables holds, keyed by the table's element name.
fn table_counts(root: &RawElement, interner: &Interner) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for node in &root.children {
        let RawNode::Element(table) = node else {
            continue;
        };
        let name = interner.resolve(table.name.local).to_owned();
        let records = table
            .children
            .iter()
            .filter(|child| matches!(child, RawNode::Element(_)))
            .count();
        counts.insert(name, records);
    }
    counts
}

// -------------------------------------------------------------------------------------------
// Walking one part's markup
// -------------------------------------------------------------------------------------------

/// The walk over one part, carrying everything a check needs.
struct PartWalker<'a> {
    part: &'a str,
    interner: &'a Interner,
    facts: &'a PackageFacts,
    relationship_ids: &'a BTreeSet<String>,
    audit: &'a mut ReferenceAudit,
    /// The in-scope `xmlns` bindings, innermost last, as `(prefix, uri)` with `None` for the
    /// default namespace.
    ///
    /// **This is not redundant with [`RawName::namespace`].** The fidelity reader resolves a
    /// namespace for an *element* name and leaves an *attribute*'s `namespace` `None` — attributes
    /// are not in the default namespace, so the resolution is per-prefix and the reader does not do
    /// it. Matching `r:embed` on `attribute.name.namespace` therefore matches nothing at all, which
    /// is a silently empty check rather than a failing one: the rule was written that way first and
    /// the paired case in `tests/reference_resolution.rs` is what caught it.
    ///
    /// [`RawName::namespace`]: mjx_ooxml_core::RawName::namespace
    namespace_scope: Vec<(Option<String>, String)>,
    /// How many `c:ser` elements with no `c:spPr` this part has already reached, as the fallback
    /// ordering for one that states neither `c:order` nor `c:idx`.
    implicit_series_count: usize,
}

impl PartWalker<'_> {
    /// Checks this element and descends into its children.
    fn element(&mut self, element: &RawElement) {
        let declared = self.push_namespace_declarations(element);
        let namespace = element
            .name
            .namespace
            .map(|symbol| self.interner.resolve(symbol).to_owned());
        let local = self.interner.resolve(element.name.local).to_owned();
        let prefixed = self.qualified(element);

        // Relationship ids are checked in every namespace, preserved markup included: a `.rels`
        // entry is packaging rather than markup, and a VML drawing's `o:relid` still has to name a
        // real relationship.
        self.check_relationship_attributes(element, &prefixed);

        match namespace.as_deref() {
            Some(ns) if ns == namespaces::DML_MAIN.transitional => {
                self.check_drawing_markup(element, &local, &prefixed);
            }
            Some(ns) if ns == namespaces::DML_CHART.transitional => {
                self.check_chart_markup(element, &local, &prefixed);
            }
            Some(ns) if ns == namespaces::WML.transitional => {
                self.check_word_markup(element, &local, &prefixed);
            }
            Some(ns) if ns == namespaces::SML.transitional => {
                self.check_sheet_markup(element, &local, &prefixed);
            }
            _ => {}
        }

        for node in &element.children {
            if let RawNode::Element(child) = node {
                self.element(child);
            }
        }
        self.namespace_scope
            .truncate(self.namespace_scope.len() - declared);
    }

    /// Pushes every `xmlns` declaration this element carries and answers how many, so the caller can
    /// pop exactly those on the way out.
    fn push_namespace_declarations(&mut self, element: &RawElement) -> usize {
        let mut pushed = 0;
        for attribute in &element.attributes {
            let local = self.interner.resolve(attribute.name.local);
            let prefix = attribute
                .name
                .prefix
                .map(|symbol| self.interner.resolve(symbol));
            let binding = match prefix {
                Some("xmlns") => Some(local.to_owned()),
                None if local == "xmlns" => None,
                _ => continue,
            };
            self.namespace_scope.push((
                binding,
                String::from_utf8_lossy(&attribute.value).into_owned(),
            ));
            pushed += 1;
        }
        pushed
    }

    /// The namespace URI an attribute prefix is bound to here, or `None` for an unprefixed
    /// attribute (which is in **no** namespace, never the default one).
    fn namespace_of_attribute_prefix(&self, prefix: Option<&str>) -> Option<&str> {
        let prefix = prefix?;
        self.namespace_scope
            .iter()
            .rev()
            .find(|(bound, _)| bound.as_deref() == Some(prefix))
            .map(|(_, uri)| uri.as_str())
    }

    /// `prefix:local`, as the file spells it.
    fn qualified(&self, element: &RawElement) -> String {
        match element.name.prefix {
            Some(prefix) => format!(
                "{}:{}",
                self.interner.resolve(prefix),
                self.interner.resolve(element.name.local)
            ),
            None => self.interner.resolve(element.name.local).to_owned(),
        }
    }

    /// Every attribute in the relationship-reference namespace names a relationship of this part.
    ///
    /// An **empty** value is skipped rather than reported: `r:link=""` and `r:embed=""` are how
    /// several schemas spell "explicitly none", and reading one as a dangling id would make the gate
    /// wrong about correct files.
    fn check_relationship_attributes(&mut self, element: &RawElement, site: &str) {
        for attribute in &element.attributes {
            let prefix = attribute
                .name
                .prefix
                .map(|symbol| self.interner.resolve(symbol));
            let namespace = self.namespace_of_attribute_prefix(prefix);
            if namespace != Some(namespaces::SHARED_RELATIONSHIP_REFERENCE.transitional) {
                continue;
            }
            let value = String::from_utf8_lossy(&attribute.value).into_owned();
            if value.is_empty() {
                continue;
            }
            let local = self.interner.resolve(attribute.name.local);
            let site = format!("{site}@r:{local}");
            if self.relationship_ids.contains(&value) {
                self.audit.resolved += 1;
            } else {
                self.report(site, value, "no relationship with that id on this part");
            }
        }
    }

    /// DrawingML: scheme colours, theme fonts and a table's style id.
    fn check_drawing_markup(&mut self, element: &RawElement, local: &str, site: &str) {
        if local == "schemeClr" {
            if let Some(value) = attribute_named(element, self.interner, "val") {
                // `phClr` is the placeholder a style matrix substitutes, never a slot.
                if value != "phClr" {
                    self.check_theme_color(&format!("{site}@val"), &value);
                }
            }
        }
        // `a:latin` / `a:ea` / `a:cs` / `a:sym`, wherever they appear.
        if matches!(local, "latin" | "ea" | "cs" | "sym") {
            if let Some(typeface) = attribute_named(element, self.interner, "typeface") {
                if let Some(reference) = typeface.strip_prefix('+') {
                    let major = reference.starts_with("mj-");
                    self.check_theme_font(&format!("{site}@typeface"), &typeface, major);
                }
            }
        }
        if local == "tableStyleId" {
            let value = text_of(element);
            if value.is_empty() {
                return;
            }
            match &self.facts.table_style_ids {
                Some(ids) if ids.contains(&value) => self.audit.resolved += 1,
                Some(_) => self.report(
                    site.to_owned(),
                    value,
                    "no a:tblStyle with that @styleId in the table-styles part",
                ),
                None => self.report(
                    site.to_owned(),
                    value,
                    "the package carries no table-styles part for it to resolve against",
                ),
            }
        }
    }

    /// DrawingML charts: the **implicit** scheme colour a series with no `c:spPr` takes its fill
    /// from.
    ///
    /// This is the rule MJXOFF-200 exists for, and it is the one that cannot be found by looking for
    /// a reference in the markup — because there is no reference in the markup. A `c:ser` that
    /// states no shape properties is not under-specified; it is *deferring*, and what it defers to
    /// is the theme's `accent1…accent6`, cycled by series order (ECMA-376 Part 1 §21.2.2.170's
    /// `c:ser` takes its formatting from the chart style when it states none, and every consumer
    /// resolves that through the document's theme). Every series this library authors is that shape,
    /// deliberately, so that the host document's brand wins.
    ///
    /// A chart with no `c:spPr` in a package with no theme is therefore a chart with **no bars**,
    /// and nothing textual in the file says so. Taking the theme back out of a document we authored
    /// reddens exactly here — see `crates/mjx-docx/tests/reference_resolution.rs`.
    fn check_chart_markup(&mut self, element: &RawElement, local: &str, site: &str) {
        if local != "ser" {
            return;
        }
        if element.children.iter().any(|node| match node {
            RawNode::Element(child) => self.interner.resolve(child.name.local) == "spPr",
            _ => false,
        }) {
            // The series states its own fill, so it defers to nothing.
            return;
        }
        let order = self
            .chart_series_index(element)
            .unwrap_or(self.implicit_series_count);
        self.implicit_series_count += 1;
        let slot = format!("accent{}", (order % 6) + 1);
        self.check_theme_color(
            &format!("{site} (no c:spPr, so its fill is the theme's)"),
            &slot,
        );
    }

    /// The series' own `c:order@val`, falling back to `c:idx@val`.
    fn chart_series_index(&self, element: &RawElement) -> Option<usize> {
        for name in ["order", "idx"] {
            let value = element.children.iter().find_map(|node| match node {
                RawNode::Element(child) if self.interner.resolve(child.name.local) == name => {
                    attribute_named(child, self.interner, "val")
                }
                _ => None,
            });
            if let Some(value) = value.and_then(|value| value.parse::<usize>().ok()) {
                return Some(value);
            }
        }
        None
    }

    /// WordprocessingML: theme colours, theme fonts, style ids and numbering ids.
    fn check_word_markup(&mut self, element: &RawElement, local: &str, site: &str) {
        for attribute in ["themeColor", "themeFill"] {
            if let Some(value) = attribute_named(element, self.interner, attribute) {
                if value != "none" {
                    self.check_theme_color(&format!("{site}@w:{attribute}"), &value);
                }
            }
        }
        if local == "rFonts" {
            for attribute in ["asciiTheme", "hAnsiTheme", "eastAsiaTheme", "cstheme"] {
                if let Some(value) = attribute_named(element, self.interner, attribute) {
                    let major = value.starts_with("major");
                    self.check_theme_font(&format!("{site}@w:{attribute}"), &value, major);
                }
            }
        }
        // Every place a WordprocessingML style id is *referenced*. `w:style@w:styleId` itself
        // declares one and is not in the list.
        let style_reference = match local {
            "pStyle" | "rStyle" | "tblStyle" | "basedOn" | "next" | "link" => {
                attribute_named(element, self.interner, "val")
            }
            _ => None,
        };
        if let Some(value) = style_reference {
            match &self.facts.word_style_ids {
                Some(ids) if ids.contains(&value) => self.audit.resolved += 1,
                Some(_) => self.report(
                    format!("{site}@w:val"),
                    value,
                    "no w:style with that @w:styleId in styles.xml",
                ),
                None => self.report(
                    format!("{site}@w:val"),
                    value,
                    "the package carries no styles.xml for it to resolve against",
                ),
            }
        }
        if local == "numId" {
            // `w:numId` appears both under `w:numPr` (a reference) and as `w:num@w:numId` (a
            // declaration); only the element form is a reference.
            if let Some(value) = attribute_named(element, self.interner, "val") {
                // `0` is the documented "no numbering" value (§17.9.18) and names no `w:num`.
                if value != "0" {
                    match &self.facts.word_numbering_ids {
                        Some(ids) if ids.contains(&value) => self.audit.resolved += 1,
                        Some(_) => self.report(
                            format!("{site}@w:val"),
                            value,
                            "no w:num with that @w:numId in numbering.xml",
                        ),
                        None => self.report(
                            format!("{site}@w:val"),
                            value,
                            "the package carries no numbering.xml for it to resolve against",
                        ),
                    }
                }
            }
        }
    }

    /// SpreadsheetML: theme colour indices, the theme font scheme and the style-table indices.
    fn check_sheet_markup(&mut self, element: &RawElement, local: &str, site: &str) {
        if matches!(local, "color" | "fgColor" | "bgColor" | "tabColor") {
            if let Some(value) = attribute_named(element, self.interner, "theme") {
                self.check_theme_color_index(&format!("{site}@theme"), &value);
            }
        }
        if local == "scheme" {
            if let Some(value) = attribute_named(element, self.interner, "val") {
                if value != "none" {
                    self.check_theme_font(&format!("{site}@val"), &value, value == "major");
                }
            }
        }
        let index_references: &[(&str, &str)] = match local {
            "c" | "row" => &[("s", "cellXfs")],
            "col" => &[("style", "cellXfs")],
            "xf" => &[
                ("fontId", "fonts"),
                ("fillId", "fills"),
                ("borderId", "borders"),
                ("xfId", "cellStyleXfs"),
            ],
            "cellStyle" => &[("xfId", "cellStyleXfs")],
            _ => &[],
        };
        for (attribute, table) in index_references {
            if let Some(value) = attribute_named(element, self.interner, attribute) {
                self.check_style_index(&format!("{site}@{attribute}"), &value, table);
            }
        }
        if local == "xf" || local == "cellStyle" {
            if let Some(value) = attribute_named(element, self.interner, "numFmtId") {
                self.check_number_format(&format!("{site}@numFmtId"), &value);
            }
        }
    }

    /// A theme colour slot named by name.
    fn check_theme_color(&mut self, site: &str, slot: &str) {
        match &self.facts.theme {
            Some(theme) if theme.defines_color(slot) => self.audit.resolved += 1,
            Some(_) => self.report(
                site.to_owned(),
                slot.to_owned(),
                "the theme's a:clrScheme defines no such slot",
            ),
            None => self.report(
                site.to_owned(),
                slot.to_owned(),
                "the package carries no theme part, so this scheme colour resolves to nothing \
                 and whatever it paints is painted with no colour",
            ),
        }
    }

    /// A theme colour slot named by index, as SpreadsheetML names one.
    fn check_theme_color_index(&mut self, site: &str, value: &str) {
        let Ok(index) = value.parse::<usize>() else {
            self.report(
                site.to_owned(),
                value.to_owned(),
                "not an integer, so it names no colour-scheme slot",
            );
            return;
        };
        match &self.facts.theme {
            Some(theme) if theme.defines_color_at(index) => self.audit.resolved += 1,
            Some(_) => self.report(
                site.to_owned(),
                value.to_owned(),
                "the theme's a:clrScheme has no slot at that index",
            ),
            None => self.report(
                site.to_owned(),
                value.to_owned(),
                "the package carries no theme part, so this colour resolves to nothing",
            ),
        }
    }

    /// A theme font collection, named the DrawingML way (`+mj-lt`), the Word way (`minorHAnsi`) or
    /// the Excel way (`minor`).
    fn check_theme_font(&mut self, site: &str, value: &str, major: bool) {
        let present = self.facts.theme.as_ref().map(|theme| {
            if major {
                theme.has_major_font
            } else {
                theme.has_minor_font
            }
        });
        match present {
            Some(true) => self.audit.resolved += 1,
            Some(false) => self.report(
                site.to_owned(),
                value.to_owned(),
                if major {
                    "the theme's a:fontScheme has no a:majorFont"
                } else {
                    "the theme's a:fontScheme has no a:minorFont"
                },
            ),
            None => self.report(
                site.to_owned(),
                value.to_owned(),
                "the package carries no theme part, so this font reference resolves to nothing",
            ),
        }
    }

    /// An index into one of `styles.xml`'s tables.
    fn check_style_index(&mut self, site: &str, value: &str, table: &str) {
        let Ok(index) = value.parse::<usize>() else {
            self.report(
                site.to_owned(),
                value.to_owned(),
                format!("not an integer, so it names no {table} record"),
            );
            return;
        };
        match &self.facts.sheet_style_counts {
            Some(counts) if index < counts.get(table).copied().unwrap_or(0) => {
                self.audit.resolved += 1;
            }
            Some(counts) => self.report(
                site.to_owned(),
                value.to_owned(),
                format!(
                    "styles.xml's {table} holds {} record(s), so index {index} names none",
                    counts.get(table).copied().unwrap_or(0)
                ),
            ),
            None => self.report(
                site.to_owned(),
                value.to_owned(),
                "the package carries no styles.xml for it to index into",
            ),
        }
    }

    /// A number-format id. Anything below [`FIRST_CUSTOM_NUMBER_FORMAT_ID`] is built in and needs no
    /// record.
    fn check_number_format(&mut self, site: &str, value: &str) {
        let Ok(id) = value.parse::<u32>() else {
            self.report(
                site.to_owned(),
                value.to_owned(),
                "not an integer, so it names no number format",
            );
            return;
        };
        if id < FIRST_CUSTOM_NUMBER_FORMAT_ID {
            self.audit.resolved += 1;
            return;
        }
        if self.facts.sheet_number_format_ids.contains(&id) {
            self.audit.resolved += 1;
        } else {
            self.report(
                site.to_owned(),
                value.to_owned(),
                "no numFmt with that @numFmtId in styles.xml, and it is past the built-in range",
            );
        }
    }

    /// Records one dangling reference.
    fn report(&mut self, site: String, reference: String, reason: impl Into<String>) {
        self.audit.dangling.push(DanglingReference {
            part: self.part.to_owned(),
            site,
            reference,
            reason: reason.into(),
        });
    }
}

/// The concatenated text of an element's direct text children.
fn text_of(element: &RawElement) -> String {
    element
        .children
        .iter()
        .filter_map(|node| match node {
            RawNode::Text(text) => Some(String::from_utf8_lossy(text).into_owned()),
            _ => None,
        })
        .collect::<String>()
        .trim()
        .to_owned()
}
