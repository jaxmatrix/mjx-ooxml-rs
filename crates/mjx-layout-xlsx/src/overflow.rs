//! Text that overflows into empty neighbours — **the Excel-specific behaviour that looks like a bug
//! when it is missing and like a bug when it is wrong.**
//!
//! # The rule, in four states
//!
//! Type a long label into `A1` of an empty sheet and it runs across `B1`, `C1`, `D1`. Type anything
//! into `C1` and the label is cut off at `C1`'s left edge — not wrapped, not shrunk, *clipped*. That
//! is the whole behaviour, and a renderer that does not have it draws a spreadsheet nobody
//! recognises.
//!
//! | State | When | What the reader sees |
//! |---|---|---|
//! | [`Overflow::Spills`] | the text is wider than its cell and the neighbours in the direction of alignment are empty | one long label crossing several columns |
//! | [`Overflow::StoppedBy`] | the same, but a populated cell is in the way | the label cut off at that cell's edge |
//! | [`Overflow::SuppressedByWrap`] | `alignment@wrapText` | several lines inside the cell |
//! | [`Overflow::SuppressedByFill`] | `alignment@horizontal="fill"` | the text repeated to fill the cell, never leaving it |
//!
//! # Which way it spills
//!
//! The direction is the **resolved** horizontal alignment, not the stated one — a number in a cell
//! whose alignment is `general` is right-aligned, so its overflow goes left:
//!
//! * left, justify, distributed and a `general` cell holding text — rightward;
//! * right and a `general` cell holding a number, a boolean or an error — leftward;
//! * centre and `centerContinuous` — both ways, and the two halves are limited independently, which
//!   is why [`Overflow::Spills`] carries a range of columns rather than a single stop.
//!
//! GUESS: that `distributed` spills rightward like `justify` rather than not at all. A distributed
//! cell whose text exceeds its width has nothing to distribute, and Excel's behaviour there has not
//! been observed on Windows.
//!
//! # Shrink-to-fit does not appear in this table, deliberately
//!
//! `alignment@shrinkToFit` reduces the font size until the text fits, so a shrunk cell never has
//! anything to overflow *with* — the suppression is arithmetic rather than a rule. It is applied in
//! [`crate::cell`] before this module is asked anything.
//!
//! # Cost
//!
//! Two [`Row::cell_after`](mjx_sml::Row::cell_after)/[`cell_before`](mjx_sml::Row::cell_before)
//! probes per overflowing cell, each a binary search in the row's own cell slice. Probing column by
//! column would ask up to 16,383 questions instead, which is the coordinate-range walk the packed
//! store exists to make unnecessary — reintroduced one row at a time.

use mjx_ooxml_types::spreadsheetml::HorizontalAlignment;

/// Which way a cell's text is allowed to leave its own column.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum OverflowDirection {
    /// Rightward only — a left-aligned cell.
    Right,
    /// Leftward only — a right-aligned cell.
    Left,
    /// Both, half the excess each way — a centred cell.
    Both,
}

impl OverflowDirection {
    /// The direction a resolved alignment overflows in.
    #[must_use]
    pub fn of(alignment: HorizontalAlignment) -> Option<Self> {
        match alignment {
            HorizontalAlignment::Left | HorizontalAlignment::Justify => Some(Self::Right),
            HorizontalAlignment::Right => Some(Self::Left),
            HorizontalAlignment::Center | HorizontalAlignment::CenterContinuous => Some(Self::Both),
            // GUESS: `distributed` behaves as `justify` for the purposes of overflow.
            HorizontalAlignment::Distributed => Some(Self::Right),
            // `fill` never leaves its cell; `general` is resolved to a concrete alignment before
            // this is asked, so reaching it here means the caller has not resolved it.
            HorizontalAlignment::Fill | HorizontalAlignment::General => None,
        }
    }
}

/// What happened to a cell whose text is wider than its column.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Overflow {
    /// The text fits inside its own cell; nothing left it.
    Fits,
    /// The text spills across the columns in `columns`, inclusive of both ends.
    ///
    /// `columns.0` is at most the cell's own column and `columns.1` at least it, so a cell that
    /// spills only rightward reports its own column as the left end.
    Spills {
        /// The leftmost and rightmost columns the text may be drawn across, inclusive.
        columns: (u16, u16),
        /// Which way it was allowed to go.
        direction: OverflowDirection,
        /// The populated cell that stopped it on the left, if one did.
        stopped_left_by: Option<u16>,
        /// The populated cell that stopped it on the right, if one did.
        stopped_right_by: Option<u16>,
    },
    /// The text is wider than its cell and a populated neighbour is immediately in the way, so it is
    /// clipped to its own cell.
    StoppedBy {
        /// Which column stopped it.
        column: u16,
        /// Which way it was trying to go.
        direction: OverflowDirection,
    },
    /// `alignment@wrapText` is set, so the text wrapped instead of leaving.
    SuppressedByWrap,
    /// `alignment@horizontal="fill"` is set, so the text repeats inside its own cell.
    SuppressedByFill,
}

impl Overflow {
    /// The columns the text may be drawn across, inclusive — the cell's own column alone when
    /// nothing spilled.
    #[must_use]
    pub fn columns(&self, own: u16) -> (u16, u16) {
        match self {
            Self::Spills { columns, .. } => *columns,
            _ => (own, own),
        }
    }

    /// Whether anything left the cell.
    #[must_use]
    pub fn spills(&self) -> bool {
        matches!(self, Self::Spills { .. })
    }

    /// Whether the text is clipped to its own cell.
    #[must_use]
    pub fn is_clipped(&self) -> bool {
        matches!(self, Self::StoppedBy { .. } | Self::SuppressedByFill)
    }
}

/// How far a cell's text may spread, given the row it is in.
///
/// `needed` is how much wider than its own cell the text is, in EMU; `available` asks the caller how
/// wide a candidate column is, so this module never has to hold a [`crate::geometry::GridGeometry`]
/// and can be tested against a plain closure.
///
/// `occupied` answers whether a candidate column holds anything a reader would see — see
/// [`SheetGrid::cell_is_occupied`](crate::SheetGrid::cell_is_occupied), which is the definition this
/// takes as given rather than repeating.
///
/// `first_populated_after` / `first_populated_before` are the two probes: the nearest populated cell
/// on each side, from the packed store's own index.
#[must_use]
pub fn resolve(
    column: u16,
    needed: mjx_ooxml_core::measure::Emu,
    direction: OverflowDirection,
    mut available: impl FnMut(u16) -> mjx_ooxml_core::measure::Emu,
    mut occupied: impl FnMut(u16) -> bool,
) -> Overflow {
    use mjx_ooxml_core::measure::Emu;
    if needed <= Emu::ZERO {
        return Overflow::Fits;
    }
    let (mut want_left, mut want_right) = match direction {
        OverflowDirection::Right => (Emu::ZERO, needed),
        OverflowDirection::Left => (needed, Emu::ZERO),
        OverflowDirection::Both => {
            let half = needed.divided_by(2);
            (half, needed - half)
        }
    };

    let mut left = column;
    let mut stopped_left_by = None;
    while want_left > Emu::ZERO {
        let Some(candidate) = left.checked_sub(1) else {
            break;
        };
        if occupied(candidate) {
            stopped_left_by = Some(candidate);
            break;
        }
        left = candidate;
        want_left -= available(candidate);
    }

    let mut right = column;
    let mut stopped_right_by = None;
    while want_right > Emu::ZERO {
        let candidate = right.saturating_add(1);
        if u32::from(candidate) >= crate::geometry::COLUMN_COUNT || candidate == right {
            break;
        }
        if occupied(candidate) {
            stopped_right_by = Some(candidate);
            break;
        }
        right = candidate;
        want_right -= available(candidate);
    }

    if left == column && right == column {
        // Nothing was gained. Either an immediate neighbour is populated — which is the state a
        // reader recognises as "cut off" — or the cell is at the edge of the grid.
        return match (direction, stopped_left_by, stopped_right_by) {
            (_, Some(stopper), _) | (_, None, Some(stopper)) => Overflow::StoppedBy {
                column: stopper,
                direction,
            },
            _ => Overflow::Fits,
        };
    }
    Overflow::Spills {
        columns: (left, right),
        direction,
        stopped_left_by,
        stopped_right_by,
    }
}

/// The alignment a `general` cell resolves to.
///
/// §18.8.1 describes `general` as *"align depending on the type of data"*, and the types are the
/// ones `ST_CellType` names: text to the left, numbers to the right, booleans and errors centred.
/// This is the rule that makes a number typed as text visibly obvious in Excel, which is half of why
/// it is worth implementing exactly.
#[must_use]
pub fn resolve_general(cell_type: mjx_ooxml_types::spreadsheetml::CellType) -> HorizontalAlignment {
    use mjx_ooxml_types::spreadsheetml::CellType;
    match cell_type {
        CellType::Number => HorizontalAlignment::Right,
        CellType::Boolean | CellType::Error => HorizontalAlignment::Center,
        CellType::SharedString | CellType::FormulaString | CellType::InlineString => {
            HorizontalAlignment::Left
        }
    }
}
