//! [`ByteBudgetCache`] — one byte-budgeted, least-recently-used cache, for every consumer that
//! needs one.
//!
//! # Why this is here and not in the crate that first needed one
//!
//! Three crates in this workspace hold expensive things under a ceiling, and they sit at three
//! different ranks: `mjx-scene`'s tessellation cache at 1.7, `mjx-session`'s worksheet residency at
//! 3.5, and `mjx-view`'s per-stage caches at 3.8. A cache written in the highest of those is
//! unreachable from the other two — an edge upward is what `xtask/tests/layering.rs` refuses — so
//! the choice is one implementation at the floor of the workspace or three implementations that
//! drift.
//!
//! This is the same argument, and the same answer, as [`crate::measure`]: `Emu` was `mjx-dml`'s
//! until MJXOFF-160, and moved down here when the box model at rank 1.6 needed it and could not
//! reach 2.0. A byte-budgeted LRU is nobody's markup either.
//!
//! # The trap this type is written against
//!
//! **A cache that evicts everything satisfies every byte bound perfectly.** A ceiling asserted from
//! above cannot tell a working cache from an empty one, and neither can a ceiling never crossed
//! tell a budget from a comment. So the type is built so both halves are observable:
//!
//! * an entry larger than the whole budget is **never admitted** — it is handed back as
//!   [`Admission::Rejected`] and counted in [`CacheStats::rejections`], rather than admitted and
//!   then made room for by throwing the cache away;
//! * an entry that fits evicts the least recently used entries until it fits **and no further**, so
//!   after any successful insertion the cache holds at least that entry;
//! * [`CacheStats::evictions`] counts what was dropped and
//!   [`ByteBudgetCache::least_recently_used`] says what would go next, so a test can require that
//!   eviction happened *and* prove which entry it took.
//!
//! # Pinning, and the one honest hole in the bound
//!
//! Some entries must not be evicted no matter how old they are: a worksheet with unwritten edits
//! (evicting it would lose them) and the page the reader is looking at (evicting it would make the
//! next frame re-do the work that produced this one). [`ByteBudgetCache::pin`] excludes an entry
//! from eviction.
//!
//! Pinned bytes are still **charged** to [`ByteBudgetCache::bytes`], and they are reported
//! separately by [`ByteBudgetCache::pinned_bytes`], because the alternative — not counting them —
//! would be a budget that quietly stopped describing the memory in use. The consequence is stated
//! rather than hidden: **pinned bytes are a floor the budget cannot go below.** If the pinned set
//! alone exceeds the budget, [`ByteBudgetCache::bytes`] exceeds it too, and every further insertion
//! is rejected with [`Rejection::PinnedFloor`]. A caller that pins more than it budgeted for is
//! told so by the rejection count rather than by an out-of-memory kill.

use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

/// How many bytes of bookkeeping one entry costs beyond the key and value a caller measured.
///
/// A hash-map slot, a b-tree node's share, the [`Arc`]'s two counters and the entry record itself.
/// An estimate, and deliberately generous: a budget that undercounted its own overhead would be a
/// budget the allocator does not honour.
pub const ENTRY_OVERHEAD_BYTES: usize = 128;

/// What a cache has done, for a caller measuring it rather than trusting it.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct CacheStats {
    /// Lookups answered from the cache.
    pub hits: u64,
    /// Lookups that were not.
    pub misses: u64,
    /// Entries dropped to stay inside the budget.
    ///
    /// **The figure a test asserts is not zero before it trusts the budget.** A cache whose
    /// eviction path has never run is a cache whose byte bound is held by an accident of the
    /// workload, and is indistinguishable from a cache with no budget at all.
    pub evictions: u64,
    /// Insertions refused — see [`Rejection`].
    pub rejections: u64,
    /// Entries dropped because a caller said their contents were stale, rather than to make room.
    pub suppressions: u64,
}

/// Why an insertion was refused.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Rejection {
    /// The entry's own cost exceeds the whole budget. Admitting it would empty the cache to make
    /// room for something the cache cannot keep.
    LargerThanBudget,
    /// Room could not be made because everything else is pinned. See the module documentation:
    /// pinned bytes are a floor the budget cannot go below.
    PinnedFloor,
}

/// What an [`insert`](ByteBudgetCache::insert) did.
///
/// A refused value is handed **back**, because the caller usually still needs it — a tessellation
/// too large to cache is still the right triangles, and a page too large to keep is still the page
/// this frame paints.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Admission<V> {
    /// The cache is holding it.
    Admitted,
    /// The cache is not, and here it is back.
    Rejected {
        /// The value that was offered.
        value: V,
        /// Why it was refused.
        reason: Rejection,
    },
}

impl<V> Admission<V> {
    /// Whether the cache took it.
    #[must_use]
    pub const fn is_admitted(&self) -> bool {
        matches!(self, Self::Admitted)
    }

    /// Why it was refused, or `None` when it was not.
    #[must_use]
    pub const fn rejection(&self) -> Option<Rejection> {
        match self {
            Self::Admitted => None,
            Self::Rejected { reason, .. } => Some(*reason),
        }
    }
}

/// One cached value, its measured cost, and when it was last reached.
#[derive(Debug)]
struct Entry<V> {
    value: V,
    bytes: usize,
    last_used: u64,
    pinned: bool,
}

/// Values kept under a byte budget, least recently used first out.
///
/// See the module documentation for the trap this is written against and for what pinning does to
/// the bound.
pub struct ByteBudgetCache<K, V> {
    budget: usize,
    bytes: usize,
    pinned_bytes: usize,
    /// Monotonic. Every use takes the next reading, so two entries never share one and the
    /// eviction order is a total order rather than a tie-break.
    clock: u64,
    entries: HashMap<Arc<K>, Entry<V>>,
    /// Every **evictable** entry by the clock reading of its last use. The first key is the least
    /// recently used one, so eviction is a `pop_first` and never a scan. A pinned entry is absent
    /// from here entirely, which is what makes "skip the pinned ones" free rather than a walk.
    order: BTreeMap<u64, Arc<K>>,
    stats: CacheStats,
}

impl<K, V> fmt::Debug for ByteBudgetCache<K, V> {
    /// Deliberately independent of `K: Debug` and `V: Debug`: a cache holding a hundred display
    /// lists should print its budget and its counters, never its contents.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ByteBudgetCache")
            .field("budget", &self.budget)
            .field("bytes", &self.bytes)
            .field("pinned_bytes", &self.pinned_bytes)
            .field("entries", &self.entries.len())
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

impl<K: Eq + Hash, V> ByteBudgetCache<K, V> {
    /// A cache that will hold at most `budget` bytes of values, keys and bookkeeping.
    ///
    /// A budget of zero is legal and means *never cache*: every lookup misses and every insertion
    /// is rejected. That is the honest reading of "no memory for this", and it is what makes the
    /// budget's lower bound testable — a cache that silently kept one entry anyway would satisfy no
    /// bound at all.
    #[must_use]
    pub fn new(budget: usize) -> Self {
        Self {
            budget,
            bytes: 0,
            pinned_bytes: 0,
            clock: 0,
            entries: HashMap::new(),
            order: BTreeMap::new(),
            stats: CacheStats::default(),
        }
    }

    /// The ceiling this cache holds itself to.
    #[must_use]
    pub const fn budget(&self) -> usize {
        self.budget
    }

    /// Moves the ceiling, evicting down to the new one immediately.
    ///
    /// Lowering a budget and leaving the cache over it would make [`budget`](Self::budget) a
    /// statement about the future rather than about the cache.
    pub fn set_budget(&mut self, budget: usize) {
        self.budget = budget;
        while self.bytes > self.budget {
            if !self.evict_least_recently_used() {
                break;
            }
        }
    }

    /// How many bytes it is holding — values, keys and per-entry overhead together, pinned entries
    /// included.
    #[must_use]
    pub const fn bytes(&self) -> usize {
        self.bytes
    }

    /// How many of [`bytes`](Self::bytes) belong to entries that cannot be evicted.
    #[must_use]
    pub const fn pinned_bytes(&self) -> usize {
        self.pinned_bytes
    }

    /// How many entries it is holding.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether it is holding none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// What it has done.
    #[must_use]
    pub const fn stats(&self) -> CacheStats {
        self.stats
    }

    /// Every key it is holding, in no particular order.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.keys().map(AsRef::as_ref)
    }

    /// The key eviction would take next, or `None` when nothing is evictable.
    ///
    /// **For a test to assert *which* entry went**, which is the half of an eviction gate that a
    /// count cannot express: a cache that evicted the most recently used entry every time would
    /// report exactly the same eviction count as one that got it right.
    #[must_use]
    pub fn least_recently_used(&self) -> Option<&K> {
        self.order.first_key_value().map(|(_, key)| key.as_ref())
    }

    /// Whether `key` is held — **without** touching the eviction order or counting a lookup.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Arc<K>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.entries.contains_key(key)
    }

    /// The value under `key`, **without** touching the eviction order or counting a lookup.
    ///
    /// For a caller inspecting the cache rather than using it — a report, an assertion, a debug
    /// view. Reaching for this on the hot path would make the LRU order a lie.
    pub fn peek<Q>(&self, key: &Q) -> Option<&V>
    where
        Arc<K>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.entries.get(key).map(|entry| &entry.value)
    }

    /// The value under `key`, marking the entry as the most recently used.
    ///
    /// Two hash lookups rather than one, and deliberately: the promotion has to write through
    /// `order` while the entry is borrowed, and a single-lookup version would either hold that
    /// borrow across the write or duplicate the entry's clock in two places. `mjx-scene`'s
    /// tessellation cache learned the second failure the expensive way — an entry left at a stale
    /// clock is promoted exactly once and never again, and nothing about the byte budget can see
    /// it. The second lookup is a hash of a key already in cache; the correctness is worth it.
    pub fn get<Q>(&mut self, key: &Q) -> Option<&V>
    where
        Arc<K>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if !self.touch(key) {
            return None;
        }
        self.entries.get(key).map(|entry| &entry.value)
    }

    /// The value under `key`, mutably, marking the entry as the most recently used.
    ///
    /// **Mutating a value does not re-measure it.** The cost passed to [`insert`](Self::insert) is
    /// what the budget goes on accounting, so a caller that grows a value in place must re-insert
    /// it with its new cost or the budget stops describing the memory in use.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        Arc<K>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if !self.touch(key) {
            return None;
        }
        self.entries.get_mut(key).map(|entry| &mut entry.value)
    }

    /// Promotes `key` to most-recently-used and records the hit or miss. `false` when absent.
    fn touch<Q>(&mut self, key: &Q) -> bool
    where
        Arc<K>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.clock += 1;
        let clock = self.clock;
        let Some(entry) = self.entries.get_mut(key) else {
            self.stats.misses += 1;
            return false;
        };
        // **One expression, not two.** The entry's clock and its key in the eviction order are two
        // records of one fact, and writing them separately is how they come apart. `mem::replace`
        // makes the value written and the value removed provably the same one.
        let previous = std::mem::replace(&mut entry.last_used, clock);
        let pinned = entry.pinned;
        if !pinned {
            if let Some(owned) = self.order.remove(&previous) {
                self.order.insert(clock, owned);
            }
        }
        self.stats.hits += 1;
        true
    }

    /// Holds `value` under `key` at a measured cost of `cost` bytes, evicting the least recently
    /// used entries until it fits.
    ///
    /// `cost` is the caller's measurement of the value and its key; [`ENTRY_OVERHEAD_BYTES`] is
    /// added here, so a caller measures what it owns and never has to guess what a hash map slot
    /// costs.
    ///
    /// Re-inserting a key that is already held replaces it — and **keeps its pin**, so a caller
    /// that re-lays out the page under the reader's eye does not have to remember to pin it again.
    /// One consequence is worth stating rather than discovering: a re-insertion refused with
    /// [`Rejection::PinnedFloor`] has already dropped the previous value, because its bytes were
    /// released before room was sought. [`Rejection::LargerThanBudget`] is decided *before* that and
    /// leaves the previous value untouched.
    pub fn insert(&mut self, key: K, value: V, cost: usize) -> Admission<V> {
        let cost = cost.saturating_add(ENTRY_OVERHEAD_BYTES);
        if cost > self.budget {
            self.stats.rejections += 1;
            return Admission::Rejected {
                value,
                reason: Rejection::LargerThanBudget,
            };
        }
        // The entry being replaced stops counting before room is made for the new one, so replacing
        // a page with a slightly larger one does not evict a neighbour it did not need to.
        let displaced = self.suppress_without_counting(&key);
        let pinned = displaced.is_some_and(|entry| entry.pinned);
        while self.bytes + cost > self.budget {
            if !self.evict_least_recently_used() {
                self.stats.rejections += 1;
                return Admission::Rejected {
                    value,
                    reason: Rejection::PinnedFloor,
                };
            }
        }
        self.clock += 1;
        let clock = self.clock;
        let key = Arc::new(key);
        self.entries.insert(
            Arc::clone(&key),
            Entry {
                value,
                bytes: cost,
                last_used: clock,
                pinned,
            },
        );
        if pinned {
            self.pinned_bytes += cost;
        } else {
            self.order.insert(clock, key);
        }
        self.bytes += cost;
        Admission::Admitted
    }

    /// Holds `value` under `key` **whatever the budget says**, after evicting down to make what
    /// room it can.
    ///
    /// The insertion cannot fail, so there is nothing to hand back. Use it where the caller must
    /// have the value resident and a refusal would be a broken feature rather than a saved byte —
    /// the worksheet a session is about to edit, the page the next frame paints. The budget then
    /// governs **how many other entries stay**, which is what a residency budget actually means;
    /// [`bytes`](Self::bytes) may exceed [`budget`](Self::budget) afterwards, and reporting that
    /// honestly is the point of measuring it.
    ///
    /// Room is made *before* the entry goes in, so the entry being inserted is never the one
    /// evicted to fit it.
    pub fn insert_retained(&mut self, key: K, value: V, cost: usize) {
        let cost = cost.saturating_add(ENTRY_OVERHEAD_BYTES);
        let displaced = self.suppress_without_counting(&key);
        let pinned = displaced.is_some_and(|entry| entry.pinned);
        while self.bytes + cost > self.budget {
            if !self.evict_least_recently_used() {
                break;
            }
        }
        self.clock += 1;
        let clock = self.clock;
        let key = Arc::new(key);
        self.entries.insert(
            Arc::clone(&key),
            Entry {
                value,
                bytes: cost,
                last_used: clock,
                pinned,
            },
        );
        if pinned {
            self.pinned_bytes += cost;
        } else {
            self.order.insert(clock, key);
        }
        self.bytes += cost;
    }

    /// Evicts until the cache is inside its budget, or until nothing evictable is left.
    ///
    /// What a caller runs after unpinning, since an unpinned entry is back under the order but the
    /// bytes it was allowed to hold over the ceiling are not given back by themselves.
    pub fn trim(&mut self) {
        while self.bytes > self.budget {
            if !self.evict_least_recently_used() {
                break;
            }
        }
    }

    /// Excludes `key` from eviction. `false` when it is not held.
    ///
    /// See the module documentation: a pinned entry's bytes are still charged, and a pinned set
    /// larger than the budget is a floor the budget cannot go below.
    pub fn pin<Q>(&mut self, key: &Q) -> bool
    where
        Arc<K>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let Some(entry) = self.entries.get_mut(key) else {
            return false;
        };
        if entry.pinned {
            return true;
        }
        entry.pinned = true;
        let (last_used, bytes) = (entry.last_used, entry.bytes);
        self.order.remove(&last_used);
        self.pinned_bytes += bytes;
        true
    }

    /// Puts `key` back under the eviction order, as the most recently used entry. `false` when it
    /// is not held.
    pub fn unpin<Q>(&mut self, key: &Q) -> bool
    where
        Arc<K>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let Some((owned, entry)) = self.entries.get_key_value(key) else {
            return false;
        };
        if !entry.pinned {
            return true;
        }
        let (owned, bytes) = (Arc::clone(owned), entry.bytes);
        self.clock += 1;
        let clock = self.clock;
        if let Some(entry) = self.entries.get_mut::<K>(&*owned) {
            entry.pinned = false;
            entry.last_used = clock;
        }
        self.pinned_bytes = self.pinned_bytes.saturating_sub(bytes);
        self.order.insert(clock, owned);
        true
    }

    /// Puts every pinned entry back under the eviction order.
    ///
    /// What a viewport calls before pinning the pages the *next* frame shows, so that a pin set is
    /// replaced rather than accumulated — an accumulating pin set is how a byte budget becomes
    /// decoration one frame at a time.
    pub fn unpin_all(&mut self) {
        let pinned: Vec<Arc<K>> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.pinned)
            .map(|(key, _)| Arc::clone(key))
            .collect();
        for key in pinned {
            self.clock += 1;
            let clock = self.clock;
            if let Some(entry) = self.entries.get_mut(&key) {
                entry.pinned = false;
                entry.last_used = clock;
                self.pinned_bytes = self.pinned_bytes.saturating_sub(entry.bytes);
                self.order.insert(clock, key);
            }
        }
    }

    /// Drops `key` because what it holds is stale, and gives the value back.
    ///
    /// Distinct from eviction and counted separately: eviction is the budget doing its job, and a
    /// suppression is a caller saying the cached answer is no longer the right answer. A gate that
    /// added the two together could not tell an invalidation storm from a memory shortage.
    ///
    /// **Suppression ignores the pin.** A pin says "do not drop this to save memory"; it cannot say
    /// "keep showing the reader a stale page".
    pub fn suppress<Q>(&mut self, key: &Q) -> Option<V>
    where
        Arc<K>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let entry = self.suppress_without_counting(key)?;
        self.stats.suppressions += 1;
        Some(entry.value)
    }

    /// Drops every entry whose key `predicate` accepts, and reports how many went.
    pub fn suppress_matching(&mut self, mut predicate: impl FnMut(&K) -> bool) -> usize {
        let doomed: Vec<Arc<K>> = self
            .entries
            .keys()
            .filter(|key| predicate(key.as_ref()))
            .map(Arc::clone)
            .collect();
        let mut suppressed = 0;
        for key in doomed {
            if self.suppress_without_counting(&key).is_some() {
                suppressed += 1;
                self.stats.suppressions += 1;
            }
        }
        suppressed
    }

    /// Forget everything, counters kept.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.bytes = 0;
        self.pinned_bytes = 0;
    }

    /// Removes an entry and un-charges it, without touching a counter.
    fn suppress_without_counting<Q>(&mut self, key: &Q) -> Option<Entry<V>>
    where
        Arc<K>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let entry = self.entries.remove(key)?;
        self.bytes = self.bytes.saturating_sub(entry.bytes);
        if entry.pinned {
            self.pinned_bytes = self.pinned_bytes.saturating_sub(entry.bytes);
        } else {
            self.order.remove(&entry.last_used);
        }
        Some(entry)
    }

    /// Drop the least recently used **evictable** entry. `false` when there was nothing to drop.
    fn evict_least_recently_used(&mut self) -> bool {
        let Some((_, key)) = self.order.pop_first() else {
            return false;
        };
        if let Some(entry) = self.entries.remove(&key) {
            self.bytes = self.bytes.saturating_sub(entry.bytes);
            self.stats.evictions += 1;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every value costs this, plus [`ENTRY_OVERHEAD_BYTES`], so an arithmetic assertion below is
    /// readable rather than a sum of magic numbers.
    const VALUE_BYTES: usize = 72;

    /// One entry's charge against the budget.
    const PER_ENTRY: usize = VALUE_BYTES + ENTRY_OVERHEAD_BYTES;

    fn cache_of(entries: usize) -> ByteBudgetCache<u32, u32> {
        ByteBudgetCache::new(entries * PER_ENTRY)
    }

    #[test]
    fn an_entry_larger_than_the_whole_budget_is_never_admitted() {
        let mut cache = cache_of(2);
        let admission = cache.insert(1, 1, cache.budget());
        assert_eq!(admission.rejection(), Some(Rejection::LargerThanBudget));
        assert!(cache.is_empty(), "the cache was not emptied to make room");
        assert_eq!(cache.stats().rejections, 1);
        assert_eq!(cache.stats().evictions, 0);
    }

    #[test]
    fn eviction_takes_the_least_recently_used_and_no_more() {
        let mut cache = cache_of(3);
        for key in 0..3 {
            assert!(cache.insert(key, key, VALUE_BYTES).is_admitted());
        }
        // Reach 0 and 2, leaving 1 as the oldest. Without this the insertion order would decide,
        // and the test would pass for a cache that evicted the *first inserted* entry.
        assert_eq!(cache.get(&0), Some(&0));
        assert_eq!(cache.get(&2), Some(&2));
        assert_eq!(cache.least_recently_used(), Some(&1));

        assert!(cache.insert(3, 3, VALUE_BYTES).is_admitted());
        assert_eq!(cache.stats().evictions, 1, "exactly one entry made room");
        assert!(!cache.contains_key(&1), "the least recently used went");
        assert!(cache.contains_key(&0) && cache.contains_key(&2) && cache.contains_key(&3));
        assert_eq!(cache.bytes(), 3 * PER_ENTRY);
    }

    #[test]
    fn a_pinned_entry_is_never_evicted_and_its_bytes_are_still_charged() {
        let mut cache = cache_of(2);
        assert!(cache.insert(0, 0, VALUE_BYTES).is_admitted());
        assert!(cache.insert(1, 1, VALUE_BYTES).is_admitted());
        assert!(cache.pin(&0));
        assert_eq!(cache.pinned_bytes(), PER_ENTRY);
        assert_eq!(cache.bytes(), 2 * PER_ENTRY, "a pin does not un-charge");

        assert!(cache.insert(2, 2, VALUE_BYTES).is_admitted());
        assert!(cache.contains_key(&0), "the pinned entry survived");
        assert!(!cache.contains_key(&1), "the evictable one went instead");

        // Everything left is pinned or newer than the budget allows: the floor is reached.
        assert!(cache.pin(&2));
        let admission = cache.insert(3, 3, VALUE_BYTES);
        assert_eq!(admission.rejection(), Some(Rejection::PinnedFloor));
    }

    #[test]
    fn unpin_all_puts_everything_back_under_the_order() {
        let mut cache = cache_of(2);
        assert!(cache.insert(0, 0, VALUE_BYTES).is_admitted());
        assert!(cache.insert(1, 1, VALUE_BYTES).is_admitted());
        assert!(cache.pin(&0));
        assert!(cache.pin(&1));
        assert_eq!(cache.least_recently_used(), None);
        cache.unpin_all();
        assert_eq!(cache.pinned_bytes(), 0);
        assert!(cache.least_recently_used().is_some());
        assert!(cache.insert(2, 2, VALUE_BYTES).is_admitted());
        assert_eq!(cache.stats().evictions, 1);
    }

    #[test]
    fn a_suppression_is_counted_apart_from_an_eviction_and_ignores_the_pin() {
        let mut cache = cache_of(4);
        for key in 0..4 {
            assert!(cache.insert(key, key * 10, VALUE_BYTES).is_admitted());
        }
        assert!(cache.pin(&2));
        assert_eq!(cache.suppress(&2), Some(20));
        assert_eq!(cache.stats().suppressions, 1);
        assert_eq!(cache.stats().evictions, 0);
        assert_eq!(cache.pinned_bytes(), 0, "the pinned charge went with it");

        assert_eq!(cache.suppress_matching(|key| *key % 2 == 1), 2);
        assert_eq!(cache.stats().suppressions, 3);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.bytes(), PER_ENTRY);
    }

    #[test]
    fn re_inserting_a_key_replaces_it_keeps_its_pin_and_does_not_evict_a_neighbour() {
        let mut cache = cache_of(2);
        assert!(cache.insert(0, 0, VALUE_BYTES).is_admitted());
        assert!(cache.insert(1, 1, VALUE_BYTES).is_admitted());
        assert!(cache.pin(&0));
        assert!(cache.insert(0, 99, VALUE_BYTES).is_admitted());
        assert_eq!(cache.peek(&0), Some(&99));
        assert_eq!(cache.len(), 2, "the neighbour was not evicted");
        assert_eq!(
            cache.pinned_bytes(),
            PER_ENTRY,
            "the pin survived the replace"
        );
    }

    #[test]
    fn a_retained_insertion_is_never_refused_and_says_so_in_the_byte_count() {
        let mut cache = cache_of(2);
        assert!(cache.insert(0, 0, VALUE_BYTES).is_admitted());
        assert!(cache.pin(&0));
        assert!(cache.insert(1, 1, VALUE_BYTES).is_admitted());
        assert!(cache.pin(&1));
        // Nothing is evictable, so an ordinary insertion is refused here.
        assert_eq!(
            cache.insert(2, 2, VALUE_BYTES).rejection(),
            Some(Rejection::PinnedFloor),
        );
        // A retained one is not.
        cache.insert_retained(2, 2, VALUE_BYTES);
        assert_eq!(cache.peek(&2), Some(&2));
        assert_eq!(cache.len(), 3);
        assert!(
            cache.bytes() > cache.budget(),
            "the overrun is reported rather than hidden: {} bytes over a budget of {}",
            cache.bytes(),
            cache.budget(),
        );

        // Unpinning alone does not give the bytes back; trimming does.
        cache.unpin_all();
        assert!(cache.bytes() > cache.budget());
        cache.trim();
        assert!(cache.bytes() <= cache.budget());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn a_retained_insertion_never_evicts_the_entry_it_is_inserting() {
        let mut cache = cache_of(1);
        cache.insert_retained(0, 0, VALUE_BYTES);
        cache.insert_retained(1, 1, VALUE_BYTES);
        assert_eq!(cache.peek(&1), Some(&1), "the newcomer is held");
        assert!(!cache.contains_key(&0), "and the older entry made room");
        assert_eq!(cache.bytes(), PER_ENTRY);
    }

    #[test]
    fn a_budget_of_zero_caches_nothing_and_says_so() {
        let mut cache: ByteBudgetCache<u32, u32> = ByteBudgetCache::new(0);
        assert_eq!(
            cache.insert(0, 0, 0).rejection(),
            Some(Rejection::LargerThanBudget),
        );
        assert_eq!(cache.get(&0), None);
        assert_eq!(cache.stats().misses, 1);
        assert!(cache.is_empty());
    }

    #[test]
    fn lowering_the_budget_evicts_down_to_it_immediately() {
        let mut cache = cache_of(4);
        for key in 0..4 {
            assert!(cache.insert(key, key, VALUE_BYTES).is_admitted());
        }
        cache.set_budget(2 * PER_ENTRY);
        assert_eq!(cache.len(), 2);
        assert!(cache.bytes() <= cache.budget());
        assert_eq!(cache.stats().evictions, 2);
    }

    #[test]
    fn a_peek_does_not_move_an_entry_in_the_eviction_order() {
        let mut cache = cache_of(3);
        for key in 0..3 {
            assert!(cache.insert(key, key, VALUE_BYTES).is_admitted());
        }
        assert_eq!(cache.least_recently_used(), Some(&0));
        assert_eq!(cache.peek(&0), Some(&0));
        assert_eq!(
            cache.least_recently_used(),
            Some(&0),
            "a peek that promoted would make an inspection change the answer",
        );
        assert_eq!(cache.stats().hits, 0, "a peek is not a lookup");
    }
}
