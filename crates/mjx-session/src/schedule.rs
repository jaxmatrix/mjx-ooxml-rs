//! When a commit fires — the policy, the triggers, and the clock they are read against.
//!
//! # A pure interval is the wrong policy on its own
//!
//! `docs/client-platform/SESSION_AND_PERSISTENCE.md` §3: an interval alone is either too slow to be
//! safe or too frequent to be worth doing. [`CommitScheduler`] fires on whichever of six conditions
//! becomes true first, and defers a due commit while a gesture is in flight — a commit landing
//! mid-drag costs a dropped frame, which is the one thing the whole render architecture is built to
//! avoid.
//!
//! # Why the clock is injected
//!
//! `std::time::Instant::now()` panics on `wasm32-unknown-unknown`, and this crate must reach the
//! browser. It is also the difference between a scheduler that can be *tested* — advance the clock
//! two seconds, assert the trigger — and one whose suite has to sleep. So the crate ships the
//! [`Clock`] trait and [`ManualClock`], and the host passes a clock that reads whatever monotonic
//! source it already has (a frame timestamp, `performance.now()`, `Instant`).

use std::cell::Cell;

/// A monotonic instant, in milliseconds from an origin only the clock knows.
///
/// Milliseconds because every budget in the specification is stated in them — 2 s idle, 30 s max
/// age, a sub-second journal flush — and a monotonic `u64` of them runs for half a billion years.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct Timestamp(u64);

impl Timestamp {
    /// The origin.
    pub const ORIGIN: Self = Self(0);

    /// The instant `millis` after the origin.
    #[must_use]
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis)
    }

    /// Milliseconds since the origin.
    #[must_use]
    pub const fn millis(self) -> u64 {
        self.0
    }

    /// How long since `earlier`, saturating at zero rather than wrapping — a clock that went
    /// backwards is a host's bug and must not become a scheduler that never commits again.
    #[must_use]
    pub const fn since(self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0)
    }
}

/// A monotonic time source.
///
/// `&self` rather than `&mut self`, because a clock is read from wherever the time is needed and
/// reading it is not a mutation of the session.
pub trait Clock {
    /// Now.
    fn now(&self) -> Timestamp;
}

/// A clock a caller drives.
///
/// It is the test clock, and it is also the honest shape for a host that already has a frame
/// timestamp: a session polled once per frame should be reading the frame's own time, not asking the
/// operating system again.
#[derive(Debug, Default)]
pub struct ManualClock {
    millis: Cell<u64>,
}

impl ManualClock {
    /// A clock at the origin.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves the clock forward by `millis`.
    pub fn advance(&self, millis: u64) {
        self.millis.set(self.millis.get().saturating_add(millis));
    }

    /// Moves the clock to exactly `millis` after the origin.
    pub fn set(&self, millis: u64) {
        self.millis.set(millis);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Timestamp {
        Timestamp(self.millis.get())
    }
}

/// Why a commit fired.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CommitTrigger {
    /// The user paused.
    Idle,
    /// Too long since the last commit, while editing continued.
    MaxAge,
    /// Enough model is waiting to be serialised that it should not wait for a clock — a paste of ten
    /// thousand rows.
    DirtyBytes,
    /// The uncommitted journal reached its memory bound.
    JournalBytes,
    /// The caller asked. Always immediate, and never deferred by a gesture.
    Explicit,
    /// The application is going to the background.
    ///
    /// **Mandatory, and never deferred.** iOS terminates backgrounded applications without warning,
    /// and this is the single likeliest way to lose work on the mobile target.
    Backgrounded,
    /// Something that needs the document consistent is about to happen — an export, a print, an
    /// external hand-off, a close.
    Consistency,
}

impl CommitTrigger {
    /// Whether a gesture in flight may defer this trigger.
    ///
    /// Four of the seven wait for the gesture to end, because they are economics and a dropped frame
    /// is worse than two more seconds of exposure. The other three do not: an explicit save was
    /// asked for, a backgrounding may be the last moment this process has, and a consistency point
    /// is about to read the document.
    #[must_use]
    pub const fn defers_to_a_gesture(self) -> bool {
        matches!(
            self,
            Self::Idle | Self::MaxAge | Self::DirtyBytes | Self::JournalBytes
        )
    }
}

/// When to commit.
///
/// Every field is optional and `None` disables that trigger, so a host that wants one condition and
/// not the others says so rather than passing a sentinel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CommitPolicy {
    /// Commit this long after the last edit.
    pub idle_millis: Option<u64>,
    /// Commit this long after the last commit, however busy the user is.
    pub max_age_millis: Option<u64>,
    /// Commit when this much model is waiting to be serialised.
    pub dirty_bytes: Option<usize>,
    /// Commit when the uncommitted journal holds this many bytes.
    pub journal_bytes: Option<usize>,
    /// Flush the journal to its sink this often. Much shorter than any commit interval: writing an
    /// operation record is orders of magnitude cheaper than serialising a part, so it costs almost
    /// nothing and it is what bounds the exposure window.
    pub journal_flush_millis: Option<u64>,
    /// Commit synchronously inside every edit.
    ///
    /// **This is the design this crate exists to avoid**, and it is here so the batching gate has a
    /// counterfactual to measure against rather than an argument. `tests/batching.rs` runs the same
    /// twenty keystrokes under [`interactive`](Self::interactive) and under
    /// [`per_operation`](Self::per_operation) and compares the serialisation counts: one against
    /// twenty. A gate that only ran the good policy would be green for an implementation with no
    /// batching in it at all.
    pub after_every_operation: bool,
}

impl CommitPolicy {
    /// The policy an interactive editor runs: idle 2 s, max age 30 s, a 1 MiB dirty threshold, a
    /// 4 MiB journal bound, and a journal flush every 250 ms.
    ///
    /// The numbers are `SESSION_AND_PERSISTENCE.md` §3 and §7.
    #[must_use]
    pub const fn interactive() -> Self {
        Self {
            idle_millis: Some(2_000),
            max_age_millis: Some(30_000),
            dirty_bytes: Some(1 << 20),
            journal_bytes: Some(4 << 20),
            journal_flush_millis: Some(250),
            after_every_operation: false,
        }
    }

    /// Nothing fires on its own; only an explicit save, a backgrounding or a consistency point
    /// commits. For a batch caller that wants residency and no schedule.
    #[must_use]
    pub const fn manual() -> Self {
        Self {
            idle_millis: None,
            max_age_millis: None,
            dirty_bytes: None,
            journal_bytes: None,
            journal_flush_millis: None,
            after_every_operation: false,
        }
    }

    /// The naive policy: serialise on every operation, synchronously, inside the edit. See the
    /// [`after_every_operation`](Self::after_every_operation) field.
    #[must_use]
    pub const fn per_operation() -> Self {
        Self {
            after_every_operation: true,
            ..Self::manual()
        }
    }
}

impl Default for CommitPolicy {
    fn default() -> Self {
        Self::interactive()
    }
}

/// Decides when a commit is due, and holds the gesture that defers one.
#[derive(Debug)]
pub struct CommitScheduler {
    policy: CommitPolicy,
    last_edit: Option<Timestamp>,
    last_commit: Timestamp,
    last_journal_flush: Timestamp,
    /// Whether anything has been recorded since the last commit. A commit with nothing to commit is
    /// a container write for no reason, so every automatic trigger is gated on this.
    pending: bool,
    /// How many gestures are in flight. A count rather than a flag, because a pinch inside a drag is
    /// two, and a flag would let the inner one's end re-enable commits during the outer one.
    gestures: u32,
    /// A trigger asked for out of band, waiting for the next poll.
    requested: Option<CommitTrigger>,
}

impl CommitScheduler {
    /// A scheduler starting at `origin`, which is the clock's reading when the session opened.
    #[must_use]
    pub fn new(policy: CommitPolicy, origin: Timestamp) -> Self {
        Self {
            policy,
            last_edit: None,
            last_commit: origin,
            last_journal_flush: origin,
            pending: false,
            gestures: 0,
            requested: None,
        }
    }

    /// The policy in force.
    #[must_use]
    pub const fn policy(&self) -> &CommitPolicy {
        &self.policy
    }

    /// Replaces the policy — for a [`DocumentSource`-style capability change](crate) where a
    /// high-latency source wants longer intervals and larger batches.
    pub fn set_policy(&mut self, policy: CommitPolicy) {
        self.policy = policy;
    }

    /// Records that an edit just happened.
    pub fn note_edit(&mut self, at: Timestamp) {
        self.last_edit = Some(at);
        self.pending = true;
    }

    /// Records that a commit just succeeded.
    pub fn note_commit(&mut self, at: Timestamp) {
        self.last_commit = at;
        self.pending = false;
        self.requested = None;
    }

    /// Records that the journal just reached its sink.
    pub fn note_journal_flush(&mut self, at: Timestamp) {
        self.last_journal_flush = at;
    }

    /// Asks for a commit at the next poll, for a reason the clock cannot see.
    ///
    /// A stronger request replaces a weaker one: [`Backgrounded`](CommitTrigger::Backgrounded) may
    /// be the last moment the process has, so it outranks everything already waiting.
    pub fn request(&mut self, trigger: CommitTrigger) {
        let replace = match (self.requested, trigger) {
            (None, _) | (_, CommitTrigger::Backgrounded) => true,
            (Some(CommitTrigger::Backgrounded), _) => false,
            _ => true,
        };
        if replace {
            self.requested = Some(trigger);
        }
    }

    /// A gesture began; economic triggers now wait for it to end.
    pub fn begin_gesture(&mut self) {
        self.gestures = self.gestures.saturating_add(1);
    }

    /// A gesture ended.
    pub fn end_gesture(&mut self) {
        self.gestures = self.gestures.saturating_sub(1);
    }

    /// Whether a gesture is in flight.
    #[must_use]
    pub const fn gesture_in_flight(&self) -> bool {
        self.gestures > 0
    }

    /// Whether anything has been recorded since the last commit.
    #[must_use]
    pub const fn has_pending_work(&self) -> bool {
        self.pending
    }

    /// Whether the journal should be flushed now.
    #[must_use]
    pub fn journal_flush_due(&self, now: Timestamp) -> bool {
        self.policy
            .journal_flush_millis
            .is_some_and(|interval| now.since(self.last_journal_flush) >= interval)
    }

    /// Which trigger, if any, fires now.
    ///
    /// The order is the order of urgency, not the order of the specification's table: a memory bound
    /// is a bound and outranks a clock, and a bulk edit outranks a clock for the same reason. `Idle`
    /// is last because it is the *cheapest* moment, so anything that has already become true should
    /// have fired before the user paused.
    #[must_use]
    pub fn due(
        &self,
        now: Timestamp,
        dirty_bytes: usize,
        journal_bytes: usize,
    ) -> Option<CommitTrigger> {
        let trigger = self.first_condition(now, dirty_bytes, journal_bytes)?;
        if self.gesture_in_flight() && trigger.defers_to_a_gesture() {
            return None;
        }
        Some(trigger)
    }

    fn first_condition(
        &self,
        now: Timestamp,
        dirty_bytes: usize,
        journal_bytes: usize,
    ) -> Option<CommitTrigger> {
        if let Some(requested) = self.requested {
            return Some(requested);
        }
        if !self.pending {
            return None;
        }
        if self
            .policy
            .journal_bytes
            .is_some_and(|bound| journal_bytes >= bound)
        {
            return Some(CommitTrigger::JournalBytes);
        }
        if self
            .policy
            .dirty_bytes
            .is_some_and(|bound| dirty_bytes >= bound)
        {
            return Some(CommitTrigger::DirtyBytes);
        }
        if self
            .policy
            .max_age_millis
            .is_some_and(|age| now.since(self.last_commit) >= age)
        {
            return Some(CommitTrigger::MaxAge);
        }
        let last_edit = self.last_edit?;
        if self
            .policy
            .idle_millis
            .is_some_and(|idle| now.since(last_edit) >= idle)
        {
            return Some(CommitTrigger::Idle);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scheduler() -> CommitScheduler {
        CommitScheduler::new(CommitPolicy::interactive(), Timestamp::ORIGIN)
    }

    #[test]
    fn nothing_is_due_before_anything_is_edited() {
        let scheduler = scheduler();
        assert_eq!(scheduler.due(Timestamp::from_millis(60_000), 0, 0), None);
    }

    #[test]
    fn idle_fires_two_seconds_after_the_last_edit_and_not_before() {
        let mut scheduler = scheduler();
        scheduler.note_edit(Timestamp::from_millis(1_000));
        assert_eq!(scheduler.due(Timestamp::from_millis(2_999), 0, 0), None);
        assert_eq!(
            scheduler.due(Timestamp::from_millis(3_000), 0, 0),
            Some(CommitTrigger::Idle)
        );
    }

    #[test]
    fn max_age_fires_while_editing_never_pauses() {
        let mut scheduler = scheduler();
        // An edit every second, so the idle trigger never becomes true.
        for second in 1..=40 {
            let now = Timestamp::from_millis(second * 1_000);
            scheduler.note_edit(now);
            if second < 30 {
                assert_eq!(scheduler.due(now, 0, 0), None, "at {second} s");
            } else {
                assert_eq!(scheduler.due(now, 0, 0), Some(CommitTrigger::MaxAge));
                return;
            }
        }
        unreachable!("the loop returns at thirty seconds");
    }

    #[test]
    fn the_journal_bound_outranks_the_dirty_threshold_and_both_outrank_the_clocks() {
        let mut scheduler = scheduler();
        scheduler.note_edit(Timestamp::from_millis(1_000));
        let now = Timestamp::from_millis(1_001);
        assert_eq!(
            scheduler.due(now, 1 << 20, 4 << 20),
            Some(CommitTrigger::JournalBytes)
        );
        assert_eq!(
            scheduler.due(now, 1 << 20, 0),
            Some(CommitTrigger::DirtyBytes)
        );
        assert_eq!(scheduler.due(now, 0, 0), None);
    }

    #[test]
    fn a_gesture_defers_the_economic_triggers_and_not_the_mandatory_ones() {
        let mut scheduler = scheduler();
        scheduler.note_edit(Timestamp::from_millis(1_000));
        scheduler.begin_gesture();
        let now = Timestamp::from_millis(9_000);
        assert_eq!(scheduler.due(now, 0, 0), None, "idle deferred");
        assert_eq!(scheduler.due(now, 1 << 20, 0), None, "dirty bytes deferred");

        scheduler.request(CommitTrigger::Backgrounded);
        assert_eq!(scheduler.due(now, 0, 0), Some(CommitTrigger::Backgrounded));

        scheduler.end_gesture();
        assert!(!scheduler.gesture_in_flight());
    }

    #[test]
    fn nested_gestures_both_have_to_end() {
        let mut scheduler = scheduler();
        scheduler.note_edit(Timestamp::from_millis(1_000));
        scheduler.begin_gesture();
        scheduler.begin_gesture();
        scheduler.end_gesture();
        assert!(scheduler.gesture_in_flight());
        assert_eq!(scheduler.due(Timestamp::from_millis(9_000), 0, 0), None);
        scheduler.end_gesture();
        assert_eq!(
            scheduler.due(Timestamp::from_millis(9_000), 0, 0),
            Some(CommitTrigger::Idle)
        );
    }

    #[test]
    fn backgrounding_outranks_a_request_already_waiting_and_is_not_outranked_back() {
        let mut scheduler = scheduler();
        scheduler.request(CommitTrigger::Consistency);
        scheduler.request(CommitTrigger::Backgrounded);
        assert_eq!(
            scheduler.due(Timestamp::ORIGIN, 0, 0),
            Some(CommitTrigger::Backgrounded)
        );
        scheduler.request(CommitTrigger::Explicit);
        assert_eq!(
            scheduler.due(Timestamp::ORIGIN, 0, 0),
            Some(CommitTrigger::Backgrounded)
        );
    }

    #[test]
    fn a_requested_commit_fires_with_nothing_pending_and_a_clock_trigger_does_not() {
        let mut scheduler = scheduler();
        assert_eq!(scheduler.due(Timestamp::from_millis(60_000), 0, 0), None);
        scheduler.request(CommitTrigger::Explicit);
        assert_eq!(
            scheduler.due(Timestamp::from_millis(60_000), 0, 0),
            Some(CommitTrigger::Explicit)
        );
    }

    #[test]
    fn the_journal_flush_interval_is_read_against_the_last_flush() {
        let mut scheduler = scheduler();
        assert!(!scheduler.journal_flush_due(Timestamp::from_millis(249)));
        assert!(scheduler.journal_flush_due(Timestamp::from_millis(250)));
        scheduler.note_journal_flush(Timestamp::from_millis(250));
        assert!(!scheduler.journal_flush_due(Timestamp::from_millis(499)));
        assert!(scheduler.journal_flush_due(Timestamp::from_millis(500)));
    }

    #[test]
    fn a_manual_policy_fires_on_nothing_a_clock_can_see() {
        let mut scheduler = CommitScheduler::new(CommitPolicy::manual(), Timestamp::ORIGIN);
        scheduler.note_edit(Timestamp::from_millis(1_000));
        assert_eq!(
            scheduler.due(Timestamp::from_millis(10_000_000), usize::MAX, usize::MAX),
            None
        );
        assert!(!scheduler.journal_flush_due(Timestamp::from_millis(10_000_000)));
    }

    #[test]
    fn a_clock_that_goes_backwards_saturates_rather_than_wrapping() {
        assert_eq!(
            Timestamp::from_millis(5).since(Timestamp::from_millis(9)),
            0
        );
    }

    #[test]
    fn a_manual_clock_advances_and_can_be_set() {
        let clock = ManualClock::new();
        assert_eq!(clock.now(), Timestamp::ORIGIN);
        clock.advance(750);
        assert_eq!(clock.now().millis(), 750);
        clock.set(10);
        assert_eq!(clock.now().millis(), 10);
        clock.advance(u64::MAX);
        assert_eq!(clock.now().millis(), u64::MAX);
    }
}
