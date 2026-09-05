//! The authoring surface: adding a tab, and writing the two parts a value or a format lands in.
//!
//! # What is here and what is not
//!
//! Reading is [`crate::workbook`], [`crate::worksheet::grid`] and
//! [`crate::worksheet::formatting`]; *editing one cell* is `grid`'s
//! [`Workbook::set_cell_value`](crate::Workbook::set_cell_value), which came with MJXOFF-102. This
//! file is the rest of what a caller needs to build a workbook up rather than change one:
//!
//! | operation | where it lands |
//! |---|---|
//! | [`add_sheet`](Workbook::add_sheet) | a new worksheet part, its content type, its relationship, and a `sheet` entry in `xl/workbook.xml` |
//! | [`set_cell_style`](Workbook::set_cell_style) | `c@s` on one cell of one worksheet part |
//! | [`intern_shared_string`](Workbook::intern_shared_string) | `xl/sharedStrings.xml` |
//! | [`append_font`](Workbook::append_font) and the three beside it | `xl/styles.xml` |
//!
//! Renaming a tab is [`Workbook::rename_sheet`](crate::Workbook::rename_sheet), which MJXOFF-100
//! wrote; it is not repeated here.
//!
//! # Concrete types, and why
//!
//! No method here takes a closure, and none returns a value that borrows the workbook. That is the
//! rule A9 applied to `mjx_ooxml::Deck` and it is what makes this surface projectable into Python
//! and TypeScript unchanged — MJXOFF-137 (D20) is what does the projecting, and a signature it
//! cannot express would be found only then.
//!
//! The four `append_*` methods therefore take [`mjx_sml::write`]'s plain-data descriptions
//! ([`FontProperties`], [`PatternFillSpec`], [`BorderSpec`], [`CellFormatSpec`]) rather than the
//! markup types. `Font`, `Fill`, `Border` and `CellFormat` each keep the `RawName` they were read
//! with, and every name in one is a symbol in the interner of the document that part was parsed
//! from — an interner a caller does not hold. The description has no interner and no lifetime, and
//! the build step happens inside the part that will hold the result.
//!
//! # Every edit is read-modify-write on the part that owns it
//!
//! A method here reads the part's **bytes**, parses a document that lives for the length of the
//! call, models it, changes it, writes the model back through
//! [`ToXml::write_back`](mjx_ooxml_core::ToXml::write_back), and replaces the part's bytes. Nothing
//! is cached, which is the same choice `crates/mjx-xlsx/src/worksheet/grid.rs` makes and for the
//! same reason: a cached tree is 913 bytes per cell against the packed store's 36.8.
//!
//! `write_back` is what keeps the rest of the part byte-identical — appending one font to a
//! twenty-font table re-flows the `fonts` element and copies every other slot of `styles.xml` from
//! the file's own bytes.

use mjx_ooxml_core::{RawDocument, ToXml};
use mjx_opc::{PartName, Relationship, TargetMode};
use mjx_sml::write::{
    AuthoredWorksheet, BorderSpec, CellFormatSpec, CellFormatTarget, PatternFillSpec,
};
use mjx_sml::{
    BorderTable, CellFormatTable, CellReference, FillTable, Font, FontProperties, FontTable,
    SharedStringTable, SheetEntry, SheetList, StylesheetPart, WorkbookPart,
};

use crate::error::XlsxError;
use crate::parts::{PartKind, CONTENT_TYPE_WORKSHEET};
use crate::workbook::Workbook;

impl Workbook {
    /// Appends a tab named `name`, authoring the worksheet part behind it, and answers its index in
    /// the sheet list.
    ///
    /// The new part is `/xl/worksheets/sheetN.xml` for the smallest `N` the package does not
    /// already use, related from `xl/workbook.xml` under the next free `rIdN`, and named by the new
    /// `sheet` entry's `r:id` — which is the **only** thing that names a part; `@sheetId` and the
    /// position in the list name nothing. See `crates/mjx-sml/src/workbook/sheets.rs`.
    ///
    /// **Everything fallible that does not touch the package happens first** — the part name, the
    /// authored markup, and reading the relationship prefix — so the common refusals leave the
    /// workbook exactly as it was. The three calls that follow write, and each is fallible only if
    /// the packaging layer rejects a name this method just derived from the package's own contents.
    ///
    /// # Errors
    /// [`XlsxError::MalformedWorkbook`] if `xl/workbook.xml` binds no prefix to the
    /// relationship-reference namespace — an element in no namespace is not `r:id` however it is
    /// spelled, so such a workbook cannot name a new sheet's part at all — or [`XlsxError`] if the
    /// workbook part cannot be read or the package refuses the new part.
    pub fn add_sheet(&mut self, name: &str) -> Result<usize, XlsxError> {
        let sheet_part = PartName::new(&self.free_worksheet_part_name())?;
        let relationship_id = self.next_workbook_relationship_id();
        let target = relationship_target(&sheet_part);
        let markup = AuthoredWorksheet::new(name)?;

        // The prefix is read before anything is written: a workbook that cannot spell `r:id` is
        // refused with the package untouched rather than left holding an orphan part.
        let workbook_part = self.workbook_part().clone();
        let prefix = {
            let document = self.package_mut().part_tree(&workbook_part)?;
            let Some(model) = WorkbookPart::read_part(document)? else {
                return Err(XlsxError::MalformedWorkbook(
                    "root element is not x:workbook",
                ));
            };
            model
                .relationship_prefix(&document.interner)
                .map(str::to_owned)
        };
        let Some(prefix) = prefix else {
            return Err(XlsxError::MalformedWorkbook(
                "xl/workbook.xml binds no prefix to the relationship-reference namespace, so a new \
                 sheet's r:id could not be written",
            ));
        };

        self.package_mut().insert_part(
            &sheet_part,
            CONTENT_TYPE_WORKSHEET,
            markup.to_part_bytes(),
        )?;
        self.package_mut().add_relationship(
            Some(&workbook_part),
            Relationship {
                id: relationship_id.clone(),
                rel_type: PartKind::Worksheet.relationship_type().to_owned(),
                target,
                mode: TargetMode::Internal,
            },
        )?;

        let sheet_number = self.next_sheet_id();
        self.edit_workbook_markup(|model, interner| {
            let mut entry = SheetEntry::new(interner, None);
            entry.set_name(interner, Some(name));
            entry.set_sheet_id(interner, Some(sheet_number));
            entry.set_relationship_id(interner, &prefix, &relationship_id);
            match model.sheets_mut() {
                Some(list) => list.push(entry),
                None => {
                    let mut list = SheetList::new(interner, None);
                    list.push(entry);
                    model.set_sheets(Some(list));
                }
            }
        })?;
        Ok(self.sheets().len().saturating_sub(1))
    }

    /// Sets one cell's `cellXfs` index (`c@s`) on the tab at `index`, creating a blank cell if there
    /// is none. `None` removes the attribute.
    ///
    /// A blank cell is not nothing: it carries a format and no value, which is what a shaded but
    /// empty cell is. Every other row and every other worksheet child stays byte-identical, by the
    /// same copy-on-write [`Workbook::set_cell_value`](crate::Workbook::set_cell_value) relies on.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, [`XlsxError::MissingWorkbookPart`] if the
    /// tab reaches no worksheet part, or [`XlsxError::Sml`] if the store refuses the edit.
    pub fn set_cell_style(
        &mut self,
        index: usize,
        reference: CellReference,
        style: Option<u32>,
    ) -> Result<(), XlsxError> {
        let mut markup = self
            .worksheet_markup(index)?
            .ok_or_else(|| XlsxError::MissingWorkbookPart(format!("sheet {index}")))?;
        markup
            .sheet_data_or_insert()
            .set_cell_style(reference, style)?;
        self.write_worksheet_markup(index, &markup)
    }

    /// The index `text` has in `xl/sharedStrings.xml`, appending it in **first-use order** if it is
    /// new, and writing the part back.
    ///
    /// The index is what a `t="s"` cell's `<v>` holds — pair this with
    /// [`Workbook::set_cell_value`](crate::Workbook::set_cell_value) and
    /// [`CellValue::SharedString`](mjx_sml::CellValue::SharedString).
    ///
    /// Only a plain `<si><t>…</t></si>` is ever reused: an entry carrying rich-text runs or phonetic
    /// markup displays the same characters and is not the same value, so pointing a plain string
    /// there would give that cell formatting nobody asked for. See
    /// [`SharedStringTable::intern`].
    ///
    /// # Errors
    /// [`XlsxError::MissingWorkbookPart`] if the workbook relates to no shared-string part — a
    /// workbook that never had one cannot gain one here, because the relationship and the content
    /// type are the package's business and adding them silently would author a part graph the caller
    /// did not ask for — or [`XlsxError`] if that part is unreadable.
    pub fn intern_shared_string(&mut self, text: &str) -> Result<u32, XlsxError> {
        let Some(part) = self.parts().shared_strings.clone() else {
            return Err(XlsxError::MissingWorkbookPart(
                "xl/sharedStrings.xml".to_owned(),
            ));
        };
        let Some(bytes) = self.package().part_bytes(&part) else {
            return Err(XlsxError::MissingWorkbookPart(part.as_str().to_owned()));
        };
        let document = mjx_xml::fidelity::parse(bytes)?;
        let Some(mut table) = SharedStringTable::read_part(&document)? else {
            return Err(XlsxError::MalformedWorkbook(
                "the shared-string part's root element is not x:sst",
            ));
        };
        let index = table.intern(text)?;
        self.package_mut()
            .replace_part_bytes(&part, table.to_part_bytes())?;
        Ok(index)
    }

    /// Appends a font to `xl/styles.xml` and answers its `@fontId`.
    ///
    /// # Errors
    /// [`XlsxError::MissingWorkbookPart`] if the workbook relates to no styles part, or
    /// [`XlsxError`] if that part is unreadable.
    pub fn append_font(&mut self, properties: &FontProperties) -> Result<u32, XlsxError> {
        self.edit_styles(|part, interner| {
            let font = Font::from_properties(interner, None, properties)?;
            let mut table = match part.fonts() {
                Some(table) => table.clone(),
                None => FontTable::new(interner, None),
            };
            table.push(interner, font);
            let index = last_index(table.len());
            part.set_fonts(interner, Some(table));
            Ok(index)
        })
    }

    /// Appends a pattern fill to `xl/styles.xml` and answers its `@fillId`.
    ///
    /// # Errors
    /// As [`append_font`](Self::append_font).
    pub fn append_pattern_fill(&mut self, spec: &PatternFillSpec) -> Result<u32, XlsxError> {
        self.edit_styles(|part, interner| {
            let fill = spec.build(interner, None);
            let mut table = match part.fills() {
                Some(table) => table.clone(),
                None => FillTable::new(interner, None),
            };
            table.push(interner, fill);
            let index = last_index(table.len());
            part.set_fills(interner, Some(table));
            Ok(index)
        })
    }

    /// Appends a border to `xl/styles.xml` and answers its `@borderId`.
    ///
    /// # Errors
    /// As [`append_font`](Self::append_font).
    pub fn append_border(&mut self, spec: &BorderSpec) -> Result<u32, XlsxError> {
        self.edit_styles(|part, interner| {
            let border = spec.build(interner, None);
            let mut table = match part.borders() {
                Some(table) => table.clone(),
                None => BorderTable::new(interner, None),
            };
            table.push(interner, border);
            let index = last_index(table.len());
            part.set_borders(interner, Some(table));
            Ok(index)
        })
    }

    /// Appends an `xf` to `cellXfs` or to `cellStyleXfs` and answers its index in that table.
    ///
    /// A `cellXfs` index is what [`set_cell_style`](Self::set_cell_style) takes. Putting a record in
    /// the wrong table formats nothing and reports nothing, which is why the table is a parameter
    /// rather than two methods free to drift.
    ///
    /// # Errors
    /// As [`append_font`](Self::append_font).
    pub fn append_cell_format(
        &mut self,
        target: CellFormatTarget,
        spec: &CellFormatSpec,
    ) -> Result<u32, XlsxError> {
        self.edit_styles(|part, interner| {
            let format = spec.build(interner, None);
            let existing = match target {
                CellFormatTarget::CellFormats => part.cell_formats(),
                CellFormatTarget::CellStyleFormats => part.cell_style_formats(),
            };
            let mut table = match existing {
                Some(table) => table.clone(),
                None => CellFormatTable::new(interner, None, table_kind(target)),
            };
            table.push(interner, format);
            let index = last_index(table.len());
            match target {
                CellFormatTarget::CellFormats => part.set_cell_formats(interner, Some(table)),
                CellFormatTarget::CellStyleFormats => {
                    part.set_cell_style_formats(interner, Some(table));
                }
            }
            Ok(index)
        })
    }

    /// Reads `xl/styles.xml`, hands the model and its interner to `edit`, and writes the result
    /// back over the root it was read from.
    ///
    /// Crate-private and closure-taking, which the four public methods above are deliberately not:
    /// this is the read-modify-write those four share, factored so that four copies of it cannot
    /// drift apart, and it is not part of the surface a binding has to project.
    fn edit_styles<R>(
        &mut self,
        edit: impl FnOnce(&mut StylesheetPart, &mut mjx_ooxml_core::Interner) -> Result<R, XlsxError>,
    ) -> Result<R, XlsxError> {
        let Some(part) = self.parts().styles.clone() else {
            return Err(XlsxError::MissingWorkbookPart("xl/styles.xml".to_owned()));
        };
        let Some(bytes) = self.package().part_bytes(&part) else {
            return Err(XlsxError::MissingWorkbookPart(part.as_str().to_owned()));
        };
        let mut document = mjx_xml::fidelity::parse(bytes)?;
        let Some(mut model) = StylesheetPart::read_root(&document.root, &document.interner)? else {
            return Err(XlsxError::MalformedWorkbook(
                "the styles part's root element is not x:styleSheet",
            ));
        };
        let result = {
            let RawDocument { interner, root, .. } = &mut document;
            let result = edit(&mut model, interner)?;
            model.write_back(root, interner);
            result
        };
        self.package_mut()
            .replace_part_bytes(&part, mjx_xml::fidelity::serialize_to_vec(&document))?;
        Ok(result)
    }

    /// `/xl/worksheets/sheetN.xml` for the smallest `N` the package does not already hold.
    ///
    /// Not `sheets().len() + 1`: a workbook whose second tab was deleted holds `sheet1.xml` and
    /// `sheet3.xml`, and a name derived from the tab count would collide with one of them.
    fn free_worksheet_part_name(&self) -> String {
        let taken: Vec<String> = self
            .package()
            .part_names()
            .map(|part| part.as_str().to_ascii_lowercase())
            .collect();
        for number in 1..=u32::MAX {
            let candidate = format!("/xl/worksheets/sheet{number}.xml");
            if !taken.iter().any(|name| name == &candidate) {
                return candidate;
            }
        }
        // Unreachable: the loop runs to four billion and a package cannot hold that many parts.
        "/xl/worksheets/sheet1.xml".to_owned()
    }

    /// The next free relationship id on `xl/workbook.xml`, one past the current maximum.
    fn next_workbook_relationship_id(&self) -> String {
        let mut highest = 0u32;
        if let Some(relationships) = self.package().relationships_for(Some(self.workbook_part())) {
            for relationship in relationships.iter() {
                if let Some(number) = relationship
                    .id
                    .strip_prefix("rId")
                    .and_then(|digits| digits.parse::<u32>().ok())
                {
                    highest = highest.max(number);
                }
            }
        }
        format!("rId{}", highest.saturating_add(1))
    }

    /// The `@sheetId` a new tab gets: one past the highest the workbook already writes.
    ///
    /// `@sheetId` identifies a tab *inside* `xl/workbook.xml` — what revision records and pivot
    /// caches refer to — and duplicating one would make those references ambiguous. It names no
    /// part.
    fn next_sheet_id(&self) -> u32 {
        self.sheets()
            .iter()
            .filter_map(|sheet| sheet.sheet_id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }
}

/// The index the entry just appended to a table of `len` entries has.
fn last_index(len: usize) -> u32 {
    u32::try_from(len.saturating_sub(1)).unwrap_or(u32::MAX)
}

/// The table kind [`CellFormatTable::new`] wants for a [`CellFormatTarget`].
fn table_kind(target: CellFormatTarget) -> mjx_sml::CellFormatTableKind {
    match target {
        CellFormatTarget::CellFormats => mjx_sml::CellFormatTableKind::CellFormats,
        CellFormatTarget::CellStyleFormats => mjx_sml::CellFormatTableKind::CellStyleFormats,
    }
}

/// The relationship target naming `part` from `/xl/workbook.xml` — relative to `/xl/`, the directory
/// the source part sits in.
fn relationship_target(part: &PartName) -> String {
    part.as_str()
        .strip_prefix("/xl/")
        .unwrap_or_else(|| part.as_str().trim_start_matches('/'))
        .to_owned()
}
