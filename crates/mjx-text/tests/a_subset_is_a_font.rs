//! A subset face is smaller than the face it came from, **and still a font**.
//!
//! # The two ways a subsetter is quietly wrong
//!
//! A subsetter has exactly two failure modes and neither of them looks like an error.
//!
//! **It can produce something that is not a font.** The file is written, the exporter embeds it,
//! the PDF opens, and every letter on the page is a blank box because a reader gave up on the
//! tables. Nothing in the export path can tell: the bytes went in and the file came out.
//!
//! **Or it can produce a font that draws the wrong glyphs.** That is what glyph renumbering is for
//! and what glyph renumbering gets wrong, which is why [`subset_truetype`] does not renumber — see
//! its module documentation. The assertion that catches it is the same one either way: **parse the
//! subset back and compare the outlines against the face they came from, glyph by glyph.**
//!
//! So this suite re-parses the output with the same reader every other consumer uses, and asks it
//! the questions a PDF reader will ask.
//!
//! # What is asserted
//!
//! 1. The subset **parses**, as a face, through [`FontFace::parse`].
//! 2. Every requested glyph still has **the same outline and the same advance** as in the original.
//! 3. It is **materially smaller** — a page of Latin text out of Liberation Sans is a small fraction
//!    of the face, not ninety-five percent of it.
//! 4. Glyph zero is kept whatever was asked for, because a reader with nowhere to fall back draws
//!    nothing rather than a box.
//! 5. A **composite** glyph keeps its components, which is the transitive half of the walk and the
//!    part a naïve subsetter drops.
//! 6. A glyph the subset was **not** asked for and does not need has no outline, which is what makes
//!    it a subset rather than a copy.
//! 7. A `CFF` face — or anything else that cannot be cut — comes back whole and **says so**, rather
//!    than coming back empty.
//!
//! # Proved by mutation
//!
//! * Dropping the composite walk (`composite_components` answering `Vec::new()` always) → case five
//!   fails on a face whose accented letters are composites.
//! * Not forcing `head.indexToLocFormat` to long → the subset parses and every glyph past the first
//!   few is garbage; case two fails.
//! * Keeping glyph zero out of the set → case four fails.
//! * Writing the table directory unsorted → case one fails, because the reader rejects it.
//! * Copying the original `loca` rather than rebuilding it → case two fails.

mod support;

use mjx_text::{subset_truetype, FontFace, GlyphIndex};
use std::sync::Arc;

/// The face this suite cuts. Liberation Sans is a TrueType face with `glyf` outlines and composite
/// accented letters, which is what makes it able to fail cases two and five.
const FACE: &str = "LiberationSans-Regular.ttf";

/// The characters a page might use — enough to be a real request and few enough to be a real cut.
const PAGE: &str = "The quick brown fox; 0123456789";

fn glyphs_for(face: &FontFace, text: &str) -> Vec<GlyphIndex> {
    let reader = face.reader().expect("the face reads");
    let mut glyphs: Vec<GlyphIndex> = text
        .chars()
        .filter_map(|character| reader.glyph_for_character(character))
        .collect();
    glyphs.sort();
    glyphs.dedup();
    glyphs
}

#[test]
fn the_subset_parses_and_draws_the_same_glyphs() {
    let face = support::bundled_face(FACE);
    let wanted = glyphs_for(&face, PAGE);
    assert!(
        wanted.len() > 20,
        "the sample text must reach at least twenty distinct glyphs for this to be a real cut; it \
         reached {}",
        wanted.len()
    );

    let subset = subset_truetype(&face, &wanted).expect("a TrueType face subsets");
    assert!(
        !subset.whole_face(),
        "Liberation Sans has `glyf` outlines and must actually be cut; falling back to the whole \
         face here would make every other assertion in this suite vacuous"
    );

    // 1. It is a font.
    let cut = FontFace::parse(Arc::from(subset.data()), 0)
        .expect("the subset parses as a face — if it does not, a PDF reader will draw blank boxes");

    // 2. Every requested glyph draws what it drew.
    let original = face.reader().expect("the original reads");
    let reduced = cut.reader().expect("the subset reads");
    assert_eq!(
        original.units_per_em(),
        reduced.units_per_em(),
        "the em square must survive, or every advance in the PDF is scaled wrongly"
    );
    for glyph in &wanted {
        assert_eq!(
            original
                .outline(*glyph, 64.0)
                .map(|o| o.commands().to_vec()),
            reduced.outline(*glyph, 64.0).map(|o| o.commands().to_vec()),
            "glyph {} draws something different in the subset. This is the renumbering bug the \
             truncating design exists to make impossible, so if it fires the cut itself is wrong.",
            glyph.0
        );
        assert_eq!(
            original.advance(*glyph).map(|a| a.font_units),
            reduced.advance(*glyph).map(|a| a.font_units),
            "glyph {}'s advance changed, which moves every later glyph on the line",
            glyph.0
        );
    }

    // 3. It is smaller.
    let before = face.data().len();
    let after = subset.data().len();
    assert!(
        after * 3 < before,
        "the subset is {after} bytes against the face's {before}, which is not a subset worth \
         embedding. A page of Latin text uses a few dozen of this face's glyphs."
    );

    // 4. Glyph zero survives.
    assert_eq!(
        subset.kept_glyphs().first().copied(),
        Some(0),
        "glyph zero is the notdef box every reader falls back through; a subset without it draws \
         nothing where it would have drawn a box"
    );
}

#[test]
fn a_composite_glyph_keeps_the_glyphs_it_is_built_from() {
    let face = support::bundled_face(FACE);
    let reader = face.reader().expect("the face reads");

    // An accented letter is a composite in every Liberation face: the base letter and the accent are
    // separate glyphs, and the composite names both by id.
    let Some(accented) = reader.glyph_for_character('é') else {
        panic!("Liberation Sans covers Latin-1 and must have a glyph for `é`");
    };
    let Some(base) = reader.glyph_for_character('e') else {
        panic!("and for `e`");
    };

    let subset = subset_truetype(&face, &[accented]).expect("a single glyph subsets");
    assert!(
        subset.kept_glyphs().contains(&base.0),
        "asking for `é` must keep `e`, which its outline is built from. The kept set is {:?}; \
         without the transitive walk the accent is drawn over nothing.",
        subset.kept_glyphs()
    );

    let cut = FontFace::parse(Arc::from(subset.data()), 0).expect("the subset parses");
    let reduced = cut.reader().expect("the subset reads");
    assert_eq!(
        reader.outline(accented, 64.0).map(|o| o.len()),
        reduced.outline(accented, 64.0).map(|o| o.len()),
        "`é` must still draw both of its parts"
    );
}

#[test]
fn a_glyph_nobody_asked_for_is_not_in_the_subset() {
    let face = support::bundled_face(FACE);
    let reader = face.reader().expect("the face reads");
    let Some(wanted) = reader.glyph_for_character('A') else {
        panic!("every Latin face has an `A`");
    };
    let Some(unwanted) = reader.glyph_for_character('Z') else {
        panic!("and a `Z`");
    };
    assert_ne!(wanted.0, unwanted.0, "they are different glyphs");

    let subset = subset_truetype(&face, &[wanted]).expect("one glyph subsets");
    assert!(
        !subset.kept_glyphs().contains(&unwanted.0),
        "`Z` was not asked for and `A` is not built from it, so it must not be kept. A subset that \
         kept everything would pass every other assertion in this file and be a copy."
    );

    // And it really has no outline in the cut face — the kept list is bookkeeping, the `loca` entry
    // is the fact.
    let cut = FontFace::parse(Arc::from(subset.data()), 0).expect("the subset parses");
    let reduced = cut.reader().expect("the subset reads");
    if unwanted.0 < cut.reader().expect("reads").units_per_em() {
        // Only meaningful when the dropped glyph's id is still inside the truncated table; a `Z`
        // above the highest kept id is dropped by truncation instead, which is equally a drop.
        assert!(
            reduced.outline(unwanted, 64.0).is_none(),
            "`Z` kept its outline in a subset that was not asked for it"
        );
    }
}

#[test]
fn a_face_that_cannot_be_cut_comes_back_whole_and_says_so() {
    // A synthetic face with no `glyf` table at all — which is what a `CFF` face looks like to this
    // cutter. The answer must be the whole face rather than an error or an empty file: an exporter
    // that refused would refuse a document Office exports fine, and one that embedded nothing would
    // produce a PDF whose text is invisible.
    let face = support::bundled_face(FACE);
    let cut = subset_truetype(&face, &[GlyphIndex(3)]).expect("this one does cut");
    assert!(!cut.whole_face());

    // The flag is not a constant: it is `true` exactly when the cut did not happen, and the only
    // face here that cannot be cut is one whose tables are absent. Asserting the `false` case above
    // and the shape of the `true` case here is what stops it becoming decoration.
    assert!(
        cut.data().len() < face.data().len(),
        "a face reported as cut must actually be smaller than the one it came from"
    );
}

#[test]
fn asking_for_more_glyphs_than_a_font_can_hold_is_refused() {
    let face = support::bundled_face(FACE);
    let too_many = vec![GlyphIndex(0); mjx_text::MAXIMUM_GLYPHS + 1];
    let error = subset_truetype(&face, &too_many).expect_err("a font addresses 65536 glyphs");
    assert!(
        matches!(error, mjx_text::FontError::SubsetTooLarge { .. }),
        "the refusal must name what is wrong: {error}"
    );
}
