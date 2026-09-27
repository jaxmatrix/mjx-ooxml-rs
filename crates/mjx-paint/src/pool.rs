//! The budgeted texture pool every effect pass draws into.
//!
//! # Why an effect needs a pool at all
//!
//! A shadow, a glow, a soft edge, a reflection and a blur are all *the subtree, rendered somewhere
//! else and then brought back*. A page with thirty shadowed shapes therefore wants thirty temporary
//! render targets a frame, and creating and destroying a GPU texture is one of the more expensive
//! things a driver does — on some it is a synchronising operation. A pool turns thirty allocations
//! a frame into approximately zero, because the sizes recur: the same shapes are shadowed on the
//! next frame at the same zoom.
//!
//! Unbounded, that pool is a memory leak with good intentions. So it carries a byte budget.
//!
//! # What the budget bounds, and what it deliberately does not
//!
//! [`TexturePool::retained_bytes`] — the textures the pool is **holding for reuse** — is bounded by
//! [`TexturePool::budget`], always, in every state the pool can be in.
//! [`TexturePool::in_flight_bytes`] — the textures the frame currently has handed out — is
//! **reported and not bounded**, and that is a decision rather than an oversight: a frame that
//! genuinely needs three targets at once needs them, and a pool that refused the third would not
//! save memory, it would fail to draw a shadow. What a pool can honestly promise is that it does not
//! *hoard*, and that is what is asserted.
//!
//! # The trap this file is written against
//!
//! **A pool that keeps nothing satisfies every byte bound perfectly.** Audit pass 5 found exactly
//! that shape in `mjx-text`'s outline cache, whose eviction branch has never executed because no
//! test creates a 257th outline; MJXOFF-163 says in as many words not to copy that pattern. So
//! `tests/the_texture_pool_holds_its_budget.rs` asserts the budget **from below as well as from
//! above**, keeps a witness entry the wrong eviction policy has to destroy, and the eviction branch
//! was shown to execute with an `std::process::abort()` at the top of
//! `TexturePool::evict_least_recently_used` — a probe a source grep cannot see, and one that a
//! passing assertion cannot be mistaken for.
//!
//! # Why it is generic over the texture
//!
//! `TexturePool<T>` knows nothing about `wgpu`. The GPU painter instantiates it at
//! `TexturePool<wgpu::Texture>`; the budget suite instantiates it at a serial-numbered stand-in and
//! runs on a machine with no graphics stack at all. A pool that could only be exercised on a GPU
//! would be a pool whose budget CI could not check, which is the failure this whole child is
//! written against.

use crate::error::PaintError;

/// How big a render target is, and how many samples deep.
///
/// Every target this pool holds is 8-bit `RGBA`, because every one of them is a colour attachment an
/// effect pass reads back as a texture. A multisampled one costs its sample count in memory, which
/// is why the count is part of the size rather than beside it: a 4x MSAA target and a single-sampled
/// one of the same dimensions are not interchangeable and must not collide in the free list.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TextureSize {
    /// Width in device pixels.
    pub width: u32,
    /// Height in device pixels.
    pub height: u32,
    /// How many samples per pixel. `1` for a resolved target, `4` for the multisampled one.
    pub samples: u32,
}

/// How many bytes one pixel of a pooled target takes: 8-bit `RGBA`.
pub const BYTES_PER_PIXEL: usize = 4;

impl TextureSize {
    /// A single-sampled target.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            samples: 1,
        }
    }

    /// The same dimensions at `samples` samples per pixel.
    #[must_use]
    pub const fn multisampled(width: u32, height: u32, samples: u32) -> Self {
        Self {
            width,
            height,
            samples,
        }
    }

    /// How many bytes of storage this target costs.
    ///
    /// Saturating, because the product of three `u32`s does not fit a `usize` on a 32-bit host and a
    /// wrapped answer would report a gigantic target as a small one — which is the one arithmetic
    /// mistake a byte budget cannot survive.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(self.samples.max(1) as usize)
            .saturating_mul(BYTES_PER_PIXEL)
    }
}

/// A borrow of one pooled texture.
///
/// Carries a generation as well as a slot so that a handle kept past its
/// [`TexturePool::release`] answers [`PaintError::StaleTexture`] rather than someone else's
/// texture. A pool that reissued slots without generations would hand a shadow pass the blur pass's
/// pixels, and the picture would merely look wrong.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PoolHandle {
    slot: u32,
    generation: u32,
}

impl PoolHandle {
    /// Which slot of the pool.
    #[must_use]
    pub const fn slot(self) -> u32 {
        self.slot
    }

    /// Which issue of that slot.
    #[must_use]
    pub const fn generation(self) -> u32 {
        self.generation
    }
}

/// What the pool has been doing.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct PoolStatistics {
    /// Acquisitions answered from a retained texture.
    pub hits: usize,
    /// Acquisitions that had to create one.
    pub misses: usize,
    /// Textures dropped to stay inside the budget.
    pub evictions: usize,
    /// Textures released that were larger than the whole budget, and so were dropped rather than
    /// retained. Retaining one would put the pool permanently over its own bound.
    pub oversized: usize,
    /// The largest [`TexturePool::retained_bytes`] ever reached.
    pub peak_retained_bytes: usize,
    /// The largest [`TexturePool::in_flight_bytes`] ever reached — the frame's real working set,
    /// which the budget does not bound and a caller may want to know about.
    pub peak_in_flight_bytes: usize,
}

/// One slot's contents.
enum Slot<T> {
    /// Never filled, or emptied by an eviction. Reusable for a new texture.
    Empty,
    /// Held for reuse, and evictable.
    Retained {
        texture: T,
        size: TextureSize,
        last_used: u64,
    },
    /// Handed out. Not evictable: something is drawing into it.
    InFlight { texture: T, size: TextureSize },
}

/// A pool of render targets held under a byte budget.
pub struct TexturePool<T> {
    slots: Vec<Slot<T>>,
    /// The generation each slot is currently issuing. Parallel to `slots` so that an emptied slot
    /// still remembers how many times it has been reissued.
    generations: Vec<u32>,
    budget: usize,
    retained_bytes: usize,
    in_flight_bytes: usize,
    clock: u64,
    statistics: PoolStatistics,
}

impl<T> core::fmt::Debug for TexturePool<T> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("TexturePool")
            .field("budget", &self.budget)
            .field("retained_bytes", &self.retained_bytes)
            .field("in_flight_bytes", &self.in_flight_bytes)
            .field("slots", &self.slots.len())
            .field("statistics", &self.statistics)
            .finish()
    }
}

/// What a pool is given when nobody said otherwise: sixteen megabytes, the same figure
/// `mjx_scene::DEFAULT_MESH_CACHE_BYTES` uses, and for the same reason — it is a few full-page
/// targets at a desktop resolution, which is the working set a page of effects actually has.
pub const DEFAULT_TEXTURE_POOL_BYTES: usize = 16 * 1024 * 1024;

impl<T> Default for TexturePool<T> {
    fn default() -> Self {
        Self::with_budget(DEFAULT_TEXTURE_POOL_BYTES)
    }
}

impl<T> TexturePool<T> {
    /// A pool that will hold at most `budget` bytes of textures for reuse.
    #[must_use]
    pub fn with_budget(budget: usize) -> Self {
        Self {
            slots: Vec::new(),
            generations: Vec::new(),
            budget,
            retained_bytes: 0,
            in_flight_bytes: 0,
            clock: 0,
            statistics: PoolStatistics::default(),
        }
    }

    /// The bound on [`TexturePool::retained_bytes`].
    #[must_use]
    pub const fn budget(&self) -> usize {
        self.budget
    }

    /// How many bytes of texture the pool is holding for reuse. Never above [`TexturePool::budget`].
    #[must_use]
    pub const fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }

    /// How many bytes of texture the caller currently has handed out. Reported, not bounded.
    #[must_use]
    pub const fn in_flight_bytes(&self) -> usize {
        self.in_flight_bytes
    }

    /// Everything the pool is keeping alive, in flight and retained together.
    #[must_use]
    pub const fn resident_bytes(&self) -> usize {
        self.retained_bytes + self.in_flight_bytes
    }

    /// How many textures are held for reuse.
    #[must_use]
    pub fn retained_len(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| matches!(slot, Slot::Retained { .. }))
            .count()
    }

    /// How many are handed out.
    #[must_use]
    pub fn in_flight_len(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| matches!(slot, Slot::InFlight { .. }))
            .count()
    }

    /// What the pool has been doing.
    #[must_use]
    pub const fn statistics(&self) -> PoolStatistics {
        self.statistics
    }

    /// Take a target of exactly `size`, reusing a retained one if there is one and calling `create`
    /// if there is not.
    ///
    /// Exactly, not "at least": a pass that rendered into a larger target and read back the whole of
    /// it would sample the previous pass's pixels around the edges, and a pool that quietly widened
    /// a request would make that a rendering bug rather than an allocation.
    ///
    /// # Errors
    ///
    /// Whatever `create` fails with.
    pub fn acquire<F>(&mut self, size: TextureSize, create: F) -> Result<PoolHandle, PaintError>
    where
        F: FnOnce(TextureSize) -> Result<T, PaintError>,
    {
        let bytes = size.byte_len();

        // A retained target of exactly this size, if there is one.
        let reusable = self
            .slots
            .iter()
            .position(|slot| matches!(slot, Slot::Retained { size: held, .. } if *held == size));
        if let Some(index) = reusable {
            let Some(slot) = self.slots.get_mut(index) else {
                // `position` answered with an index into this very vector, so this arm is not
                // reachable; it is written as a fallthrough rather than an `unwrap` because a
                // painter may not panic and the compiler cannot know that.
                return self.create_into_new_slot(size, create);
            };
            if let Slot::Retained { texture, size, .. } = core::mem::replace(slot, Slot::Empty) {
                *slot = Slot::InFlight { texture, size };
                self.retained_bytes = self.retained_bytes.saturating_sub(bytes);
                self.in_flight_bytes = self.in_flight_bytes.saturating_add(bytes);
                self.statistics.peak_in_flight_bytes = self
                    .statistics
                    .peak_in_flight_bytes
                    .max(self.in_flight_bytes);
                self.statistics.hits += 1;
                let generation = self.generations.get(index).copied().unwrap_or_default();
                let slot_index = u32::try_from(index).unwrap_or(u32::MAX);
                return Ok(PoolHandle {
                    slot: slot_index,
                    generation,
                });
            }
            // The slot was not what `position` said it was, which cannot happen; leave it empty and
            // fall through to creating one rather than losing the acquisition.
        }

        self.create_into_new_slot(size, create)
    }

    /// Create a texture and put it in the first empty slot, or a new one.
    fn create_into_new_slot<F>(
        &mut self,
        size: TextureSize,
        create: F,
    ) -> Result<PoolHandle, PaintError>
    where
        F: FnOnce(TextureSize) -> Result<T, PaintError>,
    {
        let texture = create(size)?;
        let bytes = size.byte_len();
        let index = match self
            .slots
            .iter()
            .position(|slot| matches!(slot, Slot::Empty))
        {
            Some(index) => {
                if let Some(slot) = self.slots.get_mut(index) {
                    *slot = Slot::InFlight { texture, size };
                }
                index
            }
            None => {
                self.slots.push(Slot::InFlight { texture, size });
                self.generations.push(0);
                self.slots.len() - 1
            }
        };
        self.in_flight_bytes = self.in_flight_bytes.saturating_add(bytes);
        self.statistics.peak_in_flight_bytes = self
            .statistics
            .peak_in_flight_bytes
            .max(self.in_flight_bytes);
        self.statistics.misses += 1;
        let generation = self.generations.get(index).copied().unwrap_or_default();
        Ok(PoolHandle {
            slot: u32::try_from(index).unwrap_or(u32::MAX),
            generation,
        })
    }

    /// The texture behind a handle.
    ///
    /// # Errors
    ///
    /// [`PaintError::StaleTexture`] if the handle has been released, or if the pool has reissued
    /// its slot.
    pub fn texture(&self, handle: PoolHandle) -> Result<&T, PaintError> {
        let index = handle.slot as usize;
        let stale = || PaintError::StaleTexture {
            slot: handle.slot,
            generation: handle.generation,
        };
        if self.generations.get(index).copied() != Some(handle.generation) {
            return Err(stale());
        }
        match self.slots.get(index) {
            Some(Slot::InFlight { texture, .. }) => Ok(texture),
            _ => Err(stale()),
        }
    }

    /// The texture behind a handle, to draw into.
    ///
    /// A GPU painter never needs this — it renders into a texture *view* and the pool's copy is
    /// never touched — but a **software** painter's target is an ordinary buffer, and drawing into
    /// it is a mutable borrow. The alternative would be for the software painter to keep its own
    /// pool, which would put two byte budgets and two eviction policies in one crate.
    ///
    /// # Errors
    ///
    /// [`PaintError::StaleTexture`], on the same terms as [`TexturePool::texture`].
    pub fn texture_mut(&mut self, handle: PoolHandle) -> Result<&mut T, PaintError> {
        let index = handle.slot as usize;
        let stale = || PaintError::StaleTexture {
            slot: handle.slot,
            generation: handle.generation,
        };
        if self.generations.get(index).copied() != Some(handle.generation) {
            return Err(stale());
        }
        match self.slots.get_mut(index) {
            Some(Slot::InFlight { texture, .. }) => Ok(texture),
            _ => Err(stale()),
        }
    }

    /// The size a handle was acquired at.
    ///
    /// # Errors
    ///
    /// [`PaintError::StaleTexture`], on the same terms as [`TexturePool::texture`].
    pub fn size_of(&self, handle: PoolHandle) -> Result<TextureSize, PaintError> {
        let index = handle.slot as usize;
        let stale = || PaintError::StaleTexture {
            slot: handle.slot,
            generation: handle.generation,
        };
        if self.generations.get(index).copied() != Some(handle.generation) {
            return Err(stale());
        }
        match self.slots.get(index) {
            Some(Slot::InFlight { size, .. }) => Ok(*size),
            _ => Err(stale()),
        }
    }

    /// Give a target back, so the next pass of this frame or the next frame can have it.
    ///
    /// Releasing is where the budget is enforced, because it is the only moment the pool decides to
    /// *keep* anything.
    ///
    /// # Errors
    ///
    /// [`PaintError::StaleTexture`] if the handle was already released or its slot reissued.
    pub fn release(&mut self, handle: PoolHandle) -> Result<(), PaintError> {
        let index = handle.slot as usize;
        let stale = || PaintError::StaleTexture {
            slot: handle.slot,
            generation: handle.generation,
        };
        if self.generations.get(index).copied() != Some(handle.generation) {
            return Err(stale());
        }
        let Some(slot) = self.slots.get_mut(index) else {
            return Err(stale());
        };
        let (texture, size) = match core::mem::replace(slot, Slot::Empty) {
            Slot::InFlight { texture, size } => (texture, size),
            other => {
                *slot = other;
                return Err(stale());
            }
        };
        let bytes = size.byte_len();
        self.in_flight_bytes = self.in_flight_bytes.saturating_sub(bytes);
        // **The handle retires here, not when the slot is next handed out.** A generation bumped
        // only on eviction would let a handle that was released and whose slot was then reacquired
        // name the new target: the shadow pass would be handed the blur pass's pixels, and the
        // picture would merely look wrong. `the_texture_pool_holds_its_budget.rs` case seven is
        // what found that, and it is the only assertion that can see it.
        self.bump_generation(index);

        // A single target larger than the whole budget can never be retained: keeping it would put
        // the pool permanently over its own bound, and a bound that one entry can break is not one.
        // It is dropped here, and counted, so that a caller asking for a page-sized target on a
        // pool sized for thumbnails learns why nothing is ever reused.
        if bytes > self.budget {
            drop(texture);
            self.statistics.oversized += 1;
            return Ok(());
        }

        while self.retained_bytes.saturating_add(bytes) > self.budget {
            if !self.evict_least_recently_used() {
                break;
            }
        }

        // The eviction loop above cannot leave room only if nothing was retained, in which case
        // `bytes <= budget` already holds. Keeping the check is what makes that reasoning checkable
        // rather than merely believed.
        if self.retained_bytes.saturating_add(bytes) > self.budget {
            drop(texture);
            self.statistics.oversized += 1;
            return Ok(());
        }

        let last_used = self.clock;
        self.clock = self.clock.saturating_add(1);
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = Slot::Retained {
                texture,
                size,
                last_used,
            };
        }
        self.retained_bytes = self.retained_bytes.saturating_add(bytes);
        self.statistics.peak_retained_bytes =
            self.statistics.peak_retained_bytes.max(self.retained_bytes);
        Ok(())
    }

    /// Drop the retained texture that has gone longest without being handed out.
    ///
    /// Least recently **used**, where "used" is the moment it was last given back — which is the
    /// moment it last finished being drawn into. A pool that evicted in creation order would hold a
    /// byte budget just as well and would throw away the target the current page keeps asking for.
    ///
    /// Answers whether anything was evictable. In-flight targets never are: something is drawing
    /// into them.
    fn evict_least_recently_used(&mut self) -> bool {
        let mut oldest: Option<(usize, u64)> = None;
        for (index, slot) in self.slots.iter().enumerate() {
            if let Slot::Retained { last_used, .. } = slot {
                if oldest.is_none_or(|(_, seen)| *last_used < seen) {
                    oldest = Some((index, *last_used));
                }
            }
        }
        let Some((index, _)) = oldest else {
            return false;
        };
        let Some(slot) = self.slots.get_mut(index) else {
            return false;
        };
        if let Slot::Retained { texture, size, .. } = core::mem::replace(slot, Slot::Empty) {
            self.retained_bytes = self.retained_bytes.saturating_sub(size.byte_len());
            drop(texture);
        }
        self.bump_generation(index);
        self.statistics.evictions += 1;
        true
    }

    /// Retire every handle that named `index`.
    fn bump_generation(&mut self, index: usize) {
        if let Some(generation) = self.generations.get_mut(index) {
            *generation = generation.wrapping_add(1);
        }
    }

    /// Drop every retained texture, keeping the budget and the statistics.
    ///
    /// What a painter does when the device is lost, when the window is resized past every retained
    /// size, or when the application goes to the background. In-flight targets are left alone: a
    /// frame in progress is still drawing into them.
    pub fn clear(&mut self) {
        for index in 0..self.slots.len() {
            let is_retained = matches!(self.slots.get(index), Some(Slot::Retained { .. }));
            if !is_retained {
                continue;
            }
            if let Some(slot) = self.slots.get_mut(index) {
                *slot = Slot::Empty;
            }
            self.bump_generation(index);
        }
        self.retained_bytes = 0;
    }
}
