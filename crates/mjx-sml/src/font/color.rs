//! `CT_Color` — SpreadsheetML's colour, which is **not** DrawingML's.
//!
//! # Why this is not `mjx_dml::Color`
//!
//! MJXOFF-97's ticket says to use [`mjx_dml::Color`] for a run's colour and not to introduce an
//! Excel-specific type. That instruction does not survive contact with the two schemas, and the
//! reason is structural rather than a matter of taste:
//!
//! * **DrawingML's colour is an element *choice*.** `EG_ColorChoice` is six elements —
//!   `a:srgbClr`, `a:schemeClr`, `a:sysClr`, `a:prstClr`, `a:scrgbClr`, `a:hslClr` — and the element
//!   *name* is the kind. `mjx_dml::Color` is built on exactly that: its `kind()` reads the name, and
//!   its transforms are child elements.
//! * **SpreadsheetML's colour is one element with five attributes.** `sml.xsd` line 3502 declares
//!   `CT_Color` as `auto`, `indexed`, `rgb`, `theme` and `tint`, and no children at all. The element
//!   is always named for its slot (`color`, `fgColor`, `bgColor`, `tabColor`), never for its kind.
//!
//! `indexed` (a row of the legacy 56-colour palette), `theme` (an index into `theme1.xml`'s colour
//! scheme, not a `SchemeColor` token) and `tint` (a lightening/darkening factor applied to whichever
//! of the four was given) have no representation in `mjx_dml::Color` at all. Routing them through
//! its `ColorSpec::Other` bucket would store `indexed="8"` under an element kind that does not
//! exist, and `tint` nowhere. **That is data loss dressed as reuse**, so this type exists and says
//! why.
//!
//! What the ticket was protecting against is real, and is honoured: there is exactly **one**
//! spreadsheet colour type in this workspace, shared by a rich-text run's `color` here and by
//! everything `styles.xml` colours in MJXOFF-105 — fonts, fills, borders and the tab colour.

use mjx_ooxml_core::{Interner, RawAttribute, RawElement, RawName, RawNode};

use super::value::write_qualified_name;

/// `CT_Color` — a SpreadsheetML colour, in whichever of its four mutually-exclusive spellings the
/// file used, plus the tint applied to it.
///
/// **`ST_` / `CT_` symbol:** `CT_Color`, `sml.xsd`. Wire attributes: `auto`, `indexed`, `rgb`,
/// `theme`, `tint`.
///
/// Every field is `Option` because every attribute is optional and *absent is not zero*: a colour
/// with no `theme` is not a colour with `theme="0"`, which is the first theme colour. The one
/// exception the schema names is `tint`, whose default is `0.0` — read
/// [`tint`](Self::tint) as `None` meaning "not written", and apply `0.0` where a renderer needs a
/// number.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Color {
    /// `@auto` — "the system foreground/background colour", whatever that is at render time.
    pub automatic: Option<bool>,
    /// `@indexed` — a row of the legacy 56-entry indexed palette (`CT_IndexedColors`).
    pub indexed: Option<u32>,
    /// `@rgb` — `ST_UnsignedIntHex`: eight hex digits, **alpha first** (`FFFF0000` is opaque red).
    ///
    /// Kept as the file's own text rather than as a number, because the spelling is part of the
    /// value: `ffff0000` and `FFFF0000` are the same colour and different bytes.
    pub rgb: Option<String>,
    /// `@theme` — a zero-based index into the theme's colour scheme. **Not** a
    /// [`SchemeColor`](mjx_dml::SchemeColor) token; SpreadsheetML addresses theme colours by
    /// position.
    pub theme: Option<u32>,
    /// `@tint` — how far towards white (positive) or black (negative) the chosen colour is shifted,
    /// in `-1.0 ..= 1.0`. Schema default `0.0`.
    pub tint: Option<f64>,
}

impl Color {
    /// An opaque sRGB colour, written `rgb="FFRRGGBB"`.
    ///
    /// Takes the six-digit form callers think in and prefixes the opaque alpha, because `rgb` is
    /// eight digits and a six-digit value there is the single most common way to write a colour
    /// Excel then reads as transparent.
    #[must_use]
    pub fn from_opaque_rgb(hex: &str) -> Self {
        Self {
            rgb: Some(format!("FF{}", hex.trim_start_matches('#'))),
            ..Self::default()
        }
    }

    /// A theme colour by index, optionally tinted.
    #[must_use]
    pub fn from_theme(index: u32, tint: Option<f64>) -> Self {
        Self {
            theme: Some(index),
            tint,
            ..Self::default()
        }
    }

    /// Whether this colour says nothing at all — every attribute absent.
    ///
    /// A `<color/>` with no attributes is legal and means "no colour was specified here", which is
    /// different from the element being absent only in that the file wrote it.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.automatic.is_none()
            && self.indexed.is_none()
            && self.rgb.is_none()
            && self.theme.is_none()
            && self.tint.is_none()
    }

    /// Reads a `CT_Color` element.
    ///
    /// An attribute whose value does not parse is read as absent rather than refused — the bytes
    /// this was decoded from are preserved by whoever holds them, and a malformed `tint` is not a
    /// reason to fail opening a workbook.
    #[must_use]
    pub fn read(element: &RawElement, interner: &Interner) -> Self {
        Self::read_attributes(&element.attributes, interner)
    }

    /// [`read`](Self::read) for a caller that holds the attribute list rather than the element.
    ///
    /// That is the shape a *preserving* holder is in. `crate::worksheet`'s `sheetPr/tabColor` is
    /// kept as an attribute bag — so that an attribute this project has never heard of survives
    /// with its order, quoting and prefix — and decoded through here on demand, rather than being
    /// stored as one of these and written back from it.
    #[must_use]
    pub fn read_attributes(
        attributes: &[mjx_ooxml_core::RawAttribute],
        interner: &Interner,
    ) -> Self {
        let mut color = Self::default();
        for attribute in attributes.iter() {
            let Ok(text) = core::str::from_utf8(&attribute.value) else {
                continue;
            };
            match interner.resolve(attribute.name.local) {
                "auto" => color.automatic = Some(matches!(text.trim(), "1" | "true")),
                "indexed" => color.indexed = text.trim().parse().ok(),
                "rgb" => {
                    color.rgb = mjx_xml::text::unescape_text(text)
                        .ok()
                        .map(std::borrow::Cow::into_owned);
                }
                "theme" => color.theme = text.trim().parse().ok(),
                "tint" => color.tint = text.trim().parse().ok(),
                _ => {}
            }
        }
        color
    }

    /// Writes `<local …/>` with this colour's attributes, in the schema's declaration order.
    ///
    /// `local` is the slot's element name — `color` inside a run's `rPr` or a font, `fgColor` and
    /// `bgColor` inside a pattern fill, `tabColor` on a sheet — because `CT_Color` never names
    /// itself.
    pub(crate) fn write_into(&self, out: &mut Vec<u8>, prefix: Option<&str>, local: &str) {
        out.push(b'<');
        write_qualified_name(out, prefix, local);
        if let Some(automatic) = self.automatic {
            out.extend_from_slice(if automatic {
                b" auto=\"1\""
            } else {
                b" auto=\"0\""
            });
        }
        if let Some(indexed) = self.indexed {
            out.extend_from_slice(format!(" indexed=\"{indexed}\"").as_bytes());
        }
        if let Some(rgb) = &self.rgb {
            out.extend_from_slice(b" rgb=\"");
            out.extend_from_slice(mjx_xml::text::escape_attribute(rgb).as_bytes());
            out.push(b'"');
        }
        if let Some(theme) = self.theme {
            out.extend_from_slice(format!(" theme=\"{theme}\"").as_bytes());
        }
        if let Some(tint) = self.tint {
            out.extend_from_slice(format!(" tint=\"{tint}\"").as_bytes());
        }
        out.extend_from_slice(b"/>");
    }

    /// This colour as an attribute vector, in the same declaration order
    /// [`write_into`](Self::write_into) writes.
    ///
    /// The node-building counterpart of the byte writer, for the callers that hold an element rather
    /// than an output buffer — [`ColorElement::named`] and, through it, every authored colour in
    /// `styles.xml`. `attributes_and_bytes_agree` asserts the two emit the same thing, so the pair
    /// cannot drift apart on the quiet.
    pub(crate) fn to_attributes(&self, interner: &mut Interner) -> Vec<RawAttribute> {
        let mut attributes = Vec::new();
        let mut set = |local: &str, value: &str| {
            mjx_xml::attribute::set(&mut attributes, interner, None, local, value);
        };
        if let Some(automatic) = self.automatic {
            set("auto", if automatic { "1" } else { "0" });
        }
        if let Some(indexed) = self.indexed {
            set("indexed", &indexed.to_string());
        }
        if let Some(rgb) = &self.rgb {
            set("rgb", rgb);
        }
        if let Some(theme) = self.theme {
            set("theme", &theme.to_string());
        }
        if let Some(tint) = self.tint {
            set("tint", &tint.to_string());
        }
        attributes
    }
}

/// `CT_Color` as an **element**: whichever of the five slots it stands in, with the file's own
/// attribute vector untouched.
///
/// # One type for five element names
///
/// `CT_Color` never names itself. `sml.xsd` reaches it under five different local names — `color`
/// (a font's, a border edge's, a gradient stop's, an MRU entry's), `fgColor` and `bgColor` (a
/// pattern fill's), and `tabColor` (a sheet's) — and the type is the same in all five places. So
/// this is one type, and the name it carries is the name the file wrote: [`FromXml`](mjx_ooxml_core::FromXml) keeps the
/// [`RawName`] as it stood, and [`named`](Self::named) takes the local name as an argument rather
/// than baking one in.
///
/// # Why this is not [`Color`]
///
/// The two are *preservation* and *interpretation*, and this crate keeps them apart wherever both
/// are wanted at once. [`Color`] is a decoded snapshot: five `Option`s, no attribute order, no
/// prefixes, nothing this project has never heard of. This type is the element — every attribute in
/// its original position, with its original quote character, including any a later version of the
/// schema adds — and [`color`](Self::color) decodes one on demand.
///
/// A getter takes `&self` and cannot change the file, so a colour that is read is a colour that is
/// written back byte for byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorElement {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl ColorElement {
    /// Builds a `CT_Color` element named `local`, bound to `prefix` — or to the default namespace
    /// when `prefix` is `None` — carrying `color`'s attributes in schema declaration order.
    ///
    /// `local` is the slot's element name, because the type does not name itself: `color`,
    /// `fgColor`, `bgColor` or `tabColor`.
    #[must_use]
    pub fn named(
        interner: &mut Interner,
        prefix: Option<&str>,
        local: &str,
        color: &Color,
    ) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, local),
            attributes: color.to_attributes(interner),
            extra: Vec::new(),
            empty: true,
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// The colour this element states, decoded.
    ///
    /// A snapshot, not the thing that is written back: see the [type documentation](Self).
    #[must_use]
    pub fn color(&self, interner: &Interner) -> Color {
        Color::read_attributes(&self.attributes, interner)
    }

    /// Children this type does not model — `CT_Color` declares none, so anything here is markup the
    /// file wrote that the schema does not allow, preserved rather than discarded.
    #[must_use]
    pub fn extra(&self) -> &[RawNode] {
        &self.extra
    }

    /// This element rebuilt as a [`RawElement`], **without an interner** — see
    /// [`crate::worksheet`] for why the worksheet spine needs one of these on every type it holds.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

impl mjx_ooxml_core::FromXml for ColorElement {
    fn from_xml(
        element: &RawElement,
        _interner: &Interner,
    ) -> Result<Self, mjx_ooxml_core::FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl mjx_ooxml_core::ToXml for ColorElement {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(markup: &str) -> Color {
        let document = mjx_xml::fidelity::parse(markup.as_bytes()).expect("the fragment parses");
        Color::read(&document.root, &document.interner)
    }

    #[test]
    fn the_four_spellings_and_the_tint_all_read() {
        assert_eq!(
            read(r#"<color rgb="FFFF0000"/>"#).rgb.as_deref(),
            Some("FFFF0000")
        );
        assert_eq!(read(r#"<color indexed="8"/>"#).indexed, Some(8));
        assert_eq!(read(r#"<color theme="4" tint="-0.25"/>"#).theme, Some(4));
        assert_eq!(read(r#"<color theme="4" tint="-0.25"/>"#).tint, Some(-0.25));
        assert_eq!(read(r#"<color auto="1"/>"#).automatic, Some(true));
    }

    #[test]
    fn absent_is_not_zero() {
        let color = read("<color/>");
        assert!(color.is_empty());
        assert_eq!(color.theme, None, "no theme is not theme zero");
        assert_eq!(color.tint, None, "no tint is not tint zero");
    }

    #[test]
    fn a_value_that_does_not_parse_is_absent_rather_than_an_error() {
        let color = read(r#"<color theme="not a number" tint="x"/>"#);
        assert_eq!(color.theme, None);
        assert_eq!(color.tint, None);
    }

    #[test]
    fn writing_names_the_slot_because_the_type_does_not() {
        let mut out = Vec::new();
        Color::from_opaque_rgb("FF0000").write_into(&mut out, None, "fgColor");
        assert_eq!(out, br#"<fgColor rgb="FFFF0000"/>"#);
        out.clear();
        Color::from_theme(4, Some(-0.25)).write_into(&mut out, Some("x"), "color");
        assert_eq!(out, br#"<x:color theme="4" tint="-0.25"/>"#);
    }

    /// The byte writer and the attribute builder emit the same thing.
    ///
    /// Two emitters for one type is two chances to drift — one writes `<color …/>` straight into a
    /// buffer, the other builds the vector a [`ColorElement`] owns — so the pair is asserted against
    /// each other rather than each against a literal.
    #[test]
    fn attributes_and_bytes_agree() {
        let color = Color {
            automatic: Some(false),
            indexed: Some(64),
            rgb: Some("FF00FF00".to_owned()),
            theme: Some(2),
            tint: Some(0.5),
        };
        let mut written = Vec::new();
        color.write_into(&mut written, None, "fgColor");

        let mut interner = Interner::default();
        let element = ColorElement::named(&mut interner, None, "fgColor", &color);
        let mut built = Vec::new();
        mjx_xml::fidelity::serialize_element(
            &element.as_raw_element(),
            &interner,
            None,
            &mut built,
        );

        assert_eq!(
            core::str::from_utf8(&built),
            core::str::from_utf8(&written),
            "the node builder and the byte writer must agree on which attributes are written, in \
             which order, and how they are escaped"
        );
        assert_eq!(element.color(&interner), color);
    }

    #[test]
    fn what_is_written_reads_back_as_what_was_written() {
        let original = Color {
            automatic: Some(false),
            indexed: Some(64),
            rgb: Some("FF00FF00".to_owned()),
            theme: Some(2),
            tint: Some(0.5),
        };
        let mut out = Vec::new();
        original.write_into(&mut out, None, "color");
        assert_eq!(
            read(core::str::from_utf8(&out).expect("utf-8")),
            original,
            "every attribute must survive a write followed by a read"
        );
    }
}
