//! The tessellation cache: triangles, kept per `(path, style, scale bucket)`, under a byte budget.
//!
//! # Why a cache at all
//!
//! Tessellation is the expensive half of drawing a page and its answer depends on nothing that
//! changes between frames: the same path, flattened to the same tolerance, is the same triangles.
//! A page held still on screen would otherwise re-trapezoidate every shape sixty times a second.
//!
//! # Why the key is the path itself and not a hash of it
//!
//! A 64-bit content hash would be smaller and would be wrong once. This cache hands a painter
//! *triangles*, and a collision would hand it somebody else's — a shape drawn as a different shape,
//! which is the class of defect that gets reported as *"the file is corrupt"* and traced to the
//! renderer months later. So the key holds the path's coordinate **bit patterns**, exactly, together
//! with the style words and the bucket, and equality is equality. The cost is one copy of the path
//! per entry, which is small beside the triangles it keys and is charged to the budget like
//! everything else.
//!
//! Bit patterns rather than `f32`s because `f32` is neither [`Eq`] nor [`Hash`], and because two
//! coordinates that print the same may differ in the last bit — the same reasoning
//! [`mjx_text::ScaleBucket`] is built on.
//!
//! # The trap this cache is written against
//!
//! **A cache that evicts everything satisfies every byte bound perfectly.** So the budget here is
//! asserted in both directions and the eviction path is proved to run:
//!
//! * an entry larger than the whole budget is **never inserted** — it is served uncached and
//!   counted in [`MeshCache::oversized`] — rather than admitted and then made room for by throwing
//!   the cache away;
//! * an entry that fits evicts the least recently used entries until it fits *and no further*, so
//!   after any successful insertion the cache holds at least that entry;
//! * [`MeshCache::evictions`] counts what was dropped, so a test can require that eviction happened
//!   rather than assume it.
//!
//! That is written down because the nearest cache in this workspace does not have it.
//! `mjx-text`'s outline cache (`crates/mjx-text/src/raster.rs`, `evict_oldest_outlines`) is bounded
//! by a *count* of 256 and no test in the workspace creates a 257th outline, so its eviction branch
//! has never executed. This one's does, in `tests/the_mesh_cache_holds_its_budget.rs`, and the proof
//! is a probe a source grep cannot see.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use crate::tessellate::Mesh;

/// How many bytes of bookkeeping one entry costs beyond its mesh and its key.
///
/// A hash-map slot, a b-tree node's share, an [`Arc`]'s two counters and the entry record itself.
/// An estimate, and deliberately generous: a budget that undercounted its own overhead would be a
/// budget the allocator does not honour, and `tests/the_mesh_cache_holds_its_budget.rs` measures the
/// real figure against this one with the counting allocator.
const ENTRY_OVERHEAD_BYTES: usize = 128;

/// Everything that decides what triangles a path becomes, as words.
///
/// Built by [`crate::tessellate`], opaque here: this module compares keys and never interprets them.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct MeshKey {
    words: Box<[u32]>,
}

impl MeshKey {
    /// The key those words name.
    pub(crate) fn new(words: Vec<u32>) -> Self {
        Self {
            words: words.into_boxed_slice(),
        }
    }

    /// How many bytes the key itself occupies.
    fn byte_len(&self) -> usize {
        self.words.len() * size_of::<u32>()
    }
}

/// One cached tessellation.
#[derive(Debug)]
struct Entry {
    mesh: Arc<Mesh>,
    bytes: usize,
    last_used: u64,
}

/// Triangles kept per `(path, style, scale bucket)` under a byte budget, least recently used first
/// out.
#[derive(Debug)]
pub struct MeshCache {
    budget: usize,
    bytes: usize,
    clock: u64,
    entries: HashMap<Arc<MeshKey>, Entry>,
    /// Every live entry by the clock reading of its last use, which is unique because the clock only
    /// ever goes up. The first key of this map is the least recently used entry, so eviction is a
    /// `pop_first` and never a scan.
    order: BTreeMap<u64, Arc<MeshKey>>,
    hits: u64,
    misses: u64,
    evictions: u64,
    oversized: u64,
}

impl MeshCache {
    /// A cache that will hold at most `budget` bytes of triangles, keys and bookkeeping.
    ///
    /// A budget of zero is legal and means *never cache*: every lookup misses, every insertion is
    /// counted oversized, and the tessellator still answers. That is the honest reading of "no
    /// memory for this", and it is what makes the budget's lower bound testable — a cache that
    /// silently kept one entry anyway would satisfy no bound at all.
    #[must_use]
    pub fn new(budget: usize) -> Self {
        Self {
            budget,
            bytes: 0,
            clock: 0,
            entries: HashMap::new(),
            order: BTreeMap::new(),
            hits: 0,
            misses: 0,
            evictions: 0,
            oversized: 0,
        }
    }

    /// The ceiling this cache holds itself to.
    #[must_use]
    pub fn budget(&self) -> usize {
        self.budget
    }

    /// How many bytes it is holding — meshes, keys and per-entry overhead together.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// How many tessellations it is holding.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether it is holding none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many lookups were answered from it.
    #[must_use]
    pub fn hits(&self) -> u64 {
        self.hits
    }

    /// How many lookups were not.
    #[must_use]
    pub fn misses(&self) -> u64 {
        self.misses
    }

    /// How many entries have been evicted to stay inside the budget.
    ///
    /// The figure a test asserts is **not zero** before it trusts the budget: a cache whose
    /// eviction path has never run is a cache whose byte bound is held by an accident of the
    /// workload.
    #[must_use]
    pub fn evictions(&self) -> u64 {
        self.evictions
    }

    /// How many tessellations were too large for the whole budget and were therefore never cached.
    #[must_use]
    pub fn oversized(&self) -> u64 {
        self.oversized
    }

    /// Forget everything.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.bytes = 0;
    }

    /// The triangles for `key`, if they are held, marking the entry as the most recently used.
    pub(crate) fn get(&mut self, key: &MeshKey) -> Option<Arc<Mesh>> {
        self.clock += 1;
        let clock = self.clock;
        let Some(entry) = self.entries.get_mut(key) else {
            self.misses += 1;
            return None;
        };
        let previous = entry.last_used;
        entry.last_used = clock;
        let mesh = Arc::clone(&entry.mesh);
        if let Some(owned) = self.order.remove(&previous) {
            self.order.insert(clock, owned);
        }
        self.hits += 1;
        Some(mesh)
    }

    /// Hold `mesh` under `key`, evicting the least recently used entries until it fits.
    ///
    /// An entry whose own size exceeds the whole budget is **not** admitted: admitting it would
    /// empty the cache to make room for something the cache cannot keep, which is the one behaviour
    /// a byte bound asserted from above cannot distinguish from working correctly.
    pub(crate) fn insert(&mut self, key: MeshKey, mesh: &Arc<Mesh>) {
        let cost = mesh.byte_len() + key.byte_len() + ENTRY_OVERHEAD_BYTES;
        if cost > self.budget {
            self.oversized += 1;
            return;
        }
        while self.bytes + cost > self.budget {
            if !self.evict_least_recently_used() {
                return;
            }
        }
        self.clock += 1;
        let clock = self.clock;
        let key = Arc::new(key);
        if let Some(displaced) = self.entries.insert(
            Arc::clone(&key),
            Entry {
                mesh: Arc::clone(mesh),
                bytes: cost,
                last_used: clock,
            },
        ) {
            // The same key inserted twice — two threads of work that both missed. The older entry's
            // bytes and its place in the order both go.
            self.bytes = self.bytes.saturating_sub(displaced.bytes);
            self.order.remove(&displaced.last_used);
        }
        self.order.insert(clock, key);
        self.bytes += cost;
    }

    /// Drop the least recently used entry. `false` when there was nothing left to drop.
    fn evict_least_recently_used(&mut self) -> bool {
        let Some((_, key)) = self.order.pop_first() else {
            return false;
        };
        if let Some(entry) = self.entries.remove(&key) {
            self.bytes = self.bytes.saturating_sub(entry.bytes);
            self.evictions += 1;
        }
        true
    }
}
