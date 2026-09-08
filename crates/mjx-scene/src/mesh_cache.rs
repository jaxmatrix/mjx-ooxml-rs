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
//!
//! # Where the mechanism lives, since MJXOFF-168
//!
//! The three bullets above are now [`mjx_ooxml_core::ByteBudgetCache`]'s and not this module's.
//! `mjx-session` (rank 3.5) and `mjx-view` (rank 3.8) both needed the same discipline, and neither
//! can reach rank 1.7's private copy of it — an upward edge is what `xtask/tests/layering.rs`
//! refuses — so the choice was one implementation at the floor of the workspace or three that
//! drift. What is left here is the part that is genuinely about *triangles*: what a mesh key is,
//! and what a tessellation costs.

use std::sync::Arc;

use mjx_ooxml_core::cache::ByteBudgetCache;

use crate::tessellate::Mesh;

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

/// Triangles kept per `(path, style, scale bucket)` under a byte budget, least recently used first
/// out.
#[derive(Debug)]
pub struct MeshCache {
    entries: ByteBudgetCache<MeshKey, Arc<Mesh>>,
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
            entries: ByteBudgetCache::new(budget),
        }
    }

    /// The ceiling this cache holds itself to.
    #[must_use]
    pub fn budget(&self) -> usize {
        self.entries.budget()
    }

    /// How many bytes it is holding — meshes, keys and per-entry overhead together.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.entries.bytes()
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
        self.entries.stats().hits
    }

    /// How many lookups were not.
    #[must_use]
    pub fn misses(&self) -> u64 {
        self.entries.stats().misses
    }

    /// How many entries have been evicted to stay inside the budget.
    ///
    /// The figure a test asserts is **not zero** before it trusts the budget: a cache whose
    /// eviction path has never run is a cache whose byte bound is held by an accident of the
    /// workload.
    #[must_use]
    pub fn evictions(&self) -> u64 {
        self.entries.stats().evictions
    }

    /// How many tessellations were too large for the whole budget and were therefore never cached.
    #[must_use]
    pub fn oversized(&self) -> u64 {
        self.entries.stats().rejections
    }

    /// Forget everything.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// The triangles for `key`, if they are held, marking the entry as the most recently used.
    pub(crate) fn get(&mut self, key: &MeshKey) -> Option<Arc<Mesh>> {
        self.entries.get(key).map(Arc::clone)
    }

    /// Hold `mesh` under `key`, evicting the least recently used entries until it fits.
    ///
    /// An entry whose own size exceeds the whole budget is **not** admitted: admitting it would
    /// empty the cache to make room for something the cache cannot keep, which is the one behaviour
    /// a byte bound asserted from above cannot distinguish from working correctly.
    pub(crate) fn insert(&mut self, key: MeshKey, mesh: &Arc<Mesh>) {
        let cost = mesh.byte_len() + key.byte_len();
        // The rejection is counted by the cache itself, and read back through
        // [`oversized`](Self::oversized). Nothing here pins, so `Rejection::PinnedFloor` is
        // unreachable and every rejection is an entry larger than the whole budget.
        let _ = self.entries.insert(key, Arc::clone(mesh), cost);
    }
}
