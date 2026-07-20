//! Integration tests for `Presentation::theme`: resolve a slide's theme through the
//! slide → layout → master → theme relationship chain, read its (interner-free) color scheme + fill
//! styles, and confirm reading it dirties nothing.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{ColorSchemeSlot, ColorSpec, FillSpec, FontSlot, SchemeColor, TextFont};
use mjx_opc::Package;
use mjx_pptx::Presentation;

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

#[test]
fn theme_resolves_office_color_scheme() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let theme = pres.theme(0).expect("theme").expect("fixture has a theme");

    // The fixture is the standard "Office" theme — assert its known slot colors (interner-free).
    assert_eq!(
        theme.color(ColorSchemeSlot::Accent1),
        Some(&ColorSpec::Srgb("4472C4".into()))
    );
    assert_eq!(
        theme.color(ColorSchemeSlot::FollowedHyperlink),
        Some(&ColorSpec::Srgb("954F72".into()))
    );
    // dk1/lt1 are system colors (not first-class sRGB/scheme), surfaced as `Other`.
    assert!(matches!(
        theme.color(ColorSchemeSlot::Dark1),
        Some(ColorSpec::Other { .. })
    ));
    assert_eq!(theme.colors().count(), 12);
}

#[test]
fn theme_exposes_placeholder_colored_fill_styles() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let theme = pres.theme(0).expect("theme").expect("theme");

    // The Office theme's fill styles are three placeholder-colored fills.
    assert_eq!(theme.fill_styles().len(), 3);
    assert!(theme.fill_style(0).is_none()); // idx 0 = no reference
    assert_eq!(
        theme.fill_style(1),
        Some(&FillSpec::Solid(ColorSpec::Scheme(
            SchemeColor::PlaceholderColor
        )))
    );
}

#[test]
fn theme_font_scheme_resolves_a_run_font_reference() {
    let bytes = fixture("layouts.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let theme = pres.theme(0).expect("theme").expect("fixture has a theme");
    let scheme = theme
        .font_scheme()
        .expect("fixture theme has a font scheme");

    // The fixture carries the standard "Office" font scheme.
    assert_eq!(scheme.name(), "Office");
    assert_eq!(
        scheme
            .major()
            .font(FontSlot::Latin)
            .map(|font| font.typeface.as_str()),
        Some("Calibri Light")
    );
    assert_eq!(
        scheme
            .minor()
            .font(FontSlot::Latin)
            .map(|font| font.typeface.as_str()),
        Some("Calibri")
    );

    // What a run naming `+mn-lt` — the body font — is actually drawn with.
    assert_eq!(
        scheme
            .resolve(&TextFont::named("+mn-lt"))
            .map(|font| font.typeface.as_str()),
        Some("Calibri")
    );
    assert_eq!(
        scheme
            .resolve(&TextFont::named("+mj-lt"))
            .map(|font| font.typeface.as_str()),
        Some("Calibri Light")
    );

    // Reading it dirtied nothing.
    let saved = pres.save().expect("save");
    let reopened = byte_map(&Package::open(&saved).expect("reopen"));
    for (name, original) in &snapshot {
        assert_eq!(
            reopened.get(name),
            Some(original),
            "reading the font scheme dirtied part {name}"
        );
    }
}

#[test]
fn reading_theme_keeps_all_parts_byte_identical() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let _ = pres.theme(0).expect("theme");
    let saved = pres.save().expect("save");

    // Reading the theme is non-mutating: every part is byte-identical after a save.
    let reopened = byte_map(&Package::open(&saved).expect("reopen"));
    for (name, original) in &snapshot {
        assert_eq!(
            reopened.get(name),
            Some(original),
            "reading the theme dirtied part {name}"
        );
    }
}
