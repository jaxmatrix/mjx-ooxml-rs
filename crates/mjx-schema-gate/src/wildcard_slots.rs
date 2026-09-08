//! Elements ECMA-376 declares as holding **nothing but** foreign markup.
//!
//! A *wildcard slot* is an element whose complex type has one content model, that content model is
//! built out of `xsd:any` particles and nothing else, and it cannot match the empty sequence. The
//! canonical one is `CT_Extension`:
//!
//! ```xml
//! <xsd:complexType name="CT_Extension">
//!   <xsd:sequence>
//!     <xsd:any processContents="lax"/>   <!-- no minOccurs, so minOccurs="1" -->
//!   </xsd:sequence>
//!   <xsd:attribute name="uri" type="xsd:token"/>
//! </xsd:complexType>
//! ```
//!
//! Such an element exists **only** to carry the one foreign child inside it. That is what makes it
//! the exact shape markup-compatibility resolution breaks, and it is why [`crate::inspect`] drops a
//! slot resolution emptied instead of handing the validator a hole. See that module's
//! *Markup compatibility is resolved* section for the decision and what it gives up.
//!
//! # The table is checked against the schemas, not written from a ticket
//!
//! [`WILDCARD_SLOTS`] is a constant because the gate's non-schema half runs on a machine with no
//! `References/` tree. But it is not a hand-maintained list: [`derive_wildcard_slots`] reads the
//! pinned XSDs and computes the same set, and
//! `every_wildcard_slot_in_the_reference_schemas_is_listed` asserts the two are equal. Add a schema
//! to the gate, or move to a different edition of the reference tree, and that test names what
//! changed. This is the answer to the narrow shape MJXOFF-196 offered as its third option — a
//! hand-written exemption for two named types "does not generalise to the next type with the same
//! shape". A derivation does, and it found **five** where the ticket named two and
//! `xtask/src/validation/ingest.rs` had found a third.
//!
//! # What the derivation is deliberately conservative about
//!
//! A type whose content model reaches a named `xsd:group`, or which derives by `xsd:extension` /
//! `xsd:restriction`, is **not** a slot as far as this module is concerned, because the particle it
//! inherits is not analysed here. Being conservative in that direction can only leave a hole
//! unfilled — reported as a schema failure, which is loud — never quietly drop an element that
//! carried something. Nothing in the Transitional or OPC trees has that shape today; the test above
//! is what will say so if it ever does.

use std::collections::BTreeSet;
use std::path::Path;

use mjx_mce::NamespaceScope;
use mjx_ooxml_core::{Interner, RawElement, RawNode};

/// The XML Schema namespace, the one every declaration below is read out of.
const XML_SCHEMA_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema";

/// One element the schema lets hold nothing but a foreign child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WildcardSlot {
    /// The element's namespace URI.
    pub namespace: &'static str,
    /// The element's local name, as written in the schema.
    pub local_name: &'static str,
    /// The `CT_*` symbol whose content model makes it a slot.
    pub complex_type: &'static str,
    /// The XSD it is declared in.
    pub schema_file: &'static str,
}

/// Every wildcard slot in the two pinned reference trees.
///
/// Kept sorted by (namespace, local name) so a reader can diff it against
/// [`derive_wildcard_slots`]'s output by eye as well as by test.
pub const WILDCARD_SLOTS: &[WildcardSlot] = &[
    WildcardSlot {
        namespace: "http://schemas.openxmlformats.org/drawingml/2006/chart",
        local_name: "ext",
        complex_type: "CT_Extension",
        schema_file: "dml-chart.xsd",
    },
    WildcardSlot {
        namespace: "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
        local_name: "DataBinding",
        complex_type: "CT_DataBinding",
        schema_file: "sml.xsd",
    },
    WildcardSlot {
        namespace: "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
        local_name: "Schema",
        complex_type: "CT_Schema",
        schema_file: "sml.xsd",
    },
    WildcardSlot {
        namespace: "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
        local_name: "ext",
        complex_type: "CT_Extension",
        schema_file: "sml.xsd",
    },
    WildcardSlot {
        namespace: "urn:schemas-microsoft-com:office:office",
        local_name: "equationxml",
        complex_type: "CT_EquationXml",
        schema_file: "vml-officeDrawing.xsd",
    },
];

/// Whether an element is a wildcard slot.
///
/// An element in no namespace is never one: every slot in [`WILDCARD_SLOTS`] is declared in a
/// schema whose `elementFormDefault` is `qualified`.
#[must_use]
pub fn is_wildcard_slot(namespace: Option<&str>, local_name: &str) -> bool {
    let Some(namespace) = namespace else {
        return false;
    };
    WILDCARD_SLOTS
        .iter()
        .any(|slot| slot.namespace == namespace && slot.local_name == local_name)
}

/// One wildcard slot as the derivation found it, owned so it can be compared and sorted.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DerivedWildcardSlot {
    /// The element's namespace URI.
    pub namespace: String,
    /// The element's local name.
    pub local_name: String,
    /// The `CT_*` symbol whose content model makes it a slot.
    pub complex_type: String,
    /// The XSD it is declared in.
    pub schema_file: String,
}

impl From<&WildcardSlot> for DerivedWildcardSlot {
    fn from(slot: &WildcardSlot) -> Self {
        Self {
            namespace: slot.namespace.to_owned(),
            local_name: slot.local_name.to_owned(),
            complex_type: slot.complex_type.to_owned(),
            schema_file: slot.schema_file.to_owned(),
        }
    }
}

/// Reads every `.xsd` in `directories` and returns the wildcard slots they declare, sorted.
///
/// Every XSD in the trees is read, not only those [`crate::categories::MODELED_SCHEMAS`] names.
/// That is a superset by construction, and a superset is the safe direction: an entry for an
/// element no validated part can contain simply never fires.
///
/// # Panics
/// If a directory cannot be read or an XSD does not parse. Both are harness faults — the schema
/// trees are pinned and verified by `.github/scripts/fetch-ecma-schemas.sh` — rather than anything
/// a fixture could cause.
#[must_use]
pub fn derive_wildcard_slots(directories: &[&Path]) -> Vec<DerivedWildcardSlot> {
    let mut types = BTreeSet::new();
    let mut declarations = Vec::new();
    for directory in directories {
        let mut files: Vec<_> = std::fs::read_dir(directory)
            .unwrap_or_else(|e| panic!("reading {}: {e}", directory.display()))
            .map(|entry| {
                entry
                    .unwrap_or_else(|e| panic!("reading {}: {e}", directory.display()))
                    .path()
            })
            .filter(|path| path.extension().is_some_and(|ext| ext == "xsd"))
            .collect();
        files.sort();
        for file in files {
            scan_schema(&file, &mut types, &mut declarations);
        }
    }

    let mut slots: Vec<DerivedWildcardSlot> = declarations
        .into_iter()
        .filter_map(|declaration| {
            let key = (
                declaration.type_namespace.clone(),
                declaration.type_name.clone(),
            );
            types.contains(&key).then_some(DerivedWildcardSlot {
                namespace: declaration.namespace,
                local_name: declaration.local_name,
                complex_type: declaration.type_name,
                schema_file: declaration.schema_file,
            })
        })
        .collect();
    slots.sort();
    slots.dedup();
    slots
}

/// One `xsd:element name="…" type="…"` declaration, with its type's QName already resolved.
struct ElementDeclaration {
    namespace: String,
    local_name: String,
    type_namespace: String,
    type_name: String,
    schema_file: String,
}

/// Reads one XSD, recording its wildcard-slot complex types and every element declared with a type.
fn scan_schema(
    path: &Path,
    types: &mut BTreeSet<(String, String)>,
    declarations: &mut Vec<ElementDeclaration>,
) {
    let source = std::fs::read(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let document = mjx_xml::fidelity::parse(&source)
        .unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()));
    let interner = &document.interner;
    let root = &document.root;
    let file = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let target_namespace = attribute(root, interner, "targetNamespace")
        .unwrap_or_default()
        .to_owned();
    // `elementFormDefault` decides whether a *local* element declaration is in the target namespace.
    // Every ECMA-376 schema says `qualified`; reading it rather than assuming it is what keeps this
    // honest if one ever does not.
    let qualified_by_default =
        attribute(root, interner, "elementFormDefault").is_some_and(|value| value == "qualified");

    let mut scope = NamespaceScope::new();
    walk_schema(
        root,
        interner,
        &mut scope,
        &target_namespace,
        qualified_by_default,
        &file,
        types,
        declarations,
    );
}

#[allow(clippy::too_many_arguments)]
fn walk_schema(
    element: &RawElement,
    interner: &Interner,
    scope: &mut NamespaceScope,
    target_namespace: &str,
    qualified_by_default: bool,
    file: &str,
    types: &mut BTreeSet<(String, String)>,
    declarations: &mut Vec<ElementDeclaration>,
) {
    scope.push_element(element, interner);
    match schema_local_name(element, interner) {
        Some("complexType") => {
            if let Some(name) = attribute(element, interner, "name") {
                if holds_nothing_but_a_required_wildcard(element, interner) {
                    types.insert((target_namespace.to_owned(), name.to_owned()));
                }
            }
        }
        Some("element") => {
            if let (Some(name), Some(type_name)) = (
                attribute(element, interner, "name"),
                attribute(element, interner, "type"),
            ) {
                let qualified = match attribute(element, interner, "form") {
                    Some(form) => form == "qualified",
                    None => qualified_by_default,
                };
                let (prefix, local) = match type_name.split_once(':') {
                    Some((prefix, local)) => (Some(prefix), local),
                    None => (None, type_name),
                };
                let type_namespace = match prefix {
                    Some(prefix) => scope.resolve_prefix(prefix).unwrap_or_default(),
                    // An unprefixed type name is in the *default* namespace, which in every
                    // ECMA-376 schema is the XML Schema namespace — but the type it names is the
                    // schema's own, so the target namespace is what a lookup must use.
                    None => target_namespace,
                };
                declarations.push(ElementDeclaration {
                    namespace: if qualified {
                        target_namespace.to_owned()
                    } else {
                        String::new()
                    },
                    local_name: name.to_owned(),
                    type_namespace: type_namespace.to_owned(),
                    type_name: local.to_owned(),
                    schema_file: file.to_owned(),
                });
            }
        }
        _ => {}
    }
    for child in element_children(element) {
        walk_schema(
            child,
            interner,
            scope,
            target_namespace,
            qualified_by_default,
            file,
            types,
            declarations,
        );
    }
    scope.pop();
}

/// Whether a complex type's content model is built of wildcards alone and rejects empty content.
///
/// Both halves matter. *Wildcards alone* is what makes dropping an emptied instance safe: a type
/// that also requires a named child could be emptied by an author forgetting that child, and
/// dropping the element would hide it. *Rejects empty content* is what makes the question arise at
/// all — `v:textbox`'s `CT_Textbox` is a choice one of whose branches is `minOccurs="0"`, so an
/// empty `v:textbox` validates and there is nothing to fix.
fn holds_nothing_but_a_required_wildcard(complex_type: &RawElement, interner: &Interner) -> bool {
    let mut model_group = None;
    for child in element_children(complex_type) {
        match schema_local_name(child, interner) {
            // Derivation: the inherited particle is not analysed here, so the type is not a slot.
            Some("simpleContent" | "complexContent") => return false,
            Some("sequence" | "choice" | "all") => model_group = Some(child),
            _ => {}
        }
    }
    let Some(model_group) = model_group else {
        // No content model at all: the type is attribute-only and empty content is what it wants.
        return false;
    };
    !matches_the_empty_sequence(model_group, interner) && is_wildcards_only(model_group, interner)
}

/// Whether a particle can match no elements at all.
fn matches_the_empty_sequence(particle: &RawElement, interner: &Interner) -> bool {
    if minimum_occurrences(particle, interner) == 0 {
        return true;
    }
    match schema_local_name(particle, interner) {
        Some("sequence" | "all") => {
            particles(particle, interner).all(|child| matches_the_empty_sequence(child, interner))
        }
        Some("choice") => {
            particles(particle, interner).any(|child| matches_the_empty_sequence(child, interner))
        }
        // `any`, `element`, a named `group` reference, anything unrecognised: assumed to consume
        // something. Only ever read next to `is_wildcards_only`, which refuses everything but the
        // model groups and `any`.
        _ => false,
    }
}

/// Whether every particle in a content model is an `xsd:any`.
fn is_wildcards_only(particle: &RawElement, interner: &Interner) -> bool {
    match schema_local_name(particle, interner) {
        Some("any") => true,
        Some("sequence" | "choice" | "all") => {
            particles(particle, interner).all(|child| is_wildcards_only(child, interner))
        }
        _ => false,
    }
}

/// The particles of a model group: its element children in the XML Schema namespace, minus the
/// `xsd:annotation` that carries the specification's prose.
fn particles<'a>(
    group: &'a RawElement,
    interner: &'a Interner,
) -> impl Iterator<Item = &'a RawElement> {
    element_children(group)
        .filter(move |child| schema_local_name(child, interner) != Some("annotation"))
}

/// A particle's `minOccurs`, which defaults to 1 — the default this whole module is about.
fn minimum_occurrences(particle: &RawElement, interner: &Interner) -> u32 {
    attribute(particle, interner, "minOccurs")
        .and_then(|value| value.parse().ok())
        .unwrap_or(1)
}

/// An element's children that are elements.
fn element_children(element: &RawElement) -> impl Iterator<Item = &RawElement> {
    element.children.iter().filter_map(|child| match child {
        RawNode::Element(child) => Some(child),
        _ => None,
    })
}

/// An element's local name when it is in the XML Schema namespace, and `None` otherwise.
fn schema_local_name<'a>(element: &RawElement, interner: &'a Interner) -> Option<&'a str> {
    let namespace = element.name.namespace.map(|ns| interner.resolve(ns))?;
    (namespace == XML_SCHEMA_NAMESPACE).then(|| interner.resolve(element.name.local))
}

/// An unprefixed attribute's value. Every attribute this module reads is unprefixed in XSD.
fn attribute<'a>(element: &'a RawElement, interner: &'a Interner, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| {
            attribute.name.prefix.is_none() && interner.resolve(attribute.name.local) == name
        })
        .and_then(|attribute| std::str::from_utf8(&attribute.value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Six pages of this repository state the size of [`WILDCARD_SLOTS`] as a word, and a count in
    /// prose is a fact with an expiry date unless something derives it. This is that something: it
    /// runs on every machine, with or without `References/`, and its message is the list of files to
    /// edit rather than an instruction to go and find them.
    #[test]
    fn the_number_of_slots_this_repository_spells_out_is_the_number_there_are() {
        assert_eq!(
            WILDCARD_SLOTS.len(),
            5,
            "`WILDCARD_SLOTS` has changed size. Six places say **five** in prose and every one of \
             them is now wrong:\n  \
             crates/mjx-schema-gate/src/wildcard_slots.rs (this module's documentation)\n  \
             crates/mjx-schema-gate/src/tolerances.rs (the `xl/xmlMaps.xml` entry's reason)\n  \
             xtask/src/validation/ingest.rs (the module documentation)\n  \
             tests/office-authored/README.md\n  \
             docs/validation/06-the-office-pass.md §5\n  \
             CHANGELOG.md, 0.0.148 — a dated record, so leave that one alone"
        );
    }

    /// The table above is the schemas' answer, not a ticket's.
    ///
    /// MJXOFF-196 named two `CT_Extension` declarations; `xtask/src/validation/ingest.rs` found a
    /// third by hand and wrote "so that a fix cannot stop at two". This derivation finds more, and
    /// it will find the next one without anybody auditing anything.
    #[test]
    fn every_wildcard_slot_in_the_reference_schemas_is_listed() {
        let Some(harness) = crate::harness() else {
            println!(
                "skipped: no References/ tree or no xmllint. MJX_REQUIRE_SCHEMA=1 makes that a \
                 failure."
            );
            return;
        };
        let directories = harness.schema_directories();
        let derived = derive_wildcard_slots(&directories);
        let listed: Vec<DerivedWildcardSlot> = WILDCARD_SLOTS
            .iter()
            .map(DerivedWildcardSlot::from)
            .collect();
        assert_eq!(
            derived, listed,
            "the pinned schemas and `WILDCARD_SLOTS` disagree. Every element listed by the \
             derivation is one markup-compatibility resolution can empty into a hole the validator \
             rejects, so a slot missing from the table is a false failure and a slot in the table \
             that the schemas no longer declare is an element the gate would drop for no reason.\n\
             derived: {derived:#?}\nlisted:  {listed:#?}"
        );
        println!("wildcard slots derived from the pinned schemas:");
        for slot in &derived {
            println!(
                "  {{{}}}{}  {} ({})",
                slot.namespace, slot.local_name, slot.complex_type, slot.schema_file
            );
        }
    }

    /// The predicate itself, on the four shapes the reference trees actually contain, so a
    /// misreading of `minOccurs` is caught without `References/`.
    #[test]
    fn the_predicate_separates_a_required_wildcard_from_every_neighbouring_shape() {
        let cases: &[(&str, bool, &str)] = &[
            (
                r#"<xsd:complexType xmlns:xsd="http://www.w3.org/2001/XMLSchema" name="CT_Extension">
                     <xsd:sequence><xsd:any processContents="lax"/></xsd:sequence>
                     <xsd:attribute name="uri" type="xsd:token"/>
                   </xsd:complexType>"#,
                true,
                "sml.xsd / dml-chart.xsd CT_Extension: one wildcard, minOccurs defaulting to 1",
            ),
            (
                r#"<xsd:complexType xmlns:xsd="http://www.w3.org/2001/XMLSchema" name="CT_OfficeArtExtension">
                     <xsd:sequence><xsd:any processContents="lax" minOccurs="0" maxOccurs="unbounded"/></xsd:sequence>
                   </xsd:complexType>"#,
                false,
                "dml-main.xsd CT_OfficeArtExtension: the same wildcard, but optional",
            ),
            (
                r#"<xsd:complexType xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:w="urn:w" name="CT_Textbox">
                     <xsd:choice>
                       <xsd:element ref="w:txbxContent" minOccurs="0"/>
                       <xsd:any namespace='##local' processContents="skip"/>
                     </xsd:choice>
                   </xsd:complexType>"#,
                false,
                "vml-main.xsd CT_Textbox: a choice with an optional branch matches empty content",
            ),
            (
                r#"<xsd:complexType xmlns:xsd="http://www.w3.org/2001/XMLSchema" name="CT_Mixed" mixed="true">
                     <xsd:sequence><xsd:element name="named"/><xsd:any/></xsd:sequence>
                   </xsd:complexType>"#,
                false,
                "a required wildcard beside a required named child is not a slot: emptying it could \
                 be an author's missing element",
            ),
        ];
        for (source, expected, why) in cases {
            let document =
                mjx_xml::fidelity::parse(source.as_bytes()).expect("the case parses as XML");
            assert_eq!(
                holds_nothing_but_a_required_wildcard(&document.root, &document.interner),
                *expected,
                "{why}"
            );
        }
    }
}
