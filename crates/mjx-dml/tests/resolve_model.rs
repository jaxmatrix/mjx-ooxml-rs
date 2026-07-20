//! Unit tests for color resolution: a `Color` (and a whole `Fill`) resolved against a theme color
//! scheme + color map bakes down to concrete RGB. The scheme is resolved to an interner-free
//! `SchemeColors` first, so the scheme and the color legitimately use different interners (the real
//! cross-part scenario).

use mjx_dml::{
    Color, ColorMap, ColorScheme, ColorSpec, EffectList, Emu, Fill, FillSpec, LineProperties,
    LineWidth, ResolvedColor, SchemeColors,
};
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
    // The placeholder (a shape's fillRef color) is pre-resolved to an interner-free ResolvedColor.
    let placeholder = elements.next().and_then(|el| {
        let ph = Color::from_xml(&el, &doc.interner).unwrap();
        mjx_dml::resolve_color(&ph, &scheme, map, None, &doc.interner)
    });
    mjx_dml::resolve_color(&color, &scheme, map, placeholder, &doc.interner)
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
fn shade_and_tint_operate_in_linear_rgb() {
    // 50% shade of white: linear 1.0*0.5 -> sRGB-encode(0.5) = 0xBC.
    let shaded = resolve(
        r#"<a:srgbClr val="FFFFFF"><a:shade val="50000"/></a:srgbClr>"#,
        &ColorMap::identity(),
        None,
    )
    .unwrap();
    assert_eq!(shaded.to_hex(), "BCBCBC");
    // 50% tint of black: linear 0*0.5 + 0.5 -> same 0xBC.
    let tinted = resolve(
        r#"<a:srgbClr val="000000"><a:tint val="50000"/></a:srgbClr>"#,
        &ColorMap::identity(),
        None,
    )
    .unwrap();
    assert_eq!(tinted.to_hex(), "BCBCBC");
}

#[test]
fn lum_mod_and_off_operate_in_hsl() {
    // lumMod 50% of pure red (HSL L 0.5 -> 0.25) -> 800000.
    let darker = resolve(
        r#"<a:srgbClr val="FF0000"><a:lumMod val="50000"/></a:srgbClr>"#,
        &ColorMap::identity(),
        None,
    )
    .unwrap();
    assert_eq!(darker.to_hex(), "800000");
    // Office "lighter": lumMod 60% then lumOff 40% (L 0.5 -> 0.3 -> 0.7) -> FF6666.
    let lighter = resolve(
        r#"<a:srgbClr val="FF0000"><a:lumMod val="60000"/><a:lumOff val="40000"/></a:srgbClr>"#,
        &ColorMap::identity(),
        None,
    )
    .unwrap();
    assert_eq!(lighter.to_hex(), "FF6666");
}

#[test]
fn alpha_offset_and_whole_color_transforms() {
    // alpha sets opacity; the hex (RGB) is unchanged.
    let faded = resolve(
        r#"<a:srgbClr val="112233"><a:alpha val="50000"/></a:srgbClr>"#,
        &ColorMap::identity(),
        None,
    )
    .unwrap();
    assert_eq!(faded.to_hex(), "112233");
    assert!((faded.alpha - 0.5).abs() < 1e-9);

    // inv, gray, comp.
    assert_eq!(
        resolve(
            r#"<a:srgbClr val="FF0000"><a:inv/></a:srgbClr>"#,
            &ColorMap::identity(),
            None
        )
        .unwrap()
        .to_hex(),
        "00FFFF"
    );
    assert_eq!(
        resolve(
            r#"<a:srgbClr val="FF0000"><a:gray/></a:srgbClr>"#,
            &ColorMap::identity(),
            None
        )
        .unwrap()
        .to_hex(),
        "4C4C4C"
    );
    assert_eq!(
        resolve(
            r#"<a:srgbClr val="FF8000"><a:comp/></a:srgbClr>"#,
            &ColorMap::identity(),
            None
        )
        .unwrap()
        .to_hex(),
        "007FFF"
    );
}

#[test]
fn transforms_apply_at_every_level_of_the_chain() {
    // phClr substituted by schemeClr(accent1) that itself carries a lumMod:
    // accent1 = 4472C4, HSL L; lumMod 50% darkens it. Resolving must honor the placeholder's own
    // transform (a value strictly darker than 4472C4, and not None).
    let c = resolve(
        r#"<a:schemeClr val="phClr"/>"#,
        &ColorMap::identity(),
        Some(r#"<a:schemeClr val="accent1"><a:lumMod val="50000"/></a:schemeClr>"#),
    )
    .unwrap();
    assert_ne!(c.to_hex(), "4472C4");
    // 4472C4 has L ~= 0.52; halving luminance darkens every channel.
    assert!(c.red < 0x44 && c.green < 0x72 && c.blue < 0xC4);
}

// ---------------------------------------------------------------------------------------------
// resolve_fill — resolving a whole fill's colors
// ---------------------------------------------------------------------------------------------

fn fill(frag: &str) -> (Fill, mjx_ooxml_core::RawDocument) {
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
        .expect("fill element");
    let fill = Fill::from_xml(&element, &doc.interner).expect("Fill");
    (fill, doc)
}

#[test]
fn resolve_fill_solid_scheme_color() {
    let scheme = office_scheme();
    let (f, doc) = fill(r#"<a:solidFill><a:schemeClr val="accent1"/></a:solidFill>"#);
    let spec = mjx_dml::resolve_fill(&f, &scheme, &ColorMap::identity(), None, &doc.interner);
    assert_eq!(spec, FillSpec::Solid(ColorSpec::Srgb("4472C4".into())));
}

#[test]
fn resolve_fill_solid_with_transform() {
    let scheme = office_scheme();
    let (f, doc) = fill(
        r#"<a:solidFill><a:schemeClr val="accent1"><a:lumMod val="50000"/></a:schemeClr></a:solidFill>"#,
    );
    let FillSpec::Solid(ColorSpec::Srgb(hex)) =
        mjx_dml::resolve_fill(&f, &scheme, &ColorMap::identity(), None, &doc.interner)
    else {
        panic!("expected a resolved solid fill");
    };
    assert_ne!(hex, "4472C4"); // darkened by lumMod
}

#[test]
fn resolve_fill_gradient_stops() {
    let scheme = office_scheme();
    let (f, doc) = fill(
        r#"<a:gradFill><a:gsLst>
             <a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs>
             <a:gs pos="100000"><a:schemeClr val="accent1"/></a:gs>
           </a:gsLst><a:lin ang="0"/></a:gradFill>"#,
    );
    let FillSpec::Gradient { stops, .. } =
        mjx_dml::resolve_fill(&f, &scheme, &ColorMap::identity(), None, &doc.interner)
    else {
        panic!("expected a gradient");
    };
    assert_eq!(stops.len(), 2);
    assert_eq!(stops[0].color, ColorSpec::Srgb("FF0000".into()));
    assert_eq!(stops[1].color, ColorSpec::Srgb("4472C4".into()));
}

#[test]
fn resolve_fill_theme_style_substitutes_placeholder() {
    // A theme fill style (solidFill of phClr) resolved with a pre-resolved fillRef color.
    let scheme = office_scheme();
    let (f, doc) = fill(r#"<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>"#);
    let placeholder = ResolvedColor {
        red: 0x00,
        green: 0xFF,
        blue: 0x00,
        alpha: 1.0,
    };
    let spec = mjx_dml::resolve_fill(
        &f,
        &scheme,
        &ColorMap::identity(),
        Some(placeholder),
        &doc.interner,
    );
    assert_eq!(spec, FillSpec::Solid(ColorSpec::Srgb("00FF00".into())));
}

#[test]
fn resolve_fill_passthrough_kinds() {
    let scheme = office_scheme();
    let (no_fill, doc) = fill(r#"<a:noFill/>"#);
    assert_eq!(
        mjx_dml::resolve_fill(
            &no_fill,
            &scheme,
            &ColorMap::identity(),
            None,
            &doc.interner
        ),
        FillSpec::None
    );
    let (grp, doc) = fill(r#"<a:grpFill/>"#);
    assert_eq!(
        mjx_dml::resolve_fill(&grp, &scheme, &ColorMap::identity(), None, &doc.interner),
        FillSpec::Group
    );
}

// ---------------------------------------------------------------------------------------------
// resolve_line — resolving an outline's stroke color, keeping structural attributes
// ---------------------------------------------------------------------------------------------

fn line(frag: &str) -> (LineProperties, mjx_ooxml_core::RawDocument) {
    let full = format!(r#"<a:wrap xmlns:a="{A}">{frag}</a:wrap>"#);
    let doc = fidelity::parse(full.as_bytes()).expect("parses");
    let element = doc
        .root
        .children
        .iter()
        .find_map(|node| match node {
            RawNode::Element(el) => Some(el.clone()),
            _ => None,
        })
        .expect("line element");
    let line = LineProperties::from_xml(&element, &doc.interner).expect("LineProperties");
    (line, doc)
}

#[test]
fn resolve_line_width_only_has_no_fill() {
    let scheme = office_scheme();
    let (l, doc) = line(r#"<a:ln w="12700" cap="rnd"/>"#);
    let spec = mjx_dml::resolve_line(&l, &scheme, &ColorMap::identity(), None, &doc.interner);
    assert_eq!(spec.width, Some(LineWidth::from_emu(12700)));
    assert_eq!(spec.cap, Some(mjx_dml::LineCap::Round));
    assert_eq!(spec.fill, None);
}

#[test]
fn resolve_line_bakes_a_scheme_stroke_color() {
    let scheme = office_scheme();
    let (l, doc) =
        line(r#"<a:ln w="9525"><a:solidFill><a:schemeClr val="accent1"/></a:solidFill></a:ln>"#);
    let spec = mjx_dml::resolve_line(&l, &scheme, &ColorMap::identity(), None, &doc.interner);
    assert_eq!(spec.width, Some(LineWidth::from_emu(9525)));
    assert_eq!(
        spec.fill,
        Some(FillSpec::Solid(ColorSpec::Srgb("4472C4".into())))
    );
}

#[test]
fn resolve_line_theme_style_substitutes_placeholder() {
    // A theme line style (w + solidFill of phClr) resolved with a pre-resolved lnRef color — the
    // a:lnRef -> lnStyleLst path. The width is preserved; the phClr stroke becomes the substitute.
    let scheme = office_scheme();
    let (l, doc) =
        line(r#"<a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>"#);
    let placeholder = ResolvedColor {
        red: 0x44,
        green: 0x72,
        blue: 0xC4,
        alpha: 1.0,
    };
    let spec = mjx_dml::resolve_line(
        &l,
        &scheme,
        &ColorMap::identity(),
        Some(placeholder),
        &doc.interner,
    );
    assert_eq!(spec.width, Some(LineWidth::from_emu(12700)));
    assert_eq!(
        spec.fill,
        Some(FillSpec::Solid(ColorSpec::Srgb("4472C4".into())))
    );
}

// ---------------------------------------------------------------------------------------------
// resolve_effects — resolving an effect list's colors, keeping structural attributes
// ---------------------------------------------------------------------------------------------

fn effects(frag: &str) -> (EffectList, mjx_ooxml_core::RawDocument) {
    let full = format!(r#"<a:wrap xmlns:a="{A}">{frag}</a:wrap>"#);
    let doc = fidelity::parse(full.as_bytes()).expect("parses");
    let element = doc
        .root
        .children
        .iter()
        .find_map(|node| match node {
            RawNode::Element(el) => Some(el.clone()),
            _ => None,
        })
        .expect("effectLst element");
    let effects = EffectList::from_xml(&element, &doc.interner).expect("EffectList");
    (effects, doc)
}

#[test]
fn resolve_effects_bakes_a_scheme_color() {
    let scheme = office_scheme();
    let (e, doc) = effects(
        r#"<a:effectLst><a:glow rad="63500"><a:schemeClr val="accent1"/></a:glow></a:effectLst>"#,
    );
    let spec = mjx_dml::resolve_effects(&e, &scheme, &ColorMap::identity(), None, &doc.interner);
    let glow = spec.glow.expect("glow");
    assert_eq!(glow.radius, Some(Emu::from_emu(63500)));
    assert_eq!(glow.color, ColorSpec::Srgb("4472C4".into()));
}

#[test]
fn resolve_effects_theme_style_substitutes_placeholder() {
    // A theme effect style (outerShdw of phClr) resolved with a pre-resolved effectRef color — the
    // a:effectRef -> effectStyleLst path. Structural attrs are preserved; the phClr becomes the substitute.
    let scheme = office_scheme();
    let (e, doc) = effects(
        r#"<a:effectLst>
             <a:outerShdw blurRad="40000" dist="20000" dir="5400000">
               <a:schemeClr val="phClr"/>
             </a:outerShdw>
           </a:effectLst>"#,
    );
    let placeholder = ResolvedColor {
        red: 0x44,
        green: 0x72,
        blue: 0xC4,
        alpha: 1.0,
    };
    let spec = mjx_dml::resolve_effects(
        &e,
        &scheme,
        &ColorMap::identity(),
        Some(placeholder),
        &doc.interner,
    );
    let shadow = spec.outer_shadow.expect("outer shadow");
    assert_eq!(shadow.blur_radius, Some(Emu::from_emu(40000)));
    assert_eq!(shadow.distance, Some(Emu::from_emu(20000)));
    assert_eq!(shadow.color, ColorSpec::Srgb("4472C4".into()));
}

#[test]
fn resolve_effects_preserves_structural_fields_and_bakes_srgb() {
    let scheme = office_scheme();
    let (e, doc) = effects(
        r#"<a:effectLst>
             <a:blur rad="25400" grow="1"/>
             <a:outerShdw blurRad="12700" dist="38100" dir="2700000"><a:srgbClr val="FF0000"/></a:outerShdw>
             <a:softEdge rad="50800"/>
           </a:effectLst>"#,
    );
    let spec = mjx_dml::resolve_effects(&e, &scheme, &ColorMap::identity(), None, &doc.interner);
    // Colorless effects copied verbatim.
    let blur = spec.blur.expect("blur");
    assert_eq!(blur.radius, Some(Emu::from_emu(25400)));
    assert_eq!(blur.grow, Some(true));
    assert_eq!(
        spec.soft_edge.expect("soft edge").radius,
        Emu::from_emu(50800)
    );
    // The shadow keeps its structural attributes and bakes its explicit sRGB unchanged.
    let shadow = spec.outer_shadow.expect("outer shadow");
    assert_eq!(shadow.blur_radius, Some(Emu::from_emu(12700)));
    assert_eq!(shadow.distance, Some(Emu::from_emu(38100)));
    assert_eq!(shadow.color, ColorSpec::Srgb("FF0000".into()));
}
