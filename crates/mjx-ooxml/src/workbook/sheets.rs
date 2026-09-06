//! The tab strip: what sheets a workbook has, and the two edits that change the list.
//!
//! [`mjx_xlsx::Sheet`] is already owned and borrows nothing, but it carries an
//! [`mjx_opc::PartName`] — a validated handle this facade deliberately never hands out, because a
//! binding cannot carry one across the boundary and back. [`SheetSummary`] is the same information
//! with the part named as `&str`, the same translation [`crate::Deck`] applies to every part-graph
//! accessor it keeps.

use mjx_xlsx::SheetKind;

use crate::error::{Error, ErrorCode};
use crate::index::{count, index};

use super::Workbook;

/// One tab of a workbook, as the file states it.
///
/// Every field is what `x:sheets` said, not what it ought to have said: a workbook this library can
/// open is not necessarily one it would agree to write, and [`Workbook::validate`] is where the
/// disagreements are reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetSummary {
    /// The tab's name (`@name`), with XML entities decoded.
    pub name: String,
    /// The sheet's own identifier (`@sheetId`), or `None` when the attribute is absent or is not an
    /// `xsd:unsignedInt`. Required by the schema; absent only in a file no producer here wrote.
    pub sheet_id: Option<u32>,
    /// Whether the tab is shown in a consumer's tab strip (`@state`).
    pub is_visible: bool,
    /// Which of the three sheet kinds the tab's target is, or `None` when the target is missing or
    /// carries a content type that is not one of the three.
    pub kind: Option<SheetKind>,
    /// The part the tab's `r:id` reaches — `"/xl/worksheets/sheet1.xml"` — or `None` when the
    /// workbook's `.rels` declares no such relationship, or declares it external.
    pub part: Option<String>,
}

impl Workbook {
    /// How many tabs the workbook lists.
    #[must_use]
    pub fn sheet_count(&self) -> u32 {
        count(self.workbook.sheets().len())
    }

    /// Every tab, in tab order.
    #[must_use]
    pub fn sheets(&self) -> Vec<SheetSummary> {
        self.workbook.sheets().iter().map(summary).collect()
    }

    /// One tab.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`] if `sheet` names no tab.
    pub fn sheet(&self, sheet: u32) -> Result<SheetSummary, Error> {
        self.workbook
            .sheets()
            .get(index(sheet))
            .map(summary)
            .ok_or_else(|| self.no_such_sheet(sheet))
    }

    /// The index of the tab named `name`, or `None` when no tab has that name.
    ///
    /// Names are compared exactly, because Excel treats two tabs differing only in case as the same
    /// name and this library does not repair a file that has both.
    #[must_use]
    pub fn sheet_index(&self, name: &str) -> Option<u32> {
        self.workbook.sheet_index_by_name(name).map(count)
    }

    /// Appends a new, empty worksheet named `name` and answers its index.
    ///
    /// The part, its relationship, its content-type override and the `x:sheets` entry are all
    /// written together — a sheet added without one of those four is a file Excel offers to repair.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`] if the workbook part cannot be read, or
    /// [`ErrorCode::InvalidDocument`] if the package refuses the new part.
    pub fn add_sheet(&mut self, name: &str) -> Result<u32, Error> {
        Ok(count(self.workbook.add_sheet(name)?))
    }

    /// Renames the tab at `sheet`.
    ///
    /// Only `x:sheets`'s own `@name` changes. **Formulas that reference the old name are not
    /// rewritten** — this library has no formula parser, so rewriting them would be guessing; see
    /// [*Deliberate limitations*](mjx_xlsx::guide::deliberate_limitations).
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`] if `sheet` names no tab, or
    /// [`ErrorCode::MalformedDocument`] if the workbook part cannot be read.
    pub fn rename_sheet(&mut self, sheet: u32, name: &str) -> Result<(), Error> {
        Ok(self.workbook.rename_sheet(index(sheet), name)?)
    }

    /// The index of the tab a consumer opens the workbook on (`bookViews/workbookView@activeTab`),
    /// or `None` when the workbook writes no `bookViews` at all.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`] if the workbook part cannot be read.
    pub fn active_sheet(&mut self) -> Result<Option<u32>, Error> {
        let name = self
            .workbook
            .active_sheet()?
            .map(|sheet| sheet.name.clone());
        Ok(name.and_then(|name| self.sheet_index(&name)))
    }

    /// Every `x:workbookView`, decoded, in document order.
    ///
    /// Screen geometry in the producer's own units on the producer's own display — carried because a
    /// file that loses it reopens differently, and meaningful nowhere else.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`] if the workbook part cannot be read, or
    /// [`ErrorCode::InvalidArgument`] if one of the geometry attributes holds a value its declared
    /// type rejects.
    pub fn window_views(&mut self) -> Result<Vec<WorkbookWindowInfo>, Error> {
        Ok(self
            .workbook
            .window_views()?
            .into_iter()
            .map(|view| WorkbookWindowInfo {
                active_tab_index: view.active_tab_index,
                first_visible_tab_index: view.first_visible_tab_index,
                window_left: view.window_position.map(|(left, _)| left),
                window_top: view.window_position.map(|(_, top)| top),
                window_width: view.window_size.map(|(width, _)| width),
                window_height: view.window_size.map(|(_, height)| height),
                tab_strip_ratio: view.tab_strip_ratio,
                show_sheet_tabs: view.show_sheet_tabs,
            })
            .collect())
    }

    /// The error a tab index outside the list gets, phrased the same way everywhere.
    pub(super) fn no_such_sheet(&self, sheet: u32) -> Error {
        Error::with_index(
            ErrorCode::IndexOutOfRange,
            format!(
                "sheet index {sheet} is out of range: the workbook lists {} sheet(s)",
                self.workbook.sheets().len()
            ),
            sheet,
        )
    }
}

/// One `x:workbookView`, decoded: where the window sat and what it showed.
///
/// Screen geometry is in the producer's own units on the producer's own display, so it means nothing
/// anywhere else. It is carried because a file that loses it reopens differently.
///
/// Flat where [`mjx_xlsx::WorkbookWindow`] pairs its coordinates, because a tuple has no name in
/// Python or TypeScript and this is the shape both bindings project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WorkbookWindowInfo {
    /// `@activeTab` — the index in the tab list of the tab that was selected.
    pub active_tab_index: u32,
    /// `@firstSheet` — the index of the leftmost tab shown in the tab strip.
    pub first_visible_tab_index: u32,
    /// `@xWindow`, or `None` if the file wrote neither coordinate.
    pub window_left: Option<i32>,
    /// `@yWindow`, on the same terms.
    pub window_top: Option<i32>,
    /// `@windowWidth`, or `None` if the file wrote neither dimension.
    pub window_width: Option<u32>,
    /// `@windowHeight`, on the same terms.
    pub window_height: Option<u32>,
    /// `@tabRatio` — how much of the horizontal scrollbar area the tab strip took, in thousandths.
    pub tab_strip_ratio: u32,
    /// `@showSheetTabs` — whether the tab strip was shown at all.
    pub show_sheet_tabs: bool,
}

/// One model sheet as the facade states it.
fn summary(sheet: &mjx_xlsx::Sheet) -> SheetSummary {
    SheetSummary {
        name: sheet.name.clone(),
        sheet_id: sheet.sheet_id,
        is_visible: sheet.is_visible(),
        kind: sheet.kind,
        part: sheet.part.as_ref().map(|part| part.as_str().to_owned()),
    }
}
