//! `x:definedNames` — the names a formula may use in place of a range, as a caller holding a
//! [`Workbook`] sees them.
//!
//! [`mjx_sml::DefinedName`] is the markup; [`DefinedNameEntry`] is the decoded snapshot, with one
//! thing added that only this crate can add: `@localSheetId` **resolved against the sheet list**.
//!
//! # Scope resolution is the only thing this layer contributes
//!
//! An absent `@localSheetId` scopes a name to the workbook. A present one scopes it to the sheet at
//! that **index in the `x:sheets` list** — not to the sheet with that `@sheetId`, which is a
//! different identifier space (see [`super::sheets`]). Turning the index into a tab name needs the
//! resolved sheet list, which is this crate's; that is why [`DefinedNameScope`] lives here and not
//! one crate down.
//!
//! # An index that names no sheet is reported, never repaired
//!
//! `sml.xsd` does not constrain `@localSheetId` against the number of sheets, so a file can — and
//! `tests/fixtures/workbook_sheet_order.xlsx` deliberately does — carry one that names no tab. It
//! comes back as [`DefinedNameScope::UnknownSheet`], carrying the number the file wrote. Renumbering
//! it would silently rescope somebody's name and dropping it would silently promote a sheet-scoped
//! name to a global one; both are worse than saying so.

use mjx_sml::BuiltInName;

use crate::error::XlsxError;

use super::Workbook;

/// What a defined name applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinedNameScope {
    /// No `@localSheetId`: the name applies to the whole workbook.
    Workbook,
    /// `@localSheetId` names the tab at this index in the sheet list.
    Sheet {
        /// The index in the `x:sheets` list — which is tab order.
        index: usize,
        /// That tab's `@name`, resolved.
        name: String,
    },
    /// `@localSheetId` is present and names no tab. The file is wrong; this is what it said.
    UnknownSheet {
        /// The index the file wrote.
        index: u32,
    },
}

/// One `x:definedName`, decoded and with its scope resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinedNameEntry {
    /// `@name` — the name as it appears in a consumer's user interface.
    pub name: String,
    /// Which of the eight names ECMA-376 Part 1 §18.2.6 reserves this is, if any.
    pub built_in: Option<BuiltInName>,
    /// What the name applies to.
    pub scope: DefinedNameScope,
    /// The element's character data: the formula this name stands for, **as text**. Nothing here
    /// parses or evaluates it — see [`mjx_sml::DefinedName`] for why.
    pub definition: String,
    /// `@hidden` — whether a consumer hides the name from its name manager.
    pub hidden: bool,
}

impl Workbook {
    /// Every defined name, in document order, with its scope resolved against the sheet list.
    ///
    /// # Errors
    /// Returns [`XlsxError`] if the workbook part cannot be read, or if a name's `@name` is absent
    /// (the schema requires it) or one of its attributes holds a value its type rejects.
    pub fn defined_names(&mut self) -> Result<Vec<DefinedNameEntry>, XlsxError> {
        let tabs: Vec<String> = self
            .sheets()
            .iter()
            .map(|sheet| sheet.name.clone())
            .collect();
        self.workbook_markup(|part, interner| {
            let Some(names) = part.defined_names() else {
                return Ok(Vec::new());
            };
            names
                .names()
                .map(|name| {
                    let scope = match name.local_sheet_index(interner)? {
                        None => DefinedNameScope::Workbook,
                        Some(index) => match usize::try_from(index)
                            .ok()
                            .and_then(|index| tabs.get(index).map(|name| (index, name)))
                        {
                            Some((index, tab)) => DefinedNameScope::Sheet {
                                index,
                                name: tab.clone(),
                            },
                            None => DefinedNameScope::UnknownSheet { index },
                        },
                    };
                    Ok(DefinedNameEntry {
                        name: name.name(interner)?.into_owned(),
                        built_in: name.built_in(interner)?,
                        scope,
                        definition: name.definition().to_owned(),
                        hidden: name.hidden(interner)?,
                    })
                })
                .collect::<Result<Vec<_>, mjx_ooxml_core::AttributeError>>()
        })?
        .map_err(XlsxError::from)
    }

    /// The workbook-scoped name spelled exactly `name`, or `None`.
    ///
    /// Exact and case-sensitive: `@name` is `ST_Xstring`, and a case-insensitive match is a rule
    /// this library would be inventing.
    ///
    /// # Errors
    /// As [`defined_names`](Self::defined_names).
    pub fn defined_name(&mut self, name: &str) -> Result<Option<DefinedNameEntry>, XlsxError> {
        Ok(self
            .defined_names()?
            .into_iter()
            .find(|entry| entry.name == name && entry.scope == DefinedNameScope::Workbook))
    }

    /// The `_xlnm.Print_Area` scoped to the tab at `sheet_index`, as the text the file wrote.
    ///
    /// The print area is a formula like any other defined name — `Summary!$A$1:$D$20`, or several
    /// ranges separated by commas — so it comes back as text rather than as a parsed range. A caller
    /// that knows its workbook writes a single plain reference can hand it to
    /// [`mjx_sml::CellRange::parse`].
    ///
    /// # Errors
    /// As [`defined_names`](Self::defined_names).
    pub fn print_area(&mut self, sheet_index: usize) -> Result<Option<String>, XlsxError> {
        Ok(self.defined_names()?.into_iter().find_map(|entry| {
            let matches_sheet =
                matches!(&entry.scope, DefinedNameScope::Sheet { index, .. } if *index == sheet_index);
            (entry.built_in == Some(BuiltInName::PrintArea) && matches_sheet)
                .then_some(entry.definition)
        }))
    }
}
