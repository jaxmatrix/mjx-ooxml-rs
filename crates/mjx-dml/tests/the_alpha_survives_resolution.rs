//! **A resolved colour keeps the opacity the document stated** (MJXOFF-243, RC04).
//!
//! # What this file pins, and why it is a shape rather than a field
//!
//! Decision D1: there is **one** `ColorSpec`, and a resolved colour that is not opaque comes back as
//! `ColorSpec::Transformed { base: Srgb(hex), transforms: [Alpha(x)] }`. An opaque one stays
//! `ColorSpec::Srgb(hex)`. Every *other* transform is baked into the hex, exactly as it is today —
//! `a:lumMod` changes what the colour **is**, and `a:alpha` changes how much of it you see, so only
//! the second survives resolution as a transform.
//!
//! There is deliberately no `alpha` field on `Srgb` and no second render-colour type: a parallel
//! vocabulary beside the one the whole workspace already matches on is the patchwork this project
//! refuses.
//!
//! # The trap this file is written against
//!
//! A colour whose alpha is always `0xff` is *reached* constantly, so a reachability check — "is the
//! alpha carried?" — passes on a suite that only ever supplies one value. So the assertions below
//! supply **six distinct opacities**: `1.0`, `0.0`, `0.63` (the standard Office theme's shadow),
//! `0.35` (the corporate deck's overlay), `0.5` and `0.2`. Two of them are mid values that differ
//! from each other, which is what a stuck-at-one-value implementation cannot satisfy.
//!
//! # The count that must go to zero
//!
//! `LostOpacities` (MJXOFF-300) exists because the channel was dropped here. When it is carried
//! there is nothing to lose, so `resolve_fill_reporting_lost_opacity` must report **zero** for a
//! colour that states an `a:alpha`. A fix that carried the alpha and still counted it would leave
//! every consumer reporting an approximation it no longer makes.

use mjx_dml::{
    Color, ColorMap, ColorScheme, ColorSpec, ColorTransform, EffectList, Fill, FillSpec, Fraction,
    LineProperties, SchemeColors,
};
use mjx_ooxml_core::{FromXml, RawNode};
use mjx_xml::fidelity;

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

/// The same "Office"-like scheme `resolve_model.rs` resolves against: `accent1` is `4472C4`.
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

/// The first element of a wrapped fragment, parsed under its own interner.
fn first_element(frag: &str) -> (mjx_ooxml_core::RawElement, mjx_ooxml_core::RawDocument) {
    let full = format!(r#"<a:wrap xmlns:a="{A}" xmlns:r="http://x">{frag}</a:wrap>"#);
    let doc = fidelity::parse(full.as_bytes()).expect("parses");
    let element = doc
        .root
        .children
        .iter()
        .find_map(|node| match node {
            RawNode::Element(el) => Some(el.clone()),
            _ => None,
        })
        .expect("one element");
    (element, doc)
}

fn fill(frag: &str) -> (Fill, mjx_ooxml_core::RawDocument) {
    let (element, doc) = first_element(frag);
    let fill = Fill::from_xml(&element, &doc.interner).expect("Fill");
    (fill, doc)
}

fn line(frag: &str) -> (LineProperties, mjx_ooxml_core::RawDocument) {
    let (element, doc) = first_element(frag);
    let line = LineProperties::from_xml(&element, &doc.interner).expect("LineProperties");
    (line, doc)
}

fn effects(frag: &str) -> (EffectList, mjx_ooxml_core::RawDocument) {
    let (element, doc) = first_element(frag);
    let effects = EffectList::from_xml(&element, &doc.interner).expect("EffectList");
    (effects, doc)
}

/// The resolved form this ticket decided on: a hex triplet under one `a:alpha`.
fn translucent(hex: &str, alpha: f64) -> ColorSpec {
    ColorSpec::Transformed {
        base: Box::new(ColorSpec::Srgb(hex.to_owned())),
        transforms: vec![ColorTransform::Alpha(Fraction::from_ratio(alpha))],
    }
}

/// A solid fill's colour keeps its `a:alpha`, and an opaque one is still a bare triplet.
#[test]
fn a_solid_fills_alpha_survives_and_an_opaque_one_stays_a_triplet() {
    let scheme = office_scheme();
    let (translucent_fill, doc) = fill(
        r#"<a:solidFill><a:srgbClr val="1F3864"><a:alpha val="35000"/></a:srgbClr></a:solidFill>"#,
    );
    assert_eq!(
        mjx_dml::resolve_fill(
            &translucent_fill,
            &scheme,
            &ColorMap::identity(),
            None,
            &doc.interner
        ),
        FillSpec::Solid(translucent("1F3864", 0.35)),
        "the corporate deck's 35 % overlay resolved to something other than its colour under one \
         `a:alpha`"
    );

    let (opaque, doc) = fill(r#"<a:solidFill><a:srgbClr val="1F3864"/></a:solidFill>"#);
    assert_eq!(
        mjx_dml::resolve_fill(&opaque, &scheme, &ColorMap::identity(), None, &doc.interner),
        FillSpec::Solid(ColorSpec::Srgb("1F3864".to_owned())),
        "an opaque colour must stay a bare triplet: wrapping every colour in `Alpha(1.0)` would \
         make the transform mean nothing and break every existing `Srgb` match"
    );
}

/// A fully transparent fill is `Alpha(0.0)` and not an absent colour.
#[test]
fn a_fully_transparent_colour_is_carried_rather_than_dropped() {
    let scheme = office_scheme();
    let (invisible, doc) = fill(
        r#"<a:solidFill><a:srgbClr val="FF0000"><a:alpha val="0"/></a:srgbClr></a:solidFill>"#,
    );
    assert_eq!(
        mjx_dml::resolve_fill(
            &invisible,
            &scheme,
            &ColorMap::identity(),
            None,
            &doc.interner
        ),
        FillSpec::Solid(translucent("FF0000", 0.0)),
        "a fill at zero opacity is a fill that is hit by a click and paints nothing; resolving it \
         to `a:noFill` or to an opaque red are both wrong, and differently wrong"
    );
}

/// A stroke's colour keeps its opacity, at a value no other case in this file uses.
#[test]
fn a_lines_stroke_colour_keeps_its_alpha() {
    let scheme = office_scheme();
    let (outline, doc) = line(
        r#"<a:ln w="9525"><a:solidFill><a:schemeClr val="accent1"><a:alpha val="50000"/></a:schemeClr></a:solidFill></a:ln>"#,
    );
    let spec = mjx_dml::resolve_line(
        &outline,
        &scheme,
        &ColorMap::identity(),
        None,
        &doc.interner,
    );
    assert_eq!(
        spec.fill,
        Some(FillSpec::Solid(translucent("4472C4", 0.5))),
        "a half-opaque outline resolved to {:?}",
        spec.fill
    );
    assert_eq!(
        spec.width,
        Some(mjx_dml::LineWidth::from_emu(9525)),
        "carrying the alpha must not disturb the structural attributes the same walk copies"
    );
}

/// Each gradient stop keeps its own opacity, and two stops at different opacities stay different.
#[test]
fn every_gradient_stop_keeps_its_own_alpha() {
    let scheme = office_scheme();
    let (gradient, doc) = fill(
        r#"<a:gradFill><a:gsLst>
             <a:gs pos="0"><a:srgbClr val="FF0000"><a:alpha val="20000"/></a:srgbClr></a:gs>
             <a:gs pos="50000"><a:schemeClr val="accent1"><a:alpha val="63000"/></a:schemeClr></a:gs>
             <a:gs pos="100000"><a:srgbClr val="00FF00"/></a:gs>
           </a:gsLst><a:lin ang="0"/></a:gradFill>"#,
    );
    let FillSpec::Gradient { stops, .. } = mjx_dml::resolve_fill(
        &gradient,
        &scheme,
        &ColorMap::identity(),
        None,
        &doc.interner,
    ) else {
        panic!("expected a gradient");
    };
    let colours: Vec<ColorSpec> = stops.iter().map(|stop| stop.color.clone()).collect();
    assert_eq!(
        colours,
        vec![
            translucent("FF0000", 0.2),
            translucent("4472C4", 0.63),
            ColorSpec::Srgb("00FF00".to_owned()),
        ],
        "three stops at three opacities — 20 %, 63 % and opaque — must resolve to three different \
         colours; an implementation stuck at one value collapses them"
    );
}

/// An effect colour keeps its opacity — the standard Office theme's 63 % shadow.
#[test]
fn an_effect_colour_keeps_its_alpha() {
    let scheme = office_scheme();
    let (list, doc) = effects(
        r#"<a:effectLst>
             <a:glow rad="63500"><a:srgbClr val="FFC000"><a:alpha val="20000"/></a:srgbClr></a:glow>
             <a:outerShdw blurRad="40000" dist="20000" dir="5400000" rotWithShape="0">
               <a:schemeClr val="accent1"><a:alpha val="63000"/></a:schemeClr>
             </a:outerShdw>
           </a:effectLst>"#,
    );
    let spec = mjx_dml::resolve_effects(&list, &scheme, &ColorMap::identity(), None, &doc.interner);
    let shadow = spec.outer_shadow.expect("the list states an outer shadow");
    assert_eq!(
        shadow.color,
        translucent("4472C4", 0.63),
        "the shadow the standard Office theme puts on every styled shape resolved to {:?}; at 100 % \
         it is a solid slab under the shape instead of a soft one",
        shadow.color
    );
    assert_eq!(
        shadow.blur_radius,
        Some(mjx_dml::Emu::from_emu(40000)),
        "the structural attributes of the same effect must be untouched"
    );
    let glow = spec.glow.expect("the list states a glow");
    assert_eq!(
        glow.color,
        translucent("FFC000", 0.2),
        "two effects at two opacities must not collapse to one"
    );
}

/// A theme-slot colour carrying `a:alpha` resolves through the scheme and keeps the opacity.
#[test]
fn a_theme_slot_colour_keeps_its_alpha_through_the_scheme() {
    let scheme = office_scheme();
    let (solid, doc) = fill(
        r#"<a:solidFill><a:schemeClr val="accent1"><a:alpha val="35000"/></a:schemeClr></a:solidFill>"#,
    );
    assert_eq!(
        mjx_dml::resolve_fill(&solid, &scheme, &ColorMap::identity(), None, &doc.interner),
        FillSpec::Solid(translucent("4472C4", 0.35)),
        "the slot resolved and the opacity did not travel with it"
    );
}

/// Every other transform is baked into the hex; only the alpha survives as a transform.
#[test]
fn another_transform_is_baked_while_the_alpha_survives() {
    let scheme = office_scheme();
    // `a:lumMod` at 50 % halves red's luminance: FF0000 becomes 800000, which
    // `resolve_model.rs::luminance_transforms_match_office` already pins.
    let (mixed, doc) = fill(
        r#"<a:solidFill><a:srgbClr val="FF0000"><a:lumMod val="50000"/><a:alpha val="20000"/></a:srgbClr></a:solidFill>"#,
    );
    assert_eq!(
        mjx_dml::resolve_fill(&mixed, &scheme, &ColorMap::identity(), None, &doc.interner),
        FillSpec::Solid(translucent("800000", 0.2)),
        "the luminance modulation must be baked into the triplet and the alpha must not: a \
         `Transformed` carrying both would hand every consumer a colour model to re-implement"
    );
}

/// The alpha transforms compose to one resolved opacity, not to a chain of them.
#[test]
fn composed_alpha_transforms_resolve_to_one_value() {
    let scheme = office_scheme();
    // 80 % opacity, then multiplied by 50 %: the resolved opacity is 40 %, and it is stated once.
    let (composed, doc) = fill(
        r#"<a:solidFill><a:srgbClr val="112233"><a:alpha val="80000"/><a:alphaMod val="50000"/></a:srgbClr></a:solidFill>"#,
    );
    assert_eq!(
        mjx_dml::resolve_fill(
            &composed,
            &scheme,
            &ColorMap::identity(),
            None,
            &doc.interner
        ),
        FillSpec::Solid(translucent("112233", 0.4)),
        "resolution answers what the colour *is*, so the chain collapses to the one opacity it \
         works out to — never to the transforms as written"
    );
}

/// `resolve_color` still answers the same channels, so nothing that reads it has to change.
#[test]
fn the_resolved_colour_still_carries_the_same_alpha() {
    let scheme = office_scheme();
    let (element, doc) =
        first_element(r#"<a:srgbClr val="1F3864"><a:alpha val="35000"/></a:srgbClr>"#);
    let colour = Color::from_xml(&element, &doc.interner).expect("Color");
    let resolved =
        mjx_dml::resolve_color(&colour, &scheme, &ColorMap::identity(), None, &doc.interner)
            .expect("the colour resolves");
    assert_eq!(
        (resolved.red, resolved.green, resolved.blue),
        (0x1F, 0x38, 0x64)
    );
    assert!(
        (resolved.alpha - 0.35).abs() < 1e-9,
        "`resolve_color` answered an opacity of {}",
        resolved.alpha
    );
}

/// Nothing is lost any more, so the count that named the loss must report zero.
#[test]
fn a_carried_opacity_is_not_a_lost_one() {
    let scheme = office_scheme();
    let (translucent_fill, doc) = fill(
        r#"<a:solidFill><a:srgbClr val="1F3864"><a:alpha val="35000"/></a:srgbClr></a:solidFill>"#,
    );
    let (spec, lost) = mjx_dml::resolve_fill_reporting_lost_opacity(
        &translucent_fill,
        &scheme,
        &ColorMap::identity(),
        None,
        &doc.interner,
    );
    assert_eq!(spec, FillSpec::Solid(translucent("1F3864", 0.35)));
    assert_eq!(
        lost.count(),
        0,
        "the colour's opacity is carried, so nothing was lost; a fix that carried the alpha and \
         still counted it would leave every consumer reporting an approximation it no longer makes"
    );

    let (outline, doc) = line(
        r#"<a:ln w="9525"><a:solidFill><a:srgbClr val="1F3864"><a:alpha val="63000"/></a:srgbClr></a:solidFill></a:ln>"#,
    );
    let (_, lost) = mjx_dml::resolve_line_reporting_lost_opacity(
        &outline,
        &scheme,
        &ColorMap::identity(),
        None,
        &doc.interner,
    );
    assert_eq!(lost.count(), 0, "an outline's opacity is carried too");

    let (list, doc) = effects(
        r#"<a:effectLst><a:outerShdw blurRad="40000"><a:srgbClr val="000000"><a:alpha val="63000"/></a:srgbClr></a:outerShdw></a:effectLst>"#,
    );
    let (_, lost) = mjx_dml::resolve_effects_reporting_lost_opacity(
        &list,
        &scheme,
        &ColorMap::identity(),
        None,
        &doc.interner,
    );
    assert_eq!(lost.count(), 0, "an effect's opacity is carried too");
}

/// The suite supplies more than one opacity, held as a property of the file rather than a claim.
///
/// A colour whose alpha is always `0xff` is reached constantly at one value, and a suite that only
/// ever supplies one value cannot tell a carried channel from a constant. This counts the distinct
/// `a:alpha` values the cases above state, in the file's own text.
#[test]
fn the_suite_supplies_more_than_one_opacity() {
    let source = include_str!("the_alpha_survives_resolution.rs");
    let mut values: Vec<&str> = source
        .match_indices("<a:alpha val=\"")
        .filter_map(|(at, needle)| {
            let rest = source.get(at + needle.len()..)?;
            rest.split('"').next()
        })
        .collect();
    values.sort_unstable();
    values.dedup();
    assert!(
        values.len() >= 4,
        "this suite states {} distinct `a:alpha` values ({values:?}); with fewer than four — an \
         opaque one, a transparent one and two different mid values — it cannot tell a carried \
         channel from a constant",
        values.len()
    );
}
