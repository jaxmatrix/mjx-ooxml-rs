//! DrawingML theme: the color scheme and the fill/line style matrices a shape's effective fill and
//! outline resolve against.
//!
//! A theme part (`a:theme`) carries `a:themeElements > { a:clrScheme, a:fontScheme, a:fmtScheme }`.
//! This module models the pieces the fill, outline and text workstreams need — the **color scheme**
//! (the 12 named color slots), the **font scheme** (`a:fontScheme`, the major/minor font collections a
//! `+mj-lt`-style typeface refers to), the **fill-style matrix** (`a:fmtScheme > a:fillStyleLst`), and
//! the **line-style matrix** (`a:fmtScheme > a:lnStyleLst`) — as read-only views. The theme part is
//! never edited here, so the color scheme, font scheme and fill styles are parsed value views; the line
//! styles are the [`LineProperties`] fidelity wrappers (an `a:ln` is a fidelity type). The
//! background-fill list and unknown children are simply not retained.

use mjx_ooxml_core::{FromXml, FromXmlError, Interner, RawElement, RawNode, Text};

use crate::build::{dml_child, first_color_child, is_dml};
use crate::color::{Color, ColorSpec};
use crate::effect::{EffectList, EffectListSpec};
use crate::fill::{Fill, FillSpec};
use crate::line::{LineProperties, LineSpec};
use crate::text::{FontSlot, TextFont};

pub use mjx_ooxml_types::drawingml::ColorSchemeSlot;

// ---------------------------------------------------------------------------------------------
// The attribute faces of the theme elements this module reads but does not model as types
// ---------------------------------------------------------------------------------------------
//
// A font scheme and a supplemental font are projected into the owned values below, so neither is a
// modeled type; both still declare their attributes through the `#[xml(attribute(..))]` grammar over
// the borrowed vector, which copies nothing.

/// `a:fontScheme` (`CT_FontScheme`) — the attribute face of a theme font scheme.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "name", codec = Text, accessor = name))]
struct FontSchemeAttributes<A> {
    attributes: A,
}

/// `a:font` (`CT_SupplementalFont`) — the attribute face of one script's substitute typeface.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "script", codec = Text, accessor = script, required))]
#[xml(attribute(local = "typeface", codec = Text, accessor = typeface, required))]
struct SupplementalFontAttributes<A> {
    attributes: A,
}

/// `a:clrScheme` — the theme's twelve color slots (`dk1`/`lt1`/`dk2`/`lt2`, `accent1`..`accent6`,
/// `hlink`, `folHlink`), each a [`Color`]. Look one up by [`ColorSchemeSlot`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorScheme {
    slots: Vec<(ColorSchemeSlot, Color)>,
}

impl ColorScheme {
    /// The color for `slot`, or `None` if the scheme did not define it.
    #[must_use]
    pub fn color(&self, slot: ColorSchemeSlot) -> Option<&Color> {
        self.slots
            .iter()
            .find_map(|(candidate, color)| (*candidate == slot).then_some(color))
    }

    /// The defined slots and their colors, in document order.
    pub fn slots(&self) -> impl Iterator<Item = (ColorSchemeSlot, &Color)> {
        self.slots.iter().map(|(slot, color)| (*slot, color))
    }

    /// This scheme as interner-free `(slot, ColorSpec)` pairs, resolving each color against `interner`.
    #[must_use]
    fn to_specs(&self, interner: &Interner) -> Vec<(ColorSchemeSlot, ColorSpec)> {
        self.slots
            .iter()
            .map(|(slot, color)| (*slot, color.spec(interner)))
            .collect()
    }
}

impl FromXml for ColorScheme {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let mut slots = Vec::new();
        for node in &element.children {
            let RawNode::Element(child) = node else {
                continue;
            };
            if !is_dml(&child.name, interner) {
                continue;
            }
            let Some(slot) = ColorSchemeSlot::from_wire(interner.resolve(child.name.local)) else {
                continue;
            };
            if let Some(color) = first_color_child(child, interner) {
                slots.push((slot, color));
            }
        }
        Ok(Self { slots })
    }
}

// ---------------------------------------------------------------------------------------------
// The font scheme
// ---------------------------------------------------------------------------------------------

/// Which of a font scheme's two collections a theme font reference names: the **major** font
/// (`+mj-…`), which a theme applies to headings, or the **minor** font (`+mn-…`), which it applies to
/// body text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontSchemeSlot {
    /// `a:majorFont` — the heading font collection, referenced as `+mj-lt` / `+mj-ea` / `+mj-cs`.
    Major,
    /// `a:minorFont` — the body font collection, referenced as `+mn-lt` / `+mn-ea` / `+mn-cs`.
    Minor,
}

/// A parsed theme font reference — the pair a `+mj-lt`-style typeface names: which collection of the
/// theme's [`FontScheme`], and which script slot within it.
///
/// Produced by [`TextFont::theme_reference`](crate::text::TextFont::theme_reference) and consumed by
/// [`FontScheme::font`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThemeFontReference {
    /// The collection named by the `mj` / `mn` part of the reference.
    pub collection: FontSchemeSlot,
    /// The script slot named by the `lt` / `ea` / `cs` part. Never [`FontSlot::Symbol`] — there is no
    /// `+…-sym` spelling, because a font collection has no symbol font.
    pub slot: FontSlot,
}

/// `CT_SupplementalFont` — a per-script fallback typeface (`<a:font script="Jpan" typeface="…"/>`),
/// used for text in a script the collection's three main fonts do not cover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplementalFont {
    script: String,
    typeface: String,
}

impl SupplementalFont {
    /// The script tag this font covers (`@script`) — an ISO 15924 code such as `"Jpan"` or `"Arab"`.
    #[must_use]
    pub fn script(&self) -> &str {
        &self.script
    }

    /// The font name (`@typeface`).
    #[must_use]
    pub fn typeface(&self) -> &str {
        &self.typeface
    }
}

/// `CT_FontCollection` — one half of a [`FontScheme`]: the latin, East Asian and complex-script fonts,
/// plus any per-script supplemental fonts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FontCollection {
    latin: Option<TextFont>,
    east_asian: Option<TextFont>,
    complex_script: Option<TextFont>,
    supplemental: Vec<SupplementalFont>,
}

impl FontCollection {
    /// The font in a script slot. [`FontSlot::Symbol`] is always `None`: a `CT_FontCollection` has no
    /// `a:sym` child, though a run may name one.
    #[must_use]
    pub fn font(&self, slot: FontSlot) -> Option<&TextFont> {
        match slot {
            FontSlot::Latin => self.latin.as_ref(),
            FontSlot::EastAsian => self.east_asian.as_ref(),
            FontSlot::ComplexScript => self.complex_script.as_ref(),
            FontSlot::Symbol => None,
        }
    }

    /// The collection's supplemental fonts (`a:font`), in document order.
    #[must_use]
    pub fn supplemental_fonts(&self) -> &[SupplementalFont] {
        &self.supplemental
    }

    /// The supplemental font declared for `script` (an ISO 15924 code, matched exactly), if any.
    #[must_use]
    pub fn supplemental_font(&self, script: &str) -> Option<&SupplementalFont> {
        self.supplemental.iter().find(|font| font.script == script)
    }

    /// Reads an `a:majorFont` / `a:minorFont` element (or a table style's `a:font`, which is the same
    /// `CT_FontCollection`).
    pub(crate) fn read(element: &RawElement, interner: &Interner) -> Self {
        let font = |local: &str| {
            dml_child(&element.children, interner, local)
                .map(|child| TextFont::read(child, interner))
        };
        let supplemental = element
            .children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(child)
                    if is_dml(&child.name, interner)
                        && interner.resolve(child.name.local) == "font" =>
                {
                    let font = SupplementalFontAttributes {
                        attributes: &child.attributes,
                    };
                    Some(SupplementalFont {
                        script: font.script(interner).unwrap_or_default().into_owned(),
                        typeface: font.typeface(interner).unwrap_or_default().into_owned(),
                    })
                }
                _ => None,
            })
            .collect();

        Self {
            latin: font("latin"),
            east_asian: font("ea"),
            complex_script: font("cs"),
            supplemental,
        }
    }
}

/// `a:fontScheme` — the theme's two font collections. A run that names no font, or that names one of
/// the `+mj-…` / `+mn-…` theme references, ultimately draws with a typeface from here; resolving that
/// reference is the font analogue of resolving a scheme color.
///
/// Interner-free once parsed (a [`TextFont`] owns its strings), so the same value serves both the
/// interner-bound [`Theme`] and the interner-free [`ThemeInfo`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FontScheme {
    name: String,
    major: FontCollection,
    minor: FontCollection,
}

impl FontScheme {
    /// The scheme's name (`@name`), e.g. `"Office"`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The collection for `slot`.
    #[must_use]
    pub fn collection(&self, slot: FontSchemeSlot) -> &FontCollection {
        match slot {
            FontSchemeSlot::Major => &self.major,
            FontSchemeSlot::Minor => &self.minor,
        }
    }

    /// The major (heading) font collection, `a:majorFont`.
    #[must_use]
    pub fn major(&self) -> &FontCollection {
        &self.major
    }

    /// The minor (body) font collection, `a:minorFont`.
    #[must_use]
    pub fn minor(&self) -> &FontCollection {
        &self.minor
    }

    /// The font a parsed theme reference names, or `None` when the scheme leaves that slot undefined.
    #[must_use]
    pub fn font(&self, reference: ThemeFontReference) -> Option<&TextFont> {
        self.collection(reference.collection).font(reference.slot)
    }

    /// The typeface `font` actually asks for: `font` itself when it names a literal font, and the
    /// scheme's font when it is a `+mj-lt`-style theme reference. `None` only when the reference names
    /// a slot this scheme leaves undefined.
    #[must_use]
    pub fn resolve<'a>(&'a self, font: &'a TextFont) -> Option<&'a TextFont> {
        match font.theme_reference() {
            Some(reference) => self.font(reference),
            None => Some(font),
        }
    }

    /// Reads an `a:fontScheme` element.
    fn read(element: &RawElement, interner: &Interner) -> Self {
        let collection = |local: &str| {
            dml_child(&element.children, interner, local)
                .map(|child| FontCollection::read(child, interner))
                .unwrap_or_default()
        };
        Self {
            name: FontSchemeAttributes {
                attributes: &element.attributes,
            }
            .name(interner)
            .ok()
            .flatten()
            .unwrap_or_default()
            .into_owned(),
            major: collection("majorFont"),
            minor: collection("minorFont"),
        }
    }
}

/// `a:theme` — a DrawingML theme, reduced to the pieces effective fill/outline resolution needs: the
/// [`ColorScheme`], the ordered fill styles of `a:fmtScheme > a:fillStyleLst`, and the ordered line
/// styles of `a:fmtScheme > a:lnStyleLst`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    color_scheme: Option<ColorScheme>,
    font_scheme: Option<FontScheme>,
    fill_styles: Vec<Fill>,
    line_styles: Vec<LineProperties>,
    /// The effect styles of `a:fmtScheme > a:effectStyleLst`, positional (1-based via
    /// [`Theme::effect_style`]). Each entry is the `a:effectStyle`'s `a:effectLst`, or `None` when that
    /// style uses the opaque `a:effectDag` alternative — the `None` preserves index alignment.
    effect_styles: Vec<Option<EffectList>>,
}

impl Theme {
    /// The theme's color scheme (`a:clrScheme`), if present.
    #[must_use]
    pub fn color_scheme(&self) -> Option<&ColorScheme> {
        self.color_scheme.as_ref()
    }

    /// The theme's font scheme (`a:fontScheme`), if present.
    #[must_use]
    pub fn font_scheme(&self) -> Option<&FontScheme> {
        self.font_scheme.as_ref()
    }

    /// The fill styles of `a:fmtScheme > a:fillStyleLst`, in order.
    #[must_use]
    pub fn fill_styles(&self) -> &[Fill] {
        &self.fill_styles
    }

    /// The fill style referenced by a **1-based** style-matrix index (as in `a:fillRef@idx`): `1` is
    /// the first style, and `0` — the schema's "no reference" value — returns `None`.
    #[must_use]
    pub fn fill_style(&self, idx: u32) -> Option<&Fill> {
        let position = usize::try_from(idx).ok()?.checked_sub(1)?;
        self.fill_styles.get(position)
    }

    /// The line styles of `a:fmtScheme > a:lnStyleLst`, in order.
    #[must_use]
    pub fn line_styles(&self) -> &[LineProperties] {
        &self.line_styles
    }

    /// The line style referenced by a **1-based** style-matrix index (as in `a:lnRef@idx`): `1` is the
    /// first style, and `0` — the schema's "no reference" value — returns `None`.
    #[must_use]
    pub fn line_style(&self, idx: u32) -> Option<&LineProperties> {
        let position = usize::try_from(idx).ok()?.checked_sub(1)?;
        self.line_styles.get(position)
    }

    /// The effect style referenced by a **1-based** style-matrix index (as in `a:effectRef@idx`): `1` is
    /// the first style, and `0` — the schema's "no reference" value — returns `None`. A style whose
    /// effect properties are the opaque `a:effectDag` (not modeled) also returns `None`.
    #[must_use]
    pub fn effect_style(&self, idx: u32) -> Option<&EffectList> {
        let position = usize::try_from(idx).ok()?.checked_sub(1)?;
        self.effect_styles.get(position)?.as_ref()
    }

    /// This theme as an interner-free [`ThemeInfo`], resolving every color, fill, and line against
    /// `interner` — the value an interner-less caller (`mjx-pptx`'s `theme`) reads.
    #[must_use]
    pub fn to_info(&self, interner: &Interner) -> ThemeInfo {
        ThemeInfo {
            colors: self
                .color_scheme
                .as_ref()
                .map(|scheme| scheme.to_specs(interner))
                .unwrap_or_default(),
            font_scheme: self.font_scheme.clone(),
            fill_styles: self
                .fill_styles
                .iter()
                .map(|fill| fill.spec(interner))
                .collect(),
            line_styles: self
                .line_styles
                .iter()
                .map(|line| line.spec(interner))
                .collect(),
            effect_styles: self
                .effect_styles
                .iter()
                .map(|style| style.as_ref().map(|effects| effects.spec(interner)))
                .collect(),
        }
    }
}

/// An interner-free view of a [`Theme`] — the color scheme as `(slot, ColorSpec)` pairs and the fill
/// styles as [`FillSpec`]s. This is a value description (like [`FillSpec`] itself): scheme colors and
/// fill styles carry their key values, but color transforms and opaque fill internals are dropped
/// (those are retained on the interner-bound [`Theme`] for the color resolver).
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeInfo {
    colors: Vec<(ColorSchemeSlot, ColorSpec)>,
    font_scheme: Option<FontScheme>,
    fill_styles: Vec<FillSpec>,
    line_styles: Vec<LineSpec>,
    effect_styles: Vec<Option<EffectListSpec>>,
}

impl ThemeInfo {
    /// The color for `slot`, or `None` if the theme did not define it.
    #[must_use]
    pub fn color(&self, slot: ColorSchemeSlot) -> Option<&ColorSpec> {
        self.colors
            .iter()
            .find_map(|(candidate, color)| (*candidate == slot).then_some(color))
    }

    /// The defined color slots and their colors, in document order.
    pub fn colors(&self) -> impl Iterator<Item = (ColorSchemeSlot, &ColorSpec)> {
        self.colors.iter().map(|(slot, color)| (*slot, color))
    }

    /// The theme's font scheme (`a:fontScheme`), if present — the same value the interner-bound
    /// [`Theme`] carries, since a [`FontScheme`] owns its strings.
    #[must_use]
    pub fn font_scheme(&self) -> Option<&FontScheme> {
        self.font_scheme.as_ref()
    }

    /// The theme's fill styles (`a:fillStyleLst`), in order.
    #[must_use]
    pub fn fill_styles(&self) -> &[FillSpec] {
        &self.fill_styles
    }

    /// The fill style at a **1-based** style-matrix index (`a:fillRef@idx`); `0` (no reference)
    /// returns `None`.
    #[must_use]
    pub fn fill_style(&self, idx: u32) -> Option<&FillSpec> {
        let position = usize::try_from(idx).ok()?.checked_sub(1)?;
        self.fill_styles.get(position)
    }

    /// The theme's line styles (`a:lnStyleLst`), in order.
    #[must_use]
    pub fn line_styles(&self) -> &[LineSpec] {
        &self.line_styles
    }

    /// The line style at a **1-based** style-matrix index (`a:lnRef@idx`); `0` (no reference)
    /// returns `None`.
    #[must_use]
    pub fn line_style(&self, idx: u32) -> Option<&LineSpec> {
        let position = usize::try_from(idx).ok()?.checked_sub(1)?;
        self.line_styles.get(position)
    }

    /// The effect style at a **1-based** style-matrix index (`a:effectRef@idx`); `0` (no reference), an
    /// out-of-range index, or an `a:effectDag`-based style all return `None`.
    #[must_use]
    pub fn effect_style(&self, idx: u32) -> Option<&EffectListSpec> {
        let position = usize::try_from(idx).ok()?.checked_sub(1)?;
        self.effect_styles.get(position)?.as_ref()
    }
}

impl FromXml for Theme {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let theme_elements = dml_child(&element.children, interner, "themeElements");

        let color_scheme = theme_elements
            .and_then(|elements| dml_child(&elements.children, interner, "clrScheme"))
            .map(|scheme| ColorScheme::from_xml(scheme, interner))
            .transpose()?;

        let font_scheme = theme_elements
            .and_then(|elements| dml_child(&elements.children, interner, "fontScheme"))
            .map(|scheme| FontScheme::read(scheme, interner));

        let fmt_scheme = theme_elements
            .and_then(|elements| dml_child(&elements.children, interner, "fmtScheme"));

        let fill_styles = match fmt_scheme
            .and_then(|scheme| dml_child(&scheme.children, interner, "fillStyleLst"))
        {
            Some(list) => fill_styles_of(list, interner)?,
            None => Vec::new(),
        };

        let line_styles = match fmt_scheme
            .and_then(|scheme| dml_child(&scheme.children, interner, "lnStyleLst"))
        {
            Some(list) => line_styles_of(list, interner)?,
            None => Vec::new(),
        };

        let effect_styles = match fmt_scheme
            .and_then(|scheme| dml_child(&scheme.children, interner, "effectStyleLst"))
        {
            Some(list) => effect_styles_of(list, interner)?,
            None => Vec::new(),
        };

        Ok(Self {
            color_scheme,
            font_scheme,
            fill_styles,
            line_styles,
            effect_styles,
        })
    }
}

/// Parses the `EG_FillProperties` children of an `a:fillStyleLst`, in order.
fn fill_styles_of(list: &RawElement, interner: &Interner) -> Result<Vec<Fill>, FromXmlError> {
    let mut fills = Vec::new();
    for node in &list.children {
        let RawNode::Element(child) = node else {
            continue;
        };
        if is_dml(&child.name, interner) && Fill::is_fill_local(interner.resolve(child.name.local))
        {
            fills.push(Fill::from_xml(child, interner)?);
        }
    }
    Ok(fills)
}

/// Parses the `a:ln` (`CT_LineProperties`) children of an `a:lnStyleLst`, in order.
fn line_styles_of(
    list: &RawElement,
    interner: &Interner,
) -> Result<Vec<LineProperties>, FromXmlError> {
    let mut lines = Vec::new();
    for node in &list.children {
        let RawNode::Element(child) = node else {
            continue;
        };
        if is_dml(&child.name, interner) && interner.resolve(child.name.local) == "ln" {
            lines.push(LineProperties::from_xml(child, interner)?);
        }
    }
    Ok(lines)
}

/// Parses the `a:effectStyle` (`CT_EffectStyleItem`) children of an `a:effectStyleLst`, in order. Each
/// yields its `a:effectLst` as an [`EffectList`], or `None` when the style uses the opaque `a:effectDag`
/// alternative (the `None` keeps the positional 1-based `a:effectRef@idx` alignment). Any `a:scene3d` /
/// `a:sp3d` siblings are not modeled.
fn effect_styles_of(
    list: &RawElement,
    interner: &Interner,
) -> Result<Vec<Option<EffectList>>, FromXmlError> {
    let mut styles = Vec::new();
    for node in &list.children {
        let RawNode::Element(child) = node else {
            continue;
        };
        if is_dml(&child.name, interner) && interner.resolve(child.name.local) == "effectStyle" {
            let effects = match dml_child(&child.children, interner, "effectLst") {
                Some(effect_lst) => Some(EffectList::from_xml(effect_lst, interner)?),
                None => None,
            };
            styles.push(effects);
        }
    }
    Ok(styles)
}

// ---------------------------------------------------------------------------------------------
// Authoring a theme
// ---------------------------------------------------------------------------------------------

/// The bytes of a complete `a:theme` part carrying the Office palette — the one theme this
/// workspace authors, for whichever format needs one.
///
/// # Why a byte producer rather than a writer over [`Theme`]
///
/// [`Theme`] is a **read-only** view: it keeps the color scheme, the font scheme and the two style
/// matrices and drops everything else (`a:bgFillStyleLst`, unknown children, the `@name`
/// attributes), because resolving a fill never needs them. Serializing that view would emit a theme
/// with holes in it. A theme this project *authors* is a fixed, complete document with no caller
/// input at all, so the honest shape is the document itself.
///
/// # What it says, and why each piece is the value it is
///
/// The colour scheme is the Office 2013 palette, so a file built here looks like a file built in
/// Word, Excel or PowerPoint rather than like a debugging artefact. `dk1`/`lt1` are plain
/// `a:srgbClr` rather than `a:sysClr`: the value is then the same everywhere, which is what the
/// effective-colour readers resolve against.
///
/// The three fill styles are the same colour at three strengths (`phClr` is the placeholder the
/// shape's `a:fillRef` substitutes), the three line styles are three widths, and the three effect
/// styles are empty — `a:effectStyle` requires an effect group, and an empty `a:effectLst` is the
/// honest way to say "no effect" rather than inventing a shadow nothing asked for.
///
/// # Who writes it
///
/// A theme is DrawingML, and all three formats carry the identical part under a different name
/// (`/ppt/theme/theme1.xml`, `/word/theme/theme1.xml`, `/xl/theme/theme1.xml`). Keeping the markup
/// here is what stops it being written out three times: `mjx_pptx::blank` puts it in every deck,
/// `mjx_sml::write::WorkbookPackage` puts it in every authored workbook, and `mjx-docx` and
/// `mjx-xlsx` author it **only into a package that has none** — a document that arrives with a theme
/// keeps the one it came with, because supplying a default in place of the user's own would override
/// the branding of whoever opens the file.
///
/// Deterministic: every call returns the same bytes.
#[must_use]
pub fn default_theme_xml() -> Vec<u8> {
    DEFAULT_THEME_XML.as_bytes().to_vec()
}

/// [`default_theme_xml`] without the copy, for a caller that only wants to read it.
pub const DEFAULT_THEME_XML: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    "\n",
    r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#,
    r#" name="Office Theme">"#,
    "<a:themeElements>",
    // --- colours -----------------------------------------------------------------------------
    r#"<a:clrScheme name="Office">"#,
    r#"<a:dk1><a:srgbClr val="000000"/></a:dk1>"#,
    r#"<a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>"#,
    r#"<a:dk2><a:srgbClr val="44546A"/></a:dk2>"#,
    r#"<a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>"#,
    r#"<a:accent1><a:srgbClr val="4472C4"/></a:accent1>"#,
    r#"<a:accent2><a:srgbClr val="ED7D31"/></a:accent2>"#,
    r#"<a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>"#,
    r#"<a:accent4><a:srgbClr val="FFC000"/></a:accent4>"#,
    r#"<a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>"#,
    r#"<a:accent6><a:srgbClr val="70AD47"/></a:accent6>"#,
    r#"<a:hlink><a:srgbClr val="0563C1"/></a:hlink>"#,
    r#"<a:folHlink><a:srgbClr val="954F72"/></a:folHlink>"#,
    "</a:clrScheme>",
    // --- fonts -------------------------------------------------------------------------------
    r#"<a:fontScheme name="Office">"#,
    r#"<a:majorFont><a:latin typeface="Calibri Light"/><a:ea typeface=""/>"#,
    r#"<a:cs typeface=""/></a:majorFont>"#,
    r#"<a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/>"#,
    r#"<a:cs typeface=""/></a:minorFont>"#,
    "</a:fontScheme>",
    // --- the style matrix --------------------------------------------------------------------
    r#"<a:fmtScheme name="Office">"#,
    "<a:fillStyleLst>",
    r#"<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>"#,
    r#"<a:solidFill><a:schemeClr val="phClr"><a:tint val="60000"/></a:schemeClr></a:solidFill>"#,
    r#"<a:solidFill><a:schemeClr val="phClr"><a:shade val="80000"/></a:schemeClr></a:solidFill>"#,
    "</a:fillStyleLst>",
    "<a:lnStyleLst>",
    r#"<a:ln w="6350" cap="flat" cmpd="sng" algn="ctr">"#,
    r#"<a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln>"#,
    r#"<a:ln w="12700" cap="flat" cmpd="sng" algn="ctr">"#,
    r#"<a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln>"#,
    r#"<a:ln w="19050" cap="flat" cmpd="sng" algn="ctr">"#,
    r#"<a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln>"#,
    "</a:lnStyleLst>",
    "<a:effectStyleLst>",
    "<a:effectStyle><a:effectLst/></a:effectStyle>",
    "<a:effectStyle><a:effectLst/></a:effectStyle>",
    "<a:effectStyle><a:effectLst/></a:effectStyle>",
    "</a:effectStyleLst>",
    "<a:bgFillStyleLst>",
    r#"<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>"#,
    r#"<a:solidFill><a:schemeClr val="phClr"><a:tint val="60000"/></a:schemeClr></a:solidFill>"#,
    r#"<a:solidFill><a:schemeClr val="phClr"><a:shade val="80000"/></a:schemeClr></a:solidFill>"#,
    "</a:bgFillStyleLst>",
    "</a:fmtScheme>",
    "</a:themeElements>",
    "</a:theme>",
);
