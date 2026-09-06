//! The sheet tier for the other three sheet kinds, and the two relationships a print block reaches.
//!
//! `mjx-sml` answers *what a chartsheet is*; this file answers *which part in this package is one*,
//! and resolves the two `r:id`s that leave a sheet's print block: the printer-settings blob a
//! `pageSetup` names, and the image a `picture` draws.
//!
//! # Why the four kinds are one enumeration rather than four accessors
//!
//! [`Workbook::worksheet_markup`](Workbook::worksheet_markup) answers `Ok(None)` for a tab that is
//! not a worksheet, which is right for a caller that wants cells and unhelpful for one that wants to
//! know what the tab *is*. [`sheet_markup`](Workbook::sheet_markup) is the other question, and its
//! answer is [`SheetMarkup`] — a four-variant enumeration over the whole workbook.
//!
//! **That enumeration is where "a chartsheet has no cells" becomes a compile error.** Reaching a
//! cell means matching out [`SheetMarkup::Worksheet`] and holding an
//! [`mjx_sml::WorksheetPart`]; the other three variants carry types that have no cell accessor at
//! all. There is no method that answers `None` for a chartsheet's cells, because there is no method.
//!
//! # A macrosheet is recognised by its root element, not by a content type
//!
//! ECMA-376 declares no content type and no relationship type for `CT_Macrosheet` — see
//! [`mjx_sml::sheets::macrosheet`] — so [`crate::SheetKind`], which is defined by content type,
//! stays at the three kinds §12.3.23 names. `sheet_markup` parses the part and dispatches on its
//! **root element**, exactly as `Workbook::open` identifies the workbook part in a `.xlsm`. So a
//! workbook holding a macrosheet opens, reports the tab as an unknown *kind*, and still hands back
//! the markup as [`SheetMarkup::MacroSheet`].
//!
//! # The two relationships, and what resolving one does not mean
//!
//! [`sheet_printer_settings`](Workbook::sheet_printer_settings) and
//! [`sheet_background_image`](Workbook::sheet_background_image) read the `r:id` off the markup and
//! resolve it against the sheet part's own `.rels`. Neither **opens** the part it names: a printer
//! settings blob is a Windows `DEVMODE` this project never interprets, and an image is bytes
//! `mjx-opc` carries verbatim.
//!
//! They differ from [`crate::WorksheetParts`]'s fields of the same names, and the difference
//! matters: `WorksheetParts` says *the sheet relates to a printer-settings part*, and these say *the
//! sheet's `pageSetup` names this relationship*. A sheet can have the first without the second, and
//! [`crate::SpreadsheetDefect::SheetReferenceHasTheWrongRelationshipType`] is what happens when the
//! second points somewhere the first would not.

use mjx_opc::PartName;
use mjx_sml::sheets::{ChartSheetPart, DialogSheetPart, MacroSheetPart};
use mjx_sml::WorksheetPart;

use crate::error::XlsxError;

use crate::workbook::Workbook;

/// The markup behind one tab, whichever of the four sheet kinds it is.
///
/// # Reading a chartsheet's cells is impossible by construction
///
/// Not an empty collection, not a `None`: the [`ChartSheetPart`], [`DialogSheetPart`] and
/// [`MacroSheetPart`] variants carry types with **no cell accessor at all**, so a caller reaches a
/// cell only by matching out [`Worksheet`](Self::Worksheet). The distinction is made once, at this
/// match, rather than at every accessor.
///
/// `#[non_exhaustive]` is deliberately **not** applied: the set is the four content models
/// `sml.xsd` declares for a sheet, and it has not changed since ECMA-376's first edition. A caller
/// matching all four should get a compile error if a fifth ever appears rather than a silent arm.
#[derive(Debug)]
pub enum SheetMarkup {
    /// `x:worksheet` — the ordinary grid of cells (`CT_Worksheet`, §12.3.24).
    Worksheet(WorksheetPart),
    /// `x:chartsheet` — one chart occupying a whole tab (`CT_Chartsheet`, §12.3.2). **No cells.**
    ChartSheet(ChartSheetPart),
    /// `x:dialogsheet` — a legacy Excel 5.0 dialog (`CT_Dialogsheet`, §12.3.7). **No cells.**
    DialogSheet(DialogSheetPart),
    /// A macrosheet (`CT_Macrosheet`) — a complex type ECMA-376 declares no global element, no
    /// content type and no relationship type for, reached here by its root element. **No cells
    /// modelled:** its `sheetData` is held verbatim, see [`mjx_sml::sheets::macrosheet`].
    MacroSheet(MacroSheetPart),
}

impl SheetMarkup {
    /// The part's root element name, as `sml.xsd` spells it.
    #[must_use]
    pub fn root_element(&self) -> &'static str {
        match self {
            Self::Worksheet(_) => "worksheet",
            Self::ChartSheet(_) => "chartsheet",
            Self::DialogSheet(_) => "dialogsheet",
            Self::MacroSheet(_) => "macrosheet",
        }
    }

    /// The whole part as bytes — what [`Workbook::write_sheet_markup`] writes.
    #[must_use]
    pub fn to_markup(&self) -> Vec<u8> {
        match self {
            Self::Worksheet(part) => part.to_markup(),
            Self::ChartSheet(part) => part.to_markup(),
            Self::DialogSheet(part) => part.to_markup(),
            Self::MacroSheet(part) => part.to_markup(),
        }
    }

    /// Whether the whole part can still be written straight out of the bytes it was read from.
    #[must_use]
    pub fn is_verbatim(&self) -> bool {
        match self {
            Self::Worksheet(part) => part.is_verbatim(),
            Self::ChartSheet(part) => part.is_verbatim(),
            Self::DialogSheet(part) => part.is_verbatim(),
            Self::MacroSheet(part) => part.is_verbatim(),
        }
    }

    /// The `r:id` this sheet's `x:pageSetup` names, and the prefix the part bound the
    /// relationship-reference namespace to.
    ///
    /// `None` when the sheet writes no `pageSetup`, when that `pageSetup` carries no `r:id`, or when
    /// the part binds the relationship-reference namespace to no prefix at all — in which case no
    /// element in it can spell an `r:id`.
    ///
    /// # Errors
    /// [`XlsxError::Sml`] if the attribute is present but will not decode.
    pub fn printer_settings_relationship(&self) -> Result<Option<String>, XlsxError> {
        Ok(match self {
            Self::Worksheet(part) => part.page_setup().map_or(Ok(None), |setup| {
                setup.relationship_id(part.interner(), part.relationship_prefix())
            })?,
            Self::ChartSheet(part) => part.page_setup().map_or(Ok(None), |setup| {
                setup.relationship_id(part.interner(), part.relationship_prefix())
            })?,
            Self::DialogSheet(part) => part.page_setup().map_or(Ok(None), |setup| {
                setup.relationship_id(part.interner(), part.relationship_prefix())
            })?,
            Self::MacroSheet(part) => part.page_setup().map_or(Ok(None), |setup| {
                setup.relationship_id(part.interner(), part.relationship_prefix())
            })?,
        })
    }

    /// The `r:id` this sheet's `x:picture` names — its background image.
    ///
    /// `None` for a dialogsheet, whose content model has no `picture` slot at all, and for a sheet
    /// that writes none.
    ///
    /// # Errors
    /// [`XlsxError::Sml`] if the attribute is present but will not decode.
    pub fn background_picture_relationship(&self) -> Result<Option<String>, XlsxError> {
        Ok(match self {
            Self::Worksheet(part) => part.background_picture().map_or(Ok(None), |picture| {
                picture.relationship_id(part.interner(), part.relationship_prefix())
            })?,
            Self::ChartSheet(part) => part.background_picture().map_or(Ok(None), |picture| {
                picture.relationship_id(part.interner(), part.relationship_prefix())
            })?,
            Self::MacroSheet(part) => part.background_picture().map_or(Ok(None), |picture| {
                picture.relationship_id(part.interner(), part.relationship_prefix())
            })?,
            // `CT_Dialogsheet` declares no `picture` slot.
            Self::DialogSheet(_) => None,
        })
    }
}

impl Workbook {
    /// Reads the sheet part behind the tab at `index` into whichever of the four models it is.
    ///
    /// `Ok(None)` when the tab reaches no part at all, or when the part it reaches is rooted in
    /// something that is not a sheet — which is a question rather than an error, exactly as
    /// [`worksheet_markup`](Workbook::worksheet_markup) treats a chartsheet.
    ///
    /// Reading does not dirty the package: the part keeps its container bytes and
    /// [`save`](Workbook::save) still re-emits them verbatim.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, [`XlsxError::MissingWorkbookPart`] if the
    /// package holds no such part, or [`XlsxError::Sml`] if the part is not well-formed or a
    /// modelled element does not match its complex type.
    pub fn sheet_markup(&self, index: usize) -> Result<Option<SheetMarkup>, XlsxError> {
        let sheets = self.sheets().len();
        let sheet = self
            .sheets()
            .get(index)
            .ok_or(XlsxError::NoSuchSheet { index, sheets })?;
        let Some(part) = sheet.part.clone() else {
            return Ok(None);
        };
        self.sheet_markup_of(&part)
    }

    /// [`sheet_markup`](Self::sheet_markup) for a caller that already holds the part name — from
    /// [`Worksheet::part`](crate::Worksheet::part), above all.
    ///
    /// # Errors
    /// As [`sheet_markup`](Self::sheet_markup).
    pub fn sheet_markup_of(&self, part: &PartName) -> Result<Option<SheetMarkup>, XlsxError> {
        let Some(bytes) = self.package().part_bytes(part) else {
            return Err(XlsxError::MissingWorkbookPart(part.as_str().to_owned()));
        };
        // Dispatch on the **root element**, not on the content type: a macrosheet has no content
        // type of its own, and a `.xlsm`'s parts are identified the same way. Each reader answers
        // `Ok(None)` for a root it does not recognise, so this is four questions and not a parse of
        // the root by hand.
        if let Some(worksheet) = WorksheetPart::read_part(bytes)? {
            return Ok(Some(SheetMarkup::Worksheet(worksheet)));
        }
        if let Some(chart) = ChartSheetPart::read_part(bytes)? {
            return Ok(Some(SheetMarkup::ChartSheet(chart)));
        }
        if let Some(dialog) = DialogSheetPart::read_part(bytes)? {
            return Ok(Some(SheetMarkup::DialogSheet(dialog)));
        }
        if let Some(macro_sheet) = MacroSheetPart::read_part(bytes)? {
            return Ok(Some(SheetMarkup::MacroSheet(macro_sheet)));
        }
        Ok(None)
    }

    /// Writes `markup` back over the sheet part behind the tab at `index`.
    ///
    /// The part's bytes are replaced with what the model emits, which for a model nothing edited is
    /// the buffer it was read from — so writing back an untouched sheet is a no-op the byte-identity
    /// suites cannot tell from not writing at all.
    ///
    /// **Nothing checks that `markup` is the kind the tab already holds.** Writing a chartsheet's
    /// markup over a worksheet part produces a package whose content type and root element disagree,
    /// which [`validate`](Workbook::validate) reports rather than something this call refuses:
    /// refusing would need a rule about which of the two is the truth, and the file decides that.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, [`XlsxError::MissingWorkbookPart`] if it
    /// reaches no part, or [`XlsxError::Opc`] if the package refuses the replacement.
    pub fn write_sheet_markup(
        &mut self,
        index: usize,
        markup: &SheetMarkup,
    ) -> Result<(), XlsxError> {
        let sheets = self.sheets().len();
        let part = self
            .sheets()
            .get(index)
            .ok_or(XlsxError::NoSuchSheet { index, sheets })?
            .part
            .clone()
            .ok_or_else(|| XlsxError::MissingWorkbookPart(format!("sheet {index}")))?;
        self.package_mut()
            .replace_part_bytes(&part, markup.to_markup())?;
        Ok(())
    }

    /// The printer settings part the tab at `index` names **from its own `x:pageSetup`**.
    ///
    /// `None` when the tab reaches no part, when the sheet writes no `pageSetup`, when that
    /// `pageSetup` carries no `r:id`, or when the relationship it names is not declared. The part's
    /// bytes are never read: this resolves a name.
    ///
    /// Not the same question as [`WorksheetParts::printer_settings`](crate::WorksheetParts) — see
    /// this module's own documentation.
    ///
    /// # Errors
    /// As [`sheet_markup`](Self::sheet_markup), plus [`XlsxError::ExternalTarget`] or
    /// [`XlsxError::TargetResolution`] if the relationship will not resolve to a part name.
    pub fn sheet_printer_settings(&self, index: usize) -> Result<Option<PartName>, XlsxError> {
        self.sheet_reference(index, SheetMarkup::printer_settings_relationship)
    }

    /// The image part the tab at `index` draws behind its cells, named by its own `x:picture`.
    ///
    /// `None` for a dialogsheet, whose content model has no `picture` slot, and for a sheet that
    /// writes none. The image is never decoded.
    ///
    /// # Errors
    /// As [`sheet_printer_settings`](Self::sheet_printer_settings).
    pub fn sheet_background_image(&self, index: usize) -> Result<Option<PartName>, XlsxError> {
        self.sheet_reference(index, SheetMarkup::background_picture_relationship)
    }

    /// Resolves one `r:id` read off the tab's own markup against the sheet part's `.rels`.
    fn sheet_reference(
        &self,
        index: usize,
        read: impl Fn(&SheetMarkup) -> Result<Option<String>, XlsxError>,
    ) -> Result<Option<PartName>, XlsxError> {
        let sheets = self.sheets().len();
        let sheet = self
            .sheets()
            .get(index)
            .ok_or(XlsxError::NoSuchSheet { index, sheets })?;
        let Some(part) = sheet.part.clone() else {
            return Ok(None);
        };
        let Some(markup) = self.sheet_markup_of(&part)? else {
            return Ok(None);
        };
        let Some(relationship_id) = read(&markup)? else {
            return Ok(None);
        };
        let Some(relationships) = self.package().relationships_for(Some(&part)) else {
            return Ok(None);
        };
        let Some(relationship) = relationships.by_id(&relationship_id) else {
            return Ok(None);
        };
        if relationship.mode == mjx_opc::TargetMode::External {
            return Err(XlsxError::ExternalTarget {
                target: relationship.target.clone(),
            });
        }
        Ok(Some(crate::nav::resolve_target(
            &part,
            &relationship.target,
        )?))
    }
}
