//! SpreadsheetML invariants — the checks [`Workbook::save`](crate::Workbook::save) runs on top of
//! the packaging ones.
//!
//! [`mjx_opc::Package::validate`] owns everything that is true of *any* OPC package: content-type
//! coverage, relationship targets, relationship-id uniqueness, and markup naming a relationship its
//! `.rels` never declares. What is left here is what only SpreadsheetML knows.
//!
//! # Two scopes, and why they differ
//!
//! `mjx-opc` draws the line and this module does not get to disagree with it (see
//! [`mjx_opc::Package::authored_xml_parts`]'s own doc comment). Restated in Excel's terms:
//!
//! * **Graph invariants** — [`WorkbookIsNotTheOfficeDocument`](SpreadsheetDefect::WorkbookIsNotTheOfficeDocument)
//!   and [`UnreachableSpreadsheetPart`](SpreadsheetDefect::UnreachableSpreadsheetPart) — are checked
//!   over the **whole package**, because they are properties of relationships and content types, not
//!   of anyone's markup, and an edit anywhere can break an edge the caller never looked at. This is
//!   the same scope `Package::validate` uses for its own relationship checks, which likewise refuse
//!   to save a container that arrived broken.
//! * **Markup invariants** — everything about the `x:sheets` list — are checked only over
//!   [`Package::authored_xml_parts`](mjx_opc::Package::authored_xml_parts), the parts whose bytes
//!   this library will write. A workbook opened and saved untouched is never faulted for markup it
//!   arrived with, and *reading* a sheet can never change whether a workbook saves.
//!
//! # The forward direction is one layer down
//!
//! "A `x:sheet` whose `r:id` no relationship declares" is **not** checked here. It is a dangling
//! relationship reference like any other, and `mjx-opc` already reports it as
//! [`PackageDefect::UndeclaredRelationshipReference`](mjx_opc::PackageDefect::UndeclaredRelationshipReference)
//! over the same set of parts; restating it would be a second, drifting implementation of one rule —
//! exactly the note `mjx_pptx::validate` carries for `p:sldId`. What is left is the direction
//! packaging cannot see: whether the relationship an entry names leads to a part of the *kind* a
//! sheet list is for, whether a sheet part is listed at all, and the two identifier spaces §18.2.19
//! requires to be unique.
//!
//! # The orphan question, answered differently here than in OPC
//!
//! [`mjx_opc::Package::validate`] is explicit that an unreferenced part is legal, merely dead
//! weight, and never a defect. [`UnreachableSpreadsheetPart`](SpreadsheetDefect::UnreachableSpreadsheetPart)
//! narrows that for one family and one reason. A SpreadsheetML part is reached *only* through the
//! workbook's graph — there is no other consumer of an `xl/sharedStrings.xml` — so one the graph
//! cannot reach is not dead weight, it is a missing dependency: every `t="s"` cell in every
//! worksheet then indexes into a table nothing loads. The check is deliberately limited to the
//! `…spreadsheetml.*` content-type family, so a stray theme, image or OLE object stays legal, and
//! [`Workbook::save_unchecked`](crate::Workbook::save_unchecked) is the way to write a container
//! back exactly as it arrived regardless.

use std::collections::{HashMap, HashSet};

use mjx_ooxml_core::RawDocument;
use mjx_ooxml_types::namespaces::{SHARED_RELATIONSHIP_REFERENCE, SML};
use mjx_opc::{Package, PartName, TargetMode};
use mjx_xml::fidelity;

use crate::error::XlsxError;
use crate::nav;
use crate::parts::{SheetKind, REL_OFFICE_DOCUMENT};

/// The content-type prefix every SpreadsheetML part shares.
///
/// Used by [`SpreadsheetDefect::UnreachableSpreadsheetPart`]'s check to pick out the family whose
/// only consumer is the workbook graph. Matching on the prefix rather than on [`PartKind`]'s own
/// list is deliberate: a SpreadsheetML part this crate does not classify (a revision log, a shared
/// workbook's user names) is still one nothing but the workbook can reach.
const SPREADSHEETML_CONTENT_TYPE_PREFIX: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.";

/// One broken SpreadsheetML invariant, named down to the part and the identifier at fault.
///
/// Returned — wrapped in [`XlsxError::InvalidWorkbook`](crate::XlsxError::InvalidWorkbook) — by
/// [`Workbook::validate`](crate::Workbook::validate) and [`Workbook::save`](crate::Workbook::save).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SpreadsheetDefect {
    /// The package-root `officeDocument` relationship no longer leads to the workbook part.
    ///
    /// ECMA-376 Part 1 §12.3.23: *"A package shall contain exactly one Workbook part, and that part
    /// shall be the target of a relationship in the package-relationship item."* A consumer finds
    /// the workbook through that one edge and nowhere else, so a package whose root relationship
    /// points at something else has no workbook at all, however much SpreadsheetML it contains.
    #[error(
        "the package-root officeDocument relationship targets {office_document_target:?}, not the \
         workbook part {workbook_part}"
    )]
    WorkbookIsNotTheOfficeDocument {
        /// The workbook part this [`Workbook`](crate::Workbook) was opened on.
        workbook_part: String,
        /// The `Target` the root relationship now names, exactly as written.
        office_document_target: String,
    },

    /// A SpreadsheetML part the container holds that no chain of relationships from the package root
    /// reaches.
    ///
    /// See this module's own documentation for why this one family is treated differently from
    /// `mjx-opc`'s "an unreferenced part is not a defect".
    #[error(
        "{part} is a SpreadsheetML part (content type {content_type}) that no relationship chain \
         from the package root reaches"
    )]
    UnreachableSpreadsheetPart {
        /// The unreachable part.
        part: String,
        /// Its content type.
        content_type: String,
    },

    /// A `x:sheet` entry naming a relationship that leads to something that is not a sheet.
    #[error(
        "{part}: x:sheets entry names relationship {relationship_id}, which targets {target_part} \
         of type {actual_content_type} — not a worksheet, chartsheet or dialogsheet"
    )]
    SheetEntryTargetIsNotASheet {
        /// The part holding the list (the workbook part).
        part: String,
        /// The relationship the entry names.
        relationship_id: String,
        /// The part that relationship targets.
        target_part: String,
        /// The content type that target actually has.
        actual_content_type: String,
    },

    /// A relationship leading to a sheet part that `x:sheets` never names — a sheet the workbook
    /// carries and no consumer will ever show.
    #[error("{part}: relationship {relationship_id} targets {target_part}, which x:sheets does not list")]
    UnlistedSheetRelationship {
        /// The part holding the list (the workbook part).
        part: String,
        /// The relationship no entry names.
        relationship_id: String,
        /// The part it targets.
        target_part: String,
    },

    /// Two `x:sheet` entries naming the same relationship — one sheet part listed as two tabs.
    #[error("{part}: x:sheets names relationship {relationship_id} more than once")]
    DuplicateSheetReference {
        /// The part holding the list.
        part: String,
        /// The relationship named twice.
        relationship_id: String,
    },

    /// Two `x:sheet` entries sharing a `@sheetId`.
    ///
    /// ECMA-376 Part 1 §18.2.19: *"Specifies the internal identifier for the sheet. This identifier
    /// shall be unique."*
    #[error("{part}: x:sheets has more than one entry with sheetId {sheet_id}")]
    DuplicateSheetId {
        /// The part holding the list.
        part: String,
        /// The repeated `@sheetId`.
        sheet_id: String,
    },

    /// Two `x:sheet` entries sharing a `@name`.
    ///
    /// ECMA-376 Part 1 §18.2.19: *"Specifies the name of the sheet. This name shall be unique."*
    /// Compared exactly, because that is the whole of what the clause says; a case-insensitive
    /// comparison would be a rule this project invented rather than one it read.
    #[error("{part}: x:sheets has more than one entry named {name:?}")]
    DuplicateSheetName {
        /// The part holding the list.
        part: String,
        /// The repeated `@name`.
        name: String,
    },
}

/// Checks every SpreadsheetML invariant, in a deterministic order: the workbook edge, then
/// reachability, then the sheet list.
///
/// A pure function of the package: it takes `&Package`, parses nothing that is not already parsed
/// except the authored bytes it is about to check, and changes nothing.
pub(crate) fn check(package: &Package, workbook_part: &PartName) -> Result<(), XlsxError> {
    check_office_document_edge(package, workbook_part)?;
    check_spreadsheet_parts_are_reachable(package)?;
    check_sheet_list(package, workbook_part)?;
    Ok(())
}

/// The package-root `officeDocument` relationship still leads to the workbook part.
fn check_office_document_edge(
    package: &Package,
    workbook_part: &PartName,
) -> Result<(), XlsxError> {
    let root = package
        .relationships_for(None)
        .ok_or(XlsxError::MissingOfficeDocument)?;
    let rel = root
        .by_type(REL_OFFICE_DOCUMENT)
        .next()
        .ok_or(XlsxError::MissingOfficeDocument)?;
    let names_the_workbook = rel.mode == TargetMode::Internal
        && nav::resolve_from_root(&rel.target).is_ok_and(|part| part == *workbook_part);
    if names_the_workbook {
        return Ok(());
    }
    Err(SpreadsheetDefect::WorkbookIsNotTheOfficeDocument {
        workbook_part: workbook_part.as_str().to_owned(),
        office_document_target: rel.target.clone(),
    }
    .into())
}

/// Every `…spreadsheetml.*` part is reachable from the package root by a chain of internal
/// relationships.
///
/// One breadth-first walk over the relationship graph, then one pass over the parts:
/// `O(parts + relationships)`, with no markup parsed at all.
fn check_spreadsheet_parts_are_reachable(package: &Package) -> Result<(), XlsxError> {
    let mut reached: HashSet<String> = HashSet::new();
    let mut frontier: Vec<Option<PartName>> = vec![None]; // `None` is the package root

    while let Some(source) = frontier.pop() {
        let Some(rels) = package.relationships_for(source.as_ref()) else {
            continue;
        };
        for rel in rels.iter() {
            if rel.mode == TargetMode::External {
                continue;
            }
            let resolved = match &source {
                Some(part) => nav::resolve_target(part, &rel.target),
                None => nav::resolve_from_root(&rel.target),
            };
            // An unresolvable or missing target is `mjx-opc`'s defect to report, and it reports it
            // before this pass runs; here it simply reaches nothing.
            let Ok(target) = resolved else { continue };
            if reached.insert(target.as_str().to_owned()) {
                frontier.push(Some(target));
            }
        }
    }

    for part in package.part_names() {
        let Some(content_type) = package.content_type_of(&part) else {
            continue; // `Package::validate` reports a part with no content type.
        };
        if !content_type.starts_with(SPREADSHEETML_CONTENT_TYPE_PREFIX) {
            continue;
        }
        if reached.contains(part.as_str()) {
            continue;
        }
        return Err(SpreadsheetDefect::UnreachableSpreadsheetPart {
            part: part.as_str().to_owned(),
            content_type: content_type.to_owned(),
        }
        .into());
    }
    Ok(())
}

/// The `x:sheets` list agrees with the workbook part's relationships, in both directions, and its
/// two identifier spaces are unique — checked only if this library will write the workbook's markup.
fn check_sheet_list(package: &Package, workbook_part: &PartName) -> Result<(), XlsxError> {
    let Some((_, entry)) = package
        .authored_xml_parts()
        .find(|(part, _)| part == workbook_part)
    else {
        return Ok(()); // Container bytes: not ours to fault. See this module's own docs.
    };

    match entry.tree() {
        Some(tree) => check_sheet_list_markup(package, workbook_part, tree),
        None => {
            let Some(bytes) = entry.bytes() else {
                return Ok(());
            };
            // Well-formedness of an authored part is `mjx-opc`'s defect to report; if it will not
            // parse there is nothing here to check.
            let Ok(tree) = fidelity::parse(bytes) else {
                return Ok(());
            };
            check_sheet_list_markup(package, workbook_part, &tree)
        }
    }
}

fn check_sheet_list_markup(
    package: &Package,
    workbook_part: &PartName,
    tree: &RawDocument,
) -> Result<(), XlsxError> {
    let interner = &tree.interner;
    let Some(sheets) = nav::child(&tree.root, interner, SML, "sheets") else {
        return Ok(());
    };
    let part = || workbook_part.as_str().to_owned();

    // What the list names, in order, and the two identifier spaces §18.2.19 requires to be unique.
    let mut listed: HashSet<String> = HashSet::new();
    let mut listed_order: Vec<String> = Vec::new();
    let mut sheet_ids: HashSet<String> = HashSet::new();
    let mut names: HashSet<String> = HashSet::new();
    let reference_prefix =
        nav::namespace_prefix(&tree.root, interner, SHARED_RELATIONSHIP_REFERENCE);

    for sheet in nav::children(sheets, interner, SML, "sheet") {
        if let Some(sheet_id) = nav::attr_value(sheet, interner, "sheetId") {
            let sheet_id = sheet_id?;
            if !sheet_ids.insert(sheet_id.clone()) {
                return Err(SpreadsheetDefect::DuplicateSheetId {
                    part: part(),
                    sheet_id,
                }
                .into());
            }
        }
        if let Some(name) = nav::attr_value(sheet, interner, "name") {
            let name = name?;
            if !names.insert(name.clone()) {
                return Err(SpreadsheetDefect::DuplicateSheetName { part: part(), name }.into());
            }
        }
        let Some(reference) = reference_prefix
            .and_then(|prefix| nav::prefixed_attr_value(sheet, interner, prefix, "id"))
        else {
            continue;
        };
        let reference = reference?;
        if !listed.insert(reference.clone()) {
            return Err(SpreadsheetDefect::DuplicateSheetReference {
                part: part(),
                relationship_id: reference,
            }
            .into());
        }
        listed_order.push(reference);
    }

    // What the relationships offer: every internal relationship whose target part carries one of
    // the three sheet content types. Matching on the *content type* rather than the relationship
    // type keeps this correct in both conformance worlds, whose relationship-type URIs differ while
    // their content types do not — the same reasoning `mjx_pptx::validate` states for `p:sldIdLst`.
    let Some(relationships) = package.relationships_for(Some(workbook_part)) else {
        return Ok(());
    };
    let mut sheet_relationships: HashMap<&str, PartName> = HashMap::new();
    for rel in relationships.iter() {
        if rel.mode == TargetMode::External {
            continue;
        }
        let Ok(target) = nav::resolve_target(workbook_part, &rel.target) else {
            continue; // `mjx-opc`'s defect to report.
        };
        if package
            .content_type_of(&target)
            .and_then(SheetKind::from_content_type)
            .is_some()
        {
            sheet_relationships.insert(rel.id.as_str(), target);
        }
    }

    // An entry naming a relationship that leads somewhere else.
    for reference in &listed_order {
        if sheet_relationships.contains_key(reference.as_str()) {
            continue;
        }
        let Some(rel) = relationships.by_id(reference) else {
            continue; // Undeclared: reported one layer down, as a dangling reference.
        };
        let Ok(target) = nav::resolve_target(workbook_part, &rel.target) else {
            continue;
        };
        let Some(actual) = package.content_type_of(&target) else {
            continue; // A target with no content type is `mjx-opc`'s defect to report.
        };
        return Err(SpreadsheetDefect::SheetEntryTargetIsNotASheet {
            part: part(),
            relationship_id: reference.clone(),
            target_part: target.as_str().to_owned(),
            actual_content_type: actual.to_owned(),
        }
        .into());
    }

    // …and the reverse: a sheet part the list never names.
    for (id, target) in &sheet_relationships {
        if listed.contains(*id) {
            continue;
        }
        return Err(SpreadsheetDefect::UnlistedSheetRelationship {
            part: part(),
            relationship_id: (*id).to_owned(),
            target_part: target.as_str().to_owned(),
        }
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parts::PartKind;

    /// Whether `kind` is one of the SpreadsheetML part kinds — the family
    /// [`SpreadsheetDefect::UnreachableSpreadsheetPart`] is about.
    ///
    /// A test helper, not part of the check: the check works from the content-type *prefix* so that
    /// a SpreadsheetML part this crate does not classify (a revision log, a shared workbook's user
    /// names) is covered too. This is what pins the two descriptions against each other.
    fn is_spreadsheetml_part_kind(kind: PartKind) -> bool {
        kind.content_types()
            .iter()
            .all(|content_type| content_type.starts_with(SPREADSHEETML_CONTENT_TYPE_PREFIX))
    }

    use crate::parts::{
        CONTENT_TYPE_SHARED_STRINGS, CONTENT_TYPE_THEME, CONTENT_TYPE_WORKBOOK,
        CONTENT_TYPE_WORKSHEET, REL_SHARED_STRINGS, REL_THEME, REL_WORKSHEET,
    };
    use mjx_opc::Relationship;

    const WORKBOOK_MARKUP: &str = r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets></workbook>"#;

    /// A one-sheet workbook package, authored from nothing so the sheet-list checks are in scope.
    fn one_sheet_workbook(markup: &str) -> (Package, PartName) {
        let mut package = Package::empty();
        let workbook_part = PartName::new("/xl/workbook.xml").expect("a valid part name");
        package
            .insert_part(
                &workbook_part,
                CONTENT_TYPE_WORKBOOK,
                markup.as_bytes().to_vec(),
            )
            .expect("insert the workbook part");
        package
            .add_relationship(
                None,
                Relationship {
                    id: "rIdWb".to_owned(),
                    rel_type: REL_OFFICE_DOCUMENT.to_owned(),
                    target: "xl/workbook.xml".to_owned(),
                    mode: TargetMode::Internal,
                },
            )
            .expect("relate the workbook from the package root");
        let sheet = PartName::new("/xl/worksheets/sheet1.xml").expect("a valid part name");
        package
            .insert_part(&sheet, CONTENT_TYPE_WORKSHEET, b"<worksheet/>".to_vec())
            .expect("insert the worksheet part");
        package
            .add_relationship(
                Some(&workbook_part),
                Relationship {
                    id: "rId1".to_owned(),
                    rel_type: REL_WORKSHEET.to_owned(),
                    target: "worksheets/sheet1.xml".to_owned(),
                    mode: TargetMode::Internal,
                },
            )
            .expect("relate the worksheet");
        (package, workbook_part)
    }

    /// The baseline this whole module is measured against: a well-formed graph passes.
    ///
    /// Without it, every case below could be green because `check` rejects everything.
    #[test]
    fn a_well_formed_workbook_passes_every_check() {
        let (package, workbook_part) = one_sheet_workbook(WORKBOOK_MARKUP);
        check(&package, &workbook_part).expect("a correct workbook validates");
    }

    /// Retargeting the root relationship away from the workbook is refused, and the message names
    /// both the part and the target.
    #[test]
    fn the_office_document_relationship_must_still_name_the_workbook() {
        let (mut package, workbook_part) = one_sheet_workbook(WORKBOOK_MARKUP);
        package
            .retarget_relationship(
                None,
                "rIdWb",
                "xl/worksheets/sheet1.xml",
                TargetMode::Internal,
            )
            .expect("retarget the root relationship");
        let error =
            check(&package, &workbook_part).expect_err("the root edge no longer leads home");
        let text = error.to_string();
        assert!(text.contains("/xl/workbook.xml"), "{text}");
        assert!(text.contains("xl/worksheets/sheet1.xml"), "{text}");
    }

    /// A shared string table the graph cannot reach is refused, and the message names the part.
    ///
    /// This is the mutation the ticket names: drop the `sharedStrings` relationship and the save
    /// must stop. `Package::validate` alone cannot see it — an orphan is legal OPC — so this is the
    /// check that carries it, for the reason this module's own documentation gives.
    #[test]
    fn an_unreachable_shared_string_table_is_refused() {
        let (mut package, workbook_part) = one_sheet_workbook(WORKBOOK_MARKUP);
        let strings = PartName::new("/xl/sharedStrings.xml").expect("a valid part name");
        package
            .insert_part(&strings, CONTENT_TYPE_SHARED_STRINGS, b"<sst/>".to_vec())
            .expect("insert the shared string table");
        package
            .add_relationship(
                Some(&workbook_part),
                Relationship {
                    id: "rId9".to_owned(),
                    rel_type: REL_SHARED_STRINGS.to_owned(),
                    target: "sharedStrings.xml".to_owned(),
                    mode: TargetMode::Internal,
                },
            )
            .expect("relate it");
        check(&package, &workbook_part).expect("related, so reachable");

        package
            .remove_relationship(Some(&workbook_part), "rId9")
            .expect("drop the sharedStrings relationship");
        let error = check(&package, &workbook_part).expect_err("nothing reaches it now");
        assert!(
            error.to_string().contains("/xl/sharedStrings.xml"),
            "the defect must name the part: {error}"
        );
    }

    /// A stray *non*-SpreadsheetML part is left alone: the narrowing is one family wide, not a
    /// general orphan sweep that would contradict `mjx-opc`.
    ///
    /// The discriminating half of the case above — without it, "unreachable is refused" could be a
    /// blanket rule this crate has no business asserting.
    #[test]
    fn an_unreachable_theme_is_not_a_spreadsheetml_defect() {
        let (mut package, workbook_part) = one_sheet_workbook(WORKBOOK_MARKUP);
        let theme = PartName::new("/xl/theme/theme1.xml").expect("a valid part name");
        package
            .insert_part(&theme, CONTENT_TYPE_THEME, b"<a:theme/>".to_vec())
            .expect("insert an unrelated theme part");
        check(&package, &workbook_part)
            .expect("a theme nothing relates to is dead weight, not a broken workbook");

        // …and relating it changes nothing, which is the other half of "this rule is about one
        // family".
        package
            .add_relationship(
                Some(&workbook_part),
                Relationship {
                    id: "rId8".to_owned(),
                    rel_type: REL_THEME.to_owned(),
                    target: "theme/theme1.xml".to_owned(),
                    mode: TargetMode::Internal,
                },
            )
            .expect("relate it");
        check(&package, &workbook_part).expect("still fine");
    }

    /// An entry pointing at a part that is not a sheet is refused, naming the content type it found.
    #[test]
    fn a_sheet_entry_must_lead_to_a_sheet() {
        let (mut package, workbook_part) = one_sheet_workbook(WORKBOOK_MARKUP);
        // Point `rId1` — the one the sheet list names — at the styles part instead.
        let styles = PartName::new("/xl/styles.xml").expect("a valid part name");
        package
            .insert_part(
                &styles,
                crate::parts::CONTENT_TYPE_STYLES,
                b"<styleSheet/>".to_vec(),
            )
            .expect("insert the styles part");
        package
            .retarget_relationship(
                Some(&workbook_part),
                "rId1",
                "styles.xml",
                TargetMode::Internal,
            )
            .expect("retarget the sheet's relationship");
        // The worksheet part is now unreachable, so ask the sheet-list check directly: this case is
        // about the list, and the reachability case above is about reachability.
        let error = check_sheet_list(&package, &workbook_part).expect_err("rId1 is not a sheet");
        let text = error.to_string();
        assert!(text.contains("rId1"), "{text}");
        assert!(text.contains("/xl/styles.xml"), "{text}");
        assert!(text.contains("spreadsheetml.styles+xml"), "{text}");
    }

    /// A sheet part the workbook relates to but never lists is refused — a tab no consumer shows.
    #[test]
    fn a_sheet_part_the_list_never_names_is_refused() {
        let (mut package, workbook_part) = one_sheet_workbook(WORKBOOK_MARKUP);
        let second = PartName::new("/xl/worksheets/sheet2.xml").expect("a valid part name");
        package
            .insert_part(&second, CONTENT_TYPE_WORKSHEET, b"<worksheet/>".to_vec())
            .expect("insert a second worksheet");
        package
            .add_relationship(
                Some(&workbook_part),
                Relationship {
                    id: "rId2".to_owned(),
                    rel_type: REL_WORKSHEET.to_owned(),
                    target: "worksheets/sheet2.xml".to_owned(),
                    mode: TargetMode::Internal,
                },
            )
            .expect("relate it but never list it");
        let error = check(&package, &workbook_part).expect_err("sheet2 is related and unlisted");
        let text = error.to_string();
        assert!(text.contains("rId2"), "{text}");
        assert!(text.contains("/xl/worksheets/sheet2.xml"), "{text}");
    }

    /// The three identifier spaces §18.2.19 and the list itself require to be unique.
    #[test]
    fn the_sheet_lists_identifier_spaces_must_be_unique() {
        let cases: [(&str, &str); 3] = [
            (
                r#"<sheet name="A" sheetId="1" r:id="rId1"/><sheet name="B" sheetId="1" r:id="rId1"/>"#,
                "sheetId 1",
            ),
            (
                r#"<sheet name="A" sheetId="1" r:id="rId1"/><sheet name="A" sheetId="2" r:id="rId1"/>"#,
                r#"named "A""#,
            ),
            (
                r#"<sheet name="A" sheetId="1" r:id="rId1"/><sheet name="B" sheetId="2" r:id="rId1"/>"#,
                "names relationship rId1 more than once",
            ),
        ];
        for (entries, expected) in cases {
            let markup = format!(
                r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>{entries}</sheets></workbook>"#
            );
            let (package, workbook_part) = one_sheet_workbook(&markup);
            let error = check(&package, &workbook_part).expect_err("a repeated identifier");
            assert!(
                error.to_string().contains(expected),
                "expected a defect mentioning {expected}; got: {error}"
            );
        }
    }

    /// A workbook still holding its container bytes is never faulted for markup it arrived with.
    ///
    /// The scope rule, asserted rather than asserted-in-prose: the very markup that fails above is
    /// accepted when the bytes came from a container instead of from this library.
    #[test]
    fn markup_that_arrived_in_a_container_is_out_of_scope() {
        let duplicated = r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="A" sheetId="1" r:id="rId1"/><sheet name="A" sheetId="1" r:id="rId1"/></sheets></workbook>"#;
        let (package, workbook_part) = one_sheet_workbook(duplicated);
        check(&package, &workbook_part).expect_err("authored: in scope, and broken");

        // Round-trip the same package through a container, so every part is `FromContainer`.
        let bytes = package.save_unchecked().expect("write it out anyway");
        let reopened = Package::open(&bytes).expect("reopen");
        check_sheet_list(&reopened, &workbook_part)
            .expect("container bytes are not this library's markup to fault");
    }

    /// Every SpreadsheetML [`PartKind`] is inside the content-type family the reachability check
    /// uses, and the three non-SpreadsheetML ones are outside it.
    ///
    /// Pins the prefix against the constants rather than restating it: a content type that drifted
    /// out of the family would silently stop being covered.
    #[test]
    fn the_spreadsheetml_family_is_exactly_the_spreadsheetml_kinds() {
        for kind in PartKind::ALL {
            let expected = !matches!(
                kind,
                PartKind::Theme | PartKind::Drawing | PartKind::VmlDrawing
            );
            assert_eq!(
                is_spreadsheetml_part_kind(*kind),
                expected,
                "{kind:?} is on the wrong side of the SpreadsheetML content-type family"
            );
        }
    }
}
