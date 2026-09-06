//! Print setup and the two opaque parts a sheet can reach: the printer-settings blob and the
//! background picture.
//!
//! Both are named as `&str` part names rather than [`mjx_opc::PartName`] handles, the same
//! translation every part-graph accessor on this facade applies.

use crate::error::Error;
use crate::index::index;

use super::Workbook;

impl Workbook {
    /// The printer-settings part one sheet reaches, or `None` when it reaches none.
    ///
    /// **Opaque bytes, never XML** — a Windows `DEVMODE`/`DEVNAMES` blob written by a driver on a
    /// machine that is not this one. It is preserved verbatim and nothing here reads it; the part
    /// name is here so a caller can find it, copy it, or notice it exists.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `sheet` names no tab, or
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if the sheet's part or
    /// relationships cannot be read.
    pub fn sheet_printer_settings(&self, sheet: u32) -> Result<Option<String>, Error> {
        Ok(self
            .workbook
            .sheet_printer_settings(index(sheet))?
            .map(|part| part.as_str().to_owned()))
    }

    /// The background-picture part one sheet reaches (`x:picture`), or `None` when it reaches none.
    ///
    /// # Errors
    /// As [`sheet_printer_settings`](Self::sheet_printer_settings).
    pub fn sheet_background_image(&self, sheet: u32) -> Result<Option<String>, Error> {
        Ok(self
            .workbook
            .sheet_background_image(index(sheet))?
            .map(|part| part.as_str().to_owned()))
    }
}
