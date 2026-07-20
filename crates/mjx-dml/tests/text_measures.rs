//! Unit tests for the text measures — the units DrawingML states text sizes in.
//!
//! DrawingML measures text in **hundredths of a point** while every user-facing control is in
//! points, so these newtypes exist to make the factor of 100 impossible to lose. Each one is checked
//! against a value as it appears on the wire.

use mjx_dml::{FontSize, TextPoint};

#[test]
fn font_size_converts_between_points_and_the_wire_unit() {
    // `<a:defRPr sz="4400"/>` on the fixture master's title style is 44 pt.
    assert_eq!(FontSize::from_hundredths_of_a_point(4400).points(), 44.0);
    assert_eq!(FontSize::from_points(18.0).hundredths_of_a_point(), 1800);
    // Half-point sizes are exactly representable — they are what the size UI steps in.
    assert_eq!(FontSize::from_points(10.5).hundredths_of_a_point(), 1050);

    // The schema's bounds (`ST_TextFontSize`, 100..=400000) are representable at both ends.
    assert_eq!(FontSize::from_hundredths_of_a_point(100).points(), 1.0);
    assert_eq!(
        FontSize::from_hundredths_of_a_point(400_000).points(),
        4000.0
    );

    let size = FontSize::from_points(32.0);
    assert_eq!(
        FontSize::from_hundredths_of_a_point(size.hundredths_of_a_point()),
        size
    );
}

#[test]
fn text_point_carries_spacing_and_kerning_including_negatives() {
    // `a:rPr@spc` tightens with a negative value (`ST_TextPoint`, -400000..=400000).
    assert_eq!(TextPoint::from_points(-1.5).hundredths_of_a_point(), -150);
    assert_eq!(TextPoint::from_hundredths_of_a_point(-150).points(), -1.5);
    // `a:rPr@kern="1200"` — kern from 12 pt upward.
    assert_eq!(TextPoint::from_hundredths_of_a_point(1200).points(), 12.0);
    assert_eq!(TextPoint::from_points(0.0).hundredths_of_a_point(), 0);
}

#[test]
fn the_two_text_measures_are_distinct_types_ordered_by_size() {
    // Ordering exists so a caller can clamp to the schema's range without unwrapping first.
    assert!(FontSize::from_points(12.0) < FontSize::from_points(44.0));
    assert!(TextPoint::from_points(-1.0) < TextPoint::from_points(0.0));
}
