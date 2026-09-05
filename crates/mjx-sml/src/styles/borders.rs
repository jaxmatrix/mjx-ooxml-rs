//! `x:borders` and the edges of a border (`CT_Borders` at `sml.xsd:3445`, `CT_Border` at `3451`,
//! `CT_BorderPr` at `3468`).
//!
//! # Nine edges, not seven
//!
//! `CT_Border`'s `xsd:sequence` declares **nine** children, in this order:
//!
//! | rank | element | this crate's name |
//! |---|---|---|
//! | 0 | `start` | [`Border::leading_edge`] |
//! | 1 | `end` | [`Border::trailing_edge`] |
//! | 2 | `left` | [`Border::left_edge`] |
//! | 3 | `right` | [`Border::right_edge`] |
//! | 4 | `top` | [`Border::top_edge`] |
//! | 5 | `bottom` | [`Border::bottom_edge`] |
//! | 6 | `diagonal` | [`Border::diagonal_edge`] |
//! | 7 | `vertical` | [`Border::vertical_inner_edge`] |
//! | 8 | `horizontal` | [`Border::horizontal_inner_edge`] |
//!
//! ECMA-376 Part 1 §18.8.4's prose enumerates only five of them — *"left, right, top, bottom,
//! diagonal"* — and §18.8 carries **no entry at all** for `start` or `end`. The two are documented
//! in WordprocessingML instead, as *"Leading Edge Border"* (§17.4.33) and *"Trailing Edge Border"*
//! (§17.4.12): the reading-direction-relative pair that `left`/`right` are the physical form of.
//! This crate names them the way `mjx_docx::Indentation` already names the same pair, rather than
//! inventing a second spelling for one concept. `vertical` and `horizontal` take their names from
//! their own §18.8.44 and §18.8.25 titles — *"Vertical Inner Border"* and *"Horizontal Inner
//! Borders"* — and Part 1 says both are *"used in the context of `dxf` elements only"*, which is why
//! [`super::differential`] is the child that needs them.
//!
//! A model that stopped at seven would drop two edges of every table style in every workbook that
//! has one. The order above is never written down here: every placement goes through
//! [`STYLESHEET_BORDER`], generated from `sml.xsd`.
//!
//! # `@style` is a value, and its absence is a different value
//!
//! `CT_BorderPr`'s `@style` carries the schema default `none`, so `<top/>` is *"no line on the top
//! edge"* — a statement, not a silence. [`BorderEdge::style`] therefore answers
//! [`BorderStyle::None`] for an edge that writes no attribute, which is what the schema says and
//! what `tests/fixtures/style_resources.xlsx`'s first border relies on.

use mjx_ooxml_core::{Enumeration, Interner, Number, RawAttribute, RawName, RawNode};
use mjx_ooxml_types::child_order::STYLESHEET_BORDER;
use mjx_ooxml_types::spreadsheetml::BorderStyle;
use mjx_ooxml_types::support::OnOff;

use crate::font::{Color, ColorElement};

/// `x:borders` (`CT_Borders`, `sml.xsd:3445`) — the border table, in index order.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count))]
pub struct BorderTable {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "border", variant = Border, ty = Border))]
    content: Vec<BorderTableContent>,
}

/// One child of [`BorderTable`]: a border, or markup this type does not model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BorderTableContent {
    /// `x:border`.
    Border(Border),
    /// Anything else — preserved verbatim, in position, and occupying no index.
    Raw(RawNode),
}

impl BorderTable {
    /// Builds an empty `x:borders`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "borders"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[BorderTableContent] {
        &self.content
    }

    /// Every `x:border`, in index order.
    pub fn borders(&self) -> impl Iterator<Item = &Border> + '_ {
        self.content.iter().filter_map(|item| match item {
            BorderTableContent::Border(border) => Some(border),
            BorderTableContent::Raw(_) => None,
        })
    }

    /// The border at `index` — the number an `xf`'s `@borderId` carries.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Border> {
        self.borders().nth(index)
    }

    /// How many borders the table holds — counted, not read from `@count`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.borders().count()
    }

    /// Whether the table holds no border at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends `border` after the last entry, giving it the next index, and updates `@count` when
    /// the file declared one. The only mutation — see [`super::fonts`].
    pub fn push(&mut self, interner: &mut Interner, border: Border) {
        self.content.push(BorderTableContent::Border(border));
        self.empty = false;
        if self.declared_count(interner).ok().flatten().is_some() {
            let count = u32::try_from(self.len()).unwrap_or(u32::MAX);
            self.set_declared_count(interner, Some(count));
        }
    }
}

/// `x:border` (`CT_Border`, `sml.xsd:3451`) — one entry of the border table: nine edges and three
/// flags.
///
/// `@outline` carries the schema default **`true`**, which is the opposite of what a reader who
/// assumes "absent means off" would guess.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "diagonalUp", codec = OnOff, accessor = diagonal_up))]
#[xml(attribute(local = "diagonalDown", codec = OnOff, accessor = diagonal_down))]
#[xml(attribute(local = "outline", codec = OnOff, accessor = outline_only, default = true))]
pub struct Border {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "start", variant = Leading, ty = BorderEdge),
        child(local = "end", variant = Trailing, ty = BorderEdge),
        child(local = "left", variant = Left, ty = BorderEdge),
        child(local = "right", variant = Right, ty = BorderEdge),
        child(local = "top", variant = Top, ty = BorderEdge),
        child(local = "bottom", variant = Bottom, ty = BorderEdge),
        child(local = "diagonal", variant = Diagonal, ty = BorderEdge),
        child(local = "vertical", variant = VerticalInner, ty = BorderEdge),
        child(local = "horizontal", variant = HorizontalInner, ty = BorderEdge)
    )]
    content: Vec<BorderContent>,
}

/// One child of [`Border`]: nine edges, and everything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BorderContent {
    /// `x:start` (rank 0) — the leading edge in the reading direction.
    Leading(BorderEdge),
    /// `x:end` (rank 1) — the trailing edge in the reading direction.
    Trailing(BorderEdge),
    /// `x:left` (rank 2).
    Left(BorderEdge),
    /// `x:right` (rank 3).
    Right(BorderEdge),
    /// `x:top` (rank 4).
    Top(BorderEdge),
    /// `x:bottom` (rank 5).
    Bottom(BorderEdge),
    /// `x:diagonal` (rank 6) — drawn where `@diagonalUp` or `@diagonalDown` says so.
    Diagonal(BorderEdge),
    /// `x:vertical` (rank 7) — the inner vertical border of a range; `dxf` only.
    VerticalInner(BorderEdge),
    /// `x:horizontal` (rank 8) — the inner horizontal border of a range; `dxf` only.
    HorizontalInner(BorderEdge),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl BorderContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::Leading(_) => "start",
            Self::Trailing(_) => "end",
            Self::Left(_) => "left",
            Self::Right(_) => "right",
            Self::Top(_) => "top",
            Self::Bottom(_) => "bottom",
            Self::Diagonal(_) => "diagonal",
            Self::VerticalInner(_) => "vertical",
            Self::HorizontalInner(_) => "horizontal",
            Self::Raw(_) => return None,
        })
    }

    /// This child's rank in `CT_Border`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        STYLESHEET_BORDER.rank_of(None, self.local()?)
    }
}

/// Declares one edge: a borrowing getter and a setter that replaces the existing edge in place or
/// inserts a new one at its rank in `CT_Border`'s sequence.
///
/// Nine edges share these two bodies, and writing them out nine times would be nine chances to
/// reach for the wrong variant — which is the defect a nine-slot sequence is most likely to hide.
macro_rules! edge {
    ($getter:ident, $setter:ident, $variant:ident, $local:literal, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<&BorderEdge> {
            self.content.iter().find_map(|item| match item {
                BorderContent::$variant(edge) => Some(edge),
                _ => None,
            })
        }

        #[doc = concat!("Sets `x:", $local, "`: `None` removes it; `Some` replaces the existing \
            element **where it is**, or inserts one at its rank in `CT_Border`'s `xsd:sequence`.")]
        pub fn $setter(&mut self, edge: Option<BorderEdge>) {
            self.replace_or_insert(
                $local,
                |item| matches!(item, BorderContent::$variant(_)),
                edge.map(BorderContent::$variant),
            );
        }
    };
}

impl Border {
    /// Builds an empty `x:border`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "border"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[BorderContent] {
        &self.content
    }

    edge!(
        leading_edge,
        set_leading_edge,
        Leading,
        "start",
        "`x:start` — the **leading** edge in the reading direction, which is the left one in a \
         left-to-right sheet. Part 1 documents it in WordprocessingML (§17.4.33, *Leading Edge \
         Border*); §18.8 has no entry for it."
    );
    edge!(
        trailing_edge,
        set_trailing_edge,
        Trailing,
        "end",
        "`x:end` — the **trailing** edge in the reading direction. Part 1 documents it in \
         WordprocessingML (§17.4.12, *Trailing Edge Border*); §18.8 has no entry for it."
    );
    edge!(
        left_edge,
        set_left_edge,
        Left,
        "left",
        "`x:left` — the physical left edge (§18.8.26)."
    );
    edge!(
        right_edge,
        set_right_edge,
        Right,
        "right",
        "`x:right` — the physical right edge (§18.8.34)."
    );
    edge!(
        top_edge,
        set_top_edge,
        Top,
        "top",
        "`x:top` — the top edge (§18.8.43)."
    );
    edge!(
        bottom_edge,
        set_bottom_edge,
        Bottom,
        "bottom",
        "`x:bottom` — the bottom edge (§18.8.6)."
    );
    edge!(
        diagonal_edge,
        set_diagonal_edge,
        Diagonal,
        "diagonal",
        "`x:diagonal` — the diagonal line's style and colour. **Which diagonals are drawn** is \
         [`diagonal_up`](Self::diagonal_up) and [`diagonal_down`](Self::diagonal_down), not this: an \
         edge with a style and neither flag set draws nothing."
    );
    edge!(
        vertical_inner_edge,
        set_vertical_inner_edge,
        VerticalInner,
        "vertical",
        "`x:vertical` — the inner vertical border of a *range* (§18.8.44, *Vertical Inner Border*). \
         Part 1: used in the context of `dxf` elements only."
    );
    edge!(
        horizontal_inner_edge,
        set_horizontal_inner_edge,
        HorizontalInner,
        "horizontal",
        "`x:horizontal` — the inner horizontal border of a *range* (§18.8.25, *Horizontal Inner \
         Borders*). Part 1: used in the context of `dxf` elements only."
    );

    /// Replaces the first child `is_target` accepts, keeping its position; inserts at the schema
    /// rank when there is none; removes it when `value` is `None`.
    fn replace_or_insert(
        &mut self,
        local: &str,
        is_target: impl Fn(&BorderContent) -> bool,
        value: Option<BorderContent>,
    ) {
        let existing = self.content.iter().position(&is_target);
        match (existing, value) {
            (Some(at), Some(value)) => self.content[at] = value,
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(value)) => {
                let at = STYLESHEET_BORDER
                    .insert_index_of_names(self.content.iter().map(BorderContent::rank), local);
                self.content.insert(at, value);
                self.empty = false;
            }
            (None, None) => {}
        }
    }
}

/// `x:start` / `x:end` / `x:left` / … (`CT_BorderPr`, `sml.xsd:3468`) — one edge of a border: a line
/// style and, optionally, a colour.
///
/// The element does not name itself; the slot it stands in does. So this is one type for all nine
/// edges, exactly as [`ColorElement`] is one type for all five `CT_Color` slots, and
/// [`named`](Self::named) takes the local name.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "style", codec = Enumeration<BorderStyle>, accessor = style, default = BorderStyle::None))]
pub struct BorderEdge {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "color", variant = Color, ty = ColorElement))]
    content: Vec<BorderEdgeContent>,
}

/// One child of [`BorderEdge`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BorderEdgeContent {
    /// `x:color` — absent means *automatic*, which Part 1 §18.8.4 states explicitly.
    Color(ColorElement),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl BorderEdge {
    /// Builds an edge named `local` in `style`, bound to `prefix` or to the default namespace.
    ///
    /// `local` is the slot's element name — `start`, `end`, `left`, `right`, `top`, `bottom`,
    /// `diagonal`, `vertical` or `horizontal`.
    #[must_use]
    pub fn named(
        interner: &mut Interner,
        prefix: Option<&str>,
        local: &str,
        style: BorderStyle,
    ) -> Self {
        let mut edge = Self {
            name: crate::leaf::sml_name(interner, prefix, local),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        edge.set_style(interner, Some(style));
        edge
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[BorderEdgeContent] {
        &self.content
    }

    /// `x:color` as the element the file wrote — `None` means *automatic*.
    #[must_use]
    pub fn color_element(&self) -> Option<&ColorElement> {
        self.content.iter().find_map(|item| match item {
            BorderEdgeContent::Color(color) => Some(color),
            BorderEdgeContent::Raw(_) => None,
        })
    }

    /// The edge's colour, decoded — `None` means the edge writes none, which is *automatic*.
    #[must_use]
    pub fn colour(&self, interner: &Interner) -> Option<Color> {
        self.color_element().map(|element| element.color(interner))
    }

    /// Sets `x:color`: `None` removes it, which is how an edge says *automatic*.
    pub fn set_color(&mut self, color: Option<ColorElement>) {
        let existing = self
            .content
            .iter()
            .position(|item| matches!(item, BorderEdgeContent::Color(_)));
        match (existing, color) {
            (Some(at), Some(color)) => self.content[at] = BorderEdgeContent::Color(color),
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(color)) => {
                // `CT_BorderPr` declares one child and no other, so there is no rank to consult.
                self.content.push(BorderEdgeContent::Color(color));
                self.empty = false;
            }
            (None, None) => {}
        }
    }
}
