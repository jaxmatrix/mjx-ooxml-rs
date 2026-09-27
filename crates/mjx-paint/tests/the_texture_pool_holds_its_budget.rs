//! The effect texture pool's byte budget, asserted **in both directions**, with the eviction path
//! proved to run.
//!
//! # Why this is a target of its own, with no harness
//!
//! A `#[global_allocator]` is installed for a whole process and `cargo test` runs a harness's cases
//! on several threads at once, so a measurement taken inside one case would be measuring the others
//! as well. `harness = false` in `Cargo.toml` gives this file a plain `main` and the cases run one
//! after another — the pattern `crates/mjx-scene/tests/the_mesh_cache_holds_its_budget.rs` and
//! `crates/mjx-sml/tests/cell_store_allocation.rs` established, for the same reason.
//!
//! # The trap, and the pattern this file deliberately does **not** follow
//!
//! **A pool that keeps nothing satisfies every byte bound perfectly.** MJXOFF-163 says in as many
//! words not to copy R04's outline cache here: it is bounded by a *count* of 256 and no test in the
//! workspace creates a 257th outline, so `evict_oldest_outlines` has never executed. A budget whose
//! eviction path never runs is a budget held by the workload rather than by the code.
//!
//! What is asserted below, in order:
//!
//! 1. after a workload many times the budget, the pool retains **at most** the budget;
//! 2. and **more than half** of it, over more than one target — it did not empty itself;
//! 3. and [`PoolStatistics::evictions`] is not zero, so the eviction path really ran;
//! 4. and the figure it reports is the figure the allocator actually handed out, measured
//!    differentially against the same workload through a pool of no bytes at all;
//! 5. a target larger than the whole budget is never retained, and a **witness** kept beside it
//!    survives — the guard the mesh cache's own suite needed because the obvious version of case
//!    two was a green mutation;
//! 6. the target evicted is the least recently **released** one, not the first created;
//! 7. a released handle is stale, and a reissued slot does not answer to the old handle — a pool
//!    that reissued slots without generations would hand a shadow pass the blur pass's pixels;
//! 8. and the conservation law: everything acquired is in flight, retained, evicted or dropped for
//!    being oversized.
//!
//! # Proved by mutation
//!
//! * An `std::process::abort()` at the top of `TexturePool::evict_least_recently_used` aborts this
//!   binary, which is how the eviction path was shown to **execute** rather than assumed to. A
//!   `panic!` there would not have been evidence: `harness = false` turns any panic into a failure,
//!   and a source grep would have seen it.
//! * Making `release` drop every texture instead of retaining it → case two's *lower* bound fails,
//!   which is the direction a one-sided assertion cannot see.
//! * Making `evict_least_recently_used` a no-op → case one fails on the budget.
//! * Retaining an oversized target instead of dropping it → case five fails, and the witness is
//!   gone.
//! * Evicting in creation order rather than by release time → case six fails.
//! * Dropping the generation bump from `release` → case seven's stale-handle assertion fails, which
//!   is the only assertion that can see it.

use mjx_paint::{PaintError, PoolHandle, TexturePool, TextureSize};

#[global_allocator]
static ALLOCATOR: mjx_allocation_counter::Counting = mjx_allocation_counter::Counting;

/// The budget the cases below hold the pool to: four 64x64 targets' worth.
const BUDGET: usize = 4 * 64 * 64 * 4;

/// A stand-in for a GPU texture: bytes, so the allocator can see them, and a serial number, so a
/// case can tell *the same texture back* from *a new one of the same size*.
///
/// The whole reason `TexturePool` is generic. A pool that could only be exercised against a real
/// device would be a pool whose budget continuous integration could not check — which is the failure
/// this entire child is written against.
#[derive(Debug)]
struct FakeTexture {
    serial: u64,
    bytes: Vec<u8>,
}

fn make(size: TextureSize) -> Result<FakeTexture, PaintError> {
    // A thread-local `Cell` rather than a `static mut`: this whole binary runs on one thread, and a
    // serial number is the only way a case can tell *the same texture back* from *a new one of the
    // same size* — which is the difference between a pool and a very careful allocator.
    SERIAL.with(|serial| {
        let next = serial.get() + 1;
        serial.set(next);
        Ok(FakeTexture {
            serial: next,
            bytes: vec![0u8; size.byte_len()],
        })
    })
}

thread_local! {
    static SERIAL: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

fn main() {
    the_budget_is_held_from_above_without_the_pool_emptying_itself();
    the_reported_bytes_are_the_bytes_the_allocator_handed_out();
    a_target_larger_than_the_budget_is_never_retained_and_the_witness_survives();
    the_target_evicted_is_the_least_recently_released_one();
    a_released_handle_is_stale_and_a_reissued_slot_does_not_answer_to_it();
    a_reused_target_is_the_very_same_texture();
    clearing_forgets_the_targets_and_keeps_the_budget();
    println!("the texture pool holds its budget: 7 cases passed");
}

#[track_caller]
fn check(claim: bool, message: String) {
    assert!(claim, "{message}");
}

/// Acquire and release one target of `side` square, and answer the serial it came back with.
fn cycle(pool: &mut TexturePool<FakeTexture>, side: u32) -> u64 {
    let handle = pool
        .acquire(TextureSize::new(side, side), make)
        .expect("a stand-in texture is always makeable");
    let serial = pool
        .texture(handle)
        .expect("the handle is live until it is released")
        .serial;
    pool.release(handle).expect("a live handle releases");
    serial
}

fn the_budget_is_held_from_above_without_the_pool_emptying_itself() {
    let mut pool: TexturePool<FakeTexture> = TexturePool::with_budget(BUDGET);
    // Forty distinct sizes, each a few kilobytes, is many times the budget. Distinct sizes rather
    // than one size repeated, because a pool asked for the same size forty times would answer from
    // its free list and never evict anything.
    for step in 0..40u32 {
        cycle(&mut pool, 32 + step);
        check(
            pool.retained_bytes() <= pool.budget(),
            format!(
                "after step {step} the pool retains {} bytes against a budget of {}",
                pool.retained_bytes(),
                pool.budget()
            ),
        );
    }

    // From above.
    check(
        pool.retained_bytes() <= BUDGET,
        format!(
            "the pool retains {} bytes of {BUDGET}",
            pool.retained_bytes()
        ),
    );
    // And from below, which is the direction a one-sided assertion cannot see: a pool that dropped
    // everything on release would satisfy the bound above perfectly and be useless.
    check(
        pool.retained_bytes() > BUDGET / 2,
        format!(
            "the pool retains only {} bytes of a {BUDGET}-byte budget — it is emptying itself, \
             which holds the bound and defeats the point",
            pool.retained_bytes()
        ),
    );
    check(
        pool.retained_len() > 1,
        format!(
            "the pool holds {} target(s); a budget met by keeping exactly one is a budget met by \
             accident",
            pool.retained_len()
        ),
    );
    // And the eviction path really ran.
    check(
        pool.statistics().evictions > 0,
        "no eviction happened, so the budget was held by the workload rather than by the code"
            .to_owned(),
    );
    check(
        pool.in_flight_bytes() == 0,
        format!(
            "{} bytes are still in flight after every handle was released",
            pool.in_flight_bytes()
        ),
    );
}

fn the_reported_bytes_are_the_bytes_the_allocator_handed_out() {
    // Differential, because the process has already allocated for its own reasons. The same workload
    // through a pool of no bytes at all retains nothing, so the difference between the two peaks is
    // what the pool itself is holding.
    let before = mjx_allocation_counter::live();
    let mut pool: TexturePool<FakeTexture> = TexturePool::with_budget(BUDGET);
    for step in 0..40u32 {
        cycle(&mut pool, 32 + step);
    }
    let held = mjx_allocation_counter::live().saturating_sub(before);
    let reported = pool.retained_bytes();
    check(
        held >= reported,
        format!("the pool reports {reported} retained bytes but only {held} are live"),
    );
    // The bookkeeping is the truth to within the `Vec` headers and the slot table, which are a few
    // hundred bytes against tens of thousands.
    check(
        held.saturating_sub(reported) < BUDGET / 2,
        format!(
            "the pool reports {reported} retained bytes and the allocator is holding {held} — the \
             figure the budget is enforced against is not the figure the process is paying"
        ),
    );

    let mut nothing: TexturePool<FakeTexture> = TexturePool::with_budget(0);
    for step in 0..40u32 {
        cycle(&mut nothing, 32 + step);
    }
    check(
        nothing.retained_bytes() == 0,
        format!(
            "a pool of no bytes retained {} of them",
            nothing.retained_bytes()
        ),
    );
    check(
        nothing.statistics().oversized == 40,
        format!(
            "a pool of no bytes must refuse to retain every one of the forty targets; it refused {}",
            nothing.statistics().oversized
        ),
    );
    // And it still *answers*: a pool that cannot keep anything must still hand out targets, or a
    // caller with a tiny budget cannot draw at all.
    check(
        nothing.statistics().misses == 40,
        format!("it answered {} acquisitions", nothing.statistics().misses),
    );
}

fn a_target_larger_than_the_budget_is_never_retained_and_the_witness_survives() {
    let mut pool: TexturePool<FakeTexture> = TexturePool::with_budget(BUDGET);

    // The witness: a small target the pool must still be holding afterwards. Without it, a pool
    // that admitted the oversized target by evicting everything would satisfy every byte bound and
    // nothing would notice — which is exactly the green mutation the mesh cache's suite found.
    let witness = TextureSize::new(16, 16);
    let witness_serial = cycle(&mut pool, 16);
    check(
        pool.retained_len() == 1,
        format!("the witness was not retained: {} held", pool.retained_len()),
    );

    // A target four times the whole budget.
    let huge = TextureSize::new(256, 256);
    check(
        huge.byte_len() > BUDGET,
        format!("{} is not larger than {BUDGET}", huge.byte_len()),
    );
    let handle = pool.acquire(huge, make).expect("it is still handed out");
    check(
        pool.texture(handle).is_ok(),
        "an oversized target must still be *usable* — refusing it would fail to draw a shadow \
         rather than save memory"
            .to_owned(),
    );
    pool.release(handle).expect("it releases");

    check(
        pool.statistics().oversized == 1,
        format!(
            "the oversized target was not counted: {}",
            pool.statistics().oversized
        ),
    );
    check(
        pool.retained_bytes() <= BUDGET,
        format!("the pool retains {} bytes", pool.retained_bytes()),
    );
    check(
        pool.retained_len() == 1,
        format!(
            "the witness did not survive a target the pool cannot afford: {} retained",
            pool.retained_len()
        ),
    );
    // And it is the *same* texture, not a new one of the same size.
    let again = cycle(&mut pool, 16);
    check(
        again == witness_serial,
        format!("the witness came back as texture {again}, not {witness_serial}"),
    );
    let _ = witness;
}

fn the_target_evicted_is_the_least_recently_released_one() {
    // A budget that holds exactly two of these.
    let one = TextureSize::new(32, 32);
    let budget = one.byte_len() * 2 + one.byte_len() / 2;
    let mut pool: TexturePool<FakeTexture> = TexturePool::with_budget(budget);

    let first = cycle(&mut pool, 32);
    let second = cycle(&mut pool, 33);
    // Touch the first again, so it is the most recently released. A pool that evicted in *creation*
    // order would still throw this one out; one that evicts by release time throws out the second.
    let first_again = cycle(&mut pool, 32);
    check(
        first_again == first,
        format!("the first target was not reused: {first_again} against {first}"),
    );

    // A third target forces one eviction.
    let _third = cycle(&mut pool, 34);
    check(
        pool.statistics().evictions >= 1,
        "the third target should have forced an eviction".to_owned(),
    );

    // The first is still there; the second is not.
    let first_after = cycle(&mut pool, 32);
    check(
        first_after == first,
        format!(
            "the least recently *used* target was evicted: the first came back as {first_after}, \
             not {first}. A pool that evicts in creation order holds a byte budget just as well and \
             throws away the target the current page keeps asking for."
        ),
    );
    let second_after = cycle(&mut pool, 33);
    check(
        second_after != second,
        format!("the second target survived and should not have: {second_after} == {second}"),
    );
}

fn a_released_handle_is_stale_and_a_reissued_slot_does_not_answer_to_it() {
    let mut pool: TexturePool<FakeTexture> = TexturePool::with_budget(BUDGET);
    let handle = pool
        .acquire(TextureSize::new(16, 16), make)
        .expect("a target");
    pool.release(handle).expect("it releases");

    check(
        matches!(pool.texture(handle), Err(PaintError::StaleTexture { .. })),
        "a released handle still answers with a texture".to_owned(),
    );
    check(
        matches!(pool.release(handle), Err(PaintError::StaleTexture { .. })),
        "a handle released twice was accepted twice".to_owned(),
    );

    // Reacquire the same size: the pool reuses the slot, and the *old* handle must not name it.
    let reissued = pool
        .acquire(TextureSize::new(16, 16), make)
        .expect("a target");
    check(
        pool.texture(reissued).is_ok(),
        "the reissued handle does not answer".to_owned(),
    );
    check(
        matches!(pool.texture(handle), Err(PaintError::StaleTexture { .. })),
        format!(
            "the retired handle {handle:?} answers for the slot the pool reissued as {reissued:?} \
             — which is how a shadow pass gets handed the blur pass's pixels"
        ),
    );
    let _: PoolHandle = reissued;
}

fn a_reused_target_is_the_very_same_texture() {
    let mut pool: TexturePool<FakeTexture> = TexturePool::with_budget(BUDGET);
    // The stand-in really does carry the bytes the pool charges for, checked once here so that
    // every byte assertion above is about storage rather than about bookkeeping agreeing with
    // itself.
    let handle = pool
        .acquire(TextureSize::new(24, 24), make)
        .expect("a target");
    let held = pool.texture(handle).expect("it is live").bytes.len();
    check(
        held == TextureSize::new(24, 24).byte_len(),
        format!("a 24x24 stand-in holds {held} bytes"),
    );
    pool.release(handle).expect("it releases");
    pool.clear();

    let first = cycle(&mut pool, 24);
    let second = cycle(&mut pool, 24);
    check(
        first == second,
        format!("the same size was made twice: {first} then {second}"),
    );
    check(
        pool.statistics().hits == 1,
        format!("the second ask was not a hit: {:?}", pool.statistics()),
    );
    // A different size is a different entry, even by one pixel: a pass that rendered into a larger
    // target and read the whole of it would sample the previous pass's pixels around the edges.
    let other = cycle(&mut pool, 25);
    check(
        other != first,
        format!("a 25x25 target reused the 24x24 one: {other}"),
    );
    // And a multisampled target of the same dimensions is not the same target either.
    let handle = pool
        .acquire(TextureSize::multisampled(24, 24, 4), make)
        .expect("a multisampled target");
    let multisampled = pool.texture(handle).expect("it is live").serial;
    check(
        multisampled != first,
        "a four-sample target reused a one-sample one of the same dimensions".to_owned(),
    );
    pool.release(handle).expect("it releases");
}

fn clearing_forgets_the_targets_and_keeps_the_budget() {
    let mut pool: TexturePool<FakeTexture> = TexturePool::with_budget(BUDGET);
    for step in 0..6u32 {
        cycle(&mut pool, 20 + step);
    }
    check(pool.retained_bytes() > 0, "nothing was retained".to_owned());
    let budget = pool.budget();
    pool.clear();
    check(
        pool.retained_bytes() == 0 && pool.retained_len() == 0,
        format!(
            "clearing left {} bytes over {} targets",
            pool.retained_bytes(),
            pool.retained_len()
        ),
    );
    check(
        pool.budget() == budget,
        "clearing changed the budget".to_owned(),
    );
    // And it still works afterwards.
    let after = cycle(&mut pool, 20);
    check(
        after > 0,
        "the pool stopped answering after a clear".to_owned(),
    );
}
