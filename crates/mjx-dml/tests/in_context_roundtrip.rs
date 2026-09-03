//! The gate for `mjx-dml`'s typed layer: every retrofitted type round-trips **in context** — a real
//! element out of a real part, through `from_xml` / `to_xml`, back into the part, whole part
//! byte-identical.
//!
//! # Why this suite and not the round-trip tiers
//!
//! `mjx-opc`'s `roundtrip` suite is part-level copy-on-write: a part nobody edited re-emits its
//! stored bytes and no model runs. Its `tree_roundtrip` suite runs `mjx-xml`'s fidelity reader and
//! writer, not `mjx-dml`. **Neither executes a line of this crate's typed layer**, so "the round-trip
//! tiers did not move" says nothing about it. This file is where they are made to.
//!
//! # Why the corpus disagrees with the writer
//!
//! Every committed fixture was authored here or by LibreOffice, so every `ST_OnOff` in the corpus is
//! already spelled the way this project writes one, every percentage is already in the integer form,
//! every value is already double-quoted. A byte-identity assertion over that corpus cannot see a
//! normalizing bug: the writer agrees with the corpus because we wrote both.
//!
//! So the fixtures are joined by [committed literals](#the-disagreeing-corpus) written in forms this
//! project's writer would *never* produce — `rotWithShape='on'`, `sx="105%"`, a single-quoted value,
//! an unknown attribute *between* two known ones, a namespaced attribute, a character reference in a
//! value a model actually reads — and the same byte-identity assertion is made over those.

use std::borrow::Cow;

use mjx_dml::{
    Color, EffectList, GradientFill, LineDash, LineProperties, LineWidth, PictureFill, Scene3D,
    Shape3D, SolidFill, TextBody, TextBodyContent,
};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, RawElement, RawNode, ToXml};
use mjx_opc::{Package, PartName};
use mjx_xml::fidelity;

/// Depth-first search for the first element satisfying `predicate`, returning a mutable slot.
fn find_element_mut<'a>(
    element: &'a mut RawElement,
    predicate: &impl Fn(&RawElement) -> bool,
) -> Option<&'a mut RawElement> {
    if predicate(element) {
        return Some(element);
    }
    for child in &mut element.children {
        if let RawNode::Element(child_element) = child {
            if let Some(found) = find_element_mut(child_element, predicate) {
                return Some(found);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------------------------

/// Lifts the first element `wanted` accepts out of `part` of `fixture`, runs it through
/// `T::from_xml` → `to_xml`, puts the result back where it came from, and asserts the **whole part**
/// re-serializes to the exact bytes it was read with.
///
/// `inspect` runs on the parsed value *before* the replacement, and is where a case proves the model
/// really read the element rather than carrying an opaque copy of it — without which byte identity
/// would be satisfied by a type that models nothing at all.
#[track_caller]
fn round_trips_in_context<T: FromXml + ToXml>(
    fixture: &str,
    part: &str,
    wanted: impl Fn(&RawElement, &Interner) -> bool,
    inspect: impl FnOnce(&T, &Interner),
) {
    let package = Package::open(&mjx_fixtures::fixture(fixture)).expect("the fixture opens");
    let part_name = PartName::new(part).expect("a valid part name");
    let original = package
        .part_bytes(&part_name)
        .unwrap_or_else(|| panic!("{fixture} has no {part}"))
        .to_vec();

    round_trips_in_document::<T>(&original, wanted, inspect);
}

/// [`round_trips_in_context`] over bytes that are already in hand — a part lifted from a package, or
/// one of the [disagreeing literals](#the-disagreeing-corpus).
#[track_caller]
fn round_trips_in_document<T: FromXml + ToXml>(
    original: &[u8],
    wanted: impl Fn(&RawElement, &Interner) -> bool,
    inspect: impl FnOnce(&T, &Interner),
) {
    let mut document = fidelity::parse(original).expect("the markup is well-formed");
    // Split-borrow: `interner` shared, to resolve names; `root` mutable, to locate and replace.
    let RawDocument { interner, root, .. } = &mut document;
    let slot = find_element_mut(root, &|element| wanted(element, interner))
        .expect("the markup contains the element this case is about");

    let typed = T::from_xml(slot, interner).expect("from_xml succeeds");
    inspect(&typed, interner);

    *slot = typed.to_xml(interner);
    let out = fidelity::serialize_to_vec(&document);
    assert_eq!(
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(original),
        "not byte-identical after a typed round-trip"
    );
}

/// A predicate matching the first element with this local name.
fn named(local: &'static str) -> impl Fn(&RawElement, &Interner) -> bool {
    move |element, interner| interner.resolve(element.name.local) == local
}

/// A predicate matching the first element with this local name that has any children — the theme
/// carries several empty `<a:effectLst/>` before the one with an effect in it.
fn named_with_children(local: &'static str) -> impl Fn(&RawElement, &Interner) -> bool {
    move |element, interner| {
        interner.resolve(element.name.local) == local && !element.children.is_empty()
    }
}

// ---------------------------------------------------------------------------------------------
// The real corpus: elements out of real parts, replaced in place
// ---------------------------------------------------------------------------------------------

const SAMPLE_SLIDE: &str = "/ppt/slides/slide1.xml";
const THEME: &str = "/ppt/theme/theme1.xml";

#[test]
fn a_text_body_round_trips_byte_identical_in_context() {
    round_trips_in_context::<TextBody>(
        "sample.pptx",
        SAMPLE_SLIDE,
        named("txBody"),
        |body, _interner| {
            assert_eq!(body.text(), "Hello OOXML");
            assert_eq!(body.paragraphs().count(), 1);
            assert_eq!(body.paragraphs().next().unwrap().runs().count(), 1);
            // Content order: opaque bodyPr, opaque lstStyle, then the typed paragraph.
            assert_eq!(body.content().len(), 3);
            assert!(matches!(body.content()[0], TextBodyContent::Raw(_)));
            assert!(matches!(body.content()[2], TextBodyContent::Paragraph(_)));
        },
    );
}

#[test]
fn an_outline_round_trips_byte_identical_in_context() {
    round_trips_in_context::<LineProperties>(
        "effects_theme.pptx",
        THEME,
        named("ln"),
        |line, i| {
            assert_eq!(line.width(i), Ok(Some(LineWidth::from_emu(6_350))));
        },
    );
}

#[test]
fn a_color_round_trips_byte_identical_in_context() {
    round_trips_in_context::<Color>("effects_theme.pptx", THEME, named("srgbClr"), |color, i| {
        assert!(color.value(i).expect("a legal @val").is_some());
    });
}

#[test]
fn a_solid_fill_round_trips_byte_identical_in_context() {
    round_trips_in_context::<SolidFill>(
        "effects_theme.pptx",
        THEME,
        named("solidFill"),
        |fill, _i| {
            assert!(
                fill.color().is_some(),
                "the theme's first solidFill has one"
            );
        },
    );
}

#[test]
fn an_effect_list_round_trips_byte_identical_in_context() {
    round_trips_in_context::<EffectList>(
        "effects_theme.pptx",
        THEME,
        named_with_children("effectLst"),
        |effects, i| {
            let shadow = effects.outer_shadow(i).expect("the theme's outer shadow");
            assert_eq!(shadow.blur_radius.expect("blurRad").emu(), 40_000);
            assert_eq!(shadow.distance.expect("dist").emu(), 20_000);
            assert_eq!(shadow.rotate_with_shape, Some(false));
        },
    );
}

#[test]
fn a_3d_scene_round_trips_byte_identical_in_context() {
    round_trips_in_context::<Scene3D>("effects_theme.pptx", THEME, named("scene3d"), |scene, i| {
        let rig = scene.light_rig(i).expect("the theme's light rig");
        assert!((rig.rotation.expect("a:rot").revolution.degrees() - 20.0).abs() < 1e-9);
    });
}

#[test]
fn a_3d_shape_round_trips_byte_identical_in_context() {
    round_trips_in_context::<Shape3D>("effects_theme.pptx", THEME, named("sp3d"), |shape, i| {
        let bevel = shape.bevel_top(i).expect("the theme's bevelT");
        assert_eq!(bevel.width.expect("w").emu(), 63_500);
        assert_eq!(bevel.height.expect("h").emu(), 25_400);
    });
}

// ---------------------------------------------------------------------------------------------
// The disagreeing corpus
// ---------------------------------------------------------------------------------------------
//
// Hand-written literals, none of them in a form this project's writer emits. Each one carries at
// least one property the fifteen committed fixtures do not have anywhere:
//
//   * a value in single quotes, where the writer emits double;
//   * an unknown attribute (`z:note`) sitting *between* two the model knows, so preserving it
//     requires preserving its position and not merely appending it;
//   * an `ST_OnOff` in a spelling other than `true`/`false`;
//   * a percentage in the `%` spelling, which the writer never emits;
//   * a namespaced attribute (`r:embed`), matched on its literal prefix;
//   * a character reference inside a value a model actually *reads*, so a read that re-encoded what
//     it decoded would be visible.

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const Z: &str = "urn:mjx:not-ours";

/// `a:gradFill` — `@flip` written with a character reference, an unknown attribute between the two
/// modeled ones, `@rotWithShape` in the `on` spelling, one stop position in each percentage form,
/// and a `a:lin` shade angle.
fn gradient_fill() -> Vec<u8> {
    format!(
        r#"<a:gradFill xmlns:a="{A}" xmlns:z="{Z}" flip='no&#x6E;e' z:note='between the known ones' rotWithShape="on"><a:gsLst><a:gs pos='0%'><a:srgbClr val='FF0000'/></a:gs><a:gs pos="100000"><a:schemeClr val="accent1"/></a:gs></a:gsLst><a:lin ang='5400000' scaled="1"/><a:tileRect/></a:gradFill>"#
    )
    .into_bytes()
}

/// `a:ln` — the four modeled attributes with an unknown one between them, a single-quoted cap, a
/// preset dash, a miter limit in the `%` spelling and a head end.
fn outline() -> Vec<u8> {
    format!(
        r#"<a:ln xmlns:a="{A}" xmlns:z="{Z}" w="19050" z:note='between the known ones' cap='rnd' cmpd="sng" algn='ctr'><a:solidFill><a:srgbClr val='FF0000'/></a:solidFill><a:prstDash val='dash'/><a:miter lim="800%"/><a:headEnd type='triangle' w="med" len='lg'/></a:ln>"#
    )
    .into_bytes()
}

/// `a:effectLst` — an outer shadow whose scales are in the `%` spelling, whose `@rotWithShape` is
/// `1`, and which carries an unknown attribute between two modeled ones.
fn effect_list() -> Vec<u8> {
    format!(
        r#"<a:effectLst xmlns:a="{A}" xmlns:z="{Z}"><a:outerShdw blurRad='40000' z:note='between the known ones' dist="20000" dir='2700000' sx="105%" sy='95%' kx="60000" ky='-60000' algn='bl' rotWithShape="1"><a:srgbClr val='000000'><a:alpha val='63%'/></a:srgbClr></a:outerShdw><a:reflection stA='50%' endPos="35000" dist='0'/><a:softEdge rad='12700'/></a:effectLst>"#
    )
    .into_bytes()
}

/// `a:sp3d` — a single-quoted material, an unknown attribute between two modeled ones, and a bevel.
fn shape_3d() -> Vec<u8> {
    format!(
        r#"<a:sp3d xmlns:a="{A}" xmlns:z="{Z}" z='12700' z:note='between the known ones' extrusionH="63500" contourW='6350' prstMaterial='metal'><a:bevelT w='88900' h="25400" prst='coolSlant'/><a:contourClr><a:srgbClr val='FFFF00'/></a:contourClr></a:sp3d>"#
    )
    .into_bytes()
}

/// `a:blipFill` — the namespaced `r:embed`, matched on its literal prefix, with an unknown attribute
/// carrying an entity beside it.
fn picture_fill() -> Vec<u8> {
    format!(
        r#"<a:blipFill xmlns:a="{A}" xmlns:r="{R}" xmlns:z="{Z}" rotWithShape="off"><a:blip r:embed='rId2' z:note="5 &lt; 6 &amp; 7"><a:alphaModFix amt="50000"/></a:blip><a:stretch><a:fillRect/></a:stretch></a:blipFill>"#
    )
    .into_bytes()
}

#[test]
fn a_gradient_written_in_forms_we_never_emit_survives_byte_for_byte() {
    let markup = gradient_fill();
    round_trips_in_document::<GradientFill>(&markup, named("gradFill"), |fill, i| {
        // The character reference is decoded on the way in — and, since a read cannot change the
        // file, the bytes below are still `no&#x6E;e`.
        assert_eq!(
            fill.flip(i).expect("a legal @flip").as_deref(),
            Some("none")
        );
        assert_eq!(fill.rot_with_shape(i), Ok(Some(true)), "`on` is true");
        let stops = fill.stops(i);
        assert_eq!(stops.len(), 2);
        assert!(stops[0].position.ratio().abs() < 1e-9, "`0%` is 0.0");
        assert!(
            (stops[1].position.ratio() - 1.0).abs() < 1e-9,
            "`100000` is 1.0"
        );
        assert!((fill.linear_angle(i).expect("a:lin@ang").degrees() - 90.0).abs() < 1e-9);
    });
    assert!(
        String::from_utf8_lossy(&markup).contains("no&#x6E;e"),
        "the fixture stopped carrying the character reference this case is about"
    );
}

#[test]
fn an_outline_written_in_forms_we_never_emit_survives_byte_for_byte() {
    round_trips_in_document::<LineProperties>(&outline(), named("ln"), |line, i| {
        assert_eq!(line.width(i), Ok(Some(LineWidth::from_emu(19_050))));
        assert!(matches!(line.dash(i), Some(LineDash::Preset(_))));
        // `800%` is the `%` spelling of a percentage — 8.0 as a fraction.
        let mjx_dml::LineJoin::Miter { limit } = line.join(i).expect("a:miter") else {
            panic!("expected a mitered join");
        };
        assert!((limit.expect("@lim").ratio() - 8.0).abs() < 1e-9);
        assert!(line.head_end(i).expect("a:headEnd").kind.is_some());
    });
}

#[test]
fn effects_written_in_forms_we_never_emit_survive_byte_for_byte() {
    round_trips_in_document::<EffectList>(&effect_list(), named("effectLst"), |effects, i| {
        let shadow = effects.outer_shadow(i).expect("a:outerShdw");
        // `105%` and `95%` are the `%` spelling; the writer only ever emits `105000` / `95000`.
        assert!((shadow.scale_x.expect("sx").ratio() - 1.05).abs() < 1e-9);
        assert!((shadow.scale_y.expect("sy").ratio() - 0.95).abs() < 1e-9);
        assert_eq!(shadow.rotate_with_shape, Some(true), "`1` is true");
        let reflection = effects.reflection(i).expect("a:reflection");
        assert!((reflection.start_alpha.expect("stA").ratio() - 0.5).abs() < 1e-9);
        assert_eq!(
            effects.soft_edge(i).expect("a:softEdge").radius.emu(),
            12_700
        );
    });
}

#[test]
fn a_3d_shape_written_in_forms_we_never_emit_survives_byte_for_byte() {
    round_trips_in_document::<Shape3D>(&shape_3d(), named("sp3d"), |shape, i| {
        assert_eq!(
            shape.material(i),
            Ok(Some(mjx_dml::PresetMaterial::Metal)),
            "a single-quoted enumeration reads the same as a double-quoted one"
        );
        assert_eq!(
            shape.bevel_top(i).expect("bevelT").width.expect("w").emu(),
            88_900
        );
    });
}

#[test]
fn a_namespaced_relationship_attribute_survives_byte_for_byte() {
    round_trips_in_document::<PictureFill>(&picture_fill(), named("blipFill"), |fill, i| {
        assert_eq!(
            fill.image_rel_id(i).as_deref(),
            Some("rId2"),
            "`r:embed` is matched on its literal prefix"
        );
        assert_eq!(fill.image_link_id(i), None, "there is no `r:link`");
    });
}

#[test]
fn every_on_off_spelling_reads_and_survives_byte_for_byte() {
    for (spelling, expected) in [
        ("1", true),
        ("0", false),
        ("true", true),
        ("false", false),
        ("on", true),
        ("off", false),
    ] {
        // Single-quoted, and with an unknown attribute in front of it, so nothing about the
        // surrounding markup is what this project would have written either.
        let markup = format!(
            r#"<a:gradFill xmlns:a="{A}" xmlns:z="{Z}" z:note='first' rotWithShape='{spelling}' flip="none"><a:gsLst/></a:gradFill>"#
        )
        .into_bytes();
        round_trips_in_document::<GradientFill>(&markup, named("gradFill"), |fill, i| {
            assert_eq!(
                fill.rot_with_shape(i),
                Ok(Some(expected)),
                "`{spelling}` did not read as {expected}"
            );
        });
    }
}

#[test]
fn a_write_canonicalizes_only_what_it_wrote_and_moves_nothing() {
    let markup = gradient_fill();
    let mut document = fidelity::parse(&markup).expect("the markup is well-formed");
    let RawDocument { interner, root, .. } = &mut document;
    let mut fill = GradientFill::from_xml(root, interner).expect("from_xml");

    // Both modeled attributes are assigned the value they already have. That is still a *write*,
    // and a write has one canonical spelling — the half of the contract a read must never perform.
    // `@flip` is the **first** attribute and single-quoted, `@rotWithShape` the last and
    // double-quoted, and the unknown `z:note` sits between them: a setter that removed and
    // re-appended would move `@flip` to the end and re-quote it, which is invisible if only the
    // last attribute is ever assigned to.
    fill.set_flip(interner, Some("none"));
    fill.set_rot_with_shape(interner, Some(true));
    *root = fill.to_xml(interner);
    let out = fidelity::serialize_to_vec(&document);
    let out = String::from_utf8_lossy(&out);

    assert!(
        out.contains(
            r#"flip='none' z:note='between the known ones' rotWithShape="true"><a:gsLst>"#
        ),
        "a write moved an attribute, changed a quote, or did not canonicalize: {out}"
    );
    // Nothing below the start tag was touched, including the `%` stop position and the second
    // stop's own quoting.
    assert!(
        out.contains(r#"<a:gs pos='0%'><a:srgbClr val='FF0000'/></a:gs>"#),
        "a write reached past the attribute it was given: {out}"
    );
}

// ---------------------------------------------------------------------------------------------
// Tier 3 — edit isolation
// ---------------------------------------------------------------------------------------------

#[test]
fn setting_one_attribute_changes_that_attribute_and_nothing_else_in_the_part() {
    let package = Package::open(&mjx_fixtures::fixture("effects_theme.pptx")).expect("opens");
    let part = PartName::new(THEME).expect("a valid part name");
    let original = package.part_bytes(&part).expect("theme1.xml").to_vec();

    let mut document = fidelity::parse(&original).expect("the theme parses");
    let RawDocument { interner, root, .. } = &mut document;
    let slot = find_element_mut(root, &|element| {
        interner.resolve(element.name.local) == "ln"
    })
    .expect("the theme has an a:ln");
    let mut line = LineProperties::from_xml(slot, interner).expect("from_xml");
    assert_eq!(line.width(interner), Ok(Some(LineWidth::from_emu(6_350))));

    line.set_width(interner, Some(LineWidth::from_emu(9_525)));
    *slot = line.to_xml(interner);
    let edited = fidelity::serialize_to_vec(&document);

    // Exactly one attribute value differs, in the first `a:ln`, and nothing else in the part moved.
    let expected =
        String::from_utf8_lossy(&original).replacen(r#"<a:ln w="6350">"#, r#"<a:ln w="9525">"#, 1);
    assert_ne!(expected, String::from_utf8_lossy(&original));
    assert_eq!(String::from_utf8_lossy(&edited), expected);
}

#[test]
fn editing_one_part_leaves_every_other_part_byte_identical() {
    let bytes = mjx_fixtures::fixture("effects_theme.pptx");
    let mut package = Package::open(&bytes).expect("opens");
    let part = PartName::new(THEME).expect("a valid part name");
    let original = package.part_bytes(&part).expect("theme1.xml").to_vec();

    let mut document = fidelity::parse(&original).expect("the theme parses");
    let RawDocument { interner, root, .. } = &mut document;
    let slot = find_element_mut(root, &|element| {
        interner.resolve(element.name.local) == "sp3d"
    })
    .expect("the theme has an a:sp3d");
    let mut shape = Shape3D::from_xml(slot, interner).expect("from_xml");
    shape.set_material(interner, Some(mjx_dml::PresetMaterial::Metal));
    *slot = shape.to_xml(interner);
    let edited = fidelity::serialize_to_vec(&document);
    package
        .replace_part_bytes(&part, edited.clone())
        .expect("the theme part is replaceable");

    let saved = package.save().expect("the package saves");
    let before = Package::open(&bytes).expect("opens");
    let after = Package::open(&saved).expect("the saved package opens");
    let names: Vec<PartName> = before.part_names().collect();
    assert!(names.len() > 5, "the fixture has parts to be isolated from");
    for name in names {
        let expected = if name == part {
            Cow::Owned(edited.clone())
        } else {
            Cow::Borrowed(before.part_bytes(&name).expect("a part before"))
        };
        assert_eq!(
            after.part_bytes(&name).map(String::from_utf8_lossy),
            Some(String::from_utf8_lossy(&expected)),
            "{name:?} changed while a different part was edited"
        );
    }
    // …and the edit really happened.
    assert!(String::from_utf8_lossy(&edited).contains(r#"prstMaterial="metal""#));
}
