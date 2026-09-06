//! Hyperlinks on a sheet: what a file says, and the two edits that change it.
//!
//! [`mjx_xlsx::SheetHyperlink`] is already owned, but it carries an [`mjx_sml::CellRange`] and an
//! [`mjx_opc::TargetMode`]; [`SheetHyperlinkInfo`] is the same value with the range as A1 text and
//! the mode as the `bool` a binding can actually use, which is the only reason a second type exists.
//!
//! **The pair is one thing.** An external hyperlink is an entry in the worksheet *and* a
//! relationship in the sheet's `.rels`, and neither half is written or removed without the other —
//! that is the whole point of `mjx-xlsx`'s tier, and this facade inherits it rather than restating
//! it.

use mjx_sml::{CellRange, CellReference};
use mjx_xlsx::{HyperlinkKind, HyperlinkTarget};

use crate::error::Error;
use crate::index::index;

use super::Workbook;

/// One `x:hyperlink` on a sheet, resolved against the sheet's relationships and decoded.
///
/// Every field is what the file *says*. [`target`](Self::target) is the relationship's `Target`
/// exactly as the `.rels` wrote it, and it is `None` both when the entry names no relationship and
/// when it names one the sheet does not declare — [`relationship_id`](Self::relationship_id) tells
/// those two apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetHyperlinkInfo {
    /// `@ref` — the range the link covers, as A1 text. A single-cell link is the degenerate form
    /// (`"B7"`, not `"B7:B7"`), because those are different bytes and stay different.
    pub range: String,
    /// Which of `CT_Hyperlink`'s four shapes this entry is.
    pub kind: HyperlinkKind,
    /// `@r:id`, exactly as written, or `None` when the entry names no relationship.
    pub relationship_id: Option<String>,
    /// The `Target` of the relationship `@r:id` names, exactly as the `.rels` wrote it. Never
    /// resolved, rewritten or fetched.
    pub target: Option<String>,
    /// Whether that relationship's `TargetMode` is `External` — which it is for every hyperlink
    /// Excel writes. `None` when there is no resolvable relationship to ask.
    pub target_is_external: Option<bool>,
    /// `@location` — a cell reference or a defined name inside this workbook.
    pub location: Option<String>,
    /// `@tooltip` — the hover text.
    pub tooltip: Option<String>,
    /// `@display` — the text a consumer shows. Never kept in step with the cell's own value.
    pub display: Option<String>,
}

impl Workbook {
    /// Every hyperlink on one sheet, in document order.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `sheet` names no tab,
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if it holds no worksheet, or
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if an entry's `@ref` does
    /// not parse.
    pub fn sheet_hyperlinks(&self, sheet: u32) -> Result<Vec<SheetHyperlinkInfo>, Error> {
        Ok(self
            .workbook
            .sheet_hyperlinks(index(sheet))?
            .into_iter()
            .map(info)
            .collect())
    }

    /// The hyperlink whose range covers `reference`, or `None` when none does.
    ///
    /// # Errors
    /// As [`sheet_hyperlinks`](Self::sheet_hyperlinks), plus the same
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if `reference` is not an A1
    /// cell.
    pub fn cell_hyperlink(
        &self,
        sheet: u32,
        reference: &str,
    ) -> Result<Option<SheetHyperlinkInfo>, Error> {
        let reference = CellReference::parse(reference)?;
        Ok(self
            .workbook
            .cell_hyperlink(index(sheet), reference)?
            .map(info))
    }

    /// Points `range` at an external URL, writing the entry **and** its `External` relationship.
    ///
    /// An existing entry over exactly the same range is replaced. The cell's own value is not
    /// touched: a hyperlink in SpreadsheetML is a property of a region, not of a cell's text.
    ///
    /// # Errors
    /// As [`sheet_hyperlinks`](Self::sheet_hyperlinks), plus
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if `range` is not an A1
    /// range.
    pub fn set_cell_hyperlink_url(
        &mut self,
        sheet: u32,
        range: &str,
        url: &str,
    ) -> Result<(), Error> {
        let range = CellRange::parse(range)?;
        Ok(self.workbook.set_cell_hyperlink(
            index(sheet),
            range,
            &HyperlinkTarget::Url(url.to_owned()),
        )?)
    }

    /// Points `range` at a location inside this workbook — a cell reference such as `Sheet2!A1`, or
    /// a defined name.
    ///
    /// Written as the entry's `@location` and **no relationship at all**, unlike PowerPoint's slide
    /// jump, which is an internal relationship.
    ///
    /// # Errors
    /// As [`set_cell_hyperlink_url`](Self::set_cell_hyperlink_url).
    pub fn set_cell_hyperlink_location(
        &mut self,
        sheet: u32,
        range: &str,
        location: &str,
    ) -> Result<(), Error> {
        let range = CellRange::parse(range)?;
        Ok(self.workbook.set_cell_hyperlink(
            index(sheet),
            range,
            &HyperlinkTarget::Location(location.to_owned()),
        )?)
    }

    /// Removes the hyperlink covering `reference`, and the relationship it named, answering whether
    /// one was there.
    ///
    /// # Errors
    /// As [`cell_hyperlink`](Self::cell_hyperlink).
    pub fn remove_cell_hyperlink(&mut self, sheet: u32, reference: &str) -> Result<bool, Error> {
        let reference = CellReference::parse(reference)?;
        Ok(self
            .workbook
            .remove_cell_hyperlink(index(sheet), reference)?)
    }
}

/// One model hyperlink as the facade states it.
fn info(link: mjx_xlsx::SheetHyperlink) -> SheetHyperlinkInfo {
    let kind = link.kind();
    SheetHyperlinkInfo {
        range: link.range.text().as_str().to_owned(),
        kind,
        relationship_id: link.relationship_id,
        target: link.target,
        target_is_external: link
            .target_mode
            .map(|mode| mode == mjx_xlsx::TargetMode::External),
        location: link.location,
        tooltip: link.tooltip,
        display: link.display,
    }
}
