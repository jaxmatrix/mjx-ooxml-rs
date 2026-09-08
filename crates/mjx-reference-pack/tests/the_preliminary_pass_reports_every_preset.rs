//! **The preliminary pass over all 187, with every row present and every exclusion named.**
//!
//! # What is asserted, and what is deliberately not
//!
//! Asserted: that the pass **runs**, that it produces **one row per plate** with none dropped, that
//! every row that is not a comparison **says why**, that every row is `Provisional`, and that
//! `parity_count` is **zero**.
//!
//! Not asserted: that the rows agree. A LibreOffice disagreement is a thing to look at before a
//! Windows morning, not a build failure — making it one would be the first step towards editing
//! correct code until it matches a reference that is explicitly not authoritative, which is the
//! precise trap MJXOFF-207 exists to head off.
//!
//! # A quietly-skipped case is indistinguishable from a check that never existed
//!
//! So a missing `soffice` or `pdftoppm` is a **named, printed skip**, and `MJX_REQUIRE_SOFFICE=1` /
//! `MJX_REQUIRE_TOOLS=1` turn either absence into a failure — which is what continuous integration
//! sets. That is the same shape `MJX_REQUIRE_SCHEMA` and `MJX_REQUIRE_GPU` already use here.
//!
//! # What this pass actually found, recorded so the next reader is not surprised
//!
//! Measured on LibreOffice 25.x at 96 dpi, and none of it is a defect on our side:
//!
//! * **The three `adj5` singularities are exactly where MJXOFF-205 measured them** —
//!   `circularArrow`, `leftCircularArrow`, `leftRightCircularArrow` — and this crate finds them by
//!   *resolving* the shape rather than by carrying that ticket's list. Two independent routes to
//!   the same three shapes is worth more than either.
//! * **The seventeen action buttons differ by 6–15 %**, because Impress draws its own three-dimensional
//!   button chrome for `actionButton*` rather than the preset's outline.
//! * **The connectors differ**, and in the informative direction: `bentConnector2`'s reference
//!   window has 321 inked pixels against our 6 419. A connector's path is open and we *fill* it,
//!   because the deck states a solid fill on every shape; Impress declines to fill an open
//!   connector. Which of the two PowerPoint does is one of the things the sitting will say.
//! * **The callouts differ** by 3–7 %, all in the tail.
//! * **Forty-three plates agree exactly, to the pixel** — every rectilinear one (`rect`,
//!   `flowChartProcess`, `mathPlus`, `frame`, `corner`) — which is a useful sanity check on the crop
//!   arithmetic: two renderers really do produce identical axis-aligned rectangles at integer point
//!   coordinates, and anything that did not land on the same pixel grid could not.

use mjx_reference_pack::pack::{generate, preliminary_pass, report};
use mjx_reference_pack::plates::{plates, PresetDeck};
use mjx_reference_pack::tools::{tool, REQUIRE_SOFFICE, REQUIRE_TOOLS};
use mjx_reference_pack::{parity_count, provisional_baselines};

/// Where the pass leaves its artefacts, so a failure can be looked at.
fn scratch() -> std::path::PathBuf {
    let directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/reference-pack/preliminary");
    let _ = std::fs::create_dir_all(&directory);
    directory
}

#[test]
fn every_preset_gets_a_row_in_both_decks() {
    const CASE: &str = "the preliminary pass over all 187";
    if !tool(CASE, "soffice", REQUIRE_SOFFICE) || !tool(CASE, "pdftoppm", REQUIRE_TOOLS) {
        return;
    }
    if !tool(CASE, "pdfinfo", REQUIRE_TOOLS) {
        return;
    }
    let directory = scratch();
    generate(&directory).expect("the pack generates");

    for which in PresetDeck::ALL {
        let table = plates(which);
        let pass = preliminary_pass(&directory, which).expect("the preliminary pass runs");
        println!("== {} ==\n{}", pass.artefact, report(&pass.baselines));

        assert_eq!(
            pass.baselines.len(),
            table.len(),
            "{} produced {} rows for {} plates; a dropped row is indistinguishable from a check \
             that never existed",
            which.file_name(),
            pass.baselines.len(),
            table.len()
        );
        // By subject, not only by count: 187 rows all about `rect` would satisfy a count.
        let subjects: Vec<&str> = pass
            .baselines
            .iter()
            .map(|row| row.subject.as_str())
            .collect();
        let expected: Vec<&str> = table.iter().map(|plate| plate.token).collect();
        assert_eq!(subjects, expected, "{}", which.file_name());

        // Every row that is not a comparison says why, in a sentence.
        for row in &pass.baselines {
            if row.verdict.is_evidence() {
                continue;
            }
            let printed = format!("{}", row.verdict);
            assert!(
                printed.len() > 40 && printed.contains("not evidence"),
                "`{}` is excluded and says only {printed:?}",
                row.subject
            );
        }

        // And the whole point: none of it is parity, all of it is re-adjudicable.
        assert_eq!(
            parity_count(&pass.baselines),
            0,
            "a LibreOffice pass over {} produced a row that may be called parity",
            which.file_name()
        );
        assert_eq!(
            provisional_baselines(&pass.baselines).len(),
            pass.baselines.len(),
            "not every row of a LibreOffice pass is listable as provisional"
        );

        // The pass has to be doing work. A run in which every row was excluded would satisfy
        // everything above and would mean the comparison never happened.
        let evidence = pass
            .baselines
            .iter()
            .filter(|row| row.verdict.is_evidence())
            .count();
        assert!(
            evidence > 150,
            "only {evidence} of {} rows are a comparison at all in {}",
            pass.baselines.len(),
            which.file_name()
        );
    }
}

/// The three shapes whose own formulas have no value at a handle's stop, found by **resolving them**
/// rather than by carrying MJXOFF-205's list of names.
///
/// Two independent routes to the same three shapes is worth more than either alone: that ticket
/// measured them by sweeping every adjustment's domain, and this crate finds them by asking each
/// shape for an outline at the extreme it authors. A disagreement between the two would mean the
/// extremes deck is not at the extremes it claims.
#[test]
fn the_singular_plates_are_the_three_that_have_no_value_at_a_stop() {
    let extremes = plates(PresetDeck::AtTheirExtremes);
    let singular: Vec<&str> = extremes
        .iter()
        .filter(|plate| matches!(plate.kind, mjx_reference_pack::PlateKind::Singular { .. }))
        .map(|plate| plate.token)
        .collect();
    assert_eq!(
        singular,
        vec![
            "circularArrow",
            "leftCircularArrow",
            "leftRightCircularArrow"
        ],
        "the extremes deck's singular plates are not the three MJXOFF-205 measured"
    );

    // And they are singular *because of the extreme*, not always: at their defaults they draw.
    let defaults = plates(PresetDeck::AtTheirDefaults);
    for token in &singular {
        let plate = defaults
            .iter()
            .find(|plate| plate.token == *token)
            .expect("the shape is on the defaults sheet too");
        assert!(
            plate.kind.draws(),
            "`{token}` has no geometry at its *defaults* either, which is a different and much \
             larger problem than a domain endpoint"
        );
    }
}

/// The clamped plates are the callouts and the connectors, and there are twenty-two of them —
/// derived from the sentinel's own value, not from a list of names.
#[test]
fn the_clamped_plates_are_the_callouts_and_the_connectors() {
    let extremes = plates(PresetDeck::AtTheirExtremes);
    let clamped: Vec<&str> = extremes
        .iter()
        .filter(|plate| {
            matches!(
                plate.kind,
                mjx_reference_pack::PlateKind::ClampedExtreme { .. }
            )
        })
        .map(|plate| plate.token)
        .collect();
    assert_eq!(
        clamped.len(),
        22,
        "MJXOFF-205 measured the unbounded set as twenty-two shapes and this run found {}: \
         {clamped:?}",
        clamped.len()
    );
    for token in &clamped {
        assert!(
            token.contains("allout") || token.contains("onnector"),
            "`{token}` has an unbounded handle and is neither a callout nor a connector; \
             MJXOFF-205 measured that set as exactly those twenty-two, so this is either a new \
             shape or a changed domain"
        );
    }
    println!(
        "clamped at {}: {clamped:?}",
        mjx_reference_pack::plates::UNBOUNDED_CLAMP
    );
}
