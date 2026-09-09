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

// ---------------------------------------------------------------------------------------------
// The resolved scheme colour (MJXOFF-228)
// ---------------------------------------------------------------------------------------------

/// `resolved_scheme_color` answers what a token **paints**, which is not what the theme *states*.
///
/// `theme()` above hands back a `ColorSpec` per slot, and two of the twelve are `a:sysClr` — so a
/// caller asking "what is `tx1`?" gets `Other { kind: System, .. }` and no RGB. This reader is the
/// one that resolves: it takes the token a shape writes (`a:schemeClr@val`), passes it through the
/// surface's colour **map** — `tx1` is `dk1` here, and would be `lt1` on an inverted master — and
/// then through the theme.
///
/// The two steps are asserted apart. `Accent1` exercises the identity path and pins the RGB;
/// `Text1` exercises the map, and is the case a reader that skipped the map would fail, because
/// `tx1` and `dk1` are different tokens naming the same colour only *because the map says so*.
#[test]
fn a_scheme_token_resolves_through_the_map_and_the_theme() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let before = byte_map(&Package::open(&fixture("sample.pptx")).expect("baseline"));

    let accent = pres
        .resolved_scheme_color(0, SchemeColor::Accent1)
        .expect("resolving")
        .expect("the theme defines accent1");
    assert_eq!(accent.to_hex(), "4472C4", "the Office theme's own accent 1");
    assert!((accent.alpha - 1.0).abs() < f64::EPSILON);

    // `tx1` is a *mapped* token: the master's `p:clrMap` sends it to `dk1`, which this fixture
    // states as an `a:sysClr` whose `lastClr` is black. Reading the slot directly under the name
    // `Dark1` must agree, and a resolver that ignored the map would answer for the wrong slot.
    let text = pres
        .resolved_scheme_color(0, SchemeColor::Text1)
        .expect("resolving")
        .expect("the map sends tx1 to a slot the theme defines");
    let dark = pres
        .resolved_scheme_color(0, SchemeColor::Dark1)
        .expect("resolving")
        .expect("dk1 bypasses the map");
    assert_eq!(text, dark, "tx1 must resolve to what the map sends it to");
    assert_eq!(text.to_hex(), "000000");

    // `bg1` goes to `lt1`, and must not answer the same as `tx1` — the pair is the one a resolver
    // that returned a fixed slot would collapse.
    let background = pres
        .resolved_scheme_color(0, SchemeColor::Background1)
        .expect("resolving")
        .expect("the map sends bg1 to a slot the theme defines");
    assert_eq!(background.to_hex(), "FFFFFF");
    assert_ne!(background, text);

    // **And the map is load bearing.** Everything above is also true of a resolver that ignored the
    // map, because `sample.pptx`'s map is the identity one — so the same two tokens are asked again
    // of a deck whose master maps them the other way round. `tx1` must now answer white and `bg1`
    // black, which no fixed slot table can produce.
    let mut inverted = Presentation::open(&with_an_inverted_color_map()).expect("open");
    assert_eq!(
        inverted
            .resolved_scheme_color(0, SchemeColor::Text1)
            .expect("resolving")
            .expect("a slot")
            .to_hex(),
        "FFFFFF",
        "tx1 follows the map, and this master sends it to lt1"
    );
    assert_eq!(
        inverted
            .resolved_scheme_color(0, SchemeColor::Background1)
            .expect("resolving")
            .expect("a slot")
            .to_hex(),
        "000000",
        "bg1 follows the map, and this master sends it to dk1"
    );
    // `dk1` names a slot directly and bypasses the map, so it is unmoved by the inversion.
    assert_eq!(
        inverted
            .resolved_scheme_color(0, SchemeColor::Dark1)
            .expect("resolving")
            .expect("a slot")
            .to_hex(),
        "000000"
    );

    // `phClr` is not a scheme colour: it is what a style reference substitutes.
    assert_eq!(
        pres.resolved_scheme_color(0, SchemeColor::PlaceholderColor)
            .expect("resolving"),
        None
    );

    // And the whole of it is a read.
    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(after, before, "resolving a scheme colour dirtied a part");
}

/// `sample.pptx` with its master's `p:clrMap` inverted: `bg1` sent to `dk1` and `tx1` to `lt1`.
///
/// Every fixture in the corpus carries the identity map, which is the map that makes a resolver
/// ignoring the map indistinguishable from one honouring it. So this one is authored by hand —
/// swapping two attributes in the master is the whole of it, and it is a shape real decks take
/// (an inverted, dark-background master is exactly this).
fn with_an_inverted_color_map() -> Vec<u8> {
    let mut package = Package::open(&fixture("sample.pptx")).expect("open");
    let master =
        mjx_opc::PartName::new("/ppt/slideMasters/slideMaster1.xml").expect("a literal part name");
    let bytes = package.part_bytes(&master).expect("the master").to_vec();
    let markup = String::from_utf8(bytes).expect("the master is utf-8");
    let inverted = markup.replacen(
        r#"bg1="lt1" tx1="dk1""#,
        r#"bg1="dk1" tx1="lt1""#,
        1,
    );
    assert_ne!(inverted, markup, "the fixture's colour map was not found");
    package
        .replace_part_bytes(&master, inverted.into_bytes())
        .expect("replacing the master");
    package.save_unchecked().expect("saving")
}
