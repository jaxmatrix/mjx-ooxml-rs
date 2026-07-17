//! Unit tests for base color resolution: a `Color` resolved against a theme color scheme + color map
//! bakes down to a concrete RGB. The scheme is resolved to an interner-free `SchemeColors` first, so
//! the scheme and the color legitimately use different interners (the real cross-part scenario).
//! Transform-bearing colors are (this stage) reported unresolved.

use mjx_dml::{Color, ColorMap, ColorScheme, ResolvedColor, SchemeColors};
use mjx_ooxml_core::{FromXml, RawNode};
use mjx_xml::fidelity;

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

/// A minimal "Office"-like color scheme resolved to `SchemeColors` (parsed with its own interner):
/// `dk1` system black, `lt1`/`lt2` white, `accent1` a known sRGB.
fn office_scheme() -> SchemeColors {
    let fragment = format!(
        r#"<a:clrScheme xmlns:a="{A}" name="Office">
             <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
             <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
             <a:dk2><a:srgbClr val="44546A"/></a:dk2>
             <a:lt2><a:srgbClr val="FFFFFF"/></a:lt2>
             <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
           </a:clrScheme>"#
    );
    let doc = fidelity::parse(fragment.as_bytes()).expect("clrScheme parses");
    let scheme = ColorScheme::from_xml(&doc.root, &doc.interner).expect("ColorScheme");
    SchemeColors::from_scheme(&scheme, &doc.interner)
}

/// Parses `frag` (and an optional placeholder) under one interner — the shape's part — and resolves
/// it against the office scheme (which uses its own, theme-part interner).
#[track_caller]
fn resolve(frag: &str, map: &ColorMap, placeholder_frag: Option<&str>) -> Option<ResolvedColor> {
    let scheme = office_scheme();
    let combined = match placeholder_frag {
        Some(ph) => format!(r#"<a:wrap xmlns:a="{A}">{frag}{ph}</a:wrap>"#),
        None => format!(r#"<a:wrap xmlns:a="{A}">{frag}</a:wrap>"#),
    };
    let doc = fidelity::parse(combined.as_bytes()).expect("parses");
    let mut elements = doc.root.children.iter().filter_map(|node| match node {
        RawNode::Element(el) => Some(el.clone()),
        _ => None,
    });
    let color = Color::from_xml(&elements.next().expect("color element"), &doc.interner).unwrap();
    let placeholder = elements
        .next()
        .map(|el| Color::from_xml(&el, &doc.interner).unwrap());
    mjx_dml::resolve_color(&color, &scheme, map, placeholder.as_ref(), &doc.interner)
}

#[test]
fn srgb_resolves_directly() {
    let c = resolve(r#"<a:srgbClr val="FF0000"/>"#, &ColorMap::identity(), None).unwrap();
    assert_eq!((c.red, c.green, c.blue), (255, 0, 0));
    assert_eq!(c.to_hex(), "FF0000");
    assert_eq!(c.alpha, 1.0);
}

#[test]
fn scheme_color_resolves_through_the_scheme() {
    // accent1 (identity map: accent1 -> Accent1) -> the scheme's 4472C4.
    let c = resolve(
        r#"<a:schemeClr val="accent1"/>"#,
        &ColorMap::identity(),
        None,
    )
    .unwrap();
    assert_eq!(c.to_hex(), "4472C4");
}

#[test]
fn logical_bg1_resolves_via_the_map_to_light1() {
    // bg1 -> (identity map) Light1 -> the scheme's lt1 (white).
    let c = resolve(r#"<a:schemeClr val="bg1"/>"#, &ColorMap::identity(), None).unwrap();
    assert_eq!(c.to_hex(), "FFFFFF");
}

#[test]
fn placeholder_color_substitutes_the_fill_ref_color() {
    let c = resolve(
        r#"<a:schemeClr val="phClr"/>"#,
        &ColorMap::identity(),
        Some(r#"<a:srgbClr val="00FF00"/>"#),
    )
    .unwrap();
    assert_eq!(c.to_hex(), "00FF00");
    // With no placeholder, phClr is unresolvable.
    assert!(resolve(r#"<a:schemeClr val="phClr"/>"#, &ColorMap::identity(), None).is_none());
}

#[test]
fn placeholder_may_itself_be_a_scheme_color() {
    // phClr substituted by schemeClr(accent1) -> resolves through the scheme to 4472C4.
    let c = resolve(
        r#"<a:schemeClr val="phClr"/>"#,
        &ColorMap::identity(),
        Some(r#"<a:schemeClr val="accent1"/>"#),
    )
    .unwrap();
    assert_eq!(c.to_hex(), "4472C4");
}

#[test]
fn system_color_uses_last_clr() {
    let c = resolve(
        r#"<a:sysClr val="windowText" lastClr="000000"/>"#,
        &ColorMap::identity(),
        None,
    )
    .unwrap();
    assert_eq!(c.to_hex(), "000000");
}

#[test]
fn preset_color_resolves_from_the_named_table() {
    let c = resolve(
        r#"<a:prstClr val="cornflowerBlue"/>"#,
        &ColorMap::identity(),
        None,
    )
    .unwrap();
    assert_eq!(c.to_hex(), "6495ED");
    assert!(resolve(
        r#"<a:prstClr val="notAColor"/>"#,
        &ColorMap::identity(),
        None
    )
    .is_none());
}

#[test]
fn hsl_and_scrgb_convert() {
    // HSL hue 0°, sat 100%, lum 50% -> pure red.
    let red = resolve(
        r#"<a:hslClr hue="0" sat="100000" lum="50000"/>"#,
        &ColorMap::identity(),
        None,
    )
    .unwrap();
    assert_eq!((red.red, red.green, red.blue), (255, 0, 0));

    let black = resolve(
        r#"<a:scrgbClr r="0" g="0" b="0"/>"#,
        &ColorMap::identity(),
        None,
    )
    .unwrap();
    assert_eq!(black.to_hex(), "000000");
    let white = resolve(
        r#"<a:scrgbClr r="100000" g="100000" b="100000"/>"#,
        &ColorMap::identity(),
        None,
    )
    .unwrap();
    assert_eq!(white.to_hex(), "FFFFFF");
}

#[test]
fn transformed_color_is_unresolved_at_this_stage() {
    assert!(resolve(
        r#"<a:schemeClr val="accent1"><a:lumMod val="75000"/></a:schemeClr>"#,
        &ColorMap::identity(),
        None,
    )
    .is_none());
}
