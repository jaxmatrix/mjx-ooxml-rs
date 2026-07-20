//! `CT_TextFont` — a typeface reference, as named by a run (`a:latin`, `a:ea`, `a:cs`, `a:sym`) or by
//! the theme's font scheme.
//!
//! Four attributes and no children, so [`TextFont`] is modeled **whole** as an interner-free value
//! rather than as a fidelity wrapper: there is nothing left over to preserve opaquely.

use mjx_ooxml_core::{Interner, RawAttribute, RawElement};

use crate::build::{attr_str, dml_attr, dml_element};
use crate::text::FontSlot;
use crate::theme::{FontSchemeSlot, ThemeFontReference};

/// A typeface reference (`CT_TextFont`) — the font a run asks for.
///
/// `typeface` is the only required part and may be either a literal font name (`"Calibri"`) or a
/// **theme reference**: `+mj-lt` / `+mn-lt` mean "the theme's major/minor latin font", with `-ea` and
/// `-cs` for the East Asian and complex-script slots. Resolving those against the theme's font scheme
/// is the font analogue of resolving a scheme color, and lands with the font scheme itself.
///
/// The metric hints (`panose`, `pitchFamily`, `charset`) are carried verbatim so a font a caller reads
/// and writes back is unchanged. They describe the font, not the choice of it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextFont {
    /// The font name (`@typeface`), or a `+mj-*` / `+mn-*` theme reference.
    pub typeface: String,
    /// The PANOSE classification (`@panose`), a 10-byte hex string, if given.
    pub panose: Option<String>,
    /// The pitch-and-family byte (`@pitchFamily`), if given.
    pub pitch_family: Option<i32>,
    /// The character set byte (`@charset`), if given.
    pub charset: Option<i32>,
}

impl TextFont {
    /// A font named literally — `TextFont::named("Calibri")`.
    #[must_use]
    pub fn named(typeface: &str) -> Self {
        Self {
            typeface: typeface.to_owned(),
            ..Self::default()
        }
    }

    /// Whether [`typeface`](Self::typeface) is a theme reference (`+mj-lt`, `+mn-ea`, …) rather than a
    /// literal font name. Such a reference names a slot in the theme's font scheme.
    #[must_use]
    pub fn is_theme_reference(&self) -> bool {
        self.typeface.starts_with('+')
    }

    /// This typeface parsed as a theme font reference — the font-scheme collection and script slot it
    /// names — or `None` for a literal font name or an unrecognized `+…` spelling.
    ///
    /// The six spellings the schema defines are matched exactly: `+mj-lt`, `+mj-ea`, `+mj-cs` for the
    /// major (heading) collection and `+mn-lt`, `+mn-ea`, `+mn-cs` for the minor (body) one. There is
    /// no `+…-sym`, because a font collection has no symbol font.
    #[must_use]
    pub fn theme_reference(&self) -> Option<ThemeFontReference> {
        let (collection, slot) = match self.typeface.as_str() {
            "+mj-lt" => (FontSchemeSlot::Major, FontSlot::Latin),
            "+mj-ea" => (FontSchemeSlot::Major, FontSlot::EastAsian),
            "+mj-cs" => (FontSchemeSlot::Major, FontSlot::ComplexScript),
            "+mn-lt" => (FontSchemeSlot::Minor, FontSlot::Latin),
            "+mn-ea" => (FontSchemeSlot::Minor, FontSlot::EastAsian),
            "+mn-cs" => (FontSchemeSlot::Minor, FontSlot::ComplexScript),
            _ => return None,
        };
        Some(ThemeFontReference { collection, slot })
    }

    /// Reads a `CT_TextFont` element (`a:latin`, `a:ea`, `a:cs`, `a:sym`, `a:buFont`, or a font-scheme
    /// slot). An element with no `@typeface` reads as an empty name rather than failing — the file is
    /// malformed, not unreadable.
    #[must_use]
    pub(crate) fn read(element: &RawElement, interner: &Interner) -> Self {
        Self {
            typeface: attr_str(&element.attributes, interner, "typeface")
                .unwrap_or_default()
                .to_owned(),
            panose: attr_str(&element.attributes, interner, "panose").map(str::to_owned),
            pitch_family: attr_str(&element.attributes, interner, "pitchFamily")
                .and_then(|s| s.trim().parse().ok()),
            charset: attr_str(&element.attributes, interner, "charset")
                .and_then(|s| s.trim().parse().ok()),
        }
    }

    /// Builds the element for this font under the given `local` name (`latin`, `ea`, `cs`, `sym`, …),
    /// in `CT_TextFont` attribute order.
    #[must_use]
    pub(crate) fn build(&self, interner: &mut Interner, local: &str) -> RawElement {
        let mut attributes: Vec<RawAttribute> =
            vec![dml_attr(interner, "typeface", &self.typeface)];
        if let Some(panose) = &self.panose {
            attributes.push(dml_attr(interner, "panose", panose));
        }
        if let Some(pitch_family) = self.pitch_family {
            attributes.push(dml_attr(interner, "pitchFamily", &pitch_family.to_string()));
        }
        if let Some(charset) = self.charset {
            attributes.push(dml_attr(interner, "charset", &charset.to_string()));
        }
        dml_element(interner, local, attributes, Vec::new())
    }
}
