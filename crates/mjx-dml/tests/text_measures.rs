//! Unit tests for the text measures — points on the surface, hundredths of a point on the wire.
//!
//! The API talks in points, because that is the unit every font-size control is in. The file talks
//! in hundredths of a point. These tests pin both halves and, above all, that a value read from a
//! file and written back is unchanged: the conversion is where a factor-of-100 bug would live.

use mjx_dml::{FontSize, TextPoint};

#[test]
fn a_font_size_is_stated_in_points() {
    assert_eq!(FontSize::from_points(18.0).points(), 18.0);
    // Half-point sizes are exact — they are what a size control steps in.
    assert_eq!(FontSize::from_points(10.5).points(), 10.5);
    assert_eq!(FontSize::from_points(11.25).points(), 11.25);
    // Sizes order as sizes.
    assert!(FontSize::from_points(12.0) < FontSize::from_points(44.0));
}

#[test]
fn spacing_is_stated_in_points_and_may_tighten() {
    // `a:rPr@spc` is negative to tighten.
    assert_eq!(TextPoint::from_points(-1.5).points(), -1.5);
    assert_eq!(TextPoint::from_points(0.0).points(), 0.0);
    assert_eq!(TextPoint::from_points(0.75).points(), 0.75);
    assert!(TextPoint::from_points(-1.0) < TextPoint::from_points(0.0));
}

#[test]
fn the_wire_form_is_hundredths_of_a_point() {
    // `<a:defRPr sz="4400"/>` on the fixture master's title style is 44 pt.
    assert_eq!(FontSize::from_wire(4400).points(), 44.0);
    assert_eq!(FontSize::from_points(18.0).to_wire(), 1800);
    assert_eq!(FontSize::from_points(10.5).to_wire(), 1050);
    // `a:rPr@kern="1200"` — kern from 12 pt upward; `@spc="-150"` tightens by 1.5 pt.
    assert_eq!(TextPoint::from_wire(1200).points(), 12.0);
    assert_eq!(TextPoint::from_points(-1.5).to_wire(), -150);
}

#[test]
fn a_value_read_from_a_file_writes_back_unchanged() {
    // Fidelity: every hundredth in the schema's range survives the trip through points, including
    // the bounds (`ST_TextFontSize` is 100..=400000) and values no round point can name.
    for wire in [100, 101, 999, 1050, 1234, 4400, 39_999, 400_000] {
        let size = FontSize::from_wire(wire);
        assert_eq!(size.to_wire(), wire, "font size {wire} changed");
        assert_eq!(FontSize::from_points(size.points()).to_wire(), wire);
    }
    // `ST_TextPoint` spans ±400000 and is signed.
    for wire in [-400_000, -150, -1, 0, 1, 1200, 400_000] {
        let measure = TextPoint::from_wire(wire);
        assert_eq!(measure.to_wire(), wire, "text point {wire} changed");
        assert_eq!(TextPoint::from_points(measure.points()).to_wire(), wire);
    }
}
