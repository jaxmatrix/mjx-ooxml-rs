//! **Every public enumeration variant constructed, and every public struct field read** — the first
//! of MJXOFF-155's two standing instruments, applied to this crate.
//!
//! # Why fields and not only variants
//!
//! A variant nothing constructs is the defect this programme keeps finding —
//! `ReferenceAuthority::Provisional` was declared for a whole phase and constructed by nobody, which
//! is why MJXOFF-207's brief names it. But a **field** nothing reads is the same defect one level
//! down and is much easier to write: `HatchReading::tile` could be computed on every run, stored on
//! every row and looked at by nothing at all, and every count in every other suite would still be
//! right.
//!
//! So this file reads every field of every public struct and constructs every variant of every
//! public enumeration, and it does it by *using* them rather than by naming them — a `match` with no
//! wildcard arm is what makes a new variant fail to compile here.
//!
//! # The one that this gate is actually for
//!
//! [`the_provisional_authority_is_constructed_and_reaches_a_report`]. Before this crate,
//! `grep -rn "Provisional" --include="*.rs" crates` returned exactly one hit: the variant's own
//! declaration. It is now produced by a provider, carried on a baseline, listed by
//! `provisional_baselines` and printed in a report, and each of those four steps is asserted
//! separately — because a value that is constructed and then dropped is not much better than one
//! that is never constructed.

use mjx_reference_pack::authority::{
    parity_count, provisional_baselines, Baseline, ReferenceProvider, RenderedContent, Verdict,
};
use mjx_reference_pack::hanging::{paragraphs, HangingRole};
use mjx_reference_pack::ingest::{AdvanceReading, HangingReading, HatchReading, PitchReading};
use mjx_reference_pack::layout::{PixelRect, PlateGeometry, Rect, ToShapeBounds};
use mjx_reference_pack::plates::{plates, PlateKind, PresetDeck};
use mjx_reference_pack::tools::{Raster, WordBox};
use mjx_reference_pack::typography::{baselines, pitch_specimens, probes, swatches, Probe};
use mjx_reference_pack::{SittingItem, ARTEFACTS, THE_SITTING};
use mjx_text::ReferenceAuthority;

#[test]
fn every_reference_provider_and_content_kind_is_constructed_and_asked_every_question() {
    let mut labels = std::collections::BTreeSet::new();
    for provider in ReferenceProvider::ALL {
        // A `match` with no wildcard: a fourth provider fails to compile here.
        let authority = match provider {
            ReferenceProvider::None => ReferenceAuthority::Unverified,
            ReferenceProvider::LibreOffice => ReferenceAuthority::Provisional,
            ReferenceProvider::OfficeExport => ReferenceAuthority::Published,
        };
        assert_eq!(provider.authority(), authority);
        assert_eq!(provider.is_authoritative(), authority.is_evidence());
        assert!(
            labels.insert(provider.label()),
            "two providers share a label"
        );
        for content in RenderedContent::ALL {
            let _ = provider.excludes(content);
        }
    }
    assert_eq!(labels.len(), ReferenceProvider::ALL.len());

    let mut kinds = std::collections::BTreeSet::new();
    for content in RenderedContent::ALL {
        let named = match content {
            RenderedContent::Outline => "outline",
            RenderedContent::SolidFill => "solid fill",
            RenderedContent::GradientFill => "gradient fill",
            RenderedContent::ShadedFill => "shaded fill",
            RenderedContent::PatternFill => "pattern fill",
            RenderedContent::TextLayout => "text layout",
        };
        assert_eq!(content.label(), named);
        assert!(kinds.insert(content.label()));
    }
    assert_eq!(kinds.len(), RenderedContent::ALL.len());
}

#[test]
fn every_verdict_variant_is_constructed_and_every_field_of_it_is_read() {
    let verdicts = [
        Verdict::Agreed {
            differing_fraction: 0.01,
            allowed: 0.02,
        },
        Verdict::Disagreed {
            differing_fraction: 0.5,
            allowed: 0.02,
            worst: "at (3, 4)".to_owned(),
        },
        Verdict::NotEvidence {
            reason: "because".to_owned(),
        },
    ];
    let mut words = std::collections::BTreeSet::new();
    for verdict in &verdicts {
        // Every field, read.
        match verdict {
            Verdict::Agreed {
                differing_fraction,
                allowed,
            } => assert!(differing_fraction <= allowed),
            Verdict::Disagreed {
                differing_fraction,
                allowed,
                worst,
            } => assert!(differing_fraction > allowed && !worst.is_empty()),
            Verdict::NotEvidence { reason } => assert!(!reason.is_empty()),
        }
        assert!(words.insert(verdict.word()));
        // And the printed form carries the numbers, not only the word.
        assert!(format!("{verdict}").len() > verdict.word().len());
    }
    assert_eq!(words.len(), 3, "three verdicts, three words");
}

/// The variant MJXOFF-207 exists to start constructing.
#[test]
fn the_provisional_authority_is_constructed_and_reaches_a_report() {
    // 1. A provider produces it.
    let authority = ReferenceProvider::LibreOffice.authority();
    assert_eq!(authority, ReferenceAuthority::Provisional);
    // 2. A baseline carries it.
    let row = Baseline::new(
        "a plate",
        "a run",
        ReferenceProvider::LibreOffice,
        RenderedContent::Outline,
        Verdict::Agreed {
            differing_fraction: 0.0,
            allowed: 0.02,
        },
    );
    assert_eq!(row.authority, ReferenceAuthority::Provisional);
    assert!(row.is_provisional());
    // 3. The suite can list it — MJXOFF-165's requirement, so a provisional record cannot age into
    //    an unexamined one.
    let report = vec![row.clone()];
    assert_eq!(provisional_baselines(&report).len(), 1);
    assert_eq!(parity_count(&report), 0);
    // 4. And it is printed, so a reader of the report sees it rather than having to know.
    let printed = format!("{row}");
    assert!(
        printed.contains("LibreOffice"),
        "the printed row does not say where its reference came from: {printed}"
    );
}

#[test]
fn every_field_of_a_baseline_is_read() {
    let row = Baseline::new(
        "subject",
        "provenance",
        ReferenceProvider::OfficeExport,
        RenderedContent::SolidFill,
        Verdict::Agreed {
            differing_fraction: 0.0,
            allowed: 0.02,
        },
    );
    let Baseline {
        subject,
        provenance,
        provider,
        authority,
        content,
        verdict,
    } = &row;
    assert_eq!(subject, "subject");
    assert_eq!(provenance, "provenance");
    assert_eq!(*provider, ReferenceProvider::OfficeExport);
    assert_eq!(*authority, ReferenceAuthority::Published);
    assert_eq!(*content, RenderedContent::SolidFill);
    assert!(verdict.is_pass());
    assert!(row.may_be_called_parity());
    assert!(!row.is_provisional());
}

#[test]
fn every_plate_kind_is_reachable_from_the_two_decks() {
    let mut comparable = 0usize;
    let mut no_table = 0usize;
    let mut singular = 0usize;
    let mut clamped = 0usize;
    for which in PresetDeck::ALL {
        for plate in plates(which) {
            // Every field of `Plate`, read.
            assert!(plate.index < 187);
            assert_eq!(plate.token, plate.preset.to_wire());
            assert_eq!(plate.overrides().len(), plate.adjustments.len());
            let _ = plate.page();
            let _ = plate.geometry();
            match &plate.kind {
                PlateKind::Comparable => comparable += 1,
                PlateKind::NoTable => no_table += 1,
                PlateKind::Singular { guide } => {
                    assert!(!guide.is_empty(), "a singular plate names no guide");
                    singular += 1;
                }
                PlateKind::ClampedExtreme { handles } => {
                    assert!(*handles > 0, "a clamped plate clamped nothing");
                    clamped += 1;
                }
            }
            assert_eq!(plate.kind.not_evidence().is_none(), plate.kind.draws());
        }
    }
    // All four reachable, and none of them by one plate alone.
    assert!(comparable > 300, "{comparable}");
    assert_eq!(no_table, 2, "`upArrow`, once per deck");
    assert_eq!(singular, 3, "the three `adj5` shapes, in the extremes deck");
    assert_eq!(clamped, 22, "the callouts and the connectors");
}

#[test]
fn every_layout_type_answers() {
    let geometry = PlateGeometry::at(7);
    let Rect {
        x,
        y,
        width,
        height,
    } = geometry.cell();
    assert!(x >= 0 && y >= 0 && width > 0 && height > 0);
    let window = geometry.window();
    let shape = geometry.shape_box();
    let caption = geometry.caption();
    assert!(
        window.y + window.height <= caption.y,
        "the comparison window overlaps the caption strip, which our side never draws"
    );
    assert!(
        shape.x >= window.x && shape.x + shape.width <= window.x + window.width,
        "the shape box is not inside the window that crops it"
    );
    let bounds = shape.to_shape_bounds();
    assert_eq!(bounds.width_emu, shape.width * 12_700);
    let scene = shape.to_scene_rect();
    assert!(scene.right > scene.left && scene.bottom > scene.top);
    let PixelRect {
        x,
        y,
        width,
        height,
    } = window.to_pixels(96.0, 1280, 720);
    assert!(width > 0 && height > 0 && x < 1280 && y < 720);
    assert!(
        PixelRect {
            x,
            y,
            width,
            height
        }
        .area()
            > 10_000
    );
}

#[test]
fn every_typography_type_answers() {
    let probes = probes();
    let probe = &probes[3];
    let Probe {
        family,
        character,
        page,
        single,
        multiple,
    } = probe;
    assert!(!family.is_empty());
    assert!(probe.single_text().contains(*character));
    assert_eq!(probe.multiple_text().chars().count(), 12);
    assert!(*page < 100 && single.width > 0 && multiple.width > single.width);
    assert!(Probe::advance_per_mille(Some(40.0), Some(35.0)).is_some());
    assert!(Probe::run_advance_per_mille(Some(40.0), None).is_none());

    let baselines = baselines();
    assert_eq!(baselines.len(), 5);
    for baseline in &baselines {
        assert_eq!(baseline.text(), "HH");
        assert!(baseline.bounds.width > 0);
        assert!(probes.iter().any(|probe| probe.family == baseline.family));
        // The baseline must not sit on top of a probe.
        assert!(
            !probes
                .iter()
                .any(|probe| probe.page == baseline.page && probe.single == baseline.bounds),
            "`{}`'s baseline box is in a slot a probe already uses",
            baseline.family
        );
    }

    for specimen in pitch_specimens() {
        assert!(!specimen.family.is_empty() && specimen.bounds.height > 0);
    }
    for swatch in swatches() {
        assert_eq!(swatch.preset.to_wire(), swatch.token);
        assert!(swatch.index < 54 && swatch.window.width > 0);
    }
}

#[test]
fn every_hanging_role_is_used_and_every_reading_field_exists() {
    let mut on = 0usize;
    let mut off = 0usize;
    for paragraph in paragraphs() {
        match paragraph.role {
            HangingRole::JapaneseHanging | HangingRole::LatinHanging => on += 1,
            HangingRole::JapaneseNotHanging => off += 1,
        }
        assert!(!paragraph.role.text().is_empty());
        assert!(!paragraph.role.label().is_empty());
        assert_eq!(
            paragraph.role.overflow_punctuation(),
            !matches!(paragraph.role, HangingRole::JapaneseNotHanging)
        );
    }
    assert_eq!((on, off), (2, 1), "the differential needs both answers");

    // The reading types, field by field, so a field nothing reads fails here.
    let advance = AdvanceReading {
        family: "Arial",
        character: 'a',
        advance_per_mille: Some(556.0),
        run_advance_per_mille: Some(556.0),
        verdict: Verdict::Agreed {
            differing_fraction: 0.0,
            allowed: 0.0,
        },
    };
    assert_eq!(advance.estimates_disagree_by(), Some(0.0));
    let pitch = PitchReading {
        family: "Cambria",
        pitch_per_em: Some(1.17),
        verdict: Verdict::Agreed {
            differing_fraction: 0.0,
            allowed: 0.0,
        },
    };
    assert!(pitch.pitch_per_em.is_some() && pitch.verdict.is_pass() && !pitch.family.is_empty());
    let hanging = HangingReading {
        role: HangingRole::JapaneseHanging,
        measure_points: 400.0,
        lines: 30,
        hangs: vec!['、'],
        verdict: Verdict::Agreed {
            differing_fraction: 0.0,
            allowed: 0.0,
        },
    };
    assert!(
        hanging.measure_points > 0.0
            && hanging.lines > 0
            && !hanging.hangs.is_empty()
            && hanging.verdict.is_pass()
            && hanging.role.overflow_punctuation()
    );
    let hatch = HatchReading {
        index: 3,
        token: "pct25",
        coverage: 0.25,
        mask_coverage: 0.25,
        tile: Some(vec![vec![true, false], vec![false, true]]),
        verdict: Verdict::Agreed {
            differing_fraction: 0.0,
            allowed: 0.0,
        },
    };
    assert_eq!(hatch.index, 3);
    assert_eq!(hatch.token, "pct25");
    assert!((hatch.coverage - hatch.mask_coverage).abs() < 1e-9);
    assert_eq!(hatch.tile.as_ref().map(Vec::len), Some(2));
    assert!(hatch.verdict.is_pass());
}

#[test]
fn every_tools_type_answers() {
    let raster = Raster {
        width: 2,
        height: 1,
        rgb: vec![0, 0, 0, 255, 255, 255],
    };
    assert_eq!(raster.pixel(0, 0), Some([0, 0, 0]));
    assert_eq!(raster.pixel(9, 9), None);
    assert_eq!(raster.ink(), 1, "one black pixel and one white one");

    let word = WordBox {
        page: 1,
        text: "H".to_owned(),
        x_min: 10.0,
        y_min: 10.0,
        x_max: 20.0,
        y_max: 20.0,
    };
    assert!(word.is_inside(Rect::new(0, 0, 100, 100)));
    assert!(!word.is_inside(Rect::new(100, 100, 10, 10)));
    assert_eq!(word.page, 1);
    assert_eq!(word.text, "H");
}

#[test]
fn every_sitting_item_names_an_artefact_the_pack_produces() {
    assert_eq!(THE_SITTING.len(), 7, "seven items, one morning");
    let mut keys = std::collections::BTreeSet::new();
    for item in THE_SITTING {
        let SittingItem {
            key,
            question,
            artefact,
            how_it_is_read,
        } = item;
        assert!(keys.insert(*key), "two items share the key `{key}`");
        assert!(question.len() > 40, "`{key}` asks {question:?}");
        assert!(
            how_it_is_read.len() > 30,
            "`{key}` reads its answer by {how_it_is_read:?}"
        );
        assert!(
            ARTEFACTS.contains(artefact),
            "`{key}` names `{artefact}`, which the pack does not produce"
        );
    }
    // And every artefact answers something, so none is dead weight.
    for artefact in ARTEFACTS {
        assert!(
            THE_SITTING.iter().any(|item| item.artefact == *artefact),
            "`{artefact}` is in the pack and answers no item"
        );
    }
}
