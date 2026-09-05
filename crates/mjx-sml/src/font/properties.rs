//! `CT_RPrElt` and `CT_Font` — the fifteen font-property slots, decoded once for both.
//!
//! # These are the same type twice
//!
//! `sml.xsd` declares `CT_RPrElt` (line 1826) and `CT_Font` (line 3781) as two `xsd:choice`
//! content models over the same fifteen children. They differ in exactly **two** places, and
//! nowhere else:
//!
//! | | `CT_RPrElt` (a run's `rPr`) | `CT_Font` (a `styles.xml` font) |
//! |---|---|---|
//! | the font-name element | `rFont` | `name` |
//! | the `family` element's type | `CT_IntProperty` (`xsd:int`) | `CT_FontFamily` (`ST_FontFamily`) |
//!
//! Both name types are `CT_FontName`; both `family` types are an integer on the wire. So the pair is
//! one Rust type with a two-valued [`FontPropertyOwner`] saying which spelling to read and write.
//!
//! **MJXOFF-105 (D08) reaches for this module, not for a copy of it.** The workspace has already had
//! to schedule a child (MJXOFF-99) to delete one duplicated SpreadsheetML writer; a second one here
//! would arrive with no executioner at all.
//!
//! # This type is a decoded value, not the preservation mechanism
//!
//! `CLAUDE.md` requires every modelled complex type to carry an unknown bucket and to preserve
//! unknown attributes, attribute order and namespace prefixes. The packed stores in this crate meet
//! that with **bytes** rather than with a `Vec<RawNode>` per record — see
//! [`crate::cells`] for the accounting — and this type is on the same footing. A run's `rPr` is
//! preserved verbatim by the store that holds it ([`RichTextRun::properties_markup`]), and
//! `FontProperties` is what a caller gets when it asks what those bytes *say*. Editing a run's text
//! splices the new text into the preserved bytes, so the `rPr` survives an edit untouched, including
//! anything this type does not model.
//!
//! `extra` therefore holds preserved markup **bytes**, not nodes: a child this type does not
//! recognise is serialized as it stood and replayed after the modelled children when a
//! `FontProperties` is written out. That happens only on the authoring path — the only path where
//! there are no original bytes to copy.
//!
//! # Order
//!
//! Both content models are `xsd:choice`, which imposes **no** order — `mjx_ooxml_types::child_order`
//! reports `ContentModel::Choice` with every slot at rank 0, so there is no ordering table to
//! consult and inventing one would be this crate making up a rule the schema does not have. The
//! writer emits the slots in the schema's *declaration* order because a writer has to pick
//! something and a deterministic choice is testable; any order validates.
//!
//! [`RichTextRun::properties_markup`]: crate::strings::RichTextRun::properties_markup

use mjx_ooxml_core::{Interner, RawElement, RawNode};
use mjx_ooxml_types::shared::VerticalTextPosition;
use mjx_ooxml_types::spreadsheetml::{FontScheme, UnderlineType};

use super::color::Color;
use super::value;

/// Which of the two complex types a [`FontProperties`] was read from, or is being written as.
///
/// The two differ only in the font-name element's spelling and in the declared type of `family`, but
/// both differences are on the wire, so the flavour has to be carried rather than guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontPropertyOwner {
    /// `CT_RPrElt` — the `rPr` of a rich-text run, in `sharedStrings.xml` or in a cell's `<is>`.
    /// The font-name element is `rFont`.
    RichTextRun,
    /// `CT_Font` — one entry of `styles.xml`'s font table (MJXOFF-105). The font-name element is
    /// `name`.
    FontTableEntry,
}

impl FontPropertyOwner {
    /// The local name this flavour spells the font name with.
    #[must_use]
    pub fn font_name_element(self) -> &'static str {
        match self {
            Self::RichTextRun => "rFont",
            Self::FontTableEntry => "name",
        }
    }
}

/// The fifteen font-property children of `CT_RPrElt` / `CT_Font`, decoded.
///
/// **`CT_` symbols:** `CT_RPrElt` (wire element `rPr`) and `CT_Font` (wire element `font`),
/// `sml.xsd`. Every field is `Option` and `None` means the element was **absent**, which is a third
/// state distinct from present-and-false: a run that inherits boldness from its cell format is not a
/// run that switches it off.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FontProperties {
    /// `rFont` / `name` — `CT_FontName`. The typeface name, unescaped.
    pub font_name: Option<String>,
    /// `charset` — `CT_IntProperty`. The legacy Windows character-set number.
    pub character_set: Option<i64>,
    /// `family` — `CT_IntProperty` in a run, `CT_FontFamily` in a font-table entry. Both are an
    /// integer on the wire (0 unknown, 1 roman, 2 swiss, 3 modern, 4 script, 5 decorative), so one
    /// field serves both.
    pub family: Option<i64>,
    /// `b` — `CT_BooleanProperty`.
    pub bold: Option<bool>,
    /// `i` — `CT_BooleanProperty`.
    pub italic: Option<bool>,
    /// `strike` — `CT_BooleanProperty`. Struck through.
    pub strikethrough: Option<bool>,
    /// `outline` — `CT_BooleanProperty`. Outlined glyphs (Macintosh typography).
    pub outline: Option<bool>,
    /// `shadow` — `CT_BooleanProperty`. Shadowed glyphs (Macintosh typography).
    pub shadow: Option<bool>,
    /// `condense` — `CT_BooleanProperty`. Compressed spacing (Macintosh typography).
    pub condensed: Option<bool>,
    /// `extend` — `CT_BooleanProperty`. Expanded spacing (Macintosh typography).
    pub extended: Option<bool>,
    /// `color` — `CT_Color`.
    pub color: Option<Color>,
    /// `sz` — `CT_FontSize`. The point size.
    pub size_in_points: Option<f64>,
    /// `u` — `CT_UnderlineProperty` (`ST_UnderlineValues`). `<u/>` with no `val` means
    /// [`UnderlineType::Single`], which is the schema default and *not* the absence of the element.
    pub underline: Option<UnderlineType>,
    /// `vertAlign` — `CT_VerticalAlignFontProperty` (`ST_VerticalAlignRun`): baseline, superscript
    /// or subscript.
    pub vertical_position: Option<VerticalTextPosition>,
    /// `scheme` — `CT_FontScheme` (`ST_FontScheme`): whether this is the theme's major or minor
    /// font rather than a named one.
    pub scheme: Option<FontScheme>,
    /// Markup this type does not model, serialized as it stood and in document order.
    ///
    /// Bytes rather than `Vec<RawNode>`, for the reason the [module docs](self) give: the stores in
    /// this crate preserve by byte range, and a node list here would be a second, weaker copy of a
    /// guarantee the bytes already make.
    pub extra: Vec<Box<[u8]>>,
}

impl FontProperties {
    /// Whether every slot is absent — the state a `<rPr/>` with no children decodes to.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Reads a `CT_RPrElt` or `CT_Font` element.
    ///
    /// Matches children by **local name**, ignoring the prefix: the elements are in the
    /// SpreadsheetML namespace by construction (they are children of an element that is), and a
    /// producer is free to bind that namespace to any prefix or to none.
    ///
    /// A repeated slot keeps the **last** occurrence, which is what a `xsd:choice maxOccurs`
    /// content model permits a file to write and what a reader that overwrites as it goes naturally
    /// produces. Nothing is refused here: the bytes are preserved by the caller that holds them.
    #[must_use]
    pub fn read(element: &RawElement, interner: &Interner, owner: FontPropertyOwner) -> Self {
        let mut properties = Self::default();
        let font_name_element = owner.font_name_element();
        for child in element.children.iter() {
            let RawNode::Element(child) = child else {
                continue;
            };
            let local = interner.resolve(child.name.local);
            if local == font_name_element {
                properties.font_name = value::value(child, interner);
                continue;
            }
            match local {
                "charset" => properties.character_set = value::integer(child, interner),
                "family" => properties.family = value::integer(child, interner),
                "b" => properties.bold = Some(value::boolean(child, interner)),
                "i" => properties.italic = Some(value::boolean(child, interner)),
                "strike" => properties.strikethrough = Some(value::boolean(child, interner)),
                "outline" => properties.outline = Some(value::boolean(child, interner)),
                "shadow" => properties.shadow = Some(value::boolean(child, interner)),
                "condense" => properties.condensed = Some(value::boolean(child, interner)),
                "extend" => properties.extended = Some(value::boolean(child, interner)),
                "color" => properties.color = Some(Color::read(child, interner)),
                "sz" => properties.size_in_points = value::decimal(child, interner),
                "u" => {
                    properties.underline = Some(
                        value::value(child, interner)
                            .and_then(|text| UnderlineType::from_wire(&text))
                            // `<u/>` is `single`: the schema's own default for `val`.
                            .unwrap_or(UnderlineType::Single),
                    );
                }
                "vertAlign" => {
                    properties.vertical_position = value::value(child, interner)
                        .and_then(|text| VerticalTextPosition::from_wire(&text));
                }
                "scheme" => {
                    properties.scheme =
                        value::value(child, interner).and_then(|text| FontScheme::from_wire(&text));
                }
                _ => properties.extra.push(serialize(child, interner)),
            }
        }
        properties
    }

    /// Reads the `rPr` element of a `CT_RElt` out of the markup of one rich-text run.
    ///
    /// The bytes need not declare the namespaces they use — they are a fragment lifted out of a part
    /// that did — so this matches on local names alone, exactly as [`read`](Self::read) does.
    ///
    /// # Errors
    ///
    /// [`mjx_xml::XmlError`] if `markup` is not well-formed XML.
    pub fn from_markup(markup: &[u8], owner: FontPropertyOwner) -> Result<Self, mjx_xml::XmlError> {
        let document = mjx_xml::fidelity::parse(markup)?;
        Ok(Self::read(&document.root, &document.interner, owner))
    }

    /// Writes this as a `<rPr>…</rPr>` or `<font>…</font>` element named `local`.
    ///
    /// Writes `<local/>` when nothing is set, which is what an empty `CT_RPrElt` is. The slots come
    /// out in the schema's declaration order — see the [module docs](self) on why that is a choice
    /// rather than a requirement — and [`extra`](Self::extra) is replayed last.
    pub fn write_into(
        &self,
        out: &mut Vec<u8>,
        prefix: Option<&str>,
        local: &str,
        owner: FontPropertyOwner,
    ) {
        if self.is_empty() {
            out.push(b'<');
            value::write_qualified_name(out, prefix, local);
            out.extend_from_slice(b"/>");
            return;
        }
        out.push(b'<');
        value::write_qualified_name(out, prefix, local);
        out.push(b'>');

        if let Some(name) = &self.font_name {
            value::write(out, prefix, owner.font_name_element(), Some(name));
        }
        if let Some(character_set) = self.character_set {
            value::write(out, prefix, "charset", Some(&character_set.to_string()));
        }
        if let Some(family) = self.family {
            value::write(out, prefix, "family", Some(&family.to_string()));
        }
        for (slot, flag) in [
            ("b", self.bold),
            ("i", self.italic),
            ("strike", self.strikethrough),
            ("outline", self.outline),
            ("shadow", self.shadow),
            ("condense", self.condensed),
            ("extend", self.extended),
        ] {
            if let Some(flag) = flag {
                value::write(out, prefix, slot, value::boolean_wire(flag));
            }
        }
        if let Some(color) = &self.color {
            color.write_into(out, prefix, "color");
        }
        if let Some(size) = self.size_in_points {
            value::write(out, prefix, "sz", Some(&size.to_string()));
        }
        if let Some(underline) = self.underline {
            // `single` is the schema default for `val`, so `<u/>` says it in fewer bytes — and that
            // is the spelling Excel writes.
            let written = (underline != UnderlineType::Single).then(|| underline.to_wire());
            value::write(out, prefix, "u", written);
        }
        if let Some(position) = self.vertical_position {
            value::write(out, prefix, "vertAlign", Some(position.to_wire()));
        }
        if let Some(scheme) = self.scheme {
            value::write(out, prefix, "scheme", Some(scheme.to_wire()));
        }
        for extra in &self.extra {
            out.extend_from_slice(extra);
        }

        out.extend_from_slice(b"</");
        value::write_qualified_name(out, prefix, local);
        out.push(b'>');
    }

    /// This as markup, for a caller that wants the bytes rather than to append them.
    #[must_use]
    pub fn to_markup(
        &self,
        prefix: Option<&str>,
        local: &str,
        owner: FontPropertyOwner,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        self.write_into(&mut out, prefix, local, owner);
        out
    }
}

/// One unmodelled child, serialized as it stood.
fn serialize(element: &RawElement, interner: &Interner) -> Box<[u8]> {
    let mut bytes = Vec::new();
    mjx_xml::fidelity::serialize_element(element, interner, None, &mut bytes);
    bytes.into_boxed_slice()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(markup: &str, owner: FontPropertyOwner) -> FontProperties {
        FontProperties::from_markup(markup.as_bytes(), owner).expect("the fragment parses")
    }

    #[test]
    fn a_run_and_a_font_table_entry_decode_to_the_same_value() {
        // The whole reuse claim in one case: the two spellings of the font name are the only
        // difference, and both land in the same field.
        let run = read(
            r#"<rPr><rFont val="Calibri"/><sz val="11"/><b/></rPr>"#,
            FontPropertyOwner::RichTextRun,
        );
        let font = read(
            r#"<font><name val="Calibri"/><sz val="11"/><b/></font>"#,
            FontPropertyOwner::FontTableEntry,
        );
        assert_eq!(run, font);
        assert_eq!(run.font_name.as_deref(), Some("Calibri"));
        assert_eq!(run.size_in_points, Some(11.0));
        assert_eq!(run.bold, Some(true));
    }

    #[test]
    fn the_wrong_flavour_does_not_silently_pick_up_the_other_spelling() {
        // `rFont` is not a `CT_Font` child and `name` is not a `CT_RPrElt` child. Reading with the
        // wrong flavour must leave the field unset rather than guess — which is what makes the
        // flavour worth carrying at all.
        let as_font = read(
            r#"<rPr><rFont val="Calibri"/></rPr>"#,
            FontPropertyOwner::FontTableEntry,
        );
        assert_eq!(as_font.font_name, None);
        assert_eq!(
            as_font.extra.len(),
            1,
            "the unrecognised child belongs in the unknown bucket, not nowhere"
        );
    }

    #[test]
    fn absent_present_and_false_are_three_states() {
        assert_eq!(read("<rPr/>", FontPropertyOwner::RichTextRun).bold, None);
        assert_eq!(
            read("<rPr><b/></rPr>", FontPropertyOwner::RichTextRun).bold,
            Some(true)
        );
        assert_eq!(
            read(r#"<rPr><b val="0"/></rPr>"#, FontPropertyOwner::RichTextRun).bold,
            Some(false)
        );
    }

    #[test]
    fn an_underline_with_no_val_is_single_and_not_absent() {
        assert_eq!(
            read("<rPr><u/></rPr>", FontPropertyOwner::RichTextRun).underline,
            Some(UnderlineType::Single),
            "`<u/>` carries the schema's default `val`, which is `single`"
        );
        assert_eq!(
            read(
                r#"<rPr><u val="none"/></rPr>"#,
                FontPropertyOwner::RichTextRun
            )
            .underline,
            Some(UnderlineType::None),
            "`val=\"none\"` is a value, not an absence"
        );
        assert_eq!(
            read("<rPr/>", FontPropertyOwner::RichTextRun).underline,
            None
        );
    }

    #[test]
    fn every_modelled_slot_survives_a_write_and_a_read() {
        let original = FontProperties {
            font_name: Some("Cambria".to_owned()),
            character_set: Some(1),
            family: Some(2),
            bold: Some(true),
            italic: Some(false),
            strikethrough: Some(true),
            outline: Some(false),
            shadow: Some(true),
            condensed: Some(false),
            extended: Some(true),
            color: Some(Color::from_theme(3, Some(0.4))),
            size_in_points: Some(13.5),
            underline: Some(UnderlineType::DoubleAccounting),
            vertical_position: Some(VerticalTextPosition::Superscript),
            scheme: Some(FontScheme::Minor),
            extra: vec![Box::from(&b"<q:keep xmlns:q=\"urn:q\"/>"[..])],
        };
        for owner in [
            FontPropertyOwner::RichTextRun,
            FontPropertyOwner::FontTableEntry,
        ] {
            let markup = original.to_markup(None, "rPr", owner);
            let text = core::str::from_utf8(&markup).expect("utf-8");
            assert_eq!(read(text, owner), original, "{owner:?}: {text}");
        }
    }

    #[test]
    fn an_empty_value_writes_an_empty_element() {
        let markup =
            FontProperties::default().to_markup(Some("x"), "rPr", FontPropertyOwner::RichTextRun);
        assert_eq!(markup, b"<x:rPr/>");
    }

    #[test]
    fn a_prefix_reaches_every_child() {
        let properties = FontProperties {
            bold: Some(true),
            color: Some(Color::from_opaque_rgb("FF0000")),
            ..FontProperties::default()
        };
        let markup = properties.to_markup(Some("x"), "rPr", FontPropertyOwner::RichTextRun);
        assert_eq!(
            markup,
            br#"<x:rPr><x:b/><x:color rgb="FFFF0000"/></x:rPr>"#.to_vec()
        );
    }

    #[test]
    fn the_last_occurrence_of_a_repeated_slot_wins() {
        // `xsd:choice maxOccurs="unbounded"` lets a file write `b` twice. Nothing is refused; the
        // reader states which one it answers with rather than leaving it to whichever branch runs.
        let properties = read(
            r#"<rPr><b val="1"/><b val="0"/></rPr>"#,
            FontPropertyOwner::RichTextRun,
        );
        assert_eq!(properties.bold, Some(false));
    }
}
