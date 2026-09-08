//! The four instruments this crate had to write, each checked against something **it did not
//! write**.
//!
//! A harness whose own instruments are only checked by itself is a harness that measures its own
//! opinion. So:
//!
//! | Instrument | Checked against |
//! |---|---|
//! | SHA-256 | FIPS 180-4's published vectors, **and** the system's `sha256sum` |
//! | the PNG encoder | ImageMagick, which reads the file back — dimensions, pixels and all |
//! | the premultiplication decision | the same ImageMagick read: a half-alpha red must come back `#FF000080` |
//! | the structural metric | its own algebra: identity, symmetry, and a difference it must not absorb |
//!
//! The PNG decoder is deliberately **not** in that table. It is this crate's decoder checked by this
//! crate's encoder, which proves they agree and nothing about whether either is right; that is why
//! ImageMagick is in the loop at all, and why the decoder's own documentation says so.

use std::path::PathBuf;

use mjx_render_oracle::digest::sha256_hex;
use mjx_render_oracle::perceptual::{compare, structural_similarity, Tolerance};
use mjx_render_oracle::png::{
    decode, encode, image_from_pixels, straight_from_premultiplied, Image,
};
use mjx_render_oracle::specimen::{Perturbation, SPECIMENS};
use mjx_render_oracle::tools::{one_of, tool, IMAGE_MAGICK, REQUIRE_TOOLS};
use mjx_render_oracle::{specimen, Verdict};

// ---------------------------------------------------------------------------------------------
// SHA-256
// ---------------------------------------------------------------------------------------------

#[test]
fn the_digest_answers_the_published_vectors() {
    // FIPS 180-4's own examples, plus the empty string, plus a message longer than one block so the
    // multi-block path is exercised — a single-block-only implementation passes the first three.
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    assert_eq!(
        sha256_hex(&b"a".repeat(1_000_000)),
        "cd c7 6e 5c 99 14 fb 92 81 a1 c7 e2 84 d7 3e 67 f1 80 9a 48 a4 97 20 0e 04 6d 39 cc c7 \
         11 2c d0"
            .replace(' ', "")
    );

    // A length that lands exactly on the padding boundary — 55, 56 and 64 bytes — because those are
    // where an implementation that got the extra block wrong still passes everything above.
    for length in [54usize, 55, 56, 57, 63, 64, 65] {
        let message = vec![b'x'; length];
        assert_eq!(
            sha256_hex(&message).len(),
            64,
            "a digest of a {length}-byte message is not sixty-four digits"
        );
    }
    assert_eq!(
        sha256_hex(&[b'x'; 56]),
        sha256_hex(&[b'x'; 56]),
        "the digest is not a function of its input"
    );
    assert_ne!(sha256_hex(&[b'x'; 55]), sha256_hex(&[b'x'; 56]));
}

#[test]
fn the_system_sha256sum_agrees_with_ours() {
    if !tool(
        "the digest against the system's own",
        "sha256sum",
        REQUIRE_TOOLS,
    ) {
        return;
    }
    let directory = scratch("digest");
    // The committed baselines, because they are the bytes an approval record actually binds — a
    // person auditing one runs `sha256sum` on exactly these files.
    for specimen in SPECIMENS {
        let bytes = mjx_render_oracle::Baselines::committed()
            .artefact(specimen.name, "render.png")
            .expect("a committed image");
        let path = directory.join(format!("{}.png", specimen.name));
        std::fs::write(&path, &bytes).expect("writing a copy");
        let output = std::process::Command::new("sha256sum")
            .arg(&path)
            .output()
            .expect("running sha256sum");
        let theirs = String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_owned();
        assert_eq!(
            sha256_hex(&bytes),
            theirs,
            "`{}`'s digest disagrees with the system tool, so an approval record cannot be audited \
             by the person whose approval it claims",
            specimen.name
        );
    }
    let _ = std::fs::remove_dir_all(&directory);
}

// ---------------------------------------------------------------------------------------------
// The PNG encoder, and the premultiplication decision
// ---------------------------------------------------------------------------------------------

#[test]
fn the_encoder_is_deterministic() {
    for specimen in SPECIMENS {
        let rendered = specimen::render(specimen, Perturbation::None).expect("a render");
        let image = image_from_pixels(&rendered.pixels);
        let once = encode(&image).expect("encoding once");
        let twice = encode(&image).expect("encoding twice");
        assert_eq!(
            once, twice,
            "`{}` encodes to different bytes twice running, so a baseline's digest would expire \
             for a reason that has nothing to do with the image",
            specimen.name
        );
        // And the compression is doing something: a plate is mostly flat colour, and a stream that
        // was not compressing would be four bytes a pixel plus a byte a row.
        let raw = (image.width as usize) * (image.height as usize) * 4;
        assert!(
            once.len() * 4 < raw,
            "`{}` encodes to {} bytes from {raw} raw, which is not compression",
            specimen.name,
            once.len()
        );
    }
}

#[test]
fn the_encoder_and_the_decoder_agree_on_every_plate() {
    for specimen in SPECIMENS {
        let rendered = specimen::render(specimen, Perturbation::None).expect("a render");
        let image = image_from_pixels(&rendered.pixels);
        let reread = decode(&encode(&image).expect("encoding")).expect("decoding");
        assert_eq!(
            reread, image,
            "`{}` does not survive a round trip through this crate's own PNG",
            specimen.name
        );
    }
}

#[test]
fn a_decoder_this_workspace_did_not_write_reads_our_plates() {
    let Some(magick) = one_of(
        "the PNG against an outside reader",
        &IMAGE_MAGICK,
        REQUIRE_TOOLS,
    ) else {
        return;
    };
    let directory = scratch("imagemagick");
    for specimen in SPECIMENS {
        let rendered = specimen::render(specimen, Perturbation::None).expect("a render");
        let image = image_from_pixels(&rendered.pixels);
        let path = directory.join(format!("{}.png", specimen.name));
        std::fs::write(&path, encode(&image).expect("encoding")).expect("writing");

        let output = std::process::Command::new(magick)
            .arg(&path)
            .arg("-depth")
            .arg("8")
            .arg("txt:-")
            .output()
            .expect("running magick");
        assert!(
            output.status.success(),
            "ImageMagick refused `{}`: {}",
            specimen.name,
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8_lossy(&output.stdout);
        let read: Vec<[u8; 4]> = text
            .lines()
            .skip(1)
            .filter_map(|line| line.split_once('#'))
            .filter_map(|(_, tail)| {
                let hex = tail.split_whitespace().next()?;
                (hex.len() == 8).then(|| {
                    [
                        u8::from_str_radix(&hex[0..2], 16).ok()?,
                        u8::from_str_radix(&hex[2..4], 16).ok()?,
                        u8::from_str_radix(&hex[4..6], 16).ok()?,
                        u8::from_str_radix(&hex[6..8], 16).ok()?,
                    ]
                    .into()
                })?
            })
            .collect();
        assert_eq!(
            read.len(),
            (image.width as usize) * (image.height as usize),
            "ImageMagick read {} pixels out of `{}`, and it has {}x{}",
            read.len(),
            specimen.name,
            image.width,
            image.height
        );
        for (index, (ours, theirs)) in image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .zip(read.iter())
            .enumerate()
        {
            assert_eq!(
                ours, theirs,
                "pixel {index} of `{}` reads back as {theirs:?} and was written as {ours:?}",
                specimen.name
            );
        }
    }
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn the_premultiplication_decision_is_visible_in_the_file() {
    // The conversion itself, at the three points that matter.
    assert_eq!(
        straight_from_premultiplied([0x80, 0x00, 0x00, 0x80]),
        [0xff, 0x00, 0x00, 0x80],
        "**this is the decision.** `mjx_paint::Pixels` is premultiplied — R09 measured `80 00 00 \
         80` for a half-alpha red from both rasterisers — and a PNG sample is not, so the \
         conversion happens exactly here."
    );
    assert_eq!(
        straight_from_premultiplied([0x11, 0x22, 0x33, 0xff]),
        [0x11, 0x22, 0x33, 0xff],
        "an opaque pixel is unchanged"
    );
    assert_eq!(
        straight_from_premultiplied([0x00, 0x00, 0x00, 0x00]),
        [0x00, 0x00, 0x00, 0x00],
        "a transparent pixel has no colour to recover — the one place the conversion loses \
         information, and the reason it is not applied to `Pixels` itself"
    );

    let Some(magick) = one_of(
        "the premultiplication decision against an outside reader",
        &IMAGE_MAGICK,
        REQUIRE_TOOLS,
    ) else {
        return;
    };
    let directory = scratch("premultiplied");
    let image = Image {
        width: 2,
        height: 1,
        rgba: vec![0xff, 0x00, 0x00, 0x80, 0x00, 0xff, 0x00, 0xff],
    };
    let path = directory.join("half-alpha-red.png");
    std::fs::write(&path, encode(&image).expect("encoding")).expect("writing");
    let output = std::process::Command::new(magick)
        .arg(&path)
        .arg("-depth")
        .arg("8")
        .arg("txt:-")
        .output()
        .expect("running magick");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("#FF000080"),
        "**an outside reader does not see a straight half-alpha red.** If this file were written \
         premultiplied it would read `#800000 80` here, and every viewer and every browser in \
         R11's gallery would render it wrong. What ImageMagick actually read:\n{text}"
    );
    assert!(text.contains("#00FF00FF"), "{text}");
    let _ = std::fs::remove_dir_all(&directory);
}

// ---------------------------------------------------------------------------------------------
// The structural metric
// ---------------------------------------------------------------------------------------------

/// A flat image of one colour.
fn flat(width: u32, height: u32, pixel: [u8; 4]) -> Image {
    Image {
        width,
        height,
        rgba: pixel
            .iter()
            .copied()
            .cycle()
            .take((width as usize) * (height as usize) * 4)
            .collect(),
    }
}

#[test]
fn the_structural_metric_has_the_algebra_a_metric_should() {
    let rendered = specimen::render(
        &specimen::Specimen::named("preset-star").expect("the star"),
        Perturbation::None,
    )
    .expect("a render");
    let image = image_from_pixels(&rendered.pixels);
    let moved = image_from_pixels(
        &specimen::render(
            &specimen::Specimen::named("preset-star").expect("the star"),
            Perturbation::FragmentPosition,
        )
        .expect("a render")
        .pixels,
    );

    assert!(
        (structural_similarity(&image, &image) - 1.0).abs() < 1e-9,
        "an image is not perfectly similar to itself"
    );
    assert!(
        (structural_similarity(&image, &moved) - structural_similarity(&moved, &image)).abs()
            < 1e-9,
        "the metric is not symmetric"
    );
    let shifted = structural_similarity(&image, &moved);
    assert!(
        shifted < 0.99,
        "a star moved eight points scores {shifted}, which a tolerance would absorb — the metric \
         is not seeing the shape at all"
    );

    // Different sizes answer zero rather than a plausible number: a size change is a finding, and a
    // metric that scored it would let it be absorbed by a tolerance.
    assert_eq!(structural_similarity(&image, &flat(4, 4, [0; 4])), 0.0);
    assert!(compare(&image, &flat(4, 4, [0; 4]), Tolerance::EXACT_RASTER).is_err());
}

#[test]
fn a_comparison_of_two_blank_images_is_not_a_pass() {
    // The oldest way this family of gate goes hollow, and the one `Agreement::both_drew` exists for
    // one layer down. Two transparent images are *identical*, so every number says they agree.
    let blank = flat(64, 64, [0, 0, 0, 0]);
    let difference = compare(&blank, &blank, Tolerance::EXACT_RASTER).expect("same size");
    assert_eq!(difference.differing, 0);
    assert!((difference.structural_similarity - 1.0).abs() < 1e-9);
    assert!(!difference.both_drew());
    assert!(
        matches!(
            difference.verdict(Tolerance::EXACT_RASTER, true),
            Verdict::NotEvidence { .. }
        ),
        "two blank images were reported as an agreement"
    );
    // And a caller that genuinely expects nothing gets a real answer rather than a refusal.
    assert!(difference.verdict(Tolerance::EXACT_RASTER, false).is_pass());
}

#[test]
fn a_recolour_is_not_absorbed_by_a_tolerance() {
    let one = flat(64, 64, [0x24, 0x4a, 0x8f, 0xff]);
    let other = flat(64, 64, [0xa8, 0x3c, 0x1e, 0xff]);
    let difference = compare(&one, &other, Tolerance::EXACT_RASTER).expect("same size");
    assert_eq!(difference.differing, 64 * 64);
    assert!(
        matches!(
            difference.verdict(Tolerance::EXACT_RASTER, true),
            Verdict::Disagreed { .. }
        ),
        "a whole page drawn in the wrong colour was reported as an agreement"
    );
    // The finding names a pixel and both sides of it.
    let Verdict::Disagreed { worst, .. } = difference.verdict(Tolerance::EXACT_RASTER, true) else {
        unreachable!("asserted above")
    };
    assert!(
        worst.contains("#244a8fff") && worst.contains("#a83c1eff"),
        "{worst}"
    );

    // A difference *inside* the channel tolerance is not a finding, which is the other half: a
    // metric that reported every antialiased edge would report nothing.
    let nudged = flat(64, 64, [0x24, 0x4a, 0x8f + 8, 0xff]);
    let small = compare(&one, &nudged, Tolerance::EXACT_RASTER).expect("same size");
    assert_eq!(small.differing, 0);
    assert!(small.max_channel_difference > 0, "nothing changed at all");
}

/// Where a case leaves its artefacts. Named per case, because Cargo runs a binary's cases on
/// several threads at once and two cases sharing a path is one reading a file the other is halfway
/// through writing.
fn scratch(case: &str) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/oracle-scratch")
        .join(case);
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a scratch directory");
    directory
}
