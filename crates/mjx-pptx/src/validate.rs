//! PresentationML invariants — the checks [`Presentation::save`](crate::Presentation::save) runs on
//! top of the packaging ones.
//!
//! [`mjx_opc::Package::validate`] owns everything that is true of *any* OPC package: content-type
//! coverage, relationship targets, relationship-id uniqueness, and markup naming a relationship its
//! `.rels` never declares. What is left here is what only PresentationML knows — the identifier
//! spaces the format requires to be unique, and the two places where a list inside `presentation.xml`
//! (or a master) has to agree with that part's relationships.
//!
//! # Scope
//!
//! Exactly [`Package::authored_xml_parts`](mjx_opc::Package::authored_xml_parts) — the parts whose
//! markup this library will write. The packaging layer defines that set; this one does not get to
//! disagree with it. A deck that is opened and saved untouched is therefore never faulted for markup
//! it arrived with, and reading a slide can never change whether a deck saves.
//!
//! # The forward direction is one layer down
//!
//! "A `p:sldId` whose `r:id` no relationship declares" is *not* checked here: it is a dangling
//! relationship reference like any other, and `mjx-opc` already reports it as
//! [`PackageDefect::UndeclaredRelationshipReference`](mjx_opc::PackageDefect::UndeclaredRelationshipReference)
//! over the same set of parts. Restating it here would be a second, drifting implementation of one
//! rule. What remains is the direction packaging cannot see: whether the relationship an entry names
//! leads to the *kind* of part the list is for, and whether a part of that kind is listed at all.

use std::collections::HashSet;

use mjx_ooxml_core::{Interner, RawDocument, RawElement, RawNode};
use mjx_ooxml_types::namespaces::{PML, SHARED_RELATIONSHIP_REFERENCE};
use mjx_opc::{Package, PartName, TargetMode};
use mjx_xml::fidelity;

use crate::constants;
use crate::nav;
use crate::slide::MCE;

/// One broken PresentationML invariant, named down to the part, list and identifier at fault.
///
/// Returned (wrapped in [`PptxError::InvalidPresentation`](crate::PptxError::InvalidPresentation)) by
/// [`Presentation::validate`](crate::Presentation::validate) and
/// [`Presentation::save`](crate::Presentation::save).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PresentationDefect {
    /// Two shapes in one part's shape tree sharing a `p:cNvPr@id`.
    ///
    /// The non-visual id is the handle everything else uses to name a shape — animations, the
    /// `spid` a VML shape or an OLE object is matched by, a placeholder's inheritance. Two shapes
    /// answering to one id is the ambiguity a consumer resolves by repairing the file.
    ///
    /// Branches of an `mc:AlternateContent` are alternatives rather than siblings — the same shape
    /// appears in each with the same id — so ids in different branches are not duplicates of each
    /// other. They are still duplicates of an id used outside the `mc:AlternateContent`.
    #[error("{part}: shape id {shape_id} is used by more than one shape in the same tree")]
    DuplicateShapeId {
        /// The part holding the tree.
        part: String,
        /// The repeated `p:cNvPr@id`.
        shape_id: String,
    },

    /// Two entries of one id list sharing an `@id` (`p:sldId@id`, `p:sldMasterId@id`,
    /// `p:sldLayoutId@id`), which the format requires to be unique within the list.
    #[error("{part}: {list} has more than one entry with id {entry_id}")]
    DuplicateListEntryId {
        /// The part holding the list.
        part: String,
        /// The list element, e.g. `p:sldIdLst`.
        list: String,
        /// The repeated `@id`.
        entry_id: String,
    },

    /// Two entries of one id list naming the same relationship — the same slide (or master, or
    /// layout) listed twice under different ids.
    #[error("{part}: {list} names relationship {relationship_id} more than once")]
    DuplicateListEntryReference {
        /// The part holding the list.
        part: String,
        /// The list element, e.g. `p:sldIdLst`.
        list: String,
        /// The relationship named twice.
        relationship_id: String,
    },

    /// An entry naming a relationship that leads to the wrong kind of part — a `p:sldId` pointing at
    /// something that is not a slide.
    #[error("{part}: {list} entry names relationship {relationship_id}, which targets {target_part} of type {actual_content_type} rather than {expected_content_type}")]
    ListEntryTargetHasWrongContentType {
        /// The part holding the list.
        part: String,
        /// The list element, e.g. `p:sldIdLst`.
        list: String,
        /// The relationship the entry names.
        relationship_id: String,
        /// The part that relationship targets.
        target_part: String,
        /// The content type the list requires of its entries' targets.
        expected_content_type: String,
        /// The content type the target actually has.
        actual_content_type: String,
    },

    /// A relationship leading to a part of the list's kind that no entry names — a slide part the
    /// deck relates to but `p:sldIdLst` never lists, which is a slide the presentation cannot show.
    #[error(
        "{part}: relationship {relationship_id} targets {target_part}, which {list} does not list"
    )]
    UnlistedRelationship {
        /// The part holding the list.
        part: String,
        /// The list element that should have named it, e.g. `p:sldIdLst`.
        list: String,
        /// The relationship no entry names.
        relationship_id: String,
        /// The part it targets.
        target_part: String,
    },
}

/// One id list and what its entries must lead to.
struct IdList {
    /// The list element's local name (`sldIdLst`).
    list: &'static str,
    /// The entry element's local name (`sldId`).
    entry: &'static str,
    /// The content type every entry's relationship must target.
    content_type: &'static str,
}

/// The three id lists PresentationML keeps in step with a part's relationships. `p:notesMasterIdLst`
/// and `p:handoutMasterIdLst` are deliberately absent: they hold at most one entry and no library
/// path writes them, so there is nothing here for them to be inconsistent with.
const ID_LISTS: &[IdList] = &[
    IdList {
        list: "sldIdLst",
        entry: "sldId",
        content_type: constants::CONTENT_TYPE_SLIDE,
    },
    IdList {
        list: "sldMasterIdLst",
        entry: "sldMasterId",
        content_type: constants::CONTENT_TYPE_SLIDE_MASTER,
    },
    IdList {
        list: "sldLayoutIdLst",
        entry: "sldLayoutId",
        content_type: constants::CONTENT_TYPE_SLIDE_LAYOUT,
    },
];

/// Checks every PresentationML invariant over the markup this library will write.
///
/// A pure function of the package: it takes `&Package`, parses nothing that is not already parsed
/// except the authored bytes it is about to check, and changes nothing.
pub(crate) fn check(package: &Package) -> Result<(), crate::PptxError> {
    for (part, entry) in package.authored_xml_parts() {
        match entry.tree() {
            Some(tree) => check_part(package, &part, tree)?,
            None => {
                let Some(bytes) = entry.bytes() else {
                    continue;
                };
                // Well-formedness of an authored part is `mjx-opc`'s defect to report; if it cannot
                // be parsed there is nothing here to check.
                let Ok(tree) = fidelity::parse(bytes) else {
                    continue;
                };
                check_part(package, &part, &tree)?;
            }
        }
    }
    Ok(())
}

/// Checks one part: shape-id uniqueness in each of its shape trees, then each id list it carries.
fn check_part(
    package: &Package,
    part: &PartName,
    tree: &RawDocument,
) -> Result<(), crate::PptxError> {
    check_shape_ids(part, tree)?;
    check_id_lists(package, part, tree)?;
    Ok(())
}

/// `p:cNvPr@id` is unique within a shape tree.
fn check_shape_ids(part: &PartName, tree: &RawDocument) -> Result<(), crate::PptxError> {
    let interner = &tree.interner;
    for shape_tree in shape_trees(&tree.root, interner) {
        let mut seen: HashSet<String> = HashSet::new();
        collect_shape_ids(part, shape_tree, interner, &mut seen)?;
    }
    Ok(())
}

/// Every `p:spTree` in the part, in document order. A slide, layout, master, notes slide and handout
/// master each have exactly one, under `p:cSld`; searching for the tree itself rather than for the
/// part kind means a part kind nobody has thought of yet is still checked.
fn shape_trees<'a>(root: &'a RawElement, interner: &'a Interner) -> Vec<&'a RawElement> {
    let mut found = Vec::new();
    let mut work = vec![root];
    while let Some(element) = work.pop() {
        if nav::name_is(&element.name, interner, PML, "spTree") {
            found.push(element);
            // A shape tree never nests another, so there is nothing below to search.
            continue;
        }
        for child in element.children.iter().rev() {
            if let RawNode::Element(child) = child {
                work.push(child);
            }
        }
    }
    found
}

/// Collects the `p:cNvPr@id` values under `element` into `seen`, reporting the first repeat.
///
/// Recursive, mirroring the shape-tree recursion the rest of this crate already walks (see
/// `slide::max_cnvpr_id`), because `mc:AlternateContent` is genuinely a branching structure: each
/// branch is checked against the ids in scope *outside* the `mc:AlternateContent`, and never against
/// its sibling branches, since a consumer selects exactly one of them.
fn collect_shape_ids(
    part: &PartName,
    element: &RawElement,
    interner: &Interner,
    seen: &mut HashSet<String>,
) -> Result<(), crate::PptxError> {
    if nav::name_is(&element.name, interner, MCE, "AlternateContent") {
        for child in &element.children {
            let RawNode::Element(branch) = child else {
                continue;
            };
            let mut branch_seen = seen.clone();
            collect_shape_ids(part, branch, interner, &mut branch_seen)?;
        }
        return Ok(());
    }

    if nav::name_is(&element.name, interner, PML, "cNvPr") {
        if let Some(id) = nav::attr_value(element, interner, "id") {
            if !seen.insert(id.to_owned()) {
                return Err(PresentationDefect::DuplicateShapeId {
                    part: part.as_str().to_owned(),
                    shape_id: id.to_owned(),
                }
                .into());
            }
        }
    }

    for child in &element.children {
        if let RawNode::Element(child) = child {
            collect_shape_ids(part, child, interner, seen)?;
        }
    }
    Ok(())
}

/// Each id list this part carries agrees with this part's relationships, in both directions.
fn check_id_lists(
    package: &Package,
    part: &PartName,
    tree: &RawDocument,
) -> Result<(), crate::PptxError> {
    let interner = &tree.interner;
    // The reader leaves attribute namespaces unresolved, so `r:id` is found through whichever prefix
    // the root binds to the relationship-reference namespace. A part that binds none carries no
    // relationship reference to check.
    let Some(reference_prefix) =
        nav::namespace_prefix(&tree.root, interner, SHARED_RELATIONSHIP_REFERENCE)
    else {
        return Ok(());
    };

    for spec in ID_LISTS {
        let Some(list) = nav::child(&tree.root, interner, PML, spec.list) else {
            continue;
        };

        // What the list names, in order: `@id` (unique per list) and `r:id` (unique per list).
        let mut entry_ids: HashSet<String> = HashSet::new();
        let mut listed: HashSet<String> = HashSet::new();
        let mut listed_order: Vec<String> = Vec::new();
        for entry in nav::children(list, interner, PML, spec.entry) {
            if let Some(id) = nav::attr_value(entry, interner, "id") {
                if !entry_ids.insert(id.to_owned()) {
                    return Err(PresentationDefect::DuplicateListEntryId {
                        part: part.as_str().to_owned(),
                        list: qualified(spec.list),
                        entry_id: id.to_owned(),
                    }
                    .into());
                }
            }
            let Some(reference) = nav::prefixed_attr_value(entry, interner, reference_prefix, "id")
            else {
                continue;
            };
            let reference = reference?;
            if !listed.insert(reference.clone()) {
                return Err(PresentationDefect::DuplicateListEntryReference {
                    part: part.as_str().to_owned(),
                    list: qualified(spec.list),
                    relationship_id: reference,
                }
                .into());
            }
            listed_order.push(reference);
        }

        // What the relationships offer: every internal relationship whose target part carries the
        // content type this list is for. Matching on the *content type* rather than the relationship
        // type keeps this correct in both conformance worlds, whose relationship-type URIs differ
        // while their content types do not.
        let Some(relationships) = package.relationships_for(Some(part)) else {
            continue;
        };
        let mut of_this_kind: Vec<(&str, PartName)> = Vec::new();
        for rel in relationships.iter() {
            if rel.mode == TargetMode::External {
                continue;
            }
            let Ok(target) = nav::resolve_target(part, &rel.target) else {
                continue; // Unresolvable targets are `mjx-opc`'s defect to report.
            };
            if package.content_type_of(&target) == Some(spec.content_type) {
                of_this_kind.push((rel.id.as_str(), target));
            }
        }

        // An entry naming a relationship that leads somewhere else.
        for reference in &listed_order {
            if of_this_kind.iter().any(|(id, _)| id == reference) {
                continue;
            }
            let Some(rel) = relationships.by_id(reference) else {
                // Undeclared: reported one layer down, as a dangling relationship reference.
                continue;
            };
            let Ok(target) = nav::resolve_target(part, &rel.target) else {
                continue;
            };
            let Some(actual) = package.content_type_of(&target) else {
                continue; // A target with no content type is `mjx-opc`'s defect to report.
            };
            return Err(PresentationDefect::ListEntryTargetHasWrongContentType {
                part: part.as_str().to_owned(),
                list: qualified(spec.list),
                relationship_id: reference.clone(),
                target_part: target.as_str().to_owned(),
                expected_content_type: spec.content_type.to_owned(),
                actual_content_type: actual.to_owned(),
            }
            .into());
        }

        // …and the reverse: a part of this kind that the list never names.
        for (id, target) in &of_this_kind {
            if listed.contains(*id) {
                continue;
            }
            return Err(PresentationDefect::UnlistedRelationship {
                part: part.as_str().to_owned(),
                list: qualified(spec.list),
                relationship_id: (*id).to_owned(),
                target_part: target.as_str().to_owned(),
            }
            .into());
        }
    }
    Ok(())
}

/// A PresentationML local name as it is written in markup, for a message a reader can grep for.
fn qualified(local: &str) -> String {
    format!("p:{local}")
}
