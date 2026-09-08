//! **A four-hundred-page document, walked end to end, under a measured memory ceiling** — and the
//! same walk with eviction disabled, which is the only thing that proves the first one measured
//! anything.
//!
//! # Why this is a binary with no test harness
//!
//! A `#[global_allocator]` is process-wide, so a figure measured in a binary that also runs other
//! cases measures the other cases; and `cargo test` runs a harness's cases on several threads at
//! once, so even one binary's own cases interleave. `mjx-sml`'s cell-store gate learned that first
//! and `mjx-session`'s journal gate learned it again — one of them read 92,376 bytes for a
//! zero-allocation assertion, every one of them another case's. One target, one `main`, one thread.
//!
//! # The two figures, and why both are needed
//!
//! A memory ceiling is trivially satisfied by a cache that holds nothing, and a budget so large it
//! never evicts is indistinguishable from no budget at all. So this binary runs the **identical**
//! walk twice:
//!
//! | run | budget | expected |
//! |---|---|---|
//! | bounded | [`WINDOW_BUDGET_BYTES`] per stage | live heap stays under [`CEILING_BYTES`], eviction ran, and the working set was kept |
//! | unbounded | [`CacheBudget::unbounded`] | live heap **exceeds** [`CEILING_BYTES`] by a wide margin |
//!
//! The second is the proof that the gate can fail. Without it, an implementation that materialised
//! nothing at all and one that windowed correctly would produce the same green.
//!
//! # About the absolute figures
//!
//! `docs/UI_PLATFORM_PLAN.md` §12 budgets a whole client at 400 MB desktop and 200 MB mobile for a
//! four-hundred-page document. That figure is for a **real** box model over a real document, with
//! glyph rasters, images and effect textures in it, and this crate cannot assert it without one —
//! `crates/mjx-view/src/budget.rs` says which four of the seven stages belong to other crates. What
//! *is* asserted here is the mechanism the §12 figure depends on, at a scale chosen so that whole-
//! document materialisation genuinely breaches the bound: the ratio between the two runs is the
//! measurement, and it is printed so a reader can see it rather than trust it.

use std::process::ExitCode;

use mjx_allocation_counter::Counting;
use mjx_layout::{LayoutSize, PageIndex};
use mjx_view::{CacheBudget, DocumentView, ManualFrameClock, RefinementTier, Stage, Viewport};

#[path = "support/mod.rs"]
mod support;

use support::{letter, FlowModel, Paragraphs, PlainScenes};

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// How long the document is. Four hundred, because that is the figure §12 is written about and
/// because a windowing gate on a document that fits inside its own budget proves nothing.
const PAGES: u32 = 400;

/// How many boxes are on each page. Chosen so that one page of fragments is tens of kilobytes and
/// four hundred of them are tens of megabytes — a separation wide enough that the two runs below
/// cannot be confused for measurement noise.
const BLOCKS_PER_PAGE: u32 = 300;

/// The per-stage ceiling for the bounded run.
const WINDOW_BUDGET_BYTES: usize = 2 * 1024 * 1024;

/// What the whole process may hold while walking the document under that budget.
///
/// Generous against the sum of the two evicting stages (4 MiB) because the process also holds the
/// checkpoint store, the scroll model's four hundred metrics, the box model, and the test binary
/// itself. Tight enough that whole-document materialisation cannot fit inside it — which the
/// unbounded run demonstrates rather than assumes.
const CEILING_BYTES: usize = 8 * 1024 * 1024;

/// One end-to-end walk, and what the heap looked like at its worst.
struct Walk {
    peak_bytes: usize,
    live_bytes: usize,
    pages_laid_out: u64,
    fragment_evictions: u64,
    fragments_held: usize,
}

fn main() -> ExitCode {
    let mut failures = Vec::new();

    let bounded = walk(CacheBudget::uniform(WINDOW_BUDGET_BYTES));
    println!(
        "bounded   ({WINDOW_BUDGET_BYTES} B/stage): peak {:>12} B, live {:>12} B, \
         {} pages laid out, {} fragment evictions, {} pages still held",
        bounded.peak_bytes,
        bounded.live_bytes,
        bounded.pages_laid_out,
        bounded.fragment_evictions,
        bounded.fragments_held,
    );

    let unbounded = walk(CacheBudget::unbounded());
    println!(
        "unbounded (eviction disabled): peak {:>12} B, live {:>12} B, \
         {} pages laid out, {} fragment evictions, {} pages still held",
        unbounded.peak_bytes,
        unbounded.live_bytes,
        unbounded.pages_laid_out,
        unbounded.fragment_evictions,
        unbounded.fragments_held,
    );
    println!(
        "ceiling {CEILING_BYTES} B — the bounded walk is {:.1}x smaller at its peak",
        unbounded.peak_bytes as f64 / bounded.peak_bytes.max(1) as f64,
    );

    // ---- The bound held. ------------------------------------------------------------------------
    check(
        &mut failures,
        bounded.peak_bytes <= CEILING_BYTES,
        format!(
            "the bounded walk peaked at {} bytes against a {CEILING_BYTES}-byte ceiling",
            bounded.peak_bytes
        ),
    );
    // ---- …and it held because of the bound, not because there was nothing to hold. --------------
    check(
        &mut failures,
        bounded.fragment_evictions > 0,
        "nothing was ever evicted, so the ceiling is held by an accident of the workload"
            .to_owned(),
    );
    check(
        &mut failures,
        unbounded.peak_bytes > CEILING_BYTES,
        format!(
            "with eviction disabled the walk still peaked at only {} bytes, under the \
             {CEILING_BYTES}-byte ceiling — this gate cannot fail and therefore proves nothing",
            unbounded.peak_bytes
        ),
    );
    check(
        &mut failures,
        unbounded.peak_bytes >= bounded.peak_bytes * 2,
        format!(
            "the unbounded walk peaked at {} bytes and the bounded one at {} — less than twice, \
             which is inside the noise a measurement like this carries",
            unbounded.peak_bytes, bounded.peak_bytes
        ),
    );
    // ---- The other side: the working set was kept. -----------------------------------------------
    check(
        &mut failures,
        bounded.fragments_held > 1,
        format!(
            "{} pages of fragments are held at the end of the walk — a cache that keeps nothing \
             satisfies every byte bound perfectly and is not a cache",
            bounded.fragments_held
        ),
    );
    // ---- And the same work was done either way. --------------------------------------------------
    check(
        &mut failures,
        bounded.pages_laid_out == unbounded.pages_laid_out,
        format!(
            "the two walks laid out {} and {} pages, so they are not the same walk and the \
             comparison above is between two different things",
            bounded.pages_laid_out, unbounded.pages_laid_out
        ),
    );
    check(
        &mut failures,
        unbounded.fragment_evictions == 0,
        "the unbounded run evicted something, so its budget is reachable after all".to_owned(),
    );

    if failures.is_empty() {
        println!("resident memory: 7 checks passed");
        ExitCode::SUCCESS
    } else {
        for failure in &failures {
            eprintln!("FAILED: {failure}");
        }
        ExitCode::FAILURE
    }
}

fn check(failures: &mut Vec<String>, held: bool, message: String) {
    if !held {
        failures.push(message);
    }
}

/// Opens a four-hundred-page document, scrolls from the first page to the last a page at a time,
/// and reports what the heap did.
fn walk(budget: CacheBudget) -> Walk {
    let content = Paragraphs::of(PAGES).with_blocks(BLOCKS_PER_PAGE);
    let constraints = letter();
    let clock = ManualFrameClock::new();

    // Reset **after** the content and the constraints exist, so the figure is the viewport's
    // residency and not the corpus's.
    mjx_allocation_counter::reset_peak();
    let before = mjx_allocation_counter::live();

    let mut view = DocumentView::new(
        FlowModel::flowing(),
        PlainScenes::default(),
        &content,
        constraints,
        Viewport::new(LayoutSize {
            width: constraints.page.width,
            height: constraints.page.height,
        }),
        budget,
    );

    for page in 0..PAGES {
        view.scroll_to_page(PageIndex::new(page));
        // Frames until the window is complete, because a frame budget defers rather than drops
        // and a walk that gave up after one frame would be measuring the schedule rather than the
        // caches. The model here charges no clock, so nothing ever defers and one frame suffices —
        // the loop is a guard rather than an expectation.
        for _ in 0..8 {
            let report = view
                .frame(&content, &clock, RefinementTier::Full)
                .expect("a frame");
            if report.visible_complete && !report.deferred_anything() {
                break;
            }
        }
    }

    let peak = mjx_allocation_counter::peak();
    let live = mjx_allocation_counter::live().saturating_sub(before);
    let report = view.cache_report();
    let fragments = report
        .stage(Stage::Fragments)
        .expect("the fragment stage is reported");
    Walk {
        peak_bytes: peak,
        live_bytes: live,
        pages_laid_out: view.stats().pages_laid_out,
        fragment_evictions: fragments.evictions,
        fragments_held: fragments.entries,
    }
}
