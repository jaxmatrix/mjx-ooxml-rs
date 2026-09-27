//! The tessellation cache's byte budget, asserted **in both directions**, with the eviction path
//! proved to run.
//!
//! MJX-STAND-IN: this crate is rank 1.7 and `mjx-geometry` is 2.5, so the real provider is an
//! upward edge `xtask/tests/layering.rs` refuses by name. The cache is keyed on the path and the
//! tolerance; the provider is present only because `fill` takes one.
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
//! 5. every field of a stroke's key is in the key, and no two dash patterns share one — **a key
//!    that collides hands a painter somebody else's shape**;
//! 6. the entry evicted is the least recently **used** one, not the least recently inserted;
//! 7. and the conservation law: everything admitted is either still held or was counted evicted.
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
//! * Dropping the scale bucket from the **stroke** key, dropping the width, and collapsing two dash
//!   tags onto one number were all green until cases seven and eight existed. The fill key had been
//!   gated for the same thing and the stroke key had not, because every stroke case used a fresh
//!   path or a fresh tessellator and no second ask ever reached a colliding key.
//! * Not moving an entry in the eviction order on a hit, and refreshing its clock without moving it,
//!   were both green until case nine used the two entries **alternately and finished on the older
//!   one**. A queue holds a byte budget perfectly well; what it loses is the page a reader is
//!   looking at.

use std::sync::Arc;

use mjx_scene::{
    CompoundStroke, DashPattern, FillRule, Geometry, LineCap, LineJoin, Mesh, PathCommand,
    PlaceholderGeometry, ScenePoint, SceneRect, StrokeAlignment, StrokeGeometry,
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
    a_stroke_at_a_different_scale_bucket_is_a_different_entry();
    every_dash_pattern_and_every_width_is_its_own_entry();
    the_entry_that_is_evicted_is_the_least_recently_used_one();
    clearing_forgets_the_meshes_and_keeps_the_budget();
    println!("the mesh cache holds its budget: 10 cases passed");
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

    // **The conservation law: every mesh admitted is either still held or was counted evicted.**
    //
    // Nothing about the byte budget can see the cache's *internal* bookkeeping go out of step — the
    // eviction order and each entry's own clock are two records of one fact, and desynchronising
    // them was a green mutation under every assertion above. It shows up here, because a stale key
    // in the eviction order drops an entry the eviction count never learns about, and the two sides
    // of this equation stop matching.
    let admitted = cache.misses() - cache.oversized();
    check(
        cache.len() as u64 + cache.evictions() == admitted,
        format!(
            "{} entries held plus {} evicted is not the {admitted} admitted; the cache's eviction \
             order and its entries disagree about what it is holding",
            cache.len(),
            cache.evictions()
        ),
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

    // **A small entry goes in first, and the case is what happens to it.** Without one, the
    // oversized entry arrives at an empty cache and *every* policy looks the same from outside: a
    // cache that refuses it and a cache that evicts its way down to nothing before giving up both
    // end holding zero bytes. That is a green mutation this file caught — removing the early return
    // in `insert` changed nothing any assertion could see — and the fix is a witness the wrong
    // policy would have to destroy.
    let mut tessellator = Tessellator::with_budget(512);
    let small = Geometry::Rectangle(SceneRect::new(0.0, 0.0, 4.0, 4.0));
    let _ = tessellator
        .fill(&small, &PlaceholderGeometry::new(), options())
        .expect("a rectangle tessellates");
    check(
        tessellator.cache().len() == 1,
        format!(
            "the witness was not cached: {} entries",
            tessellator.cache().len()
        ),
    );
    let held = tessellator.cache().bytes();

    let _ = tessellator
        .fill(&big, &PlaceholderGeometry::new(), options())
        .expect("the oversized entry still tessellates");
    let cache = tessellator.cache();
    check(
        cache.oversized() == 1,
        format!("{} entries were refused, not one", cache.oversized()),
    );
    check(
        cache.len() == 1 && cache.bytes() == held,
        format!(
            "the witness did not survive an entry the cache cannot afford: {} entries, {} bytes \
             against the {held} it held before",
            cache.len(),
            cache.bytes()
        ),
    );
    check(
        cache.evictions() == 0,
        "the cache evicted something to make room for an entry it then could not keep, which is \
         the one failure a byte bound asserted from above cannot tell from working correctly"
            .to_owned(),
    );

    // And the witness is still *the same triangles*, not merely the same byte count.
    let before = tessellator.cache().hits();
    let _ = tessellator
        .fill(&small, &PlaceholderGeometry::new(), options())
        .expect("the rectangle is still held");
    check(
        tessellator.cache().hits() == before + 1 && tessellator.cache().bytes() > 0,
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

/// A plain stroke, with everything geometric stated so a case can vary exactly one thing.
fn a_stroke(width: f32, dash: DashPattern) -> StrokeGeometry {
    StrokeGeometry {
        width,
        cap: LineCap::Flat,
        join: LineJoin::Bevel,
        dash,
        alignment: StrokeAlignment::Centered,
        compound: CompoundStroke::Single,
    }
}

/// A path long enough that every dash pattern cuts it differently.
fn a_long_line() -> Geometry {
    Geometry::path(
        vec![
            PathCommand::MoveTo(ScenePoint::new(0.0, 0.0)),
            PathCommand::LineTo(ScenePoint::new(400.0, 0.0)),
        ],
        FillRule::NonZero,
    )
}

fn a_stroke_at_a_different_scale_bucket_is_a_different_entry() {
    // The fill key was gated for this and **the stroke key was not**: dropping the bucket from
    // `stroke_key_words` was a green mutation, because every stroke in the suite was tessellated at
    // one bucket. A painter that magnifies a stroke magnifies its flattening error exactly as it
    // does a fill's.
    let mut tessellator = Tessellator::with_budget(BUDGET);
    let stroke = a_stroke(4.0, DashPattern::Solid);
    let near = tessellator
        .stroke(
            &a_long_line(),
            stroke,
            &PlaceholderGeometry::new(),
            TessellationOptions::for_bucket(ScaleBucket::from_steps(8)),
        )
        .expect("at one bucket");
    let far = tessellator
        .stroke(
            &a_long_line(),
            stroke,
            &PlaceholderGeometry::new(),
            TessellationOptions::for_bucket(ScaleBucket::from_steps(64)),
        )
        .expect("and at another");
    check(
        !Arc::ptr_eq(&near, &far),
        "two scale buckets shared one stroke cache entry".to_owned(),
    );
    check(
        tessellator.cache().len() == 2 && tessellator.cache().hits() == 0,
        format!(
            "the cache holds {} entries for two buckets",
            tessellator.cache().len()
        ),
    );
}

fn every_dash_pattern_and_every_width_is_its_own_entry() {
    // **A key that collides hands a painter somebody else's shape**, and the two fields most likely
    // to collide are the ones with the most values: eleven dash patterns and a continuous width.
    // Two dash tags mapping to one number, and the width dropped from the key, were both green
    // mutations — every earlier stroke case used a fresh path or a fresh tessellator, so no second
    // ask ever reached a colliding key.
    let dashes = [
        DashPattern::Solid,
        DashPattern::Dot,
        DashPattern::Dash,
        DashPattern::LargeDash,
        DashPattern::DashDot,
        DashPattern::LargeDashDot,
        DashPattern::LargeDashDotDot,
        DashPattern::SystemDash,
        DashPattern::SystemDot,
        DashPattern::SystemDashDot,
        DashPattern::SystemDashDotDot,
    ];
    let mut tessellator = Tessellator::with_budget(BUDGET);
    let mut buffers: Vec<(String, Vec<u8>)> = Vec::new();
    for dash in dashes {
        let mesh = tessellator
            .stroke(
                &a_long_line(),
                a_stroke(6.0, dash),
                &PlaceholderGeometry::new(),
                options(),
            )
            .expect("a dashed line strokes");
        check(!mesh.is_empty(), format!("{dash:?} produced no triangles"));
        buffers.push((format!("{dash:?}"), mesh.vertex_bytes()));
    }
    check(
        tessellator.cache().len() == dashes.len() && tessellator.cache().hits() == 0,
        format!(
            "eleven dash patterns produced {} cache entries and {} hits, so two share a key",
            tessellator.cache().len(),
            tessellator.cache().hits()
        ),
    );
    for (first, (name, bytes)) in buffers.iter().enumerate() {
        for (other, bytes_of_other) in buffers.iter().skip(first + 1) {
            check(
                bytes != bytes_of_other,
                format!("`{name}` and `{other}` produced the same triangles"),
            );
        }
    }

    // And the width, on one path through one tessellator, so a dropped width is a cache hit.
    let mut tessellator = Tessellator::with_budget(BUDGET);
    let thin = tessellator
        .stroke(
            &a_long_line(),
            a_stroke(1.0, DashPattern::Solid),
            &PlaceholderGeometry::new(),
            options(),
        )
        .expect("a thin line");
    let thick = tessellator
        .stroke(
            &a_long_line(),
            a_stroke(9.0, DashPattern::Solid),
            &PlaceholderGeometry::new(),
            options(),
        )
        .expect("a thick one");
    check(
        !Arc::ptr_eq(&thin, &thick) && tessellator.cache().hits() == 0,
        "two widths shared one cache entry, so a thin line would be drawn thick".to_owned(),
    );
    check(
        (thick.bounds().height() - 9.0).abs() < 0.001
            && (thin.bounds().height() - 1.0).abs() < 0.001,
        format!(
            "the widths reached the triangles as {} and {}",
            thin.bounds().height(),
            thick.bounds().height()
        ),
    );
}

fn the_entry_that_is_evicted_is_the_least_recently_used_one() {
    // **Least recently *used*, not least recently inserted.** Dropping the clock update in
    // `MeshCache::get` turns the cache into a queue, and every assertion about the byte budget stays
    // green — a queue holds its budget perfectly well. What it loses is the thing a cache is for:
    // the page held still on screen, re-asked for every frame, is the page that gets thrown away.
    let one = a_distinct_path(1);
    let two = a_distinct_path(2);
    let three = a_distinct_path(3);
    let sizes: Vec<usize> = [&one, &two, &three]
        .iter()
        .map(|path| {
            Tessellator::new()
                .fill(path, &PlaceholderGeometry::new(), options())
                .expect("a star tessellates")
                .byte_len()
        })
        .collect();
    // Room for two of the three and no more. The per-entry bookkeeping is charged as well, so the
    // allowance is two entries' overhead rather than a third entry's worth of anything.
    let budget = sizes.iter().take(2).sum::<usize>() + 2 * 400;
    let mut tessellator = Tessellator::with_budget(budget);

    let _ = tessellator.fill(&one, &PlaceholderGeometry::new(), options());
    let _ = tessellator.fill(&two, &PlaceholderGeometry::new(), options());
    check(
        tessellator.cache().len() == 2 && tessellator.cache().evictions() == 0,
        format!(
            "two entries did not both fit: {} held, {} evicted",
            tessellator.cache().len(),
            tessellator.cache().evictions()
        ),
    );

    // Now use them **alternately, and finish on the older one**, so that insertion order, first-use
    // order and last-use order all disagree.
    //
    // The middle re-use is not decoration. An implementation that promotes an entry on its *first*
    // re-use and then loses track of it — the entry's clock left stale while its key in the
    // eviction order moves — satisfies every simpler version of this case, and satisfies the byte
    // budget, and is still a cache that forgets the page a reader is looking at. Only a second
    // re-use of the same entry, after the other has been touched, can tell the two apart.
    let hits = tessellator.cache().hits();
    let _ = tessellator.fill(&one, &PlaceholderGeometry::new(), options());
    let _ = tessellator.fill(&two, &PlaceholderGeometry::new(), options());
    let _ = tessellator.fill(&one, &PlaceholderGeometry::new(), options());
    check(
        tessellator.cache().hits() == hits + 3,
        format!(
            "three re-uses produced {} hits; both paths were meant to still be held",
            tessellator.cache().hits() - hits
        ),
    );

    // A third entry has to displace exactly one, and it must be `two` — the one used longest ago,
    // which is neither the one inserted first nor the one re-used first.
    let _ = tessellator.fill(&three, &PlaceholderGeometry::new(), options());
    check(
        tessellator.cache().evictions() == 1,
        format!(
            "{} entries were evicted to make room for one",
            tessellator.cache().evictions()
        ),
    );
    let before = tessellator.cache().hits();
    let _ = tessellator.fill(&one, &PlaceholderGeometry::new(), options());
    check(
        tessellator.cache().hits() == before + 1,
        "the most recently used entry was evicted and a staler one kept — the cache is ordering by \
         something other than last use"
            .to_owned(),
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
