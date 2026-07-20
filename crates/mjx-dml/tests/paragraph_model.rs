//! Unit tests for the DrawingML paragraph properties and list-style models, through the public API
//! only.
//!
//! Every round-trip assertion is paired with a typed one so byte-identity cannot pass by everything
//! landing in the opaque bucket. Two things get particular attention: the **units** (points on the
//! surface, EMU on the wire) and the **level → `lvlNpPr` off-by-one**, which exists in exactly one
//! place and is pinned here.

use mjx_dml::{
    AutoNumberBullet, AutonumberScheme, Bullet, BulletCharacter, BulletColor, BulletPicture,
    BulletSize, BulletTypeface, CharacterPropertiesSpec, ColorSpec, FontAlignment, IndentLevel,
    Paragraph, ParagraphContent, ParagraphProperties, ParagraphPropertiesSpec, TabAlignment,
    TabStop, TextAlignment, TextBody, TextFont, TextListStyle, TextSpacing,
};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, ToXml};
use mjx_xml::fidelity;

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

fn parse_typed<T: FromXml>(fragment: &[u8]) -> (T, RawDocument) {
    let doc = fidelity::parse(fragment).expect("fragment parses");
    let typed = T::from_xml(&doc.root, &doc.interner).expect("from_xml");
    (typed, doc)
}

#[track_caller]
fn assert_round_trips<T: ToXml>(typed: &T, mut doc: RawDocument, expected: &[u8]) {
    doc.root = typed.to_xml(&mut doc.interner);
    let out = fidelity::serialize_to_vec(&doc);
    assert_eq!(
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(expected),
        "round-trip byte mismatch"
    );
}

fn serialize_built<T: ToXml>(mut interner: Interner, typed: &T) -> String {
    let root = typed.to_xml(&mut interner);
    let doc = RawDocument {
        interner,
        bom: false,
        prologue: Vec::new(),
        root,
        epilogue: Vec::new(),
    };
    String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8")
}

fn serialize_edited<T: ToXml>(mut doc: RawDocument, typed: &T) -> String {
    doc.root = typed.to_xml(&mut doc.interner);
    String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8")
}

// ---------------------------------------------------------------------------------------------
// IndentLevel — the hierarchy axis
// ---------------------------------------------------------------------------------------------

#[test]
fn an_indent_level_cannot_leave_its_range() {
    assert_eq!(IndentLevel::TOP.value(), 0);
    assert_eq!(IndentLevel::of(2).value(), 2);
    assert_eq!(IndentLevel::of(8).value(), 8);
    // A literal saturates; a value off the wire is rejected outright.
    assert_eq!(IndentLevel::of(47).value(), IndentLevel::DEEPEST);
    assert_eq!(IndentLevel::new(47), None);
    assert_eq!(IndentLevel::new(8), Some(IndentLevel::of(8)));
    // Levels order as depths, which is what makes them walkable.
    assert!(IndentLevel::TOP < IndentLevel::of(1));
}

// ---------------------------------------------------------------------------------------------
// Typed reads
// ---------------------------------------------------------------------------------------------

#[test]
fn every_modeled_property_reads_back() {
    let fragment = format!(
        concat!(
            r#"<a:pPr xmlns:a="{A}" marL="457200" marR="91440" lvl="2" indent="-228600""#,
            r#" algn="just" defTabSz="914400" rtl="0" fontAlgn="base">"#,
            r#"<a:lnSpc><a:spcPct val="150000"/></a:lnSpc>"#,
            r#"<a:spcBef><a:spcPts val="600"/></a:spcBef>"#,
            r#"<a:spcAft><a:spcPct val="50000"/></a:spcAft>"#,
            r#"<a:tabLst><a:tab pos="914400" algn="ctr"/><a:tab pos="1828800" algn="dec"/></a:tabLst>"#,
            r#"<a:defRPr sz="1800" b="1"/>"#,
            r#"</a:pPr>"#
        ),
        A = A
    );
    let (properties, doc): (ParagraphProperties, _) = parse_typed(fragment.as_bytes());
    let interner = &doc.interner;

    assert_eq!(properties.level(interner), Some(IndentLevel::of(2)));
    assert_eq!(
        properties.alignment(interner),
        Some(TextAlignment::Justified)
    );
    assert_eq!(
        properties.font_alignment(interner),
        Some(FontAlignment::Baseline)
    );
    assert_eq!(properties.is_right_to_left(interner), Some(false));

    // Margins and indents read in points — 457200 EMU is 36 pt.
    assert_eq!(
        properties.left_margin(interner).map(|m| m.points()),
        Some(36.0)
    );
    assert_eq!(
        properties.right_margin(interner).map(|m| m.points()),
        Some(7.2)
    );
    assert_eq!(
        properties.indent(interner).map(|i| i.points()),
        Some(-18.0),
        "a hanging indent is negative"
    );
    assert_eq!(
        properties.default_tab_size(interner).map(|t| t.points()),
        Some(72.0)
    );

    // The two spacing kinds are genuinely different measurements.
    assert_eq!(
        properties.line_spacing(interner),
        Some(TextSpacing::proportion(1.5))
    );
    assert_eq!(
        properties.space_before(interner),
        Some(TextSpacing::points(6.0))
    );
    assert_eq!(
        properties.space_after(interner),
        Some(TextSpacing::proportion(0.5))
    );

    let stops = properties.tab_stops(interner);
    assert_eq!(stops.len(), 2);
    assert_eq!(stops[0].position_points(), 72.0);
    assert_eq!(stops[0].alignment, Some(TabAlignment::Center));
    assert_eq!(stops[1].alignment, Some(TabAlignment::Decimal));

    let default_run = properties
        .default_run_properties(interner)
        .expect("defRPr is typed as character properties");
    assert_eq!(default_run.size(interner).map(|s| s.points()), Some(18.0));
    assert_eq!(default_run.is_bold(interner), Some(true));

    assert_round_trips(&properties, doc, fragment.as_bytes());
}

#[test]
fn an_unset_property_reads_as_none_not_as_a_default() {
    let fragment = format!(r#"<a:pPr xmlns:a="{A}"/>"#);
    let (properties, doc): (ParagraphProperties, _) = parse_typed(fragment.as_bytes());
    let interner = &doc.interner;
    // An absent `lvl` means the level is inherited. Reading must not substitute level 0 — that
    // substitution belongs to resolution.
    assert_eq!(properties.level(interner), None);
    assert_eq!(properties.alignment(interner), None);
    assert_eq!(properties.left_margin(interner), None);
    assert_eq!(properties.line_spacing(interner), None);
    assert!(properties.tab_stops(interner).is_empty());
    assert_eq!(properties.default_run_properties(interner), None);
    assert_round_trips(&properties, doc, fragment.as_bytes());
}

#[test]
fn the_same_type_serves_every_name_the_complex_type_appears_under() {
    for local in ["pPr", "defPPr", "lvl1pPr", "lvl9pPr"] {
        let fragment = format!(r#"<a:{local} xmlns:a="{A}" algn="ctr" marL="342900"/>"#);
        let (properties, doc): (ParagraphProperties, _) = parse_typed(fragment.as_bytes());
        assert_eq!(
            properties.alignment(&doc.interner),
            Some(TextAlignment::Center),
            "{local}"
        );
        assert_eq!(
            properties.left_margin(&doc.interner).map(|m| m.points()),
            Some(27.0),
            "{local}"
        );
        assert_round_trips(&properties, doc, fragment.as_bytes());
    }
}

// ---------------------------------------------------------------------------------------------
// The spec builder
// ---------------------------------------------------------------------------------------------

#[test]
fn points_go_in_and_emu_comes_out() {
    let spec = ParagraphPropertiesSpec::new()
        .with_left_margin_points(36.0)
        .with_indent_points(-18.0)
        .with_default_tab_size_points(72.0);
    assert_eq!(spec.left_margin_points(), Some(36.0));
    assert_eq!(spec.indent_points(), Some(-18.0));

    let mut interner = Interner::new();
    let built = spec.to_properties(&mut interner, "pPr");
    let out = serialize_built(interner, &built);
    assert!(out.contains(r#"marL="457200""#), "{out}");
    assert!(out.contains(r#"indent="-228600""#), "{out}");
    assert!(out.contains(r#"defTabSz="914400""#), "{out}");
}

#[test]
fn a_built_element_follows_the_schema_sequence() {
    // Order is validity: lnSpc → spcBef → spcAft → tabLst → defRPr.
    let spec = ParagraphPropertiesSpec::new()
        .with_default_run_properties(CharacterPropertiesSpec::new().with_size_points(12.0))
        .with_tab_stops(vec![TabStop::at_points(72.0, TabAlignment::Left)])
        .with_space_after(TextSpacing::points(6.0))
        .with_line_spacing(TextSpacing::proportion(1.5))
        .with_space_before(TextSpacing::points(3.0));

    let mut interner = Interner::new();
    let built = spec.to_properties(&mut interner, "pPr");
    let out = serialize_built(interner, &built);

    let position = |needle: &str| {
        out.find(needle)
            .unwrap_or_else(|| panic!("{needle}: {out}"))
    };
    assert!(position("<a:lnSpc>") < position("<a:spcBef>"), "{out}");
    assert!(position("<a:spcBef>") < position("<a:spcAft>"), "{out}");
    assert!(position("<a:spcAft>") < position("<a:tabLst>"), "{out}");
    assert!(position("<a:tabLst>") < position("<a:defRPr"), "{out}");
}

#[test]
fn a_spec_round_trips_through_an_element() {
    let fragment = format!(
        r#"<a:pPr xmlns:a="{A}" lvl="3" algn="r" marL="457200"><a:lnSpc><a:spcPct val="200000"/></a:lnSpc></a:pPr>"#
    );
    let (properties, doc): (ParagraphProperties, _) = parse_typed(fragment.as_bytes());
    let spec = properties.spec(&doc.interner);
    assert_eq!(spec.level(), Some(IndentLevel::of(3)));
    assert_eq!(spec.alignment(), Some(TextAlignment::Right));
    assert_eq!(spec.left_margin_points(), Some(36.0));
    assert_eq!(spec.line_spacing(), Some(TextSpacing::proportion(2.0)));

    let mut interner = Interner::new();
    let rebuilt = spec.to_properties(&mut interner, "pPr");
    assert_eq!(rebuilt.level(&interner), Some(IndentLevel::of(3)));
    assert_eq!(
        rebuilt.line_spacing(&interner),
        Some(TextSpacing::proportion(2.0))
    );
}

// ---------------------------------------------------------------------------------------------
// Merging
// ---------------------------------------------------------------------------------------------

#[test]
fn applying_a_spec_keeps_the_state_the_model_does_not_describe() {
    // The line-breaking attributes and a bullet — none of which this model writes yet — must survive
    // a change of alignment, as must the margin the spec does not name.
    let fragment = format!(
        concat!(
            r#"<a:pPr xmlns:a="{A}" marL="457200" eaLnBrk="1" latinLnBrk="0" hangingPunct="1">"#,
            r#"<a:buChar char="•"/>"#,
            r#"</a:pPr>"#
        ),
        A = A
    );
    let (mut properties, mut doc): (ParagraphProperties, _) = parse_typed(fragment.as_bytes());
    properties.apply(
        &ParagraphPropertiesSpec::new().with_alignment(TextAlignment::Center),
        &mut doc.interner,
    );
    let out = serialize_edited(doc, &properties);

    assert!(out.contains(r#"algn="ctr""#), "{out}");
    for kept in [
        r#"marL="457200""#,
        r#"eaLnBrk="1""#,
        r#"latinLnBrk="0""#,
        r#"hangingPunct="1""#,
        r#"<a:buChar char="•"/>"#,
    ] {
        assert!(out.contains(kept), "lost {kept}: {out}");
    }
}

#[test]
fn a_new_child_lands_after_a_bullet_it_does_not_model() {
    // The bullet groups sit between the spacing elements and `a:tabLst`, so a tab list added to a
    // paragraph that already has a bullet must go *after* it.
    let fragment = format!(r#"<a:pPr xmlns:a="{A}"><a:buChar char="•"/></a:pPr>"#);
    let (mut properties, mut doc): (ParagraphProperties, _) = parse_typed(fragment.as_bytes());
    properties.apply(
        &ParagraphPropertiesSpec::new()
            .with_tab_stops(vec![TabStop::at_points(36.0, TabAlignment::Left)]),
        &mut doc.interner,
    );
    let out = serialize_edited(doc, &properties);
    assert!(
        out.find("<a:buChar").expect("bullet") < out.find("<a:tabLst>").expect("tabs"),
        "{out}"
    );
}

// ---------------------------------------------------------------------------------------------
// TextListStyle — where the off-by-one lives
// ---------------------------------------------------------------------------------------------

#[test]
fn a_level_reads_the_element_one_past_its_number() {
    // Level 0 is `lvl1pPr`; level 8 is `lvl9pPr`. This is the only place that knows it.
    let fragment = format!(
        concat!(
            r#"<a:lstStyle xmlns:a="{A}">"#,
            r#"<a:defPPr algn="l"/>"#,
            r#"<a:lvl1pPr marL="0"/>"#,
            r#"<a:lvl2pPr marL="457200"/>"#,
            r#"<a:lvl9pPr marL="3657600"/>"#,
            r#"</a:lstStyle>"#
        ),
        A = A
    );
    let (style, doc): (TextListStyle, _) = parse_typed(fragment.as_bytes());
    let interner = &doc.interner;

    assert_eq!(
        style
            .level(interner, IndentLevel::TOP)
            .and_then(|p| p.left_margin(interner))
            .map(|m| m.points()),
        Some(0.0),
        "level 0 reads lvl1pPr"
    );
    assert_eq!(
        style
            .level(interner, IndentLevel::of(1))
            .and_then(|p| p.left_margin(interner))
            .map(|m| m.points()),
        Some(36.0),
        "level 1 reads lvl2pPr"
    );
    assert_eq!(
        style
            .level(interner, IndentLevel::of(8))
            .and_then(|p| p.left_margin(interner))
            .map(|m| m.points()),
        Some(288.0),
        "level 8 reads lvl9pPr"
    );
    // A level the style does not define is absent, not defaulted.
    assert_eq!(style.level(interner, IndentLevel::of(4)), None);

    assert_eq!(
        style
            .default_properties(interner)
            .and_then(|p| p.alignment(interner)),
        Some(TextAlignment::Left)
    );
    // Only the levels actually defined are enumerated, shallowest first.
    let defined: Vec<u8> = style
        .levels(interner)
        .map(|(level, _)| level.value())
        .collect();
    assert_eq!(defined, vec![0, 1, 8]);

    assert_round_trips(&style, doc, fragment.as_bytes());
}

// ---------------------------------------------------------------------------------------------
// Typed access from the tree
// ---------------------------------------------------------------------------------------------

#[test]
fn a_paragraph_reaches_its_properties_and_keeps_their_position() {
    let fragment = format!(
        r#"<a:p xmlns:a="{A}"><a:pPr lvl="1" algn="ctr"/><a:r><a:t>Text</a:t></a:r></a:p>"#
    );
    let (paragraph, doc): (Paragraph, _) = parse_typed(fragment.as_bytes());
    assert!(matches!(
        paragraph.content()[0],
        ParagraphContent::Properties(_)
    ));
    assert_eq!(
        paragraph
            .properties()
            .expect("properties")
            .level(&doc.interner),
        Some(IndentLevel::of(1))
    );
    assert_eq!(paragraph.text(), "Text");
    assert_round_trips(&paragraph, doc, fragment.as_bytes());
}

#[test]
fn a_paragraph_without_properties_gains_them_before_its_runs() {
    let fragment = format!(r#"<a:p xmlns:a="{A}"><a:r><a:t>Text</a:t></a:r></a:p>"#);
    let (mut paragraph, mut doc): (Paragraph, _) = parse_typed(fragment.as_bytes());
    assert!(paragraph.properties().is_none());

    paragraph.set_properties(
        &ParagraphPropertiesSpec::new().with_level(IndentLevel::of(2)),
        &mut doc.interner,
    );
    let out = serialize_edited(doc, &paragraph);
    // `CT_TextParagraph` requires pPr before the runs.
    assert!(
        out.find("<a:pPr").expect("pPr") < out.find("<a:r>").expect("run"),
        "{out}"
    );
    assert!(out.contains(r#"lvl="2""#), "{out}");
    assert!(out.contains("<a:t>Text</a:t>"), "text disturbed: {out}");
}

#[test]
fn setting_properties_on_a_paragraph_that_has_them_merges() {
    let fragment = format!(
        r#"<a:p xmlns:a="{A}"><a:pPr marL="457200" hangingPunct="1"/><a:r><a:t>x</a:t></a:r></a:p>"#
    );
    let (mut paragraph, mut doc): (Paragraph, _) = parse_typed(fragment.as_bytes());
    paragraph.set_properties(
        &ParagraphPropertiesSpec::new().with_alignment(TextAlignment::Right),
        &mut doc.interner,
    );
    let out = serialize_edited(doc, &paragraph);
    assert!(out.contains(r#"algn="r""#), "{out}");
    assert!(out.contains(r#"marL="457200""#), "{out}");
    assert!(out.contains(r#"hangingPunct="1""#), "{out}");
    assert_eq!(out.matches("<a:pPr").count(), 1, "duplicated pPr: {out}");
}

#[test]
fn a_text_body_reaches_its_list_style() {
    let fragment = format!(
        concat!(
            r#"<a:txBody xmlns:a="{A}"><a:bodyPr/>"#,
            r#"<a:lstStyle><a:lvl1pPr algn="ctr"/></a:lstStyle>"#,
            r#"<a:p><a:r><a:t>x</a:t></a:r></a:p></a:txBody>"#
        ),
        A = A
    );
    let (body, doc): (TextBody, _) = parse_typed(fragment.as_bytes());
    let style = body.list_style().expect("the body declares a list style");
    assert_eq!(
        style
            .level(&doc.interner, IndentLevel::TOP)
            .and_then(|p| p.alignment(&doc.interner)),
        Some(TextAlignment::Center)
    );
    assert_eq!(body.paragraphs().count(), 1);
    assert_round_trips(&body, doc, fragment.as_bytes());
}

// ---------------------------------------------------------------------------------------------
// Bullets — four groups that inherit independently
// ---------------------------------------------------------------------------------------------

/// Parses an `a:pPr` whose body is `inner`.
fn paragraph_properties(inner: &str) -> (ParagraphProperties, RawDocument) {
    let fragment = format!(r#"<a:pPr xmlns:a="{A}">{inner}</a:pPr>"#);
    parse_typed(fragment.as_bytes())
}

#[test]
fn each_kind_of_bullet_reads_back() {
    let (properties, doc) = paragraph_properties(r#"<a:buNone/>"#);
    assert_eq!(properties.bullet(&doc.interner), Some(Bullet::None));

    let (properties, doc) = paragraph_properties(r#"<a:buChar char="•"/>"#);
    assert_eq!(
        properties.bullet(&doc.interner),
        Some(Bullet::Character(BulletCharacter::new("•")))
    );

    let (properties, doc) =
        paragraph_properties(r#"<a:buAutoNum type="arabicPeriod" startAt="3"/>"#);
    assert_eq!(
        properties.bullet(&doc.interner),
        Some(Bullet::AutoNumber(
            AutoNumberBullet::new(AutonumberScheme::ArabicPeriod).starting_at(3)
        ))
    );

    let (properties, doc) =
        paragraph_properties(r#"<a:buBlip><a:blip r:embed="rId7"/></a:buBlip>"#);
    assert_eq!(
        properties.bullet(&doc.interner),
        Some(Bullet::Picture(BulletPicture::new("rId7")))
    );
}

#[test]
fn an_autonumber_bullet_starts_at_one_by_default() {
    let (properties, doc) = paragraph_properties(r#"<a:buAutoNum type="alphaLcParenR"/>"#);
    let Some(Bullet::AutoNumber(auto)) = properties.bullet(&doc.interner) else {
        panic!("expected an auto-numbered bullet");
    };
    assert_eq!(
        auto.scheme,
        AutonumberScheme::LowercaseLetterParenthesisRight
    );
    assert_eq!(auto.start_at, 1, "the schema default");

    // …and a default `startAt` is not written back out as noise.
    let mut interner = Interner::new();
    let built = ParagraphPropertiesSpec::new()
        .with_bullet(Bullet::AutoNumber(AutoNumberBullet::new(
            AutonumberScheme::ArabicPeriod,
        )))
        .to_properties(&mut interner, "pPr");
    let out = serialize_built(interner, &built);
    assert!(out.contains(r#"type="arabicPeriod""#), "{out}");
    assert!(
        !out.contains("startAt"),
        "the default should be implicit: {out}"
    );
}

#[test]
fn follow_the_text_is_a_decision_not_an_absence() {
    // `<a:buClrTx/>` says "match the text". No group at all says "inherit whatever the level above
    // decided". They are different answers and must not collapse into one.
    let (following, doc) =
        paragraph_properties(r#"<a:buClrTx/><a:buSzTx/><a:buFontTx/><a:buChar char="•"/>"#);
    assert_eq!(
        following.bullet_color(&doc.interner),
        Some(BulletColor::FollowText)
    );
    assert_eq!(
        following.bullet_size(&doc.interner),
        Some(BulletSize::FollowText)
    );
    assert_eq!(
        following.bullet_typeface(&doc.interner),
        Some(BulletTypeface::FollowText)
    );

    let (inheriting, doc) = paragraph_properties(r#"<a:buChar char="•"/>"#);
    assert_eq!(inheriting.bullet_color(&doc.interner), None);
    assert_eq!(inheriting.bullet_size(&doc.interner), None);
    assert_eq!(inheriting.bullet_typeface(&doc.interner), None);
}

#[test]
fn bullet_sizes_read_in_both_spellings_and_write_the_spec_form() {
    // `buSzPts` is a font size.
    let (properties, doc) = paragraph_properties(r#"<a:buSzPts val="1400"/>"#);
    assert_eq!(
        properties.bullet_size(&doc.interner),
        Some(BulletSize::points(14.0))
    );

    // The schema (and ECMA §21.1.2.4.9's example) spell the percentage `"111%"`…
    let (properties, doc) = paragraph_properties(r#"<a:buSzPct val="111%"/>"#);
    assert_eq!(
        properties.bullet_size(&doc.interner),
        Some(BulletSize::percentage(1.11))
    );
    // …while the integer spelling appears in the wild, so it reads too.
    let (properties, doc) = paragraph_properties(r#"<a:buSzPct val="45000"/>"#);
    assert_eq!(
        properties.bullet_size(&doc.interner),
        Some(BulletSize::percentage(0.45))
    );

    // What we write is the spec form.
    let mut interner = Interner::new();
    let built = ParagraphPropertiesSpec::new()
        .with_bullet_size(BulletSize::percentage(1.11))
        .to_properties(&mut interner, "pPr");
    let out = serialize_built(interner, &built);
    assert!(out.contains(r#"val="111%""#), "{out}");
}

#[test]
fn a_multi_code_unit_bullet_glyph_survives() {
    // `buChar@char` is an xsd:string, not a character — a `char` would truncate this.
    let glyph = "👍🏽";
    let mut interner = Interner::new();
    let built = ParagraphPropertiesSpec::new()
        .with_bullet_character(glyph)
        .to_properties(&mut interner, "pPr");
    let out = serialize_built(interner, &built);
    assert!(out.contains(glyph), "{out}");

    // …and reads back whole from a real fragment (the built element above has no namespace
    // declaration of its own — it is meant to be spliced into a part that binds `a`).
    let (properties, doc) = paragraph_properties(&format!(r#"<a:buChar char="{glyph}"/>"#));
    assert_eq!(
        properties.bullet(&doc.interner),
        Some(Bullet::Character(BulletCharacter::new(glyph)))
    );
}

#[test]
fn a_full_bullet_specification_round_trips() {
    let fragment = format!(
        concat!(
            r#"<a:pPr xmlns:a="{A}" marL="342900" indent="-342900">"#,
            r#"<a:spcBef><a:spcPts val="600"/></a:spcBef>"#,
            r#"<a:buClr><a:srgbClr val="C00000"/></a:buClr>"#,
            r#"<a:buSzPct val="111%"/>"#,
            r#"<a:buFont typeface="Wingdings" pitchFamily="2" charset="2"/>"#,
            r#"<a:buChar char="§"/>"#,
            r#"<a:defRPr sz="1800"/>"#,
            r#"</a:pPr>"#
        ),
        A = A
    );
    let (properties, doc): (ParagraphProperties, _) = parse_typed(fragment.as_bytes());
    let interner = &doc.interner;
    assert_eq!(
        properties.bullet_color(interner),
        Some(BulletColor::Explicit(ColorSpec::Srgb("C00000".into())))
    );
    assert_eq!(
        properties.bullet_size(interner),
        Some(BulletSize::percentage(1.11))
    );
    assert_eq!(
        properties.bullet_typeface(interner),
        Some(BulletTypeface::Explicit(TextFont {
            typeface: "Wingdings".into(),
            panose: None,
            pitch_family: Some(2),
            charset: Some(2),
        }))
    );
    assert_eq!(
        properties.bullet(interner),
        Some(Bullet::Character(BulletCharacter::new("§")))
    );
    assert_round_trips(&properties, doc, fragment.as_bytes());
}

#[test]
fn the_four_groups_do_not_move_as_a_block() {
    // Setting only the character must leave an inherited-from-elsewhere colour and size alone: the
    // groups are independent, which is the whole reason they are four fields.
    let fragment = format!(
        concat!(
            r#"<a:pPr xmlns:a="{A}">"#,
            r#"<a:buClr><a:srgbClr val="C00000"/></a:buClr>"#,
            r#"<a:buSzPct val="80%"/>"#,
            r#"<a:buChar char="•"/>"#,
            r#"</a:pPr>"#
        ),
        A = A
    );
    let (mut properties, mut doc): (ParagraphProperties, _) = parse_typed(fragment.as_bytes());
    properties.apply(
        &ParagraphPropertiesSpec::new().with_bullet_character("–"),
        &mut doc.interner,
    );
    let out = serialize_edited(doc, &properties);

    assert!(out.contains(r#"char="–""#), "{out}");
    assert!(
        !out.contains(r#"char="•""#),
        "the old character should be gone: {out}"
    );
    assert!(out.contains("C00000"), "the colour was disturbed: {out}");
    assert!(
        out.contains(r#"val="80%""#),
        "the size was disturbed: {out}"
    );
    assert_eq!(out.matches("<a:buChar").count(), 1, "duplicated: {out}");
}

#[test]
fn a_bullet_lands_between_the_spacing_and_the_tab_stops() {
    // The schema puts the bullet groups after spcAft and before tabLst.
    let fragment = format!(
        concat!(
            r#"<a:pPr xmlns:a="{A}">"#,
            r#"<a:spcBef><a:spcPts val="600"/></a:spcBef>"#,
            r#"<a:tabLst><a:tab pos="914400"/></a:tabLst>"#,
            r#"</a:pPr>"#
        ),
        A = A
    );
    let (mut properties, mut doc): (ParagraphProperties, _) = parse_typed(fragment.as_bytes());
    properties.apply(
        &ParagraphPropertiesSpec::new()
            .with_bullet_character("•")
            .with_bullet_color(BulletColor::FollowText),
        &mut doc.interner,
    );
    let out = serialize_edited(doc, &properties);

    let position = |needle: &str| {
        out.find(needle)
            .unwrap_or_else(|| panic!("{needle}: {out}"))
    };
    assert!(position("<a:spcBef>") < position("<a:buClrTx/>"), "{out}");
    assert!(position("<a:buClrTx/>") < position("<a:buChar"), "{out}");
    assert!(position("<a:buChar") < position("<a:tabLst>"), "{out}");
}

// ---------------------------------------------------------------------------------------------
// Merging tiers — what an inherited property means
// ---------------------------------------------------------------------------------------------

#[test]
fn a_higher_tier_wins_and_an_unset_property_inherits() {
    let higher = ParagraphPropertiesSpec::new()
        .with_level(IndentLevel::of(1))
        .with_alignment(TextAlignment::Center);
    let lower = ParagraphPropertiesSpec::new()
        .with_alignment(TextAlignment::Left)
        .with_left_margin_points(36.0)
        .with_indent_points(-18.0);

    let merged = higher.merge_under(&lower);

    assert_eq!(merged.level(), Some(IndentLevel::of(1)));
    assert_eq!(merged.alignment(), Some(TextAlignment::Center));
    assert_eq!(merged.left_margin_points(), Some(36.0));
    assert_eq!(merged.indent_points(), Some(-18.0));
    assert_eq!(merged.right_margin_points(), None);
}

#[test]
fn merging_an_empty_tier_changes_nothing_either_way() {
    let full = ParagraphPropertiesSpec::new()
        .with_alignment(TextAlignment::Justified)
        .with_bullet_character("•");
    let empty = ParagraphPropertiesSpec::new();

    assert_eq!(full.clone().merge_under(&empty), full);
    assert_eq!(empty.merge_under(&full), full);
}

#[test]
fn each_bullet_group_inherits_independently() {
    // The higher tier names the character; the colour and size come from below. The four groups are
    // separate elements in the schema and inherit separately — a level may restyle the glyph and keep
    // the theme's colour.
    let merged = ParagraphPropertiesSpec::new()
        .with_bullet_character("–")
        .merge_under(
            &ParagraphPropertiesSpec::new()
                .with_bullet_character("•")
                .with_bullet_color(BulletColor::FollowText)
                .with_bullet_size(BulletSize::percentage(0.8)),
        );

    assert_eq!(
        merged.bullet(),
        Some(&Bullet::Character(BulletCharacter::new("–")))
    );
    assert_eq!(merged.bullet_color(), Some(&BulletColor::FollowText));
    assert_eq!(merged.bullet_size(), Some(BulletSize::percentage(0.8)));
}

#[test]
fn an_explicit_no_bullet_blocks_an_inherited_one() {
    // `<a:buNone/>` is a decision, so a paragraph that says "no bullet" stays unbulleted under a
    // master `bodyStyle` that bullets its level.
    let merged = ParagraphPropertiesSpec::new()
        .without_bullet()
        .merge_under(&ParagraphPropertiesSpec::new().with_bullet_character("•"));
    assert_eq!(merged.bullet(), Some(&Bullet::None));
}

#[test]
fn tab_stops_merge_as_one_list() {
    let higher = vec![TabStop::at_points(72.0, TabAlignment::Left)];
    let lower = vec![
        TabStop::at_points(36.0, TabAlignment::Center),
        TabStop::at_points(144.0, TabAlignment::Right),
    ];

    // A tier that names any tab stops replaces the list; `a:tabLst` is not additive.
    let merged = ParagraphPropertiesSpec::new()
        .with_tab_stops(higher.clone())
        .merge_under(&ParagraphPropertiesSpec::new().with_tab_stops(lower.clone()));
    assert_eq!(merged.tab_stops(), higher.as_slice());

    // An empty list means "unset", so the whole lower list is inherited.
    let merged = ParagraphPropertiesSpec::new()
        .merge_under(&ParagraphPropertiesSpec::new().with_tab_stops(lower.clone()));
    assert_eq!(merged.tab_stops(), lower.as_slice());
}

#[test]
fn default_run_properties_merge_recursively() {
    // A tier setting only the size must not shadow a lower tier's weight: what a level contributes to
    // `a:defRPr` is per property, exactly as it is for a run's own `a:rPr`.
    let merged = ParagraphPropertiesSpec::new()
        .with_default_run_properties(CharacterPropertiesSpec::new().with_size_points(28.0))
        .merge_under(
            &ParagraphPropertiesSpec::new().with_default_run_properties(
                CharacterPropertiesSpec::new()
                    .with_size_points(32.0)
                    .with_bold(true),
            ),
        );

    let run = merged
        .default_run_properties()
        .expect("default run properties");
    assert_eq!(run.size_points(), Some(28.0));
    assert_eq!(run.is_bold(), Some(true));
}

#[test]
fn no_bullet_is_an_override_not_an_absence() {
    let spec = ParagraphPropertiesSpec::new().without_bullet();
    assert_eq!(spec.bullet(), Some(&Bullet::None));

    let mut interner = Interner::new();
    let built = spec.to_properties(&mut interner, "pPr");
    let out = serialize_built(interner, &built);
    assert!(out.contains("<a:buNone/>"), "{out}");
}
