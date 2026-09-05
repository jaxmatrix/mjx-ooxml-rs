//! `x:fonts` / `x:font` (`CT_Fonts` at `sml.xsd:3477`, `CT_Font` at `3781`) — the font table.
//!
//! # A font's identity is its position
//!
//! Nothing in a workbook names a font. An `xf` says `fontId="3"` and a `dxf` carries one inline;
//! `fontId="3"` means *the fourth `<font>` element of `<fonts>`*, counted in document order. So the
//! table is an **array**, and this module offers exactly the operations an array of identities
//! allows: read one by index, iterate them, and append. There is no `remove`, no `insert_at`, no
//! `sort` and no deduplication, because every one of them renumbers the entries after the one it
//! touches and silently repaints every cell that referred to them. [`FontTable::push`] is the only
//! mutation, and `@count` moves with it.
//!
//! That is not a hypothetical. `tests/fixtures/style_resources.xlsx` writes `<font>` entries 2 and 3
//! byte-identically, which is what a real producer leaves behind after a user removes a format —
//! and it is exactly the shape an implementer is tempted to "optimise" away.
//!
//! # `CT_Font` is not modelled twice
//!
//! `CT_Font` is character for character the same fifteen font-property slots as `CT_RPrElt`, a rich
//! text run's `rPr`, differing only in `rFont`/`name` and in `family`'s declared type. MJXOFF-97
//! modelled that family once, in [`crate::font`], deliberately outside both subjects. [`Font`] is
//! therefore the *element* — every child in its original position, every attribute a later schema
//! might add — and [`Font::properties`] decodes it through
//! [`FontProperties`] on demand, with
//! [`FontPropertyOwner::FontTableEntry`].
//!
//! Preservation and interpretation are different jobs, and this crate keeps them apart wherever both
//! are wanted at once — the same split `crate::worksheet` makes for `sheetPr/tabColor`.

use mjx_ooxml_core::{
    FromXml, FromXmlError, Interner, Number, RawAttribute, RawElement, RawName, RawNode, ToXml,
};

use crate::error::SmlError;
use crate::font::{FontProperties, FontPropertyOwner};

/// `x:font` (`CT_Font`, `sml.xsd:3781`) — one entry of the font table.
///
/// The element as the file wrote it. Its fifteen property children are read through
/// [`properties`](Self::properties) rather than stored decoded, so a `<font>` this project has never
/// seen the whole of still writes back byte for byte.
///
/// `CT_Font`'s content model is `xsd:choice maxOccurs="unbounded"`, so the schema imposes **no**
/// order on those children and this type invents none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Font {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl Font {
    /// The wire local name this type is written under.
    pub const WIRE_LOCAL: &'static str = "font";

    /// Builds an `x:font` from decoded properties, bound to `prefix` — or to the default namespace
    /// when `prefix` is `None`.
    ///
    /// The element is exactly what [`FontProperties::write_into`] emits, parsed back and re-interned
    /// into `interner`: **one** description of the fifteen slots, not two. That is the design and
    /// not a shortcut — `FontProperties` keeps the markup it does not model as *bytes*, so a
    /// node-building writer would have to parse those anyway, and the two writers would then be free
    /// to drift apart.
    ///
    /// # Errors
    /// [`SmlError::Xml`] if an entry of `properties.extra` is not well-formed XML. Markup this crate
    /// produced never is; a hand-authored unknown bucket can be.
    pub fn from_properties(
        interner: &mut Interner,
        prefix: Option<&str>,
        properties: &FontProperties,
    ) -> Result<Self, SmlError> {
        let markup =
            properties.to_markup(prefix, Self::WIRE_LOCAL, FontPropertyOwner::FontTableEntry);
        let element = crate::leaf::parse_into(&markup, interner)?;
        Ok(Self::from_element(&element))
    }

    /// This entry's fifteen font-property slots, decoded.
    ///
    /// A snapshot: changing it changes nothing in the table. It is [`FontProperties`], the same type
    /// a rich-text run's `rPr` decodes to, so a caller comparing a run's font with a table entry's
    /// compares two values of one type.
    #[must_use]
    pub fn properties(&self, interner: &Interner) -> FontProperties {
        FontProperties::read(
            &self.as_raw_element(),
            interner,
            FontPropertyOwner::FontTableEntry,
        )
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order and exactly as it was read.
    #[must_use]
    pub fn children(&self) -> &[RawNode] {
        &self.children
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self.children.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }

    /// The shared body of [`FromXml`] and [`from_properties`](Self::from_properties).
    fn from_element(element: &RawElement) -> Self {
        Self {
            name: element.name,
            attributes: element.attributes.clone(),
            children: element.children.clone(),
            empty: element.empty,
        }
    }
}

impl FromXml for Font {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self::from_element(element))
    }
}

impl ToXml for Font {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:fonts` (`CT_Fonts`, `sml.xsd:3477`) — the font table, in index order.
///
/// See the [module documentation](self) for why the only mutation is [`push`](Self::push).
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count))]
pub struct FontTable {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "font", variant = Font, ty = Font))]
    content: Vec<FontTableContent>,
}

/// One child of [`FontTable`]: a font entry, or markup this type does not model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontTableContent {
    /// `x:font`.
    Font(Font),
    /// Anything else — preserved verbatim, in position. It does **not** occupy an index: see
    /// [`FontTable::get`].
    Raw(RawNode),
}

impl FontTable {
    /// Builds an empty `x:fonts`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "fonts"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[FontTableContent] {
        &self.content
    }

    /// Every `x:font`, in index order.
    pub fn fonts(&self) -> impl Iterator<Item = &Font> + '_ {
        self.content.iter().filter_map(|item| match item {
            FontTableContent::Font(font) => Some(font),
            FontTableContent::Raw(_) => None,
        })
    }

    /// The font at `index` — the number an `xf`'s `@fontId` carries.
    ///
    /// Indexes the **font entries**, stepping over anything unmodelled between them, because a
    /// comment between two `<font>` elements does not shift the numbering Excel counts in.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Font> {
        self.fonts().nth(index)
    }

    /// How many fonts the table holds — counted, not read from `@count`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fonts().count()
    }

    /// Whether the table holds no font at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends `font` after the last entry, giving it the next index, and updates `@count`.
    ///
    /// **The only mutation this type offers**, for the reason the [module documentation](self)
    /// gives. `@count` is rewritten only when the file already declared one: the attribute is
    /// optional, and adding one to a table that had none would author markup the producer chose not
    /// to write.
    pub fn push(&mut self, interner: &mut Interner, font: Font) {
        self.content.push(FontTableContent::Font(font));
        self.empty = false;
        if self.declared_count(interner).ok().flatten().is_some() {
            let count = u32::try_from(self.len()).unwrap_or(u32::MAX);
            self.set_declared_count(interner, Some(count));
        }
    }
}
