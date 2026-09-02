//! Unit tests for the DrawingML character (run) properties model, through the public API only.
//!
//! Every round-trip assertion is paired with a typed/structural one, so byte-identity cannot pass by
//! the model dumping everything into its opaque bucket. The tests that matter most are the ones about
//! **merging**: `a:rPr` carries state this model does not describe, and restyling a run must not lose
//! it.

use mjx_dml::{
    CharacterProperties, CharacterPropertiesSpec, ColorSpec, FillSpec, FontSlot, Fraction,
    LineSpec, LineWidth, Paragraph, RunContent, SchemeColor, TextCapitalization, TextFont, TextRun,
    TextStrike, TextUnderline, UnderlineFill, UnderlineLine,
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

/// Serializes a typed value against `interner`, for asserting on built (not parsed) markup.
fn serialize_built<T: ToXml>(mut interner: Interner, typed: &T) -> String {
    let root = typed.to_xml(&mut interner);
    let doc = RawDocument::new(interner, false, Vec::new(), root, Vec::new());
    String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8")
}

/// Re-serializes an edited value against the document's own interner.
fn serialize_edited<T: ToXml>(mut doc: RawDocument, typed: &T) -> String {
    doc.root = typed.to_xml(&mut doc.interner);
    String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8")
}

// ---------------------------------------------------------------------------------------------
// Typed reads
// ---------------------------------------------------------------------------------------------

#[test]
fn every_modeled_property_reads_back() {
    let fragment = format!(
        concat!(
            r#"<a:rPr xmlns:a="{A}" lang="en-GB" sz="1800" b="1" i="0" u="dashHeavy" strike="sngStrike""#,
            r#" cap="small" spc="-150" kern="1200" baseline="30000">"#,
            r#"<a:ln w="9525"><a:solidFill><a:srgbClr val="112233"/></a:solidFill></a:ln>"#,
            r#"<a:solidFill><a:schemeClr val="accent1"/></a:solidFill>"#,
            r#"<a:effectLst><a:glow rad="12700"><a:srgbClr val="FF0000"/></a:glow></a:effectLst>"#,
            r#"<a:highlight><a:srgbClr val="FFFF00"/></a:highlight>"#,
            r#"<a:latin typeface="Calibri" pitchFamily="34" charset="0"/>"#,
            r#"<a:ea typeface="+mn-ea"/>"#,
            r#"</a:rPr>"#
        ),
        A = A
    );
    let (properties, doc): (CharacterProperties, _) = parse_typed(fragment.as_bytes());
    let interner = &doc.interner;

    // Sizes read in points, never in the file's hundredths.
    assert_eq!(properties.size(interner).map(|s| s.points()), Some(18.0));
    assert_eq!(properties.is_bold(interner), Some(true));
    assert_eq!(properties.is_italic(interner), Some(false));
    assert_eq!(
        properties.underline(interner),
        Some(TextUnderline::HeavyDashed)
    );
    assert_eq!(properties.strike(interner), Some(TextStrike::SingleStrike));
    assert_eq!(
        properties.capitalization(interner),
        Some(TextCapitalization::Small)
    );
    assert_eq!(
        properties.spacing(interner).map(|s| s.points()),
        Some(-1.5),
        "negative spacing tightens"
    );
    assert_eq!(properties.kerning(interner).map(|k| k.points()), Some(12.0));
    assert_eq!(
        properties.baseline(interner).map(Fraction::ratio),
        Some(0.3),
        "30000 is 30% of the font size"
    );
    assert_eq!(properties.language(interner), Some("en-GB"));

    assert!(properties.fill(interner).is_some(), "text fill");
    assert_eq!(
        properties
            .outline(interner)
            .and_then(|line| line.width(interner))
            .map(LineWidth::emu),
        Some(9525)
    );
    assert!(properties.effects(interner).is_some(), "text effects");
    assert!(properties.highlight(interner).is_some(), "highlight");

    let latin = properties
        .font(interner, FontSlot::Latin)
        .expect("latin font");
    assert_eq!(latin.typeface, "Calibri");
    assert_eq!(latin.pitch_family, Some(34));
    assert!(!latin.is_theme_reference());
    let east_asian = properties
        .font(interner, FontSlot::EastAsian)
        .expect("ea font");
    assert!(
        east_asian.is_theme_reference(),
        "+mn-ea names a theme font, not a typeface"
    );

    assert_round_trips(&properties, doc, fragment.as_bytes());
}

#[test]
fn an_unset_property_reads_as_none_not_as_a_default() {
    // Unset means *inherited*. Nothing here may invent `false` or a default size.
    let fragment = format!(r#"<a:rPr xmlns:a="{A}"/>"#);
    let (properties, doc): (CharacterProperties, _) = parse_typed(fragment.as_bytes());
    let interner = &doc.interner;
    assert_eq!(properties.size(interner), None);
    assert_eq!(properties.is_bold(interner), None);
    assert_eq!(properties.is_italic(interner), None);
    assert_eq!(properties.underline(interner), None);
    assert_eq!(properties.fill(interner), None);
    assert_eq!(properties.font(interner, FontSlot::Latin), None);
    assert_round_trips(&properties, doc, fragment.as_bytes());
}

#[test]
fn the_same_type_serves_every_name_the_complex_type_appears_under() {
    // `CT_TextCharacterProperties` is `a:rPr` on a run, `a:defRPr` in paragraph properties and
    // `a:endParaRPr` on a paragraph. Each must read alike and re-emit under its own tag.
    for local in ["rPr", "defRPr", "endParaRPr"] {
        let fragment = format!(r#"<a:{local} xmlns:a="{A}" sz="2400" b="1"/>"#);
        let (properties, doc): (CharacterProperties, _) = parse_typed(fragment.as_bytes());
        assert_eq!(
            properties.size(&doc.interner).map(|s| s.points()),
            Some(24.0),
            "{local}"
        );
        assert_eq!(properties.is_bold(&doc.interner), Some(true), "{local}");
        assert_round_trips(&properties, doc, fragment.as_bytes());
    }
}

// ---------------------------------------------------------------------------------------------
// The spec builder
// ---------------------------------------------------------------------------------------------

#[test]
fn the_builder_names_only_what_it_sets() {
    let spec = CharacterPropertiesSpec::new()
        .with_size_points(28.0)
        .with_bold(true)
        .with_color(ColorSpec::Scheme(SchemeColor::Accent1));

    assert_eq!(spec.size_points(), Some(28.0));
    assert_eq!(spec.is_bold(), Some(true));
    assert!(spec.fill().is_some());
    // Everything unnamed stays unset — it inherits.
    assert_eq!(spec.is_italic(), None);
    assert_eq!(spec.underline(), None);
    assert_eq!(spec.language(), None);
}

#[test]
fn a_property_can_be_turned_off_explicitly() {
    // "Not bold" is a real value that overrides inherited bold — distinct from "unset".
    let spec = CharacterPropertiesSpec::new()
        .with_bold(false)
        .with_underline(TextUnderline::None);
    assert_eq!(spec.is_bold(), Some(false));
    assert_eq!(spec.underline(), Some(TextUnderline::None));

    let mut interner = Interner::new();
    let built = spec.to_properties(&mut interner, "rPr");
    let out = serialize_built(interner, &built);
    assert!(out.contains(r#"b="false""#), "{out}");
    assert!(out.contains(r#"u="none""#), "{out}");
}

#[test]
fn a_built_element_follows_the_schema_sequence() {
    // Order is validity, not cosmetics: attributes, then ln → fill → effectLst → highlight → fonts.
    let spec = CharacterPropertiesSpec::new()
        .with_size_points(12.0)
        .with_color(ColorSpec::Srgb("C00000".into()))
        .with_outline(LineSpec::solid(
            LineWidth::from_points(1.0),
            ColorSpec::Srgb("000000".into()),
        ))
        .with_highlight(ColorSpec::Srgb("FFFF00".into()))
        .with_font("Calibri");

    let mut interner = Interner::new();
    let built = spec.to_properties(&mut interner, "rPr");
    let out = serialize_built(interner, &built);

    let position = |needle: &str| {
        out.find(needle)
            .unwrap_or_else(|| panic!("{needle}: {out}"))
    };
    assert!(position("<a:ln") < position("<a:solidFill"), "{out}");
    assert!(position("<a:solidFill") < position("<a:highlight"), "{out}");
    assert!(position("<a:highlight") < position("<a:latin"), "{out}");
    assert!(
        out.contains(r#"sz="1200""#),
        "size is written in hundredths: {out}"
    );
}

#[test]
fn points_go_in_and_hundredths_come_out() {
    let spec = CharacterPropertiesSpec::new()
        .with_size_points(10.5)
        .with_spacing_points(-1.5)
        .with_kerning_points(12.0)
        .with_baseline(Fraction::from_ratio(-0.25));
    let mut interner = Interner::new();
    let built = spec.to_properties(&mut interner, "rPr");
    let out = serialize_built(interner, &built);
    assert!(out.contains(r#"sz="1050""#), "{out}");
    assert!(out.contains(r#"spc="-150""#), "{out}");
    assert!(out.contains(r#"kern="1200""#), "{out}");
    assert!(out.contains(r#"baseline="-25000""#), "{out}");
}

#[test]
fn a_spec_round_trips_through_an_element() {
    let fragment = format!(
        r#"<a:rPr xmlns:a="{A}" sz="1400" b="1" u="sng"><a:solidFill><a:srgbClr val="336699"/></a:solidFill></a:rPr>"#
    );
    let (properties, doc): (CharacterProperties, _) = parse_typed(fragment.as_bytes());
    let spec = properties.spec(&doc.interner);
    assert_eq!(spec.size_points(), Some(14.0));
    assert_eq!(spec.is_bold(), Some(true));
    assert_eq!(spec.underline(), Some(TextUnderline::Single));

    let mut interner = Interner::new();
    let rebuilt = spec.to_properties(&mut interner, "rPr");
    assert_eq!(rebuilt.size(&interner).map(|s| s.points()), Some(14.0));
    assert_eq!(rebuilt.is_bold(&interner), Some(true));
    assert_eq!(rebuilt.underline(&interner), Some(TextUnderline::Single));
    assert!(rebuilt.fill(&interner).is_some());
}

// ---------------------------------------------------------------------------------------------
// Merging — the reason `apply` exists
// ---------------------------------------------------------------------------------------------

#[test]
fn applying_a_spec_keeps_the_state_the_model_does_not_describe() {
    // A run as PowerPoint writes one: housekeeping attributes and a hyperlink we do not model.
    let fragment = format!(
        concat!(
            r#"<a:rPr xmlns:a="{A}" lang="en-US" sz="1800" dirty="0" err="1" smtClean="0" altLang="en-GB">"#,
            r#"<a:hlinkClick/>"#,
            r#"</a:rPr>"#
        ),
        A = A
    );
    let (mut properties, doc): (CharacterProperties, _) = parse_typed(fragment.as_bytes());

    let mut doc = doc;
    properties.apply(
        &CharacterPropertiesSpec::new().with_bold(true),
        &mut doc.interner,
    );
    let out = serialize_edited(doc, &properties);

    // What we asked for.
    assert!(out.contains(r#"b="true""#), "{out}");
    // Everything we did not: the housekeeping attributes, the unmodeled child…
    for kept in [
        r#"lang="en-US""#,
        r#"dirty="0""#,
        r#"err="1""#,
        r#"smtClean="0""#,
        r#"altLang="en-GB""#,
        "<a:hlinkClick/>",
    ] {
        assert!(out.contains(kept), "lost {kept}: {out}");
    }
    // …and the modeled property the spec did not name.
    assert!(
        out.contains(r#"sz="1800""#),
        "unnamed size was cleared: {out}"
    );
}

#[test]
fn applying_a_spec_replaces_a_child_in_place_rather_than_appending() {
    let fragment = format!(
        concat!(
            r#"<a:rPr xmlns:a="{A}">"#,
            r#"<a:solidFill><a:srgbClr val="111111"/></a:solidFill>"#,
            r#"<a:latin typeface="Arial"/>"#,
            r#"</a:rPr>"#
        ),
        A = A
    );
    let (mut properties, mut doc): (CharacterProperties, _) = parse_typed(fragment.as_bytes());
    properties.apply(
        &CharacterPropertiesSpec::new().with_color(ColorSpec::Srgb("222222".into())),
        &mut doc.interner,
    );
    let out = serialize_edited(doc, &properties);

    assert!(out.contains("222222"), "{out}");
    assert!(
        !out.contains("111111"),
        "the old fill should be gone: {out}"
    );
    assert_eq!(out.matches("<a:solidFill>").count(), 1, "duplicated: {out}");
    // The font, untouched, keeps its position after the fill.
    let fill_at = out.find("<a:solidFill>").expect("fill");
    let latin_at = out.find("<a:latin").expect("latin");
    assert!(fill_at < latin_at, "child order disturbed: {out}");
}

// ---------------------------------------------------------------------------------------------
// Typed access from a run and a paragraph
// ---------------------------------------------------------------------------------------------

#[test]
fn a_run_reaches_its_properties_and_keeps_their_position() {
    let fragment =
        format!(r#"<a:r xmlns:a="{A}"><a:rPr lang="en-US" b="1"/><a:t>Bold</a:t></a:r>"#);
    let (run, doc): (TextRun, _) = parse_typed(fragment.as_bytes());
    assert!(matches!(run.content()[0], RunContent::Properties(_)));
    assert_eq!(
        run.properties().expect("properties").is_bold(&doc.interner),
        Some(true)
    );
    assert_eq!(run.text(), "Bold");
    assert_round_trips(&run, doc, fragment.as_bytes());
}

#[test]
fn a_run_without_properties_answers_none_and_gains_them_on_set() {
    let fragment = format!(r#"<a:r xmlns:a="{A}"><a:t>Plain</a:t></a:r>"#);
    let (mut run, mut doc): (TextRun, _) = parse_typed(fragment.as_bytes());
    assert!(
        run.properties().is_none(),
        "an absent a:rPr must not be synthesized on read"
    );

    run.set_properties(
        &CharacterPropertiesSpec::new().with_size_points(20.0),
        &mut doc.interner,
    );
    let out = serialize_edited(doc, &run);
    // `CT_RegularTextRun` requires rPr before t.
    assert!(
        out.find("<a:rPr").expect("rPr") < out.find("<a:t>").expect("t"),
        "{out}"
    );
    assert!(out.contains(r#"sz="2000""#), "{out}");
    assert!(out.contains("<a:t>Plain</a:t>"), "text disturbed: {out}");
}

#[test]
fn setting_properties_on_a_run_that_has_them_merges() {
    let fragment =
        format!(r#"<a:r xmlns:a="{A}"><a:rPr lang="en-US" sz="1200"/><a:t>Text</a:t></a:r>"#);
    let (mut run, mut doc): (TextRun, _) = parse_typed(fragment.as_bytes());
    run.set_properties(
        &CharacterPropertiesSpec::new().with_italic(true),
        &mut doc.interner,
    );
    let out = serialize_edited(doc, &run);
    assert!(out.contains(r#"i="true""#), "{out}");
    assert!(out.contains(r#"lang="en-US""#), "{out}");
    assert!(out.contains(r#"sz="1200""#), "{out}");
    assert_eq!(out.matches("<a:rPr").count(), 1, "duplicated rPr: {out}");
}

#[test]
fn a_paragraph_reaches_its_end_properties() {
    // `a:endParaRPr` is how an empty paragraph still has a size.
    let fragment = format!(
        r#"<a:p xmlns:a="{A}"><a:r><a:t>x</a:t></a:r><a:endParaRPr lang="en-US" sz="3200"/></a:p>"#
    );
    let (paragraph, doc): (Paragraph, _) = parse_typed(fragment.as_bytes());
    assert_eq!(
        paragraph
            .end_properties()
            .expect("end properties")
            .size(&doc.interner)
            .map(|s| s.points()),
        Some(32.0)
    );
    assert_eq!(paragraph.runs().count(), 1);
    assert_round_trips(&paragraph, doc, fragment.as_bytes());
}

#[test]
fn a_font_carries_its_metric_hints_through() {
    let font = TextFont {
        typeface: "Cambria".into(),
        panose: Some("02040503050406030204".into()),
        pitch_family: Some(18),
        charset: Some(0),
    };
    let spec = CharacterPropertiesSpec::new().with_font_for(FontSlot::Latin, font.clone());
    assert_eq!(spec.font(FontSlot::Latin), Some(&font));

    let mut interner = Interner::new();
    let built = spec.to_properties(&mut interner, "rPr");
    assert_eq!(built.font(&interner, FontSlot::Latin), Some(font));
}

// ---------------------------------------------------------------------------------------------
// Merging tiers — what an inherited property means
// ---------------------------------------------------------------------------------------------

#[test]
fn a_higher_tier_wins_and_an_unset_property_inherits() {
    let higher = CharacterPropertiesSpec::new()
        .with_size_points(18.0)
        .with_bold(true);
    let lower = CharacterPropertiesSpec::new()
        .with_size_points(32.0)
        .with_italic(true)
        .with_language("en-US");

    let merged = higher.merge_under(&lower);

    // Named above: kept. Named only below: inherited. Named nowhere: still unset.
    assert_eq!(merged.size_points(), Some(18.0));
    assert_eq!(merged.is_bold(), Some(true));
    assert_eq!(merged.is_italic(), Some(true));
    assert_eq!(merged.language(), Some("en-US"));
    assert_eq!(merged.underline(), None);
}

#[test]
fn merging_an_empty_tier_changes_nothing_either_way() {
    let full = CharacterPropertiesSpec::new()
        .with_size_points(24.0)
        .with_underline(TextUnderline::Single);
    let empty = CharacterPropertiesSpec::new();

    // An empty tier below adds nothing; an empty tier above inherits everything.
    assert_eq!(full.clone().merge_under(&empty), full);
    assert_eq!(empty.merge_under(&full), full);
}

#[test]
fn an_explicit_off_blocks_the_tier_below() {
    // `b="0"` is a decision, not an absence — a title that says "not bold" stays unbold under a
    // master that says bold.
    let merged = CharacterPropertiesSpec::new()
        .with_bold(false)
        .merge_under(&CharacterPropertiesSpec::new().with_bold(true));
    assert_eq!(merged.is_bold(), Some(false));

    // Same for a fill: `a:noFill` is a present value.
    let merged = CharacterPropertiesSpec::new()
        .with_fill(FillSpec::None)
        .merge_under(
            &CharacterPropertiesSpec::new()
                .with_fill(FillSpec::Solid(ColorSpec::Srgb("FF0000".into()))),
        );
    assert_eq!(merged.fill(), Some(&FillSpec::None));
}

#[test]
fn fonts_merge_per_script_slot() {
    let latin = TextFont::named("Cambria");
    let east_asian = TextFont::named("Yu Gothic");
    let other_latin = TextFont::named("Calibri");

    // The higher tier names only the latin font, so the lower tier's East Asian font survives — the
    // two are separate elements, not one choice.
    let merged = CharacterPropertiesSpec::new()
        .with_font_for(FontSlot::Latin, latin.clone())
        .merge_under(
            &CharacterPropertiesSpec::new()
                .with_font_for(FontSlot::Latin, other_latin)
                .with_font_for(FontSlot::EastAsian, east_asian.clone()),
        );

    assert_eq!(merged.font(FontSlot::Latin), Some(&latin));
    assert_eq!(merged.font(FontSlot::EastAsian), Some(&east_asian));
    assert_eq!(merged.font(FontSlot::ComplexScript), None);
}

// ---------------------------------------------------------------------------------------------
// Underline line/fill groups (a:uLn / a:uLnTx / a:uFill / a:uFillTx)
// ---------------------------------------------------------------------------------------------

#[test]
fn an_explicit_underline_line_and_fill_read_from_a_run() {
    // An underline styled independently of the text: its own stroke width and its own colour.
    let fragment = format!(
        concat!(
            r#"<a:rPr xmlns:a="{A}">"#,
            r#"<a:uLn w="12700"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:uLn>"#,
            r#"<a:uFill><a:solidFill><a:srgbClr val="00FF00"/></a:solidFill></a:uFill>"#,
            r#"</a:rPr>"#
        ),
        A = A
    );
    let (properties, doc): (CharacterProperties, _) = parse_typed(fragment.as_bytes());
    let interner = &doc.interner;

    match properties.underline_line(interner) {
        Some(UnderlineLine::Explicit(line)) => {
            assert_eq!(
                line.width.map(LineWidth::emu),
                Some(12700),
                "underline width"
            );
            assert_eq!(
                line.fill,
                Some(FillSpec::Solid(ColorSpec::Srgb("FF0000".into()))),
                "underline stroke colour"
            );
        }
        other => panic!("expected an explicit underline line, got {other:?}"),
    }
    match properties.underline_fill(interner) {
        Some(UnderlineFill::Explicit(FillSpec::Solid(ColorSpec::Srgb(hex)))) => {
            assert_eq!(hex, "00FF00", "underline fill colour");
        }
        other => panic!("expected an explicit underline fill, got {other:?}"),
    }
    assert_round_trips(&properties, doc, fragment.as_bytes());
}

#[test]
fn follow_text_underline_markers_read_as_follow_text() {
    let fragment = format!(
        r#"<a:rPr xmlns:a="{A}"><a:uLnTx/><a:uFillTx/></a:rPr>"#,
        A = A
    );
    let (properties, doc): (CharacterProperties, _) = parse_typed(fragment.as_bytes());
    let interner = &doc.interner;
    assert_eq!(
        properties.underline_line(interner),
        Some(UnderlineLine::FollowText)
    );
    assert_eq!(
        properties.underline_fill(interner),
        Some(UnderlineFill::FollowText)
    );
    assert_round_trips(&properties, doc, fragment.as_bytes());
}

#[test]
fn an_unset_underline_group_reads_as_none() {
    // Unset means the underline inherits its line/fill — not a default of "follow text".
    let fragment = format!(r#"<a:rPr xmlns:a="{A}" u="sng"/>"#);
    let (properties, doc): (CharacterProperties, _) = parse_typed(fragment.as_bytes());
    let interner = &doc.interner;
    assert_eq!(properties.underline(interner), Some(TextUnderline::Single));
    assert_eq!(properties.underline_line(interner), None);
    assert_eq!(properties.underline_fill(interner), None);
    assert_round_trips(&properties, doc, fragment.as_bytes());
}

#[test]
fn building_underline_groups_places_them_in_schema_order() {
    // uLn (rank 4) and uFill (rank 5) sit after a:highlight and before the script fonts.
    let mut interner = Interner::new();
    let spec = CharacterPropertiesSpec::new()
        .with_highlight(ColorSpec::Srgb("FFFF00".into()))
        .with_underline_line(UnderlineLine::Explicit(LineSpec::solid(
            LineWidth::from_emu(9525),
            ColorSpec::Srgb("FF0000".into()),
        )))
        .with_underline_fill(UnderlineFill::Explicit(FillSpec::Solid(ColorSpec::Srgb(
            "0000FF".into(),
        ))))
        .with_font("Calibri");
    let built = spec.to_properties(&mut interner, "rPr");
    let out = serialize_built(interner, &built);

    assert!(out.contains(r#"<a:uLn w="9525">"#), "{out}");
    assert!(out.contains("<a:uFill><a:solidFill>"), "{out}");
    let highlight_at = out.find("<a:highlight").expect("highlight");
    let uln_at = out.find("<a:uLn").expect("uLn");
    let ufill_at = out.find("<a:uFill>").expect("uFill");
    let latin_at = out.find("<a:latin").expect("latin");
    assert!(
        highlight_at < uln_at && uln_at < ufill_at && ufill_at < latin_at,
        "underline groups out of schema order: {out}"
    );
}

#[test]
fn writing_one_member_of_a_group_replaces_the_other_in_place() {
    // The line group is a two-way choice: setting an explicit uLn must drop an existing uLnTx, not
    // leave both. Same for the fill group.
    let fragment = format!(
        r#"<a:rPr xmlns:a="{A}"><a:uLnTx/><a:uFillTx/></a:rPr>"#,
        A = A
    );
    let (mut properties, mut doc): (CharacterProperties, _) = parse_typed(fragment.as_bytes());
    properties.apply(
        &CharacterPropertiesSpec::new()
            .with_underline_line(UnderlineLine::Explicit(LineSpec::solid(
                LineWidth::from_emu(6350),
                ColorSpec::Srgb("123456".into()),
            )))
            .with_underline_fill(UnderlineFill::Explicit(FillSpec::Solid(ColorSpec::Srgb(
                "654321".into(),
            )))),
        &mut doc.interner,
    );
    let out = serialize_edited(doc, &properties);

    assert!(out.contains("<a:uLn "), "{out}");
    assert!(out.contains("<a:uFill>"), "{out}");
    assert!(
        !out.contains("<a:uLnTx/>"),
        "old uLnTx should be gone: {out}"
    );
    assert!(
        !out.contains("<a:uFillTx/>"),
        "old uFillTx should be gone: {out}"
    );
    assert_eq!(out.matches("<a:uLn").count(), 1, "duplicated uLn: {out}");
    assert_eq!(
        out.matches("<a:uFill").count(),
        1,
        "duplicated uFill: {out}"
    );
}

#[test]
fn setting_an_underline_group_leaves_unmodeled_state_untouched() {
    let fragment = format!(
        r#"<a:rPr xmlns:a="{A}" lang="en-US" dirty="0"><a:hlinkClick/></a:rPr>"#,
        A = A
    );
    let (mut properties, mut doc): (CharacterProperties, _) = parse_typed(fragment.as_bytes());
    properties.apply(
        &CharacterPropertiesSpec::new().with_underline_fill(UnderlineFill::FollowText),
        &mut doc.interner,
    );
    let out = serialize_edited(doc, &properties);
    assert!(out.contains("<a:uFillTx/>"), "{out}");
    for kept in [r#"lang="en-US""#, r#"dirty="0""#, "<a:hlinkClick/>"] {
        assert!(out.contains(kept), "lost {kept}: {out}");
    }
}

#[test]
fn underline_groups_merge_as_whole_values() {
    // Each group merges as a single value: a tier naming only the line leaves a lower tier's fill
    // standing, and a present "follow text" blocks the tier below.
    let higher = CharacterPropertiesSpec::new().with_underline_line(UnderlineLine::FollowText);
    let lower = CharacterPropertiesSpec::new()
        .with_underline_line(UnderlineLine::Explicit(LineSpec::solid(
            LineWidth::from_emu(9525),
            ColorSpec::Srgb("FF0000".into()),
        )))
        .with_underline_fill(UnderlineFill::FollowText);

    let merged = higher.merge_under(&lower);
    assert_eq!(
        merged.underline_line(),
        Some(&UnderlineLine::FollowText),
        "the higher tier's follow-text blocks the lower tier's explicit line"
    );
    assert_eq!(
        merged.underline_fill(),
        Some(&UnderlineFill::FollowText),
        "the fill the higher tier left unset is inherited"
    );
}

// ---------------------------------------------------------------------------------------------
// Unmodeled-state comparison (the run-coalescing safety gate)
// ---------------------------------------------------------------------------------------------

/// Parses a two-run paragraph and returns each run's properties, sharing one interner (as
/// `unmodeled_state_eq` requires). Panics if either run has no `a:rPr`.
fn two_run_properties(fragment: &str) -> (CharacterProperties, CharacterProperties, Interner) {
    let (paragraph, doc): (Paragraph, _) = parse_typed(fragment.as_bytes());
    let mut runs = paragraph.runs();
    let first = runs.next().expect("first run").properties().cloned();
    let second = runs.next().expect("second run").properties().cloned();
    (
        first.expect("first rPr"),
        second.expect("second rPr"),
        doc.interner,
    )
}

#[test]
fn unmodeled_state_ignores_modeled_and_housekeeping_differences() {
    // Same modeled attributes in a different order, and a housekeeping `dirty` on only one — the two
    // still carry the same unmodeled state, so they compare equal (this is "effective, not raw").
    let (a, b, interner) = two_run_properties(&format!(
        concat!(
            r#"<a:p xmlns:a="{A}">"#,
            r#"<a:r><a:rPr b="1" i="1" dirty="0"/><a:t>x</a:t></a:r>"#,
            r#"<a:r><a:rPr i="1" b="1"/><a:t>y</a:t></a:r>"#,
            r#"</a:p>"#
        ),
        A = A
    ));
    assert!(a.unmodeled_state_eq(&b, &interner));
    assert!(a.has_only_modeled_state(&interner));
    assert!(b.has_only_modeled_state(&interner));
}

#[test]
fn unmodeled_state_blocks_on_a_hyperlink() {
    // Identical modeled formatting, but one run carries a hyperlink — merging would drop it, so the
    // gate must report them different.
    let (a, b, interner) = two_run_properties(&format!(
        concat!(
            r#"<a:p xmlns:a="{A}" xmlns:r="{R}">"#,
            r#"<a:r><a:rPr b="1"><a:hlinkClick r:id="rId2"/></a:rPr><a:t>x</a:t></a:r>"#,
            r#"<a:r><a:rPr b="1"/><a:t>y</a:t></a:r>"#,
            r#"</a:p>"#
        ),
        A = A,
        R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
    ));
    assert!(!a.unmodeled_state_eq(&b, &interner));
    assert!(
        !a.has_only_modeled_state(&interner),
        "the hyperlink is unmodeled state"
    );
    assert!(b.has_only_modeled_state(&interner));
}

#[test]
fn unmodeled_state_blocks_on_a_foreign_attribute() {
    let (a, b, interner) = two_run_properties(&format!(
        concat!(
            r#"<a:p xmlns:a="{A}" xmlns:x="urn:example">"#,
            r#"<a:r><a:rPr b="1" x:tag="keep"/><a:t>x</a:t></a:r>"#,
            r#"<a:r><a:rPr b="1"/><a:t>y</a:t></a:r>"#,
            r#"</a:p>"#
        ),
        A = A
    ));
    assert!(!a.unmodeled_state_eq(&b, &interner));
    assert!(!a.has_only_modeled_state(&interner));
}
