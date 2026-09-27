//! Frozen and split panes — up to four independently scrolled regions over **one** grid geometry.
//!
//! # What a pane is, and what it is not
//!
//! `x:sheetView/pane` divides the window, not the sheet. `@xSplit` and `@ySplit` mean two different
//! things depending on `@state`, and `mjx-sml` deliberately converts neither:
//!
//! * [`PaneState::Frozen`] (and `frozenSplit`) — a **number of columns and rows**. `xSplit="2"
//!   ySplit="1"` freezes columns A and B and row 1.
//! * [`PaneState::Split`] — a position in **twentieths of a point of window space**. The divider is
//!   a draggable bar rather than a frozen edge, and the panes on either side of it scroll
//!   independently in the axis it divides.
//!
//! Both produce the same thing here: up to four [`PaneRegion`]s, each a rectangle of the *view* and
//! a range of rows and columns of the *sheet*, all four measured with the one [`GridGeometry`]. That
//! sharing is the property worth stating — a frozen row is the same height in the top-left pane and
//! the top-right one, because there is one row geometry and four windows onto it.
//!
//! # ⚠ What a split pane's offset means is a reading
//!
//! GUESS: a split's `@xSplit`/`@ySplit` are converted from twentieths of a point to EMU and used as
//! the divider's position in the view, and the columns and rows on the far side start from
//! `@topLeftCell`. Excel's own behaviour for a split whose divider falls part-way through a column
//! — does the partial column show, and is `topLeftCell` then the partial one or the next whole one?
//! — is not written down in the specification and has not been observed on Windows.
//!
//! A **frozen** pane needs no such reading: the split is a column and row count, and the frozen
//! region is exactly those columns and rows.

use mjx_layout::LayoutRect;
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_core::measure::EMU_PER_POINT;
use mjx_ooxml_types::spreadsheetml::{Pane, PaneState};
use mjx_sml::{CellReference, SheetPane, WorksheetPart};

use crate::geometry::GridGeometry;

/// How a sheet's window is divided.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PaneSplit {
    /// How many columns are frozen at the left, for a frozen split. Zero when nothing is.
    pub frozen_columns: u16,
    /// How many rows are frozen at the top. Zero when nothing is.
    pub frozen_rows: u32,
    /// Where the vertical divider is in the view, for a *split* rather than a freeze.
    pub split_x: Emu,
    /// Where the horizontal divider is.
    pub split_y: Emu,
    /// The cell the scrolling pane starts at — `pane@topLeftCell`.
    pub top_left: Option<(u32, u16)>,
    /// Which pane the file says is active.
    pub active: Pane,
    /// Whether this is a freeze or a draggable split.
    pub state: PaneState,
}

impl Default for PaneSplit {
    /// An undivided window.
    ///
    /// Written out rather than derived because neither `ST_Pane` nor `ST_PaneState` has a `Default`
    /// — and neither should: `topLeft` and `split` are the *schema's* defaults for the attributes of
    /// an element that is present, which is a different claim from "this sheet has no pane".
    fn default() -> Self {
        Self {
            frozen_columns: 0,
            frozen_rows: 0,
            split_x: Emu::ZERO,
            split_y: Emu::ZERO,
            top_left: None,
            active: Pane::TopLeft,
            state: PaneState::Split,
        }
    }
}

impl PaneSplit {
    /// Whether the window is divided at all.
    #[must_use]
    pub fn is_divided(&self) -> bool {
        self.frozen_columns > 0
            || self.frozen_rows > 0
            || self.split_x > Emu::ZERO
            || self.split_y > Emu::ZERO
    }

    /// Whether it is divided horizontally — whether there is a left pane distinct from a right one.
    #[must_use]
    pub fn divides_columns(&self) -> bool {
        self.frozen_columns > 0 || self.split_x > Emu::ZERO
    }

    /// Whether it is divided vertically.
    #[must_use]
    pub fn divides_rows(&self) -> bool {
        self.frozen_rows > 0 || self.split_y > Emu::ZERO
    }

    /// The split the sheet's **first** `sheetView` states, or the undivided default.
    ///
    /// The first view, not all of them: `sheetViews` holds one `sheetView` per workbook *window*,
    /// and a renderer draws one window. A caller that wants another passes it to
    /// [`PaneSplit::from_pane`] itself.
    #[must_use]
    pub fn read(worksheet: &WorksheetPart) -> Self {
        let Some(views) = worksheet.sheet_views() else {
            return Self::default();
        };
        let Some(pane) = views.views().next().and_then(mjx_sml::SheetView::pane) else {
            return Self::default();
        };
        Self::from_pane(pane, worksheet.interner())
    }

    /// The split one `x:pane` element states.
    #[must_use]
    pub fn from_pane(pane: &SheetPane, interner: &mjx_ooxml_core::Interner) -> Self {
        let state = pane.state(interner).unwrap_or(PaneState::Split);
        let horizontal = pane.horizontal_split(interner).unwrap_or(0.0);
        let vertical = pane.vertical_split(interner).unwrap_or(0.0);
        let horizontal = if horizontal.is_finite() && horizontal > 0.0 {
            horizontal
        } else {
            0.0
        };
        let vertical = if vertical.is_finite() && vertical > 0.0 {
            vertical
        } else {
            0.0
        };
        let frozen = matches!(state, PaneState::Frozen | PaneState::FrozenSplit);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (frozen_columns, frozen_rows) = if frozen {
            (
                (horizontal.min(f64::from(crate::geometry::COLUMN_COUNT)) as u32) as u16,
                vertical.min(f64::from(crate::geometry::ROW_COUNT)) as u32,
            )
        } else {
            (0, 0)
        };
        let (split_x, split_y) = if frozen {
            (Emu::ZERO, Emu::ZERO)
        } else {
            // `@xSplit` is in twentieths of a point when the pane is a draggable split — the same
            // unit a twip is, which is what `EMU_PER_POINT / 20` says.
            (
                Emu::from_emu_rounded(horizontal * (EMU_PER_POINT as f64) / 20.0),
                Emu::from_emu_rounded(vertical * (EMU_PER_POINT as f64) / 20.0),
            )
        };
        Self {
            frozen_columns,
            frozen_rows,
            split_x,
            split_y,
            top_left: pane
                .top_left_cell(interner)
                .ok()
                .flatten()
                .map(|cell: CellReference| (cell.row(), cell.column())),
            active: pane.active_pane(interner).unwrap_or(Pane::TopLeft),
            state,
        }
    }
}

/// One of the up-to-four regions a divided window shows.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PaneRegion {
    /// Which of the four this is.
    pub pane: Pane,
    /// Where it sits in the page, in page coordinates.
    pub view: LayoutRect,
    /// Which rows of the sheet it shows, zero-based and half-open.
    pub rows: std::ops::Range<u32>,
    /// Which columns it shows, zero-based and half-open.
    pub columns: std::ops::Range<u32>,
    /// Where the sheet-coordinate origin of this region maps to in the page — subtract a sheet
    /// position from this to place a fragment.
    pub origin: mjx_layout::LayoutPoint,
    /// Whether the region's rows are frozen, and therefore repeat on every band.
    pub rows_are_frozen: bool,
    /// Whether its columns are frozen.
    pub columns_are_frozen: bool,
}

impl PaneRegion {
    /// Whether the region shows nothing — an empty row or column range, or no room on the page.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() || self.columns.is_empty() || self.view.is_empty()
    }
}

/// The window a caller is looking at the sheet through: which cell is at the top left of the
/// **scrolling** region, and how much room the page gives it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Window {
    /// The first row of the scrolling region, zero-based.
    pub first_row: u32,
    /// The first column of the scrolling region, zero-based.
    pub first_column: u16,
    /// The area of the page the grid is drawn into.
    pub content: LayoutRect,
}

/// Divides `window` into up to four regions under `split`, all measured through `geometry`.
///
/// The regions come back in **paint order**: the scrolling region first and the frozen corner last,
/// so that a frozen row painted over a scrolled cell is the last thing drawn, which is what a reader
/// sees in Excel.
#[must_use]
pub fn regions(geometry: &GridGeometry, split: &PaneSplit, window: &Window) -> Vec<PaneRegion> {
    let content = window.content;
    let rows = geometry.rows();
    let columns = geometry.columns();

    // The frozen bands, measured in sheet coordinates from the sheet's own origin.
    let frozen_height = if split.frozen_rows > 0 {
        rows.top(split.frozen_rows)
    } else {
        split.split_y.minimum(content.height())
    };
    let frozen_width = if split.frozen_columns > 0 {
        columns.left(split.frozen_columns)
    } else {
        split.split_x.minimum(content.width())
    };
    let frozen_height = frozen_height.maximum(Emu::ZERO).minimum(content.height());
    let frozen_width = frozen_width.maximum(Emu::ZERO).minimum(content.width());

    // The scrolling region starts where the divider leaves off.
    let scroll_left = content.left + frozen_width;
    let scroll_top = content.top + frozen_height;

    // Which rows and columns each half shows. The frozen half always starts at the sheet's own
    // origin — that is what "frozen" means — and the scrolling half at the window's anchor.
    let frozen_rows = 0..split.frozen_rows;
    let frozen_columns = 0..u32::from(split.frozen_columns);
    let scroll_rows = window.first_row..crate::geometry::ROW_COUNT;
    let scroll_columns = u32::from(window.first_column)..crate::geometry::COLUMN_COUNT;

    let mut out: Vec<PaneRegion> = Vec::with_capacity(4);
    let push = |out: &mut Vec<PaneRegion>,
                pane: Pane,
                view: LayoutRect,
                row_range: std::ops::Range<u32>,
                column_range: std::ops::Range<u32>,
                rows_frozen: bool,
                columns_frozen: bool| {
        if view.is_empty() || row_range.is_empty() || column_range.is_empty() {
            return;
        }
        let sheet_top = rows.top(row_range.start);
        let sheet_left = columns.left(u16::try_from(column_range.start).unwrap_or(u16::MAX));
        out.push(PaneRegion {
            pane,
            view,
            rows: row_range,
            columns: column_range,
            origin: mjx_layout::LayoutPoint {
                x: view.left - sheet_left,
                y: view.top - sheet_top,
            },
            rows_are_frozen: rows_frozen,
            columns_are_frozen: columns_frozen,
        });
    };

    push(
        &mut out,
        Pane::BottomRight,
        LayoutRect::from_edges(scroll_left, scroll_top, content.right, content.bottom),
        scroll_rows.clone(),
        scroll_columns.clone(),
        false,
        false,
    );
    if frozen_width > Emu::ZERO {
        push(
            &mut out,
            Pane::BottomLeft,
            LayoutRect::from_edges(content.left, scroll_top, scroll_left, content.bottom),
            scroll_rows,
            frozen_columns.clone(),
            false,
            true,
        );
    }
    if frozen_height > Emu::ZERO {
        push(
            &mut out,
            Pane::TopRight,
            LayoutRect::from_edges(scroll_left, content.top, content.right, scroll_top),
            frozen_rows.clone(),
            scroll_columns,
            true,
            false,
        );
    }
    if frozen_width > Emu::ZERO && frozen_height > Emu::ZERO {
        push(
            &mut out,
            Pane::TopLeft,
            LayoutRect::from_edges(content.left, content.top, scroll_left, scroll_top),
            frozen_rows,
            frozen_columns,
            true,
            true,
        );
    }
    if out.is_empty() {
        // An undivided window is one region, and saying so with a value is what keeps every caller
        // from having to special-case the common sheet.
        push(
            &mut out,
            Pane::TopLeft,
            content,
            window.first_row..crate::geometry::ROW_COUNT,
            u32::from(window.first_column)..crate::geometry::COLUMN_COUNT,
            false,
            false,
        );
    }
    out
}
