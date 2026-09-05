//! `xl/workbook.xml`, authored: the sheet list, and nothing else.
//!
//! # There is no schema-valid empty workbook
//!
//! `CT_Workbook` has nineteen slots and **exactly one** of them is not `minOccurs="0"`:
//!
//! ```xml
//! <xsd:element name="sheets" type="CT_Sheets" minOccurs="1" maxOccurs="1"/>
//!
//! <xsd:complexType name="CT_Sheets">
//!   <xsd:sequence>
//!     <xsd:element name="sheet" type="CT_Sheet" minOccurs="1" maxOccurs="unbounded"/>
//!   </xsd:sequence>
//! </xsd:complexType>
//! ```
//!
//! `sheets` is mandatory, and it in turn requires **at least one** `sheet`. So a workbook with no
//! tab is invalid, and "author a blank workbook" is not the minimal-shell operation the name
//! suggests: it necessarily authors a worksheet part too, its content type, and the relationship
//! from here that names it. `fileVersion`, `workbookPr`, `bookViews` and `calcPr` are all optional
//! and none of them is written — every byte this crate emits is a byte it can explain.
//!
//! # `r:id` is what names the part
//!
//! Not `@sheetId`, not the position in the list. `crates/mjx-sml/src/workbook/sheets.rs` sets that
//! out in full; the consequence here is that the relationship-reference namespace has to be **bound
//! on the root element** before any `sheet` can carry an `r:id`, which is why the seed declares
//! `xmlns:r` and why [`AuthoredWorkbook::relationship_prefix`] answers `r`.

use mjx_ooxml_core::{RawDocument, ToXml};
use mjx_ooxml_types::namespaces::{SHARED_RELATIONSHIP_REFERENCE, SML};

use crate::error::SmlError;
use crate::workbook::{SheetEntry, SheetList, WorkbookPart};

use super::constants::{WORKBOOK_PART, XML_DECLARATION};

/// The prefix this crate binds the relationship-reference namespace to when it authors a workbook.
///
/// The producer's choice, not the schema's — every file this project has read writes `r`, and none
/// of them is obliged to. It is a constant here so that the seed's `xmlns:r` and the `r:id` written
/// on each `sheet` cannot drift apart.
const RELATIONSHIP_PREFIX: &str = "r";

/// `xl/workbook.xml` under construction: the parsed part, and the document its names live in.
#[derive(Debug)]
pub struct AuthoredWorkbook {
    document: RawDocument,
    part: WorkbookPart,
}

impl AuthoredWorkbook {
    /// The bytes a workbook part is seeded from: the declaration, a root binding both namespaces,
    /// and an empty `sheets`.
    fn seed_bytes() -> Vec<u8> {
        format!(
            r#"{XML_DECLARATION}<workbook xmlns="{sml}" xmlns:{RELATIONSHIP_PREFIX}="{rel}"><sheets/></workbook>"#,
            sml = SML.transitional,
            rel = SHARED_RELATIONSHIP_REFERENCE.transitional,
        )
        .into_bytes()
    }

    /// A workbook with an empty sheet list.
    ///
    /// Invalid on its own — see this module's documentation — until
    /// [`push_sheet`](Self::push_sheet) has been called at least once.
    ///
    /// # Errors
    /// [`SmlError::Xml`] if the seed does not parse, [`SmlError::Model`] if it does not match
    /// `CT_Workbook`, or [`SmlError::AuthoredPartSeedRejected`] if its root is not an `x:workbook`.
    /// None is reachable — the seed is a literal in this file.
    pub fn new() -> Result<Self, SmlError> {
        let document = mjx_xml::fidelity::parse(&Self::seed_bytes())?;
        let part = WorkbookPart::read_root(&document.root, &document.interner)?.ok_or(
            SmlError::AuthoredPartSeedRejected {
                part: WORKBOOK_PART,
            },
        )?;
        Ok(Self { document, part })
    }

    /// The modelled part, for a caller reading back what was authored.
    #[must_use]
    pub fn part(&self) -> &WorkbookPart {
        &self.part
    }

    /// The prefix the authored root binds the relationship-reference namespace to.
    #[must_use]
    pub fn relationship_prefix(&self) -> &'static str {
        RELATIONSHIP_PREFIX
    }

    /// How many tabs the sheet list names.
    #[must_use]
    pub fn sheet_count(&self) -> usize {
        self.part.sheets().map_or(0, SheetList::len)
    }

    /// Appends a tab: its name, its `@sheetId` and the relationship that names its part.
    ///
    /// `@sheetId` is `index + 1`, which is what a producer writes for a workbook whose sheets have
    /// never been deleted and reordered. It is **not** an identifier of the part — that is `r:id`
    /// alone — and nothing in this crate reads it back to find one.
    pub fn push_sheet(&mut self, name: &str, relationship_id: &str) {
        let sheet_number = u32::try_from(self.sheet_count().saturating_add(1)).unwrap_or(u32::MAX);
        let RawDocument { interner, .. } = &mut self.document;
        let mut entry = SheetEntry::new(interner, None);
        entry.set_name(interner, Some(name));
        entry.set_sheet_id(interner, Some(sheet_number));
        entry.set_relationship_id(interner, RELATIONSHIP_PREFIX, relationship_id);

        let mut list = match self.part.sheets() {
            Some(list) => list.clone(),
            None => SheetList::new(interner, None),
        };
        list.push(entry);
        self.part.set_sheets(Some(list));
    }

    /// Renames the tab at `index`, leaving the relationship it names untouched — a tab's name and
    /// the part behind it are independent.
    ///
    /// Answers whether there was a tab at `index`.
    pub fn rename_sheet(&mut self, index: usize, name: &str) -> bool {
        let RawDocument { interner, .. } = &mut self.document;
        let Some(list) = self.part.sheets_mut() else {
            return false;
        };
        let Some(entry) = list.entry_mut(index) else {
            return false;
        };
        entry.set_name(interner, Some(name));
        true
    }

    /// The whole part as bytes, with the model written back over the root it was read from.
    ///
    /// `&mut self` for the reason `crates/mjx-sml/src/write/stylesheet.rs` gives on its own writer:
    /// [`ToXml::write_back`] needs the interner mutably. Calling it twice produces identical bytes.
    pub fn write_into(&mut self, out: &mut Vec<u8>) {
        let RawDocument { interner, root, .. } = &mut self.document;
        self.part.write_back(root, interner);
        mjx_xml::fidelity::serialize(&self.document, out);
    }

    /// The whole part as bytes. See [`write_into`](Self::write_into).
    pub fn to_part_bytes(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        self.write_into(&mut out);
        out
    }
}
