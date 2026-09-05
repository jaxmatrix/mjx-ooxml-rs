//! Worksheet tables at the package tier: the part each one lives in, the edge the sheet reaches it
//! through, and the surface that creates one.
//!
//! # This is where the crate split shows most plainly
//!
//! Every other feature in [`crate::worksheet::features`] is markup inside one part. A table is
//! **four things that have to agree**:
//!
//! | thing | where it lives |
//! |---|---|
//! | the table itself | `xl/tables/tableN.xml`, an `x:table` — [`mjx_sml::WorksheetTable`] |
//! | its content type | `[Content_Types].xml`, [`CONTENT_TYPE_TABLE`](crate::parts::CONTENT_TYPE_TABLE) |
//! | the edge to it | `xl/worksheets/_rels/sheetN.xml.rels`, [`REL_TABLE`](crate::parts::REL_TABLE) |
//! | the sheet's claim on it | `x:tableParts/tablePart@r:id` — [`mjx_sml::TableParts`] |
//!
//! `mjx-sml` owns the first and the fourth and **resolves nothing**: a `tablePart` holds the
//! relationship identifier as the string the file wrote, and there is no method in that crate that
//! turns one into a part. This file is where the identifier becomes a [`PartName`], and
//! [`Workbook::add_table`] is what writes all four in one call so that none of them can be left
//! behind.
//!
//! [`Workbook::validate`] then checks the direction packaging cannot see — see
//! [`SpreadsheetDefect::TablePartTargetIsNotATable`](crate::SpreadsheetDefect::TablePartTargetIsNotATable)
//! and [`DuplicateTableId`](crate::SpreadsheetDefect::DuplicateTableId). A `tablePart@r:id` that
//! names no relationship at all is `mjx-opc`'s
//! [`UndeclaredRelationshipReference`](mjx_opc::PackageDefect::UndeclaredRelationshipReference),
//! already reported over the same set of parts; restating it here would be a second, drifting
//! implementation of one rule.
//!
//! # Creating a table allocates an id; nothing ever renumbers one
//!
//! §18.5.1.2: *"A non zero integer representing the unique identifier for this table. Each table in
//! the workbook shall have a unique id."* [`Workbook::add_table`] therefore reads **every** table
//! part in the package, takes one past the highest id it finds, and writes that. It never reuses an
//! id and never renumbers an existing table: ids are what other records name a table by, and moving
//! one silently repoints whatever named it.
//!
//! # Creating a table writes no cell, and editing a cell moves no table
//!
//! [`add_table`](Workbook::add_table) authors a part and touches `sheetData` not at all — it does
//! not write the headings into row 1, and it does not clear what is there. The reverse is the rule
//! the ticket names: **an unrelated cell edit must never move a table's boundary.**
//! [`Workbook::set_cell_value`] rewrites one row of one worksheet part; the table part is not even
//! opened, so its `@ref`, `@headerRowCount` and `@totalsRowCount` come back byte for byte.
//! Resizing is [`mjx_sml::WorksheetTable::resize`], which changes all three together or refuses.

use mjx_ooxml_core::{Interner, RawDocument, ToXml};
use mjx_ooxml_types::spreadsheetml::TotalsRowFunction;
use mjx_opc::{PartName, Relationship, TargetMode};
use mjx_sml::write::AuthoredTable;
use mjx_sml::{CellRange, TableColumn, TableStyleOrigin, WorksheetTable, WorksheetTableSpec};

use crate::error::XlsxError;
use crate::parts::{PartKind, CONTENT_TYPE_TABLE};
use crate::workbook::Workbook;

/// One column of a [`SheetTable`], decoded.
///
/// A **report**, not a second model: every field is read out of [`mjx_sml::TableColumn`] and
/// nothing here can write one. The markup itself is reached through
/// [`Workbook::table_markup`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetTableColumn {
    /// `@id`, unique within the table.
    pub id: u32,
    /// `@name` — the heading text, and what a structured reference (`Sales[Region]`) names.
    pub name: String,
    /// `@totalsRowFunction`, or `None` when the column writes none (the schema default `none`).
    pub totals_row_function: Option<TotalsRowFunction>,
    /// `@totalsRowLabel` — the literal text a totals cell shows instead of an aggregate.
    pub totals_row_label: Option<String>,
    /// `x:calculatedColumnFormula`'s text, **exactly as the file wrote it**. Never expanded into
    /// per-cell formulas and never evaluated; see [`mjx_sml::features::tables`].
    pub calculated_column_formula: Option<String>,
    /// `x:totalsRowFormula`'s text, on the same terms.
    pub totals_row_formula: Option<String>,
}

/// One table on a sheet, resolved to its part and decoded.
///
/// What [`Workbook::sheet_tables`] answers with: owned, borrowing nothing, and holding no interner —
/// the shape [`crate::DefinedNameEntry`] takes, and the shape a binding can project. For the markup
/// itself, and for anything this report does not carry, use [`Workbook::table_markup`] with the
/// [`part`](Self::part) named here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetTable {
    /// The part the table lives in — `/xl/tables/table1.xml` in everything a real producer writes,
    /// though nothing requires that spelling.
    pub part: PartName,
    /// The `tablePart@r:id` the sheet reached it through.
    pub relationship_id: String,
    /// `@id` — **workbook-unique**, and never renumbered by anything here.
    pub id: u32,
    /// `@displayName` — what a formula references the table by.
    pub display_name: String,
    /// `@name`, the programmatic name, or `None` when the table writes none. §18.5.1.2 says it
    /// *"should be the same as the table's displayName"* by default, and an absent attribute says
    /// exactly that rather than restating it.
    pub name: Option<String>,
    /// `@ref` — the whole region, **header and totals rows included**.
    pub range: CellRange,
    /// `@headerRowCount`, or the schema default 1.
    pub header_row_count: u32,
    /// `@totalsRowCount`, or the schema default 0.
    pub totals_row_count: u32,
    /// `tableStyleInfo@name`, or `None` when the table names no style at all — which is a different
    /// statement from naming one the file does not define. Ask
    /// [`Workbook::table_style_origin`] which of the three that is.
    pub style_name: Option<String>,
    /// The columns, left to right.
    pub columns: Vec<SheetTableColumn>,
}

impl SheetTable {
    /// How many rows of [`range`](Self::range) are data rows, or `None` when the header and totals
    /// counts do not fit inside it.
    ///
    /// `None` is a **report about the file**, not a repair of it: a table saying `ref="A1:C3"` with
    /// `totalsRowCount="9"` keeps saying exactly that.
    #[must_use]
    pub fn data_row_count(&self) -> Option<u32> {
        let bounds = self.range.normalized_bounds();
        let height = bounds
            .last_row()
            .saturating_sub(bounds.first_row())
            .checked_add(1)?;
        height.checked_sub(self.header_row_count.checked_add(self.totals_row_count)?)
    }
}

impl Workbook {
    /// Every table on the tab at `index`, in the order `x:tableParts` lists them, resolved to their
    /// parts and decoded.
    ///
    /// The order is the sheet's own claim, not the relationship order:
    /// [`WorksheetParts::tables`](crate::WorksheetParts::tables) answers the latter and is the right
    /// call for "which table parts does this sheet relate to at all". A relationship this sheet has
    /// but `tableParts` never names is therefore **absent from this list** and reported by
    /// [`validate`](Workbook::validate) instead.
    ///
    /// Reading does not dirty the package: every part is read from its bytes into a document that
    /// lives for the length of the call, and [`save`](Workbook::save) still re-emits them verbatim.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab; [`XlsxError::Sml`] or [`XlsxError::Xml`]
    /// if a table part is unreadable or its markup does not match `CT_Table`;
    /// [`XlsxError::MalformedWorkbook`] if a table part's root is not an `x:table`.
    pub fn sheet_tables(&self, index: usize) -> Result<Vec<SheetTable>, XlsxError> {
        let Some(markup) = self.worksheet_markup(index)? else {
            return Ok(Vec::new());
        };
        let sheet_part = self
            .sheets()
            .get(index)
            .and_then(|sheet| sheet.part.clone())
            .ok_or_else(|| XlsxError::MissingWorkbookPart(format!("sheet {index}")))?;
        let Some(list) = markup.table_parts() else {
            return Ok(Vec::new());
        };
        let prefix = markup.relationship_prefix();

        let mut tables = Vec::new();
        for entry in list.parts() {
            let Some(relationship_id) = entry.relationship_id(markup.interner(), prefix)? else {
                // No `r:id`, or the part binds no prefix to the relationship-reference namespace, so
                // the entry names nothing. `CT_TablePart` declares `r:id` required, which makes this
                // a defect in the file — and one `mjx-opc` reports over the parts it will write.
                // Answering around it here would be repairing it.
                continue;
            };
            let Some(part) = self.resolve_sheet_relationship(&sheet_part, &relationship_id)? else {
                continue;
            };
            let Some(table) = self.read_table_part(&part)? else {
                continue;
            };
            let (interner, table) = table;
            tables.push(decode(&table, &interner, part, relationship_id)?);
        }
        Ok(tables)
    }

    /// The table part at `part`, handed to `read` together with the [`Interner`] it was parsed with.
    ///
    /// A visitor rather than a returned value because every accessor on
    /// [`WorksheetTable`] takes an interner — an attribute name is a symbol — and the only interner
    /// those symbols are meaningful in is the one this part was parsed with. The same shape
    /// [`Workbook::auto_filter`] takes, for the same reason.
    ///
    /// `Ok(None)` when the package holds no such part or its root is not an `x:table`.
    ///
    /// Reading does not dirty the package.
    ///
    /// # Errors
    /// [`XlsxError::Xml`] if the part is not well-formed, or [`XlsxError::Sml`] if its markup does
    /// not match `CT_Table`.
    pub fn table_markup<R>(
        &self,
        part: &PartName,
        read: impl FnOnce(&WorksheetTable, &Interner) -> R,
    ) -> Result<Option<R>, XlsxError> {
        let Some((interner, table)) = self.read_table_part(part)? else {
            return Ok(None);
        };
        Ok(Some(read(&table, &interner)))
    }

    /// Reads the table part at `part`, hands the model and its interner to `edit`, and writes the
    /// result back over the root it was read from.
    ///
    /// The write-back goes through [`ToXml::write_back`], which restores the source range of every
    /// node the rebuild reproduced unchanged — so renaming one column re-flows that one start tag
    /// and copies the rest of the part, extension list included.
    ///
    /// # Errors
    /// [`XlsxError::MissingWorkbookPart`] if the package holds no such part,
    /// [`XlsxError::MalformedWorkbook`] if its root is not an `x:table`, or [`XlsxError`] if it
    /// cannot be read or the package refuses the replacement.
    pub fn edit_table_markup<R>(
        &mut self,
        part: &PartName,
        edit: impl FnOnce(&mut WorksheetTable, &mut Interner) -> Result<R, XlsxError>,
    ) -> Result<R, XlsxError> {
        let Some(bytes) = self.package().part_bytes(part) else {
            return Err(XlsxError::MissingWorkbookPart(part.as_str().to_owned()));
        };
        let mut document = mjx_xml::fidelity::parse(bytes)?;
        let Some(mut model) = WorksheetTable::read_root(&document.root, &document.interner)? else {
            return Err(XlsxError::MalformedWorkbook(
                "a table part's root element is not x:table",
            ));
        };
        let result = {
            let RawDocument { interner, root, .. } = &mut document;
            let result = edit(&mut model, interner)?;
            model.write_back(root, interner);
            result
        };
        self.package_mut()
            .replace_part_bytes(part, mjx_xml::fidelity::serialize_to_vec(&document))?;
        Ok(result)
    }

    /// What this workbook has to say about the table style called `name`: defined in
    /// `xl/styles.xml`, one of ECMA-376's 144 presets, or neither.
    ///
    /// **This never answers "not found" for a preset name.** `TableStyleMedium2` is in no `.xlsx`
    /// anywhere — the presets are the application's, not the file's — so reporting a missing
    /// definition for one would be a confident wrong answer. See
    /// [`mjx_sml::styles::table_styles`] for the whole of the reasoning, and
    /// [`mjx_sml::TableStyles::lookup`] for the borrowing form that hands back the definition itself.
    ///
    /// A workbook that relates to no styles part, or whose styles part writes no `tableStyles`, can
    /// still answer: a preset is a preset whether or not the file has a table-style block at all.
    ///
    /// # Errors
    /// [`XlsxError`] if the styles part exists and is unreadable, or
    /// [`XlsxError::MalformedWorkbook`] if its root is not an `x:styleSheet`.
    pub fn table_style_origin(&self, name: &str) -> Result<TableStyleOrigin, XlsxError> {
        let Some(part) = self.parts().styles.clone() else {
            return Ok(origin_without_a_styles_part(name));
        };
        let Some(bytes) = self.package().part_bytes(&part) else {
            return Ok(origin_without_a_styles_part(name));
        };
        let document = mjx_xml::fidelity::parse(bytes)?;
        let Some(styles) = mjx_sml::StylesheetPart::read_root(&document.root, &document.interner)?
        else {
            return Err(XlsxError::MalformedWorkbook(
                "the styles part's root element is not x:styleSheet",
            ));
        };
        let Some(table_styles) = styles.table_styles() else {
            return Ok(origin_without_a_styles_part(name));
        };
        Ok(table_styles
            .lookup(&document.interner, name)
            .map_err(mjx_ooxml_core::FromXmlError::from)?
            .origin())
    }

    /// The `@id` a new table would get: one past the highest any table part in the package already
    /// writes.
    ///
    /// **One past the highest, never a hole.** §18.5.1.2 makes the id workbook-unique and says other
    /// records may name a table by it; filling a gap left by a deleted table would hand a new table
    /// an identifier something else may still be pointing at. This is
    /// [`next_sheet_id`](crate::Workbook::add_sheet)'s rule for `sheet@sheetId`, restated.
    ///
    /// Answers 1 for a workbook with no tables, because §18.5.1.2 requires a **non-zero** id.
    ///
    /// # Errors
    /// [`XlsxError::Xml`] if a table part in the package is not well-formed, or [`XlsxError::Sml`]
    /// if one does not match `CT_Table`. A workbook whose tables cannot be read is one whose ids
    /// cannot be counted, and guessing past that would be the renumbering this refuses.
    pub fn next_table_id(&self) -> Result<u32, XlsxError> {
        let mut highest = 0u32;
        for part in self.table_part_names() {
            let Some((interner, table)) = self.read_table_part(&part)? else {
                continue;
            };
            if let Ok(id) = table.id(&interner) {
                highest = highest.max(id);
            }
        }
        Ok(highest.saturating_add(1))
    }

    /// Adds a table over `spec.range` to the tab at `index`, and answers what was created.
    ///
    /// Four things are written, and they are written together: the part
    /// `/xl/tables/tableN.xml`, its content-type override, a [`REL_TABLE`](crate::parts::REL_TABLE)
    /// relationship from the **sheet** part, and a `tablePart` entry in the sheet's `x:tableParts`
    /// (created at rank 37 of `CT_Worksheet` if the sheet has none).
    ///
    /// **`spec.id` is ignored.** The `@id` is allocated by [`next_table_id`](Self::next_table_id),
    /// because only something holding the package knows which ids are taken, and a caller-supplied
    /// one could collide with a table on another sheet. Nothing here renumbers an existing table.
    ///
    /// **No cell is touched.** The headings in `spec.columns` are written into the table part, not
    /// into row 1 of the sheet: making them match is the caller's, and doing it silently would write
    /// over values nobody asked to lose.
    ///
    /// Everything fallible that does not touch the package happens first — the part name, the
    /// relationship prefix, and the authored markup — so the common refusals leave the workbook
    /// exactly as it was.
    ///
    /// A worksheet that binds no prefix to the relationship-reference namespace gains one: a
    /// `tablePart` is nothing but an `r:id`, and a sheet authored from nothing declares only the
    /// SpreadsheetML namespace. See
    /// [`WorksheetPart::bind_relationship_prefix`](mjx_sml::WorksheetPart::bind_relationship_prefix),
    /// which never overwrites a binding the file made.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab; [`XlsxError::MissingWorkbookPart`] if it
    /// reaches no worksheet part; [`XlsxError::Sml`] for a spec whose geometry does not fit or which
    /// names no column; or [`XlsxError`] if the package refuses the new part.
    pub fn add_table(
        &mut self,
        index: usize,
        spec: &WorksheetTableSpec,
    ) -> Result<SheetTable, XlsxError> {
        let sheets = self.sheets().len();
        let sheet_part = self
            .sheets()
            .get(index)
            .ok_or(XlsxError::NoSuchSheet { index, sheets })?
            .part
            .clone()
            .ok_or_else(|| XlsxError::MissingWorkbookPart(format!("sheet {index}")))?;

        let mut markup = self
            .worksheet_markup(index)?
            .ok_or_else(|| XlsxError::MissingWorkbookPart(format!("sheet {index}")))?;
        // A `tablePart` is nothing but an `r:id`, so the part has to be able to spell one. A
        // worksheet this library authored declares only the SpreadsheetML namespace — see
        // `AuthoredWorksheet`'s seed — and refusing here would mean a sheet built from nothing could
        // never gain a table. The declaration is added only when there is none, and never
        // overwrites a binding the file made.
        let prefix = markup.bind_relationship_prefix();

        let allocated = WorksheetTableSpec {
            id: self.next_table_id()?,
            ..spec.clone()
        };
        let mut authored = AuthoredTable::from_spec(&allocated)?;
        let bytes = authored.to_part_bytes();

        let table_part = PartName::new(&self.free_table_part_name())?;
        let relationship_id = self.next_sheet_relationship_id(&sheet_part);
        let target = relative_target(&sheet_part, &table_part);

        self.package_mut()
            .insert_part(&table_part, CONTENT_TYPE_TABLE, bytes)?;
        self.package_mut().add_relationship(
            Some(&sheet_part),
            Relationship {
                id: relationship_id.clone(),
                rel_type: PartKind::Table.relationship_type().to_owned(),
                target,
                mode: TargetMode::Internal,
            },
        )?;

        {
            let entry_prefix = markup.element_prefix().map(str::to_owned);
            let interner = markup.interner_mut();
            let mut entry = mjx_sml::TablePart::new(interner, entry_prefix.as_deref());
            entry.set_relationship_id(interner, &prefix, &relationship_id);
            let mut list = match markup.table_parts() {
                Some(existing) => existing.clone(),
                None => mjx_sml::TableParts::new(markup.interner_mut(), entry_prefix.as_deref()),
            };
            list.push(markup.interner_mut(), entry);
            markup.set_table_parts(Some(list));
        }
        self.write_worksheet_markup(index, &markup)?;

        let interner = authored.interner();
        decode(authored.part(), interner, table_part, relationship_id)
    }

    /// Every part in the package registered under the table content type, in container order.
    fn table_part_names(&self) -> Vec<PartName> {
        self.package()
            .part_names()
            .filter(|part| {
                self.package()
                    .content_type_of(part)
                    .is_some_and(|content_type| content_type == CONTENT_TYPE_TABLE)
            })
            .collect()
    }

    /// Parses one table part into a model and the interner its names live in.
    fn read_table_part(
        &self,
        part: &PartName,
    ) -> Result<Option<(Interner, WorksheetTable)>, XlsxError> {
        let Some(bytes) = self.package().part_bytes(part) else {
            return Ok(None);
        };
        let document = mjx_xml::fidelity::parse(bytes)?;
        let Some(table) = WorksheetTable::read_part(&document)? else {
            return Ok(None);
        };
        Ok(Some((document.interner, table)))
    }

    /// Resolves one of a sheet part's own relationships by id, or `None` when it declares none with
    /// that id.
    fn resolve_sheet_relationship(
        &self,
        sheet_part: &PartName,
        relationship_id: &str,
    ) -> Result<Option<PartName>, XlsxError> {
        let Some(rels) = self.package().relationships_for(Some(sheet_part)) else {
            return Ok(None);
        };
        let Some(rel) = rels.by_id(relationship_id) else {
            return Ok(None);
        };
        if rel.mode == TargetMode::External {
            return Err(XlsxError::ExternalTarget {
                target: rel.target.clone(),
            });
        }
        Ok(Some(crate::nav::resolve_target(sheet_part, &rel.target)?))
    }

    /// `/xl/tables/tableN.xml` for the smallest `N` the package does not already hold.
    ///
    /// Not the table count: a workbook whose second table was deleted holds `table1.xml` and
    /// `table3.xml`, and a name derived from the count would collide with one of them. This is a
    /// *part name*, which names nothing but itself — unlike a table's `@id`, which other records may
    /// point at and which is therefore never reused.
    fn free_table_part_name(&self) -> String {
        let taken: Vec<String> = self
            .package()
            .part_names()
            .map(|part| part.as_str().to_ascii_lowercase())
            .collect();
        for number in 1..=u32::MAX {
            let candidate = format!("/xl/tables/table{number}.xml");
            if !taken.iter().any(|name| name == &candidate) {
                return candidate;
            }
        }
        // Unreachable: the loop runs to four billion and a package cannot hold that many parts.
        "/xl/tables/table1.xml".to_owned()
    }

    /// The next free relationship id on `part`'s own `.rels`, one past the current maximum.
    ///
    /// One past the **maximum** rather than one past the count, and that is the whole of it: an id a
    /// caller deleted may still be named by markup this library did not write, so nothing here ever
    /// reuses one. `pub(crate)` because MJXOFF-127's hyperlinks allocate from the same `.rels` and a
    /// second allocator could hand out an id this one had already promised.
    pub(crate) fn next_sheet_relationship_id(&self, part: &PartName) -> String {
        let mut highest = 0u32;
        if let Some(relationships) = self.package().relationships_for(Some(part)) {
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
}

/// The answer for a workbook that defines no table styles of its own.
///
/// Not `Undefined` for a preset name: a preset is the application's, and whether the file carries a
/// `tableStyles` block has nothing to do with it.
fn origin_without_a_styles_part(name: &str) -> TableStyleOrigin {
    if mjx_sml::builtin_table_style_name(name).is_some() {
        TableStyleOrigin::BuiltIn
    } else {
        TableStyleOrigin::Undefined
    }
}

/// The relationship `Target` naming `to` from `from` — relative to the directory `from` sits in,
/// which is what a `.rels` target is resolved against.
///
/// `/xl/worksheets/sheet1.xml` → `/xl/tables/table1.xml` gives `../tables/table1.xml`, which is what
/// Excel writes. Computed rather than hard-coded because nothing in OPC requires a worksheet to live
/// in `/xl/worksheets/`, and a package that puts one elsewhere would otherwise get a target that
/// resolves to a part that is not there.
fn relative_target(from: &PartName, to: &PartName) -> String {
    let from_segments: Vec<&str> = from.as_str().trim_start_matches('/').split('/').collect();
    let to_segments: Vec<&str> = to.as_str().trim_start_matches('/').split('/').collect();
    // The last segment of each is the file name, so only the directories are compared.
    let from_dirs = &from_segments[..from_segments.len().saturating_sub(1)];
    let to_dirs = &to_segments[..to_segments.len().saturating_sub(1)];
    let shared = from_dirs
        .iter()
        .zip(to_dirs.iter())
        .take_while(|(a, b)| a.eq_ignore_ascii_case(b))
        .count();
    let mut target = String::new();
    for _ in shared..from_dirs.len() {
        target.push_str("../");
    }
    for segment in &to_dirs[shared..] {
        target.push_str(segment);
        target.push('/');
    }
    target.push_str(to_segments.last().copied().unwrap_or_default());
    target
}

/// Decodes one parsed table into the owned report [`Workbook::sheet_tables`] answers with.
fn decode(
    table: &WorksheetTable,
    interner: &Interner,
    part: PartName,
    relationship_id: String,
) -> Result<SheetTable, XlsxError> {
    use mjx_ooxml_core::FromXmlError;

    let columns = table
        .table_columns()
        .map(|column| decode_column(column, interner))
        .collect::<Result<Vec<_>, XlsxError>>()?;
    Ok(SheetTable {
        part,
        relationship_id,
        id: table.id(interner).map_err(FromXmlError::from)?,
        display_name: table
            .display_name(interner)
            .map_err(FromXmlError::from)?
            .into_owned(),
        name: table
            .name(interner)
            .map_err(FromXmlError::from)?
            .map(std::borrow::Cow::into_owned),
        range: table.range(interner).map_err(FromXmlError::from)?,
        header_row_count: table
            .header_row_count(interner)
            .map_err(FromXmlError::from)?,
        totals_row_count: table
            .totals_row_count(interner)
            .map_err(FromXmlError::from)?,
        style_name: match table.style() {
            Some(style) => style
                .name(interner)
                .map_err(FromXmlError::from)?
                .map(std::borrow::Cow::into_owned),
            None => None,
        },
        columns,
    })
}

/// Decodes one table column into its report.
fn decode_column(column: &TableColumn, interner: &Interner) -> Result<SheetTableColumn, XlsxError> {
    use mjx_ooxml_core::FromXmlError;

    // The schema default is `none`, and this reports `None` for it: "the column states no totals
    // function" and "the column states `none`" are the same claim about the file, and inventing a
    // `Some(TotalsRowFunction::None)` for an absent attribute would make them look different.
    let totals_row_function = match column
        .totals_row_function(interner)
        .map_err(FromXmlError::from)?
    {
        TotalsRowFunction::None => None,
        other => Some(other),
    };
    Ok(SheetTableColumn {
        id: column.id(interner).map_err(FromXmlError::from)?,
        name: column
            .name(interner)
            .map_err(FromXmlError::from)?
            .into_owned(),
        totals_row_function,
        totals_row_label: column
            .totals_row_label(interner)
            .map_err(FromXmlError::from)?
            .map(std::borrow::Cow::into_owned),
        calculated_column_formula: column
            .calculated_column_formula()
            .map(|formula| formula.text().to_owned()),
        totals_row_formula: column
            .totals_row_formula()
            .map(|formula| formula.text().to_owned()),
    })
}
