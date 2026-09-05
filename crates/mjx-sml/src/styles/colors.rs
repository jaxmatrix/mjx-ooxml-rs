//! `x:colors` (`CT_Colors` at `sml.xsd:3670`, `CT_IndexedColors` at `3676`, `CT_MRUColors` at
//! `3681`, `CT_RgbColor` at `3686`) — the workbook's own colour tables.
//!
//! # Two lists that look alike and are not
//!
//! * **`indexedColors`** is a *replacement palette*: sixty-four `rgbColor` elements that redefine
//!   what `<color indexed="N"/>` means everywhere in the workbook. Part 1 §18.8.27: *"When using
//!   the default indexed color palette, the values are not written out, but instead are implied.
//!   When the color palette has been modified from default, then the entire color palette is written
//!   out."* So an absent `indexedColors` is not an empty palette — it is [the default
//!   one](super::palette::IndexedColorPalette::DEFAULT).
//! * **`mruColors`** is the *most-recently-used* list behind the colour picker's bottom row. It
//!   changes nothing about how the workbook renders, and its entries are full `CT_Color`s rather
//!   than the bare `CT_RgbColor` an indexed entry is.
//!
//! The two are different types on the wire for that reason, and they are different types here.

use mjx_ooxml_core::{Interner, Text};
use mjx_ooxml_core::{RawAttribute, RawName, RawNode};
use mjx_ooxml_types::child_order::STYLESHEET_COLOR_TABLE;

use crate::font::{Color, ColorElement};
use crate::leaf::attribute_bag;

attribute_bag! {
    /// `x:rgbColor` (`CT_RgbColor`, `sml.xsd:3686`) — one entry of a replacement palette.
    ///
    /// `@rgb` is `ST_UnsignedIntHex`: eight hex digits, alpha first. Part 1 §18.8.27 prints the
    /// default palette with an alpha of `00` throughout, and this crate reports whatever the file
    /// wrote rather than normalizing it.
    #[xml(attribute(local = "rgb", codec = Text, accessor = rgb))]
    RgbColor, "rgbColor"
}

/// `x:colors` (`CT_Colors`, `sml.xsd:3670`) — the indexed palette and the most-recently-used list.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = SML)]
pub struct ColorTable {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "indexedColors", variant = Indexed, ty = IndexedColors),
        child(local = "mruColors", variant = MostRecentlyUsed, ty = MruColors)
    )]
    content: Vec<ColorTableContent>,
}

/// One child of [`ColorTable`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorTableContent {
    /// `x:indexedColors` (rank 0).
    Indexed(IndexedColors),
    /// `x:mruColors` (rank 1).
    MostRecentlyUsed(MruColors),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl ColorTableContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::Indexed(_) => "indexedColors",
            Self::MostRecentlyUsed(_) => "mruColors",
            Self::Raw(_) => return None,
        })
    }

    /// This child's rank in `CT_Colors`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        STYLESHEET_COLOR_TABLE.rank_of(None, self.local()?)
    }
}

impl ColorTable {
    /// Builds an empty `x:colors`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "colors"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[ColorTableContent] {
        &self.content
    }

    /// `x:indexedColors` — `None` when the workbook uses the default palette, which is what an
    /// absent element means.
    #[must_use]
    pub fn indexed_colors(&self) -> Option<&IndexedColors> {
        self.content.iter().find_map(|item| match item {
            ColorTableContent::Indexed(colors) => Some(colors),
            _ => None,
        })
    }

    /// `x:mruColors` — the colour picker's most-recently-used row.
    #[must_use]
    pub fn most_recently_used(&self) -> Option<&MruColors> {
        self.content.iter().find_map(|item| match item {
            ColorTableContent::MostRecentlyUsed(colors) => Some(colors),
            _ => None,
        })
    }

    /// Sets `x:indexedColors`: `None` removes it, which returns the workbook to the default palette.
    pub fn set_indexed_colors(&mut self, colors: Option<IndexedColors>) {
        self.replace_or_insert(
            "indexedColors",
            |item| matches!(item, ColorTableContent::Indexed(_)),
            colors.map(ColorTableContent::Indexed),
        );
    }

    /// Sets `x:mruColors`, as [`set_indexed_colors`](Self::set_indexed_colors).
    pub fn set_most_recently_used(&mut self, colors: Option<MruColors>) {
        self.replace_or_insert(
            "mruColors",
            |item| matches!(item, ColorTableContent::MostRecentlyUsed(_)),
            colors.map(ColorTableContent::MostRecentlyUsed),
        );
    }

    /// Replaces the first child `is_target` accepts, keeping its position; inserts at the schema
    /// rank when there is none; removes it when `value` is `None`.
    fn replace_or_insert(
        &mut self,
        local: &str,
        is_target: impl Fn(&ColorTableContent) -> bool,
        value: Option<ColorTableContent>,
    ) {
        let existing = self.content.iter().position(&is_target);
        match (existing, value) {
            (Some(at), Some(value)) => self.content[at] = value,
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(value)) => {
                let at = STYLESHEET_COLOR_TABLE
                    .insert_index_of_names(self.content.iter().map(ColorTableContent::rank), local);
                self.content.insert(at, value);
                self.empty = false;
            }
            (None, None) => {}
        }
    }
}

/// `x:indexedColors` (`CT_IndexedColors`, `sml.xsd:3676`) — a replacement for the legacy indexed
/// palette, written **whole** or not at all.
///
/// Decode it into a usable palette with
/// [`IndexedColorPalette::from_indexed_colors`](super::palette::IndexedColorPalette::from_indexed_colors).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = SML)]
pub struct IndexedColors {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "rgbColor", variant = Entry, ty = RgbColor))]
    content: Vec<IndexedColorsContent>,
}

/// One child of [`IndexedColors`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexedColorsContent {
    /// `x:rgbColor` — the schema declares at least one.
    Entry(RgbColor),
    /// Anything else — preserved verbatim, in position, and occupying no index.
    Raw(RawNode),
}

impl IndexedColors {
    /// Builds an empty `x:indexedColors`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "indexedColors"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[IndexedColorsContent] {
        &self.content
    }

    /// Every `x:rgbColor`, in index order — the order **is** the meaning.
    pub fn entries(&self) -> impl Iterator<Item = &RgbColor> + '_ {
        self.content.iter().filter_map(|item| match item {
            IndexedColorsContent::Entry(entry) => Some(entry),
            IndexedColorsContent::Raw(_) => None,
        })
    }

    /// How many entries the palette declares.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries().count()
    }

    /// Whether the palette declares no entry at all, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends an entry after the last one — the only mutation, for the reason
    /// [`super::fonts`] gives about every table addressed by position.
    pub fn push(&mut self, entry: RgbColor) {
        self.content.push(IndexedColorsContent::Entry(entry));
        self.empty = false;
    }
}

/// `x:mruColors` (`CT_MRUColors`, `sml.xsd:3681`) — the colours a user picked most recently.
///
/// Its entries are full `CT_Color`s, not the bare `CT_RgbColor` an indexed palette entry is, so a
/// theme colour can sit in the picker's recent row.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = SML)]
pub struct MruColors {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "color", variant = Color, ty = ColorElement))]
    content: Vec<MruColorsContent>,
}

/// One child of [`MruColors`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MruColorsContent {
    /// `x:color`.
    Color(ColorElement),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl MruColors {
    /// Builds an empty `x:mruColors`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "mruColors"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[MruColorsContent] {
        &self.content
    }

    /// Every `x:color` element, most recent first.
    pub fn color_elements(&self) -> impl Iterator<Item = &ColorElement> + '_ {
        self.content.iter().filter_map(|item| match item {
            MruColorsContent::Color(color) => Some(color),
            MruColorsContent::Raw(_) => None,
        })
    }

    /// Every colour, decoded, most recent first.
    pub fn colours<'a>(&'a self, interner: &'a Interner) -> impl Iterator<Item = Color> + 'a {
        self.color_elements().map(|element| element.color(interner))
    }

    /// Appends a colour after the last one.
    pub fn push(&mut self, color: ColorElement) {
        self.content.push(MruColorsContent::Color(color));
        self.empty = false;
    }
}
