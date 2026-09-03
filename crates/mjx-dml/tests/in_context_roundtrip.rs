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
    AdjustCoordinate, Bullet, BulletSize, CharacterProperties, Color, CustomGeometry, DrawCommand,
    EffectList, Emu, FontSlot, GeometryGuide, GradientFill, LineDash, LineProperties, LineWidth,
    ParagraphProperties, PathFillMode, PictureFill, PresetGeometry, Scene3D, Shape3D, SolidFill,
    TabAlignment, Table, TablePart, TextAlignment, TextAnchoring, TextBody, TextBodyContent,
    TextSpacing, TextUnderline, Transform2D,
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
const SLIDE: &str = "/ppt/slides/slide1.xml";

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
// The composite tiers, out of real parts — and the nested assertion
// ---------------------------------------------------------------------------------------------
//
// A table is the deepest nest this crate models: `a:tbl` → `a:tr` → `a:tc` → `a:tcPr` / `a:txBody`
// → `a:p` → `a:r` → `a:rPr`. Every rung is a separate `to_xml`, so a cell's attributes can survive
// a round trip *in isolation* and still lose their order when the cell is rebuilt as part of a row
// rebuilt as part of a table. **These cases therefore assert at the outermost container**: the
// element lifted out of the part is the `a:tbl`, and the whole part must come back byte-identical.

#[test]
fn a_table_round_trips_byte_identical_at_the_outermost_container() {
    round_trips_in_context::<Table>("tables.pptx", SLIDE, named("tbl"), |table, i| {
        // The structural closure is what discriminates: byte identity alone would be satisfied by a
        // type that carried the table around opaquely and modelled none of it.
        assert_eq!(table.row_count(), 3);
        assert_eq!(table.column_count(), 3);
        let properties = table.properties().expect("a:tblPr");
        assert_eq!(properties.part(i, TablePart::FirstRow), Some(true));
        assert_eq!(properties.part(i, TablePart::BandedRows), Some(true));
        assert_eq!(properties.part(i, TablePart::LastRow), None);
        let widths: Vec<i64> = table
            .grid()
            .expect("a:tblGrid")
            .columns()
            .map(|column| {
                column
                    .width(i)
                    .expect("a legal @w")
                    .expect("a stated width")
                    .emu()
            })
            .collect();
        assert_eq!(widths, [2_438_400, 2_438_400, 2_438_400]);
        assert_eq!(
            table
                .row(0)
                .expect("row 0")
                .height(i)
                .expect("a legal @h")
                .expect("a stated height")
                .emu(),
            914_400
        );
        assert_eq!(table.cell(0, 0).expect("0,0").text(), "Region");
        assert_eq!(table.cell(2, 2).expect("2,2").text(), "-3%");
        assert_eq!(table.cell(0, 0).expect("0,0").column_span(i), 1);
        assert!(!table.cell(0, 0).expect("0,0").is_covered_by_merge(i));
    });
}

#[test]
fn a_table_with_extension_lists_round_trips_byte_identical_at_the_outermost_container() {
    // `table_extensions.pptx` carries an `a:extLst` on the table properties *and* on a cell's
    // `a:tcPr` — the two places a rebuild of the nest would drop one.
    round_trips_in_context::<Table>("table_extensions.pptx", SLIDE, named("tbl"), |table, i| {
        let properties = table.properties().expect("a:tblPr");
        assert!(
            properties
                .children()
                .iter()
                .any(|node| is_named(node, i, "extLst")),
            "the table properties carry an extension list"
        );
        let cell = table.cell(0, 0).expect("0,0");
        assert!(
            cell.properties()
                .expect("a:tcPr")
                .children()
                .iter()
                .any(|node| is_named(node, i, "extLst")),
            "the first cell's properties carry an extension list"
        );
    });
}

#[test]
fn a_paragraphs_properties_round_trip_byte_identical_in_context() {
    // The slide's *second* text body is the one with a level per paragraph; the title has one
    // paragraph and no `@lvl` anywhere.
    let has_five_paragraphs = |element: &RawElement, interner: &Interner| {
        interner.resolve(element.name.local) == "txBody"
            && element
                .children
                .iter()
                .filter(|node| is_named(node, interner, "p"))
                .count()
                == 5
    };
    round_trips_in_context::<TextBody>(
        "text_levels.pptx",
        SLIDE,
        has_five_paragraphs,
        |body, i| {
            let levels: Vec<u8> = body
                .paragraphs()
                .map(|paragraph| {
                    paragraph
                        .properties()
                        .and_then(|properties| {
                            properties
                                .level(i)
                                .expect("a legal @lvl")
                                .map(|l| l.value())
                        })
                        .unwrap_or(0)
                })
                .collect();
            assert_eq!(levels, [0, 1, 2, 3, 4]);
        },
    );
}

#[test]
fn a_preset_geometry_round_trips_byte_identical_in_context() {
    round_trips_in_context::<PresetGeometry>(
        "text_levels.pptx",
        SLIDE,
        named("prstGeom"),
        |geometry, i| {
            assert_eq!(geometry.preset_token(i).expect("a legal @prst"), "rect");
            assert!(geometry.adjust_values().is_some(), "an empty a:avLst");
        },
    );
}

#[test]
fn a_transform_round_trips_byte_identical_in_context() {
    // `Transform2D` is a *value* over an `a:xfrm`, not a fidelity type, so the round trip here is
    // read-then-`apply`-back: every field it names is written onto the element it came from, in
    // place, which must reproduce the bytes it read.
    let package = Package::open(&mjx_fixtures::fixture("text_levels.pptx")).expect("opens");
    let part = PartName::new(SLIDE).expect("a valid part name");
    let original = package.part_bytes(&part).expect("slide1.xml").to_vec();

    let mut document = fidelity::parse(&original).expect("the slide parses");
    let RawDocument { interner, root, .. } = &mut document;
    let slot = find_element_mut(root, &|element| {
        interner.resolve(element.name.local) == "xfrm"
    })
    .expect("the slide has an a:xfrm");
    let transform = Transform2D::read(slot, interner);
    assert_eq!(transform.position.expect("a:off").x, Emu::from_emu(685_800));
    assert_eq!(
        transform.size.expect("a:ext").width,
        Emu::from_emu(7_772_400)
    );
    assert_eq!(transform.rotation, None, "the shape states no rotation");
    transform.apply(slot, interner);

    assert_eq!(
        String::from_utf8_lossy(&fidelity::serialize_to_vec(&document)),
        String::from_utf8_lossy(&original),
        "writing a transform's own values back moved a byte"
    );
}

/// Whether `node` is an element with this local name.
fn is_named(node: &RawNode, interner: &Interner, local: &str) -> bool {
    matches!(node, RawNode::Element(element)
        if interner.resolve(element.name.local) == local)
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
// The disagreeing corpus, composite tiers
// ---------------------------------------------------------------------------------------------
//
// Same rules as above, aimed at what the composite tiers read. The text tier is the worst-served by
// the fixtures: `a:rPr`'s `@b`, `@i`, `@u`, `@strike`, `@spc`, `@baseline`, `@lang` are an
// `ST_OnOff`-and-measure thicket, and every fixture spells each of them the one way this project
// would have written it, or not at all.

/// `a:rPr` — an `ST_OnOff` in a spelling we never write on `@b` and `@i`, an unknown attribute
/// *between* two modelled ones, single quotes throughout, a percentage baseline in the `%` form,
/// a character reference inside `@lang` (a value a model reads), and a namespaced `r:id` on the
/// nested `a:hlinkClick`.
fn run_properties() -> Vec<u8> {
    format!(
        r#"<a:rPr xmlns:a="{A}" xmlns:r="{R}" xmlns:z="{Z}" sz='1800' b='on' z:note="between the known ones" i="off" u='dashHeavy' strike="sngStrike" cap='small' spc="-150" kern='1200' baseline='30%' lang="en&#x2D;GB"><a:solidFill><a:srgbClr val='FF0000'/></a:solidFill><a:latin typeface='Calibri' pitchFamily="34" charset='0'/><a:hlinkClick r:id="rId7" action='ppaction://hlinksldjump'/></a:rPr>"#
    )
    .into_bytes()
}

/// `a:pPr` — a level and margins in forms we never write, an unknown attribute between two
/// modelled ones, `@rtl` in the `1` spelling, a `%`-spelled bullet size, and a tab stop.
fn paragraph_properties() -> Vec<u8> {
    format!(
        r#"<a:pPr xmlns:a="{A}" xmlns:z="{Z}" marL='342900' z:note="between the known ones" indent="-342900" lvl='2' algn="just" defTabSz='914400' rtl="1" fontAlgn='base'><a:lnSpc><a:spcPct val='150%'/></a:lnSpc><a:spcBef><a:spcPts val="600"/></a:spcBef><a:buSzPct val='111%'/><a:buFont typeface="Wingdings" pitchFamily='2' charset="2"/><a:buChar char='&#x00A7;'/><a:tabLst><a:tab pos="457200" algn='ctr'/></a:tabLst></a:pPr>"#
    )
    .into_bytes()
}

/// `a:tbl` — the nested case in disagreeing form: `@gridSpan` and `@hMerge` spelled the ways we
/// never write, an unknown attribute *between* two modelled ones on a cell, single-quoted cell
/// margins, and an `a:extLst` inside a `a:tcPr`.
fn table() -> Vec<u8> {
    format!(
        r#"<a:tbl xmlns:a="{A}" xmlns:z="{Z}"><a:tblPr firstRow='1' z:note="between the known ones" bandRow="on" rtl='0'/><a:tblGrid><a:gridCol w='2438400'/><a:gridCol w="2438400"/></a:tblGrid><a:tr h='914400'><a:tc id="c1" gridSpan='2' z:note="between the known ones" rowSpan="1"><a:txBody><a:bodyPr/><a:p><a:r><a:rPr b='on'/><a:t>Merged</a:t></a:r></a:p></a:txBody><a:tcPr marL='91440' marR="45720" anchor='ctr' anchorCtr="off" horzOverflow='clip'><a:extLst><a:ext uri="{{TAG}}"><z:tag keep='1'/></a:ext></a:extLst></a:tcPr></a:tc><a:tc hMerge="on"><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl>"#
    )
    .into_bytes()
}

/// `a:custGeom` — a path whose flags are spelled the ways we never write, guide references beside
/// literals in the same `ST_AdjCoordinate` position, an unknown attribute between two modelled ones
/// on the `a:path`, and a character reference in an `a:gd@fmla` this model reads.
fn custom_geometry() -> Vec<u8> {
    format!(
        r#"<a:custGeom xmlns:a="{A}" xmlns:z="{Z}"><a:avLst><a:gd name='adj' fmla="val 25000"/></a:avLst><a:gdLst><a:gd name="hc" fmla='*/ w 1 &#x32;'/></a:gdLst><a:ahLst><a:ahXY gdRefX='adj' minX="0" maxX='50000'><a:pos x='hc' y="33"/></a:ahXY></a:ahLst><a:cxnLst><a:cxn ang='0'><a:pos x="hc" y='0'/></a:cxn></a:cxnLst><a:rect l='10' t="20" r='hc' b="40"/><a:pathLst><a:path w='200' z:note="between the known ones" h="100" fill='lighten' stroke="0" extrusionOk='on'><a:moveTo><a:pt x='11' y="22"/></a:moveTo><a:arcTo wR='hc' hR="50" stAng='0' swAng="5400000"/><a:close/></a:path></a:pathLst></a:custGeom>"#
    )
    .into_bytes()
}

/// `a:xfrm` — a rotation and both mirror flags in spellings we never write, with an unknown
/// attribute between two modelled ones and single-quoted child coordinates.
fn transform() -> Vec<u8> {
    format!(
        r#"<a:xfrm xmlns:a="{A}" xmlns:z="{Z}" rot='2700000' z:note="between the known ones" flipH="1" flipV='off'><a:off x='914400' y="914400"/><a:ext cx="3657600" cy='1828800'/></a:xfrm>"#
    )
    .into_bytes()
}

#[test]
fn run_properties_written_in_forms_we_never_emit_survive_byte_for_byte() {
    round_trips_in_document::<CharacterProperties>(
        &run_properties(),
        named("rPr"),
        |properties, i| {
            assert_eq!(
                properties.size(i).expect("a legal @sz").map(|s| s.points()),
                Some(18.0)
            );
            assert_eq!(properties.is_bold(i), Ok(Some(true)), "`on` is true");
            assert_eq!(properties.is_italic(i), Ok(Some(false)), "`off` is false");
            assert_eq!(
                properties.underline(i),
                Ok(Some(TextUnderline::HeavyDashed)),
                "a single-quoted enumeration reads like a double-quoted one"
            );
            assert_eq!(
                properties
                    .spacing(i)
                    .expect("a legal @spc")
                    .map(|s| s.points()),
                Some(-1.5)
            );
            assert_eq!(
                properties
                    .kerning(i)
                    .expect("a legal @kern")
                    .map(|k| k.points()),
                Some(12.0)
            );
            let baseline = properties
                .baseline(i)
                .expect("a legal @baseline")
                .expect("stated");
            assert!(
                (baseline.ratio() - 0.3).abs() < 1e-9,
                "`30%` is the `%` spelling of 30 %"
            );
            assert_eq!(
                properties.language(i).expect("a legal @lang").as_deref(),
                Some("en-GB"),
                "the character reference is decoded on the way in"
            );
            assert_eq!(properties.hyperlink_rel_id(i).as_deref(), Some("rId7"));
            assert_eq!(
                properties.hyperlink_action(i).as_deref(),
                Some("ppaction://hlinksldjump")
            );
            let font = properties.font(i, FontSlot::Latin).expect("a:latin");
            assert_eq!(font.typeface, "Calibri");
            assert_eq!(font.pitch_family, Some(34));
            assert_eq!(font.charset, Some(0));
        },
    );
    assert!(
        String::from_utf8_lossy(&run_properties()).contains("en&#x2D;GB"),
        "the fixture stopped carrying the character reference this case is about"
    );
}

#[test]
fn paragraph_properties_written_in_forms_we_never_emit_survive_byte_for_byte() {
    round_trips_in_document::<ParagraphProperties>(
        &paragraph_properties(),
        named("pPr"),
        |properties, i| {
            assert_eq!(
                properties
                    .level(i)
                    .expect("a legal @lvl")
                    .map(|l| l.value()),
                Some(2)
            );
            assert_eq!(properties.alignment(i), Ok(Some(TextAlignment::Justified)));
            assert_eq!(
                properties.left_margin(i),
                Ok(Some(Emu::from_emu(342_900))),
                "a single-quoted measure reads like a double-quoted one"
            );
            assert_eq!(properties.indent(i), Ok(Some(Emu::from_emu(-342_900))));
            assert_eq!(
                properties.is_right_to_left(i),
                Ok(Some(true)),
                "`1` is true"
            );
            let Some(TextSpacing::Percentage(line)) = properties.line_spacing(i) else {
                panic!("a percentage line spacing")
            };
            assert!(
                (line.ratio() - 1.5).abs() < 1e-9,
                "`150%` is the `%` spelling"
            );
            let Some(TextSpacing::Points(before)) = properties.space_before(i) else {
                panic!("a point-valued space before")
            };
            assert!((before.points() - 6.0).abs() < 1e-9);
            let Some(BulletSize::Percentage(size)) = properties.bullet_size(i) else {
                panic!("a percentage bullet size")
            };
            assert!((size.ratio() - 1.11).abs() < 1e-9, "`111%`");
            assert!(
                matches!(properties.bullet(i), Some(Bullet::Character(_))),
                "a character bullet given by reference"
            );
            let stops = properties.tab_stops(i);
            assert_eq!(stops.len(), 1);
            assert_eq!(stops[0].position, Emu::from_emu(457_200));
            assert_eq!(stops[0].alignment, Some(TabAlignment::Center));
        },
    );
}

#[test]
fn a_table_written_in_forms_we_never_emit_survives_byte_for_byte_at_the_outermost_container() {
    round_trips_in_document::<Table>(&table(), named("tbl"), |table, i| {
        let properties = table.properties().expect("a:tblPr");
        assert_eq!(properties.part(i, TablePart::FirstRow), Some(true));
        assert_eq!(
            properties.part(i, TablePart::BandedRows),
            Some(true),
            "`on` is true"
        );
        assert_eq!(
            properties.part(i, TablePart::RightToLeft),
            Some(false),
            "`0` is false, and is not the same as unstated"
        );
        let anchor = table.cell(0, 0).expect("0,0");
        assert_eq!(anchor.column_span(i), 2, "a single-quoted span");
        assert!(!anchor.is_covered_by_merge(i), "the anchor is not covered");
        assert_eq!(
            anchor.id(i).expect("a legal @id").as_deref(),
            Some("c1"),
            "the id sits in front of the unknown attribute"
        );
        let cell_properties = anchor.properties().expect("a:tcPr");
        // Two *different* margins, asserted apart: a pair of equal ones cannot tell a reader that
        // swaps `@marL` and `@marR` from one that does not.
        assert_eq!(
            cell_properties.left_margin(i),
            Ok(Some(Emu::from_emu(91_440)))
        );
        assert_eq!(
            cell_properties.right_margin(i),
            Ok(Some(Emu::from_emu(45_720)))
        );
        assert_eq!(cell_properties.anchor(i), Ok(Some(TextAnchoring::Center)));
        assert_eq!(
            cell_properties.anchor_centered(i),
            Ok(Some(false)),
            "`off` is false"
        );
        assert!(
            table.cell(0, 1).expect("0,1").merged_horizontally(i),
            "`on` is true"
        );
    });
}

#[test]
fn a_custom_geometry_written_in_forms_we_never_emit_survives_byte_for_byte() {
    round_trips_in_document::<CustomGeometry>(
        &custom_geometry(),
        named("custGeom"),
        |geometry, i| {
            let adjust_values = geometry.adjust_values(i);
            assert_eq!(adjust_values.len(), 1);
            assert_eq!(adjust_values[0].formula, "val 25000");
            let guides = geometry.guides(i);
            assert_eq!(
                guides[0].formula, "*/ w 1 2",
                "the character reference is decoded on the way in"
            );
            let paths = geometry.paths(i);
            assert_eq!(paths.len(), 1);
            assert_eq!(paths[0].width, Some(Emu::from_emu(200)));
            assert_eq!(paths[0].fill, Some(PathFillMode::Lighten));
            assert_eq!(paths[0].stroke, Some(false), "`0` is false");
            assert_eq!(paths[0].extrusion_ok, Some(true), "`on` is true");
            // An `ST_AdjCoordinate` is a length *or* a guide name, and the arc carries one of each.
            let DrawCommand::ArcTo {
                width_radius,
                height_radius,
                ..
            } = &paths[0].commands[1]
            else {
                panic!("expected an arc")
            };
            assert_eq!(*width_radius, AdjustCoordinate::Guide("hc".to_owned()));
            assert_eq!(*height_radius, AdjustCoordinate::Emu(Emu::from_emu(50)));
            let rectangle = geometry.text_rectangle(i).expect("a:rect");
            assert_eq!(rectangle.right, AdjustCoordinate::Guide("hc".to_owned()));
            assert_eq!(geometry.connection_sites(i).len(), 1);
            assert_eq!(geometry.adjust_handles(i).len(), 1);
        },
    );
}

#[test]
fn a_transform_written_in_forms_we_never_emit_survives_byte_for_byte() {
    let markup = transform();
    let mut document = fidelity::parse(&markup).expect("the markup is well-formed");
    let RawDocument { interner, root, .. } = &mut document;
    let read = Transform2D::read(root, interner);
    assert_eq!(read.flip_horizontal, Some(true), "`1` is true");
    assert_eq!(read.flip_vertical, Some(false), "`off` is false");
    assert!((read.rotation.expect("@rot").degrees() - 45.0).abs() < 1e-9);
    assert_eq!(read.position.expect("a:off").x, Emu::from_emu(914_400));
    assert_eq!(read.size.expect("a:ext").height, Emu::from_emu(1_828_800));
    // `apply` is a **write**, and a write canonicalizes: the two `ST_OnOff`s the transform names
    // come back in the one spelling this project emits. Everything else is untouched — each
    // attribute keeps its position and its quote character, the unknown one included, and `@rot`
    // stays single-quoted because it was.
    read.apply(root, interner);
    let out = fidelity::serialize_to_vec(&document);
    let out = String::from_utf8_lossy(&out);
    assert!(
        out.contains(r#"rot='2700000' z:note="between the known ones" flipH="true" flipV='false'"#),
        "a write moved an attribute, changed a quote, or did not canonicalize: {out}"
    );
    assert!(
        out.contains(r#"<a:off x='914400' y="914400"/><a:ext cx="3657600" cy='1828800'/>"#),
        "the children were rewritten in place, keeping their own quoting: {out}"
    );
}

#[test]
fn a_guide_reads_its_pair_and_a_rewritten_formula_moves_nothing_else() {
    let markup = format!(
        r#"<a:gd xmlns:a="{A}" xmlns:z="{Z}" name='adj1' z:note="between the known ones" fmla="val 2&#x35;000"/>"#
    )
    .into_bytes();
    let mut document = fidelity::parse(&markup).expect("well-formed");
    let RawDocument { interner, root, .. } = &mut document;
    let mut guide = GeometryGuide::from_xml(root, interner).expect("from_xml");
    assert_eq!(guide.name(interner).expect("a legal @name"), "adj1");
    assert_eq!(
        guide.formula(interner).expect("a legal @fmla"),
        "val 25000",
        "the character reference is decoded on the way in"
    );
    guide.set_formula(interner, "val 50000");
    *root = guide.to_xml(interner);
    let out = fidelity::serialize_to_vec(&document);
    let out = String::from_utf8_lossy(&out);
    assert!(
        out.contains(r#"name='adj1' z:note="between the known ones" fmla="val 50000""#),
        "the rewritten formula stayed where it was, and nothing else moved: {out}"
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

#[test]
fn editing_one_cell_leaves_every_other_cell_row_and_attribute_byte_identical() {
    // Tier 3 at the depth this child is about. The edit is one attribute of one cell, three rungs
    // down (`a:tbl` → `a:tr` → `a:tc` → `a:tcPr`), and it is made through a `Table` that rebuilds
    // *every* row and cell on the way out. What must not move: the other eight cells, the other two
    // rows, the grid, the table properties, and — inside the edited cell's own `a:tcPr` — the
    // unknown attribute that sits in front of the one being written.
    let markup = format!(
        r#"<a:tbl xmlns:a="{A}" xmlns:z="{Z}"><a:tblPr firstRow='1'/><a:tblGrid><a:gridCol w='1'/><a:gridCol w="2"/></a:tblGrid><a:tr h='3'><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>a</a:t></a:r></a:p></a:txBody><a:tcPr z:note="in front of marL" marL='1' anchor="t"/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>b</a:t></a:r></a:p></a:txBody><a:tcPr marL='9'/></a:tc></a:tr><a:tr h="4"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>c</a:t></a:r></a:p></a:txBody><a:tcPr marL='9'/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>d</a:t></a:r></a:p></a:txBody><a:tcPr marL='9'/></a:tc></a:tr></a:tbl>"#
    )
    .into_bytes();

    let mut document = fidelity::parse(&markup).expect("the markup is well-formed");
    let RawDocument { interner, root, .. } = &mut document;
    let mut table = Table::from_xml(root, interner).expect("from_xml");
    table
        .cell_mut(0, 0)
        .expect("0,0")
        .properties_mut()
        .expect("a:tcPr")
        .set_margins(interner, Some(Emu::from_emu(7)), None, None, None);
    *root = table.to_xml(interner);
    let edited = fidelity::serialize_to_vec(&document);
    let edited = String::from_utf8_lossy(&edited);

    // Exactly one attribute value differs from the source, and it differs where it stood.
    let expected = String::from_utf8_lossy(&markup).replacen(
        r#"<a:tcPr z:note="in front of marL" marL='1' anchor="t"/>"#,
        r#"<a:tcPr z:note="in front of marL" marL='7' anchor="t"/>"#,
        1,
    );
    assert_ne!(expected, String::from_utf8_lossy(&markup));
    assert_eq!(
        edited, expected,
        "an edit three rungs down moved something it did not write"
    );
}
