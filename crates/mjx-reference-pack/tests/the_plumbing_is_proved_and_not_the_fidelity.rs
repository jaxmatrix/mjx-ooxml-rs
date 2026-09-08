//! **The whole pipeline, end to end, against a stand-in PDF our own painter exported — and the file
//! is named so that nobody can quote it as anything else.**
//!
//! # Why the name is the most important line in this file
//!
//! Comparing our render against our own export is **self-referential**. It proves that the
//! authoring, the display list, the provider, the PDF exporter, poppler's rasteriser, the crop
//! arithmetic and the report all fit together, and it proves *nothing whatever* about whether the
//! geometry is right. MJXOFF-207's brief calls this the epic's most vacuous-looking child by
//! construction, and it is: every gate here passes with no authoritative reference in existence.
//!
//! So the claim is written into the file name, into every case name, and into the type system:
//! [`ReferenceProvider::None`] excludes *everything*, so a [`Baseline`] built from this run is
//! [`Verdict::NotEvidence`] and [`may_be_called_parity`](Baseline::may_be_called_parity) is `false`
//! for every row. A caller cannot accidentally record this as fidelity, because the value it gets
//! back is not one that can be.
//!
//! # And the plumbing gate has its own way of going hollow
//!
//! A pipeline that compares a page against itself agrees perfectly whatever it computes — including
//! if it compares nothing, crops an empty rectangle, or ignores one of its two inputs. So the two
//! halves are asserted together:
//!
//! * [`the_pipeline_runs_end_to_end_and_proves_only_the_plumbing`] — same page against itself, every
//!   plate agrees, **and** every crop has ink on both sides.
//! * [`the_comparator_tells_two_different_pages_apart`] — page one's raster against page two's, and
//!   most plates must **disagree**. A comparator that always agreed would pass the first case and
//!   fail this one, which is the only reason the first case means anything.
//!
//! # Proved by mutation — including the one that came back **green**
//!
//! * `compare_window`'s `distance > CHANNEL_TOLERANCE` → `false`:
//!   [`the_comparator_tells_two_different_pages_apart`] fails, *"only 0 of 24 plates differ between
//!   two different pages"*.
//! * `Baseline::new`'s exclusion arm removed: three cases in
//!   `an_excluded_result_is_not_evidence.rs` fail. Replacing that arm with `std::process::abort()`
//!   aborts the process (SIGABRT), which is what proves the line *executes* rather than merely
//!   being covered by a passing assertion.
//! * **⚠ `plate_baseline`'s window replaced by `PlateGeometry::at(0).window()` for every plate — so
//!   that all 187 plates crop the same corner — was GREEN.** MJXOFF-207 ran that mutation expecting
//!   it to fail and it did not, because nothing routed a *discriminating* comparison through
//!   `plate_baseline`: the case that discriminates called `compare_window` directly, and the case
//!   that called `plate_baseline` compared a page against itself, which agrees through any window.
//!   [`the_plate_baseline_crops_each_plates_own_window`] is what closes it. **The hole was in the
//!   test, not in the code**, which is the only reason a green mutation is worth writing down.
//!
//!   And the *first* attempt to close it was green too, which is the more useful half of the story:
//!   a case that asserted *"most plates disagree"* through `plate_baseline` still passed under the
//!   mutation, because cropping plate zero's window from two **different** pages still compares two
//!   different pictures — it just compares the same pair twenty-four times. What only a per-plate
//!   crop can produce is twenty-four *different numbers*, and that is what the case asserts now:
//!   under the mutation it reports *"24 plates produced 1 distinct differing-fractions"*. Counting
//!   answers is not the same as counting **distinct** answers, and this gate needed the second one.

use std::path::PathBuf;

use mjx_reference_pack::authority::{Baseline, ReferenceProvider, RenderedContent, Verdict};
use mjx_reference_pack::compare::{compare_window, plate_baseline};
use mjx_reference_pack::plates::{plates, PresetDeck};
use mjx_reference_pack::scene::our_page_as_pdf;
use mjx_reference_pack::tools::{rasterise, tool, Raster, REQUIRE_TOOLS};

/// Where a case leaves its artefacts, so a failure can be looked at rather than re-run.
fn scratch(name: &str) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/reference-pack");
    let _ = std::fs::create_dir_all(&directory);
    directory.join(name)
}

/// One page of our own side, exported and rasterised by poppler.
fn our_raster(page: usize, name: &str) -> (Raster, usize) {
    let deck = plates(PresetDeck::AtTheirDefaults);
    let render = our_page_as_pdf(&deck, page).expect("our page exports");
    assert_eq!(
        render.report.placeholders, 0,
        "our own page drew {} stand-in outline(s); a comparison against a page of framed crossed \
         rectangles is worse than no comparison",
        render.report.placeholders
    );
    // A zero placeholder count is also true of a page that drew nothing at all, so the count is
    // asserted beside it — two draws a plate (a fill and a stroke) plus the paper.
    assert_eq!(
        render.report.draw_calls,
        render.drawn_plates * 2 + 1,
        "page {page} drew {} times for {} plates filled and stroked over one paper rectangle",
        render.report.draw_calls,
        render.drawn_plates
    );
    let path = scratch(name);
    std::fs::write(&path, &render.pdf).expect("the export is written");
    (
        rasterise(&path, 1).expect("poppler rasterises it"),
        render.drawn_plates,
    )
}

#[test]
fn the_pipeline_runs_end_to_end_and_proves_only_the_plumbing() {
    const CASE: &str = "the pipeline runs end to end";
    if !tool(CASE, "pdftoppm", REQUIRE_TOOLS) {
        return;
    }
    let deck = plates(PresetDeck::AtTheirDefaults);
    let (raster, drawn) = our_raster(0, "plumbing-page-1.pdf");
    assert!(drawn > 20, "page one drew only {drawn} plates");

    let rows: Vec<Baseline> = deck
        .iter()
        .filter(|plate| plate.page() == 0)
        .map(|plate| {
            plate_baseline(
                plate,
                &raster,
                &raster,
                ReferenceProvider::None,
                "our own PDF export",
            )
        })
        .collect();

    assert_eq!(
        rows.len(),
        deck.iter().filter(|plate| plate.page() == 0).count(),
        "a page's plates and its rows are not the same set"
    );

    // The two halves. First: the pipeline ran, and the crops were not empty.
    let mut compared = 0usize;
    for plate in deck.iter().filter(|plate| plate.page() == 0) {
        if !plate.kind.draws() {
            continue;
        }
        let agreement = compare_window(&raster, &raster, plate.geometry().window());
        assert!(
            agreement.pixels > 10_000,
            "`{}`'s window compared only {} pixels; the crop arithmetic is wrong",
            plate.token,
            agreement.pixels
        );
        assert!(
            agreement.reference_ink > 0 && agreement.ours_ink > 0,
            "`{}`'s window has no ink on one side, so an agreement there means nothing",
            plate.token
        );
        assert_eq!(
            agreement.differing, 0,
            "`{}` disagrees with itself",
            plate.token
        );
        compared += 1;
    }
    assert!(
        compared > 20,
        "only {compared} plates were actually compared, which is not a page"
    );

    // Second, and the reason for the file's name: **none of it is evidence.**
    assert!(
        rows.iter().all(|row| !row.verdict.is_evidence()),
        "a row of a self-comparison was recorded as evidence"
    );
    assert_eq!(
        mjx_reference_pack::parity_count(&rows),
        0,
        "a self-comparison produced a row that may be called parity"
    );
    println!(
        "{CASE}: {compared} plates through author -> display list -> provider -> PDF -> pdftoppm -> \
         crop -> report. Every row is `not evidence`, because the reference is our own export."
    );
}

#[test]
fn the_comparator_tells_two_different_pages_apart() {
    const CASE: &str = "the comparator tells two pages apart";
    if !tool(CASE, "pdftoppm", REQUIRE_TOOLS) {
        return;
    }
    let deck = plates(PresetDeck::AtTheirDefaults);
    let (first, _) = our_raster(0, "plumbing-page-1.pdf");
    let (second, _) = our_raster(1, "plumbing-page-2.pdf");

    let mut differing = 0usize;
    let mut total = 0usize;
    for plate in deck.iter().filter(|plate| plate.page() == 0) {
        if !plate.kind.draws() {
            continue;
        }
        total += 1;
        if compare_window(&first, &second, plate.geometry().window())
            .verdict(true)
            .is_failure()
        {
            differing += 1;
        }
    }
    // Not "all": two plates of a rectilinear shape at the same size really are the same picture,
    // and `flowChartProcess` on page one against whatever is in that cell on page two may well
    // both be plain rectangles. Most of them is the claim, and it is the claim that fails for a
    // comparator that always agrees.
    assert!(
        differing * 2 > total,
        "only {differing} of {total} plates differ between two different pages, so the comparator \
         is not looking at what it thinks it is"
    );
    println!("{CASE}: {differing} of {total} plates differ between page one and page two");
}

/// **`plate_baseline` crops the plate it was given**, and not some other plate.
///
/// This case exists because the mutation that should have caught that came back green: see this
/// file's own heading. It is the same discrimination
/// [`the_comparator_tells_two_different_pages_apart`] makes, routed through the function a report is
/// actually built from — which is what the earlier version was missing.
#[test]
fn the_plate_baseline_crops_each_plates_own_window() {
    const CASE: &str = "plate_baseline crops the plate it was given";
    if !tool(CASE, "pdftoppm", REQUIRE_TOOLS) {
        return;
    }
    let deck = plates(PresetDeck::AtTheirDefaults);
    let (first, _) = our_raster(0, "plumbing-page-1.pdf");
    let (second, _) = our_raster(1, "plumbing-page-2.pdf");

    let mut differing = 0usize;
    let mut total = 0usize;
    let mut fractions: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    for plate in deck.iter().filter(|plate| plate.page() == 0) {
        if !plate.kind.draws() {
            continue;
        }
        total += 1;
        // Deliberately the authoritative provider: an exclusion would make every row `not evidence`
        // and the counts below would be zero for a reason that has nothing to do with the window.
        let row = plate_baseline(
            plate,
            &first,
            &second,
            ReferenceProvider::OfficeExport,
            "two pages of our own export",
        );
        assert!(
            row.verdict.is_evidence(),
            "`{}` produced no evidence out of two inked pages: {row:?}",
            plate.token
        );
        let fraction = match &row.verdict {
            Verdict::Agreed {
                differing_fraction, ..
            } => *differing_fraction,
            Verdict::Disagreed {
                differing_fraction, ..
            } => {
                differing += 1;
                *differing_fraction
            }
            Verdict::NotEvidence { .. } => unreachable!("asserted above"),
        };
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a fraction in 0..=1, bucketed to six decimal places for a distinctness count"
        )]
        fractions.insert((fraction * 1e6) as u64);
    }
    assert!(
        differing * 2 > total,
        "only {differing} of {total} plates differ when `plate_baseline` is handed two *different* \
         pages, so the comparator is blind"
    );
    // **The assertion the window mutation actually fails.** Counting disagreements is not enough:
    // a `plate_baseline` that cropped *plate zero's* window for all 24 plates would still be
    // comparing two different pictures and would still report 21 disagreements — it would just
    // report the same one 24 times. What only a per-plate crop can produce is 24 *different*
    // numbers.
    assert!(
        fractions.len() * 2 > total,
        "{total} plates produced {} distinct differing-fractions, so most rows are the same \
         measurement repeated: every plate is being cropped from the same place.",
        fractions.len()
    );
    println!(
        "{CASE}: {differing} of {total} plates differ through `plate_baseline`, over {} distinct \
         fractions",
        fractions.len()
    );
}

/// The third state exists and is reachable from the pipeline, not only from a unit test.
#[test]
fn a_plate_with_nothing_to_draw_is_not_evidence_rather_than_a_disagreement() {
    const CASE: &str = "a plate with nothing to draw is not evidence";
    if !tool(CASE, "pdftoppm", REQUIRE_TOOLS) {
        return;
    }
    let deck = plates(PresetDeck::AtTheirDefaults);
    let up_arrow = deck
        .iter()
        .find(|plate| plate.token == "upArrow")
        .expect("`upArrow` is on the sheet");
    let (raster, _) = our_raster(up_arrow.page(), "plumbing-up-arrow.pdf");

    let row = plate_baseline(
        up_arrow,
        &raster,
        &raster,
        // Deliberately the *authoritative* provider, so the exclusion cannot be the provider's:
        // this plate is not evidence because there is no table for it, whoever produced the
        // reference.
        ReferenceProvider::OfficeExport,
        "our own PDF export",
    );
    assert!(
        matches!(row.verdict, Verdict::NotEvidence { .. }),
        "`upArrow` produced {row:?}, but nothing on our side draws it"
    );
    assert!(
        !row.may_be_called_parity(),
        "a plate with no geometry to compare was called parity"
    );
    assert_eq!(row.content, RenderedContent::Outline);
    println!("{CASE}: {row}");
}
