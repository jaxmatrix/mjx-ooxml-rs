//! The shaped-run cache.
//!
//! Shaping is the most expensive thing this crate does — a table walk, a substitution pass and a
//! positioning pass over every glyph — and documents repeat text relentlessly: the same word in the
//! same face at the same size, the same empty table cell forty times, the same bullet on every line
//! of a list. A scroll re-shapes whatever came back into view. This cache is what makes that
//! affordable.
//!
//! # What the key has to hold
//!
//! `docs/UI_PLATFORM_PLAN.md` §4 names the key as `(font, size, features, text)`. That is not quite
//! enough to be *sound*, and the two additions are not optional:
//!
//! * **Direction.** The same string shaped right-to-left produces the glyphs in the other order,
//!   with the Arabic joining forms chosen differently. A hit across directions would draw a word
//!   backwards.
//! * **Script and language.** The script selects the whole shaping engine, and the language selects
//!   a language system inside the font. A hit across either would draw the wrong glyphs.
//!
//! Everything in the key is compared by value except the face, which is compared by **identity**:
//! two [`Arc<FontFace>`] that point at the same allocation are the same face, and two that do not
//! are treated as different even if their bytes agree. That is exact and cheap. It costs the cache a
//! strong reference to every face it holds an entry for — which is what keeps the pointer it hashes
//! valid, so the identity can never be a stale address a new face happened to be allocated at.
//!
//! # Eviction
//!
//! Least-recently-used, evicted in **batches**. A strict LRU needs an intrusive list to find the
//! oldest entry in constant time; a scan would be `O(n)` on every insertion past the capacity, which
//! is the worst possible shape for the case the cache exists for. Instead, when the map is full,
//! the oldest half is dropped in one pass: `O(n log n)` once per `n/2` insertions, so `O(log n)`
//! amortised each, with no intrusive structure and no `unsafe`.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::direction::TextDirection;
use crate::face::FontFace;
use crate::feature::FeatureSet;
use crate::script::TextScript;
use crate::shaping::{FontSize, ShapedGlyph, ShapingRequest};

/// How many shaped runs a cache holds before it starts evicting.
///
/// A page of dense text is a few hundred runs and a scroll keeps several pages live, so a few
/// thousand entries covers the working set of a document being read without the cache becoming a
/// second copy of it.
pub const DEFAULT_CACHE_CAPACITY: usize = 4096;

/// A face compared by identity rather than by content.
#[derive(Clone, Debug)]
struct FaceIdentityKey(Arc<FontFace>);

impl PartialEq for FaceIdentityKey {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for FaceIdentityKey {}

impl Hash for FaceIdentityKey {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        (Arc::as_ptr(&self.0) as usize).hash(hasher);
    }
}

/// Everything that decides what shaping a run produces.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct ShapedRunKey {
    face: FaceIdentityKey,
    size: FontSize,
    direction: TextDirection,
    script: TextScript,
    language: Option<Box<str>>,
    features: FeatureSet,
    text: Box<str>,
}

#[derive(Clone, Debug)]
struct Entry {
    glyphs: Arc<[ShapedGlyph]>,
    last_used: u64,
}

/// What the cache has been doing, for a diagnostic pane or a test.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct CacheStatistics {
    /// Runs answered from the cache.
    pub hits: u64,
    /// Runs that had to be shaped.
    pub misses: u64,
    /// Entries evicted to stay within capacity.
    pub evictions: u64,
    /// How many entries the cache holds now.
    pub entries: usize,
    /// How many it holds before evicting.
    pub capacity: usize,
}

impl CacheStatistics {
    /// The fraction of lookups the cache answered, or `None` when nothing has been looked up.
    #[must_use]
    pub fn hit_rate(&self) -> Option<f64> {
        let total = self.hits + self.misses;
        if total == 0 {
            return None;
        }
        // Both counts are non-negative and their sum is non-zero, so this is a well-defined ratio.
        #[allow(clippy::cast_precision_loss)]
        Some(self.hits as f64 / total as f64)
    }
}

/// Shaped runs, keyed by everything that decides their glyphs.
#[derive(Debug)]
pub struct ShapedRunCache {
    entries: HashMap<ShapedRunKey, Entry>,
    capacity: usize,
    clock: u64,
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl ShapedRunCache {
    /// A cache holding [`DEFAULT_CACHE_CAPACITY`] runs.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CACHE_CAPACITY)
    }

    /// A cache holding `capacity` runs. Zero disables caching entirely.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            capacity,
            clock: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// What the cache has been doing.
    #[must_use]
    pub fn statistics(&self) -> CacheStatistics {
        CacheStatistics {
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            entries: self.entries.len(),
            capacity: self.capacity,
        }
    }

    /// Forget every entry. The counters are kept: they describe the cache's life, not its contents.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// The glyphs for `request` in `face`, if they are held.
    ///
    /// Counts a hit or a miss either way, which is what makes the statistics a record of the
    /// caller's behaviour rather than of the cache's.
    pub fn get(
        &mut self,
        face: &Arc<FontFace>,
        request: &ShapingRequest<'_>,
    ) -> Option<Arc<[ShapedGlyph]>> {
        if self.capacity == 0 {
            self.misses += 1;
            return None;
        }
        let key = key_for(face, request);
        self.clock += 1;
        let clock = self.clock;
        match self.entries.get_mut(&key) {
            Some(entry) => {
                entry.last_used = clock;
                self.hits += 1;
                Some(Arc::clone(&entry.glyphs))
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }

    /// Remember `glyphs` as the shaping of `request` in `face`.
    pub fn insert(
        &mut self,
        face: &Arc<FontFace>,
        request: &ShapingRequest<'_>,
        glyphs: Arc<[ShapedGlyph]>,
    ) {
        if self.capacity == 0 {
            return;
        }
        if self.entries.len() >= self.capacity {
            self.evict_oldest_half();
        }
        self.clock += 1;
        self.entries.insert(
            key_for(face, request),
            Entry {
                glyphs,
                last_used: self.clock,
            },
        );
    }

    /// Drop the least recently used half of the entries.
    ///
    /// One `O(n log n)` pass per `n/2` insertions rather than an `O(n)` scan per insertion; see the
    /// module documentation.
    fn evict_oldest_half(&mut self) {
        let mut ages: Vec<u64> = self.entries.values().map(|entry| entry.last_used).collect();
        if ages.is_empty() {
            return;
        }
        ages.sort_unstable();
        // `ages` is non-empty, so the midpoint index is in range; `>= 1` keeps a capacity-1 cache
        // making progress instead of evicting nothing and growing past its bound.
        let cut = ages[ages.len() / 2];
        let before = self.entries.len();
        self.entries.retain(|_, entry| entry.last_used > cut);
        self.evictions += (before - self.entries.len()) as u64;
    }
}

impl Default for ShapedRunCache {
    fn default() -> Self {
        Self::new()
    }
}

fn key_for(face: &Arc<FontFace>, request: &ShapingRequest<'_>) -> ShapedRunKey {
    ShapedRunKey {
        face: FaceIdentityKey(Arc::clone(face)),
        size: request.size,
        direction: request.direction,
        script: request.script,
        language: request.language.map(Box::from),
        features: request.features.clone(),
        text: Box::from(request.text),
    }
}
