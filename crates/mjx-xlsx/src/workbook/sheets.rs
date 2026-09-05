//! `x:sheets` — the workbook's own list of its sheets, and how each entry reaches a part.
//!
//! # Why the list, and not the relationships, is the sheet order
//!
//! A workbook part relates to its worksheets, chartsheets and dialogsheets through ordinary OPC
//! relationships, and `xl/_rels/workbook.xml.rels` says nothing about which tab comes first. The
//! order — and the name on the tab, and whether the tab is hidden — is in the workbook's own markup:
//! ECMA-376 Part 1 §12.3.24 is explicit that *"the `id` attribute on the `sheet` element shall
//! reference the desired worksheet part"*. So the list is read from `x:sheets`, in document order,
//! and each entry's `r:id` is resolved through the workbook part's relationships.
//!
//! # What this is not
//!
//! [`Sheet`] is a **graph** entry, not `CT_Sheet`. It carries what is needed to find the part and
//! name it to a user, and nothing else; `mjx_sml`'s `workbook` module is where the modelled
//! `CT_Workbook`/`CT_Sheet` land (MJXOFF-100, D06), and `views.rs`, `properties.rs` and
//! `defined_names.rs` beside this file are that child's other homes.

use mjx_ooxml_core::RawDocument;
use mjx_ooxml_types::namespaces::{SHARED_RELATIONSHIP_REFERENCE, SML};
use mjx_ooxml_types::spreadsheetml::SheetState;
use mjx_opc::{Package, PartName, TargetMode};

use crate::error::XlsxError;
use crate::nav;
use crate::parts::SheetKind;

/// One entry of the workbook's `x:sheets` list: a tab, and the part behind it.
///
/// Every field is what the file said, not what it ought to have said. A workbook this crate can open
/// is not necessarily one it would agree to write — [`crate::Workbook::validate`] is where the
/// disagreements are reported, so that a caller can always *read* a broken file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sheet {
    /// The tab's name (`@name`), with XML entities decoded.
    pub name: String,
    /// The sheet's own identifier (`@sheetId`), or `None` if the attribute is absent or is not an
    /// `xsd:unsignedInt`. Required by the schema; absent only in a file no producer here wrote.
    pub sheet_id: Option<u32>,
    /// Whether the tab is shown (`@state`, `ST_SheetState`). Defaults to
    /// [`SheetState::Visible`], which is what the schema declares and what an absent attribute
    /// means.
    pub visibility: SheetState,
    /// The relationship this entry names (`r:id`).
    pub relationship_id: String,
    /// The part that relationship reaches, or `None` if the workbook's `.rels` declares no such
    /// relationship, or declares it `TargetMode="External"`.
    ///
    /// A dangling `r:id` is reported by [`mjx_opc::Package::validate`] over the markup this library
    /// writes; it is *not* a reason to refuse to open a file somebody else wrote.
    pub part: Option<PartName>,
    /// Which of the three sheet kinds the target is, by its content type — or `None` if the target
    /// is missing, or carries a content type that is not one of the three.
    pub kind: Option<SheetKind>,
}

impl Sheet {
    /// Whether this tab is shown in a consumer's tab strip.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.visibility == SheetState::Visible
    }
}

/// What `x:sheets` said, before any of it is resolved against the package.
struct SheetEntry {
    name: String,
    sheet_id: Option<u32>,
    visibility: SheetState,
    relationship_id: String,
}

/// Reads the workbook part's `x:sheets` list and resolves each entry's `r:id`.
///
/// Two passes on purpose: the markup is read into owned entries while the package is borrowed for
/// the tree, and only then is each `r:id` resolved against the workbook's relationships. One pass
/// would need `&Package` and `&mut Package` at once.
///
/// # Errors
/// Returns [`XlsxError`] if the workbook part cannot be read or is not well-formed, or if an
/// attribute is not valid UTF-8.
pub(crate) fn resolve(
    package: &mut Package,
    workbook_part: &PartName,
) -> Result<Vec<Sheet>, XlsxError> {
    let entries = {
        let doc = package.part_tree(workbook_part)?;
        read_entries(doc)?
    };

    let mut sheets = Vec::with_capacity(entries.len());
    for entry in entries {
        let part = package
            .relationships_for(Some(workbook_part))
            .and_then(|rels| rels.by_id(&entry.relationship_id))
            .filter(|rel| rel.mode == TargetMode::Internal)
            .and_then(|rel| nav::resolve_target(workbook_part, &rel.target).ok());
        let kind = part
            .as_ref()
            .and_then(|part| package.content_type_of(part))
            .and_then(SheetKind::from_content_type);
        sheets.push(Sheet {
            name: entry.name,
            sheet_id: entry.sheet_id,
            visibility: entry.visibility,
            relationship_id: entry.relationship_id,
            part,
            kind,
        });
    }
    Ok(sheets)
}

/// Reads `x:workbook/x:sheets/x:sheet` out of a parsed workbook part.
fn read_entries(doc: &RawDocument) -> Result<Vec<SheetEntry>, XlsxError> {
    let interner = &doc.interner;
    let Some(sheets) = nav::child(&doc.root, interner, SML, "sheets") else {
        // Legal: `CT_Workbook`'s `sheets` is `minOccurs="0"`. A workbook with no sheet list has no
        // tabs, which is a thing this crate reads rather than a thing it refuses.
        return Ok(Vec::new());
    };
    // The reader leaves attribute namespaces unresolved, so `r:id` is found through whichever prefix
    // the root binds. A workbook that binds none can carry no relationship reference at all.
    let reference_prefix =
        nav::namespace_prefix(&doc.root, interner, SHARED_RELATIONSHIP_REFERENCE);

    let mut entries = Vec::new();
    for sheet in nav::children(sheets, interner, SML, "sheet") {
        let name = match nav::attr_value(sheet, interner, "name") {
            Some(value) => value?,
            None => String::new(),
        };
        let sheet_id = match nav::attr_value(sheet, interner, "sheetId") {
            Some(value) => value?.parse::<u32>().ok(),
            None => None,
        };
        let visibility = match nav::attr_value(sheet, interner, "state") {
            Some(value) => SheetState::from_wire(&value?).unwrap_or(SheetState::Visible),
            None => SheetState::Visible,
        };
        let relationship_id = match reference_prefix
            .and_then(|prefix| nav::prefixed_attr_value(sheet, interner, prefix, "id"))
        {
            Some(value) => value?,
            None => String::new(),
        };
        entries.push(SheetEntry {
            name,
            sheet_id,
            visibility,
            relationship_id,
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parts::{
        CONTENT_TYPE_CHARTSHEET, CONTENT_TYPE_WORKBOOK, CONTENT_TYPE_WORKSHEET, REL_CHARTSHEET,
        REL_WORKSHEET,
    };
    use mjx_opc::Relationship;

    /// A workbook package with the sheet list `markup` declares, and the relationships `rels`
    /// declares — built from nothing so each case states exactly the graph it is about.
    fn workbook_package(markup: &str, rels: &[(&str, &str, &str, &str)]) -> (Package, PartName) {
        let mut package = Package::empty();
        let workbook_part = PartName::new("/xl/workbook.xml").expect("a valid part name");
        package
            .insert_part(
                &workbook_part,
                CONTENT_TYPE_WORKBOOK,
                markup.as_bytes().to_vec(),
            )
            .expect("insert the workbook part");
        for (id, rel_type, target, content_type) in rels {
            let part = workbook_part.resolve(target).expect("a resolvable target");
            package
                .insert_part(&part, content_type, b"<sheet/>".to_vec())
                .expect("insert a sheet part");
            package
                .add_relationship(
                    Some(&workbook_part),
                    Relationship {
                        id: (*id).to_owned(),
                        rel_type: (*rel_type).to_owned(),
                        target: (*target).to_owned(),
                        mode: TargetMode::Internal,
                    },
                )
                .expect("relate it");
        }
        (package, workbook_part)
    }

    const TWO_SHEETS: &str = r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Revenue &amp; Cost" sheetId="1" r:id="rId1"/><sheet name="Chart" sheetId="4" state="hidden" r:id="rId2"/></sheets></workbook>"#;

    /// The list is read in document order, each entry keeps what the markup said, and each `r:id`
    /// resolves to the part and the kind its content type gives it.
    #[test]
    fn the_sheet_list_is_read_in_document_order_with_each_entry_resolved() {
        let (mut package, workbook_part) = workbook_package(
            TWO_SHEETS,
            &[
                (
                    "rId1",
                    REL_WORKSHEET,
                    "worksheets/sheet1.xml",
                    CONTENT_TYPE_WORKSHEET,
                ),
                (
                    "rId2",
                    REL_CHARTSHEET,
                    "chartsheets/sheet1.xml",
                    CONTENT_TYPE_CHARTSHEET,
                ),
            ],
        );
        let sheets = resolve(&mut package, &workbook_part).expect("read the sheet list");

        assert_eq!(sheets.len(), 2);
        assert_eq!(
            sheets[0].name, "Revenue & Cost",
            "a tab name is user text and comes back unescaped"
        );
        assert_eq!(sheets[0].sheet_id, Some(1));
        assert_eq!(sheets[0].visibility, SheetState::Visible);
        assert!(sheets[0].is_visible());
        assert_eq!(sheets[0].kind, Some(SheetKind::Worksheet));
        assert_eq!(
            sheets[0].part.as_ref().map(PartName::as_str),
            Some("/xl/worksheets/sheet1.xml")
        );

        // Document order, not relationship order and not `@sheetId` order: the second tab's id is 4.
        assert_eq!(sheets[1].name, "Chart");
        assert_eq!(sheets[1].sheet_id, Some(4));
        assert_eq!(sheets[1].visibility, SheetState::Hidden);
        assert!(!sheets[1].is_visible());
        assert_eq!(sheets[1].kind, Some(SheetKind::Chartsheet));
        assert_eq!(
            sheets[1].part.as_ref().map(PartName::as_str),
            Some("/xl/chartsheets/sheet1.xml")
        );
    }

    /// An entry whose `r:id` no relationship declares is read, not rejected — the file still opens,
    /// with the entry reporting no part.
    #[test]
    fn a_dangling_relationship_reference_is_read_rather_than_refused() {
        let (mut package, workbook_part) = workbook_package(
            TWO_SHEETS,
            &[(
                "rId1",
                REL_WORKSHEET,
                "worksheets/sheet1.xml",
                CONTENT_TYPE_WORKSHEET,
            )],
        );
        let sheets = resolve(&mut package, &workbook_part).expect("read the sheet list");
        assert_eq!(sheets.len(), 2);
        assert_eq!(sheets[1].relationship_id, "rId2");
        assert_eq!(sheets[1].part, None, "rId2 is declared nowhere");
        assert_eq!(sheets[1].kind, None);
    }

    /// A workbook with no `x:sheets` at all reads as no sheets — `CT_Workbook` declares the element
    /// `minOccurs="0"`, so this is a legal file rather than a malformed one.
    #[test]
    fn a_workbook_with_no_sheet_list_reads_as_no_sheets() {
        let (mut package, workbook_part) = workbook_package(
            r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"/>"#,
            &[],
        );
        assert!(resolve(&mut package, &workbook_part)
            .expect("read the sheet list")
            .is_empty());
    }

    /// An unknown `@state` token falls back to the schema's own default rather than failing.
    ///
    /// `ST_SheetState` has exactly three values, so a fourth is a file defect; refusing to open the
    /// workbook over it would trade a readable file for an unreadable one, and the original token is
    /// still in the part's bytes and still written back verbatim.
    #[test]
    fn an_unknown_visibility_token_falls_back_to_visible() {
        let (mut package, workbook_part) = workbook_package(
            r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="S" sheetId="1" state="quiteHidden" r:id="rId1"/></sheets></workbook>"#,
            &[(
                "rId1",
                REL_WORKSHEET,
                "worksheets/sheet1.xml",
                CONTENT_TYPE_WORKSHEET,
            )],
        );
        let sheets = resolve(&mut package, &workbook_part).expect("read the sheet list");
        assert_eq!(sheets[0].visibility, SheetState::Visible);

        let saved = package.save().expect("save");
        let reopened = Package::open(&saved).expect("reopen");
        let bytes = reopened
            .part_bytes(&workbook_part)
            .expect("the workbook part is still there");
        assert!(
            String::from_utf8_lossy(bytes).contains(r#"state="quiteHidden""#),
            "reading a token this crate does not recognise must not rewrite it"
        );
    }
}
