//! Workbook-level metadata: defined names, print areas, the date system, and what the producer's
//! calculation engine was told.
//!
//! [`mjx_xlsx::DefinedNameEntry`] carries its scope as a `usize` sheet index; [`DefinedName`] is the
//! same value with the index widened to the `u32` every address on this facade is, which is the only
//! reason a second type exists here.

use mjx_xlsx::DefinedNameScope;

use crate::error::Error;
use crate::index::{count, index};

use super::Workbook;

/// One `x:definedName`, decoded, with its scope resolved against the tab list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinedName {
    /// `@name` — the name as it appears in a consumer's name manager.
    pub name: String,
    /// The index of the tab this name is local to, or `None` when it applies to the whole workbook.
    ///
    /// `Some` with [`sheet_name`](Self::sheet_name) `None` is a file whose `@localSheetId` names no
    /// tab: what it said, reported rather than repaired.
    pub sheet: Option<u32>,
    /// The name of the tab [`sheet`](Self::sheet) indexes, when there is one there.
    pub sheet_name: Option<String>,
    /// The element's character data: the formula this name stands for, **as text**. Nothing here
    /// parses or evaluates it.
    pub definition: String,
    /// `@hidden` — whether a consumer hides the name from its name manager.
    pub hidden: bool,
}

impl Workbook {
    /// Every defined name, in document order, with its scope resolved against the tab list.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if the workbook part
    /// cannot be read, or if a name's `@name` is absent — which the schema requires.
    pub fn defined_names(&mut self) -> Result<Vec<DefinedName>, Error> {
        Ok(self
            .workbook
            .defined_names()?
            .into_iter()
            .map(defined_name)
            .collect())
    }

    /// One defined name by its `@name`, or `None` when the workbook defines none by that name.
    ///
    /// # Errors
    /// As [`defined_names`](Self::defined_names).
    pub fn defined_name(&mut self, name: &str) -> Result<Option<DefinedName>, Error> {
        Ok(self.workbook.defined_name(name)?.map(defined_name))
    }

    /// The print area of one tab — the `_xlnm.Print_Area` defined name scoped to it — as the text
    /// the file wrote, or `None` when that tab defines none.
    ///
    /// Carried verbatim, including its sheet qualification: `Sheet1!$A$1:$D$20`.
    ///
    /// # Errors
    /// As [`defined_names`](Self::defined_names).
    pub fn print_area(&mut self, sheet: u32) -> Result<Option<String>, Error> {
        Ok(self.workbook.print_area(index(sheet))?)
    }

    /// Which epoch this workbook's date serials count from.
    ///
    /// The two systems are 1,462 days apart, so reading a workbook with the wrong one shifts every
    /// date in it by just over four years — which is why this is a named two-valued type and not a
    /// `bool` somewhere.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if the workbook part
    /// cannot be read, or [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if
    /// `@date1904` holds a value that is not an `xsd:boolean`.
    pub fn date_system(&mut self) -> Result<mjx_xlsx::DateSystem, Error> {
        Ok(self.workbook.date_system()?)
    }

    /// `x:calcPr`, decoded — what the producer's calculation engine was told.
    ///
    /// **Reported, never acted on.** There is no calculation engine here; every field is a statement
    /// the file makes about how some *other* application recalculates.
    ///
    /// # Errors
    /// As [`date_system`](Self::date_system).
    pub fn calculation_settings(&mut self) -> Result<mjx_xlsx::CalculationSettings, Error> {
        Ok(self.workbook.calculation_settings()?)
    }
}

/// One model defined name as the facade states it.
fn defined_name(entry: mjx_xlsx::DefinedNameEntry) -> DefinedName {
    let (sheet, sheet_name) = match entry.scope {
        DefinedNameScope::Workbook => (None, None),
        DefinedNameScope::Sheet { index, name } => (Some(count(index)), Some(name)),
        DefinedNameScope::UnknownSheet { index } => (Some(index), None),
    };
    DefinedName {
        name: entry.name,
        sheet,
        sheet_name,
        definition: entry.definition,
        hidden: entry.hidden,
    }
}
