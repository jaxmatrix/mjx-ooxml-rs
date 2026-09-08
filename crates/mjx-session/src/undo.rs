//! Undo units — **semantic**, and deliberately independent of the commit window.
//!
//! # The bug this module is written against
//!
//! `docs/client-platform/SESSION_AND_PERSISTENCE.md` §5 names it: using the persistence coalescing
//! window as the undo unit, so undo jumps back by however much happened to be batched. It is easy,
//! common, and very visible — the user types a sentence, the commit timer happens to fire in the
//! middle of it, and one undo removes either three words or thirty depending on nothing the user did.
//!
//! The two are different kinds of thing:
//!
//! * **Undo units are semantic** — a word of typing, one drag, one formatting command. They are
//!   decided by what the operation *is*, what it targets, and whether the user paused.
//! * **Persistence batches are economic** — whatever accumulated since the last commit.
//!
//! Neither constrains the other. A single undo unit may span several commits; a single commit may
//! contain many undo units. `tests/undo_granularity.rs` asserts both directions, because asserting
//! only one of them is green for an implementation that made units *smaller* than batches by
//! accident.
//!
//! # Why the stacks hold their own operations rather than pointing into the journal
//!
//! It looks like duplication and it is the whole independence. **The journal is truncated at every
//! successful commit** — that is what makes recovery "the last commit plus the tail" and what stops
//! it growing without bound. A unit that began before the last commit would have nothing left to
//! undo if undo read from there, and the undo depth would then be *exactly* the commit window: the
//! bug above, arrived at from the other direction.
//!
//! So a unit owns its steps, and its cost is bounded by [`UndoPolicy::maximum_units`] rather than by
//! a schedule.
//!
//! # What starts a new unit
//!
//! An operation joins the unit in progress when all three hold, and starts a new one otherwise:
//!
//! 1. it is the **same kind** of operation — typing does not merge into a drag;
//! 2. it targets the **same node** — the part and the path, ignoring the character range, because a
//!    caret moving through one run is still one word of typing; and
//! 3. the user has not **paused** longer than [`UndoPolicy::idle_break_millis`].
//!
//! A run of typing therefore coalesces, a drag's hundred pointer moves coalesce, and the moment the
//! user moves to another shape or stops for a beat the next edit is its own unit.

use std::collections::VecDeque;

use mjx_layout::SourceRef;

use crate::operation::Operation;
use crate::schedule::Timestamp;

/// Which undo unit an entry belongs to.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct UndoUnitId(u64);

impl UndoUnitId {
    /// The unit numbered `number`.
    #[must_use]
    pub const fn new(number: u64) -> Self {
        Self(number)
    }

    /// The number.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// One operation and the operation that takes it back.
#[derive(Clone, PartialEq, Debug)]
pub struct UndoStep {
    /// What was done.
    pub forward: Operation,
    /// What undoes it.
    pub inverse: Operation,
}

impl UndoStep {
    /// Bytes this step holds on the heap.
    #[must_use]
    pub fn heap_bytes(&self) -> usize {
        self.forward.heap_bytes() + self.inverse.heap_bytes()
    }
}

/// One semantic unit — everything a single undo takes back.
#[derive(Clone, PartialEq, Debug)]
pub struct UndoUnit {
    id: UndoUnitId,
    kind_tag: u8,
    address: SourceRef,
    last_at: Timestamp,
    steps: Vec<UndoStep>,
}

impl UndoUnit {
    /// Which unit.
    #[must_use]
    pub const fn id(&self) -> UndoUnitId {
        self.id
    }

    /// Its steps, oldest first. An undo applies their inverses in **reverse**; a redo applies their
    /// forwards in this order.
    #[must_use]
    pub fn steps(&self) -> &[UndoStep] {
        &self.steps
    }

    /// Bytes the unit holds on the heap.
    #[must_use]
    pub fn heap_bytes(&self) -> usize {
        self.steps.iter().map(UndoStep::heap_bytes).sum()
    }
}

/// When one edit joins the previous one's undo unit, and how much history is kept.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UndoPolicy {
    /// How long a pause ends the unit in progress.
    ///
    /// 700 ms by default: long enough that ordinary typing does not fragment, short enough that
    /// stopping to think ends the word.
    pub idle_break_millis: u64,
    /// How many units of history to keep. The oldest is dropped past this, which is what bounds the
    /// stacks' memory — a session open for a day must not hold a day of edits.
    pub maximum_units: usize,
}

impl Default for UndoPolicy {
    fn default() -> Self {
        Self {
            idle_break_millis: 700,
            maximum_units: 200,
        }
    }
}

/// Decides unit boundaries, and holds the two stacks.
#[derive(Debug)]
pub struct UndoUnits {
    policy: UndoPolicy,
    next: u64,
    /// Whether the newest undoable unit is still accepting joins.
    open: bool,
    /// Units that can be undone, oldest first. A `VecDeque` because the bound drops from the front.
    undoable: VecDeque<UndoUnit>,
    /// Units that have been undone and can be redone, oldest first.
    redoable: Vec<UndoUnit>,
}

impl Default for UndoUnits {
    fn default() -> Self {
        Self::new(UndoPolicy::default())
    }
}

impl UndoUnits {
    /// Units under `policy`.
    #[must_use]
    pub fn new(policy: UndoPolicy) -> Self {
        Self {
            policy,
            next: 0,
            open: false,
            undoable: VecDeque::new(),
            redoable: Vec::new(),
        }
    }

    /// The policy in force.
    #[must_use]
    pub const fn policy(&self) -> UndoPolicy {
        self.policy
    }

    /// Records an edit, joining the unit in progress or opening a new one, and returns which unit it
    /// landed in.
    ///
    /// Recording an edit discards the redo stack, which is the universal rule: once the user does
    /// something new, the branch they undid away is gone.
    pub fn record(
        &mut self,
        operation: &Operation,
        inverse: Operation,
        at: Timestamp,
    ) -> UndoUnitId {
        self.redoable.clear();
        let step = UndoStep {
            forward: operation.clone(),
            inverse,
        };
        if self.open {
            if let Some(unit) = self.undoable.back_mut() {
                let joins = unit.kind_tag == operation.kind().tag()
                    && unit.address.part() == operation.address().part()
                    && unit.address.path() == operation.address().path()
                    && at.since(unit.last_at) <= self.policy.idle_break_millis;
                if joins {
                    unit.last_at = at;
                    unit.steps.push(step);
                    return unit.id;
                }
            }
        }
        let id = UndoUnitId(self.next);
        self.next = self.next.saturating_add(1);
        self.undoable.push_back(UndoUnit {
            id,
            kind_tag: operation.kind().tag(),
            address: operation.address().clone(),
            last_at: at,
            steps: vec![step],
        });
        self.open = true;
        while self.undoable.len() > self.policy.maximum_units.max(1) {
            self.undoable.pop_front();
        }
        id
    }

    /// Ends the unit in progress, so the next edit starts a new one whatever it is.
    ///
    /// This is what a caller calls at a semantic boundary the clock cannot see — a selection change,
    /// a menu command, the end of a drag. It is also what makes "one drag is one undo" hold for a
    /// drag that takes longer than the idle break.
    ///
    /// **A commit never calls it.** That is the independence this module exists for.
    pub fn close_unit(&mut self) {
        self.open = false;
    }

    /// The unit an undo would take back, without taking it.
    #[must_use]
    pub fn next_undo(&self) -> Option<UndoUnitId> {
        self.undoable.back().map(UndoUnit::id)
    }

    /// The unit a redo would put back, without putting it.
    #[must_use]
    pub fn next_redo(&self) -> Option<UndoUnitId> {
        self.redoable.last().map(UndoUnit::id)
    }

    /// Takes the newest undoable unit off the stack.
    ///
    /// The caller applies its inverses in reverse order and then hands it back through
    /// [`finish_undo`](Self::finish_undo) — or through [`return_undo`](Self::return_undo) if a
    /// residency refused one of them.
    pub fn take_undo(&mut self) -> Option<UndoUnit> {
        let unit = self.undoable.pop_back()?;
        self.open = false;
        Some(unit)
    }

    /// Files an undone unit on the redo stack.
    pub fn finish_undo(&mut self, unit: UndoUnit) {
        self.redoable.push(unit);
    }

    /// Puts a unit back where it came from, because applying it failed part of the way through.
    ///
    /// Retrying is safe: every operation in this crate is an absolute assignment, so re-applying an
    /// inverse that already landed changes nothing.
    pub fn return_undo(&mut self, unit: UndoUnit) {
        self.undoable.push_back(unit);
    }

    /// Takes the newest redoable unit off the stack.
    pub fn take_redo(&mut self) -> Option<UndoUnit> {
        let unit = self.redoable.pop()?;
        self.open = false;
        Some(unit)
    }

    /// Files a redone unit back on the undo stack.
    pub fn finish_redo(&mut self, unit: UndoUnit) {
        self.undoable.push_back(unit);
    }

    /// Puts a unit back on the redo stack, because applying it failed part of the way through.
    pub fn return_redo(&mut self, unit: UndoUnit) {
        self.redoable.push(unit);
    }

    /// How many units can be undone.
    #[must_use]
    pub fn undoable(&self) -> usize {
        self.undoable.len()
    }

    /// How many units can be redone.
    #[must_use]
    pub fn redoable(&self) -> usize {
        self.redoable.len()
    }

    /// Bytes both stacks hold on the heap.
    #[must_use]
    pub fn heap_bytes(&self) -> usize {
        self.undoable
            .iter()
            .chain(self.redoable.iter())
            .map(UndoUnit::heap_bytes)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_layout::{LayoutRect, PartId, SourcePath};

    use crate::operation::Value;

    fn typing(path: &[u32], text: &str) -> Operation {
        Operation::set_value(
            SourceRef::new(PartId::PRIMARY, SourcePath::new(path), 0..1),
            Value::text(text),
        )
    }

    fn dragging(path: &[u32]) -> Operation {
        Operation::set_bounds(
            SourceRef::node(PartId::PRIMARY, SourcePath::new(path)),
            LayoutRect::ZERO,
        )
    }

    fn record(units: &mut UndoUnits, operation: &Operation, at: u64) -> UndoUnitId {
        let inverse = operation.clone();
        units.record(operation, inverse, Timestamp::from_millis(at))
    }

    #[test]
    fn a_run_of_typing_at_one_address_is_one_unit_holding_every_step() {
        let mut units = UndoUnits::default();
        let ids: Vec<UndoUnitId> = (0..20)
            .map(|keystroke| record(&mut units, &typing(&[0, 1], "x"), keystroke * 50))
            .collect();
        assert!(ids.windows(2).all(|pair| pair[0] == pair[1]));
        assert_eq!(units.undoable(), 1);
        let unit = units.take_undo().expect("one unit");
        assert_eq!(unit.steps().len(), 20);
    }

    #[test]
    fn a_pause_longer_than_the_break_starts_a_new_unit() {
        let mut units = UndoUnits::default();
        let first = record(&mut units, &typing(&[0], "a"), 0);
        let joined = record(&mut units, &typing(&[0], "b"), 700);
        assert_eq!(first, joined, "exactly at the break still joins");
        let after = record(&mut units, &typing(&[0], "c"), 1_401);
        assert_ne!(first, after);
        assert_eq!(units.undoable(), 2);
    }

    #[test]
    fn moving_to_another_node_starts_a_new_unit_however_fast_the_user_is() {
        let mut units = UndoUnits::default();
        let first = record(&mut units, &typing(&[0], "a"), 0);
        let second = record(&mut units, &typing(&[1], "b"), 1);
        assert_ne!(first, second);
    }

    #[test]
    fn a_different_kind_of_operation_starts_a_new_unit_at_the_same_node() {
        let mut units = UndoUnits::default();
        let typed = record(&mut units, &typing(&[0], "a"), 0);
        let dragged = record(&mut units, &dragging(&[0]), 1);
        assert_ne!(typed, dragged);
    }

    #[test]
    fn the_character_range_does_not_split_a_unit() {
        let mut units = UndoUnits::default();
        let first = record(
            &mut units,
            &Operation::set_value(
                SourceRef::new(PartId::PRIMARY, SourcePath::new(&[0]), 0..1),
                Value::text("a"),
            ),
            0,
        );
        let second = record(
            &mut units,
            &Operation::set_value(
                SourceRef::new(PartId::PRIMARY, SourcePath::new(&[0]), 1..2),
                Value::text("ab"),
            ),
            10,
        );
        assert_eq!(first, second);
    }

    #[test]
    fn closing_the_unit_splits_a_run_the_clock_would_have_joined() {
        let mut units = UndoUnits::default();
        let first = record(&mut units, &typing(&[0], "a"), 0);
        units.close_unit();
        let second = record(&mut units, &typing(&[0], "b"), 1);
        assert_ne!(first, second);
    }

    #[test]
    fn undo_and_redo_move_units_between_the_stacks() {
        let mut units = UndoUnits::default();
        let first = record(&mut units, &typing(&[0], "a"), 0);
        let second = record(&mut units, &typing(&[1], "b"), 1);
        assert_eq!(units.next_undo(), Some(second));
        assert_eq!(units.next_redo(), None);

        let taken = units.take_undo().expect("a unit");
        assert_eq!(taken.id(), second);
        units.finish_undo(taken);
        assert_eq!(units.undoable(), 1);
        assert_eq!(units.next_undo(), Some(first));
        assert_eq!(units.next_redo(), Some(second));

        let redone = units.take_redo().expect("a unit");
        units.finish_redo(redone);
        assert_eq!(units.undoable(), 2);
        assert_eq!(units.redoable(), 0);
    }

    #[test]
    fn a_refused_undo_puts_its_unit_back_where_it_was() {
        let mut units = UndoUnits::default();
        record(&mut units, &typing(&[0], "a"), 0);
        let unit = units.take_undo().expect("a unit");
        assert_eq!(units.undoable(), 0);
        units.return_undo(unit);
        assert_eq!(units.undoable(), 1);
        assert_eq!(units.redoable(), 0);

        let unit = units.take_undo().expect("a unit");
        units.finish_undo(unit);
        let unit = units.take_redo().expect("a unit");
        units.return_redo(unit);
        assert_eq!(units.redoable(), 1);
        assert_eq!(units.undoable(), 0);
    }

    #[test]
    fn a_new_edit_discards_the_redo_branch() {
        let mut units = UndoUnits::default();
        record(&mut units, &typing(&[0], "a"), 0);
        let unit = units.take_undo().expect("a unit");
        units.finish_undo(unit);
        assert_eq!(units.redoable(), 1);
        record(&mut units, &typing(&[0], "b"), 1);
        assert_eq!(units.redoable(), 0);
    }

    #[test]
    fn a_longer_break_policy_joins_what_the_default_would_split() {
        let mut units = UndoUnits::new(UndoPolicy {
            idle_break_millis: 5_000,
            ..UndoPolicy::default()
        });
        assert_eq!(units.policy().idle_break_millis, 5_000);
        let first = record(&mut units, &typing(&[0], "a"), 0);
        let second = record(&mut units, &typing(&[0], "b"), 4_000);
        assert_eq!(first, second);
    }

    #[test]
    fn history_is_bounded_and_the_oldest_unit_is_what_goes() {
        let mut units = UndoUnits::new(UndoPolicy {
            idle_break_millis: 0,
            maximum_units: 3,
        });
        for edit in 0..10_u32 {
            record(&mut units, &typing(&[edit], "x"), u64::from(edit) * 10);
        }
        assert_eq!(units.undoable(), 3);
        // The three that survive are the newest three, so the next undo is edit 9's.
        assert_eq!(units.next_undo(), Some(UndoUnitId::new(9)));
    }

    #[test]
    fn a_maximum_of_zero_still_keeps_the_unit_just_recorded() {
        let mut units = UndoUnits::new(UndoPolicy {
            maximum_units: 0,
            ..UndoPolicy::default()
        });
        record(&mut units, &typing(&[0], "a"), 0);
        assert_eq!(units.undoable(), 1, "a bound of zero would lose the edit");
    }

    #[test]
    fn the_stacks_report_the_bytes_they_hold() {
        let mut units = UndoUnits::default();
        assert_eq!(units.heap_bytes(), 0);
        record(&mut units, &typing(&[0], "abcde"), 0);
        // The forward and the inverse are both the same five-byte text in this helper.
        assert_eq!(units.heap_bytes(), 10);
        let unit = units.take_undo().expect("a unit");
        units.finish_undo(unit);
        assert_eq!(units.heap_bytes(), 10, "the redo stack costs the same");
    }
}
