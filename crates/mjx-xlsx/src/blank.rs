//! Authoring a workbook from nothing, and creating a part a workbook does not yet have.
//!
//! [`Workbook::open`](crate::Workbook::open) needs a `.xlsx` to exist already. This module is the
//! other half: [`Workbook::blank`](crate::Workbook::blank) writes a complete SpreadsheetML package
//! from code rather than unpacking a committed template, exactly as `mjx_pptx::Presentation::blank`
//! and `mjx_docx::Document::blank` do, so the markup is markup this project can explain and the same
//! schema gate that validates an edited workbook validates this one.
//!
//! # There is no schema-valid empty workbook — so `blank()` is not a shell
//!
//! `CT_Workbook` has nineteen slots and exactly one is mandatory:
//!
//! ```xml
//! <xsd:element name="sheets" type="CT_Sheets" minOccurs="1" maxOccurs="1"/>
//! <!-- and CT_Sheets: <xsd:element name="sheet" … minOccurs="1" maxOccurs="unbounded"/> -->
//! ```
//!
//! `sheets` is required and it requires at least one `sheet`, and `CT_Worksheet` in turn requires a
//! `sheetData`. So a blank workbook necessarily authors **a worksheet part too** — its content type,
//! its relationship from `xl/workbook.xml`, and an empty `sheetData` — plus `xl/styles.xml` (or
//! every `@fontId`, `@fillId`, `@borderId` and `c@s` in the file dangles) and
//! `xl/sharedStrings.xml`. It is not the minimal-shell operation the name suggests, and that is why
//! Excel is the least forgiving of the three formats about structural detail.
//!
//! # The markup is `mjx-sml`'s, not this crate's
//!
//! Every byte of it comes from [`mjx_sml::write::WorkbookPackage`]. That is deliberate and it is the
//! whole point of the crate split: `mjx-chart` embeds a real workbook inside a `.pptx` and cannot
//! depend on this crate (that edge points upward), so the writer lives one tier down and both
//! callers use the same one. A second writer here would be the duplicate MJXOFF-99 exists to remove,
//! rebuilt on the day it was removed.
//!
//! What this module adds is the seam: the [`Package`] the writer produced goes straight into
//! [`Workbook::from_package`](crate::Workbook::from_package), so a workbook built from nothing is
//! resolved by the same code that resolves one read off disk rather than surviving as a special case
//! nothing checks.
//!
//! # The rule this module is written to
//!
//! **When a part is authored on demand, write back a value that was READ from that part — never a
//! freshly constructed root written over a parsed one.**
//!
//! This is not hypothetical. In `mjx-docx`, `create_footnotes_part` parsed its template and then
//! wrote a fresh `Footnotes::blank()` over the root. A freshly built value has no ancestor to
//! inherit an `xmlns:w` declaration from, so the declaration was discarded, and every footnote
//! vanished the next time the document was opened. The gate was green throughout, because it
//! asserted on the model that had just been built rather than on the file that came back.
//!
//! The correct shape is:
//!
//! ```text
//! insert_part(part, content_type, minimal_bytes_carrying_the_namespace_declaration)
//! let root = package.part_tree_mut(&part)?;
//! let mut model = X::from_xml(root, interner)?;   // read what is there
//! model.mutate(...);                              // change it
//! model.write_back(root, interner);               // write back what was read
//! ```
//!
//! A freshly built *child* inserted into a value that *was* read from the root is fine; a freshly
//! built *root* is the bug. Every authored part in [`mjx_sml::write`] is built that way — each is
//! seeded as bytes carrying its own `xmlns`, parsed, and only then modelled — and
//! [`crate::Workbook::add_sheet`], the one place *this* crate authors a part, does the same.
//!
//! **And assert on the reopened file, not on the model just built.** `tests/blank.rs` opens the
//! bytes `blank()` returned and asserts against those, including a raw-byte assertion that each
//! authored part still declares the SpreadsheetML namespace — so a "fix" which merely made the
//! reader more forgiving would not satisfy it.

use mjx_opc::doc_props::{CoreProperties, ExtendedProperties};
use mjx_opc::Package;
use mjx_sml::write::WorkbookPackage;

use crate::error::XlsxError;
use crate::workbook::Workbook;

impl Workbook {
    /// A complete, valid workbook with one visible tab named `Sheet1` and no cells in it.
    ///
    /// The package holds `[Content_Types].xml`, `_rels/.rels`, `xl/workbook.xml`,
    /// `xl/_rels/workbook.xml.rels`, `xl/worksheets/sheet1.xml`, `xl/sharedStrings.xml`,
    /// `xl/styles.xml`, `docProps/core.xml` and `docProps/app.xml` — see this module's own
    /// documentation for why none of the first five is optional.
    ///
    /// Document properties are written with every field absent, which is schema-valid and
    /// deterministic (`CT_CoreProperties` and `CT_Properties` are `xs:all` groups with every child
    /// `minOccurs="0"`). [`blank_with_properties`](Self::blank_with_properties) is the same
    /// constructor for a caller who wants to set title, creator, created/modified or the application
    /// name.
    ///
    /// Deterministic: two calls produce byte-identical containers, because nothing here reads a
    /// clock or a random number.
    ///
    /// # Errors
    /// [`XlsxError::Sml`] if the writer refuses a part — unreachable, since every part it authors is
    /// built from constants — or [`XlsxError`] if the resulting package does not resolve, which
    /// would mean this constructor had written a workbook it could not itself open.
    pub fn blank() -> Result<Self, XlsxError> {
        Self::blank_with_properties(&CoreProperties::default(), &ExtendedProperties::default())
    }

    /// [`blank`](Self::blank) with the two document-property parts filled in.
    ///
    /// # Errors
    /// As [`blank`](Self::blank).
    pub fn blank_with_properties(
        core: &CoreProperties,
        extended: &ExtendedProperties,
    ) -> Result<Self, XlsxError> {
        Self::from_package(blank_package(core, extended)?)
    }
}

/// The package [`Workbook::blank`] resolves.
///
/// Separate from the constructor so that a caller who wants the container bytes without resolving
/// them — a test comparing part lists, above all — does not have to go through a `Workbook` that
/// would then have to be saved again.
///
/// # Errors
/// [`XlsxError::Sml`] if the writer refuses a part.
pub(crate) fn blank_package(
    core: &CoreProperties,
    extended: &ExtendedProperties,
) -> Result<Package, XlsxError> {
    let mut writer = WorkbookPackage::new()?;
    writer.set_document_properties(core.clone(), extended.clone());
    Ok(writer.to_package()?)
}
