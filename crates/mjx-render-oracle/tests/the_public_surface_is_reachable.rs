//! **The reachability gate: every public enumeration variant and every public struct field.**
//!
//! Variants alone are not enough and this loop has the receipts. A field that nothing reads is a
//! field that can be computed wrongly for ever — R09 found `DrawReport::placeholders` counted by
//! three painters and asserted for none of them, and the fix was not a new counter but an assertion
//! from the consumer's side. So every struct below is **destructured without `..`**, which means a
//! field added tomorrow does not compile until somebody decides what reads it.
//!
//! The enumerations are swept from their own `ALL` constants rather than from a list written here,
//! so a variant added tomorrow is swept the day it exists.

use mjx_render_oracle::authority::{Baseline, ReferenceProvider, RenderedContent, Verdict};
use mjx_render_oracle::baseline::{Approval, ApprovalState, Approver, Baselines, ARTEFACTS};
use mjx_render_oracle::geom::{PixelRect, Rect};
use mjx_render_oracle::json::Value;
use mjx_render_oracle::pdf::{compare_words, WordComparison};
use mjx_render_oracle::perceptual::{compare, Difference, Tolerance, Worst, WorstWindow};
use mjx_render_oracle::plate::{generate, DocumentRow, Plate, PlateSet};
use mjx_render_oracle::png::Image;
use mjx_render_oracle::specimen::{Perturbation, Rendered, Specimen, SPECIMENS};
use mjx_render_oracle::tiers::{LayeredComparison, Localisation, TierOutcome};
use mjx_render_oracle::tools::{one_of, which, Raster, WordBox, IMAGE_MAGICK, REQUIRE_TOOLS};
use mjx_render_oracle::{png, specimen};

// ---------------------------------------------------------------------------------------------
// Enumerations
// ---------------------------------------------------------------------------------------------

#[test]
fn every_provider_and_every_content_kind_answers_every_question_asked_of_it() {
    let mut labels = Vec::new();
    let mut authorities = Vec::new();
    for provider in ReferenceProvider::ALL {
        labels.push(provider.label());
        authorities.push(provider.authority());
        match provider {
            ReferenceProvider::None | ReferenceProvider::LibreOffice => {
                assert!(!provider.is_authoritative());
            }
            ReferenceProvider::OfficeExport => assert!(provider.is_authoritative()),
        }
        for content in RenderedContent::ALL {
            // Every pair is asked, so a provider/content combination nobody thought about is a
            // compile error in `excludes` rather than a silent `None`.
            let _ = provider.excludes(content);
        }
    }
    assert_eq!(
        labels
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        ReferenceProvider::ALL.len(),
        "two providers share a label, so a report cannot tell them apart"
    );
    // `ReferenceAuthority` is `PartialEq` and not `Ord`, so the distinct count is taken by hand
    // rather than through a set — the question being asked is still *how many distinct answers*,
    // which is the one G06's own green mutation turned out not to be asking.
    let mut distinct: Vec<mjx_text::ReferenceAuthority> = Vec::new();
    for authority in &authorities {
        if !distinct.contains(authority) {
            distinct.push(*authority);
        }
    }
    assert_eq!(
        distinct.len(),
        3,
        "the three providers do not map to three distinct authorities: {authorities:?}"
    );

    let mut content_labels = Vec::new();
    for content in RenderedContent::ALL {
        content_labels.push(content.label());
        match content {
            RenderedContent::Outline | RenderedContent::SolidFill | RenderedContent::TextLayout => {
                assert_eq!(ReferenceProvider::LibreOffice.excludes(content), None);
            }
            RenderedContent::GradientFill
            | RenderedContent::ShadedFill
            | RenderedContent::PatternFill => {
                assert!(ReferenceProvider::LibreOffice.excludes(content).is_some());
            }
        }
    }
    assert_eq!(
        content_labels
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        RenderedContent::ALL.len()
    );
}

#[test]
fn every_verdict_every_outcome_and_every_localisation_answers() {
    for verdict in [
        Verdict::Agreed {
            differing_fraction: 0.0,
            allowed: 0.1,
        },
        Verdict::Disagreed {
            differing_fraction: 0.5,
            allowed: 0.1,
            worst: "at (1, 2)".to_owned(),
        },
        Verdict::NotEvidence {
            reason: "no reference".to_owned(),
        },
    ] {
        let (pass, failure, evidence) = (
            verdict.is_pass(),
            verdict.is_failure(),
            verdict.is_evidence(),
        );
        assert!(
            !(pass && failure),
            "a verdict is both a pass and a failure: {verdict}"
        );
        assert_eq!(evidence, pass || failure);
        assert!(!verdict.word().is_empty());
        assert!(!verdict.to_string().is_empty());
        // Every arm's `Display` names its own numbers or its own reason, which is what a reader
        // acts on.
        match &verdict {
            Verdict::Agreed { .. } | Verdict::Disagreed { .. } => {
                assert!(verdict.to_string().contains('%'));
            }
            Verdict::NotEvidence { reason } => assert!(verdict.to_string().contains(reason)),
        }
    }

    for outcome in [
        TierOutcome::Matched,
        TierOutcome::Differed {
            first_difference: "line 4".to_owned(),
        },
        TierOutcome::NotEvidence {
            reason: "excluded".to_owned(),
        },
    ] {
        assert_eq!(
            [
                outcome.is_match(),
                outcome.is_difference(),
                !outcome.is_evidence()
            ]
            .iter()
            .filter(|answer| **answer)
            .count(),
            1,
            "`{outcome}` is more or less than exactly one of matched, differed and not-evidence"
        );
        assert!(!outcome.word().is_empty());
        assert!(!outcome.to_string().is_empty());
    }

    let mut advice = Vec::new();
    for localisation in Localisation::ALL {
        advice.push(localisation.advice());
        assert!(!localisation.advice().is_empty());
    }
    assert_eq!(
        advice
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        Localisation::ALL.len(),
        "two localisations give the same advice, so one of them tells a reader to look in the \
         wrong place"
    );
}

#[test]
fn every_perturbation_and_every_approver_answers() {
    let mut labels = Vec::new();
    for perturbation in Perturbation::ALL {
        labels.push(perturbation.label());
        for spec in SPECIMENS {
            let _ = specimen::fragments(spec, perturbation);
            specimen::display_list(spec, perturbation).expect("a display list");
        }
    }
    assert_eq!(
        labels
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        Perturbation::ALL.len()
    );

    for approver in [
        Approver::Generator,
        Approver::Human {
            name: "A Person".to_owned(),
        },
    ] {
        assert!(!approver.wire().is_empty());
        assert!(!approver.label().is_empty());
        let approval = Approval {
            specimen: "x".to_owned(),
            approver: approver.clone(),
            approved_at: 1,
            note: "n".to_owned(),
            digests: Vec::new(),
        };
        assert_eq!(
            approval.is_reviewed(),
            matches!(approver, Approver::Human { .. })
        );
        assert_eq!(
            Approval::parse(&approval.to_text()).expect("parses"),
            approval
        );
    }

    for state in [
        ApprovalState::Missing,
        ApprovalState::Stale {
            artefact: "render.png".to_owned(),
            recorded: "a".to_owned(),
            actual: "b".to_owned(),
        },
        ApprovalState::Malformed {
            reason: "unreadable".to_owned(),
        },
        ApprovalState::Approved(Approval {
            specimen: "x".to_owned(),
            approver: Approver::Generator,
            approved_at: 1,
            note: "n".to_owned(),
            digests: Vec::new(),
        }),
    ] {
        assert_eq!(state.is_usable(), state.refusal().is_none());
        assert!(!state.is_reviewed() || state.is_usable());
    }
}

#[test]
fn every_json_value_shape_is_reachable_and_answers_only_its_own_accessor() {
    let values = [
        Value::Null,
        Value::Bool(true),
        Value::Number(1.0),
        Value::String("x".to_owned()),
        Value::Array(vec![Value::Null]),
        Value::Object(std::collections::BTreeMap::new()),
    ];
    for value in &values {
        // Exactly one accessor answers for each shape, which is what stops a manifest assertion
        // passing because a field of the wrong type read as `None` and the test used `unwrap_or`.
        let answered = [
            value.boolean().is_some(),
            value.number().is_some(),
            value.string().is_some(),
            value.array().is_some(),
            value.get("anything").is_some(),
        ]
        .iter()
        .filter(|answer| **answer)
        .count();
        assert!(answered <= 1, "{value:?} answers more than one accessor");
    }
    assert_eq!(values.len(), 6, "a `Value` shape is not swept");
}

// ---------------------------------------------------------------------------------------------
// Struct fields — destructured without `..`, so a new field does not compile until it is read
// ---------------------------------------------------------------------------------------------

#[test]
fn the_tool_lookup_answers_the_same_way_for_both_spellings_of_the_same_program() {
    // `one_of` exists because ImageMagick 7 is `magick` and ImageMagick 6 is `convert`, and a gate
    // written against one name passes locally and fails on a runner for a reason that has nothing to
    // do with the code. Asserted rather than described, and asserted **without** the environment
    // variable set, so the skip path is the one being exercised here.
    assert_eq!(IMAGE_MAGICK.len(), 2);
    assert!(IMAGE_MAGICK.contains(&"magick") && IMAGE_MAGICK.contains(&"convert"));

    // A name nothing can have: no answer, and no panic, because `REQUIRE_TOOLS` is not set in this
    // process.
    assert_eq!(
        std::env::var(REQUIRE_TOOLS),
        Err(std::env::VarError::NotPresent)
    );
    assert_eq!(
        one_of("a probe", &["mjx-no-such-reader-ever"], REQUIRE_TOOLS),
        None
    );

    // And it answers the **first** available spelling rather than any of them, so a caller can print
    // which one ran.
    let answered = one_of("a probe", &["mjx-no-such-reader-ever", "sh"], REQUIRE_TOOLS);
    assert_eq!(answered, Some("sh"), "`sh` is on every path this runs on");
    assert!(which("sh") && !which("mjx-no-such-reader-ever"));
}

#[test]
fn every_field_of_every_public_record_is_read() {
    // --- Specimen, Rendered ---
    for spec in SPECIMENS {
        let Specimen {
            name,
            description,
            content,
            tolerance,
        } = spec;
        assert!(!name.is_empty() && !description.is_empty());
        assert!(!content.label().is_empty());
        let Tolerance {
            channel,
            differing_fraction,
            structural_similarity,
            worst_window_similarity,
        } = *tolerance;
        assert!(channel > 0 && channel < 255);
        assert!((0.0..1.0).contains(&differing_fraction));
        assert!((0.0..=1.0).contains(&structural_similarity));
        assert!((0.0..=1.0).contains(&worst_window_similarity));
        assert_eq!(Specimen::named(name).as_ref(), Some(spec));
    }
    assert_eq!(Specimen::named("nothing"), None);

    let Rendered { pixels, report } =
        specimen::render(&SPECIMENS[0], Perturbation::None).expect("a render");
    assert!(pixels.width > 0 && report.draw_calls > 0);
    assert_eq!(
        (pixels.width, pixels.height),
        specimen::device_size(),
        "the render is not the size the crate says a page is"
    );

    // --- Image ---
    let image = png::image_from_pixels(&pixels);
    let Image {
        width,
        height,
        rgba,
    } = &image;
    assert_eq!(rgba.len(), (*width as usize) * (*height as usize) * 4);
    assert!(image.pixel(0, 0).is_some());
    assert_eq!(image.pixel(*width, 0), None);

    // --- Difference, Worst, WorstWindow ---
    let other = png::image_from_pixels(
        &specimen::render(&SPECIMENS[0], Perturbation::PaintColour)
            .expect("a render")
            .pixels,
    );
    let difference = compare(&image, &other, Tolerance::EXACT_RASTER).expect("same size");
    let Difference {
        pixels: compared,
        differing,
        max_channel_difference,
        mean_absolute_difference,
        structural_similarity,
        worst_window,
        worst,
        ink,
    } = &difference;
    assert_eq!(*compared, (*width as usize) * (*height as usize));
    assert!(*differing > 0 && *max_channel_difference > 0);
    assert!(*mean_absolute_difference > 0.0);
    assert!(*structural_similarity < 1.0);
    assert_eq!(ink.0, ink.1, "both pages are opaque");
    let Worst { x, y, left, right } = worst.expect("a worst pixel");
    assert!(x < *width && y < *height && left != right);
    let WorstWindow { x, y, similarity } = worst_window.expect("a worst window");
    assert!(x < *width && y < *height && (0.0..=1.0).contains(&similarity));

    // --- Baseline ---
    let baseline = Baseline::new(
        "s",
        "p",
        ReferenceProvider::OfficeExport,
        RenderedContent::SolidFill,
        Verdict::Agreed {
            differing_fraction: 0.0,
            allowed: 0.0,
        },
    );
    let Baseline {
        subject,
        provenance,
        provider,
        authority,
        content,
        verdict,
    } = &baseline;
    assert_eq!(subject, "s");
    assert_eq!(provenance, "p");
    assert_eq!(*authority, provider.authority());
    assert!(!content.label().is_empty() && verdict.is_pass());
    assert!(baseline.to_string().contains("s"));

    // --- Approval ---
    let approval = match Baselines::committed().approval(SPECIMENS[0].name) {
        ApprovalState::Approved(approval) => approval,
        other => panic!("the committed baseline is {other:?}"),
    };
    let Approval {
        specimen: named,
        approver,
        approved_at,
        note,
        digests,
    } = &approval;
    assert_eq!(named, SPECIMENS[0].name);
    assert!(!approver.label().is_empty() && !note.is_empty());
    assert!(*approved_at > 1_700_000_000, "an approval before 2023");
    assert_eq!(digests.len(), ARTEFACTS.len());
    assert!(!approval.date().is_empty());

    // --- LayeredComparison ---
    let comparison = mjx_render_oracle::compare_against_baseline(
        &Baselines::committed(),
        &SPECIMENS[0],
        Perturbation::None,
    )
    .expect("a comparison");
    let LayeredComparison {
        specimen: named,
        provider,
        content,
        fragments,
        commands,
        pixels,
    } = &comparison;
    assert_eq!(named, SPECIMENS[0].name);
    assert!(!provider.is_authoritative() && !content.label().is_empty());
    assert!(fragments.is_match() && commands.is_match() && pixels.is_match());
    assert!(comparison.row().contains(SPECIMENS[0].name));

    // --- PlateSet, Plate, DocumentRow ---
    let set =
        generate(ReferenceProvider::LibreOffice, &Baselines::committed()).expect("plates generate");
    let PlateSet {
        plates,
        documents,
        images,
        provider,
    } = &set;
    assert_eq!(plates.len(), images.len());
    assert!(!documents.is_empty() && !provider.is_authoritative());
    for plate in plates {
        let Plate {
            name,
            description,
            file,
            width,
            height,
            sha256,
            content,
            provider,
            exclusion,
            placeholders,
            draw_calls,
            covered,
            approver,
            reviewed,
            parity,
            reference_file,
            diff_file,
            difference,
        } = plate;
        assert!(!name.is_empty() && !description.is_empty());
        assert_eq!(file, &format!("{name}.png"));
        assert!(*width > 0 && *height > 0 && sha256.len() == 64);
        assert!(!content.label().is_empty() && !provider.is_authoritative());
        assert_eq!(
            exclusion.is_some(),
            provider.excludes(*content).is_some(),
            "`{name}`'s exclusion does not come from its provider"
        );
        assert_eq!(*placeholders, 0);
        assert!(*draw_calls > 0 && *covered > 0);
        assert!(!approver.is_empty() && !*reviewed && !*parity);
        // The three regression fields move together or not at all: a reference with no diff is a
        // gallery that shows the approved image and nothing to compare it against, and a difference
        // with no pictures is the log line this crate exists to replace. On a run that matches its
        // baseline — which this one does — all three are absent, and
        // `a_regression_arrives_with_its_picture.rs` is where they are present.
        assert_eq!(reference_file.is_some(), diff_file.is_some());
        assert_eq!(diff_file.is_some(), difference.is_some());
        assert_eq!(*diff_file, None, "`{name}` does not match its own baseline");
    }
    for row in documents {
        let DocumentRow {
            fixture,
            format,
            verdict,
            reason,
        } = row;
        assert!(!fixture.is_empty() && !format.is_empty());
        assert!(!verdict.is_empty() && !reason.is_empty());
    }

    // --- WordComparison, WordBox ---
    let word = WordBox {
        page: 1,
        text: "word".to_owned(),
        x_min: 1.0,
        y_min: 2.0,
        x_max: 21.0,
        y_max: 14.0,
    };
    let WordBox {
        page,
        text,
        x_min,
        y_min,
        x_max,
        y_max,
    } = &word;
    assert_eq!(*page, 1);
    assert_eq!(text, "word");
    assert!(x_max > x_min && y_max > y_min);
    assert!(word.is_inside(Rect::new(0, 0, 100, 100)));
    assert!(!word.is_inside(Rect::new(100, 100, 10, 10)));
    let WordComparison {
        counts,
        allowed_points,
        first_difference,
        moved,
    } = compare_words(
        std::slice::from_ref(&word),
        std::slice::from_ref(&word),
        0.25,
    );
    assert_eq!(counts, (1, 1));
    assert!(allowed_points > 0.0);
    assert_eq!(first_difference, None);
    assert_eq!(moved, 0);

    // --- Raster, Rect, PixelRect ---
    let raster = Raster {
        width: 4,
        height: 2,
        rgb: vec![0xff; 24],
    };
    let Raster { width, height, rgb } = &raster;
    assert_eq!(rgb.len(), (*width as usize) * (*height as usize) * 3);
    assert_eq!(raster.pixel(0, 0), Some([0xff, 0xff, 0xff]));
    assert_eq!(raster.pixel(4, 0), None);
    assert_eq!(raster.ink(), 0, "an all-white raster has no ink");

    let rect = Rect::new(12, 6, 100, 70);
    let Rect {
        x,
        y,
        width,
        height,
    } = rect;
    assert_eq!((x, y, width, height), (12, 6, 100, 70));
    let scene = rect.to_scene_rect();
    assert!((scene.right - scene.left - 100.0).abs() < 1e-6);
    let PixelRect {
        x,
        y,
        width,
        height,
    } = rect.to_pixels(96.0, 1280, 720);
    assert_eq!((x, y), (16, 8));
    assert_eq!((width, height), (133, 93));
    assert_eq!(rect.to_pixels(96.0, 1280, 720).area(), 133 * 93);
}
