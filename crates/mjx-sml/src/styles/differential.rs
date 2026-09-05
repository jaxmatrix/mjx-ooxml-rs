//! `x:dxfs` / `x:dxf` (`CT_Dxfs` at `sml.xsd:3635`, `CT_Dxf` at `3641`) — the **differential**
//! formats.
//!
//! # Absent means *inherit*, and that is the whole type
//!
//! A `dxf` is not a format. It is a **delta** applied on top of whatever the cell already has, and
//! all seven of its children are `minOccurs="0"`. So an absent `font` means *"the font is whatever
//! the cell's own format says"* — never *"the default font"* — and the two have to be
//! distinguishable, because a conditional-formatting rule that turned every matched cell's font
//! back to Calibri 11 when it only meant to colour the background would repaint the sheet.
//!
//! `Option` is therefore load-bearing on every accessor here, and
//! `tests/fixtures/style_resources.xlsx` writes four `dxf` entries specifically to pin it: one with
//! only a fill, one with only a font, one with all six members set, and one written `<dxf/>` — which
//! means *"inherit everything"* and is a legal, meaningful entry rather than an empty one.
//!
//! # Its consumers arrive later, and it is built now anyway
//!
//! Nothing in this workspace references a `dxf` yet. Conditional formatting (MJXOFF-120) and table
//! styles (MJXOFF-127) both address one **by index into `dxfs`**, exactly as an `xf` addresses a
//! font — so the table is a resource table like the other three, it belongs with them, and building
//! it here is what lets those children be about their own subject rather than about `styles.xml`.
//!
//! # Three of the six members are not declared here
//!
//! `alignment`, `protection` and `numFmt` are shared with `CT_Xf`, whose child is MJXOFF-108. They
//! live in [`super::cell_format`], a module that belongs to neither, for the reason that module's
//! own documentation gives.

use mjx_ooxml_core::{Interner, Number, RawAttribute, RawName, RawNode};
use mjx_ooxml_types::child_order::STYLESHEET_DIFFERENTIAL_FORMAT;

use super::borders::Border;
use super::cell_format::{CellAlignment, CellProtection, NumberFormat};
use super::fills::Fill;
use super::fonts::Font;

/// `x:dxfs` (`CT_Dxfs`, `sml.xsd:3635`) — the differential-format table, in index order.
///
/// Addressed by `@dxfId`, so the index rules of [`super::fonts`] apply here exactly: append, never
/// reorder.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count))]
pub struct DifferentialFormats {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "dxf", variant = Format, ty = DifferentialFormat))]
    content: Vec<DifferentialFormatsContent>,
}

/// One child of [`DifferentialFormats`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DifferentialFormatsContent {
    /// `x:dxf`.
    Format(DifferentialFormat),
    /// Anything else — preserved verbatim, in position, and occupying no index.
    Raw(RawNode),
}

impl DifferentialFormats {
    /// Builds an empty `x:dxfs`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "dxfs"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[DifferentialFormatsContent] {
        &self.content
    }

    /// Every `x:dxf`, in index order.
    pub fn formats(&self) -> impl Iterator<Item = &DifferentialFormat> + '_ {
        self.content.iter().filter_map(|item| match item {
            DifferentialFormatsContent::Format(format) => Some(format),
            DifferentialFormatsContent::Raw(_) => None,
        })
    }

    /// The format at `index` — the number a `@dxfId` carries.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&DifferentialFormat> {
        self.formats().nth(index)
    }

    /// How many formats the table holds — counted, not read from `@count`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.formats().count()
    }

    /// Whether the table holds no format at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends `format` after the last entry, giving it the next index, and updates `@count` when
    /// the file declared one. The only mutation — see [`super::fonts`].
    pub fn push(&mut self, interner: &mut Interner, format: DifferentialFormat) {
        self.content
            .push(DifferentialFormatsContent::Format(format));
        self.empty = false;
        if self.declared_count(interner).ok().flatten().is_some() {
            let count = u32::try_from(self.len()).unwrap_or(u32::MAX);
            self.set_declared_count(interner, Some(count));
        }
    }
}

/// `x:dxf` (`CT_Dxf`, `sml.xsd:3641`) — one differential format: up to six members, each of which
/// is absent when it is inherited.
///
/// The type carries **no attributes at all**, which is worth saying out loud: a `dxf` is entirely
/// its children, and an empty one is a meaningful value.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = SML)]
pub struct DifferentialFormat {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "font", variant = Font, ty = Font),
        child(local = "numFmt", variant = NumberFormat, ty = NumberFormat),
        child(local = "fill", variant = Fill, ty = Fill),
        child(local = "alignment", variant = Alignment, ty = CellAlignment),
        child(local = "border", variant = Border, ty = Border),
        child(local = "protection", variant = Protection, ty = CellProtection)
    )]
    content: Vec<DifferentialFormatContent>,
}

/// One child of [`DifferentialFormat`]: six modelled members, and `extLst`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DifferentialFormatContent {
    /// `x:font` (rank 0).
    Font(Font),
    /// `x:numFmt` (rank 1).
    NumberFormat(NumberFormat),
    /// `x:fill` (rank 2).
    Fill(Fill),
    /// `x:alignment` (rank 3).
    Alignment(CellAlignment),
    /// `x:border` (rank 4).
    Border(Border),
    /// `x:protection` (rank 5).
    Protection(CellProtection),
    /// `x:extLst` (rank 6) and anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl DifferentialFormatContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::Font(_) => "font",
            Self::NumberFormat(_) => "numFmt",
            Self::Fill(_) => "fill",
            Self::Alignment(_) => "alignment",
            Self::Border(_) => "border",
            Self::Protection(_) => "protection",
            Self::Raw(_) => return None,
        })
    }

    /// This child's rank in `CT_Dxf`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        STYLESHEET_DIFFERENTIAL_FORMAT.rank_of(None, self.local()?)
    }
}

/// Declares one member: a borrowing getter whose `None` means *inherited*, and a setter that
/// replaces the existing member in place or inserts a new one at its rank in `CT_Dxf`'s sequence.
macro_rules! member {
    ($getter:ident, $setter:ident, $variant:ident, $ty:ty, $local:literal, $doc:literal) => {
        #[doc = $doc]
        ///
        /// `None` means the member is **inherited**, not that it takes a default value.
        #[must_use]
        pub fn $getter(&self) -> Option<&$ty> {
            self.content.iter().find_map(|item| match item {
                DifferentialFormatContent::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("Sets `x:", $local, "`: `None` makes the member inherited again; `Some` \
            replaces the existing element **where it is**, or inserts one at its rank in `CT_Dxf`'s \
            `xsd:sequence`.")]
        pub fn $setter(&mut self, value: Option<$ty>) {
            self.replace_or_insert(
                $local,
                |item| matches!(item, DifferentialFormatContent::$variant(_)),
                value.map(DifferentialFormatContent::$variant),
            );
        }
    };
}

impl DifferentialFormat {
    /// Builds an `x:dxf` that inherits everything, bound to `prefix` or to the default namespace.
    ///
    /// That is a meaningful value, not a placeholder: `<dxf/>` is what a table style writes for an
    /// element it wants left alone.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "dxf"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including `extLst` and anything else unmodelled.
    #[must_use]
    pub fn content(&self) -> &[DifferentialFormatContent] {
        &self.content
    }

    /// Whether this format states nothing at all — every member inherited.
    ///
    /// True for `<dxf/>`, and false for a `dxf` that carries only an `extLst`: something is being
    /// said there, even if this type does not model what.
    #[must_use]
    pub fn inherits_everything(&self) -> bool {
        self.content
            .iter()
            .all(|item| matches!(item, DifferentialFormatContent::Raw(RawNode::Text(_))))
    }

    member!(
        font,
        set_font,
        Font,
        Font,
        "font",
        "`x:font` — the font properties this format overrides."
    );
    member!(
        number_format,
        set_number_format,
        NumberFormat,
        NumberFormat,
        "numFmt",
        "`x:numFmt` — the number format this format overrides."
    );
    member!(
        fill,
        set_fill,
        Fill,
        Fill,
        "fill",
        "`x:fill` — the fill this format overrides. In a conditional format this is the member that \
         is set on its own most often."
    );
    member!(
        alignment,
        set_alignment,
        Alignment,
        CellAlignment,
        "alignment",
        "`x:alignment` — the alignment this format overrides."
    );
    member!(
        border,
        set_border,
        Border,
        Border,
        "border",
        "`x:border` — the border this format overrides."
    );
    member!(
        protection,
        set_protection,
        Protection,
        CellProtection,
        "protection",
        "`x:protection` — the locked and formula-hidden flags this format overrides."
    );

    /// Replaces the first child `is_target` accepts, keeping its position; inserts at the schema
    /// rank when there is none; removes it when `value` is `None`.
    fn replace_or_insert(
        &mut self,
        local: &str,
        is_target: impl Fn(&DifferentialFormatContent) -> bool,
        value: Option<DifferentialFormatContent>,
    ) {
        let existing = self.content.iter().position(&is_target);
        match (existing, value) {
            (Some(at), Some(value)) => self.content[at] = value,
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(value)) => {
                let at = STYLESHEET_DIFFERENTIAL_FORMAT.insert_index_of_names(
                    self.content.iter().map(DifferentialFormatContent::rank),
                    local,
                );
                self.content.insert(at, value);
                self.empty = false;
            }
            (None, None) => {}
        }
    }
}
