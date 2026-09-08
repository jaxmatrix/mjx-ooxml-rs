//! The seven cache stages, their declared ceilings, and what a viewport reports about them.
//!
//! # Seven stages, three of them here, and the other four named rather than faked
//!
//! `docs/UI_PLATFORM_PLAN.md` §4 L6 lists seven caches — checkpoints, fragments, display lists,
//! tessellations, the glyph atlas, image decodes and effect textures. [`Stage`] has all seven,
//! because a budget table with three rows would quietly redefine the problem, and
//! [`CacheBudget`] carries a ceiling for each.
//!
//! **Only three of them are this crate's to hold**, and saying which is the point of the table
//! rather than a caveat on it:
//!
//! | Stage | Owner | Why |
//! |---|---|---|
//! | [`Stage::Checkpoints`] | this crate | kept for **every** page, never evicted — see below |
//! | [`Stage::Fragments`] | this crate | a `PageFragments` per windowed page |
//! | [`Stage::DisplayLists`] | this crate | a `DisplayList` per windowed page |
//! | [`Stage::Tessellations`] | `mjx-scene` | `MeshCache`, keyed by `(path, style, scale bucket)`, and a painter's to construct |
//! | [`Stage::GlyphAtlas`] | `mjx-text` / `mjx-scene` | an atlas is a texture, and its residency is the painter's |
//! | [`Stage::ImageDecodes`] | `mjx-paint` | a decoded image is a GPU upload away from being a texture |
//! | [`Stage::EffectTextures`] | `mjx-paint` | an effect's render target only exists on a surface |
//!
//! The last four are all above or beside this crate's own rank, and a viewport that constructed them
//! would be a viewport that had to know what a device is. What this module can honestly do is carry
//! their **declared ceilings** in one table, so the shell that builds a `MeshCache` and an atlas
//! reads its numbers from the same place the viewport reads its own, instead of from four
//! constants in four crates. [`CacheBudget::ceiling`] is that reading.
//!
//! # Checkpoints are kept for every page and fragments are not, and that asymmetry is the design
//!
//! A [`Checkpoint`](mjx_layout::Checkpoint) is bounded at
//! [`MAXIMUM_CHECKPOINT_BYTES`](mjx_layout::MAXIMUM_CHECKPOINT_BYTES) — one kibibyte — so four
//! hundred of them is four hundred kibibytes, and keeping all of them is what turns *"scroll to page
//! 300"* from a re-layout of three hundred pages into a re-layout of one. A `PageFragments` is
//! three orders of magnitude larger and there is no reason to keep one for a page nobody is looking
//! at. That is why [`Stage::Checkpoints`]'s ceiling is a **reported bound rather than an evicting
//! budget**: the viewport asserts it stays under the figure and never drops a checkpoint to do so.

/// One stage of the pipeline that holds something expensive.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Stage {
    /// Resume points, one per laid-out page. Kept for every page; never evicted.
    Checkpoints,
    /// `PageFragments` — the tree and the spatial index — for the windowed pages.
    Fragments,
    /// `DisplayList`s for the windowed pages.
    DisplayLists,
    /// Triangles per `(path, style, scale bucket)`. `mjx-scene`'s `MeshCache`.
    Tessellations,
    /// Rasterised glyphs, packed into a texture.
    GlyphAtlas,
    /// Decoded raster images, before upload.
    ImageDecodes,
    /// Render targets for blur, shadow and reflection.
    EffectTextures,
}

impl Stage {
    /// Every stage, in the order `UI_PLATFORM_PLAN.md` §4 L6 lists them.
    pub const ALL: [Self; 7] = [
        Self::Checkpoints,
        Self::Fragments,
        Self::DisplayLists,
        Self::Tessellations,
        Self::GlyphAtlas,
        Self::ImageDecodes,
        Self::EffectTextures,
    ];

    /// Whether a [`DocumentView`](crate::DocumentView) holds this stage itself.
    ///
    /// The four that answer `false` are named in the module documentation together with the crate
    /// that owns each. This is not a gap being hidden: a budget is carried for every stage so that
    /// the shell reads one table, and a viewport reports on the three it can measure.
    #[must_use]
    pub const fn is_held_by_the_viewport(self) -> bool {
        matches!(
            self,
            Self::Checkpoints | Self::Fragments | Self::DisplayLists
        )
    }

    /// A short name, for a report a person reads.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Checkpoints => "checkpoints",
            Self::Fragments => "fragments",
            Self::DisplayLists => "display lists",
            Self::Tessellations => "tessellations",
            Self::GlyphAtlas => "glyph atlas",
            Self::ImageDecodes => "image decodes",
            Self::EffectTextures => "effect textures",
        }
    }
}

/// A ceiling for every stage, in bytes.
///
/// # Where the defaults come from
///
/// `docs/UI_PLATFORM_PLAN.md` §12 gives the whole client **400 MB on the desktop and 200 MB on
/// mobile** for a four-hundred-page document, and **256 MB / 96 MB of GPU texture** beside it.
/// [`CacheBudget::desktop`] and [`CacheBudget::mobile`] divide those two figures across the seven
/// stages; [`CacheBudget::total`] adds them back up, and
/// `tests/the_declared_budgets_add_up.rs` asserts the sums against §12 so that moving one row
/// without moving the others fails rather than quietly overspending.
///
/// A budget is a **policy**, not a measurement, and it is a plain public struct for that reason: a
/// caller that knows its device sets its own numbers, and a test sets numbers small enough that
/// eviction has to happen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CacheBudget {
    /// Bytes of resume points. A **reported** bound: checkpoints are never evicted.
    pub checkpoints: usize,
    /// Bytes of `PageFragments`.
    pub fragments: usize,
    /// Bytes of `DisplayList`.
    pub display_lists: usize,
    /// Bytes of triangles. `mjx-scene`'s `MeshCache` is constructed with this.
    pub tessellations: usize,
    /// Bytes of glyph atlas.
    pub glyph_atlas: usize,
    /// Bytes of decoded images.
    pub image_decodes: usize,
    /// Bytes of effect render targets.
    pub effect_textures: usize,
}

impl CacheBudget {
    /// The desktop division of §12's 400 MB resident and 256 MB of texture.
    ///
    /// The first three rows spend the *resident* figure and the last four the *texture* one; they
    /// are separate budgets because they are separate memories, and
    /// `tests/the_declared_budgets_add_up.rs` adds each up against §12 rather than trusting that
    /// whoever moved one row remembered the others. It caught this table's first draft, which spent
    /// 352 MB of a 256 MB texture budget.
    #[must_use]
    pub const fn desktop() -> Self {
        Self {
            checkpoints: 4 * MIB,
            fragments: 224 * MIB,
            display_lists: 128 * MIB,
            tessellations: 32 * MIB,
            glyph_atlas: 32 * MIB,
            image_decodes: 64 * MIB,
            effect_textures: 128 * MIB,
        }
    }

    /// The mobile division of §12's 200 MB resident and 96 MB of texture.
    #[must_use]
    pub const fn mobile() -> Self {
        Self {
            checkpoints: 2 * MIB,
            fragments: 112 * MIB,
            display_lists: 64 * MIB,
            tessellations: 16 * MIB,
            glyph_atlas: 16 * MIB,
            image_decodes: 24 * MIB,
            effect_textures: 40 * MIB,
        }
    }

    /// Every stage at `bytes`.
    ///
    /// For a caller that wants one number, and for a test that wants a number small enough that
    /// eviction has to happen — which is the only kind of budget test that proves anything.
    #[must_use]
    pub const fn uniform(bytes: usize) -> Self {
        Self {
            checkpoints: bytes,
            fragments: bytes,
            display_lists: bytes,
            tessellations: bytes,
            glyph_atlas: bytes,
            image_decodes: bytes,
            effect_textures: bytes,
        }
    }

    /// Every stage at `usize::MAX` — **a budget nothing ever reaches**.
    ///
    /// It exists so that a memory gate can run the same workload twice and show that the bounded
    /// run's figure came from the bound. A cache with a budget so large it never evicts is
    /// indistinguishable from a cache with no budget at all, so the way to prove a windowing gate
    /// can fail is to disable eviction and watch it fail.
    ///
    /// **Never ship this.** `crates/mjx-view/tests/resident_memory.rs` is its one intended caller.
    #[must_use]
    pub const fn unbounded() -> Self {
        Self::uniform(usize::MAX)
    }

    /// The ceiling declared for `stage`.
    #[must_use]
    pub const fn ceiling(&self, stage: Stage) -> usize {
        match stage {
            Stage::Checkpoints => self.checkpoints,
            Stage::Fragments => self.fragments,
            Stage::DisplayLists => self.display_lists,
            Stage::Tessellations => self.tessellations,
            Stage::GlyphAtlas => self.glyph_atlas,
            Stage::ImageDecodes => self.image_decodes,
            Stage::EffectTextures => self.effect_textures,
        }
    }

    /// Every declared ceiling added up.
    ///
    /// Saturating, so [`unbounded`](Self::unbounded) reports `usize::MAX` rather than wrapping to a
    /// small number, which is the one arithmetic mistake that would turn the eviction-disabled
    /// control into a passing test.
    #[must_use]
    pub fn total(&self) -> usize {
        Stage::ALL.iter().fold(0usize, |sum, stage| {
            sum.saturating_add(self.ceiling(*stage))
        })
    }

    /// The ceilings for the three stages a viewport holds itself.
    #[must_use]
    pub fn viewport_total(&self) -> usize {
        Stage::ALL
            .iter()
            .filter(|stage| stage.is_held_by_the_viewport())
            .fold(0usize, |sum, stage| {
                sum.saturating_add(self.ceiling(*stage))
            })
    }
}

/// One mebibyte, so the tables above read as the figures §12 states.
const MIB: usize = 1024 * 1024;

/// What one stage is actually holding.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StageReport {
    /// Which stage.
    pub stage: Stage,
    /// Its declared ceiling.
    pub ceiling: usize,
    /// What it is holding now.
    pub bytes: usize,
    /// How many entries.
    pub entries: usize,
    /// How many entries have been evicted to stay inside the ceiling.
    ///
    /// **Zero is the figure that makes a byte bound meaningless**, because a cache whose eviction
    /// path never ran is holding its bound by an accident of the workload.
    pub evictions: u64,
    /// How many entries were dropped because an edit made them stale, rather than to save memory.
    pub suppressions: u64,
    /// Lookups answered from the stage.
    pub hits: u64,
    /// Lookups that were not.
    pub misses: u64,
}

impl StageReport {
    /// Whether the stage is inside its declared ceiling.
    #[must_use]
    pub const fn within_ceiling(&self) -> bool {
        self.bytes <= self.ceiling
    }
}

/// What every stage a viewport holds is doing.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CacheReport {
    stages: Vec<StageReport>,
}

impl CacheReport {
    /// Builds a report from the stages a viewport measured.
    #[must_use]
    pub fn new(stages: Vec<StageReport>) -> Self {
        Self { stages }
    }

    /// Every stage reported.
    #[must_use]
    pub fn stages(&self) -> &[StageReport] {
        &self.stages
    }

    /// One stage, or `None` when the viewport does not hold it.
    #[must_use]
    pub fn stage(&self, stage: Stage) -> Option<&StageReport> {
        self.stages.iter().find(|report| report.stage == stage)
    }

    /// Every byte the viewport is holding across its stages.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.stages
            .iter()
            .fold(0usize, |sum, report| sum.saturating_add(report.bytes))
    }

    /// Whether every stage is inside its declared ceiling.
    #[must_use]
    pub fn within_ceilings(&self) -> bool {
        self.stages.iter().all(StageReport::within_ceiling)
    }

    /// The first stage that is over its ceiling, or `None`.
    #[must_use]
    pub fn first_overrun(&self) -> Option<&StageReport> {
        self.stages.iter().find(|report| !report.within_ceiling())
    }
}
