//! Package-graph invariants — the checks [`Package::save`] runs *before* it writes a byte.
//!
//! # Why this exists
//!
//! A part can be perfectly well-formed, and perfectly schema-valid against its XSD, while the
//! *package* that holds it is broken: markup naming a relationship id its own `.rels` never
//! declares, a relationship pointing at a part that is not in the container, a part no content-type
//! rule covers. None of those are visible to a per-part schema check, because none of them are
//! properties of a part — they are properties of the graph. They are also exactly what makes a
//! consumer report that it "found a problem with the content and needs to repair".
//!
//! # What is checked, and over what
//!
//! The invariants fall into two groups, and they are deliberately scoped differently.
//!
//! **Graph invariants** — content-type coverage, relationship targets, relationship-id uniqueness —
//! are checked over the **whole package**. They decide whether the container opens at all, and an
//! edit anywhere (removing a part, retargeting a relationship) can break an edge the caller never
//! looked at, so there is no smaller honest scope.
//!
//! **Markup invariants** — a relationship reference resolving to a declared relationship — are
//! checked only over the parts whose bytes *this library produced*
//! ([`Package::authored_xml_parts`]). That scope is a correctness decision before it is a cost one:
//! a part still holding its container bytes re-emits verbatim, so faulting it would mean refusing to
//! write back a file we were given — the opposite of this project's promise. It also means a save
//! never parses markup it was not going to re-serialize, and that *reading* a part can never change
//! whether a package saves.
//!
//! # Relationship to the orphan sweep
//!
//! [`Package::remove_unreferenced_parts`] walks the same edges with the same resolver
//! (`resolve_rel`), and the two must not disagree. They differ only in what they do with a broken
//! edge: the sweep *tolerates* it (a package that already contained a broken target is not made
//! worse by deleting something else), this pass *reports* it. The sweep's other conclusion — that a
//! part nothing references is legal, merely dead weight — is preserved here: an unreferenced part is
//! **not** a defect, and validation never removes anything.
//!
//! # Cost
//!
//! One pass, with the indexes built once: `O(parts + content-type rules + relationships)` for the
//! graph invariants, plus the markup of the authored parts for the reference check. Untouched
//! container bytes are never tokenized.

use std::collections::HashSet;

use mjx_ooxml_core::{Interner, RawDocument, RawElement, RawName, RawNode, Symbol};
use mjx_xml::fidelity;

use crate::content_types::CONTENT_TYPES_ZIP_NAME;
use crate::name::PartName;
use crate::package::{resolve_rel, Package};
use crate::rels::{rels_zip_name_for, TargetMode};

/// The Transitional namespace of the shared *relationship reference* attributes (`r:id`, `r:embed`,
/// `r:link`, …) — the namespace document markup uses to name a relationship in its own `.rels`.
///
/// This is **not** the OPC relationships namespace (which is
/// `…/package/2006/relationships`, the namespace of the `.rels` markup itself); see
/// [`RELATIONSHIPS_NS`](crate::rels::RELATIONSHIPS_NS).
const RELATIONSHIP_REFERENCE_NS_TRANSITIONAL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// The Strict (ISO-29500) namespace of the same attributes.
const RELATIONSHIP_REFERENCE_NS_STRICT: &str =
    "http://purl.oclc.org/ooxml/officeDocument/relationships";

/// One broken package invariant, named down to the part, relationship and identifier at fault — so a
/// caller can fix the fault without re-deriving where it is.
///
/// Returned by [`Package::validate`] and, wrapped in [`OpcError::Invalid`](crate::OpcError::Invalid),
/// by [`Package::save`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PackageDefect {
    /// A part that no content-type rule covers — neither an `Override` naming it nor a `Default` for
    /// its extension.
    ///
    /// ECMA-376 Part 2 §6.2.3: *"Each part shall have a MIME media type"*, and §7.2.3.5 defines the
    /// two-step lookup (`Override`, then `Default` by extension) this mirrors. A part a consumer
    /// cannot type is a part it cannot open.
    #[error(
        "part {part} has no content type: no Override names it and no Default covers its extension"
    )]
    PartWithoutContentType {
        /// The part with no content type.
        part: String,
    },

    /// An internal relationship whose target does not resolve to a part name at all — it climbed
    /// above the package root, failed part-name validation, or was an absolute URI written without
    /// `TargetMode="External"`.
    #[error("relationship {relationship_id} in {relationships_part} has an unresolvable internal target {target:?}")]
    UnresolvableRelationshipTarget {
        /// The `.rels` part the relationship is written in.
        relationships_part: String,
        /// The relationship's `Id`.
        relationship_id: String,
        /// The `Target` exactly as written.
        target: String,
    },

    /// An internal relationship naming a part the package does not contain.
    #[error("relationship {relationship_id} in {relationships_part} targets {resolved_part}, which is not in the package (Target={target:?})")]
    RelationshipTargetMissing {
        /// The `.rels` part the relationship is written in.
        relationships_part: String,
        /// The relationship's `Id`.
        relationship_id: String,
        /// The `Target` exactly as written.
        target: String,
        /// The part name that target resolves to, and which the package does not hold.
        resolved_part: String,
    },

    /// Two relationships in one `.rels` part sharing an `Id`.
    ///
    /// ECMA-376 Part 2 §6.5.3: *"The value of the Id attribute shall be unique within the
    /// Relationships part."* Markup naming the id then resolves to whichever the consumer happens to
    /// pick.
    #[error("relationship id {relationship_id} appears more than once in {relationships_part}")]
    DuplicateRelationshipId {
        /// The `.rels` part holding both.
        relationships_part: String,
        /// The duplicated `Id`.
        relationship_id: String,
    },

    /// Markup naming a relationship its part's `.rels` does not declare — the dangling `r:id`.
    ///
    /// Every attribute in the shared relationship-reference namespace is an `ST_RelationshipId`
    /// (ECMA-376 Part 1, `shared-relationshipReference.xsd`: `id`, `embed`, `link`, `pict`, `href`,
    /// `dm`, `lo`, `qs`, `cs`, `blip`, and the four corner attributes), so this covers all of them
    /// rather than `r:id` alone. The empty value those attributes default to means *no relationship*
    /// and is not reported.
    #[error("{part}: {element}/@{attribute} names relationship {relationship_id}, which its .rels does not declare")]
    UndeclaredRelationshipReference {
        /// The part whose markup names it.
        part: String,
        /// The element carrying the attribute, as written (prefix included).
        element: String,
        /// The attribute, as written (prefix included).
        attribute: String,
        /// The relationship id that is not declared.
        relationship_id: String,
    },

    /// A part this library produced, typed as XML, whose bytes are not well-formed XML.
    ///
    /// Only ever reported for markup this library wrote (see [`Package::authored_xml_parts`]) — a
    /// part still holding its container bytes is re-emitted verbatim and is never re-parsed here.
    #[error(
        "part {part} is typed as XML but this library's bytes for it are not well-formed: {error}"
    )]
    PartIsNotWellFormedXml {
        /// The part.
        part: String,
        /// The parse failure, as reported by the XML layer.
        error: String,
    },
}

impl Package {
    /// Checks every package-graph invariant, returning the first defect found in a deterministic
    /// order (content types, then relationships, then markup references — parts in container order,
    /// relationships in document order).
    ///
    /// This is a **read-only** pass: it parses nothing that is not already parsed except the markup
    /// this library itself authored, it caches nothing, it reorders nothing, and it leaves every
    /// part in exactly the copy-on-write state it found it in. [`save`](Self::save) runs it; see the
    /// module documentation for what is checked over what.
    ///
    /// # Errors
    /// Returns the first [`PackageDefect`] found.
    pub fn validate(&self) -> Result<(), PackageDefect> {
        self.check_content_type_coverage()?;
        self.check_relationships()?;
        self.check_relationship_references()?;
        Ok(())
    }

    /// Every part is covered by an `Override` or by a `Default` for its extension (§6.2.3).
    ///
    /// The two indexes mirror [`ContentTypes::content_type_of`](crate::ContentTypes::content_type_of)
    /// exactly — `Override` by part name, then `Default` by lowercased extension — so that the check
    /// and the lookup can never reach different answers; `content_type_lookup_agrees_with_the_index`
    /// pins that.
    fn check_content_type_coverage(&self) -> Result<(), PackageDefect> {
        let overrides: HashSet<&str> = self
            .content_types()
            .overrides()
            .iter()
            .map(|rule| rule.part_name.as_str())
            .collect();
        let defaults: HashSet<&str> = self
            .content_types()
            .defaults()
            .iter()
            .map(|rule| rule.extension.as_str())
            .collect();

        for entry in self.entries() {
            // `[Content_Types].xml` is the content-type stream itself, not a part, so nothing types
            // it. An entry name that is not a valid part name is skipped for the same reason
            // `part_names` skips it: it cannot be addressed, and this library never writes one.
            if entry.name == CONTENT_TYPES_ZIP_NAME {
                continue;
            }
            let Ok(part) = PartName::from_zip_name(&entry.name) else {
                continue;
            };
            let covered = overrides.contains(part.as_str())
                || part
                    .extension()
                    .is_some_and(|extension| defaults.contains(extension.as_str()));
            if !covered {
                return Err(PackageDefect::PartWithoutContentType {
                    part: part.as_str().to_owned(),
                });
            }
        }
        Ok(())
    }

    /// Relationship ids are unique within their `.rels` (§6.5.3), and every internal target resolves
    /// to a part the package holds.
    fn check_relationships(&self) -> Result<(), PackageDefect> {
        let present: HashSet<&str> = self
            .entries()
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();

        for rels_part in self.relationships() {
            let relationships_part = rels_zip_name_for(rels_part.source.as_ref());
            let mut seen: HashSet<&str> = HashSet::new();
            for rel in rels_part.relationships.iter() {
                if !seen.insert(rel.id.as_str()) {
                    return Err(PackageDefect::DuplicateRelationshipId {
                        relationships_part: relationships_part.clone(),
                        relationship_id: rel.id.clone(),
                    });
                }
                if rel.mode == TargetMode::External {
                    continue;
                }
                // The same resolver the orphan sweep walks with, so the two agree on what an edge
                // points at; they differ only in tolerating versus reporting a broken one.
                let Ok(target) = resolve_rel(rels_part.source.as_ref(), &rel.target) else {
                    return Err(PackageDefect::UnresolvableRelationshipTarget {
                        relationships_part: relationships_part.clone(),
                        relationship_id: rel.id.clone(),
                        target: rel.target.clone(),
                    });
                };
                if !present.contains(target.zip_name()) {
                    return Err(PackageDefect::RelationshipTargetMissing {
                        relationships_part: relationships_part.clone(),
                        relationship_id: rel.id.clone(),
                        target: rel.target.clone(),
                        resolved_part: target.as_str().to_owned(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Every relationship-reference attribute in markup this library produced names a relationship
    /// that part's `.rels` declares.
    fn check_relationship_references(&self) -> Result<(), PackageDefect> {
        for (part, entry) in self.authored_xml_parts() {
            let declared: HashSet<&str> = self
                .relationships_for(Some(&part))
                .map(|rels| rels.iter().map(|rel| rel.id.as_str()).collect())
                .unwrap_or_default();

            match entry.tree() {
                // Already in memory (edited, or read earlier): walked as it stands, never re-parsed.
                Some(tree) => check_part_references(&part, tree, &declared)?,
                // Bytes this library wrote and has not parsed. Tokenizing them costs what we are
                // about to write, never what the container gave us.
                None => {
                    let Some(bytes) = entry.bytes() else {
                        continue;
                    };
                    let tree = fidelity::parse(bytes).map_err(|error| {
                        PackageDefect::PartIsNotWellFormedXml {
                            part: part.as_str().to_owned(),
                            error: error.to_string(),
                        }
                    })?;
                    check_part_references(&part, &tree, &declared)?;
                }
            }
        }
        Ok(())
    }
}

/// One step of the iterative markup walk. The walk is iterative rather than recursive because it runs
/// over untrusted markup, where nesting depth is the attacker's to choose.
enum Step<'a> {
    /// Visit this element (bind its namespace declarations, check its attributes, queue its
    /// children).
    Visit(&'a RawElement),
    /// Every descendant of an element has been visited: drop the namespace bindings it introduced.
    LeaveScope(usize),
}

/// Checks one part's markup: every attribute in the relationship-reference namespace must name a
/// relationship in `declared`.
///
/// The fidelity reader resolves *element* namespaces but leaves *attribute* namespaces unresolved
/// (only the literal prefix is kept), so this resolves prefixes itself, with proper scoping: a
/// binding introduced by an element covers that element and its descendants, an inner binding
/// shadows an outer one, and a default `xmlns="…"` binds nothing here because a default namespace
/// never applies to an unprefixed attribute.
fn check_part_references(
    part: &PartName,
    tree: &RawDocument,
    declared: &HashSet<&str>,
) -> Result<(), PackageDefect> {
    let interner = &tree.interner;
    // (prefix symbol, whether it is bound to the relationship-reference namespace), innermost last.
    let mut bindings: Vec<(Symbol, bool)> = Vec::new();
    let mut reference_bindings = 0usize;
    let mut work: Vec<Step<'_>> = vec![Step::Visit(&tree.root)];

    while let Some(step) = work.pop() {
        let element = match step {
            Step::LeaveScope(depth) => {
                for (_, is_reference) in bindings.drain(depth..) {
                    if is_reference {
                        reference_bindings -= 1;
                    }
                }
                continue;
            }
            Step::Visit(element) => element,
        };

        let depth = bindings.len();
        for attribute in &element.attributes {
            if !is_namespace_declaration(&attribute.name, interner) {
                continue;
            }
            let is_reference =
                is_relationship_reference_namespace(&attribute_text(&attribute.value));
            bindings.push((attribute.name.local, is_reference));
            if is_reference {
                reference_bindings += 1;
            }
        }

        // Nothing in scope binds the relationship-reference namespace, so no attribute here can name
        // a relationship: skip the attribute scan entirely.
        if reference_bindings > 0 {
            for attribute in &element.attributes {
                let Some(prefix) = attribute.name.prefix else {
                    continue;
                };
                if is_namespace_declaration(&attribute.name, interner) {
                    continue;
                }
                let bound_to_references = bindings
                    .iter()
                    .rev()
                    .find(|(bound, _)| *bound == prefix)
                    .is_some_and(|(_, is_reference)| *is_reference);
                if !bound_to_references {
                    continue;
                }
                let value = attribute_text(&attribute.value);
                // `ST_RelationshipId` defaults to the empty string on `r:dm`, `r:lo`, `r:qs`, `r:cs`
                // and `r:blip`, where it means *no relationship* rather than a broken one.
                if value.is_empty() {
                    continue;
                }
                if !declared.contains(value.as_str()) {
                    return Err(PackageDefect::UndeclaredRelationshipReference {
                        part: part.as_str().to_owned(),
                        element: qualified_name(&element.name, interner),
                        attribute: qualified_name(&attribute.name, interner),
                        relationship_id: value,
                    });
                }
            }
        }

        if bindings.len() != depth {
            work.push(Step::LeaveScope(depth));
        }
        for child in element.children.iter().rev() {
            if let RawNode::Element(child) = child {
                work.push(Step::Visit(child));
            }
        }
    }
    Ok(())
}

/// Whether a name is an `xmlns:PREFIX` declaration. A default `xmlns="…"` is *not* one for this
/// purpose: it binds the default namespace, which never applies to an unprefixed attribute.
fn is_namespace_declaration(name: &RawName, interner: &Interner) -> bool {
    name.prefix
        .is_some_and(|prefix| interner.resolve(prefix) == "xmlns")
}

/// Whether a namespace URI is the shared relationship-reference namespace, in either conformance
/// world.
fn is_relationship_reference_namespace(uri: &str) -> bool {
    uri == RELATIONSHIP_REFERENCE_NS_TRANSITIONAL || uri == RELATIONSHIP_REFERENCE_NS_STRICT
}

/// An attribute's value as text: the raw bytes are stored escaped and are never unescaped on read, so
/// they are decoded here. Lossy on the (impossible for well-formed XML) non-UTF-8 case rather than
/// failing — a value that is not text matches no declared relationship id, which is what it will be
/// reported as.
fn attribute_text(value: &[u8]) -> String {
    let raw = String::from_utf8_lossy(value);
    match mjx_xml::text::unescape_text(&raw) {
        Ok(text) => text.into_owned(),
        Err(_) => raw.into_owned(),
    }
}

/// A name as it was written: `prefix:local`, or `local` when unprefixed.
fn qualified_name(name: &RawName, interner: &Interner) -> String {
    match name.prefix {
        Some(prefix) => format!(
            "{}:{}",
            interner.resolve(prefix),
            interner.resolve(name.local)
        ),
        None => interner.resolve(name.local).to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rels::Relationship;

    fn part(name: &str) -> PartName {
        PartName::new(name).expect("valid part name")
    }

    /// The coverage index and the content-type lookup must agree part for part, or the validator
    /// would refuse packages the rest of the crate considers well typed (or the reverse).
    #[test]
    fn content_type_lookup_agrees_with_the_index() {
        let mut package = Package::empty();
        package
            .insert_part(&part("/a/one.xml"), "app/one", b"<a/>".to_vec())
            .expect("insert");
        package
            .set_content_type_default("png", "image/png")
            .expect("default");
        package
            .insert_part(&part("/a/two.png"), "image/png", vec![0])
            .expect("insert");

        for name in package.part_names() {
            let covered = package.content_type_of(&name).is_some();
            let indexed = package.check_content_type_coverage().is_ok();
            assert!(covered, "{} has no content type", name.as_str());
            assert!(indexed, "index disagrees for {}", name.as_str());
        }

        // And a part with neither rule is reported by both.
        let mut broken = Package::empty();
        broken
            .insert_part(&part("/a/one.bin"), "app/bin", vec![0])
            .expect("insert");
        broken
            .remove_content_type_override(&part("/a/one.bin"))
            .expect("remove override");
        assert!(broken.content_type_of(&part("/a/one.bin")).is_none());
        assert!(matches!(
            broken.check_content_type_coverage(),
            Err(PackageDefect::PartWithoutContentType { .. })
        ));
    }

    /// A prefix rebound inside the document must be honoured: `r` bound to something else in a
    /// subtree names no relationship there.
    #[test]
    fn a_rebound_prefix_is_not_a_relationship_reference() {
        let xml = br#"<p:x xmlns:p="urn:p" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:y xmlns:r="urn:not-relationships" r:id="nonsense"/></p:x>"#;
        let tree = fidelity::parse(xml).expect("well-formed");
        let declared = HashSet::new();
        check_part_references(&part("/p.xml"), &tree, &declared).expect("no reference in scope");
    }

    /// …and the binding must fall out of scope again when the subtree ends.
    #[test]
    fn a_binding_leaves_scope_with_its_element() {
        let xml = br#"<p:x xmlns:p="urn:p"><p:y xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" r:id="rId1"/><p:z r:id="rId1"/></p:x>"#;
        let tree = fidelity::parse(xml).expect("well-formed");
        let declared: HashSet<&str> = ["rId1"].into_iter().collect();
        // `p:z`'s `r:` prefix is unbound, so it is not a relationship reference and is not checked;
        // `p:y`'s is bound and resolves.
        check_part_references(&part("/p.xml"), &tree, &declared).expect("bindings scoped");
    }

    /// The empty value `ST_RelationshipId` defaults to means "no relationship", not a broken one.
    #[test]
    fn an_empty_reference_is_not_a_dangling_one() {
        let xml = br#"<p:x xmlns:p="urn:p" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" r:dm="" r:lo=""/>"#;
        let tree = fidelity::parse(xml).expect("well-formed");
        let declared = HashSet::new();
        check_part_references(&part("/p.xml"), &tree, &declared).expect("empty is not dangling");
    }

    /// Every attribute in the reference namespace is an `ST_RelationshipId`, not only `r:id`.
    #[test]
    fn every_reference_attribute_is_checked_not_only_r_id() {
        let xml = br#"<p:x xmlns:p="urn:p" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><a:blip r:embed="rId9" xmlns:a="urn:a"/></p:x>"#;
        let tree = fidelity::parse(xml).expect("well-formed");
        let declared = HashSet::new();
        let err = check_part_references(&part("/p.xml"), &tree, &declared)
            .expect_err("r:embed is a relationship id");
        match err {
            PackageDefect::UndeclaredRelationshipReference {
                attribute,
                relationship_id,
                ..
            } => {
                assert_eq!(attribute, "r:embed");
                assert_eq!(relationship_id, "rId9");
            }
            other => panic!("unexpected defect: {other:?}"),
        }
    }

    /// An unreferenced part is legal (the orphan sweep's own conclusion), so it is not a defect.
    #[test]
    fn an_unreferenced_part_is_not_a_defect() {
        let mut package = Package::empty();
        package
            .insert_part(&part("/orphan.xml"), "app/x", b"<a/>".to_vec())
            .expect("insert");
        package.validate().expect("an orphan is legal");
    }

    /// A relationship whose target names no part is reported with both forms of the target.
    #[test]
    fn a_missing_target_names_the_part_it_resolves_to() {
        let mut package = Package::empty();
        package
            .add_relationship(
                None,
                Relationship {
                    id: "rId1".to_owned(),
                    rel_type: "urn:t".to_owned(),
                    target: "ppt/gone.xml".to_owned(),
                    mode: TargetMode::Internal,
                },
            )
            .expect("add");
        match package.validate().expect_err("target is absent") {
            PackageDefect::RelationshipTargetMissing {
                relationships_part,
                relationship_id,
                target,
                resolved_part,
            } => {
                assert_eq!(relationships_part, "_rels/.rels");
                assert_eq!(relationship_id, "rId1");
                assert_eq!(target, "ppt/gone.xml");
                assert_eq!(resolved_part, "/ppt/gone.xml");
            }
            other => panic!("unexpected defect: {other:?}"),
        }
    }
}
