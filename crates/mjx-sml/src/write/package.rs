//! The whole `.xlsx` package, authored: the parts, the content types and the relationships.
//!
//! # Why this is in `mjx-sml` and not in `mjx-xlsx`
//!
//! Because `mjx-chart` has to reach it. A PowerPoint chart embeds a real workbook package at
//! `/ppt/embeddings/*.xlsx` — the package **Edit Data** opens — and a Word document that carries a
//! chart does the same, so a chart needs a SpreadsheetML *writer* before it needs anything Excel.
//! `mjx-chart` is rank 2.2 and `mjx-xlsx` is rank 3.0, so `mjx-chart → mjx-xlsx` points **upward**
//! and `CLAUDE.md` forbids it; `mjx-chart → mjx-sml` (2.2 → 2.1) points down.
//! `xtask/tests/layering.rs` checks that rather than trusting this paragraph, and
//! `crates/mjx-sml/tests/package_writer.rs` proves the writer works with nothing above it by using
//! it from a crate whose dependency graph contains no `mjx-xlsx` at all.
//!
//! That is the whole reason MJXOFF-132 created this crate, and it is what makes MJXOFF-99 a
//! *deletion* of `crates/mjx-chart/src/workbook.rs` rather than a migration of it.
//!
//! # The part graph this writes
//!
//! ```text
//! /_rels/.rels                       rId1 officeDocument -> xl/workbook.xml
//!                                    rId2 core-properties -> docProps/core.xml     (optional)
//!                                    rId3 extended-properties -> docProps/app.xml  (optional)
//! /xl/_rels/workbook.xml.rels        rId1..rIdN worksheet -> worksheets/sheetN.xml
//!                                    rId(N+1)  styles     -> styles.xml
//!                                    rId(N+2)  sharedStrings -> sharedStrings.xml
//! ```
//!
//! The worksheet relationships come **first** within `xl/_rels/workbook.xml.rels` because each
//! `sheet@r:id` names one, and reading a package back is cheaper when the ids a caller sees are the
//! ones the sheet list wrote. Nothing depends on the order — a relationship is found by id, never by
//! position — and `crates/mjx-sml/src/workbook/sheets.rs` says why at length.
//!
//! # What is deliberately not written
//!
//! * **A theme.** No schema or OPC rule requires one in a SpreadsheetML package, and
//!   `mjx_chart::EmbeddedWorkbook` has shipped without one through every release — its packages open
//!   in PowerPoint and in LibreOffice. Authoring one here would put a **third** hand-written
//!   `a:theme` in this workspace, beside `mjx-pptx`'s, on the very child whose premise is that a
//!   duplicated markup writer is a debt. A caller that wants one relates it itself.
//! * **`calcChain.xml`.** There is no formula authoring here (MJXOFF-115 is that), and a calculation
//!   chain that disagrees with the formulas is worse than none — Excel rebuilds it.
//! * **`fileVersion`, `workbookPr`, `bookViews`, `calcPr`.** All optional; see
//!   `crates/mjx-sml/src/write/workbook.rs`.
//!
//! Document properties **are** written when [`set_document_properties`](WorkbookPackage::set_document_properties)
//! asks for them, through [`mjx_opc::doc_props`] — the one module every format's `blank` constructor
//! shares, so this adds no fourth copy either. They are off by default because
//! `EmbeddedWorkbook` wrote none and the parity gate compares part lists.

use mjx_opc::doc_props::{self, CoreProperties, ExtendedProperties};
use mjx_opc::{OpcError, Package, PartName, Relationship, TargetMode};

use crate::address::CellReference;
use crate::cells::CellValue;
use crate::error::SmlError;
use crate::font::FontProperties;
use crate::strings::SharedStringTable;

use super::constants::{
    worksheet_part_name, worksheet_relationship_target, CONTENT_TYPE_SHARED_STRINGS,
    CONTENT_TYPE_STYLES, CONTENT_TYPE_WORKBOOK, CONTENT_TYPE_WORKSHEET, DEFAULT_SHEET_NAME,
    REL_OFFICE_DOCUMENT, REL_SHARED_STRINGS, REL_STYLES, REL_WORKSHEET, SHARED_STRINGS_PART,
    STYLES_PART, WORKBOOK_PART,
};
use super::sheet::AuthoredWorksheet;
use super::style_specs::{BorderSpec, CellFormatSpec, PatternFillSpec};
use super::stylesheet::{AuthoredStylesheet, CellFormatTarget};
use super::workbook::AuthoredWorkbook;

/// A cell value stated by an **author**, before there is a shared-string table to point into.
///
/// [`CellValue`] is the wire shape: `SharedString(u32)` is an index, and a caller holding text has
/// to intern it first. This is the shape a caller actually has — text, a number, a flag — and
/// [`WorkbookPackage::push_row`] does the interning.
///
/// # Nothing is written for a value there is nothing to write for
///
/// [`Blank`](Self::Blank) writes **no cell**, and so does a [`Number`](Self::Number) that is not
/// finite. Both are what `mjx_chart::EmbeddedWorkbook` did, and both are right for a grid: a chart's
/// header row starts with an empty corner, and a series with no value at a category has no data
/// point there. `NaN` and the infinities have no numeric spelling in SpreadsheetML at all — see
/// [`SmlError::UnrepresentableNumber`], which is what the *cell-at-a-time* door
/// ([`WorkbookPackage::set_cell_value`]) returns for them instead, because a caller naming one cell
/// is stating a value rather than laying out a grid.
#[derive(Debug, Clone, PartialEq)]
pub enum AuthoredCellValue {
    /// Nothing is written at this position.
    Blank,
    /// A number (`<c r="B2"><v>19.2</v></c>`). No `t`: `n` is the schema default and a cell that
    /// would not have carried the attribute must not gain it.
    Number(f64),
    /// Text, written through the shared-string table (`<c r="A2" t="s"><v>0</v></c>`).
    SharedText(String),
    /// Text stored in the cell itself (`t="inlineStr"`), for a value that is not worth a table
    /// entry.
    InlineText(String),
    /// A boolean (`t="b"`, `1` or `0`).
    Boolean(bool),
    /// An error code (`t="e"`) — `#DIV/0!`, `#N/A`.
    Error(String),
}

/// A SpreadsheetML package under construction: its sheets, its shared strings, its styles.
///
/// Build one with [`new`](Self::new) (one sheet, named `Sheet1`, and the styles skeleton), fill it,
/// then serialize with [`to_package_bytes`](Self::to_package_bytes) or take the assembled
/// [`Package`] with [`to_package`](Self::to_package).
#[derive(Debug)]
pub struct WorkbookPackage {
    workbook: AuthoredWorkbook,
    sheets: Vec<AuthoredWorksheet>,
    shared_strings: SharedStringTable,
    styles: AuthoredStylesheet,
    document_properties: Option<(CoreProperties, ExtendedProperties)>,
}

impl WorkbookPackage {
    /// A package with one sheet named `Sheet1`, an empty shared-string table and the styles
    /// skeleton — the smallest thing that is a valid workbook.
    ///
    /// One sheet and not zero, because `CT_Workbook` declares `sheets` `minOccurs="1"` and
    /// `CT_Sheets` declares `sheet` `minOccurs="1"`. See `crates/mjx-sml/src/write/workbook.rs`.
    ///
    /// # Errors
    /// [`SmlError`] if one of the three seeded parts does not parse back into its model — none is
    /// reachable, because every seed is a literal in this crate.
    pub fn new() -> Result<Self, SmlError> {
        Self::with_sheet_named(DEFAULT_SHEET_NAME)
    }

    /// [`new`](Self::new) with the first tab named something other than `Sheet1`.
    ///
    /// # Errors
    /// As [`new`](Self::new).
    pub fn with_sheet_named(name: &str) -> Result<Self, SmlError> {
        let mut package = Self {
            workbook: AuthoredWorkbook::new()?,
            sheets: Vec::new(),
            shared_strings: SharedStringTable::authored(None)?,
            styles: AuthoredStylesheet::skeleton()?,
            document_properties: None,
        };
        package.add_sheet(name)?;
        Ok(package)
    }

    /// Appends a tab named `name` and answers its index.
    ///
    /// # Errors
    /// [`SmlError`] if the worksheet seed does not parse — unreachable, as [`new`](Self::new).
    pub fn add_sheet(&mut self, name: &str) -> Result<usize, SmlError> {
        let index = self.sheets.len();
        self.sheets.push(AuthoredWorksheet::new(name)?);
        self.workbook
            .push_sheet(name, &sheet_relationship_id(index));
        Ok(index)
    }

    /// How many tabs the package holds.
    #[must_use]
    pub fn sheet_count(&self) -> usize {
        self.sheets.len()
    }

    /// The tab at `index`, or `None`.
    #[must_use]
    pub fn sheet(&self, index: usize) -> Option<&AuthoredWorksheet> {
        self.sheets.get(index)
    }

    /// The tab at `index`, mutably — the door to [`AuthoredWorksheet::part_mut`] and the
    /// twenty-six worksheet slots this writer does not author itself.
    pub fn sheet_mut(&mut self, index: usize) -> Option<&mut AuthoredWorksheet> {
        self.sheets.get_mut(index)
    }

    /// Renames the tab at `index`, in `xl/workbook.xml` and in this writer's own record of it.
    ///
    /// # Errors
    /// [`SmlError::SheetIndexOutOfRange`] if `index` names no tab.
    pub fn rename_sheet(&mut self, index: usize, name: &str) -> Result<(), SmlError> {
        let sheets = self.sheets.len();
        let sheet = self
            .sheets
            .get_mut(index)
            .ok_or(SmlError::SheetIndexOutOfRange { index, sheets })?;
        sheet.set_name(name);
        self.workbook.rename_sheet(index, name);
        Ok(())
    }

    /// The shared-string table, for a caller reading back what was interned.
    #[must_use]
    pub fn shared_strings(&self) -> &SharedStringTable {
        &self.shared_strings
    }

    /// The index `text` has in the shared-string table, appending it in **first-use order** if it is
    /// new.
    ///
    /// First-use order, not sorted order: `sharedStrings.xml` is a list a cell indexes into, so the
    /// order is part of the file's meaning and re-sorting it would repoint every `t="s"` cell.
    ///
    /// # Errors
    /// [`SmlError::PackedStoreTooLarge`] past the table's four-gigabyte byte space.
    pub fn intern_shared_string(&mut self, text: &str) -> Result<u32, SmlError> {
        self.shared_strings.intern(text)
    }

    /// The styles part, for a caller reading back what was authored.
    #[must_use]
    pub fn styles(&self) -> &AuthoredStylesheet {
        &self.styles
    }

    /// Appends a font to `xl/styles.xml` and answers its `@fontId`.
    ///
    /// # Errors
    /// As [`AuthoredStylesheet::append_font`].
    pub fn append_font(&mut self, properties: &FontProperties) -> Result<u32, SmlError> {
        self.styles.append_font(properties)
    }

    /// Appends a pattern fill and answers its `@fillId`.
    pub fn append_pattern_fill(&mut self, spec: &PatternFillSpec) -> u32 {
        self.styles.append_pattern_fill(spec)
    }

    /// Appends a border and answers its `@borderId`.
    pub fn append_border(&mut self, spec: &BorderSpec) -> u32 {
        self.styles.append_border(spec)
    }

    /// Appends an `xf` to `cellXfs` or to `cellStyleXfs` and answers its index there.
    ///
    /// A `cellXfs` index is what [`set_cell_style`](Self::set_cell_style) takes.
    pub fn append_cell_format(&mut self, target: CellFormatTarget, spec: &CellFormatSpec) -> u32 {
        self.styles.append_cell_format(target, spec)
    }

    /// Sets one cell's value on the tab at `index`, in its wire shape.
    ///
    /// # Errors
    /// [`SmlError::SheetIndexOutOfRange`] if `index` names no tab, or whatever the cell store
    /// refuses — [`SmlError::UnrepresentableNumber`] for `NaN` and the infinities above all.
    pub fn set_cell_value(
        &mut self,
        index: usize,
        reference: CellReference,
        value: CellValue<'_>,
    ) -> Result<(), SmlError> {
        let sheets = self.sheets.len();
        self.sheets
            .get_mut(index)
            .ok_or(SmlError::SheetIndexOutOfRange { index, sheets })?
            .set_cell_value(reference, value)
    }

    /// Sets one cell's `cellXfs` index (`c@s`) on the tab at `index`, creating a blank cell if
    /// there is none. `None` removes the attribute.
    ///
    /// # Errors
    /// As [`set_cell_value`](Self::set_cell_value).
    pub fn set_cell_style(
        &mut self,
        index: usize,
        reference: CellReference,
        style: Option<u32>,
    ) -> Result<(), SmlError> {
        let sheets = self.sheets.len();
        self.sheets
            .get_mut(index)
            .ok_or(SmlError::SheetIndexOutOfRange { index, sheets })?
            .set_cell_style(reference, style)
    }

    /// Appends a row of cells to the tab at `index`, starting at column `A` of the row after the
    /// last one written, and answers the one-based row number it landed on.
    ///
    /// **This is the door a grid goes through**, and the one
    /// [`mjx_chart::EmbeddedWorkbook::to_package_bytes`](https://docs.rs/mjx-chart) is replaced by:
    /// text is interned into the shared-string table in first-use order, a
    /// [`Blank`](AuthoredCellValue::Blank) and a non-finite number write nothing at all, and a row
    /// with nothing in it writes no `<row>` either.
    ///
    /// # Errors
    /// [`SmlError::SheetIndexOutOfRange`] if `index` names no tab,
    /// [`SmlError::PackedStoreTooLarge`] past a packed store's byte space, or
    /// [`SmlError::Address`] if the row would fall outside the grid.
    pub fn push_row(&mut self, index: usize, cells: &[AuthoredCellValue]) -> Result<u32, SmlError> {
        let sheets = self.sheets.len();
        if index >= sheets {
            return Err(SmlError::SheetIndexOutOfRange { index, sheets });
        }
        let row = self.next_row_number(index);
        for (column, cell) in cells.iter().enumerate() {
            let column = u16::try_from(column).map_err(|_| crate::AddressError::ColumnOutOfGrid)?;
            let reference = CellReference::relative(column, row)?;
            match cell {
                AuthoredCellValue::Blank => {}
                AuthoredCellValue::Number(number) if !number.is_finite() => {}
                AuthoredCellValue::Number(number) => {
                    self.set_cell_value(index, reference, CellValue::Number(*number))?;
                }
                AuthoredCellValue::SharedText(text) => {
                    let entry = self.intern_shared_string(text)?;
                    self.set_cell_value(index, reference, CellValue::SharedString(entry))?;
                }
                AuthoredCellValue::InlineText(text) => {
                    self.set_cell_value(index, reference, CellValue::InlineString(text))?;
                }
                AuthoredCellValue::Boolean(flag) => {
                    self.set_cell_value(index, reference, CellValue::Boolean(*flag))?;
                }
                AuthoredCellValue::Error(code) => {
                    self.set_cell_value(index, reference, CellValue::Error(code))?;
                }
            }
        }
        Ok(row.saturating_add(1))
    }

    /// Writes `docProps/core.xml` and `docProps/app.xml` into the package, and relates both from the
    /// package root.
    ///
    /// Off until this is called, because `mjx_chart::EmbeddedWorkbook` wrote neither and the parity
    /// gate compares part lists. `mjx_xlsx::Workbook::blank` calls it: MJXOFF-149 decided this
    /// project authors document properties, and every file real Office writes has them.
    pub fn set_document_properties(&mut self, core: CoreProperties, extended: ExtendedProperties) {
        self.document_properties = Some((core, extended));
    }

    /// Recomputes every sheet's `x:dimension` from the cells it actually holds.
    ///
    /// **The caller's ask, never implicit** — the same rule
    /// [`WorksheetPart::recompute_dimension`](crate::WorksheetPart::recompute_dimension) states. A
    /// package writer usually wants it, because a sheet it just authored has no cached box at all,
    /// and a `dimension` is what Excel sizes its scroll bars from.
    pub fn recompute_dimensions(&mut self) {
        for sheet in &mut self.sheets {
            sheet.recompute_dimension();
        }
    }

    /// Assembles the package: every part, its content type, and every relationship.
    ///
    /// `&mut self` because two of the three modelled parts write back through
    /// [`ToXml::write_back`](mjx_ooxml_core::ToXml::write_back), which needs their interners
    /// mutably. Calling it twice produces byte-identical packages; nothing here is consumed.
    ///
    /// # Errors
    /// [`SmlError::Opc`] if the packaging layer refuses a part name, a content type or a
    /// relationship. Every one of them is a constant in this crate, so in practice this cannot fail
    /// — it is a `Result` because the packaging API is fallible, not because there is a failure mode
    /// to handle.
    pub fn to_package(&mut self) -> Result<Package, SmlError> {
        let mut package = Package::empty();
        let workbook_part = part_name(WORKBOOK_PART)?;

        package.insert_part(
            &workbook_part,
            CONTENT_TYPE_WORKBOOK,
            self.workbook.to_part_bytes(),
        )?;
        for (index, sheet) in self.sheets.iter().enumerate() {
            let part = part_name(&worksheet_part_name(index))?;
            package.insert_part(&part, CONTENT_TYPE_WORKSHEET, sheet.to_part_bytes())?;
        }
        // Shared strings before styles, which is the order `[Content_Types].xml` then lists the two
        // overrides in — and the order `mjx_chart::EmbeddedWorkbook` wrote, so that the part is
        // byte-identical to the one it produced. Nothing reads a content-type list positionally; the
        // parity gate compares bytes, and a gate that had to normalise before comparing would be a
        // weaker gate.
        package.insert_part(
            &part_name(SHARED_STRINGS_PART)?,
            CONTENT_TYPE_SHARED_STRINGS,
            self.shared_strings.to_part_bytes(),
        )?;
        package.insert_part(
            &part_name(STYLES_PART)?,
            CONTENT_TYPE_STYLES,
            self.styles.to_part_bytes(),
        )?;

        add_relationship(
            &mut package,
            None,
            "rId1",
            REL_OFFICE_DOCUMENT,
            "xl/workbook.xml",
        )?;
        if let Some((core, extended)) = &self.document_properties {
            package.insert_part(
                &part_name(doc_props::CORE_PROPERTIES_PART)?,
                doc_props::CORE_PROPERTIES_CONTENT_TYPE,
                doc_props::core_xml(core),
            )?;
            package.insert_part(
                &part_name(doc_props::EXTENDED_PROPERTIES_PART)?,
                doc_props::EXTENDED_PROPERTIES_CONTENT_TYPE,
                doc_props::extended_xml(extended),
            )?;
            add_relationship(
                &mut package,
                None,
                "rId2",
                doc_props::CORE_PROPERTIES_REL_TYPE,
                "docProps/core.xml",
            )?;
            add_relationship(
                &mut package,
                None,
                "rId3",
                doc_props::EXTENDED_PROPERTIES_REL_TYPE,
                "docProps/app.xml",
            )?;
        }

        for index in 0..self.sheets.len() {
            add_relationship(
                &mut package,
                Some(&workbook_part),
                &sheet_relationship_id(index),
                REL_WORKSHEET,
                &worksheet_relationship_target(index),
            )?;
        }
        let next = self.sheets.len();
        add_relationship(
            &mut package,
            Some(&workbook_part),
            &relationship_id(next),
            REL_STYLES,
            "styles.xml",
        )?;
        add_relationship(
            &mut package,
            Some(&workbook_part),
            &relationship_id(next + 1),
            REL_SHARED_STRINGS,
            "sharedStrings.xml",
        )?;
        Ok(package)
    }

    /// The package, serialized to container bytes.
    ///
    /// # Errors
    /// As [`to_package`](Self::to_package), plus [`SmlError::Opc`] if the ZIP writer fails.
    pub fn to_package_bytes(&mut self) -> Result<Vec<u8>, SmlError> {
        Ok(self.to_package()?.save()?)
    }

    /// The row number a new row appended to the tab at `index` gets: one past the last populated
    /// one, zero-based.
    fn next_row_number(&self, index: usize) -> u32 {
        let Some(sheet) = self.sheets.get(index) else {
            return 0;
        };
        sheet
            .part()
            .rows()
            .filter_map(|row| row.number())
            .max()
            .unwrap_or(0)
    }
}

/// The relationship id of the `index`-th worksheet, from `xl/workbook.xml`.
fn sheet_relationship_id(index: usize) -> String {
    relationship_id(index)
}

/// `rId1`, `rId2`, … for a zero-based position.
fn relationship_id(index: usize) -> String {
    format!("rId{}", index.saturating_add(1))
}

/// Parses a part name that is a constant in this crate.
///
/// A failure is a bug here rather than bad input, so it surfaces as a packaging error rather than a
/// panic — the same shape `mjx_chart::EmbeddedWorkbook` used, and for the same reason.
fn part_name(name: &str) -> Result<PartName, SmlError> {
    PartName::new(name)
        .map_err(|_| SmlError::Opc(OpcError::Malformed(format!("invalid part name: {name}"))))
}

/// Adds one relationship, keeping the call sites above readable.
fn add_relationship(
    package: &mut Package,
    source: Option<&PartName>,
    id: &str,
    rel_type: &str,
    target: &str,
) -> Result<(), SmlError> {
    package.add_relationship(
        source,
        Relationship {
            id: id.to_owned(),
            rel_type: rel_type.to_owned(),
            target: target.to_owned(),
            mode: TargetMode::Internal,
        },
    )?;
    Ok(())
}
