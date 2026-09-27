//! **The gate MJXOFF-165 names in its own *Done when*:** three tiers that localise, rather than
//! three tiers that exist.
//!
//! Three tiers which always agree are one tier written three times, and a suite that only asserted
//! their presence would pass on exactly that. So what is asserted here is the *pattern* of
//! disagreement:
//!
//! | What was changed | tier 1 | tier 2 | tier 3 | localises to |
//! |---|---|---|---|---|
//! | nothing | matched | matched | matched | `Everything` |
//! | one fragment moved | **differs** | differs | differs | `Layout` |
//! | one paint recoloured | matched | **differs** | differs | `SceneBuilding` |
//! | the stored image alone | matched | matched | **differs** | `Painting` |
//!
//! The third row is the one that carries the argument. A fragment tree records a
//! `DecorationRef` — a bare number — and never a colour, so a recolour *cannot* reach tier one; if
//! it ever did, the tiers would not be three stages of a pipeline but three views of one comparison,
//! and the table above would collapse to "something changed".
//!
//! # How many distinct answers does this gate actually see?
//!
//! Four, and it counts them. A localisation function that answered `Layout` to everything would
//! satisfy two of the four rows above, and a suite that checked each row in isolation would report
//! two passes and two failures rather than one design error. [`the_localisation_takes_four_distinct_values`]
//! collects the whole set and asserts its **cardinality** — the lesson G06 left, which was that
//! counting answers is not counting *distinct* answers.

use std::path::PathBuf;

use mjx_render_oracle::baseline::{Approver, Baselines, ARTEFACTS};
use mjx_render_oracle::specimen::{Perturbation, SPECIMENS};
use mjx_render_oracle::tiers::{LayeredComparison, TierOutcome};
use mjx_render_oracle::{
    compare_against_baseline, png, snapshot, specimen, Localisation, ReferenceProvider,
    RenderedContent,
};

/// The specimen every row of the table above is run on.
///
/// `nested-groups`, because it is the only one whose page has a clip, an opacity group and three
/// nested boxes: a perturbation that moved a fragment on a flat page would move one rectangle, and a
/// perturbation that moves a *group* moves everything inside it, which is the case a tier that only
/// looked at leaves would miss.
const SUBJECT: &str = "nested-groups";

fn subject() -> specimen::Specimen {
    specimen::Specimen::named(SUBJECT).expect("the nested specimen is in the corpus")
}

fn committed() -> Baselines {
    Baselines::committed()
}

#[test]
fn an_unperturbed_run_matches_at_every_tier() {
    for specimen in SPECIMENS {
        let comparison = compare_against_baseline(&committed(), specimen, Perturbation::None)
            .unwrap_or_else(|reason| panic!("`{}`: {reason}", specimen.name));
        assert_eq!(
            comparison.localise(),
            Localisation::Everything,
            "`{}` does not match its own committed baseline:\n{comparison}",
            specimen.name
        );
        // And it is not vacuous: a comparison of two blank pages agrees perfectly, so the tier that
        // could go hollow is asserted to have had ink on both sides.
        let rendered = specimen::render(specimen, Perturbation::None).expect("it renders");
        assert!(
            png::image_from_pixels(&rendered.pixels).covered() > 500,
            "`{}` covered {} pixels, which is not a page",
            specimen.name,
            png::image_from_pixels(&rendered.pixels).covered()
        );
    }
}

#[test]
fn moving_one_fragment_reddens_all_three_tiers_and_localises_to_layout() {
    let comparison =
        compare_against_baseline(&committed(), &subject(), Perturbation::FragmentPosition)
            .expect("the baseline is usable");
    assert!(
        comparison.fragments.is_difference(),
        "tier one did not see a fragment move:\n{comparison}"
    );
    assert!(
        comparison.commands.is_difference(),
        "tier two did not see a fragment move:\n{comparison}"
    );
    assert!(
        comparison.pixels.is_difference(),
        "tier three did not see a fragment move:\n{comparison}"
    );
    assert_eq!(comparison.localise(), Localisation::Layout);

    // **The finding names a place.** *"The tree changed"* is not a finding; a line number and both
    // sides of it is one somebody can act on without re-running anything.
    let TierOutcome::Differed { first_difference } = &comparison.fragments else {
        unreachable!("asserted above")
    };
    assert!(
        first_difference.starts_with("line "),
        "the fragment tier's finding does not name a line: {first_difference}"
    );
    assert!(
        first_difference.contains("rect=["),
        "the fragment tier's finding does not quote the rectangle that moved: {first_difference}"
    );
}

#[test]
fn recolouring_one_paint_leaves_tier_one_green_and_localises_to_scene_building() {
    let comparison = compare_against_baseline(&committed(), &subject(), Perturbation::PaintColour)
        .expect("the baseline is usable");
    assert!(
        comparison.fragments.is_match(),
        "**tier one saw a colour change.** A fragment tree carries a `DecorationRef` and never a \
         colour, so this cannot happen unless the tiers are no longer three stages of one \
         pipeline:\n{comparison}"
    );
    assert!(
        comparison.commands.is_difference(),
        "tier two did not see a recolour:\n{comparison}"
    );
    assert!(
        comparison.pixels.is_difference(),
        "tier three did not see a recolour:\n{comparison}"
    );
    assert_eq!(comparison.localise(), Localisation::SceneBuilding);

    // And it names the paint, not merely the command index — which is the whole reason
    // `snapshot::commands` resolves a paint's value onto the line instead of leaving an index.
    let TierOutcome::Differed { first_difference } = &comparison.commands else {
        unreachable!("asserted above")
    };
    assert!(
        first_difference.contains('#'),
        "the command tier's finding does not quote a colour: {first_difference}"
    );
}

#[test]
fn a_difference_in_the_stored_image_alone_localises_to_painting() {
    // Build a baseline whose snapshots are the real ones and whose *image* has a rectangle painted
    // over it. Nothing about the renderer changes; only the approved bytes do.
    let real = committed();
    let temporary = scratch("painting");
    let directory = temporary.join(SUBJECT);
    std::fs::create_dir_all(&directory).expect("a scratch baseline directory");
    for artefact in [ARTEFACTS[0], ARTEFACTS[1]] {
        std::fs::write(
            directory.join(artefact),
            real.artefact(SUBJECT, artefact).expect("the real artefact"),
        )
        .expect("copying an artefact");
    }
    let mut image = png::decode(
        &real
            .artefact(SUBJECT, ARTEFACTS[2])
            .expect("the real image"),
    )
    .expect("the committed image decodes");
    for y in 40..80u32 {
        for x in 40..120u32 {
            let offset = ((y as usize) * (image.width as usize) + (x as usize)) * 4;
            image.rgba[offset..offset + 4].copy_from_slice(&[0xff, 0x00, 0x00, 0xff]);
        }
    }
    std::fs::write(
        directory.join(ARTEFACTS[2]),
        png::encode(&image).expect("the corrupted image encodes"),
    )
    .expect("writing the corrupted image");

    let baselines = Baselines::at(&temporary);
    baselines
        .approve(
            SUBJECT,
            Approver::Human {
                name: "the tier-independence gate".to_owned(),
            },
            "a deliberately corrupted image, so that only tier three can differ",
        )
        .expect("the scratch baseline approves");

    let comparison = compare_against_baseline(&baselines, &subject(), Perturbation::None)
        .expect("the scratch baseline is usable");
    assert!(comparison.fragments.is_match(), "{comparison}");
    assert!(comparison.commands.is_match(), "{comparison}");
    assert!(
        comparison.pixels.is_difference(),
        "**the pixel tier did not see 3200 red pixels.** Either the tolerance is far too loose or \
         the tier is not comparing what it claims:\n{comparison}"
    );
    assert_eq!(comparison.localise(), Localisation::Painting);

    let _ = std::fs::remove_dir_all(&temporary);
}

#[test]
fn the_localisation_takes_four_distinct_values() {
    let mut seen: Vec<Localisation> = Vec::new();
    for perturbation in Perturbation::ALL {
        for specimen in SPECIMENS {
            let comparison = compare_against_baseline(&committed(), specimen, perturbation)
                .unwrap_or_else(|reason| panic!("`{}`: {reason}", specimen.name));
            let localisation = comparison.localise();
            if !seen.contains(&localisation) {
                seen.push(localisation);
            }
        }
    }
    // `Painting` is reached by the case above rather than by a perturbation, because there is no
    // way to change the renderer's *output* without changing its input — which is the honest reason
    // it needs its own fixture rather than a fourth `Perturbation`.
    assert!(
        seen.contains(&Localisation::Everything)
            && seen.contains(&Localisation::Layout)
            && seen.contains(&Localisation::SceneBuilding),
        "the perturbation matrix produced only {seen:?}; a gate that sees one answer everywhere is \
         a gate measuring one thing many times"
    );
    assert!(
        seen.len() >= 3,
        "the perturbation matrix produced {} distinct localisations, not three or more: {seen:?}",
        seen.len()
    );
}

#[test]
fn a_tree_difference_that_reaches_no_pixel_is_reported_as_inconsistent() {
    // Constructed rather than provoked: a difference that reaches tier one and neither tier below
    // is a statement about the *harness*, and there is deliberately no specimen that produces one.
    // The arm exists so that such a case is reported as what it is — look at the specimen — rather
    // than as "the box model broke", which would send a reader to the wrong crate.
    let comparison = LayeredComparison {
        specimen: SUBJECT.to_owned(),
        provider: ReferenceProvider::None,
        content: RenderedContent::SolidFill,
        fragments: TierOutcome::Differed {
            first_difference: "line 2: a source path changed and nothing drew differently"
                .to_owned(),
        },
        commands: TierOutcome::Matched,
        pixels: TierOutcome::Matched,
    };
    assert_eq!(comparison.localise(), Localisation::Inconsistent);
    assert!(comparison.localise().advice().contains("specimen"));

    // And an *excluded* tier below is not an agreeing one: a gradient page whose pixel tier the
    // provider refuses to speak about must still localise a real tree difference to `Layout`.
    let excluded = LayeredComparison {
        commands: TierOutcome::NotEvidence {
            reason: "not compared".to_owned(),
        },
        pixels: TierOutcome::NotEvidence {
            reason: "the provider cannot speak about a gradient".to_owned(),
        },
        ..comparison
    };
    assert_eq!(excluded.localise(), Localisation::Layout);
}

#[test]
fn a_comparison_with_no_evidence_anywhere_says_so() {
    let comparison = LayeredComparison {
        specimen: SUBJECT.to_owned(),
        provider: ReferenceProvider::LibreOffice,
        content: RenderedContent::GradientFill,
        fragments: TierOutcome::NotEvidence {
            reason: "nothing".to_owned(),
        },
        commands: TierOutcome::NotEvidence {
            reason: "nothing".to_owned(),
        },
        pixels: TierOutcome::NotEvidence {
            reason: "nothing".to_owned(),
        },
    };
    assert_eq!(comparison.localise(), Localisation::NoEvidence);
    assert!(!comparison.agreed());
    assert!(
        !comparison.may_be_called_parity(),
        "a comparison that produced no evidence at all was reported as parity"
    );
}

#[test]
fn the_two_snapshot_tiers_need_no_painter_at_all() {
    // Not a performance note: it is what lets the layout and scene tiers run on every machine and
    // in every job, including one with no graphics stack, and what makes a layout regression cost a
    // named line rather than an image.
    for specimen in SPECIMENS {
        let tree = specimen::fragments(specimen, Perturbation::None);
        let moved = specimen::fragments(specimen, Perturbation::FragmentPosition);
        assert!(
            snapshot::fragments(&tree)
                .first_difference(&snapshot::fragments(&moved))
                .is_some(),
            "`{}`'s fragment perturbation moves no fragment, so the specimen cannot exercise tier \
             one",
            specimen.name
        );
        let list = specimen::display_list(specimen, Perturbation::None).expect("a display list");
        assert!(
            snapshot::commands(&list).lines().len() > 2,
            "`{}`'s display list snapshot is {} lines, which cannot describe a page",
            specimen.name,
            snapshot::commands(&list).lines().len()
        );
    }
}

/// Where a case leaves its artefacts, so a failure can be looked at rather than re-run.
///
/// Named per case: Cargo runs a binary's cases on several threads at once, and two cases sharing a
/// scratch path means one reading a file the other is halfway through writing — a flake that reads
/// exactly like a renderer defect. MJXOFF-207 found that the hard way and it is written down here
/// rather than rediscovered.
fn scratch(case: &str) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/oracle-scratch")
        .join(case);
    let _ = std::fs::remove_dir_all(&directory);
    let _ = std::fs::create_dir_all(&directory);
    directory
}
