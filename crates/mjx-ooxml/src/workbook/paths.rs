//! The part graph, named as `&str`.
//!
//! A [`mjx_opc::PartName`] is a validated handle, and a binding cannot carry one across the boundary
//! and back — so this facade speaks part names as strings, exactly as [`crate::Deck`] does for the
//! ink, VML and diagram byte windows it keeps.
//!
//! There is no `Workbook::package`: handing out the whole part graph would let a caller write a
//! package [`Workbook::save`]'s validation pass could no longer vouch for. What is here is enough to
//! *see* the graph — which parts are there, what each is, and what one holds.

use crate::error::{Error, ErrorCode};
use crate::index::part_name;

use super::Workbook;

impl Workbook {
    /// The workbook part the package's `officeDocument` relationship names — `"/xl/workbook.xml"` in
    /// everything a real producer writes, though nothing requires that spelling.
    #[must_use]
    pub fn workbook_part(&self) -> String {
        self.workbook.workbook_part().as_str().to_owned()
    }

    /// Every part in the package, in the order the container holds them.
    #[must_use]
    pub fn part_names(&self) -> Vec<String> {
        self.workbook
            .package()
            .part_names()
            .map(|part| part.as_str().to_owned())
            .collect()
    }

    /// The content type of one part, or `None` when the package holds no such part.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`] if `part` is not a
    /// well-formed part name.
    pub fn content_type_of(&self, part: &str) -> Result<Option<String>, Error> {
        let part = part_name(part)?;
        Ok(self
            .workbook
            .package()
            .content_type_of(&part)
            .map(str::to_owned))
    }

    /// The bytes of one part, exactly as the package holds them.
    ///
    /// The door to everything this facade does not model: a pivot cache, a drawing, a printer
    /// settings blob, an `x:metadata`. Reading never dirties a part, so the workbook still saves
    /// byte-identically afterwards.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`] if `part` is not a
    /// well-formed part name, or [`ErrorCode::NotFound`] if the package holds no such part.
    pub fn part_bytes(&self, part: &str) -> Result<Vec<u8>, Error> {
        let name = part_name(part)?;
        self.workbook
            .package()
            .part_bytes(&name)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| {
                Error::new(
                    ErrorCode::NotFound,
                    format!("the package holds no part named {part}"),
                )
            })
    }
}
