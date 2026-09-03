//! The gate for `#[derive(XmlAttributes)]` — the five attribute shapes, on hand-written markup.
//!
//! The markup below is **not** what this project's writer emits, and that is deliberate. A synthetic
//! type authored beside the macro, fed markup the macro's own writer produced, proves only that the
//! reader and the writer agree with each other. So every fixture here is a committed literal written
//! in a form the writer would never produce:
//!
//! * **single-quoted values** (`val='FF0000'`) — the writer emits double quotes, so a grammar that
//!   silently normalizes quote style fails here rather than in a Word file six months from now;
//! * **an unknown attribute between two known ones** (`z:note`, between `val` and `rtlCol`) — keeping
//!   it requires keeping its *position*, not merely appending it at the end;
//! * **an unusual-but-legal `ST_OnOff` spelling** (`rtlCol="on"`), which must survive untouched on a
//!   read and become the canonical `true` only when something writes it.
//!
//! The five shapes are the ones the schemas actually use, at their real wire names:
//! `a:srgbClr/@val` (required hex colour), `a:alpha/@val` (percentage, two wire forms),
//! `a:bodyPr/@rtlCol` (`ST_OnOff`, schema default), `a:ln/@cap` (enumeration, no default) and
//! `a:off/@x` (a signed 64-bit EMU measure). Four sit on one synthetic element; the percentage sits
//! on a nested one, because two attributes on the same element cannot both be spelled `@val` and
//! renaming one would have tested a wire name OOXML does not contain.

use std::borrow::Cow;

use mjx_derive::{FromXml, ToXml, XmlAttributes};
use mjx_ooxml_core::{
    AttributeCodec, AttributeError, Enumeration, FromXml, Interner, InvalidAttributeValue,
    RawAttribute, RawDocument, RawName, RawNode, Text, ToXml,
};
use mjx_ooxml_types::drawingml::LineCap;
use mjx_ooxml_types::support::{HexColorRgb, OnOff};
use mjx_xml::fidelity;

// ---------------------------------------------------------------------------------------------
// Two codecs a *downstream* crate writes, over measure types this crate owns.
//
// `mjx-derive` cannot name `mjx_dml::Emu` or `mjx_dml::Fraction` (they live several layers above),
// and inventing shared measure types here would fork them. So the grammar's extension point is
// exercised the way `mjx-dml` will exercise it in MJXOFF-141: the crate that owns the measure owns
// the codec, in about fifteen lines.
// ---------------------------------------------------------------------------------------------

/// English Metric Units — 914,400 to the inch. `ST_Coordinate` is a signed 64-bit count of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Emu(i64);

/// `ST_Coordinate` / `ST_PositiveCoordinate` as an [`Emu`].
#[derive(Debug)]
struct EmuCoordinate;

impl AttributeCodec for EmuCoordinate {
    type Value<'a> = Emu;
    type Input<'a> = Emu;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<Emu, InvalidAttributeValue> {
        raw.parse::<i64>().map(Emu).map_err(|error| {
            InvalidAttributeValue::new(format!("not a 64-bit EMU coordinate: {error}"))
        })
    }

    fn encode<'a>(value: Self::Input<'a>) -> Cow<'a, str> {
        Cow::Owned(value.0.to_string())
    }
}

/// A DrawingML percentage, held in the schema's own native unit: thousandths of a percent, so
/// `100_000` is 100%. Integral, because the wire form is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Percentage(i32);

impl Percentage {
    fn from_percent(percent: f64) -> Self {
        Self((percent * 1000.0).round() as i32)
    }
}

/// `ST_Percentage`, whose wire form admits **both** the native integer (`50000`) and the
/// `%`-suffixed spelling (`50%`) — and writes only the first.
#[derive(Debug)]
struct PercentageCodec;

impl AttributeCodec for PercentageCodec {
    type Value<'a> = Percentage;
    type Input<'a> = Percentage;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<Percentage, InvalidAttributeValue> {
        let text = raw.as_ref();
        let reject = || {
            InvalidAttributeValue::new(format!(
                "expected a percentage, in thousandths of a percent or with a `%` suffix, found \
                 {text:?}"
            ))
        };
        match text.strip_suffix('%') {
            Some(percent) => percent
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(Percentage::from_percent)
                .ok_or_else(reject),
            None => text.parse::<i32>().map(Percentage).map_err(|_| reject()),
        }
    }

    fn encode<'a>(value: Self::Input<'a>) -> Cow<'a, str> {
        Cow::Owned(value.0.to_string())
    }
}

// ---------------------------------------------------------------------------------------------
// The synthetic types.
// ---------------------------------------------------------------------------------------------

/// Carries four of the five shapes, at their exact wire names. Its one modeled child is [`Alpha`].
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, XmlAttributes)]
#[xml(namespace = DML_MAIN)]
#[xml(attribute(local = "val", codec = HexColorRgb, accessor = color, required))]
#[xml(attribute(local = "rtlCol", codec = OnOff, default = false))]
#[xml(attribute(local = "cap", codec = Enumeration<LineCap>, accessor = line_cap))]
#[xml(attribute(local = "x", codec = EmuCoordinate, required))]
struct Styled {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "alpha", variant = Alpha, ty = Alpha))]
    content: Vec<StyledContent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StyledContent {
    Alpha(Alpha),
    Raw(RawNode),
}

/// `a:alpha` — the fifth shape, at its real wire name `@val`, which `Styled` cannot also spell.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, XmlAttributes)]
#[xml(attribute(local = "val", codec = PercentageCodec, accessor = amount, required))]
struct Alpha {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(text)]
    text: String,
}

/// A prefixed attribute, to prove the match is on the prefix and not on the local name alone.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, XmlAttributes)]
#[xml(attribute(local = "embed", prefix = "r", codec = Text, accessor = image_relationship))]
struct Blip {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(text)]
    text: String,
}

// ---------------------------------------------------------------------------------------------
// The committed fixtures. Hand-written; nothing here was produced by this project's writer.
// ---------------------------------------------------------------------------------------------

const DML: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

/// All five attributes present. `z:note` sits between `val` and `rtlCol`; four of the six values are
/// single-quoted; `@rtlCol` uses the `on` spelling and `@val` on `a:alpha` the `%` spelling.
const ALL_PRESENT: &[u8] = br#"<a:demo xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:z="urn:mjx:not-ours" val='FF0000' z:note='between the known ones' rtlCol="on" cap='sq' x="-914400"><a:alpha val='50%'/></a:demo>"#;

/// Only what is required. `@rtlCol` and `@cap` are absent, so the default and `None` are in play.
const ONLY_REQUIRED: &[u8] = br#"<a:demo xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" val='00FF00' x="0"><a:alpha val='100000'/></a:demo>"#;

/// Neither required attribute is there.
const NOTHING_PRESENT: &[u8] = br#"<a:demo xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" rtlCol='off'><a:alpha/></a:demo>"#;

fn parse<T: FromXml>(fragment: &[u8]) -> (T, RawDocument) {
    let document = fidelity::parse(fragment).expect("the fixture is well-formed XML");
    let typed = T::from_xml(&document.root, &document.interner).expect("from_xml succeeds");
    (typed, document)
}

/// Serializes `typed` back into `document` and returns the bytes.
fn serialize<T: ToXml>(typed: &T, mut document: RawDocument) -> Vec<u8> {
    document.root = typed.to_xml(&mut document.interner);
    fidelity::serialize_to_vec(&document)
}

#[track_caller]
fn assert_bytes(actual: &[u8], expected: &[u8]) {
    assert_eq!(
        String::from_utf8_lossy(actual),
        String::from_utf8_lossy(expected)
    );
}

/// The one modeled `a:alpha` child of a `Styled`.
fn alpha(styled: &Styled) -> &Alpha {
    styled
        .content
        .iter()
        .find_map(|item| match item {
            StyledContent::Alpha(alpha) => Some(alpha),
            StyledContent::Raw(_) => None,
        })
        .expect("the fixture has one modeled a:alpha child")
}

// ---------------------------------------------------------------------------------------------
// Reading — present, absent, malformed, for each of the five.
// ---------------------------------------------------------------------------------------------

#[test]
fn all_five_shapes_read_from_hand_written_markup() {
    let (styled, document) = parse::<Styled>(ALL_PRESENT);
    let interner = &document.interner;

    // 1. required hex colour, single-quoted, letter case as written
    assert_eq!(styled.color(interner).as_deref(), Ok("FF0000"));
    // 2. percentage in the `%` wire form, on the nested a:alpha
    assert_eq!(alpha(&styled).amount(interner), Ok(Percentage(50_000)));
    // 3. ST_OnOff in the `on` spelling
    assert_eq!(styled.rtl_col(interner), Ok(true));
    // 4. enumeration, by its exact wire token
    assert_eq!(styled.line_cap(interner), Ok(Some(LineCap::Square)));
    // 5. a signed EMU coordinate, negative
    assert_eq!(styled.x(interner), Ok(Emu(-914_400)));
}

#[test]
fn the_percentage_reads_from_both_of_its_wire_forms() {
    // Both spellings are legal `ST_Percentage`, and the `%` one is not merely tolerated: `50%` and
    // `50000` are the same quantity, so a codec that only handled the integer form would read the
    // first as an error and a codec that stripped the suffix would read it as 50 thousandths.
    for (fixture, expected) in [
        (ALL_PRESENT, Percentage(50_000)),
        (ONLY_REQUIRED, Percentage(100_000)),
    ] {
        let (styled, document) = parse::<Styled>(fixture);
        assert_eq!(alpha(&styled).amount(&document.interner), Ok(expected));
    }
}

#[test]
fn an_absent_attribute_means_what_its_presence_says_it_means() {
    let (styled, document) = parse::<Styled>(ONLY_REQUIRED);
    let interner = &document.interner;

    // Optional with a schema default: the default, which is not in the file and is not written to it.
    assert_eq!(styled.rtl_col(interner), Ok(false));
    // Optional with no default: absent is `None`, which is a different fact from "some default".
    assert_eq!(styled.line_cap(interner), Ok(None));

    // Required: a typed error naming the attribute, never a substituted value.
    let (bare, bare_document) = parse::<Styled>(NOTHING_PRESENT);
    let interner = &bare_document.interner;
    assert_eq!(
        bare.color(interner).err(),
        Some(AttributeError::Missing { attribute: "val" })
    );
    assert_eq!(
        bare.x(interner).err(),
        Some(AttributeError::Missing { attribute: "x" })
    );
    assert_eq!(
        alpha(&bare).amount(interner).err(),
        Some(AttributeError::Missing { attribute: "val" })
    );
}

#[test]
fn a_malformed_value_is_a_typed_error_naming_the_attribute() {
    // One malformed value of each of the five shapes, in one document, so nothing can be missed by
    // an early return: `FFF` is three hex digits, `120` has no `%` and is not a legal thousandths
    // value here only in that it is fine — so the percentage is broken with `half` instead.
    const MALFORMED: &[u8] = br#"<a:demo xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" val='GG0000' rtlCol='yes' cap='round' x='12.5'><a:alpha val='half'/></a:demo>"#;
    let (styled, document) = parse::<Styled>(MALFORMED);
    let interner = &document.interner;

    for (name, error) in [
        ("val", styled.color(interner).err()),
        ("rtlCol", styled.rtl_col(interner).err()),
        ("cap", styled.line_cap(interner).err()),
        ("x", styled.x(interner).err()),
        ("val", alpha(&styled).amount(interner).err()),
    ] {
        let error = error.unwrap_or_else(|| panic!("`{name}` was accepted with a malformed value"));
        assert_eq!(
            error.attribute(),
            name,
            "wrong attribute named by {error:?}"
        );
        assert!(
            matches!(error, AttributeError::InvalidValue { .. }),
            "expected an invalid-value error for `{name}`, got {error:?}"
        );
    }
}

#[test]
fn the_emu_measure_spans_the_signed_64_bit_range() {
    for boundary in [i64::MIN, i64::MAX, -1, 0] {
        let fragment =
            format!(r#"<a:demo xmlns:a="{DML}" val='000000' x='{boundary}'/>"#).into_bytes();
        let (styled, document) = parse::<Styled>(&fragment);
        assert_eq!(styled.x(&document.interner), Ok(Emu(boundary)));
    }
    // One past the top of the range is rejected, not wrapped.
    let fragment =
        format!(r#"<a:demo xmlns:a="{DML}" val='000000' x='9223372036854775808'/>"#).into_bytes();
    let (styled, document) = parse::<Styled>(&fragment);
    assert!(matches!(
        styled.x(&document.interner),
        Err(AttributeError::InvalidValue { attribute: "x", .. })
    ));
}

#[test]
fn a_prefixed_attribute_is_matched_on_its_prefix_not_its_local_name() {
    // Both an unprefixed `embed` and an `r:embed` are present, and they hold different values.
    const BOTH: &[u8] = br#"<a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" embed='unprefixed' r:embed='rId7'/>"#;
    let (blip, document) = parse::<Blip>(BOTH);
    assert_eq!(
        blip.image_relationship(&document.interner)
            .expect("the value is well-formed")
            .as_deref(),
        Some("rId7"),
        "matched the unprefixed attribute, so the prefix is not part of the match"
    );
    // And the round trip leaves the one it did not model exactly where it was.
    assert_bytes(&serialize(&blip, document), BOTH);
}

#[test]
fn an_entity_in_a_value_is_decoded_on_read_and_re_escaped_on_write() {
    const ESCAPED: &[u8] = br#"<a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="urn:r" r:embed='a &amp; b &lt; c'/>"#;
    let (mut blip, document) = parse::<Blip>(ESCAPED);
    assert_eq!(
        blip.image_relationship(&document.interner)
            .expect("the value is well-formed")
            .as_deref(),
        Some("a & b < c"),
        "the read hands back characters, not entity references"
    );

    // Writing the decoded text back reproduces the source byte for byte.
    let mut document = document;
    blip.set_image_relationship(&mut document.interner, Some("a & b < c"));
    assert_bytes(&serialize(&blip, document), ESCAPED);
}

#[test]
fn an_undecodable_reference_is_reported_rather_than_panicked_on() {
    const BOGUS: &[u8] = br#"<a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="urn:r" r:embed='&bogus;'/>"#;
    let (blip, document) = parse::<Blip>(BOGUS);
    assert!(matches!(
        blip.image_relationship(&document.interner),
        Err(AttributeError::InvalidEntity {
            attribute: "r:embed",
            ..
        })
    ));
}

// ---------------------------------------------------------------------------------------------
// Reading does not normalize; writing does.
// ---------------------------------------------------------------------------------------------

#[test]
fn markup_nobody_assigned_to_re_emits_byte_for_byte() {
    // Every property a lifting grammar would lose is in this one comparison: `val='FF0000'` keeps its
    // single quotes, `z:note` keeps its position between two modeled attributes, `rtlCol="on"` keeps
    // the spelling the file used rather than the canonical one, and `val='50%'` keeps the `%` form.
    for fixture in [ALL_PRESENT, ONLY_REQUIRED, NOTHING_PRESENT] {
        let (styled, document) = parse::<Styled>(fixture);
        assert_bytes(&serialize(&styled, document), fixture);
    }
}

#[test]
fn reading_a_default_does_not_write_it() {
    let (styled, document) = parse::<Styled>(ONLY_REQUIRED);
    // Read it — twice, so a getter that memoized into the vector would be caught.
    assert_eq!(styled.rtl_col(&document.interner), Ok(false));
    assert_eq!(styled.rtl_col(&document.interner), Ok(false));
    assert_eq!(styled.line_cap(&document.interner), Ok(None));
    assert_bytes(&serialize(&styled, document), ONLY_REQUIRED);
}

#[test]
fn every_on_off_spelling_reads_and_exactly_one_form_writes() {
    for (spelling, expected) in [
        ("1", true),
        ("0", false),
        ("true", true),
        ("false", false),
        ("on", true),
        ("off", false),
    ] {
        let fragment =
            format!(r#"<a:demo xmlns:a="{DML}" val='000000' rtlCol='{spelling}' x='0'/>"#)
                .into_bytes();
        let (mut styled, mut document) = parse::<Styled>(&fragment);
        assert_eq!(
            styled.rtl_col(&document.interner),
            Ok(expected),
            "`{spelling}` did not read as {expected}"
        );

        // Untouched, the file keeps the spelling it had.
        assert_bytes(
            &serialize(&styled, fidelity::parse(&fragment).unwrap()),
            &fragment,
        );

        // Written, it becomes the one canonical form — in place, keeping the single quotes.
        styled.set_rtl_col(&mut document.interner, Some(expected));
        let canonical =
            format!(r#"<a:demo xmlns:a="{DML}" val='000000' rtlCol='{expected}' x='0'/>"#)
                .into_bytes();
        assert_bytes(&serialize(&styled, document), &canonical);
    }
}

// ---------------------------------------------------------------------------------------------
// Writing — in place, and only the attribute asked for.
// ---------------------------------------------------------------------------------------------

#[test]
fn setting_every_modeled_attribute_leaves_the_unknown_one_where_it_was() {
    let (mut styled, mut document) = parse::<Styled>(ALL_PRESENT);
    styled.set_color(&mut document.interner, "00FF00");
    styled.set_rtl_col(&mut document.interner, Some(false));
    styled.set_line_cap(&mut document.interner, Some(LineCap::Flat));
    styled.set_x(&mut document.interner, Emu(i64::MIN));

    let StyledContent::Alpha(child) = &mut styled.content[0] else {
        panic!("the first content item is the modeled a:alpha");
    };
    child.set_amount(&mut document.interner, Percentage::from_percent(12.5));

    // `z:note` is still the second attribute, still single-quoted, still spelled exactly as it was —
    // and so are the quote characters of every attribute that *was* rewritten.
    const EXPECTED: &[u8] = br#"<a:demo xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:z="urn:mjx:not-ours" val='00FF00' z:note='between the known ones' rtlCol="false" cap='flat' x="-9223372036854775808"><a:alpha val='12500'/></a:demo>"#;
    assert_bytes(&serialize(&styled, document), EXPECTED);
}

#[test]
fn setting_an_optional_attribute_to_none_removes_only_that_attribute() {
    let (mut styled, mut document) = parse::<Styled>(ALL_PRESENT);
    styled.set_line_cap(&mut document.interner, None);

    const EXPECTED: &[u8] = br#"<a:demo xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:z="urn:mjx:not-ours" val='FF0000' z:note='between the known ones' rtlCol="on" x="-914400"><a:alpha val='50%'/></a:demo>"#;
    assert_bytes(&serialize(&styled, document), EXPECTED);
}

#[test]
fn an_attribute_that_was_not_there_is_appended_double_quoted() {
    let (mut styled, mut document) = parse::<Styled>(ONLY_REQUIRED);
    styled.set_line_cap(&mut document.interner, Some(LineCap::Round));
    styled.set_rtl_col(&mut document.interner, Some(true));

    // Appended in the order they were set, after the attributes the file already carried, and
    // double-quoted — what this library writes when it has no precedent to follow.
    const EXPECTED: &[u8] = br#"<a:demo xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" val='00FF00' x="0" cap="rnd" rtlCol="true"><a:alpha val='100000'/></a:demo>"#;
    assert_bytes(&serialize(&styled, document), EXPECTED);
}

#[test]
fn a_written_value_is_escaped_for_the_quote_it_lands_in() {
    const SINGLE: &[u8] = br#"<a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="urn:r" r:embed='x'/>"#;
    let (mut blip, mut document) = parse::<Blip>(SINGLE);
    blip.set_image_relationship(&mut document.interner, Some(r#"it's <a> & "b""#));

    // The attribute was single-quoted, so the apostrophe is escaped and the double quote is not.
    const EXPECTED: &[u8] = br#"<a:blip xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="urn:r" r:embed='it&apos;s &lt;a> &amp; "b"'/>"#;
    assert_bytes(&serialize(&blip, document), EXPECTED);
}

#[test]
fn a_setter_reaches_exactly_one_attribute() {
    // Written as a count rather than as bytes: setting a value must never add, remove or reorder an
    // attribute, and the neighbours' raw bytes must be untouched.
    let (mut styled, mut document) = parse::<Styled>(ALL_PRESENT);
    let before: Vec<(RawName, Box<[u8]>)> = styled
        .attributes
        .iter()
        .map(|attribute| (attribute.name, attribute.value.clone()))
        .collect();

    styled.set_rtl_col(&mut document.interner, Some(false));

    let after: Vec<(RawName, Box<[u8]>)> = styled
        .attributes
        .iter()
        .map(|attribute| (attribute.name, attribute.value.clone()))
        .collect();
    assert_eq!(before.len(), after.len(), "the attribute count moved");
    let changed: Vec<&str> = before
        .iter()
        .zip(&after)
        .filter(|(was, now)| was.1 != now.1)
        .map(|(was, _)| document.interner.resolve(was.0.local))
        .collect();
    assert_eq!(
        changed,
        ["rtlCol"],
        "a setter touched more than its own attribute"
    );
}

// ---------------------------------------------------------------------------------------------
// The grammar composes with what was already there.
// ---------------------------------------------------------------------------------------------

#[test]
fn typed_attributes_and_typed_children_are_independent() {
    // Adding an unmodeled child does not disturb the attributes, and setting an attribute does not
    // disturb the children — the two halves of the derive share only the struct.
    let (mut styled, mut document) = parse::<Styled>(ALL_PRESENT);
    let comment = RawNode::Comment(b" kept ".as_slice().into());
    styled.content.push(StyledContent::Raw(comment));
    styled.set_color(&mut document.interner, "0000FF");

    const EXPECTED: &[u8] = br#"<a:demo xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:z="urn:mjx:not-ours" val='0000FF' z:note='between the known ones' rtlCol="on" cap='sq' x="-914400"><a:alpha val='50%'/><!-- kept --></a:demo>"#;
    assert_bytes(&serialize(&styled, document), EXPECTED);
}

#[test]
fn an_authored_element_gains_its_attributes_double_quoted_in_the_order_they_were_set() {
    // Nothing was parsed here: the type is built from scratch, so every attribute is appended.
    let mut interner = Interner::new();
    let name = RawName {
        prefix: Some(interner.intern("a")),
        local: interner.intern("demo"),
        namespace: Some(interner.intern(DML)),
    };
    let mut styled = Styled {
        name,
        attributes: Vec::new(),
        empty: true,
        content: Vec::new(),
    };
    styled.set_color(&mut interner, "123ABC");
    styled.set_x(&mut interner, Emu(914_400));
    styled.set_line_cap(&mut interner, Some(LineCap::Flat));

    let element = styled.to_xml(&mut interner);
    let rendered: Vec<String> = element
        .attributes
        .iter()
        .map(|attribute| {
            format!(
                "{}={}{}{}",
                interner.resolve(attribute.name.local),
                char::from(attribute.quote.byte()),
                String::from_utf8_lossy(&attribute.value),
                char::from(attribute.quote.byte())
            )
        })
        .collect();
    assert_eq!(rendered, ["val=\"123ABC\"", "x=\"914400\"", "cap=\"flat\""]);
}

// ---------------------------------------------------------------------------------------------
// The attribute vector a declaration works over need not be owned by the declaring type.
//
// `mjx-dml`'s effect, 3-D and line tiers project a handful of facts out of an element they do not
// model as a type: there is no struct to hang `attributes: Vec<RawAttribute>` on, and cloning the
// element's vector to make one would allocate on every read. The accessors need only `AsRef` to read
// and `AsMut` to write, so **one** declaration serves a borrowed read and an owned write.
// ---------------------------------------------------------------------------------------------

/// The attribute face of an `a:demo`, over whatever holds its attributes.
#[derive(XmlAttributes)]
#[xml(attribute(local = "cap", codec = Enumeration<LineCap>, accessor = line_cap))]
#[xml(attribute(local = "x", codec = EmuCoordinate, accessor = offset))]
struct DemoAttributes<A> {
    attributes: A,
}

/// A read-only view. `&[RawAttribute]` is `AsRef<[RawAttribute]>` and is not
/// `AsMut<Vec<RawAttribute>>`, so this type has getters and no setters at all — the read-only case
/// says so in its type rather than in a second grammar.
#[derive(XmlAttributes)]
#[xml(attribute(local = "cap", codec = Enumeration<LineCap>, accessor = line_cap))]
struct DemoView<'a> {
    attributes: &'a [RawAttribute],
}

#[test]
fn a_borrowed_view_reads_the_element_it_does_not_own() {
    let (_styled, document) = parse::<Styled>(ALL_PRESENT);
    let view = DemoView {
        attributes: &document.root.attributes,
    };
    assert_eq!(view.line_cap(&document.interner), Ok(Some(LineCap::Square)));
    // The view borrows the element's own vector — nothing was copied to read through it.
    assert!(std::ptr::eq(
        view.attributes.as_ptr(),
        document.root.attributes.as_ptr()
    ));
}

#[test]
fn one_declaration_serves_a_borrowed_read_and_an_owned_write() {
    let (_styled, document) = parse::<Styled>(ALL_PRESENT);
    let read = DemoAttributes {
        attributes: &document.root.attributes,
    };
    let cap = read
        .line_cap(&document.interner)
        .expect("a legal @cap")
        .expect("@cap is present");
    let offset = read
        .offset(&document.interner)
        .expect("a legal @x")
        .expect("@x is present");
    assert_eq!((cap, offset), (LineCap::Square, Emu(-914_400)));

    // The same declaration, writing the vector a newly built element will own.
    let mut interner = document.interner;
    let mut written = DemoAttributes {
        attributes: Vec::new(),
    };
    written.set_line_cap(&mut interner, Some(cap));
    written.set_offset(&mut interner, Some(offset));
    let spelled: Vec<String> = written
        .attributes
        .iter()
        .map(|attribute| {
            format!(
                "{}={}",
                interner.resolve(attribute.name.local),
                String::from_utf8_lossy(&attribute.value)
            )
        })
        .collect();
    // Written in declaration order, in the codecs' one canonical spelling each.
    assert_eq!(spelled, ["cap=sq", "x=-914400"]);
}
