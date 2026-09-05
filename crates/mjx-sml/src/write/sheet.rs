//! One authored worksheet: its tab name, its cells, and the cached bounding box over them.
//!
//! # This type adds a name to a [`WorksheetPart`] and nothing else
//!
//! `CT_Worksheet` does not carry the sheet's name — `xl/workbook.xml`'s `sheet@name` does, and the
//! two parts are joined by a relationship (`crates/mjx-sml/src/workbook/sheets.rs` says why the
//! relationship and not the position is what names the part). A package writer therefore has to
//! carry the pair, and this is the pair.
//!
//! Everything else here is a thin call into MJXOFF-102's frame and MJXOFF-95's packed store:
//! [`set_cell_value`](AuthoredWorksheet::set_cell_value) is [`WorksheetPart::set_cell_value`], which
//! creates the row, the cell and the `sheetData` element as needed and widens `dimension` when the
//! cell falls outside it.
//!
//! # The seed, and why `dimension` starts absent
//!
//! The part is seeded as `<worksheet xmlns="…"><sheetData/></worksheet>` and **parsed**, for the
//! reason `crates/mjx-sml/src/write/stylesheet.rs` gives at length: a freshly constructed root has
//! no namespace declaration to inherit, and a part that loses one comes back empty on the next open
//! with every gate still green.
//!
//! There is no `dimension` in the seed, because a sheet with no populated cell has no bounding box
//! to cache — `mjx_chart::EmbeddedWorkbook::dimension` answers `None` for exactly that case and
//! writes no element. [`recompute_dimension`](AuthoredWorksheet::recompute_dimension) is what puts
//! one there once there are cells, and [`WorksheetPart`]'s own rank table is what places it at rank
//! 1, before `sheetData` at rank 5.

use crate::address::{CellRange, CellReference};
use crate::cells::CellValue;
use crate::error::SmlError;
use crate::worksheet::{SheetDimension, WorksheetPart};

use super::constants::XML_DECLARATION;

/// A worksheet being authored: the name its tab carries, and the part its cells live in.
#[derive(Debug)]
pub struct AuthoredWorksheet {
    name: String,
    part: WorksheetPart,
}

impl AuthoredWorksheet {
    /// The bytes a worksheet part is seeded from.
    ///
    /// `sheetData` is in the seed because `CT_Worksheet` declares it `minOccurs="1"` — a worksheet
    /// without one is invalid however few cells it has — and it is written self-closing because
    /// that is what a producer writes for an empty sheet, and because the packed store then keeps
    /// the self-closing form until there is a row to put in it.
    fn seed_bytes() -> Vec<u8> {
        format!(
            r#"{XML_DECLARATION}<worksheet xmlns="{}"><sheetData/></worksheet>"#,
            mjx_ooxml_types::namespaces::SML.transitional
        )
        .into_bytes()
    }

    /// An empty sheet named `name`.
    ///
    /// # Errors
    /// [`SmlError`] if the seed does not parse or its root is not an `x:worksheet` — neither is
    /// reachable, because the seed is a literal in this file.
    pub fn new(name: &str) -> Result<Self, SmlError> {
        let part = WorksheetPart::read_part(&Self::seed_bytes())?.ok_or(
            SmlError::AuthoredPartSeedRejected {
                part: "/xl/worksheets/sheetN.xml",
            },
        )?;
        Ok(Self {
            name: name.to_owned(),
            part,
        })
    }

    /// The sheet's tab name — what `xl/workbook.xml` writes as `sheet@name` and what a chart's
    /// `c:f` formulas qualify their ranges with.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Renames the tab. The part is untouched: a sheet's name is not in its own markup.
    pub fn set_name(&mut self, name: &str) {
        self.name.clear();
        self.name.push_str(name);
    }

    /// The worksheet markup, for a caller reading back what was authored.
    #[must_use]
    pub fn part(&self) -> &WorksheetPart {
        &self.part
    }

    /// The worksheet markup, mutably — for the thirty-two slots this writer does not author itself.
    pub fn part_mut(&mut self) -> &mut WorksheetPart {
        &mut self.part
    }

    /// Sets one cell's value, creating the cell and its row.
    ///
    /// [`CellValue::Blank`] still creates the cell: a blank cell carries a style and is a different
    /// statement from no cell at all. A writer laying out a grid with holes in it — which is what a
    /// chart's header row is — should **skip** the position rather than write a blank there, which
    /// is what [`WorkbookPackage::push_row`](crate::write::WorkbookPackage::push_row) does.
    ///
    /// # Errors
    /// [`SmlError::UnrepresentableNumber`] for a non-finite number, or
    /// [`SmlError::PackedStoreTooLarge`] past the store's four-gigabyte byte space.
    pub fn set_cell_value(
        &mut self,
        reference: CellReference,
        value: CellValue<'_>,
    ) -> Result<(), SmlError> {
        self.part.set_cell_value(reference, value)
    }

    /// Sets one cell's `cellXfs` index (`c@s`), creating a blank cell if there is none.
    ///
    /// # Errors
    /// As [`set_cell_value`](Self::set_cell_value).
    pub fn set_cell_style(
        &mut self,
        reference: CellReference,
        style: Option<u32>,
    ) -> Result<(), SmlError> {
        self.part
            .sheet_data_or_insert()
            .set_cell_style(reference, style)
    }

    /// How many cells the sheet holds.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.part.cell_count()
    }

    /// Replaces `x:dimension` with the box the populated cells occupy, inserting the element when
    /// the sheet has none, and answers the range written.
    ///
    /// `None` — and no element — for a sheet with no populated cell. That is not a defensive
    /// nicety: `@ref` is `use="required"` on `CT_SheetDimension`, and `ref=""` is not an `ST_Ref`,
    /// so a sheet with nothing in it has no schema-valid dimension to write and must write none.
    /// `mjx_chart::EmbeddedWorkbook` reached the same answer from the other side, by returning
    /// `None` from its own `dimension()` for an empty grid.
    ///
    /// The box itself is [`WorksheetPart::recompute_dimension`]'s — this method only makes sure
    /// there is an element for it to write into, placed through
    /// [`mjx_ooxml_types::child_order::WORKSHEET`] so that it lands at rank 1, before `sheetData` at
    /// rank 5, however many other slots the part already holds.
    pub fn recompute_dimension(&mut self) -> Option<CellRange> {
        if self.part.dimension().is_none() {
            if self.part.cell_count() == 0 {
                return None;
            }
            // The interner is swapped out and back because the constructor wants it mutably at the
            // same moment `self.part` is borrowed mutably, and the two live in one struct. Swapping
            // is a pointer move of an empty `Interner`, not a rebuild of this one — the same trick
            // `WorksheetPart::write_dimension` uses one layer down, and for the same reason.
            let mut interner = mjx_ooxml_core::Interner::default();
            core::mem::swap(&mut interner, self.part.interner_mut());
            let dimension = SheetDimension::new(&mut interner, None);
            core::mem::swap(&mut interner, self.part.interner_mut());
            self.part.set_dimension(Some(dimension));
        }
        match self.part.recompute_dimension() {
            Some(range) => Some(range),
            // Unreachable while `cell_count()` is non-zero, and handled rather than left: an
            // element with no `@ref` is invalid markup, so it is taken back out again.
            None => {
                self.part.set_dimension(None);
                None
            }
        }
    }

    /// The whole part as bytes.
    #[must_use]
    pub fn to_part_bytes(&self) -> Vec<u8> {
        self.part.to_markup()
    }
}
