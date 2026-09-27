//! **The identity-value probe, over every branch — and the question of how many distinct answers a
//! gate actually sees.**
//!
//! A function tested only at the value where it does nothing is a function nobody has tested. Half
//! of this loop's findings are that shape:
//!
//! * R09: `Effect::new` leaves both alphas at zero, so every effect fixture in the workspace built
//!   a reflection that drew no pixel — and two painters drawing nothing agree perfectly.
//! * G04: `parametric_angle` compared a product to exact zero, and `sin π` is `1.22e-16`, so a shape
//!   at `adj = 0` drew eighty pixels left of a box that begins at zero.
//! * G06: a green mutation that turned out to be *counting answers rather than counting distinct
//!   answers*.
//!
//! So every function below is called at the value where it is the identity **and** at one where it
//! is not, and the two answers are asserted to differ. Where a gate counts, it counts *distinct*
//! values.
//!
//! This file also found one of its own, which is recorded rather than quietly fixed: the nested
//! specimen's innermost box was originally drawn in the same ink as the group containing it, so it
//! was invisible and a change to it would have moved no pixel. See `specimen::DECORATION_ACCENT`.

use mjx_render_oracle::authority::{ReferenceProvider, RenderedContent, Verdict};
use mjx_render_oracle::digest::sha256_hex;
use mjx_render_oracle::geom::Rect;
use mjx_render_oracle::pdf::{compare_words, WORD_TOLERANCE_POINTS};
use mjx_render_oracle::perceptual::{compare, diff_image, structural_detail, Tolerance};
use mjx_render_oracle::png::{decode, encode, straight_from_premultiplied, Image};
use mjx_render_oracle::snapshot::Snapshot;
use mjx_render_oracle::specimen::{Perturbation, SPECIMENS};
use mjx_render_oracle::tools::{parse_bbox_layout, Raster, WordBox};
use mjx_render_oracle::{png, snapshot, specimen};

#[test]
fn the_premultiplication_conversion_is_probed_off_its_identity() {
    // Opaque is the identity. A conversion tested only there is a conversion never run.
    assert_eq!(
        straight_from_premultiplied([0x11, 0x22, 0x33, 0xff]),
        [0x11, 0x22, 0x33, 0xff]
    );
    // And at four alphas that are not, including the two ends.
    let probes = [
        ([0x80, 0x40, 0x00, 0x80], [0xff, 0x80, 0x00, 0x80]),
        ([0x40, 0x20, 0x10, 0x40], [0xff, 0x80, 0x40, 0x40]),
        ([0x01, 0x00, 0x00, 0x01], [0xff, 0x00, 0x00, 0x01]),
        ([0x00, 0x00, 0x00, 0x00], [0x00, 0x00, 0x00, 0x00]),
    ];
    let mut answers = Vec::new();
    for (input, expected) in probes {
        let answer = straight_from_premultiplied(input);
        assert_eq!(answer, expected, "converting {input:?}");
        answers.push(answer);
    }
    assert_eq!(
        distinct(&answers),
        4,
        "the conversion gives fewer than four distinct answers to four different inputs"
    );
    // The one case where it is *not* a round trip, stated rather than discovered: a transparent
    // pixel has no colour to recover.
    assert_ne!(
        straight_from_premultiplied([0x00, 0x00, 0x00, 0x00]),
        [0x12, 0x34, 0x56, 0x00]
    );
}

#[test]
fn the_scale_conversion_is_probed_away_from_seventy_two_dots_an_inch() {
    let rect = Rect::new(10, 20, 100, 50);
    // Seventy-two is the identity: one point is one pixel.
    let identity = rect.to_pixels(72.0, 10_000, 10_000);
    assert_eq!((identity.x, identity.y), (10, 20));
    assert_eq!((identity.width, identity.height), (100, 50));

    let mut areas = Vec::new();
    for dpi in [72.0, 96.0, 144.0, 300.0] {
        let pixels = rect.to_pixels(dpi, 10_000, 10_000);
        areas.push(pixels.area());
    }
    assert_eq!(
        distinct(&areas),
        4,
        "four resolutions gave {} distinct areas: {areas:?}",
        distinct(&areas)
    );
    // Rounded **inwards**, so a window never borrows a column from its neighbour: at 96 dpi the
    // left edge is 13.33 and the right is 146.67, and the answer is 14..146.
    let ninety_six = rect.to_pixels(96.0, 10_000, 10_000);
    assert_eq!((ninety_six.x, ninety_six.width), (14, 132));
    // And clamped to the raster it is being cropped out of.
    assert_eq!(rect.to_pixels(96.0, 20, 20).area(), 0);
}

#[test]
fn the_digest_is_probed_at_the_empty_message_and_away_from_it() {
    let mut digests = Vec::new();
    for message in [
        b"".to_vec(),
        b"\0".to_vec(),
        vec![0u8; 64],
        vec![0u8; 65],
        b"the same length".to_vec(),
        b"but not the same".to_vec(),
    ] {
        digests.push(sha256_hex(&message));
    }
    assert_eq!(
        distinct(&digests),
        6,
        "six different messages produced {} distinct digests, so an approval could bind to the \
         wrong bytes",
        distinct(&digests)
    );
    // A single flipped bit anywhere changes the answer — the property the whole approval record
    // rests on.
    let mut message = vec![0x5au8; 1000];
    let before = sha256_hex(&message);
    message[500] ^= 0x01;
    assert_ne!(before, sha256_hex(&message));
}

#[test]
fn the_encoder_is_probed_on_more_than_a_flat_image() {
    // A flat image is the identity for an LZ77 compressor: everything is one long match, and an
    // encoder that only ever emitted matches would pass on it.
    let flat = Image {
        width: 32,
        height: 32,
        rgba: vec![0x40; 32 * 32 * 4],
    };
    assert_eq!(
        decode(&encode(&flat).expect("encoding")).expect("decoding"),
        flat
    );

    // Noise is the other end: nothing matches, so every byte is a literal.
    let noisy = Image {
        width: 32,
        height: 32,
        rgba: (0..32 * 32 * 4)
            .map(|index| ((index * 2_654_435_761usize) >> 13) as u8)
            .collect(),
    };
    assert_eq!(
        decode(&encode(&noisy).expect("encoding")).expect("decoding"),
        noisy
    );

    // A run of one repeated byte, which is where deflate writes an **overlapping** match — a
    // decoder that copied the match with `copy_from_slice` rather than a byte at a time gets this
    // wrong and nothing else in the suite would catch it.
    let mut runs = vec![0u8; 4];
    runs.extend(std::iter::repeat_n(0xab, 4 * 32 * 32 - 4));
    let repeated = Image {
        width: 32,
        height: 32,
        rgba: runs,
    };
    assert_eq!(
        decode(&encode(&repeated).expect("encoding")).expect("decoding"),
        repeated
    );

    // A single pixel, which is the smallest thing the header arithmetic has to get right.
    let one = Image {
        width: 1,
        height: 1,
        rgba: vec![1, 2, 3, 4],
    };
    assert_eq!(
        decode(&encode(&one).expect("encoding")).expect("decoding"),
        one
    );

    // And the sizes it refuses rather than writing a file whose header disagrees with its body.
    assert!(encode(&Image {
        width: 0,
        height: 1,
        rgba: Vec::new()
    })
    .is_err());
    assert!(encode(&Image {
        width: 2,
        height: 2,
        rgba: vec![0; 4]
    })
    .is_err());
    assert!(decode(b"not a png").is_err());
    let mut corrupt = encode(&one).expect("encoding");
    corrupt[20] ^= 0xff;
    assert!(
        decode(&corrupt).is_err(),
        "a corrupted PNG decoded successfully, so the CRC and the Adler-32 are not being checked"
    );

    // Four encodings, four distinct byte strings: an encoder that answered the same bytes for
    // different images would make every baseline digest meaningless.
    let encodings = [
        encode(&flat).expect("a"),
        encode(&noisy).expect("b"),
        encode(&repeated).expect("c"),
        encode(&one).expect("d"),
    ];
    assert_eq!(distinct(&encodings), 4);
}

#[test]
fn the_metric_is_probed_off_identity_in_both_directions() {
    let base = png::image_from_pixels(
        &specimen::render(&SPECIMENS[2], Perturbation::None)
            .expect("a render")
            .pixels,
    );
    // Identity: an image against itself.
    let (mean, worst) = structural_detail(&base, &base);
    assert!((mean - 1.0).abs() < 1e-9);
    assert!((worst.expect("a window").similarity - 1.0).abs() < 1e-9);
    let identical = compare(&base, &base, Tolerance::EXACT_RASTER).expect("same size");
    assert_eq!(identical.differing, 0);
    assert_eq!(identical.max_channel_difference, 0);
    assert_eq!(
        identical.worst, None,
        "an identical pair has no worst pixel"
    );
    assert!(identical.verdict(Tolerance::EXACT_RASTER, true).is_pass());

    // And a diff image of an identical pair is black everywhere, which is the identity for
    // `diff_image` — a diff that amplified nothing would look the same on a real difference.
    let blank = diff_image(&base, &base).expect("same size");
    assert!(blank
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .all(|pixel| pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0 && pixel[3] == 0xff));

    // Off identity: three different changes, three different answers.
    let moved = png::image_from_pixels(
        &specimen::render(&SPECIMENS[2], Perturbation::FragmentPosition)
            .expect("a render")
            .pixels,
    );
    let recoloured = png::image_from_pixels(
        &specimen::render(&SPECIMENS[2], Perturbation::PaintColour)
            .expect("a render")
            .pixels,
    );
    let answers = [
        compare(&base, &base, Tolerance::EXACT_RASTER)
            .expect("a")
            .differing,
        compare(&base, &moved, Tolerance::EXACT_RASTER)
            .expect("b")
            .differing,
        compare(&base, &recoloured, Tolerance::EXACT_RASTER)
            .expect("c")
            .differing,
    ];
    assert_eq!(
        distinct(&answers),
        3,
        "three different changes gave {} distinct differing counts: {answers:?}",
        distinct(&answers)
    );
    let coloured = diff_image(&base, &recoloured).expect("same size");
    assert!(
        coloured
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[0] > 0),
        "the diff image of a recoloured page is black, so a reviewer would see a pass"
    );
    assert!(diff_image(&base, &blank).is_err() || blank.width == base.width);

    // The two failures behave differently under the metric, which is the whole reason it is here
    // beside a pixel count: **a positional error destroys local covariance and a tint does not.**
    let moved_similarity = structural_detail(&base, &moved).0;
    let tint_similarity = structural_detail(&base, &recoloured).0;
    assert!(
        moved_similarity < tint_similarity,
        "a moved shape scores {moved_similarity} and a recoloured one {tint_similarity}; if a tint \
         were the more structural of the two, the metric is not measuring structure"
    );
}

#[test]
fn the_snapshot_diff_is_probed_at_equality_and_at_every_way_of_being_unequal() {
    let one = Snapshot::from_lines(vec!["a".to_owned(), "b".to_owned()]);
    assert_eq!(
        one.first_difference(&one),
        None,
        "a snapshot differs from itself"
    );

    let mut answers = Vec::new();
    for other in [
        Snapshot::from_lines(vec!["a".to_owned(), "c".to_owned()]),
        Snapshot::from_lines(vec!["a".to_owned()]),
        Snapshot::from_lines(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]),
        Snapshot::from_lines(Vec::new()),
    ] {
        let answer = one
            .first_difference(&other)
            .expect("a difference is reported");
        assert!(answer.starts_with("line "), "{answer}");
        answers.push(answer);
    }
    assert_eq!(
        distinct(&answers),
        4,
        "four different ways of differing gave {} messages: {answers:?}",
        distinct(&answers)
    );

    // Round-tripping through text is the identity, including the trailing-newline case an editor
    // decides for you.
    for text in ["a\nb\n", "a\nb", "a\nb\n\n\n", "a\nb  \n"] {
        assert_eq!(
            Snapshot::parse(text),
            one,
            "`{text:?}` does not parse to the same snapshot"
        );
    }
    assert_eq!(Snapshot::parse(&one.to_text()), one);
    assert_eq!(Snapshot::parse("").lines().len(), 0);
}

#[test]
fn the_word_comparison_is_probed_at_zero_displacement_and_away_from_it() {
    let at = |x: f64| WordBox {
        page: 1,
        text: "w".to_owned(),
        x_min: x,
        y_min: 0.0,
        x_max: x + 10.0,
        y_max: 10.0,
    };
    // Zero is the identity, and a comparison tested only there is one that could report every word
    // as moved.
    assert!(compare_words(&[at(0.0)], &[at(0.0)], WORD_TOLERANCE_POINTS).agreed());
    let mut moved = Vec::new();
    for shift in [0.0, 0.1, 0.3, 8.0] {
        moved.push(compare_words(&[at(0.0)], &[at(shift)], WORD_TOLERANCE_POINTS).moved);
    }
    assert_eq!(
        moved,
        vec![0, 0, 1, 1],
        "the tolerance is not where it says it is"
    );

    // An empty comparison agrees, and that is exactly the vacuous case a caller has to guard: it is
    // reported as an agreement of *nothing*, which the counts say.
    let empty = compare_words(&[], &[], WORD_TOLERANCE_POINTS);
    assert!(empty.agreed());
    assert_eq!(empty.counts, (0, 0));

    // Reading order rather than input order, so two documents whose words come back in different
    // order still line up — and a word that moved to another line is reported rather than matched.
    let one_line = vec![at(30.0), at(10.0)];
    let same = vec![at(10.0), at(30.0)];
    assert!(compare_words(&one_line, &same, WORD_TOLERANCE_POINTS).agreed());
}

#[test]
fn the_bbox_reader_is_probed_on_output_it_did_not_generate_itself() {
    // A recorded sample of what `pdftotext -bbox-layout` actually writes, so the hand-rolled scan
    // cannot quietly stop matching poppler's markup.
    let sample = "\
<doc>
  <page width=\"360.000000\" height=\"120.000000\">
    <flow>
      <block>
        <line xMin=\"16.0\" yMin=\"55.6\" xMax=\"96.0\" yMax=\"73.6\">
          <word xMin=\"16.000000\" yMin=\"55.632000\" xMax=\"96.008000\" yMax=\"73.632000\">Fidelity</word>
          <word xMin=\"114.000000\" yMin=\"55.632000\" xMax=\"156.000000\" yMax=\"73.632000\">a &amp; b</word>
        </line>
      </block>
    </flow>
  </page>
</doc>";
    let words = parse_bbox_layout(sample);
    assert_eq!(words.len(), 2);
    assert_eq!(words[0].text, "Fidelity");
    assert_eq!(
        words[1].text, "a & b",
        "the five XML entities are not unescaped"
    );
    assert_eq!(words[0].page, 1);
    assert!((words[0].x_min - 16.0).abs() < 1e-9);
    assert!((words[1].y_max - 73.632).abs() < 1e-9);

    // The identity input — no words at all — answers nothing rather than one empty word.
    assert!(parse_bbox_layout("<doc></doc>").is_empty());
    assert!(parse_bbox_layout("").is_empty());
    // And a `<word>` missing a coordinate is dropped rather than defaulted to zero, which would put
    // a phantom word at the origin of every page.
    assert!(parse_bbox_layout("<word xMin=\"1\">x</word>").is_empty());
}

#[test]
fn the_ink_count_is_probed_on_a_raster_that_has_some() {
    // White is the identity for `Raster::ink`, and a comparison that believed an agreement between
    // two blank crops is the oldest way this family of gate goes hollow.
    let white = Raster {
        width: 4,
        height: 4,
        rgb: vec![0xff; 48],
    };
    assert_eq!(white.ink(), 0);
    let mut inked = white.clone();
    inked.rgb[0] = 0x00;
    assert_eq!(inked.ink(), 1);
    // Two hundred and fifty is the threshold, so a near-white pixel is not ink and a slightly
    // darker one is — a threshold tested only at black would pass for one set anywhere.
    let mut nearly = white.clone();
    nearly.rgb[3] = 250;
    assert_eq!(nearly.ink(), 0);
    nearly.rgb[3] = 249;
    assert_eq!(nearly.ink(), 1);
}

#[test]
fn every_verdict_branch_is_reached_by_a_real_comparison() {
    // Not constructed: each of the three states is produced by a comparison that ran, so a branch
    // that could never be reached from real data would show up here as a missing answer.
    let base = png::image_from_pixels(
        &specimen::render(&SPECIMENS[0], Perturbation::None)
            .expect("a render")
            .pixels,
    );
    let other = png::image_from_pixels(
        &specimen::render(&SPECIMENS[0], Perturbation::PaintColour)
            .expect("a render")
            .pixels,
    );
    let blank = Image {
        width: base.width,
        height: base.height,
        rgba: vec![0; base.rgba.len()],
    };
    let agreed = compare(&base, &base, Tolerance::EXACT_RASTER)
        .expect("a")
        .verdict(Tolerance::EXACT_RASTER, true);
    let disagreed = compare(&base, &other, Tolerance::EXACT_RASTER)
        .expect("b")
        .verdict(Tolerance::EXACT_RASTER, true);
    let not_evidence = compare(&blank, &blank, Tolerance::EXACT_RASTER)
        .expect("c")
        .verdict(Tolerance::EXACT_RASTER, true);
    assert!(matches!(agreed, Verdict::Agreed { .. }));
    assert!(matches!(disagreed, Verdict::Disagreed { .. }));
    assert!(matches!(not_evidence, Verdict::NotEvidence { .. }));
    assert_eq!(
        distinct(&[agreed.word(), disagreed.word(), not_evidence.word()]),
        3
    );

    // And the exclusion path, which is the fourth way to reach `NotEvidence` and the only one that
    // does not come from the pixels.
    let excluded = mjx_render_oracle::Baseline::new(
        "gradient-panel",
        "here",
        ReferenceProvider::LibreOffice,
        RenderedContent::GradientFill,
        agreed,
    );
    assert!(matches!(excluded.verdict, Verdict::NotEvidence { .. }));
}

#[test]
fn every_snapshot_line_kind_is_produced_by_a_specimen() {
    // The snapshot writers have one arm per fragment kind and one per command; a corpus that only
    // produced boxes would leave most of them unexercised, and an unexercised formatter is one that
    // can be wrong for ever.
    let mut fragment_kinds = Vec::new();
    let mut command_kinds = Vec::new();
    for spec in SPECIMENS {
        let tree = specimen::fragments(spec, Perturbation::None);
        for line in snapshot::fragments(&tree).lines().iter().skip(1) {
            if let Some(kind) = line.split_whitespace().nth(1) {
                if !fragment_kinds.contains(&kind.to_owned()) {
                    fragment_kinds.push(kind.to_owned());
                }
            }
        }
        let list = specimen::display_list(spec, Perturbation::None).expect("a display list");
        for line in snapshot::commands(&list).lines().iter() {
            if let Some(kind) = line.split_whitespace().nth(1) {
                if !command_kinds.contains(&kind.to_owned()) {
                    command_kinds.push(kind.to_owned());
                }
            }
        }
    }
    assert!(
        fragment_kinds.contains(&"box".to_owned()) && fragment_kinds.contains(&"shape".to_owned()),
        "the corpus produces only {fragment_kinds:?}"
    );
    for wanted in ["fill", "stroke", "push-clip", "push-opacity", "pop"] {
        assert!(
            command_kinds.contains(&wanted.to_owned()),
            "no specimen produces a `{wanted}` command; the corpus is {command_kinds:?}"
        );
    }
    // Both paint shapes the corpus can carry, so the resolver in `paint_of` is exercised past its
    // first arm.
    let gradient = specimen::display_list(
        &specimen::Specimen::named("gradient-panel").expect("the gradient"),
        Perturbation::None,
    )
    .expect("a display list");
    let text = snapshot::commands(&gradient).to_text();
    assert!(
        text.contains("gradient["),
        "the gradient page names no gradient: {text}"
    );
    assert!(
        text.contains("stops="),
        "the gradient's stops are not in the snapshot"
    );
}

/// How many distinct values a slice holds.
///
/// By hand rather than through a set, because *counting answers is not counting distinct answers* —
/// which is the mutation G06 found green — and several of the types here are only `PartialEq`.
fn distinct<T: PartialEq>(values: &[T]) -> usize {
    let mut seen: Vec<&T> = Vec::new();
    for value in values {
        if !seen.contains(&value) {
            seen.push(value);
        }
    }
    seen.len()
}
