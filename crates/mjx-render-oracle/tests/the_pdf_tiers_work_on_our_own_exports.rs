//! **Proved before the Windows sitting, not during it.**
//!
//! MJXOFF-165 asks for the PDF comparison pipeline to work *today*, against our own exports, and the
//! reason is scheduling rather than pedantry: a pipeline first exercised on the morning the Office
//! artefacts arrive is a pipeline debugged on the one day it is expensive to debug. So both sides
//! here are our own PDF exporter, and the second one has **one word moved eight points**.
//!
//! Two tiers, and they answer different questions:
//!
//! * **The layout tier** reads word boxes with `pdftotext -bbox-layout` and must **name the word**.
//!   That is the whole claim: not *"the pages differ"* but *"word 2 `every` is at (137.4, 55.6) and
//!   was at (129.4, 55.6)"*. Exact, rasteriser-independent, and untouched by any provider exclusion.
//! * **The pixel tier** rasterises both PDFs with **one rasteriser** at one DPI, which is the only
//!   construction in which *"pixel perfect against PowerPoint"* is a coherent phrase.
//!
//! # The readers are external on purpose
//!
//! Checking our own export with our own reader proves nothing whatever. `pdftotext` and `pdftoppm`
//! are poppler's, and their absence is a **named** skip that `MJX_REQUIRE_TOOLS=1` turns into a
//! failure — which is what continuous integration sets, so this coverage cannot quietly evaporate.

use std::path::PathBuf;

use mjx_render_oracle::pdf::{
    compare_words, export, layout_tier, pixel_tier, text_page, MOVED_WORD, WORDS,
    WORD_TOLERANCE_POINTS,
};
use mjx_render_oracle::perceptual::Tolerance;
use mjx_render_oracle::tools::{page_count, tool, word_boxes, REQUIRE_TOOLS};

/// How far the moved word moves. Eight points: two-thirds of the type size, so it is unambiguous to
/// a reader and still far smaller than the interword gap, which is what makes it a *displacement*
/// rather than a reordering.
const SHIFT: f32 = 8.0;

/// Export both sides into a scratch directory and answer their paths.
fn two_exports(case: &str) -> (PathBuf, PathBuf) {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/oracle-scratch")
        .join(case);
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a scratch directory");
    let steady = directory.join("steady.pdf");
    let moved = directory.join("moved.pdf");
    std::fs::write(
        &steady,
        export(&text_page(0.0).expect("the steady page builds")).expect("it exports"),
    )
    .expect("writing the steady export");
    std::fs::write(
        &moved,
        export(&text_page(SHIFT).expect("the moved page builds")).expect("it exports"),
    )
    .expect("writing the moved export");
    (steady, moved)
}

#[test]
fn the_layout_tier_reads_our_own_words_back_out() {
    if !tool("the layout tier", "pdftotext", REQUIRE_TOOLS) {
        return;
    }
    let (steady, _) = two_exports("layout-reads");
    let words = word_boxes(&steady).expect("pdftotext reads our export");
    let text: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
    assert_eq!(
        text,
        WORDS.to_vec(),
        "**poppler did not read our words back.** Without a `/ToUnicode` map it reads raw glyph \
         ids as bytes — `)LGHOLW\\` rather than `Fidelity` — which is why this assertion is \
         load-bearing rather than decorative."
    );
    for word in &words {
        assert!(
            word.x_max > word.x_min && word.y_max > word.y_min,
            "`{}` has an empty box: {word:?}",
            word.text
        );
    }
    let _ = std::fs::remove_dir_all(steady.parent().expect("a parent"));
}

#[test]
fn a_one_word_displacement_is_detected_and_named() {
    if !tool("the layout tier", "pdftotext", REQUIRE_TOOLS) {
        return;
    }
    let (steady, moved) = two_exports("layout-names");

    // The page against itself: no divergence at all. **Without this, the case below would pass for
    // a tier that reported every word as moved.**
    let identical = layout_tier(&steady, &steady, WORD_TOLERANCE_POINTS).expect("pdftotext runs");
    assert!(
        identical.agreed(),
        "a document does not agree with itself: {:?}",
        identical.first_difference
    );
    assert_eq!(identical.counts, (WORDS.len(), WORDS.len()));

    let comparison = layout_tier(&steady, &moved, WORD_TOLERANCE_POINTS).expect("pdftotext runs");
    assert!(!comparison.agreed(), "the moved word was not detected");
    assert_eq!(
        comparison.moved, 1,
        "{} words moved, and exactly one was displaced — a tier that reports the whole page is a \
         tier that names nothing",
        comparison.moved
    );
    let named = comparison
        .first_difference
        .as_deref()
        .expect("a divergence names something");
    assert!(
        named.contains(WORDS[MOVED_WORD]),
        "**the layout tier did not name the word.** That is the whole of what this tier buys over \
         a pixel diff: {named}"
    );
    assert!(
        named.contains(&format!("{SHIFT:.2}")),
        "the finding does not say how far it moved: {named}"
    );
    let _ = std::fs::remove_dir_all(steady.parent().expect("a parent"));
}

#[test]
fn the_word_tolerance_is_a_tolerance_and_not_a_licence() {
    // Two boxes a hundredth of a point apart are the same box; two a whole point apart are not. A
    // tier whose tolerance swallowed a point would swallow a fifth of an interword gap.
    let at = |x: f64| mjx_render_oracle::tools::WordBox {
        page: 1,
        text: "word".to_owned(),
        x_min: x,
        y_min: 10.0,
        x_max: x + 20.0,
        y_max: 22.0,
    };
    assert!(compare_words(&[at(10.0)], &[at(10.01)], WORD_TOLERANCE_POINTS).agreed());
    assert!(!compare_words(&[at(10.0)], &[at(11.0)], WORD_TOLERANCE_POINTS).agreed());

    // A word that changed *text* without moving is a divergence too, and it is reported as one
    // rather than as a position: the same box holding a different word is the failure mode a
    // comparison matched by text could never see.
    let mut renamed = at(10.0);
    renamed.text = "different".to_owned();
    let comparison = compare_words(&[at(10.0)], &[renamed], WORD_TOLERANCE_POINTS);
    assert!(!comparison.agreed());
    let named = comparison.first_difference.expect("a divergence");
    assert!(
        named.contains("reads `word`") && named.contains("was `different`"),
        "{named}"
    );

    // And two documents with different word counts are a divergence even when every shared word
    // agrees — a comparison that zipped and stopped would report a dropped line as a pass.
    let comparison = compare_words(&[at(10.0), at(60.0)], &[at(10.0)], WORD_TOLERANCE_POINTS);
    assert!(!comparison.agreed());
    assert!(comparison
        .first_difference
        .expect("a divergence")
        .contains("2 words and the other 1"));
}

#[test]
fn the_pixel_tier_rasterises_both_sides_with_one_rasteriser() {
    if !tool("the pixel tier", "pdftoppm", REQUIRE_TOOLS)
        || !tool("the pixel tier", "pdfinfo", REQUIRE_TOOLS)
    {
        return;
    }
    let (steady, moved) = two_exports("pixel-tier");
    assert_eq!(page_count(&steady).expect("pdfinfo reads our export"), 1);

    // The page against itself, at the exact-raster tolerance: **one rasteriser, both sides, one
    // DPI**, so antialiasing cancels and a remaining difference is real.
    let same = pixel_tier(&steady, &steady, 1, Tolerance::EXACT_RASTER).expect("pdftoppm runs");
    assert_eq!(same.differing, 0, "a page differs from itself");
    assert!(
        same.both_drew(),
        "the rasteriser produced a blank page, and two blank pages agree perfectly"
    );
    assert_eq!(
        same.pixels,
        480 * 160,
        "a 360 by 120 point page rasterised at 96 dpi is 480 by 160 pixels, and this compared {} — \
         which means the two sides were not taken at the DPI this crate states",
        same.pixels
    );

    // And against the moved page, at the tolerance a comparison between two *producers* would use —
    // so the assertion is that a real displacement survives the loosest tolerance in the crate.
    let different =
        pixel_tier(&steady, &moved, 1, Tolerance::CROSS_PRODUCER).expect("pdftoppm runs");
    assert!(
        different.differing > 0,
        "a word moved eight points changed no pixel"
    );
    assert!(
        !different.verdict(Tolerance::CROSS_PRODUCER, true).is_pass(),
        "a word moved eight points was absorbed by the cross-producer tolerance: {:?}",
        different
    );
    let _ = std::fs::remove_dir_all(steady.parent().expect("a parent"));
}

#[test]
fn the_layout_tier_is_untouched_by_any_provider_exclusion() {
    use mjx_render_oracle::{ReferenceProvider, RenderedContent};
    // Stated as an assertion rather than a comment, because it is the half of the user's constraint
    // that is easiest to over-apply: the pixel tier of a gradient page is not evidence, and its
    // **word boxes still are**. Disabling a whole fixture when only its pixel tier is compromised
    // throws away the strong tier to protect the weak one.
    for provider in ReferenceProvider::ALL {
        assert_eq!(
            provider.excludes(RenderedContent::TextLayout),
            if provider == ReferenceProvider::None {
                Some(
                    "there is no reference to compare against; this run proves the pipeline and \
                     nothing about fidelity",
                )
            } else {
                None
            },
            "`{}` excludes text layout, and a word's box says nothing about how the shape behind \
             it is filled",
            provider.label()
        );
    }
}
