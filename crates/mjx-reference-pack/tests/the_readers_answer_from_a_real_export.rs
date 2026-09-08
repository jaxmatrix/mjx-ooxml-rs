//! **The four non-geometry readers, run end to end against an export a different program made —
//! and every number they produce is still `Provisional`.**
//!
//! # What this proves, and the line it must not be quoted past
//!
//! LibreOffice is a *real* producer: it lays out the type specimens with its own engine, substitutes
//! its own faces, breaks the Japanese paragraph with its own rules and draws its own hatches. So
//! running the readers over its export proves the readers **work on a PDF this project did not
//! write** — that the probe rectangles find their words, that the span arithmetic yields a plausible
//! advance, that the pitch specimen comes back as six lines, that the hanging paragraph breaks into
//! enough lines to have a measure, and that a hatch swatch has ink.
//!
//! It proves **nothing about the numbers**. LibreOffice's Arial is Liberation Sans and its Cambria
//! is Caladea; its hatches are its own. Every row is
//! [`ReferenceAuthority::Provisional`](mjx_text::ReferenceAuthority::Provisional) and
//! [`parity_count`] over the lot is zero, which is asserted here rather than assumed.
//!
//! # The assertion that is actually about us
//!
//! [`the_metric_clone_reads_back_within_the_tolerance_the_table_states`] is the one exception, and
//! it is worth its own paragraph. Liberation Sans is a **metric-compatible** clone of Arial: that is
//! the entire property `mjx_text::reference` exists to check, and it is a property of the *face*,
//! not of the renderer. So its advances must come back matching the reference table's Arial to
//! within [`TOLERANCE`] — and if they do, the reader's arithmetic is right, the
//! probe design works, and the table's Arial row is confirmed against a third source. That is a
//! genuine result out of a preliminary run, and it is confined to the one place where the producer's
//! substitution is *supposed* to preserve the number.
//!
//! # Proved by mutation
//!
//! * `advance_of`'s extracted-text check removed: Arial's `%` row becomes a number, and
//!   [`the_metric_clone_reads_back_within_the_tolerance_the_table_states`] fails on it by 80
//!   thousandths of an em. **That is the mutation that matters**, because the wrong number is
//!   plausible and nothing else in the file would have noticed.
//! * `Probe::advance_per_mille`'s divisor `PROBE_REPEATS - 1` → `PROBE_REPEATS`: every advance is
//!   ten percent short and the same case fails on all ninety-two.

use std::path::PathBuf;
use std::sync::OnceLock;

use mjx_reference_pack::authority::ReferenceProvider;
use mjx_reference_pack::ingest::{
    as_baselines, read_advances, read_hanging, read_hatch_tiles, read_line_pitch,
};
use mjx_reference_pack::pack::generate;
use mjx_reference_pack::tools::{convert_to_pdf, tool, REQUIRE_SOFFICE, REQUIRE_TOOLS};
use mjx_reference_pack::{parity_count, provisional_baselines};
use mjx_text::reference::reference_for_family;

/// How far a measured advance may sit from the published table's, in thousandths of an em.
///
/// `mjx_text::reference::ADVANCE_TOLERANCE_PER_MILLE` is **0.5**, which is the tolerance for
/// comparing a *face's own numbers* against a published table — quantisation and nothing else. This
/// is a different measurement: it goes through a layout engine, a PDF exporter and a text
/// extractor, each of which rounds. Four thousandths of an em is a tenth of a point at the 24-point
/// size the probes are set at, which is the scale poppler's own two decimal places live on, and it
/// is far below the smallest disagreement between two *different* faces.
const TOLERANCE: f64 = 4.0;

/// Where the artefacts and their conversions live.
fn scratch() -> PathBuf {
    let directory =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/reference-pack/readers");
    let _ = std::fs::create_dir_all(&directory);
    directory
}

/// The specimen deck and the hanging document, converted by LibreOffice — **once**.
///
/// Cargo runs a binary's cases on several threads at a time, and two `soffice --headless` processes
/// racing for the same user profile is a flake that would read as a reader defect. So the conversion
/// happens under a `OnceLock` and both cases share the result, which also halves the run.
fn exported(case: &str) -> Option<(PathBuf, PathBuf)> {
    static CONVERTED: OnceLock<Option<(PathBuf, PathBuf)>> = OnceLock::new();
    CONVERTED
        .get_or_init(|| {
            if !tool(case, "soffice", REQUIRE_SOFFICE)
                || !tool(case, "pdftotext", REQUIRE_TOOLS)
                || !tool(case, "pdftoppm", REQUIRE_TOOLS)
            {
                return None;
            }
            let directory = scratch();
            generate(&directory).expect("the pack generates");
            let specimens = convert_to_pdf(
                &directory.join(mjx_reference_pack::typography::FILE_NAME),
                &directory,
            )
            .expect("LibreOffice converts the specimen deck");
            let hanging = convert_to_pdf(
                &directory.join(mjx_reference_pack::hanging::FILE_NAME),
                &directory,
            )
            .expect("LibreOffice converts the hanging document");
            Some((specimens, hanging))
        })
        .clone()
}

#[test]
fn every_reader_produces_a_complete_answer_and_none_of_it_is_parity() {
    const CASE: &str = "the four readers over a LibreOffice export";
    let Some((specimens, hanging)) = exported(CASE) else {
        return;
    };

    let advances = read_advances(&specimens).expect("the advance ruler reads");
    let pitches = read_line_pitch(&specimens).expect("the pitch specimens read");
    let hangs = read_hanging(&hanging).expect("the hanging paragraphs read");
    let hatches = read_hatch_tiles(&specimens).expect("the swatches read");

    assert_eq!(advances.len(), 92 * 5, "one row per probe");
    assert_eq!(pitches.len(), 5, "one row per family");
    assert_eq!(hangs.len(), 3, "one row per paragraph");
    assert_eq!(hatches.len(), 54, "one row per preset pattern");

    // Every reader has to actually read something. A run in which each answered `not evidence`
    // 460 times would satisfy every count above.
    let read = advances
        .iter()
        .filter(|row| row.advance_per_mille.is_some())
        .count();
    assert!(
        read > 400,
        "only {read} of {} probes came back with an advance",
        advances.len()
    );
    let pitched = pitches
        .iter()
        .filter(|row| row.pitch_per_em.is_some())
        .count();
    assert!(pitched >= 4, "only {pitched} of five families gave a pitch");
    for reading in &hangs {
        assert!(
            reading.lines >= 4,
            "`{}` came back as {} line(s)",
            reading.role.label(),
            reading.lines
        );
        println!(
            "{:<30} {} lines, measure {:.2} pt, hangs {:?}",
            reading.role.label(),
            reading.lines,
            reading.measure_points,
            reading.hangs
        );
    }
    let inked = hatches.iter().filter(|row| row.coverage > 0.01).count();
    assert!(
        inked > 40,
        "only {inked} of 54 hatch swatches have ink; a swatch sheet that drew nothing would say the \
         same thing"
    );
    let tiles = hatches.iter().filter(|row| row.tile.is_some()).count();
    println!("hatches: {inked} inked, {tiles} with a recoverable tile");

    // And the whole point.
    let rows = as_baselines(
        &advances,
        &pitches,
        &hangs,
        &hatches,
        ReferenceProvider::LibreOffice,
        "a LibreOffice export",
    );
    assert_eq!(
        rows.len(),
        advances.len() + pitches.len() + hangs.len() + hatches.len()
    );
    assert_eq!(
        parity_count(&rows),
        0,
        "a LibreOffice run produced a typography row that may be called parity"
    );
    assert_eq!(provisional_baselines(&rows).len(), rows.len());

    // The hatches are excluded *by the provider*, so they are `not evidence` even though the
    // reader read them perfectly well. That is the exclusion doing its job on real data.
    let hatch_rows = &rows[advances.len() + pitches.len() + hangs.len()..];
    assert!(
        hatch_rows.iter().all(|row| !row.verdict.is_evidence()),
        "a hatch compared against LibreOffice was recorded as evidence"
    );
}

/// The one genuine number a preliminary run can produce, and why it is genuine.
#[test]
fn the_metric_clone_reads_back_within_the_tolerance_the_table_states() {
    const CASE: &str = "Liberation Sans against the table's Arial";
    let Some((specimens, _)) = exported(CASE) else {
        return;
    };
    let advances = read_advances(&specimens).expect("the advance ruler reads");
    let arial = reference_for_family("Arial").expect("the table has Arial");

    let mut checked = 0usize;
    let mut refused = Vec::new();
    let mut disagreed: Vec<(char, f64, f64)> = Vec::new();
    let mut worst = (0.0f64, ' ');
    for reading in advances.iter().filter(|row| row.family == "Arial") {
        let Some(measured) = reading.advance_per_mille else {
            refused.push(reading.character);
            continue;
        };
        let published = arial
            .advance_for_character(reading.character)
            .expect("every probe is in the table it was read from");
        let expected = f64::from(published.font_units) / f64::from(published.units_per_em) * 1000.0;
        let difference = (measured - expected).abs();
        checked += 1;
        if difference > worst.0 {
            worst = (difference, reading.character);
        }
        if difference > TOLERANCE {
            disagreed.push((reading.character, measured, expected));
        }
    }
    assert!(
        checked > 80,
        "only {checked} of 92 Arial probes were checked; refused: {refused:?}"
    );
    // **Not "all of them", and the reason is this ticket's central warning.** A disagreement here is
    // the *producer's*: LibreOffice's PDF export of a run of `1`s comes back 489.5 thousandths of an
    // em wide where the same character in the one-copy box measures 556.75 — the character's own
    // advance is right and only the repeated run is not. Making that fail the build would be exactly
    // the mistake MJXOFF-207 exists to head off: editing correct code until it matches a reference
    // that is explicitly not authoritative. So the disagreements are **named and counted**, and the
    // gate is on how many there are.
    assert!(
        disagreed.len() <= 2,
        "{} of {checked} Arial probes disagree with the published table by more than {TOLERANCE} \
         thousandths of an em: {disagreed:?}. One or two is this producer's own export; a dozen is \
         the reader's arithmetic.",
        disagreed.len()
    );

    // **The second estimate, and the finding it exists to carry.** The run of ten copies is shaped:
    // `ff` is a ligature, and LibreOffice lays out a run of `1`s narrower than ten separate ones. So
    // the two estimates disagree for a handful of characters, and that disagreement is a fact about
    // the *producer* rather than about either arithmetic — which is exactly why the reported number
    // is the ligature-free one and this is only a cross-check.
    let mut shaped: Vec<(char, f64)> = advances
        .iter()
        .filter(|row| row.family == "Arial")
        .filter_map(|row| Some((row.character, row.estimates_disagree_by()?)))
        .filter(|(_, difference)| *difference > TOLERANCE)
        .collect();
    shaped.sort_by(|left, right| right.1.total_cmp(&left.1));
    assert!(
        shaped.len() < 10,
        "{} of the Arial probes have two estimates that disagree: {shaped:?}. A handful is shaping; \
         a tenth of the alphabet is one of the two arithmetics being wrong.",
        shaped.len()
    );
    // And the cross-check has to be doing something: a run that agreed everywhere because it was
    // never computed would satisfy the bound above perfectly.
    let cross_checked = advances
        .iter()
        .filter(|row| row.family == "Arial" && row.estimates_disagree_by().is_some())
        .count();
    assert!(
        cross_checked > 80,
        "only {cross_checked} Arial probes have a second estimate at all"
    );
    println!("{CASE}: {cross_checked} probes cross-checked, shaped differently: {shaped:?}");
    println!(
        "{CASE}: {checked} probes read, {} disagree by more than {TOLERANCE} per mille (worst \
         {:.3} on {:?}); {} refused: {refused:?}. Disagreements: {disagreed:?}",
        disagreed.len(),
        worst.0,
        worst.1,
        refused.len()
    );
}
