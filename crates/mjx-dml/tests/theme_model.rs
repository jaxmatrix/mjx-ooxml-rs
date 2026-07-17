//! Unit tests for the DrawingML theme read-view (`ColorScheme` + fill-style matrix), through the
//! public API only, against a minimal but faithful `a:theme` fragment.

use mjx_dml::{ColorKind, ColorSchemeSlot, ColorSpec, Emu, Fill, LineWidth, SchemeColor, Theme};
use mjx_ooxml_core::FromXml;
use mjx_xml::fidelity;

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

fn parse_theme(fragment: &[u8]) -> (Theme, mjx_ooxml_core::Interner) {
    let doc = fidelity::parse(fragment).expect("theme parses");
    let theme = Theme::from_xml(&doc.root, &doc.interner).expect("Theme::from_xml");
    (theme, doc.interner)
}

/// A faithful reduction of the standard "Office" theme (as in `tests/fixtures/sample.pptx`).
fn office_theme() -> String {
    format!(
        r#"<a:theme xmlns:a="{A}" name="Office"><a:themeElements>
             <a:clrScheme name="Office">
               <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
               <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
               <a:dk2><a:srgbClr val="44546A"/></a:dk2>
               <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
               <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
               <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
               <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
               <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
               <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
               <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
               <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
               <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
             </a:clrScheme>
             <a:fontScheme name="Office"><a:majorFont><a:latin typeface="Calibri Light"/></a:majorFont></a:fontScheme>
             <a:fmtScheme name="Office">
               <a:fillStyleLst>
                 <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
                 <a:gradFill><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"/></a:gs></a:gsLst><a:lin ang="5400000"/></a:gradFill>
                 <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
               </a:fillStyleLst>
               <a:lnStyleLst>
                 <a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
                 <a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
                 <a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
               </a:lnStyleLst>
               <a:effectStyleLst>
                 <a:effectStyle><a:effectLst/></a:effectStyle>
                 <a:effectStyle><a:effectLst/></a:effectStyle>
                 <a:effectStyle>
                   <a:effectLst>
                     <a:outerShdw blurRad="40000" dist="20000" dir="5400000" rotWithShape="0">
                       <a:schemeClr val="phClr"><a:alpha val="63000"/></a:schemeClr>
                     </a:outerShdw>
                   </a:effectLst>
                   <a:scene3d><a:camera prst="orthographicFront"/></a:scene3d>
                   <a:sp3d><a:bevelT w="63500" h="25400"/></a:sp3d>
                 </a:effectStyle>
               </a:effectStyleLst><a:bgFillStyleLst/>
             </a:fmtScheme>
           </a:themeElements></a:theme>"#
    )
}

#[test]
fn color_scheme_exposes_srgb_and_system_slots() {
    let (theme, interner) = parse_theme(office_theme().as_bytes());
    let scheme = theme.color_scheme().expect("color scheme");

    // accent1 is an sRGB color.
    let accent1 = scheme.color(ColorSchemeSlot::Accent1).expect("accent1");
    assert_eq!(accent1.kind(&interner), ColorKind::Srgb);
    assert_eq!(accent1.hex(&interner), Some("4472C4"));

    // dk1 is a system color; its raw `val` is the system name.
    let dark1 = scheme.color(ColorSchemeSlot::Dark1).expect("dk1");
    assert_eq!(dark1.kind(&interner), ColorKind::System);
    assert_eq!(dark1.value(&interner), Some("windowText"));

    assert_eq!(
        scheme
            .color(ColorSchemeSlot::FollowedHyperlink)
            .unwrap()
            .hex(&interner),
        Some("954F72")
    );

    // All twelve slots are present.
    assert_eq!(scheme.slots().count(), 12);
}

#[test]
fn fill_styles_are_indexed_one_based() {
    let (theme, interner) = parse_theme(office_theme().as_bytes());

    assert_eq!(theme.fill_styles().len(), 3);
    // idx 0 is the schema's "no reference".
    assert!(theme.fill_style(0).is_none());
    // idx 1 is the first style: a solidFill whose color is the placeholder color.
    let Some(Fill::Solid(solid)) = theme.fill_style(1) else {
        panic!("fill style 1 should be a solid fill");
    };
    assert_eq!(
        solid.color().unwrap().scheme_color(&interner),
        Some(SchemeColor::PlaceholderColor)
    );
    // idx 2 is the gradient.
    assert!(matches!(theme.fill_style(2), Some(Fill::Gradient(_))));
    // Out-of-range indices are absent, no panic.
    assert!(theme.fill_style(4).is_none());
}

#[test]
fn line_styles_are_indexed_one_based() {
    let (theme, interner) = parse_theme(office_theme().as_bytes());

    assert_eq!(theme.line_styles().len(), 3);
    // idx 0 is the schema's "no reference".
    assert!(theme.line_style(0).is_none());
    // idx 2 is the middle line (w=12700) whose stroke is the placeholder color.
    let ln = theme.line_style(2).expect("line style 2");
    assert_eq!(ln.width(&interner), Some(LineWidth::from_emu(12700)));
    let Some(Fill::Solid(solid)) = ln.fill(&interner) else {
        panic!("line style 2 should have a solid stroke fill");
    };
    assert_eq!(
        solid.color().unwrap().scheme_color(&interner),
        Some(SchemeColor::PlaceholderColor)
    );
    // Out-of-range indices are absent, no panic.
    assert!(theme.line_style(4).is_none());
}

#[test]
fn effect_styles_are_indexed_one_based() {
    let (theme, interner) = parse_theme(office_theme().as_bytes());

    // idx 0 is the schema's "no reference".
    assert!(theme.effect_style(0).is_none());
    // idx 1 is the first (empty) effect style — present but declares no effects.
    let first = theme.effect_style(1).expect("effect style 1");
    assert_eq!(first.outer_shadow(&interner), None);
    // idx 3 is the populated style: an outer shadow whose color is the placeholder color, with its
    // scene3d/sp3d siblings ignored.
    let third = theme.effect_style(3).expect("effect style 3");
    let shadow = third.outer_shadow(&interner).expect("outer shadow");
    assert_eq!(shadow.blur_radius, Some(Emu::from_emu(40_000)));
    assert_eq!(shadow.distance, Some(Emu::from_emu(20_000)));
    assert_eq!(
        shadow.color,
        ColorSpec::Scheme(SchemeColor::PlaceholderColor)
    );
    // Out-of-range indices are absent, no panic.
    assert!(theme.effect_style(4).is_none());
}

#[test]
fn theme_without_fmt_scheme_has_no_fill_styles() {
    let fragment = format!(
        r#"<a:theme xmlns:a="{A}"><a:themeElements>
             <a:clrScheme name="X"><a:dk1><a:srgbClr val="000000"/></a:dk1></a:clrScheme>
             <a:fontScheme name="X"/>
           </a:themeElements></a:theme>"#
    );
    let (theme, interner) = parse_theme(fragment.as_bytes());
    assert!(theme.fill_styles().is_empty());
    assert_eq!(
        theme
            .color_scheme()
            .unwrap()
            .color(ColorSchemeSlot::Dark1)
            .unwrap()
            .hex(&interner),
        Some("000000")
    );
}
