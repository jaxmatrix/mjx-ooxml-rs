//! Which rules apply to one cell, in which order — and the `dxf` layer that sits *beside* the
//! cell's base format rather than inside it.
//!
//! # The seam this file exists for
//!
//! `conditionalFormatting` and `cols` are the only two children of `CT_Worksheet` declared
//! `maxOccurs="unbounded"`. A worksheet therefore holds a **list** of blocks, each with its own
//! `@sqref` — and `cfRule@priority` is not scoped to a block. ECMA-376 Part 1 §18.3.1.10:
//!
//! > The priority of this conditional formatting rule. This value is used to determine which format
//! > should be evaluated and rendered. **Lower numeric values are higher priority than higher
//! > numeric values, where 1 is the highest priority.**
//!
//! So the rules that apply to a cell cannot be assembled one block at a time. Three blocks may hold
//! priorities 1 and 4, 2, and 3 respectively; a cell covered by all three is governed by 1, 2, 3, 4
//! — an order in which consecutive rules come from *different* blocks. A per-block sort produces
//! 1, 4, 2, 3 and is wrong in a way no single-block fixture can see, which is why
//! `crates/mjx-sml/tests/conditional_formatting.rs` uses exactly that interleaving.
//!
//! # Priorities are as read
//!
//! §18.3.1.10 says lower wins. It does not say the numbers are dense, unique, or start at one, and
//! files Excel writes are none of those things — deleting a rule leaves a gap, and copying a range
//! duplicates a number. [`ConditionalRuleChain`] sorts on the numbers the file wrote and **never
//! renumbers them**, on write or on read. A stable sort keeps rules of equal priority in the order
//! they were read: block by block in document order, and rule by rule within a block.
//!
//! # `stopIfTrue` is reported as a stop, not applied as one
//!
//! §18.3.1.10: *"If this flag is 1, no rules with lower priority shall be applied over this rule,
//! when this rule evaluates to true."* The second clause is the whole difficulty — the stop is
//! **conditional on the rule firing**, and nothing here can know whether it fires. So the chain
//! reports every rule that applies, in priority order, and
//! [`ConditionalRuleChain::first_stopping_rule`] names the position at which a consumer that *can*
//! evaluate would stop if the rule there were true. Truncating the chain would be asserting the
//! condition held.
//!
//! # Reporting, never evaluating
//!
//! Restated because it is the boundary of this whole feature: this crate answers *which rules apply
//! and what each would impose*. It never answers *what the cell looks like*. There is no expression
//! parser and no calculation engine, and MJXOFF-115 settles that as scope.

use mjx_ooxml_core::{AttributeError, Interner};

use crate::address::CellReference;
use crate::error::SmlError;
use crate::styles::differential::DifferentialFormat;
use crate::styles::effective::{CellFormatResolver, EffectiveCellFormat};
use crate::worksheet::WorksheetPart;

use super::conditional_rules::ConditionalFormattingRule;

/// One rule that applies to a cell, and where in the sheet it was found.
///
/// Borrows the [`WorksheetPart`] the rule lives in; nothing is copied out of the markup but the two
/// numbers a caller sorts and reasons on.
#[derive(Debug, Clone, Copy)]
pub struct AppliedConditionalRule<'a> {
    block_index: usize,
    rule_index: usize,
    priority: i32,
    rule: &'a ConditionalFormattingRule,
}

impl<'a> AppliedConditionalRule<'a> {
    /// The rule itself — every attribute and child the file wrote.
    #[must_use]
    pub fn rule(&self) -> &'a ConditionalFormattingRule {
        self.rule
    }

    /// `@priority`, **exactly as the file wrote it**. Lower wins; nothing here renumbers.
    #[must_use]
    pub const fn priority(&self) -> i32 {
        self.priority
    }

    /// Which `x:conditionalFormatting` block this rule came from, counting from zero in document
    /// order.
    ///
    /// Reported because the block is what carries the `@sqref`: a caller asking *why* a rule applies
    /// to this cell needs the block, and a caller editing one needs its index for
    /// [`WorksheetPart::conditional_formatting_block_mut`].
    #[must_use]
    pub const fn block_index(&self) -> usize {
        self.block_index
    }

    /// Which `x:cfRule` of that block this is, counting from zero in document order.
    #[must_use]
    pub const fn rule_index(&self) -> usize {
        self.rule_index
    }

    /// `@stopIfTrue` — whether a consumer that found this rule true would consider no rule of lower
    /// priority.
    ///
    /// # Errors
    /// [`SmlError::Model`] if the attribute is present and is not an `xsd:boolean`.
    pub fn stops_lower_priority_rules(&self, interner: &Interner) -> Result<bool, SmlError> {
        self.rule
            .stops_lower_priority_rules(interner)
            .map_err(|error| SmlError::Model(error.into()))
    }

    /// `@dxfId` — the index into `dxfs` of the formatting this rule would impose, or `None` for a
    /// rule that imposes none (every `colorScale`, `dataBar` and `iconSet` rule).
    ///
    /// # Errors
    /// [`SmlError::Model`] if the attribute is present and is not an `ST_DxfId`.
    pub fn differential_format_index(&self, interner: &Interner) -> Result<Option<u32>, SmlError> {
        self.rule
            .differential_format_index(interner)
            .map_err(|error| SmlError::Model(error.into()))
    }
}

/// Every rule that applies to one cell, merged across blocks and ordered by `@priority`.
///
/// Built by [`WorksheetPart::conditional_rules_for`]. See this module's own documentation for the
/// ordering rule, for why the priorities are never renumbered, and for what `stopIfTrue` does and
/// does not do here.
#[derive(Debug, Clone)]
pub struct ConditionalRuleChain<'a> {
    rules: Vec<AppliedConditionalRule<'a>>,
    first_stopping_rule: Option<usize>,
}

impl<'a> ConditionalRuleChain<'a> {
    /// The rules, highest priority first — that is, ascending `@priority`.
    ///
    /// Rules that share a priority keep the order they were read in: block by block in document
    /// order, then rule by rule within the block.
    #[must_use]
    pub fn rules(&self) -> &[AppliedConditionalRule<'a>] {
        &self.rules
    }

    /// How many rules apply to the cell.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Whether no rule applies to the cell.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// The position in [`rules`](Self::rules) of the first rule carrying `stopIfTrue="1"`, or
    /// `None` when none does.
    ///
    /// **A position, not a truncation.** §18.3.1.10 makes the stop conditional on the rule
    /// evaluating to true, and nothing here evaluates anything, so every applicable rule is still
    /// reported. A consumer that *can* evaluate walks the chain from the front and stops at the
    /// first rule that is both true and stopping; this number says where the first candidate for
    /// that is.
    #[must_use]
    pub const fn first_stopping_rule(&self) -> Option<usize> {
        self.first_stopping_rule
    }
}

impl WorksheetPart {
    /// Every conditional-formatting rule that applies to `cell`, merged across every
    /// `x:conditionalFormatting` block whose `@sqref` covers it, in `@priority` order.
    ///
    /// **Which rules apply is the question this answers. Whether any of them is *true* is not** —
    /// see [`crate::features`]. A caller receives candidates in the order a consumer would consider
    /// them, and no claim about how the cell renders.
    ///
    /// The priorities come back exactly as the file wrote them, gaps and duplicates included.
    ///
    /// # Errors
    /// [`SmlError::Address`] if a block's `@sqref` is absent or does not parse, and
    /// [`SmlError::Model`] if a rule's `@priority` — which the schema declares `use="required"` — is
    /// absent or is not an `xsd:int`. Neither is an answer this can give *around*: a block whose
    /// range list will not parse might cover the cell, and a rule with no priority has no place in
    /// the order, so a silently shortened chain would report the wrong rule as winning.
    pub fn conditional_rules_for(
        &self,
        cell: CellReference,
    ) -> Result<ConditionalRuleChain<'_>, SmlError> {
        let interner = self.interner();
        let mut rules: Vec<AppliedConditionalRule<'_>> = Vec::new();
        for (block_index, block) in self.conditional_formatting_blocks().enumerate() {
            let ranges = block
                .ranges(interner)
                .map_err(|error| SmlError::Model(error.into()))?
                .ok_or(SmlError::ConditionalFormattingBlockHasNoRange { block: block_index })?;
            if !ranges.contains(cell) {
                continue;
            }
            for (rule_index, rule) in block.rules().enumerate() {
                let priority = rule.priority(interner).map_err(|error| match error {
                    // The schema declares `@priority` required, so its absence is a defect the file
                    // states rather than a value this can pick. Locating it is what a chain built
                    // over several blocks has to add: `AttributeError::Missing` names the
                    // attribute, and a caller needs to know *which rule of which block*.
                    AttributeError::Missing { .. } => {
                        SmlError::ConditionalFormattingRuleHasNoPriority {
                            block: block_index,
                            rule: rule_index,
                        }
                    }
                    other => SmlError::Model(other.into()),
                })?;
                rules.push(AppliedConditionalRule {
                    block_index,
                    rule_index,
                    priority,
                    rule,
                });
            }
        }
        // Stable, so that rules sharing a priority keep document order — which is the only order
        // the file states for them. Nothing renumbers.
        rules.sort_by_key(AppliedConditionalRule::priority);

        let mut first_stopping_rule = None;
        for (position, applied) in rules.iter().enumerate() {
            if applied.stops_lower_priority_rules(interner)? {
                first_stopping_rule = Some(position);
                break;
            }
        }
        Ok(ConditionalRuleChain {
            rules,
            first_stopping_rule,
        })
    }
}

/// One candidate of a cell's conditional layer: the rule, and the `dxf` it would impose.
#[derive(Debug, Clone, Copy)]
pub struct ConditionalFormatLayer<'a> {
    applied: AppliedConditionalRule<'a>,
    differential_format_index: Option<u32>,
    differential_format: Option<&'a DifferentialFormat>,
}

impl<'a> ConditionalFormatLayer<'a> {
    /// The rule this layer would come from, and where it was found.
    #[must_use]
    pub const fn applied_rule(&self) -> AppliedConditionalRule<'a> {
        self.applied
    }

    /// `@priority`, as the file wrote it.
    #[must_use]
    pub const fn priority(&self) -> i32 {
        self.applied.priority
    }

    /// `@dxfId`, or `None` for a rule that names no differential format.
    #[must_use]
    pub const fn differential_format_index(&self) -> Option<u32> {
        self.differential_format_index
    }

    /// The `x:dxf` `@dxfId` names, or `None` when the rule names none — and also when it names one
    /// the `dxfs` table does not hold, which is the file's defect and is reported rather than
    /// repaired.
    ///
    /// Every member of a `dxf` is `Option`, and absent means **inherited**: a `dxf` stating only a
    /// fill leaves the font, the border and the number format exactly as the base format has them.
    /// See [`DifferentialFormat`].
    #[must_use]
    pub const fn differential_format(&self) -> Option<&'a DifferentialFormat> {
        self.differential_format
    }
}

/// A cell's formatting in **two layers that are never merged**: what `styles.xml` says it is, and
/// what conditional formatting might put on top.
///
/// # Why they are not merged
///
/// §18.8.15 says a `dxf` is *"to be applied on top of or in addition to any formatting already
/// present"* — *when the rule's criteria are met*. Whether they are met needs a calculation engine,
/// and there is none here. A merged answer would therefore be a claim this library cannot support:
/// it would say the cell is red when all that is known is that a rule *would* make it red if its
/// condition held.
///
/// So [`base`](Self::base) is the resolved format and nothing else, [`layer`](Self::layer) is the
/// ordered candidates and nothing else, and there is deliberately no method that combines them. A
/// consumer that can evaluate conditions does the combining, with the information it has and this
/// library does not.
#[derive(Debug, Clone)]
pub struct ConditionalCellFormat<'a> {
    base: EffectiveCellFormat,
    layer: Vec<ConditionalFormatLayer<'a>>,
    first_stopping_rule: Option<usize>,
}

impl<'a> ConditionalCellFormat<'a> {
    /// What `styles.xml` says the cell's format is, with no conditional rule taken into account.
    ///
    /// Exactly what [`CellFormatResolver::effective_cell_format`] answers for the same cell — this
    /// type adds a layer beside it and changes nothing about it.
    #[must_use]
    pub const fn base(&self) -> EffectiveCellFormat {
        self.base
    }

    /// The candidate conditional formats, highest priority first.
    ///
    /// Empty for a cell no block covers. Every entry is a rule that *applies*; none is a rule that
    /// is known to *fire*.
    #[must_use]
    pub fn layer(&self) -> &[ConditionalFormatLayer<'a>] {
        &self.layer
    }

    /// The position in [`layer`](Self::layer) of the first rule carrying `stopIfTrue="1"`.
    ///
    /// As [`ConditionalRuleChain::first_stopping_rule`]: a position a consumer that can evaluate
    /// would stop at, not a truncation this library performed.
    #[must_use]
    pub const fn first_stopping_rule(&self) -> Option<usize> {
        self.first_stopping_rule
    }
}

impl<'a> CellFormatResolver<'a> {
    /// The cell at `reference`, resolved in both layers: its base format, and the conditional
    /// candidates over it.
    ///
    /// `column_style` is what [`column_style_index`](crate::styles::column_style_index) answered for
    /// the cell's column, exactly as [`effective_cell_format`](Self::effective_cell_format) takes
    /// it; `worksheet` supplies the cell, its row and its conditional-formatting blocks.
    ///
    /// **The two layers are returned side by side and never combined.** See
    /// [`ConditionalCellFormat`].
    ///
    /// # Errors
    /// [`SmlError::CellFormatIndexOutOfRange`] if the style index in force names no `cellXfs`
    /// record, and otherwise as [`WorksheetPart::conditional_rules_for`].
    pub fn conditional_cell_format(
        &self,
        worksheet: &'a WorksheetPart,
        reference: CellReference,
        column_style: Option<u32>,
    ) -> Result<ConditionalCellFormat<'a>, SmlError> {
        let sheet_data = worksheet.sheet_data();
        let cell = sheet_data.and_then(|cells| cells.cell(reference));
        let row = sheet_data.and_then(|cells| cells.row(reference.row().saturating_add(1)));
        let base = self.effective_cell_format(cell.as_ref(), row.as_ref(), column_style)?;

        let chain = worksheet.conditional_rules_for(reference)?;
        let interner = worksheet.interner();
        let mut layer = Vec::with_capacity(chain.len());
        for applied in chain.rules() {
            let index = applied.differential_format_index(interner)?;
            layer.push(ConditionalFormatLayer {
                applied: *applied,
                differential_format_index: index,
                differential_format: index.and_then(|index| self.differential_format(index)),
            });
        }
        Ok(ConditionalCellFormat {
            base,
            layer,
            first_stopping_rule: chain.first_stopping_rule(),
        })
    }
}
