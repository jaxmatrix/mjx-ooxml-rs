//! DrawingML theme: the color scheme and the fill/line style matrices a shape's effective fill and
//! outline resolve against.
//!
//! A theme part (`a:theme`) carries `a:themeElements > { a:clrScheme, a:fontScheme, a:fmtScheme }`.
//! This module models the pieces the fill and outline workstreams need — the **color scheme** (the 12
//! named color slots), the **fill-style matrix** (`a:fmtScheme > a:fillStyleLst`), and the **line-style
//! matrix** (`a:fmtScheme > a:lnStyleLst`) — as read-only views. The theme part is never edited here,
//! so the color scheme and fill styles are parsed value views; the line styles are the [`LineProperties`]
//! fidelity wrappers (an `a:ln` is a fidelity type). The font scheme, effect/background-fill lists, and
//! unknown children are simply not retained.

use mjx_ooxml_core::{FromXml, FromXmlError, Interner, RawElement, RawNode};

use crate::build::{dml_child, first_color_child, is_dml};
use crate::color::{Color, ColorSpec};
use crate::effect::{EffectList, EffectListSpec};
use crate::fill::{Fill, FillSpec};
use crate::line::{LineProperties, LineSpec};

pub use mjx_ooxml_types::drawingml::ColorSchemeSlot;

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

/// `a:theme` — a DrawingML theme, reduced to the pieces effective fill/outline resolution needs: the
/// [`ColorScheme`], the ordered fill styles of `a:fmtScheme > a:fillStyleLst`, and the ordered line
/// styles of `a:fmtScheme > a:lnStyleLst`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    color_scheme: Option<ColorScheme>,
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
