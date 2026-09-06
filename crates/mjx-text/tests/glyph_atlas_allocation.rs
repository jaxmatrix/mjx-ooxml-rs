//! **The atlas's byte ceiling, measured (MJXOFF-159).** One binary, one `main`, one thread, one
//! counting global allocator, and a byte bound on a glyph atlas that has been asked for far more
//! than it is allowed to hold.
//!
//! `docs/UI_PLATFORM_PLAN.md` §12 gives the whole GPU texture budget 256 MB on a desktop and 96 MB
//! on a phone, and says the glyph atlas is its largest single contributor. This file is where that
//! stops being a sentence in a plan.
//!
//! # Why this is a target of its own with no test harness
//!
//! A `#[global_allocator]` is installed for a whole process, and `cargo test` runs a harness's cases
//! on several threads inside one process — so a figure measured under those conditions is the figure
//! of whatever else happened to be running. `harness = false` in `Cargo.toml` gives this file a
//! plain `main`: the cases below run in sequence, on one thread, with nothing else in the process.
//! `crates/mjx-sml/tests/cell_store_allocation.rs` states the same reasoning at length and installs
//! the same allocator; there is one `unsafe impl GlobalAlloc` in this workspace and this is its
//! third consumer, not a second implementation.
//!
//! # Why an allocation counter and not `resident_bytes`
//!
//! [`GlyphAtlas::resident_bytes`] is the atlas's own opinion of what it costs, and a gate that
//! consulted only it would pass against an atlas that dropped a page's `Vec` from its accounting and
//! kept it alive somewhere else. The allocator sees bytes the program actually asked for, whatever
//! the bookkeeping says. Both are asserted below, and the pair is the point: the structural figure
//! says the policy believes it held, and the measured figure says it really did.
//!
//! # The trap this file is written against, in its own terms
//!
//! **A cache that evicts everything satisfies every byte bound perfectly.** So does one that never
//! inserted anything. Case one therefore asserts four things at once — the ceiling held, eviction
//! actually ran, the atlas is not empty, and *the glyphs of the frame being drawn are still there* —
//! and dropping any one of the four leaves a claim that a broken atlas would also satisfy.
//!
//! # Proving the gate can fail
//!
//! Case three runs the identical workload under a ceiling half the size and asserts that both the
//! structural and the measured figure come out **strictly smaller**. If the ceiling were not what
//! bounded case one — if the workload simply never filled the atlas — the two cases would measure
//! the same thing and case three would fail.

#[path = "support/mod.rs"]
mod support;

use mjx_text::{
    place_run, DeviceScale, FeatureSet, FontError, FontSize, GlyphAtlas, GlyphRasterKey,
    GlyphRasteriser, Hinting, RunPlacement, Shaper, ShapingRequest, TextScript,
};

use support::synthetic_font;

#[global_allocator]
static ALLOCATOR: mjx_allocation_counter::Counting = mjx_allocation_counter::Counting;

/// How wide and tall one page is in these cases.
///
/// Far smaller than [`mjx_text::ATLAS_PAGE_SIZE_PIXELS`], so that a working set outgrows the atlas
/// inside a test rather than inside a document. The policy under measurement does not depend on the
/// figure; the running time does.
const PAGE: u16 = 128;

/// The ceiling case one holds, in bytes: six coverage pages of [`PAGE`] square.
const CEILING: usize = (PAGE as usize) * (PAGE as usize) * 6;

/// What the atlas is allowed to cost *beyond* its pages, in bytes.
///
/// The pages are the budget; this covers what the bookkeeping around them costs — the entry map, the
/// page table, the delta being accumulated for the frame in hand — plus whatever `swash`'s scratch
/// buffers grew to while the glyphs were being scan-converted, since the allocator counts the whole
/// process and the rasteriser is in it. Generous on purpose: this is the line between "the atlas is
/// bounded by its ceiling" and "the atlas is bounded by nothing", not a regression bound on an exact
/// figure.
const OVERHEAD_BOUND: usize = 256 * 1024;

/// The alphabet every case draws with: eight letters, eight differently shaped rectangles.
const ALPHABET: &str = "ABCDEFGH";

fn main() {
    let first = case_the_ceiling_holds_and_the_working_set_survives();
    case_a_ceiling_below_one_page_refuses_rather_than_evicting_the_frame();
    case_halving_the_ceiling_halves_what_is_held(first);
    println!("\nthe glyph atlas holds its declared byte ceiling.");
}

/// Everything a frame needs. Built before any measurement begins, so that the face's bytes, the
/// shaper's cache and `swash`'s context are not counted as the atlas's cost.
struct Frames {
    face: std::sync::Arc<mjx_text::FontFace>,
    identity: mjx_text::FaceId,
    rasteriser: GlyphRasteriser,
    shaper: Shaper,
    features: FeatureSet,
}

impl Frames {
    fn new() -> Self {
        let face = synthetic_font::outlined_latin_face().face();
        let mut rasteriser = GlyphRasteriser::new();
        let identity = rasteriser
            .register(&face)
            .expect("the synthetic face registers");
        Self {
            face,
            identity,
            rasteriser,
            shaper: Shaper::new(),
            features: FeatureSet::default(),
        }
    }

    fn place(&mut self, points: f64) -> RunPlacement {
        let size = FontSize::from_points(points);
        let request = ShapingRequest::new(ALPHABET, TextScript::LATIN, size, &self.features);
        let run = self.shaper.shape(&self.face, &request).expect("shapes");
        place_run(
            &run,
            self.identity,
            DeviceScale::from_pixels_per_point(1.0),
            Hinting::GridFitted,
            (0.0, 0.0),
        )
    }
}

/// Fill `atlas` from `frames`, one size per frame, and return the keys of the last frame's glyphs.
///
/// The last frame is the working set: its glyphs go in first, and then three more runs are asked for
/// inside the *same* frame, which is what forces the atlas to choose between the ceiling and the
/// frame it is drawing.
fn draw_many_frames(atlas: &mut GlyphAtlas, frames: &mut Frames) -> Vec<GlyphRasterKey> {
    for step in 0..24 {
        atlas.begin_frame();
        let placement = frames.place(16.0 + f64::from(step));
        match atlas.prepare_run(&mut frames.rasteriser, &placement) {
            Ok(_) | Err(FontError::GlyphAtlasExhausted { .. }) => {}
            Err(other) => panic!("an unexpected failure while filling the atlas: {other}"),
        }
        // Taking the delta and dropping it is what a painter does at the end of a frame, and doing
        // it here keeps the measurement about the atlas rather than about an unbounded queue of
        // uploads nobody consumed.
        drop(atlas.take_delta());
    }

    atlas.begin_frame();
    let working_set = frames.place(40.0);
    atlas
        .prepare_run(&mut frames.rasteriser, &working_set)
        .expect("the working set fits, because the pages before it are all older frames'");
    let held: Vec<GlyphRasterKey> = working_set
        .glyphs()
        .iter()
        .map(|placed| placed.key)
        .collect();

    for step in 0..3 {
        let filler = frames.place(41.0 + f64::from(step));
        match atlas.prepare_run(&mut frames.rasteriser, &filler) {
            Ok(_) | Err(FontError::GlyphAtlasExhausted { .. }) => {}
            Err(other) => panic!("an unexpected failure while crowding the atlas: {other}"),
        }
    }
    drop(atlas.take_delta());
    held
}

/// The ceiling held, eviction ran, the atlas is not empty, and the frame being drawn survived.
fn case_the_ceiling_holds_and_the_working_set_survives() -> (usize, usize) {
    println!("case 1 — a glyph atlas under a {CEILING} byte ceiling");

    let mut frames = Frames::new();
    // Warm the rasteriser outside the measurement: `swash`'s scratch buffers grow to fit the largest
    // glyph it has been shown, and that growth belongs to the rasteriser rather than to the atlas.
    {
        let mut warm = GlyphAtlas::with_configuration(PAGE, CEILING);
        drop(draw_many_frames(&mut warm, &mut frames));
    }

    let before = mjx_allocation_counter::live();
    let mut atlas = GlyphAtlas::with_configuration(PAGE, CEILING);
    let held = draw_many_frames(&mut atlas, &mut frames);
    let retained = mjx_allocation_counter::live().saturating_sub(before);

    let statistics = atlas.statistics();
    println!("  pages held                  {:>12}", statistics.pages);
    println!("  glyph images held           {:>12}", statistics.entries);
    println!(
        "  pages evicted               {:>12}",
        statistics.pages_evicted
    );
    println!(
        "  glyph images evicted        {:>12}",
        statistics.entries_evicted
    );
    println!(
        "  the atlas says it costs     {:>12} bytes",
        statistics.resident_bytes
    );
    println!("  the allocator says          {:>12} bytes", retained);
    println!("  the ceiling                 {:>12} bytes", CEILING);

    assert!(
        statistics.resident_bytes <= CEILING,
        "the atlas's own accounting says it holds {} bytes against a ceiling of {CEILING}",
        statistics.resident_bytes
    );
    assert!(
        retained <= CEILING + OVERHEAD_BOUND,
        "the allocator says the atlas retained {retained} bytes against a ceiling of {CEILING} \
         plus {OVERHEAD_BOUND} of bookkeeping — which is the figure that would move if a page were \
         dropped from the accounting and kept alive anyway"
    );

    // Three assertions that a broken atlas would also have satisfied, if they were not here.
    assert!(
        statistics.pages_evicted > 0 && statistics.entries_evicted > 0,
        "eviction must actually have run: a ceiling that held because the workload never reached it \
         is not a ceiling that was tested ({statistics:?})"
    );
    assert!(
        statistics.entries > 0 && statistics.pages > 0,
        "and the atlas must not be empty, because an atlas that evicted everything satisfies every \
         byte bound perfectly ({statistics:?})"
    );
    let mut survivors = 0_usize;
    for key in &held {
        assert!(
            atlas.get(key).is_some(),
            "a glyph the frame in hand has already drawn must still be there: {key:?}"
        );
        survivors += 1;
    }
    assert_eq!(
        survivors,
        ALPHABET.chars().count(),
        "and the working set must have been the whole alphabet, or the survival above is the \
         survival of nothing"
    );
    println!("  the working set survived    {survivors:>12} glyphs");

    (statistics.resident_bytes, retained)
}

/// An atlas that cannot evict says so, rather than sacrificing the frame it is drawing.
fn case_a_ceiling_below_one_page_refuses_rather_than_evicting_the_frame() {
    println!("\ncase 2 — a ceiling of exactly one page, and a frame that needs more");

    let mut frames = Frames::new();
    let ceiling = usize::from(PAGE) * usize::from(PAGE);
    let mut atlas = GlyphAtlas::with_configuration(PAGE, ceiling);

    atlas.begin_frame();
    let mut refusals = 0_usize;
    for step in 0..24 {
        let placement = frames.place(30.0 + f64::from(step));
        match atlas.prepare_run(&mut frames.rasteriser, &placement) {
            Ok(_) => {}
            Err(FontError::GlyphAtlasExhausted {
                resident_bytes,
                ceiling_bytes,
            }) => {
                assert!(resident_bytes <= ceiling_bytes);
                refusals += 1;
            }
            Err(other) => panic!("an unexpected failure: {other}"),
        }
    }

    println!("  runs refused                {refusals:>12}");
    println!(
        "  the atlas holds             {:>12} bytes",
        atlas.resident_bytes()
    );
    assert!(
        refusals > 0,
        "once every page belongs to the frame in hand there is nothing left to evict, and the \
         refusal is what stops the atlas throwing away the frame it is drawing in order to satisfy \
         a byte bound"
    );
    assert!(atlas.resident_bytes() <= ceiling);
    assert!(
        atlas.statistics().entries > 0,
        "and it must still hold what it managed to pack before it ran out"
    );
}

/// The same workload under half the ceiling holds strictly less — which is what proves the ceiling,
/// and not the workload, is what bounded case one.
fn case_halving_the_ceiling_halves_what_is_held(first: (usize, usize)) {
    let halved = CEILING / 2;
    println!("\ncase 3 — the identical workload under a {halved} byte ceiling");

    let mut frames = Frames::new();
    {
        let mut warm = GlyphAtlas::with_configuration(PAGE, halved);
        drop(draw_many_frames(&mut warm, &mut frames));
    }

    let before = mjx_allocation_counter::live();
    let mut atlas = GlyphAtlas::with_configuration(PAGE, halved);
    drop(draw_many_frames(&mut atlas, &mut frames));
    let retained = mjx_allocation_counter::live().saturating_sub(before);

    let statistics = atlas.statistics();
    println!(
        "  the atlas says it costs     {:>12} bytes",
        statistics.resident_bytes
    );
    println!("  the allocator says          {:>12} bytes", retained);
    println!(
        "  case 1 said                 {:>12} and {} bytes",
        first.0, first.1
    );

    assert!(statistics.resident_bytes <= halved);
    assert!(
        statistics.resident_bytes < first.0,
        "halving the ceiling must halve what is held: it held {} against case one's {}, which means \
         the ceiling is not what bounds this atlas",
        statistics.resident_bytes,
        first.0
    );
    assert!(
        retained < first.1,
        "and the allocator must see the difference too: {retained} against case one's {}",
        first.1
    );
    assert!(
        statistics.entries > 0 && statistics.pages_evicted > 0,
        "under the tighter ceiling the atlas must still be evicting and still be holding something \
         ({statistics:?})"
    );
}
