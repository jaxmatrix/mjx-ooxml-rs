//! The `val`-wrapper family: eight complex types that are one shape.
//!
//! # The family
//!
//! `sml.xsd` declares eight complex types whose entire content is a single `val` attribute. They are
//! *elements* rather than attributes, which is the shape that catches a reader out — `<b/>` is bold,
//! not an attribute named `b` — and they are the vocabulary both `CT_RPrElt` (a rich-text run's
//! properties) and `CT_Font` (a `styles.xml` font-table entry) are built from:
//!
//! | Symbol | `val` type | Default | Used by |
//! |---|---|---|---|
//! | `CT_BooleanProperty` | `xsd:boolean` | `true` | `b`, `i`, `strike`, `outline`, `shadow`, `condense`, `extend` |
//! | `CT_FontSize` | `xsd:double` | *(required)* | `sz` |
//! | `CT_IntProperty` | `xsd:int` | *(required)* | `charset`, and `family` in `CT_RPrElt` |
//! | `CT_FontName` | `ST_Xstring` | *(required)* | `rFont` / `name` |
//! | `CT_UnderlineProperty` | `ST_UnderlineValues` | `single` | `u` |
//! | `CT_VerticalAlignFontProperty` | `ST_VerticalAlignRun` | *(required)* | `vertAlign` |
//! | `CT_FontScheme` | `ST_FontScheme` | *(required)* | `scheme` |
//! | `CT_FontFamily` | `ST_FontFamily` | *(required)* | `family` in `CT_Font` |
//!
//! Two of them carry a **default**, and that is the part a model gets wrong by accident. `<b/>` and
//! `<b val="1"/>` mean the same thing; `<b val="0"/>` means the opposite; and *no `b` at all* is a
//! third state, because a run that inherits boldness from its cell format is not a run that turns it
//! off. So every reader here answers `Option<T>`, where `None` is *absent* and never *false*.
//!
//! # Why one Rust shape and not eight
//!
//! Eight structs differing only in the type of one field would be eight copies of the same
//! `val`-reading, `val`-writing and default-applying code, and the reader would still have to look
//! at the *element name* to know which is which. This module is instead the shape once — read `val`,
//! parse it as the slot's type, apply the slot's default — and
//! [`FontProperties`](super::FontProperties) is the fifteen slots with a typed accessor each. The
//! type safety a reader wants (`bold()` gives a `bool`, `size_in_points()` gives an `f64`) lives on
//! those accessors, which is where it is useful, rather than on eight wrapper structs a caller would
//! have to unwrap first.
//!
//! `CLAUDE.md`'s two-valued rule is met exactly: the `CT_BooleanProperty` slots are `Option<bool>`,
//! every wire spelling (`1`, `0`, `true`, `false`) is normalized on read, and one canonical form is
//! written.

use mjx_ooxml_core::{Interner, RawElement};

/// The still-escaped bytes of `element`'s `val` attribute, or `None` when it wrote none.
///
/// Matches on the local name only. `val` is unprefixed in every producer's output and a prefixed
/// spelling bound to the SpreadsheetML namespace would mean the same thing, so refusing it would be
/// stricter than the file.
pub(crate) fn raw_value<'a>(element: &'a RawElement, interner: &Interner) -> Option<&'a [u8]> {
    element
        .attributes
        .iter()
        .find(|attribute| interner.resolve(attribute.name.local) == "val")
        .map(|attribute| &*attribute.value)
}

/// `element`'s `val` with its entity references resolved, or `None` when it wrote none or the value
/// is not decodable text.
pub(crate) fn value(element: &RawElement, interner: &Interner) -> Option<String> {
    let raw = raw_value(element, interner)?;
    let text = core::str::from_utf8(raw).ok()?;
    Some(mjx_xml::text::unescape_text(text).ok()?.into_owned())
}

/// A `CT_BooleanProperty` slot: the element's `val`, normalized, defaulting to `true` when the
/// attribute is absent — which is what `<b/>` means.
///
/// A `val` outside the four `xsd:boolean` spellings is read as the default rather than refused: the
/// bytes are preserved elsewhere, and a value this cannot decode is not a reason to fail an open.
pub(crate) fn boolean(element: &RawElement, interner: &Interner) -> bool {
    match raw_value(element, interner) {
        None => true,
        Some(b"1" | b"true") => true,
        Some(b"0" | b"false") => false,
        Some(_) => true,
    }
}

/// A `CT_IntProperty` / `CT_FontFamily` slot, or `None` when `val` is absent or unparseable.
pub(crate) fn integer(element: &RawElement, interner: &Interner) -> Option<i64> {
    core::str::from_utf8(raw_value(element, interner)?)
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// A `CT_FontSize` slot, or `None` when `val` is absent or unparseable.
pub(crate) fn decimal(element: &RawElement, interner: &Interner) -> Option<f64> {
    core::str::from_utf8(raw_value(element, interner)?)
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// Writes one member of the family — `<local val="value"/>`, or `<local/>` when `value` is `None`.
///
/// Self-closing, unprefixed `val`, double quotes: the spelling every producer writes and the one
/// this crate authors, so that authored markup is one shape rather than a per-call decision.
pub(crate) fn write(out: &mut Vec<u8>, prefix: Option<&str>, local: &str, value: Option<&str>) {
    out.push(b'<');
    write_qualified_name(out, prefix, local);
    if let Some(value) = value {
        out.extend_from_slice(b" val=\"");
        out.extend_from_slice(mjx_xml::text::escape_attribute(value).as_bytes());
        out.push(b'"');
    }
    out.extend_from_slice(b"/>");
}

/// Writes `prefix:local`, or `local` when there is no prefix.
pub(crate) fn write_qualified_name(out: &mut Vec<u8>, prefix: Option<&str>, local: &str) {
    if let Some(prefix) = prefix {
        out.extend_from_slice(prefix.as_bytes());
        out.push(b':');
    }
    out.extend_from_slice(local.as_bytes());
}

/// The canonical spelling of a boolean this crate writes: `None` for `true` (so `<b/>`, which is
/// what Excel writes) and `Some("0")` for `false`.
///
/// The asymmetry is the schema's, not a preference: `CT_BooleanProperty`'s `val` defaults to `true`,
/// so the shorter spelling is available for `true` and not for `false`.
pub(crate) fn boolean_wire(value: bool) -> Option<&'static str> {
    if value {
        None
    } else {
        Some("0")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_ooxml_core::RawNode;

    /// Parses one element out of a fragment, so the cases below read as the markup they are about.
    fn element(markup: &str) -> (RawElement, Interner) {
        let document = mjx_xml::fidelity::parse(markup.as_bytes()).expect("the fragment parses");
        let mjx_ooxml_core::RawDocument { interner, root, .. } = document;
        (root, interner)
    }

    #[test]
    fn an_absent_val_is_true_and_a_zero_is_false() {
        for (markup, expected) in [
            ("<b/>", true),
            ("<b val=\"1\"/>", true),
            ("<b val=\"true\"/>", true),
            ("<b val=\"0\"/>", false),
            ("<b val=\"false\"/>", false),
        ] {
            let (element, interner) = element(markup);
            assert_eq!(boolean(&element, &interner), expected, "{markup}");
        }
    }

    #[test]
    fn a_value_outside_the_boolean_spellings_reads_as_the_default_rather_than_failing() {
        let (element, interner) = element("<b val=\"yes\"/>");
        assert!(
            boolean(&element, &interner),
            "an undecodable val must fall back to the schema default, not fail the open"
        );
    }

    #[test]
    fn numbers_and_text_come_back_decoded() {
        let (size, interner) = element("<sz val=\"11.5\"/>");
        assert_eq!(decimal(&size, &interner), Some(11.5));
        let (family, interner) = element("<family val=\"2\"/>");
        assert_eq!(integer(&family, &interner), Some(2));
        let (name, interner) = element("<rFont val=\"Times &amp; Co\"/>");
        assert_eq!(value(&name, &interner).as_deref(), Some("Times & Co"));
    }

    #[test]
    fn an_absent_attribute_is_none_and_not_a_zero() {
        let (element, interner) = element("<sz/>");
        assert_eq!(decimal(&element, &interner), None);
        assert_eq!(integer(&element, &interner), None);
        assert_eq!(value(&element, &interner), None);
    }

    #[test]
    fn writing_round_trips_through_reading() {
        let mut out = Vec::new();
        write(&mut out, None, "rFont", Some("Times & Co"));
        assert_eq!(out, br#"<rFont val="Times &amp; Co"/>"#);
        let (reparsed, interner) = element(core::str::from_utf8(&out).expect("utf-8"));
        assert_eq!(value(&reparsed, &interner).as_deref(), Some("Times & Co"));

        out.clear();
        write(&mut out, Some("x"), "b", boolean_wire(true));
        assert_eq!(out, b"<x:b/>".to_vec());
        out.clear();
        write(&mut out, Some("x"), "b", boolean_wire(false));
        assert_eq!(out, br#"<x:b val="0"/>"#);
    }

    #[test]
    fn an_element_with_no_attributes_at_all_reads_as_absent() {
        // `RawNode` is named so the import is not unused when the case list changes.
        let (element, interner) = element("<u/>");
        assert!(element
            .children
            .iter()
            .all(|c| !matches!(c, RawNode::Text(_))));
        assert_eq!(raw_value(&element, &interner), None);
    }
}
