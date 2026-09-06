//! The tessellation cache's byte budget, asserted **in both directions**, with the eviction path
//! proved to run.
//!
//! # Why this is a target of its own, with no harness
//!
//! A `#[global_allocator]` is installed for a whole process and `cargo test` runs a harness's cases
//! on several threads at once, so a measurement taken inside one case would be measuring the others
//! as well. `harness = false` in `Cargo.toml` gives this file a plain `main` and the cases run one
//! after another — the pattern `crates/mjx-sml/tests/cell_store_allocation.rs` established, for the
//! same reason.
//!
//! # The trap, and why R04's version of this cache does not defeat it
//!
//! **A cache that evicts everything satisfies every byte bound perfectly.** So a budget asserted
//! only from above is not asserted at all, and a budget whose eviction path never runs is a budget
//! held by the workload rather than by the code.
//!
//! `mjx-text`'s outline cache is the nearest cache in this workspace and it has neither guard: it is
//! bounded by a *count* of [`DEFAULT_OUTLINE_CACHE_CAPACITY`](mjx_text::DEFAULT_OUTLINE_CACHE_CAPACITY)
//! — 256 — and no test in the workspace creates a 257th outline, so `evict_oldest_outlines` has
//! never executed. That is recorded here rather than in a comment nobody reads, because this file is
//! what stops the same thing being true of *this* cache.
//!
//! What is asserted below, in order:
//!
//! 1. after a workload many times the budget, the cache holds **at most** the budget;
//! 2. and **more than half** of it, over more than one entry — it did not empty itself;
//! 3. and [`MeshCache::evictions`] is **not zero**, so the eviction path really ran;
//! 4. and the figure it reports is the figure the allocator actually handed out, measured
//!    differentially against the same workload through a cache of no bytes at all.
//!
//! # Proved by mutation
//!
//! * Making `insert` admit an oversized entry (evicting until the cache is empty) → case two fails,
//!   naming a cache that holds one entry it cannot afford.
//! * Making `evict_least_recently_used` a no-op → case one fails on the budget.
//! * Making `insert` return before touching the cache → case two's *lower* bound fails, which is the
//!   direction a one-sided assertion cannot see.
//! * An `std::process::abort()` at the top of `evict_least_recently_used` aborts this binary, which
//!   is how the eviction path was shown to execute rather than assumed to. A `panic!` there would
//!   not have been evidence: this file's own assertions would have caught it either way.

use std::sync::Arc;

use mjx_scene::{
    FillRule, Geometry, Mesh, PathCommand, PlaceholderGeometry, ScenePoint, SceneRect,
    TessellationOptions, Tessellator,
};
use mjx_text::ScaleBucket;

#[global_allocator]
static ALLOCATOR: mjx_allocation_counter::Counting = mjx_allocation_counter::Counting;

/// The budget the cases below hold the cache to.
const BUDGET: usize = 32 * 1024;

/// How many distinct paths the workload tessellates. Each is a few hundred bytes of triangles, so
/// the workload is many times [`BUDGET`] and eviction is not optional.
const WORKLOAD: usize = 400;

fn main() {
    the_budget_is_held_from_above_without_the_cache_emptying_itself();
    an_entry_larger_than_the_whole_budget_is_never_admitted();
    a_cache_of_no_bytes_caches_nothing_and_still_answers();
    the_reported_bytes_are_the_bytes_the_allocator_handed_out();
    a_second_ask_for_the_same_path_is_the_very_same_triangles();
    a_different_scale_bucket_is_a_different_entry();
    clearing_forgets_the_meshes_and_keeps_the_budget();
    println!("the mesh cache holds its budget: 7 cases passed");
}

// -------------------------------------------------------------------------------------------
// The workload
// -------------------------------------------------------------------------------------------

fn options() -> TessellationOptions {
    TessellationOptions::for_bucket(ScaleBucket::from_steps(8))
}

/// The `which`th of [`WORKLOAD`] distinct star polygons.
///
/// Distinct in their **coordinates**, not merely in a label, because the cache's key is the path
/// itself: a workload of one path under `n` names would exercise nothing.
fn a_distinct_path(which: usize) -> Geometry {
    let points = 5 + which % 7;
    let radius = 8.0 + (which % 23) as f32;
    let mut commands = Vec::with_capacity(points * 2 + 2);
    for step in 0..points * 2 {
        let angle = std::f32::consts::PI * step as f32 / points as f32;
        let reach = if step % 2 == 0 { radius } else { radius / 2.5 };
        let at = ScenePoint::new(
            50.0 + reach * angle.cos() + which as f32 * 0.001,
            50.0 + reach * angle.sin(),
        );
        commands.push(if step == 0 {
            PathCommand::MoveTo(at)
        } else {
            PathCommand::LineTo(at)
        });
    }
    commands.push(PathCommand::Close);
    Geometry::path(commands, FillRule::NonZero)
}

/// Tessellate every path of the workload through `tessellator`, dropping each mesh at once so that
/// nothing but the cache holds one.
fn run_the_workload(tessellator: &mut Tessellator) {
    for which in 0..WORKLOAD {
        let mesh = tessellator
            .fill(
                &a_distinct_path(which),
                &PlaceholderGeometry::new(),
                options(),
            )
            .expect("a star polygon tessellates");
        assert!(!mesh.is_empty(), "path {which} became no triangles");
    }
}

#[track_caller]
fn check(claim: bool, message: String) {
    assert!(claim, "{message}");
}

// -------------------------------------------------------------------------------------------
// 1 — both directions, and the eviction path
// -------------------------------------------------------------------------------------------

fn the_budget_is_held_from_above_without_the_cache_emptying_itself() {
    let mut tessellator = Tessellator::with_budget(BUDGET);
    run_the_workload(&mut tessellator);
    let cache = tessellator.cache();

    check(
        cache.budget() == BUDGET,
        format!("the budget changed to {}", cache.budget()),
    );
    // From above.
    check(
        cache.bytes() <= BUDGET,
        format!(
            "the cache holds {} bytes against a budget of {BUDGET}",
            cache.bytes()
        ),
    );
    // From below — the direction a cache that threw everything away would pass silently.
    check(
        cache.bytes() > BUDGET / 2,
        format!(
            "the cache holds only {} of its {BUDGET} bytes after {WORKLOAD} distinct paths, which \
             is a cache emptying itself rather than one making room",
            cache.bytes()
        ),
    );
    check(
        cache.len() > 1,
        format!("the cache holds {} entries", cache.len()),
    );
    // And the eviction path really ran, rather than the workload happening to fit.
    check(
        cache.evictions() > 0,
        "nothing was ever evicted, so the byte bound above is held by the workload and not by the \
         cache"
            .to_owned(),
    );
    check(
        cache.misses() == WORKLOAD as u64 && cache.hits() == 0,
        format!(
            "{} hits and {} misses over {WORKLOAD} distinct paths",
            cache.hits(),
            cache.misses()
        ),
    );
    check(
        cache.oversized() == 0,
        format!("{} entries were refused as oversized", cache.oversized()),
    );

    // The most recently tessellated path is still there: least recently used out, not most.
    let before = tessellator.cache().hits();
    let _ = tessellator
        .fill(
            &a_distinct_path(WORKLOAD - 1),
            &PlaceholderGeometry::new(),
            options(),
        )
        .expect("the last path tessellates again");
    check(
        tessellator.cache().hits() == before + 1,
        "the path tessellated last was not the one still held".to_owned(),
    );
}

// -------------------------------------------------------------------------------------------
// 2 — an entry the cache cannot afford is refused rather than admitted
// -------------------------------------------------------------------------------------------

fn an_entry_larger_than_the_whole_budget_is_never_admitted() {
    // A placeholder is a frame and a cross: sixty triangles and over a kilobyte of mesh.
    let big = Geometry::Unresolved {
        outline: 7,
        bounds: SceneRect::new(0.0, 0.0, 400.0, 300.0),
    };
    let measured = Tessellator::new()
        .fill(&big, &PlaceholderGeometry::new(), options())
        .expect("the placeholder tessellates")
        .byte_len();
    check(
        measured > 512,
        format!("the placeholder is only {measured} bytes, so this case proves nothing"),
    );

    let mut tessellator = Tessellator::with_budget(512);
    let _ = tessellator
        .fill(&big, &PlaceholderGeometry::new(), options())
        .expect("it still tessellates");
    let cache = tessellator.cache();
    check(
        cache.oversized() == 1,
        format!("{} entries were refused, not one", cache.oversized()),
    );
    check(
        cache.is_empty() && cache.bytes() == 0,
        format!(
            "the cache admitted an entry it cannot afford: {} entries, {} bytes",
            cache.len(),
            cache.bytes()
        ),
    );
    check(
        cache.evictions() == 0,
        "the cache evicted something to make room for an entry it then could not keep".to_owned(),
    );

    // And it still caches what it *can* afford, so refusing the giant did not break it.
    let small = Geometry::Rectangle(SceneRect::new(0.0, 0.0, 4.0, 4.0));
    let _ = tessellator
        .fill(&small, &PlaceholderGeometry::new(), options())
        .expect("a rectangle tessellates");
    check(
        tessellator.cache().len() == 1 && tessellator.cache().bytes() > 0,
        format!(
            "after refusing an oversized entry the cache holds {} entries",
            tessellator.cache().len()
        ),
    );
}

// -------------------------------------------------------------------------------------------
// 3 — no budget at all
// -------------------------------------------------------------------------------------------

fn a_cache_of_no_bytes_caches_nothing_and_still_answers() {
    let mut tessellator = Tessellator::with_budget(0);
    run_the_workload(&mut tessellator);
    let cache = tessellator.cache();
    check(
        cache.bytes() == 0 && cache.is_empty(),
        format!("a cache of no bytes holds {} of them", cache.bytes()),
    );
    check(
        cache.oversized() == WORKLOAD as u64,
        format!(
            "{} of {WORKLOAD} tessellations were refused",
            cache.oversized()
        ),
    );
    check(
        cache.evictions() == 0,
        format!(
            "{} evictions from a cache that never held anything",
            cache.evictions()
        ),
    );
}

// -------------------------------------------------------------------------------------------
// 4 — the reported figure is the real one
// -------------------------------------------------------------------------------------------

fn the_reported_bytes_are_the_bytes_the_allocator_handed_out() {
    // Measured **differentially**: the same workload through a cache of no bytes and through a
    // cache of `BUDGET`, with both tessellators still alive at the reading. Everything that is not
    // the cache — lyon's own working buffers, the paths, the transient meshes — is in both figures
    // and cancels.
    let start = mjx_allocation_counter::live();
    let mut uncached = Tessellator::with_budget(0);
    run_the_workload(&mut uncached);
    let without = mjx_allocation_counter::live().saturating_sub(start);

    let start = mjx_allocation_counter::live();
    let mut cached = Tessellator::with_budget(BUDGET);
    run_the_workload(&mut cached);
    let with = mjx_allocation_counter::live().saturating_sub(start);

    let reported = cached.cache().bytes();
    let real = with.saturating_sub(without);
    check(
        reported > 0 && real > 0,
        format!("nothing was measured: {reported} reported, {real} live"),
    );
    // The reported figure charges a flat per-entry overhead against a real one that is a hash map's
    // slot, a b-tree node's share and two `Arc` counters, so the two are the same figure and not
    // the same number. A factor of two either way is the honest window; the assertion that matters
    // is that they track at all, because a cache reporting a tenth of what it holds is a budget the
    // allocator does not honour.
    check(
        real <= reported * 2 && reported <= real * 2,
        format!(
            "the cache reports {reported} bytes and the allocator says it is holding {real} \
             (uncached workload {without}, cached workload {with})"
        ),
    );
    check(
        real <= BUDGET * 2,
        format!("a cache budgeted at {BUDGET} bytes is really holding {real}"),
    );
    drop(uncached);
    drop(cached);
}

// -------------------------------------------------------------------------------------------
// 5, 6, 7 — what the key means, and clearing
// -------------------------------------------------------------------------------------------

fn a_second_ask_for_the_same_path_is_the_very_same_triangles() {
    let mut tessellator = Tessellator::with_budget(BUDGET);
    let path = a_distinct_path(3);
    let first: Arc<Mesh> = tessellator
        .fill(&path, &PlaceholderGeometry::new(), options())
        .expect("a star tessellates");
    let second: Arc<Mesh> = tessellator
        .fill(&path, &PlaceholderGeometry::new(), options())
        .expect("and is remembered");
    check(
        Arc::ptr_eq(&first, &second),
        "the same path at the same scale was tessellated twice".to_owned(),
    );
    check(
        tessellator.cache().hits() == 1 && tessellator.cache().misses() == 1,
        format!(
            "{} hits and {} misses over one path asked for twice",
            tessellator.cache().hits(),
            tessellator.cache().misses()
        ),
    );
}

fn a_different_scale_bucket_is_a_different_entry() {
    let mut tessellator = Tessellator::with_budget(BUDGET);
    let path = a_distinct_path(4);
    let near = tessellator
        .fill(
            &path,
            &PlaceholderGeometry::new(),
            TessellationOptions::for_bucket(ScaleBucket::from_steps(8)),
        )
        .expect("at one bucket");
    let far = tessellator
        .fill(
            &path,
            &PlaceholderGeometry::new(),
            TessellationOptions::for_bucket(ScaleBucket::from_steps(64)),
        )
        .expect("and at another");
    check(
        !Arc::ptr_eq(&near, &far),
        "two scale buckets shared one cache entry, so a painter that magnifies a mesh magnifies \
         its flattening error with it"
            .to_owned(),
    );
    check(
        tessellator.cache().len() == 2 && tessellator.cache().hits() == 0,
        format!("the cache holds {} entries", tessellator.cache().len()),
    );
}

fn clearing_forgets_the_meshes_and_keeps_the_budget() {
    let mut tessellator = Tessellator::with_budget(BUDGET);
    run_the_workload(&mut tessellator);
    check(
        !tessellator.cache().is_empty(),
        "nothing was cached to clear".to_owned(),
    );
    tessellator.clear_cache();
    let cache = tessellator.cache();
    check(
        cache.is_empty() && cache.bytes() == 0 && cache.budget() == BUDGET,
        format!(
            "after clearing: {} entries, {} bytes, budget {}",
            cache.len(),
            cache.bytes(),
            cache.budget()
        ),
    );
}
