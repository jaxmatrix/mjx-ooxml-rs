//! The nine indent levels, their bullets, and the hanging indent that puts a marker in front of its
//! text.
//!
//! # Nine levels is nine values of one parameter
//!
//! A suite that demoted one paragraph would exercise level 0 and level 1 and prove nothing about
//! the other seven. So the fixture here is **all nine at once**, in one text body, and the
//! assertion is that the nine left edges are nine *distinct*, monotonically increasing numbers —
//! which is false for a renderer that reads the level and ignores it, false for one that clamps at
//! five, and false for one that resolves every level to `a:lvl1pPr`.

mod support;

use mjx_dml::{
    AutoNumberBullet, AutonumberScheme, Bullet, BulletCharacter, Emu, IndentLevel,
    ParagraphPropertiesSpec,
};
use mjx_layout::{Fragment, FragmentTree, LayoutRect};
use mjx_layout_pptx::{AutoNumberCounters, RunStyle};
use mjx_pptx::{Presentation, ShapeBounds};

use support::{blank_deck, lay_out, model, text_box};

/// A deck with one text box of nine paragraphs, each at the level of its index.
///
/// The nine levels are declared on the **shape's own `a:lstStyle`** rather than left to the master.
/// That is not a convenience: a blank deck's master states only five outline levels, so levels 6
/// through 9 would fall through to `a:defPPr` and share an indent — and a fixture where four of the
/// nine levels agree cannot tell a renderer that reads the level from one that clamps at five. It
/// also means this exercises the tier between a paragraph and its placeholder, which the master
/// alone would not.
fn nine_levels() -> (Presentation, usize, usize) {
    let (mut deck, slide) = blank_deck();
    let text: Vec<String> = (0..9).map(|level| format!("Level {level}")).collect();
    let shape = text_box(
        &mut deck,
        slide,
        &text.join("\n"),
        ShapeBounds::from_inches(0.5, 0.5, 8.0, 5.0),
    );
    for level in 0..9_u8 {
        deck.set_shape_list_style_level(
            slide,
            shape,
            IndentLevel::of(level),
            &ParagraphPropertiesSpec::new()
                .with_left_margin_points(f64::from(level) * 24.0 + 24.0)
                .with_indent_points(-18.0)
                .with_bullet(Bullet::Character(BulletCharacter::new("\u{2022}"))),
        )
        .expect("the level's list style lands");
        deck.set_paragraph_properties(
            slide,
            shape,
            usize::from(level),
            &ParagraphPropertiesSpec::new().with_level(IndentLevel::of(level)),
        )
        .expect("the level lands");
    }
    (deck, slide, shape)
}

/// The rectangle of every line, grouped by the paragraph its address names.
fn lines_by_paragraph(tree: &FragmentTree) -> Vec<(u32, LayoutRect)> {
    tree.nodes()
        .filter_map(|(_, node)| match node.fragment() {
            Fragment::Line(_) => Some((*node.source().path().segments().get(2)?, node.rect())),
            _ => None,
        })
        .collect()
}

/// The glyph runs whose path names a paragraph rather than a run — the markers.
fn markers(tree: &FragmentTree) -> Vec<(u32, LayoutRect)> {
    tree.nodes()
        .filter_map(|(_, node)| match node.fragment() {
            Fragment::GlyphRun(_) if node.source().path().depth() == 3 => {
                Some((*node.source().path().segments().get(2)?, node.rect()))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn nine_levels_indent_nine_distinct_amounts() {
    let (mut deck, slide, _) = nine_levels();
    let tree = lay_out(&mut model(), &mut deck, slide);
    let lines = lines_by_paragraph(&tree);
    assert_eq!(lines.len(), 9, "one line per level: {lines:?}");

    let lefts: Vec<i64> = lines.iter().map(|(_, rect)| rect.left.emu()).collect();
    for pair in lefts.windows(2) {
        assert!(
            pair[1] > pair[0],
            "each level indents further than the one above it: {lefts:?}"
        );
    }
    let distinct: std::collections::BTreeSet<i64> = lefts.iter().copied().collect();
    assert_eq!(distinct.len(), 9, "nine levels, nine indents: {lefts:?}");
}

#[test]
fn every_level_draws_its_own_marker_in_front_of_its_text() {
    let (mut deck, slide, _) = nine_levels();
    let tree = lay_out(&mut model(), &mut deck, slide);
    let markers = markers(&tree);
    let lines = lines_by_paragraph(&tree);
    assert_eq!(markers.len(), 9, "every one of the nine levels is bulleted");

    for (paragraph, marker) in &markers {
        let (_, line) = lines
            .iter()
            .find(|(index, _)| index == paragraph)
            .expect("every marker has a line");
        assert!(
            marker.left < line.left,
            "paragraph {paragraph}: the marker hangs in front of its text ({} against {})",
            marker.left.emu(),
            line.left.emu()
        );
        assert!(
            marker.width() > Emu::ZERO,
            "a marker with no width draws nothing"
        );
    }
}

#[test]
fn a_hanging_indent_puts_the_marker_left_of_the_margin_and_the_text_at_it() {
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        "Hanging",
        ShapeBounds::from_inches(1.0, 1.0, 6.0, 2.0),
    );
    deck.set_paragraph_properties(
        slide,
        shape,
        0,
        &ParagraphPropertiesSpec::new()
            .with_left_margin_points(72.0)
            .with_indent_points(-36.0)
            .with_bullet(Bullet::Character(BulletCharacter::new("\u{2022}"))),
    )
    .expect("the paragraph properties land");

    let tree = lay_out(&mut model(), &mut deck, slide);
    let marker = markers(&tree)[0].1;
    let line = lines_by_paragraph(&tree)[0].1;
    // The content box starts 0.1 inch in; the margin is another inch, the hang half an inch back.
    let content_left = Emu::from_inches(1.0) + Emu::from_emu(91_440);
    assert_eq!(marker.left, content_left + Emu::from_points(36.0));
    assert_eq!(line.left, content_left + Emu::from_points(72.0));
}

#[test]
fn a_marker_wider_than_its_hang_pushes_its_text_along() {
    // The case a naive implementation draws on top of itself: an automatic number that is wider than
    // the space `indent` leaves for it. The text must move right rather than the marker overlapping.
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        "Numbered",
        ShapeBounds::from_inches(1.0, 1.0, 6.0, 2.0),
    );
    deck.set_paragraph_properties(
        slide,
        shape,
        0,
        &ParagraphPropertiesSpec::new()
            .with_left_margin_points(4.0)
            .with_indent_points(-4.0)
            .with_bullet(Bullet::AutoNumber(AutoNumberBullet::new(
                AutonumberScheme::UppercaseRomanParenthesesBoth,
            ))),
    )
    .expect("the paragraph properties land");

    let tree = lay_out(&mut model(), &mut deck, slide);
    let marker = markers(&tree)[0].1;
    let line = lines_by_paragraph(&tree)[0].1;
    assert!(
        line.left >= marker.right,
        "the text starts after the marker ends: {} against {}",
        line.left.emu(),
        marker.right.emu()
    );
}

#[test]
fn no_bullet_means_no_marker_and_the_text_moves_to_the_hang() {
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        "Unmarked",
        ShapeBounds::from_inches(1.0, 1.0, 6.0, 2.0),
    );
    deck.set_paragraph_properties(
        slide,
        shape,
        0,
        &ParagraphPropertiesSpec::new()
            .with_left_margin_points(72.0)
            .with_indent_points(-36.0)
            .without_bullet(),
    )
    .expect("the paragraph properties land");

    let tree = lay_out(&mut model(), &mut deck, slide);
    assert!(markers(&tree).is_empty(), "`a:buNone` draws nothing");
    let line = lines_by_paragraph(&tree)[0].1;
    let content_left = Emu::from_inches(1.0) + Emu::from_emu(91_440);
    assert_eq!(
        line.left,
        content_left + Emu::from_points(36.0),
        "with no marker the first line starts at the hang"
    );
}

#[test]
fn an_automatic_sequence_numbers_the_paragraphs_it_is_on() {
    // The bullet whose text is *not* a property: it depends on every paragraph before it. Asserted
    // through the counter rather than through glyphs, because a glyph assertion would be an
    // assertion about a shaper version.
    let mut counters = AutoNumberCounters::new();
    let style = RunStyle {
        range: 0..0,
        family: support::FAMILY.to_owned(),
        size: mjx_text::FontSize::from_points(18.0),
        weight: mjx_text::FontWeight::REGULAR,
        slant: mjx_text::FontSlant::Upright,
        language: None,
    };
    let numbered = ParagraphPropertiesSpec::new().with_bullet(Bullet::AutoNumber(
        AutoNumberBullet::new(AutonumberScheme::ArabicPeriod),
    ));
    let texts: Vec<String> = (0..3)
        .map(|_| {
            mjx_layout_pptx::bullet::marker_for(&numbered, 0, &style, &mut counters)
                .expect("a marker")
                .text
        })
        .collect();
    assert_eq!(texts, vec!["1.", "2.", "3."]);
}

#[test]
fn a_paragraph_with_no_text_still_occupies_a_line() {
    // A reader must be able to put a caret in an empty paragraph, so it takes a line's height even
    // though there is nothing to measure.
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        "First\n\nThird",
        ShapeBounds::from_inches(1.0, 1.0, 6.0, 3.0),
    );
    let _ = shape;
    let tree = lay_out(&mut model(), &mut deck, slide);
    let lines = lines_by_paragraph(&tree);
    assert_eq!(lines.len(), 3, "three paragraphs, three lines: {lines:?}");
    let (_, empty) = lines[1];
    assert!(empty.height() > Emu::ZERO, "the empty line has a height");
    assert!(
        lines[2].1.top >= empty.bottom,
        "and the third starts below it"
    );
}
