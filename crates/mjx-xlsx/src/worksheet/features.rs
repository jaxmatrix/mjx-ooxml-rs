//! The optional things a worksheet carries beside its cells — conditional formatting, data
//! validation, autofilters and sort state today; tables, comments, hyperlinks and form controls as
//! the later Phase D children land.
//!
//! **MJXOFF-120 (D13) — done**: conditional formatting. **MJXOFF-123 (D14) — done**: data
//! validation, autofilters and sort state. MJXOFF-125, MJXOFF-127 and MJXOFF-129 (D15–D17) fill the
//! rest.
//!
//! # What this file adds, and what it deliberately does not
//!
//! The cross-block priority order, the rule kinds and the `dxf` layer all live in
//! [`mjx_sml::features`], and none of them is repeated here. What this tier adds is the thing the
//! markup tier cannot reach on its own: **conditional formatting spans two parts**. A rule is in
//! `xl/worksheets/sheetN.xml` and the formatting it imposes is in `xl/styles.xml`, so answering
//! *"what would this cell look like if that rule fired"* needs both — and authoring a rule with a
//! highlight means writing both, in one call, without either half being left behind.
//!
//! # Reporting, never evaluating — at this tier too
//!
//! [`Workbook::conditional_cell_format`] hands back a base format and a list of candidates, and
//! there is no call here that merges them, for the reason
//! [`mjx_sml::ConditionalCellFormat`] gives: whether a rule's condition holds needs a calculation
//! engine, and this workspace has none. See the guide page *Conditional formatting*.
//!
//! # Appending a `dxf` never renumbers one
//!
//! [`Workbook::append_differential_format`] appends and answers the index it appended at. Every
//! `@dxfId` already in the workbook — in this worksheet's rules, in every other worksheet's, in
//! every table style — still names what it named. That is
//! [`mjx_sml::DifferentialFormats`]'s stated contract, and
//! `crates/mjx-xlsx/tests/conditional_formatting.rs` asserts it against the file rather than against
//! a second run of this crate's writer.

use mjx_sml::{
    AutoFilter, AutoFilterSpec, CellRangeList, CellReference, ConditionalCellFormat,
    ConditionalFormatting, ConditionalRuleChain, ConditionalRuleSpec, DataValidation,
    DataValidationSpec, DifferentialFormatSpec, WorksheetPart,
};

use crate::error::XlsxError;
use crate::workbook::Workbook;
use crate::worksheet::formatting::SheetFormatting;

impl SheetFormatting {
    /// The cell at `reference`, resolved in **both layers**: what `xl/styles.xml` says its format
    /// is, and the conditional-formatting candidates over it.
    ///
    /// Both parts are already parsed here, so this is the call to make when there are many cells to
    /// ask about; [`Workbook::conditional_cell_format`] is the one-off.
    ///
    /// # Errors
    /// [`XlsxError::Sml`] if the style index in force names no record in `cellXfs`, if a block's
    /// `@sqref` will not parse, or if a rule states no `@priority`.
    pub fn conditional_cell_format(
        &self,
        reference: CellReference,
    ) -> Result<ConditionalCellFormat<'_>, XlsxError> {
        let resolver = self.resolver()?;
        let column = u32::from(reference.column()).saturating_add(1);
        let column_style = resolver.columns().style_index(column);
        Ok(resolver
            .formats()
            .conditional_cell_format(self.worksheet(), reference, column_style)?)
    }
}

impl Workbook {
    /// Every conditional-formatting rule that applies to `reference` on the tab at `index`, merged
    /// across every block and in `@priority` order, handed to `read`.
    ///
    /// A visitor rather than a returned value because the chain **borrows the worksheet part**, and
    /// this call is what parses it: the same shape [`Workbook::calculation_chain`] takes, for the
    /// same reason. `Ok(None)` when the tab reaches no worksheet part.
    ///
    /// Reading does not dirty the package. The part keeps its container bytes and
    /// [`save`](Workbook::save) still re-emits them verbatim.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab; [`XlsxError::Sml`] if a block writes no
    /// `@sqref` or a rule writes no `@priority` — neither of which a chain can be answered around.
    pub fn conditional_rules_for<R>(
        &self,
        index: usize,
        reference: CellReference,
        read: impl FnOnce(&ConditionalRuleChain<'_>) -> R,
    ) -> Result<Option<R>, XlsxError> {
        let Some(markup) = self.worksheet_markup(index)? else {
            return Ok(None);
        };
        let chain = markup.conditional_rules_for(reference)?;
        Ok(Some(read(&chain)))
    }

    /// The cell at `reference` on the tab at `index`, resolved in both layers, handed to `read`.
    ///
    /// **Reads and decodes both parts on every call.** For more than a handful of cells, hold a
    /// [`SheetFormatting`] from [`sheet_formatting`](Self::sheet_formatting) and call
    /// [`SheetFormatting::conditional_cell_format`] on it instead.
    ///
    /// `Ok(None)` when [`sheet_formatting`](Self::sheet_formatting) answers `None` — the tab reaches
    /// no worksheet part, or the workbook relates to no styles part, so there is nothing to resolve
    /// *against*.
    ///
    /// # Errors
    /// As [`SheetFormatting::conditional_cell_format`].
    pub fn conditional_cell_format<R>(
        &self,
        index: usize,
        reference: CellReference,
        read: impl FnOnce(&ConditionalCellFormat<'_>) -> R,
    ) -> Result<Option<R>, XlsxError> {
        let Some(formatting) = self.sheet_formatting(index)? else {
            return Ok(None);
        };
        let resolved = formatting.conditional_cell_format(reference)?;
        Ok(Some(read(&resolved)))
    }

    /// Appends a differential format to `xl/styles.xml`'s `dxfs` and answers the `@dxfId` a rule can
    /// now name it by.
    ///
    /// **Appending is the only mutation the table has.** A `dxf` is addressed by position, so
    /// inserting, removing or reordering one would silently repoint every `@dxfId` above it. The
    /// index answered is the last one, and every index handed out before it still names what it
    /// named.
    ///
    /// The table is created at its rank in `CT_Stylesheet`'s sequence if the part has none.
    ///
    /// # Errors
    /// [`XlsxError::MissingWorkbookPart`] if the workbook relates to no styles part, or
    /// [`XlsxError`] if that part is unreadable.
    pub fn append_differential_format(
        &mut self,
        spec: &DifferentialFormatSpec,
    ) -> Result<u32, XlsxError> {
        self.edit_styles(|part, interner| {
            let format = spec.build(interner, None)?;
            Ok(part.append_differential_format(interner, None, format))
        })
    }

    /// Adds one `x:conditionalFormatting` block over `ranges` to the tab at `index`, holding
    /// `rules`, at rank 16 of `CT_Worksheet`'s sequence and after any block already there.
    ///
    /// The priorities are the caller's: each [`ConditionalRuleSpec`] states its own, and nothing
    /// here derives, renumbers or deduplicates one. §18.3.1.10 says lower wins, and which rule wins
    /// is not a decision this library makes on a caller's behalf.
    ///
    /// A rule that imposes a highlight names a `@dxfId`; allocate one first with
    /// [`append_differential_format`](Self::append_differential_format), which appends.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, [`XlsxError::MissingWorkbookPart`] if it
    /// reaches no part, and [`XlsxError::SheetIsNotAWorksheet`] if the part it reaches is not a
    /// worksheet.
    pub fn add_conditional_formatting(
        &mut self,
        index: usize,
        ranges: &CellRangeList,
        rules: &[ConditionalRuleSpec],
    ) -> Result<(), XlsxError> {
        self.edit_worksheet(index, |markup| {
            let prefix = markup.element_prefix().map(str::to_owned);
            let block = {
                let interner = markup.interner_mut();
                let mut block = ConditionalFormatting::new(interner, prefix.as_deref());
                block.set_ranges(interner, Some(ranges.clone()));
                for spec in rules {
                    let rule = spec.build(interner, prefix.as_deref());
                    block.push_rule(rule);
                }
                block
            };
            markup.push_conditional_formatting(block);
            Ok(())
        })
    }
}

// -------------------------------------------------------------------------------------------
// MJXOFF-123 (D14) — data validation, autofilters and sort state
// -------------------------------------------------------------------------------------------

/// **Neither of the two features below is ever applied**, and the package tier is where that has to
/// be said again, because it is the tier a caller reaches for when they want something *done*.
///
/// * [`Workbook::set_auto_filter`] writes an `x:autoFilter`. It reads no cell, evaluates no filter,
///   and sets no row's `@hidden` — a hidden row is MJXOFF-117's row property and survives untouched.
/// * A `sortState` inside that autofilter records a sort. Nothing reorders a row.
/// * [`Workbook::add_data_validation`] writes an `x:dataValidation`. Nothing compares a cell against
///   it, and a `list` rule's source formula — a range reference as often as a quoted literal list —
///   is the caller's text, written through unchanged.
///
/// The reasoning is in [`mjx_sml::features::filters`] and [`mjx_sml::features::validation`], and
/// `crates/mjx-xlsx/tests/validation_and_filters.rs` asserts it against a file rather than against a
/// second run of this crate's writer.
impl Workbook {
    /// `x:autoFilter` on the tab at `index`, handed to `read` — the filtered range, its per-column
    /// filters and the sort state over it.
    ///
    /// A visitor rather than a returned value because the element borrows the worksheet part, and
    /// this call is what parses it: the same shape [`Workbook::conditional_rules_for`] takes.
    /// `Ok(None)` when the tab reaches no worksheet part, and `read` receives `None` for its second
    /// argument when that worksheet writes no `x:autoFilter`.
    ///
    /// **The part comes first because an attribute's value needs its interner.** Every accessor on
    /// [`AutoFilter`] and the six filter kinds takes one — an attribute name is a symbol — and the
    /// only interner those symbols are meaningful in is the one this part was parsed with, which
    /// [`WorksheetPart::interner`] answers.
    ///
    /// Reading does not dirty the package.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, or [`XlsxError::Sml`] if the part will
    /// not read.
    pub fn auto_filter<R>(
        &self,
        index: usize,
        read: impl FnOnce(&WorksheetPart, Option<&AutoFilter>) -> R,
    ) -> Result<Option<R>, XlsxError> {
        let Some(markup) = self.worksheet_markup(index)? else {
            return Ok(None);
        };
        let filter = markup.auto_filter();
        Ok(Some(read(&markup, filter)))
    }

    /// Every `x:dataValidation` on the tab at `index`, in document order, handed to `read`.
    ///
    /// A visitor, and the part comes first, both for the reason [`auto_filter`](Self::auto_filter)
    /// gives. `Ok(None)` when the tab reaches no worksheet part; an empty slice when it writes no
    /// `x:dataValidations`.
    ///
    /// # Errors
    /// As [`auto_filter`](Self::auto_filter).
    pub fn data_validations<R>(
        &self,
        index: usize,
        read: impl FnOnce(&WorksheetPart, &[&DataValidation]) -> R,
    ) -> Result<Option<R>, XlsxError> {
        let Some(markup) = self.worksheet_markup(index)? else {
            return Ok(None);
        };
        let rules: Vec<&DataValidation> = markup.data_validation_rules().collect();
        Ok(Some(read(&markup, &rules)))
    }

    /// Sets the tab's `x:autoFilter` at rank 10 of `CT_Worksheet`'s sequence, replacing whichever
    /// one is there.
    ///
    /// A worksheet carries **at most one** — the slot is `minOccurs="0" maxOccurs="1"` — so this is
    /// a setter rather than an append, unlike
    /// [`add_conditional_formatting`](Self::add_conditional_formatting) whose slot is unbounded.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `index` names no tab, [`XlsxError::MissingWorkbookPart`] if it
    /// reaches no part, and [`XlsxError::SheetIsNotAWorksheet`] if the part it reaches is not a
    /// worksheet.
    pub fn set_auto_filter(
        &mut self,
        index: usize,
        spec: &AutoFilterSpec,
    ) -> Result<(), XlsxError> {
        self.edit_worksheet(index, |markup| {
            let prefix = markup.element_prefix().map(str::to_owned);
            let filter = spec.build(markup.interner_mut(), prefix.as_deref());
            markup.set_auto_filter(Some(filter));
            Ok(())
        })
    }

    /// Removes the tab's `x:autoFilter`, reporting whether there was one.
    ///
    /// **Removing a filter unhides nothing.** Whatever rows the file records as hidden stay hidden,
    /// because their `@hidden` is the file's statement and this library did not put it there.
    ///
    /// # Errors
    /// As [`set_auto_filter`](Self::set_auto_filter).
    pub fn remove_auto_filter(&mut self, index: usize) -> Result<bool, XlsxError> {
        let mut had_one = false;
        self.edit_worksheet(index, |markup| {
            had_one = markup.auto_filter().is_some();
            markup.set_auto_filter(None);
            Ok(())
        })?;
        Ok(had_one)
    }

    /// Appends one `x:dataValidation` to the tab at `index`, creating `x:dataValidations` at rank 17
    /// of `CT_Worksheet`'s sequence if the sheet has none.
    ///
    /// `@count` on the enclosing element is left exactly as the file wrote it: it is a producer's
    /// cache, and rewriting one nobody asked about is the correction this phase keeps refusing.
    ///
    /// # Errors
    /// As [`set_auto_filter`](Self::set_auto_filter).
    pub fn add_data_validation(
        &mut self,
        index: usize,
        spec: &DataValidationSpec,
    ) -> Result<(), XlsxError> {
        self.edit_worksheet(index, |markup| {
            let prefix = markup.element_prefix().map(str::to_owned);
            let rule = spec.build(markup.interner_mut(), prefix.as_deref());
            markup.add_data_validation(rule);
            Ok(())
        })
    }

    /// Removes the `rule`-th `x:dataValidation` from the tab at `index`, reporting whether there was
    /// one.
    ///
    /// When the last rule goes the whole `x:dataValidations` goes with it, because the schema
    /// declares `dataValidation` `minOccurs="1"`.
    ///
    /// # Errors
    /// As [`set_auto_filter`](Self::set_auto_filter).
    pub fn remove_data_validation(&mut self, index: usize, rule: usize) -> Result<bool, XlsxError> {
        let mut removed = false;
        self.edit_worksheet(index, |markup| {
            removed = markup.remove_data_validation(rule);
            Ok(())
        })?;
        Ok(removed)
    }
}
