//! `x:bookViews` — how a consumer opens the workbook window, as a caller holding a [`Workbook`]
//! sees it.
//!
//! [`mjx_sml::WorkbookView`] is the markup; [`WorkbookWindow`] is the decoded snapshot, for the same
//! reason [`super::properties`] gives. The custom-view list (`x:customWorkbookViews`) has no
//! shortcut here and is reached through [`Workbook::workbook_markup`]: it is a list of
//! twenty-four-attribute saved views that no navigation question is about.
//!
//! # `activeTab` is an index, and it is not checked
//!
//! `@activeTab` and `@firstSheet` are positions in the tab list, and `sml.xsd` constrains neither
//! against the number of tabs. [`WorkbookWindow::active_tab_index`] is the number the file wrote.
//! [`Workbook::active_sheet`] is the shortcut that resolves it, and it answers `None` when the index
//! names no tab — reporting that the file is wrong, rather than clamping it to the last tab and
//! pretending it was right.

use crate::error::XlsxError;

use super::{Sheet, Workbook};

/// One `x:workbookView`, decoded: where the window sat and what it showed.
///
/// Screen geometry is in the producer's own units on the producer's own display, so it means
/// nothing anywhere else. It is carried because a file that loses it reopens differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WorkbookWindow {
    /// `@activeTab` — the index in the tab list of the tab that was selected.
    pub active_tab_index: u32,
    /// `@firstSheet` — the index of the leftmost tab shown in the tab strip.
    pub first_visible_tab_index: u32,
    /// `@xWindow` / `@yWindow` — the window's top-left corner, or `None` if the file wrote neither.
    pub window_position: Option<(i32, i32)>,
    /// `@windowWidth` / `@windowHeight`, or `None` if the file wrote neither.
    pub window_size: Option<(u32, u32)>,
    /// `@tabRatio` — how much of the horizontal scrollbar area the tab strip took, in thousandths.
    pub tab_strip_ratio: u32,
    /// `@showSheetTabs` — whether the tab strip was shown at all.
    pub show_sheet_tabs: bool,
}

impl Workbook {
    /// Every `x:workbookView`, decoded, in document order.
    ///
    /// Empty for a workbook that writes no `bookViews` — which is legal, and means a consumer picks
    /// its own window.
    ///
    /// # Errors
    /// Returns [`XlsxError`] if the workbook part cannot be read, or if one of the geometry
    /// attributes holds a value its declared type rejects.
    pub fn window_views(&mut self) -> Result<Vec<WorkbookWindow>, XlsxError> {
        self.workbook_markup(|part, interner| {
            let Some(views) = part.book_views() else {
                return Ok(Vec::new());
            };
            views
                .views()
                .map(|view| {
                    let left = view.window_left(interner)?;
                    let top = view.window_top(interner)?;
                    let width = view.window_width(interner)?;
                    let height = view.window_height(interner)?;
                    Ok(WorkbookWindow {
                        active_tab_index: view.active_tab_index(interner)?,
                        first_visible_tab_index: view.first_visible_tab_index(interner)?,
                        window_position: left.zip(top),
                        window_size: width.zip(height),
                        tab_strip_ratio: view.tab_strip_ratio(interner)?,
                        show_sheet_tabs: view.show_sheet_tabs(interner)?,
                    })
                })
                .collect::<Result<Vec<_>, mjx_ooxml_core::AttributeError>>()
        })?
        .map_err(XlsxError::from)
    }

    /// The tab the first `x:workbookView` says was selected, or `None` if the workbook writes no
    /// view — or if its `@activeTab` names no tab, which is a defect this reports rather than
    /// repairs.
    ///
    /// # Errors
    /// Returns [`XlsxError`] if the workbook part cannot be read or a view attribute is malformed.
    pub fn active_sheet(&mut self) -> Result<Option<&Sheet>, XlsxError> {
        let Some(view) = self.window_views()?.into_iter().next() else {
            return Ok(None);
        };
        let index = usize::try_from(view.active_tab_index).unwrap_or(usize::MAX);
        Ok(self.sheets().get(index))
    }
}
