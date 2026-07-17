//! DrawingML shape-style references and the theme color map — the two remaining inputs a shape's
//! *effective* fill resolves against (with the theme from [`crate::theme`]).
//!
//! [`StyleMatrixReference`] models `a:fillRef` (and its `a:lnRef`/`a:effectRef` siblings): a 1-based
//! index into a theme style-matrix list plus the color that substitutes the style's `phClr`.
//! [`ColorMap`] models the master's `p:clrMap` (with any `p:clrMapOvr`): it maps the logical color
//! names a shape can name (`bg1`/`tx1`/…) to the concrete [`ColorSchemeSlot`]s of the theme scheme.

use mjx_ooxml_core::{FromXml, FromXmlError, Interner, RawElement};

use crate::build::{attr_str, first_color_child};
use crate::color::{Color, SchemeColor};
use crate::theme::ColorSchemeSlot;

/// A style-matrix reference (`CT_StyleMatrixReference`: `a:fillRef` / `a:lnRef` / `a:effectRef`) — the
/// `@idx` into a theme style list plus the optional color that replaces the referenced style's
/// `phClr`. A read-only parsed view (this workstream does not write style references).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleMatrixReference {
    idx: Option<u32>,
    color: Option<Color>,
}

impl StyleMatrixReference {
    /// The style-matrix index (`@idx`) — a **1-based** index into the theme list the reference names
    /// (for `a:fillRef`, `a:fmtScheme > a:fillStyleLst`); `0` means "no reference". `None` if the
    /// attribute is absent or not a `u32`.
    #[must_use]
    pub fn idx(&self) -> Option<u32> {
        self.idx
    }

    /// The reference's color (the `EG_ColorChoice` child), which substitutes the referenced style's
    /// `phClr`. `None` if the reference carries no color.
    #[must_use]
    pub fn color(&self) -> Option<&Color> {
        self.color.as_ref()
    }
}

impl FromXml for StyleMatrixReference {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let idx =
            attr_str(&element.attributes, interner, "idx").and_then(|s| s.parse::<u32>().ok());
        let color = first_color_child(element, interner);
        Ok(Self { idx, color })
    }
}

/// The theme color map (`p:clrMap`, optionally overridden by `p:clrMapOvr`): the mapping from the
/// logical color names a shape may reference (`bg1`/`tx1`/`bg2`/`tx2`, `accent1`..`accent6`, `hlink`,
/// `folHlink`) to the concrete [`ColorSchemeSlot`]s of the theme's color scheme.
///
/// Resolving a `schemeClr` color: `bg1`/`tx1`/`bg2`/`tx2` and the accents/hyperlinks pass through this
/// map; `dk1`/`lt1`/`dk2`/`lt2` reference a scheme slot directly (bypassing the map); `phClr` is not a
/// scheme color (it is substituted from a [`StyleMatrixReference`]'s color).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorMap {
    /// The slot `bg1` maps to.
    pub background1: ColorSchemeSlot,
    /// The slot `tx1` maps to.
    pub text1: ColorSchemeSlot,
    /// The slot `bg2` maps to.
    pub background2: ColorSchemeSlot,
    /// The slot `tx2` maps to.
    pub text2: ColorSchemeSlot,
    /// The slot `accent1` maps to.
    pub accent1: ColorSchemeSlot,
    /// The slot `accent2` maps to.
    pub accent2: ColorSchemeSlot,
    /// The slot `accent3` maps to.
    pub accent3: ColorSchemeSlot,
    /// The slot `accent4` maps to.
    pub accent4: ColorSchemeSlot,
    /// The slot `accent5` maps to.
    pub accent5: ColorSchemeSlot,
    /// The slot `accent6` maps to.
    pub accent6: ColorSchemeSlot,
    /// The slot `hlink` maps to.
    pub hyperlink: ColorSchemeSlot,
    /// The slot `folHlink` maps to.
    pub followed_hyperlink: ColorSchemeSlot,
}

impl ColorMap {
    /// The standard identity map used when a deck declares no color map: `bg1→lt1`, `tx1→dk1`,
    /// `bg2→lt2`, `tx2→dk2`, and each accent / hyperlink to its like-named slot.
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            background1: ColorSchemeSlot::Light1,
            text1: ColorSchemeSlot::Dark1,
            background2: ColorSchemeSlot::Light2,
            text2: ColorSchemeSlot::Dark2,
            accent1: ColorSchemeSlot::Accent1,
            accent2: ColorSchemeSlot::Accent2,
            accent3: ColorSchemeSlot::Accent3,
            accent4: ColorSchemeSlot::Accent4,
            accent5: ColorSchemeSlot::Accent5,
            accent6: ColorSchemeSlot::Accent6,
            hyperlink: ColorSchemeSlot::Hyperlink,
            followed_hyperlink: ColorSchemeSlot::FollowedHyperlink,
        }
    }

    /// The concrete scheme slot a [`SchemeColor`] resolves to under this map, or `None` for `phClr`
    /// (which is not a scheme color — it is substituted from a style reference's color).
    #[must_use]
    pub fn resolve(&self, color: SchemeColor) -> Option<ColorSchemeSlot> {
        Some(match color {
            SchemeColor::Background1 => self.background1,
            SchemeColor::Text1 => self.text1,
            SchemeColor::Background2 => self.background2,
            SchemeColor::Text2 => self.text2,
            SchemeColor::Accent1 => self.accent1,
            SchemeColor::Accent2 => self.accent2,
            SchemeColor::Accent3 => self.accent3,
            SchemeColor::Accent4 => self.accent4,
            SchemeColor::Accent5 => self.accent5,
            SchemeColor::Accent6 => self.accent6,
            SchemeColor::Hyperlink => self.hyperlink,
            SchemeColor::FollowedHyperlink => self.followed_hyperlink,
            // dk1/lt1/dk2/lt2 reference a scheme slot directly, bypassing the map.
            SchemeColor::Dark1 => ColorSchemeSlot::Dark1,
            SchemeColor::Light1 => ColorSchemeSlot::Light1,
            SchemeColor::Dark2 => ColorSchemeSlot::Dark2,
            SchemeColor::Light2 => ColorSchemeSlot::Light2,
            SchemeColor::PlaceholderColor => return None,
        })
    }
}
