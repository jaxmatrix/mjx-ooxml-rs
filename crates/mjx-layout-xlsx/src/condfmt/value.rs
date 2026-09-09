//! [`RuleValue`] — what a predicate compares, and the ordering it compares in.
//!
//! # Why not [`CellValue`](crate::numfmt::CellValue)
//!
//! [`CellValue`](crate::numfmt::CellValue) borrows the text it was read from, which is exactly right
//! for a formatter that answers a string and exactly wrong for a statistic: a colour scale's minimum
//! has to outlive the cell it came from. So this is the owned twin, built from the same
//! `(ST_CellType, text)` pair by the same reading, plus the one state a formatter never sees —
//! [`RuleValue::Blank`], which conditional formatting treats differently from every other value.
//!
//! # Excel's sort order, and why a predicate needs one
//!
//! `cellIs` states `greaterThan` and one operand, and the operand may be a number where the cell
//! holds text. Excel's answer is its whole-workbook sort order — **numbers < text < FALSE < TRUE** —
//! and it is the same order `SORT`, `MATCH` and `>` all use. [`RuleValue::compare`] implements it.
//!
//! Two states are outside the order and answer `None` rather than taking a position:
//!
//! * an **error** never satisfies a comparison. `#N/A > 0` is `#N/A`, not `TRUE`, and a conditional
//!   format whose condition errors does not apply.
//! * a **blank** is skipped. GUESS: Excel's own generated `cellIs` formula compares the cell
//!   directly, which would coerce a blank to `0` and paint every empty cell in the range under
//!   `cellIs lessThan 5`; the behaviour everybody reports is that it does not, so blanks are
//!   excluded here.

use std::cmp::Ordering;

use mjx_ooxml_types::spreadsheetml::CellType;

/// One cell's value, owned, in the four states a rule can compare plus the absence of all four.
#[derive(Clone, PartialEq, Debug)]
pub enum RuleValue {
    /// The cell holds nothing a reader would see.
    Blank,
    /// A number — every date, time, currency and percentage included.
    Number(f64),
    /// Text.
    Text(String),
    /// `TRUE` or `FALSE`.
    Boolean(bool),
    /// An error code, as the file spells it.
    Error(String),
}

impl RuleValue {
    /// What a cell of `cell_type` whose stored text is `text` holds.
    ///
    /// Follows [`CellValue::read`](crate::numfmt::CellValue::read) exactly, and then folds an empty
    /// string into [`RuleValue::Blank`]: a `<c>` carrying a style and no `<v>` is how Excel records
    /// a formatted-but-empty cell, and a rule must not fire on one.
    #[must_use]
    pub fn read(cell_type: CellType, text: &str) -> Self {
        if text.is_empty() {
            return Self::Blank;
        }
        match cell_type {
            CellType::Number => match text.trim().parse::<f64>() {
                Ok(value) if value.is_finite() => Self::Number(value),
                _ => Self::Text(text.to_owned()),
            },
            CellType::Boolean => Self::Boolean(text.trim() != "0"),
            CellType::Error => Self::Error(text.to_owned()),
            CellType::SharedString | CellType::FormulaString | CellType::InlineString => {
                Self::Text(text.to_owned())
            }
        }
    }

    /// The number this value contributes to a statistic, or `None` when it contributes none.
    ///
    /// Text, booleans, errors and blanks all contribute nothing: a colour scale over a column of
    /// labels has no minimum, and `AVERAGE` does not count a `TRUE`.
    #[must_use]
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(number) => Some(*number),
            _ => None,
        }
    }

    /// The characters a text operator searches, or `None` for a value that has none.
    ///
    /// GUESS: that a **number** is searched as no characters at all rather than as the ones it
    /// would display. Excel's own generated formula is `SEARCH("x",A1)`, which coerces the value
    /// under `General` — so `containsText "5"` finds a `15` in Excel and not here. Reading the
    /// displayed form instead would put a number-format code into the predicate, and therefore into
    /// the cache key of every rule on the sheet; the narrower answer is the one this crate can give
    /// without that, and it is marked rather than smoothed over.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Whether this value is an error code.
    #[must_use]
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Whether this value is blank.
    #[must_use]
    pub fn is_blank(&self) -> bool {
        matches!(self, Self::Blank)
    }

    /// Where this value sits in Excel's sort order relative to `other`, or `None` when either is
    /// outside that order.
    ///
    /// Numbers before text before `FALSE` before `TRUE`; text compared **case-insensitively**,
    /// which is what `="a"="A"` answers in Excel.
    #[must_use]
    pub fn compare(&self, other: &Self) -> Option<Ordering> {
        let rank = |value: &Self| match value {
            Self::Number(_) => Some(0_u8),
            Self::Text(_) => Some(1),
            Self::Boolean(_) => Some(2),
            Self::Blank | Self::Error(_) => None,
        };
        let (left, right) = (rank(self)?, rank(other)?);
        if left != right {
            return Some(left.cmp(&right));
        }
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left.partial_cmp(right),
            (Self::Text(left), Self::Text(right)) => Some(fold_case(left).cmp(&fold_case(right))),
            (Self::Boolean(left), Self::Boolean(right)) => Some(left.cmp(right)),
            _ => None,
        }
    }

    /// The key this value counts under for `duplicateValues` and `uniqueValues`, or `None` when it
    /// takes part in neither.
    ///
    /// GUESS: that text is folded to lower case, so `Apple` and `apple` are duplicates of one
    /// another. ECMA-376 says nothing about it; Excel's *Highlight Duplicates* is case-insensitive
    /// in every account anybody has written down, and the same fold is what
    /// [`compare`](Self::compare) already uses for `equal`.
    ///
    /// Blanks and errors count under nothing: a range of forty empty cells is not forty duplicates.
    #[must_use]
    pub fn duplicate_key(&self) -> Option<String> {
        match self {
            Self::Number(number) => Some(format!("n{number}")),
            Self::Text(text) => Some(format!("t{}", fold_case(text))),
            Self::Boolean(value) => Some(format!("b{}", u8::from(*value))),
            Self::Blank | Self::Error(_) => None,
        }
    }
}

/// The case fold both `equal` and `duplicateValues` use.
///
/// `to_lowercase` rather than `to_ascii_lowercase`: a workbook of Turkish or Greek labels is not a
/// special case, and the cost is paid per comparison rather than per frame.
fn fold_case(text: &str) -> String {
    text.to_lowercase()
}
