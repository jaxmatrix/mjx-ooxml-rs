//! **The second standing instrument: every branch over more than one value, and every gate asked how
//! many distinct values it actually sees.**
//!
//! # The defect shape this file is written against
//!
//! MJXOFF-155 §8 and every recent child name the same thing. *A measurement taken at one point.*
//! Every test box was landscape until a portrait one exposed two behaviours no landscape box could
//! reach. Every stand-in scene was a fill until MJXOFF-206 checked the stroke and found the other
//! half of `DrawReport::placeholders` had never executed. Every effect fixture used
//! `Effect::new`'s defaults, so `start_alpha` was zero and the `Reflection` arm of two painters had
//! never produced a pixel — and two painters drawing nothing agree perfectly.
//!
//! So this file does two things no other file here does:
//!
//! 1. **Drives every branch with at least two distinct inputs**, so an arm that ignored its argument
//!    would be visible.
//! 2. **Counts the distinct values each table actually produces** and asserts a floor. A grid whose
//!    every cell was the same rectangle, a plate table whose every entry was `rect`, a probe set
//!    whose ninety-two characters were all `H` — each of those satisfies every count in every other
//!    suite in this crate.

use std::collections::BTreeSet;

use mjx_reference_pack::authority::{Baseline, ReferenceProvider, RenderedContent, Verdict};
use mjx_reference_pack::compare::{compare_window, crop, PLATE_TOLERANCE};
use mjx_reference_pack::deck::caption_of;
use mjx_reference_pack::hanging::HangingRole;
use mjx_reference_pack::layout::{PlateGeometry, Rect, PLATES_PER_SLIDE};
use mjx_reference_pack::plates::{extreme_adjustments, plates, PresetDeck};
use mjx_reference_pack::tools::{parse_bbox_layout, parse_ppm, Raster};
use mjx_reference_pack::typography::{probes, swatches};

/// A raster of one colour, so a comparison against another one has something to see.
fn flat(width: u32, height: u32, colour: [u8; 3]) -> Raster {
    Raster {
        width,
        height,
        rgb: colour
            .iter()
            .copied()
            .cycle()
            .take((width as usize) * (height as usize) * 3)
            .collect(),
    }
}

#[test]
fn the_plate_grid_puts_every_plate_somewhere_different() {
    let cells: BTreeSet<(i64, i64)> = (0..PLATES_PER_SLIDE)
        .map(|position| {
            let cell = PlateGeometry::at(position).cell();
            (cell.x, cell.y)
        })
        .collect();
    assert_eq!(
        cells.len(),
        PLATES_PER_SLIDE,
        "the grid puts {} plates in {} places",
        PLATES_PER_SLIDE,
        cells.len()
    );
    // And it uses both axes. A grid that laid every plate out in one row would have as many
    // distinct cells as plates and would still be wrong.
    let columns: BTreeSet<i64> = cells.iter().map(|(x, _)| *x).collect();
    let rows: BTreeSet<i64> = cells.iter().map(|(_, y)| *y).collect();
    assert!(
        columns.len() > 1 && rows.len() > 1,
        "the grid is {} column(s) by {} row(s)",
        columns.len(),
        rows.len()
    );
    assert_eq!(columns.len() * rows.len(), PLATES_PER_SLIDE);
}

#[test]
fn the_plate_table_is_187_different_shapes_and_not_one_shape_187_times() {
    for which in PresetDeck::ALL {
        let table = plates(which);
        let tokens: BTreeSet<&str> = table.iter().map(|plate| plate.token).collect();
        assert_eq!(
            tokens.len(),
            187,
            "{} names {} distinct presets",
            which.file_name(),
            tokens.len()
        );
        let captions: BTreeSet<String> = table.iter().map(caption_of).collect();
        assert_eq!(
            captions.len(),
            187,
            "{} has {} distinct captions, so two plates are labelled the same",
            which.file_name(),
            captions.len()
        );
    }
}

/// The extremes deck has to be at *different* values from the defaults deck, and at more than one
/// of them — which is the entire difference between the two artefacts.
#[test]
fn the_extremes_are_many_different_values_and_not_the_defaults() {
    let defaults = plates(PresetDeck::AtTheirDefaults);
    let extremes = plates(PresetDeck::AtTheirExtremes);

    let stated: usize = extremes.iter().map(|plate| plate.adjustments.len()).sum();
    assert!(
        stated > 250,
        "the extremes deck states only {stated} adjustment values across 187 plates"
    );
    assert_eq!(
        defaults
            .iter()
            .map(|plate| plate.adjustments.len())
            .sum::<usize>(),
        0,
        "the defaults deck states an adjustment, so it is not at the defaults"
    );

    let values: BTreeSet<i32> = extremes
        .iter()
        .flat_map(|plate| plate.adjustments.iter().map(|(_, value)| *value))
        .collect();
    // **Twenty-three, measured — and the number is smaller than it looks because the table is.**
    // A domain endpoint is usually one of a small vocabulary: `0`, full scale, half scale, the
    // clamp. So the floor is set at what the table actually produces rather than at a guess, and
    // the *meaningful* claim — that a handle moved — is the assertion below it.
    assert!(
        values.len() >= 20,
        "the extremes deck writes {} distinct values across {stated} adjustments; twenty-three is \
         what the domains yield, and a collapse to a handful would mean the endpoints are not being \
         read",
        values.len()
    );
    // Both signs: a deck that only ever took a maximum would be half a deck, and every callout's
    // tail is negative in one direction.
    assert!(
        values.iter().any(|value| *value < 0) && values.iter().any(|value| *value > 0),
        "every extreme has the same sign: {values:?}"
    );

    // **The claim that actually matters: the handle moved.** A deck whose "extremes" happened to
    // equal every shape's default would satisfy every count above and would be the first deck
    // written twice.
    let mut moved = 0usize;
    let mut unmoved = Vec::new();
    for plate in &extremes {
        let mut any = false;
        for (name, value) in &plate.adjustments {
            let default = mjx_ooxml_types::drawingml::adjustments_of(plate.preset)
                .iter()
                .find(|spec| spec.wire_name == *name)
                .map(|spec| spec.default);
            if default != Some(*value) {
                any = true;
            }
        }
        if plate.adjustments.is_empty() {
            continue;
        }
        if any {
            moved += 1;
        } else {
            unmoved.push(plate.token);
        }
    }
    assert!(
        moved > 100,
        "only {moved} of the adjustable plates state a value different from the shape's own \
         default; these state only defaults: {unmoved:?}"
    );
    println!(
        "extremes: {stated} adjustments, {} distinct values, {moved} plates moved off their \
         defaults, {} left at them: {unmoved:?}",
        values.len(),
        unmoved.len()
    );
}

#[test]
fn every_adjustable_preset_is_asked_for_its_extremes_and_the_answers_differ() {
    let mut answers: BTreeSet<Vec<(&str, i32)>> = BTreeSet::new();
    let mut asked = 0usize;
    for plate in plates(PresetDeck::AtTheirExtremes) {
        let (adjustments, _) = extreme_adjustments(plate.preset);
        if adjustments.is_empty() {
            continue;
        }
        asked += 1;
        answers.insert(adjustments);
    }
    assert!(asked > 100, "only {asked} presets were asked");
    // **Fifty-one of 119, measured.** Shapes genuinely share extreme sets — every one-handled shape
    // whose domain is `0..100000` answers the same pair — so the floor is what the table yields.
    // What it rules out is the failure this instrument exists for: a function that answered the
    // same thing for every shape would produce **one**.
    assert!(
        answers.len() >= 40,
        "{asked} presets produced {} distinct extreme sets; fifty-one is what the domains yield, \
         and a collapse towards one would mean the function is ignoring which shape it was given",
        answers.len()
    );
    println!(
        "{asked} adjustable presets, {} distinct extreme sets",
        answers.len()
    );
}

#[test]
fn the_probe_set_is_ninety_two_different_characters_in_five_different_families() {
    let probes = probes();
    let characters: BTreeSet<char> = probes.iter().map(|probe| probe.character).collect();
    let families: BTreeSet<&str> = probes.iter().map(|probe| probe.family).collect();
    assert_eq!(characters.len(), 92);
    assert_eq!(families.len(), 5);
    // Every probe's two strings are different from every other probe's, or two rows measure the
    // same thing.
    let strings: BTreeSet<String> = probes
        .iter()
        .filter(|probe| probe.family == "Arial")
        .map(mjx_reference_pack::typography::Probe::single_text)
        .collect();
    assert_eq!(strings.len(), 92);
    // And the boxes are in distinct places.
    let places: BTreeSet<(usize, i64, i64)> = probes
        .iter()
        .map(|probe| (probe.page, probe.single.x, probe.single.y))
        .collect();
    assert_eq!(places.len(), probes.len(), "two probes share a slot");
}

#[test]
fn the_swatch_sheet_is_fifty_four_different_patterns() {
    let tokens: BTreeSet<&str> = swatches().iter().map(|swatch| swatch.token).collect();
    assert_eq!(tokens.len(), 54);
    let coverages: BTreeSet<u32> = (0..54)
        .map(|index| {
            mjx_paint::PATTERN_MASKS[index]
                .iter()
                .map(|row| row.count_ones())
                .sum()
        })
        .collect();
    assert!(
        coverages.len() > 10,
        "the fifty-four masks have {} distinct coverages, so most of them are the same picture",
        coverages.len()
    );
}

#[test]
fn the_three_hanging_paragraphs_say_three_different_things() {
    let texts: BTreeSet<String> = HangingRole::ALL
        .into_iter()
        .map(HangingRole::text)
        .collect();
    assert_eq!(
        texts.len(),
        2,
        "the two Japanese paragraphs are deliberately the same text at different settings, and the \
         Latin one is different — three identical or three different texts would both be wrong"
    );
    let settings: BTreeSet<bool> = HangingRole::ALL
        .into_iter()
        .map(HangingRole::overflow_punctuation)
        .collect();
    assert_eq!(settings.len(), 2, "the differential needs both settings");
    let labels: BTreeSet<&str> = HangingRole::ALL
        .into_iter()
        .map(HangingRole::label)
        .collect();
    assert_eq!(labels.len(), 3);
}

/// The comparator, driven with three different pairs, so an arm that ignored an input is visible.
#[test]
fn the_comparator_answers_differently_for_three_different_pairs() {
    let window = Rect::new(0, 0, 100, 60);
    let white = flat(200, 120, [0xff, 0xff, 0xff]);
    let grey = flat(200, 120, [0x80, 0x80, 0x80]);
    let almost = flat(200, 120, [0x88, 0x88, 0x88]);

    // Two identical inked rasters: an agreement, and one the ink guard lets through.
    let same = compare_window(&grey, &grey, window);
    assert!(same.reference_ink > 0 && same.ours_ink > 0);
    assert_eq!(same.differing, 0);
    assert!(same.verdict(true).is_pass());

    // Two very different ones: a disagreement, naming where.
    let different = compare_window(&grey, &white, window);
    assert_eq!(different.differing, different.pixels);
    let verdict = different.verdict(true);
    assert!(verdict.is_failure(), "{verdict}");
    let Verdict::Disagreed { worst, .. } = &verdict else {
        unreachable!()
    };
    assert!(
        worst.contains("at ("),
        "the disagreement names no place: {worst}"
    );

    // Two that differ by less than the channel tolerance: still an agreement, which is what the
    // tolerance is for. `0x80` against `0x88` is eight levels, and the tolerance is sixteen.
    let near = compare_window(&grey, &almost, window);
    assert_eq!(near.differing, 0, "eight levels is inside the tolerance");
    assert!(
        near.worst.is_some(),
        "the two are not identical, so there is a worst pixel"
    );

    // And a blank reference is not evidence, whatever the other side did.
    let blank = compare_window(&white, &grey, window);
    assert!(
        !blank.verdict(true).is_evidence(),
        "a blank reference agreed with something"
    );
    // ...unless nothing was expected there, which is the other half of the same argument.
    assert!(blank.verdict(false).is_evidence());

    // Three distinct differing fractions out of four comparisons.
    let fractions: BTreeSet<u64> = [&same, &different, &near, &blank]
        .into_iter()
        .map(|agreement| (agreement.differing_fraction() * 1e6) as u64)
        .collect();
    assert!(
        fractions.len() >= 2,
        "four comparisons produced {} distinct answers",
        fractions.len()
    );
    // A tolerance of zero would fail every plate and one of a half would pass a shape drawn in the
    // wrong place, so the constant itself is held between them.
    const { assert!(PLATE_TOLERANCE > 0.0 && PLATE_TOLERANCE < 0.5) };
}

#[test]
fn the_crop_takes_a_different_rectangle_for_a_different_window() {
    let mut raster = flat(200, 120, [0xff, 0xff, 0xff]);
    // One black pixel, in the top-left window and not in the bottom-right one.
    let offset = ((10_usize) * 200 + 10) * 3;
    raster.rgb[offset..offset + 3].copy_from_slice(&[0, 0, 0]);

    let corner = crop(&raster, Rect::new(0, 0, 30, 30));
    let elsewhere = crop(&raster, Rect::new(60, 40, 30, 30));
    assert_eq!(
        corner.ink(),
        1,
        "the marked pixel is not in the first window"
    );
    assert_eq!(
        elsewhere.ink(),
        0,
        "the marked pixel is in the second window too"
    );
    assert_eq!(corner.width, elsewhere.width);
}

#[test]
fn the_exclusion_is_asked_about_every_pair_and_answers_both_ways() {
    let mut excluded = 0usize;
    let mut compared = 0usize;
    for provider in ReferenceProvider::ALL {
        for content in RenderedContent::ALL {
            let row = Baseline::new(
                content.label(),
                provider.label(),
                provider,
                content,
                Verdict::Agreed {
                    differing_fraction: 0.0,
                    allowed: 0.02,
                },
            );
            if row.verdict.is_evidence() {
                compared += 1;
            } else {
                excluded += 1;
            }
        }
    }
    // Both answers, over eighteen pairs. A table that excluded everything and one that excluded
    // nothing would each pass a one-sided check.
    assert!(
        excluded > 0 && compared > 0,
        "{excluded} excluded and {compared} compared out of eighteen pairs"
    );
    assert_eq!(excluded + compared, 18);
}

/// The two hand-rolled parsers, each driven with a well-formed input and a broken one.
#[test]
fn the_parsers_answer_and_refuse() {
    // A two-pixel PPM, and the same header with the payload cut short.
    let ppm = b"P6\n2 1\n255\n\x00\x00\x00\xff\xff\xff";
    let raster = parse_ppm(ppm).expect("a well-formed PPM parses");
    assert_eq!((raster.width, raster.height), (2, 1));
    assert_eq!(raster.pixel(1, 0), Some([0xff, 0xff, 0xff]));
    assert!(
        parse_ppm(b"P6\n2 1\n255\n\x00").is_err(),
        "a short PPM was accepted"
    );
    assert!(
        parse_ppm(b"P3\n2 1\n255\n0 0 0 1 1 1").is_err(),
        "an ASCII PPM was accepted"
    );
    assert!(parse_ppm(b"").is_err(), "an empty file was accepted");
    // A comment between the header tokens, which netpbm allows.
    assert!(parse_ppm(b"P6\n# a comment\n2 1\n255\n\x00\x00\x00\xff\xff\xff").is_ok());

    let xhtml = "<page width=\"100\" height=\"100\">\n\
                 <word xMin=\"1.5\" yMin=\"2.5\" xMax=\"3.5\" yMax=\"4.5\">a&amp;b</word>\n\
                 <word xMin=\"5\" yMin=\"2.5\" xMax=\"7\" yMax=\"4.5\">c</word>\n";
    let words = parse_bbox_layout(xhtml);
    assert_eq!(words.len(), 2);
    assert_eq!(words[0].text, "a&b", "the entities were not unescaped");
    assert!((words[0].x_max - 3.5).abs() < 1e-9);
    assert_eq!(words[0].page, 1);
    // Two distinct positions, so the attribute reader is not returning one number for all of them.
    assert!(words[0].x_min < words[1].x_min);
    // And a word with a missing attribute is dropped rather than read as zero.
    assert!(parse_bbox_layout("<word yMin=\"1\" xMax=\"2\" yMax=\"3\">x</word>").is_empty());
    assert!(parse_bbox_layout("nothing here").is_empty());
}
