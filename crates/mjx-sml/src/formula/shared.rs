//! Shared formulas: where the text lives, and why it must stay there.
//!
//! # The distribution is the data
//!
//! §18.3.1.40 (`si`): *"The first formula in a group of shared formulas is saved in the `f` element.
//! This is considered the 'master' formula cell. Subsequent cells sharing this formula need not have
//! the formula written in their `f` element."* In practice they never do — Excel writes
//! `<f t="shared" ref="B2:B6" si="0">A2*2</f>` on the host and `<f t="shared" si="0"/>` on every
//! other cell in the range.
//!
//! So a five-cell group is one formula and four empty elements, and **writing the host's text into
//! the members on the way out is a corruption, not an optimisation**. It changes bytes in a part
//! nobody asked to edit; it inflates a sheet whose whole reason for sharing was size; and because a
//! shared formula's text is written relative to its *host*, copying it verbatim into a member states
//! a different formula from the one that cell has. `crates/mjx-sml/tests/formulas.rs` fails if the
//! distribution ever changes, including after an edit to one member's style.
//!
//! # What this index does and does not answer
//!
//! [`SharedFormulaGroups`] is a *report*, built on demand by a caller that wants one and held
//! nowhere. It says which `si` groups a sheet has, where each one's host is, and how many cells
//! carry each index. It deliberately does **not** answer "what is this member's formula", because
//! that answer is the host's text with every relative reference shifted by the offset between the two
//! cells — reference translation, which this workspace does not do and is not scheduled to. A caller
//! that needs it has [`host`](SharedFormulaGroup::host) and the member's own address, which is
//! everything the shift needs and nothing this library has to guess at.
//!
//! # It costs nothing until it is asked for
//!
//! The store keeps no group table. Building this walks the sheet's cells once and allocates one
//! entry per *group*, not per cell — a sheet of a million members in one group builds a one-element
//! vector. That is what makes a group member cost exactly what any formula cell costs and nothing
//! for its membership.

use mjx_ooxml_core::AttributeError;

use crate::address::{CellRange, CellReference};
use crate::cells::SheetData;

use super::{CellFormula, FormulaKind};

/// Every `@si` group on one sheet, in ascending index order.
///
/// Built by [`SheetData::shared_formula_groups`]; see the [module docs](self) for what it reports and
/// what it deliberately does not.
#[derive(Debug, Clone, PartialEq)]
pub struct SharedFormulaGroups<'a> {
    groups: Vec<SharedFormulaGroup<'a>>,
}

/// One `@si` group: its host, and how many cells carry the index.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SharedFormulaGroup<'a> {
    index: u32,
    host: Option<CellReference>,
    host_formula: Option<CellFormula<'a>>,
    cell_count: u32,
    host_count: u32,
}

impl<'a> SharedFormulaGroups<'a> {
    /// Indexes `sheet`'s shared formulas.
    ///
    /// # Errors
    /// [`AttributeError`] if any `<f>` on the sheet carries a `t` that is not one of
    /// `ST_CellFormulaType`'s four tokens, an `si` that is not an `xsd:unsignedInt`, or a `ref` that
    /// is not an `ST_Ref`. The index is a report about a well-formed sheet; a sheet that says
    /// something the schema does not allow is reported rather than silently summarised, and it still
    /// round-trips byte for byte because nothing here is on the write path.
    pub(crate) fn of(sheet: &'a SheetData) -> Result<Self, AttributeError> {
        let mut groups: Vec<SharedFormulaGroup<'a>> = Vec::new();
        for cell in sheet.cells() {
            let Some(formula) = cell.formula() else {
                continue;
            };
            if formula.kind()? != FormulaKind::Shared {
                continue;
            }
            let Some(index) = formula.shared_group_index()? else {
                continue;
            };
            let is_host = formula.raw_attribute("ref").is_some();
            // Parsed for its side effect: a `@ref` that is not an `ST_Ref` is an error here rather
            // than a range silently dropped from the report.
            let _ = formula.range()?;
            let position = match groups.binary_search_by_key(&index, |group| group.index) {
                Ok(position) => position,
                Err(position) => {
                    groups.insert(
                        position,
                        SharedFormulaGroup {
                            index,
                            host: None,
                            host_formula: None,
                            cell_count: 0,
                            host_count: 0,
                        },
                    );
                    position
                }
            };
            let group = &mut groups[position];
            group.cell_count = group.cell_count.saturating_add(1);
            if is_host {
                group.host_count = group.host_count.saturating_add(1);
                // The **first** host wins, so a file that broke §18.3.1.40's *"Master cell references
                // on the same sheet shall not overlap with each other"* is reported through
                // `host_count` rather than answered with whichever host happened to come last.
                if group.host.is_none() {
                    group.host = Some(cell.reference());
                    group.host_formula = Some(formula);
                }
            }
        }
        Ok(Self { groups })
    }

    /// Every group, in ascending `@si` order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &SharedFormulaGroup<'a>> + '_ {
        self.groups.iter()
    }

    /// How many groups the sheet has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.groups.len()
    }

    /// Whether the sheet has no shared formulas at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// The group with this `@si`, or `None` if the sheet has none.
    #[must_use]
    pub fn get(&self, index: u32) -> Option<&SharedFormulaGroup<'a>> {
        self.groups
            .binary_search_by_key(&index, |group| group.index)
            .ok()
            .and_then(|position| self.groups.get(position))
    }
}

impl<'a> SharedFormulaGroup<'a> {
    /// The `@si` this group is identified by.
    #[must_use]
    pub fn index(self) -> u32 {
        self.index
    }

    /// The address of the group's host — §18.3.1.40's *master* cell, the one whose `<f>` carries the
    /// text and the `@ref`.
    ///
    /// `None` for a group whose members reference an `si` no cell on this sheet hosts. That is a file
    /// the standard calls implementation-defined rather than invalid, and it is reported rather than
    /// repaired.
    #[must_use]
    pub fn host(self) -> Option<CellReference> {
        self.host
    }

    /// The host's formula, whose [`raw_text`](CellFormula::raw_text) is the group's expression.
    ///
    /// It is the **host's** text: relative references in it are written from the host's position, and
    /// a member's own formula is that text shifted by the offset between the two cells. This
    /// workspace does not perform that shift — see the [module docs](self).
    #[must_use]
    pub fn host_formula(self) -> Option<CellFormula<'a>> {
        self.host_formula
    }

    /// The range the host's `@ref` names, when it has one that parses.
    #[must_use]
    pub fn range(self) -> Option<CellRange> {
        self.host_formula
            .and_then(|formula| formula.range().ok())
            .flatten()
    }

    /// How many cells on the sheet carry this `@si`, host included.
    #[must_use]
    pub fn cell_count(self) -> u32 {
        self.cell_count
    }

    /// How many cells carrying this `@si` also carry a `@ref` — one for a well-formed group.
    ///
    /// More than one means the file has overlapping master cells, which §18.3.1.40 forbids. It is
    /// reported here and changed nowhere.
    #[must_use]
    pub fn host_count(self) -> u32 {
        self.host_count
    }
}
