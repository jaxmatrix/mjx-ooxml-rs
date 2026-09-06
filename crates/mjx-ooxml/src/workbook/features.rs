//! What a sheet's conditional formatting, autofilter and data validation **say** — where each one
//! applies, and nothing about whether it fires.
//!
//! # Reporting, never enforcing
//!
//! There is no calculation engine and no rule evaluator in this project, and there is not going to
//! be one — see [*Deliberate limitations*](mjx_xlsx::guide::deliberate_limitations). So the honest
//! surface for all three features is *where they claim cells*, which is what a caller wants in order
//! to leave those regions alone, to report them, or to reproduce them somewhere else. Whether a
//! particular cell's value satisfies a rule is a question only a consumer that computes can answer.
//!
//! # Why authoring is not here
//!
//! Each of the three is written through an `mjx-sml` spec *tree* rather than a flat struct, and [the
//! module documentation above](super) says why those are left to [`Workbook::workbook_mut`]. What is
//! here is the half a binding can carry, and the half that keeps a caller from destroying a feature
//! it cannot see.

use crate::error::Error;

use super::Workbook;

impl Workbook {
    /// The range one sheet's autofilter covers (`autoFilter@ref`), or `None` when the sheet has no
    /// autofilter, or has one that states no range.
    ///
    /// The filter's own columns and criteria are not projected: `AutoFilterSpec` is a spec tree, and
    /// a filter's *effect* is something only a consumer that applies it can report. This says the
    /// region is claimed.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `sheet` names no tab,
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if it holds no worksheet, or
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if `@ref` does not parse.
    pub fn auto_filter_range(&self, sheet: u32) -> Result<Option<String>, Error> {
        let markup = self.worksheet_or_refuse(sheet)?;
        let Some(filter) = markup.auto_filter() else {
            return Ok(None);
        };
        let range = filter
            .range(markup.interner())
            .map_err(mjx_ooxml_core::FromXmlError::from)
            .map_err(mjx_sml::SmlError::from)?;
        Ok(range.map(|range| range.text().as_str().to_owned()))
    }

    /// Removes one sheet's autofilter, answering whether there was one.
    ///
    /// The `x:autoFilter` element goes; the rows it hid stay hidden, because `row@hidden` is what a
    /// consumer actually wrote and removing a filter does not unhide them — see
    /// [`set_row_hidden`](Self::set_row_hidden) for the other half.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `sheet` names no tab, or
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if it holds no worksheet.
    pub fn remove_auto_filter(&mut self, sheet: u32) -> Result<bool, Error> {
        Ok(self
            .workbook
            .remove_auto_filter(crate::index::index(sheet))?)
    }

    /// The ranges every data-validation rule on one sheet claims, one entry per rule, in document
    /// order.
    ///
    /// A rule's `@sqref` is a *list* of ranges, so each entry is that list's text as the file wrote
    /// it: `"A1:B2 D4 F6:F9"`.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `sheet` names no tab,
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if it holds no worksheet, or
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if a rule writes no
    /// `@sqref` — which `CT_DataValidation` declares required.
    pub fn data_validation_ranges(&self, sheet: u32) -> Result<Vec<String>, Error> {
        let markup = self.worksheet_or_refuse(sheet)?;
        let interner = markup.interner();
        let mut ranges = Vec::new();
        for rule in markup.data_validation_rules() {
            let list = rule
                .ranges(interner)
                .map_err(mjx_ooxml_core::FromXmlError::from)
                .map_err(mjx_sml::SmlError::from)?;
            ranges.push(range_list_text(&list));
        }
        Ok(ranges)
    }

    /// Removes the `rule`-th data-validation rule of one sheet, answering whether there was one.
    ///
    /// When the last rule goes, `x:dataValidations` goes with it: the schema declares
    /// `dataValidation` `minOccurs="1"`, so an empty element is markup no validator accepts.
    ///
    /// # Errors
    /// As [`remove_auto_filter`](Self::remove_auto_filter).
    pub fn remove_data_validation(&mut self, sheet: u32, rule: u32) -> Result<bool, Error> {
        Ok(self
            .workbook
            .remove_data_validation(crate::index::index(sheet), crate::index::index(rule))?)
    }

    /// The ranges every conditional-formatting block on one sheet claims, one entry per block, in
    /// document order.
    ///
    /// A block's `@sqref` is a *list*, so each entry is that list's text. A block that writes no
    /// `@sqref` at all is skipped rather than reported as covering nothing: it is markup that exists
    /// and says nothing about where it applies, and it is still written back verbatim.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `sheet` names no tab,
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if it holds no worksheet, or
    /// [`ErrorCode::InvalidArgument`](crate::ErrorCode::InvalidArgument) if a `@sqref` does not
    /// parse.
    pub fn conditional_formatting_ranges(&self, sheet: u32) -> Result<Vec<String>, Error> {
        let markup = self.worksheet_or_refuse(sheet)?;
        let interner = markup.interner();
        let mut ranges = Vec::new();
        for block in markup.conditional_formatting_blocks() {
            let list = block
                .ranges(interner)
                .map_err(mjx_ooxml_core::FromXmlError::from)
                .map_err(mjx_sml::SmlError::from)?;
            if let Some(list) = list {
                ranges.push(range_list_text(&list));
            }
        }
        Ok(ranges)
    }

    /// How many conditional-formatting rules the block at `block` holds, or `None` when the sheet
    /// has no such block.
    ///
    /// The companion to [`conditional_formatting_ranges`](Self::conditional_formatting_ranges): the
    /// ranges say *where*, this says how many rules compete there. Their priorities and their
    /// effects are `mjx-sml`'s [`ConditionalRuleChain`](mjx_sml::ConditionalRuleChain), reachable
    /// through [`workbook_mut`](Self::workbook_mut).
    ///
    /// # Errors
    /// As [`conditional_formatting_ranges`](Self::conditional_formatting_ranges).
    pub fn conditional_formatting_rule_count(
        &self,
        sheet: u32,
        block: u32,
    ) -> Result<Option<u32>, Error> {
        let markup = self.worksheet_or_refuse(sheet)?;
        let rules = markup
            .conditional_formatting_blocks()
            .nth(crate::index::index(block))
            .map(|block| block.rules().count());
        Ok(rules.map(crate::index::count))
    }
}

/// A range list as the text a caller reads and writes — the file's own spelling where it has one.
fn range_list_text(list: &mjx_sml::CellRangeList) -> String {
    list.ranges()
        .iter()
        .map(|range| range.text().as_str().to_owned())
        .collect::<Vec<_>>()
        .join(" ")
}
