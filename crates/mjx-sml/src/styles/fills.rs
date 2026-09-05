//! `x:fills` and the two kinds of fill (`CT_Fills` at `sml.xsd:3483`, `CT_Fill` at `3489`,
//! `CT_PatternFill` at `3495`, `CT_GradientFill` at `3532`, `CT_GradientStop` at `3542`).
//!
//! # A fill is a *choice*, and the choice is the whole type
//!
//! `CT_Fill` declares nothing but an `xsd:choice` of `patternFill` and `gradientFill`, with no
//! attributes at all. So [`Fill`] is a wrapper whose only job is to say which of the two the file
//! wrote, and [`Fill::set_pattern`] removes a `gradientFill` where one stood: the two share a rank
//! in `CT_Fill`'s content model, which is what an `xsd:choice` means.
//!
//! # Two colours, and the one that surprises
//!
//! A pattern fill has a **foreground** and a **background** colour, and for `patternType="solid"` —
//! the overwhelmingly common case — the colour a user sees is the **foreground** one. Excel writes
//! a solid yellow cell as `<patternFill patternType="solid"><fgColor rgb="FFFFFF00"/>
//! <bgColor indexed="64"/></patternFill>`; ECMA-376 Part 1 §18.8.21 prints that exact example. A
//! reader that took `bgColor` for "the fill colour" would report the *system background* for every
//! solid fill in every workbook.
//!
//! Both are held as [`ColorElement`], the one `CT_Color` element type, and decoded through
//! [`Color`] on demand.
//!
//! # Index identity
//!
//! `CT_Fill` is addressed by position exactly as `CT_Font` is — an `xf` says `fillId="3"` — so
//! [`FillTable`] offers the same three operations and no more. See [`super::fonts`] for the whole
//! rule.

use mjx_ooxml_core::{Enumeration, Interner, Number, RawAttribute, RawName, RawNode};
use mjx_ooxml_types::child_order::STYLESHEET_PATTERN_FILL;
use mjx_ooxml_types::spreadsheetml::{GradientType, PatternType};

use crate::font::{Color, ColorElement};

/// `x:fills` (`CT_Fills`, `sml.xsd:3483`) — the fill table, in index order.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count))]
pub struct FillTable {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "fill", variant = Fill, ty = Fill))]
    content: Vec<FillTableContent>,
}

/// One child of [`FillTable`]: a fill, or markup this type does not model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FillTableContent {
    /// `x:fill`.
    Fill(Fill),
    /// Anything else — preserved verbatim, in position, and occupying no index.
    Raw(RawNode),
}

impl FillTable {
    /// Builds an empty `x:fills`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "fills"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[FillTableContent] {
        &self.content
    }

    /// Every `x:fill`, in index order.
    pub fn fills(&self) -> impl Iterator<Item = &Fill> + '_ {
        self.content.iter().filter_map(|item| match item {
            FillTableContent::Fill(fill) => Some(fill),
            FillTableContent::Raw(_) => None,
        })
    }

    /// The fill at `index` — the number an `xf`'s `@fillId` carries.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Fill> {
        self.fills().nth(index)
    }

    /// How many fills the table holds — counted, not read from `@count`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fills().count()
    }

    /// Whether the table holds no fill at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends `fill` after the last entry, giving it the next index, and updates `@count` when the
    /// file declared one. The only mutation — see [`super::fonts`].
    pub fn push(&mut self, interner: &mut Interner, fill: Fill) {
        self.content.push(FillTableContent::Fill(fill));
        self.empty = false;
        if self.declared_count(interner).ok().flatten().is_some() {
            let count = u32::try_from(self.len()).unwrap_or(u32::MAX);
            self.set_declared_count(interner, Some(count));
        }
    }
}

/// `x:fill` (`CT_Fill`, `sml.xsd:3489`) — one entry of the fill table: a pattern **or** a gradient.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = SML)]
pub struct Fill {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "patternFill", variant = Pattern, ty = PatternFill),
        child(local = "gradientFill", variant = Gradient, ty = GradientFill)
    )]
    content: Vec<FillContent>,
}

/// One child of [`Fill`]: the two alternatives of its `xsd:choice`, or markup it does not model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FillContent {
    /// `x:patternFill`.
    Pattern(PatternFill),
    /// `x:gradientFill`.
    Gradient(GradientFill),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl Fill {
    /// Builds an empty `x:fill`, bound to `prefix` or to the default namespace.
    ///
    /// The schema declares the choice `minOccurs="1"`, so a fill with neither alternative is
    /// invalid; this exists so a caller can build one and then set the alternative it wants.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "fill"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[FillContent] {
        &self.content
    }

    /// `x:patternFill` — `None` if this fill is a gradient, or writes neither.
    #[must_use]
    pub fn pattern(&self) -> Option<&PatternFill> {
        self.content.iter().find_map(|item| match item {
            FillContent::Pattern(fill) => Some(fill),
            _ => None,
        })
    }

    /// `x:gradientFill` — `None` if this fill is a pattern, or writes neither.
    #[must_use]
    pub fn gradient(&self) -> Option<&GradientFill> {
        self.content.iter().find_map(|item| match item {
            FillContent::Gradient(fill) => Some(fill),
            _ => None,
        })
    }

    /// Makes this a pattern fill, **replacing** a gradient where one stood.
    ///
    /// The two alternatives share one rank in `CT_Fill`'s `xsd:choice`, so a file that wrote both
    /// would be invalid; setting one is therefore choosing, not adding.
    pub fn set_pattern(&mut self, fill: PatternFill) {
        self.replace_choice(FillContent::Pattern(fill));
    }

    /// Makes this a gradient fill, **replacing** a pattern where one stood.
    pub fn set_gradient(&mut self, fill: GradientFill) {
        self.replace_choice(FillContent::Gradient(fill));
    }

    /// Puts `value` where the existing alternative stood, or at the end when there is none.
    fn replace_choice(&mut self, value: FillContent) {
        let existing = self
            .content
            .iter()
            .position(|item| matches!(item, FillContent::Pattern(_) | FillContent::Gradient(_)));
        match existing {
            Some(at) => self.content[at] = value,
            None => self.content.push(value),
        }
        self.empty = false;
    }
}

/// `x:patternFill` (`CT_PatternFill`, `sml.xsd:3495`) — a pattern, and the two colours it is drawn
/// in.
///
/// `@patternType` is **optional and has no schema default**: `<patternFill/>` states a fill whose
/// pattern the file does not say, which is a third state beside `none` and `solid` and is exactly
/// what a `dxf` writes when it means *"the background colour changes, the pattern is inherited"*.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "patternType", codec = Enumeration<PatternType>, accessor = pattern_type))]
pub struct PatternFill {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "fgColor", variant = Foreground, ty = ColorElement),
        child(local = "bgColor", variant = Background, ty = ColorElement)
    )]
    content: Vec<PatternFillContent>,
}

/// One child of [`PatternFill`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternFillContent {
    /// `x:fgColor` (rank 0).
    Foreground(ColorElement),
    /// `x:bgColor` (rank 1).
    Background(ColorElement),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl PatternFillContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::Foreground(_) => "fgColor",
            Self::Background(_) => "bgColor",
            Self::Raw(_) => return None,
        })
    }

    /// This child's rank in `CT_PatternFill`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        STYLESHEET_PATTERN_FILL.rank_of(None, self.local()?)
    }
}

impl PatternFill {
    /// Builds an empty `x:patternFill`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "patternFill"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[PatternFillContent] {
        &self.content
    }

    /// `x:fgColor` as the element the file wrote — the colour a **solid** fill actually shows.
    #[must_use]
    pub fn foreground_color_element(&self) -> Option<&ColorElement> {
        self.content.iter().find_map(|item| match item {
            PatternFillContent::Foreground(color) => Some(color),
            _ => None,
        })
    }

    /// `x:bgColor` as the element the file wrote.
    #[must_use]
    pub fn background_color_element(&self) -> Option<&ColorElement> {
        self.content.iter().find_map(|item| match item {
            PatternFillContent::Background(color) => Some(color),
            _ => None,
        })
    }

    /// The foreground colour, decoded — `None` if the fill writes no `fgColor`.
    #[must_use]
    pub fn foreground_colour(&self, interner: &Interner) -> Option<Color> {
        self.foreground_color_element()
            .map(|element| element.color(interner))
    }

    /// The background colour, decoded — `None` if the fill writes no `bgColor`.
    #[must_use]
    pub fn background_colour(&self, interner: &Interner) -> Option<Color> {
        self.background_color_element()
            .map(|element| element.color(interner))
    }

    /// Sets `x:fgColor`: `None` removes it; `Some` replaces the existing element where it is, or
    /// inserts one at its rank in `CT_PatternFill`'s sequence.
    pub fn set_foreground_color(&mut self, color: Option<ColorElement>) {
        self.replace_or_insert(
            "fgColor",
            |item| matches!(item, PatternFillContent::Foreground(_)),
            color.map(PatternFillContent::Foreground),
        );
    }

    /// Sets `x:bgColor`, as [`set_foreground_color`](Self::set_foreground_color).
    pub fn set_background_color(&mut self, color: Option<ColorElement>) {
        self.replace_or_insert(
            "bgColor",
            |item| matches!(item, PatternFillContent::Background(_)),
            color.map(PatternFillContent::Background),
        );
    }

    /// Replaces the first child `is_target` accepts, keeping its position; inserts at the schema
    /// rank when there is none; removes it when `value` is `None`.
    fn replace_or_insert(
        &mut self,
        local: &str,
        is_target: impl Fn(&PatternFillContent) -> bool,
        value: Option<PatternFillContent>,
    ) {
        let existing = self.content.iter().position(&is_target);
        match (existing, value) {
            (Some(at), Some(value)) => self.content[at] = value,
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(value)) => {
                let at = STYLESHEET_PATTERN_FILL.insert_index_of_names(
                    self.content.iter().map(PatternFillContent::rank),
                    local,
                );
                self.content.insert(at, value);
                self.empty = false;
            }
            (None, None) => {}
        }
    }
}

/// `x:gradientFill` (`CT_GradientFill`, `sml.xsd:3532`) — a gradient and its stops.
///
/// The six attributes mean two different things depending on [`gradient_type`](Self::gradient_type):
/// for `linear` the gradient runs at `@degree` and the four inset attributes are unused; for `path`
/// the four describe the inner rectangle the gradient converges on, as fractions of the cell.
/// Nothing here converts between them; each is the number the file wrote.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "type", codec = Enumeration<GradientType>, accessor = gradient_type, default = GradientType::Linear))]
#[xml(attribute(local = "degree", codec = Number<f64>, accessor = degrees, default = 0.0))]
#[xml(attribute(local = "left", codec = Number<f64>, accessor = left_inset, default = 0.0))]
#[xml(attribute(local = "right", codec = Number<f64>, accessor = right_inset, default = 0.0))]
#[xml(attribute(local = "top", codec = Number<f64>, accessor = top_inset, default = 0.0))]
#[xml(attribute(local = "bottom", codec = Number<f64>, accessor = bottom_inset, default = 0.0))]
pub struct GradientFill {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "stop", variant = Stop, ty = GradientStop))]
    content: Vec<GradientFillContent>,
}

/// One child of [`GradientFill`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GradientFillContent {
    /// `x:stop`.
    Stop(GradientStop),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl GradientFill {
    /// Builds an empty `x:gradientFill`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "gradientFill"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[GradientFillContent] {
        &self.content
    }

    /// Every `x:stop`, in the order the file wrote them.
    ///
    /// The order is the file's, not sorted by `@position`: `CT_GradientFill`'s content model is a
    /// repeated element and re-ordering the stops would change the bytes without changing the
    /// gradient.
    pub fn stops(&self) -> impl Iterator<Item = &GradientStop> + '_ {
        self.content.iter().filter_map(|item| match item {
            GradientFillContent::Stop(stop) => Some(stop),
            GradientFillContent::Raw(_) => None,
        })
    }

    /// Appends a stop after the last one already present.
    pub fn push_stop(&mut self, stop: GradientStop) {
        self.content.push(GradientFillContent::Stop(stop));
        self.empty = false;
    }
}

/// `x:stop` (`CT_GradientStop`, `sml.xsd:3542`) — one stop of a gradient: a position and a colour.
///
/// `@position` is `use="required"`, and is still read as `Option` here: a file that omits it is
/// reported as it stands rather than refused, for the reason
/// [`SheetEntry`](crate::SheetEntry) gives for its own four required attributes.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "position", codec = Number<f64>, accessor = position))]
pub struct GradientStop {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "color", variant = Color, ty = ColorElement))]
    content: Vec<GradientStopContent>,
}

/// One child of [`GradientStop`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GradientStopContent {
    /// `x:color` — the schema declares it `minOccurs="1"`.
    Color(ColorElement),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl GradientStop {
    /// Builds an `x:stop` at `position` in `color`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        prefix: Option<&str>,
        position: f64,
        color: &Color,
    ) -> Self {
        let color = ColorElement::named(interner, prefix, "color", color);
        let mut stop = Self {
            name: crate::leaf::sml_name(interner, prefix, "stop"),
            attributes: Vec::new(),
            empty: false,
            content: vec![GradientStopContent::Color(color)],
        };
        stop.set_position(interner, Some(position));
        stop
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[GradientStopContent] {
        &self.content
    }

    /// `x:color` as the element the file wrote.
    #[must_use]
    pub fn color_element(&self) -> Option<&ColorElement> {
        self.content.iter().find_map(|item| match item {
            GradientStopContent::Color(color) => Some(color),
            GradientStopContent::Raw(_) => None,
        })
    }

    /// The stop's colour, decoded — `None` if it writes none, which the schema forbids.
    #[must_use]
    pub fn colour(&self, interner: &Interner) -> Option<Color> {
        self.color_element().map(|element| element.color(interner))
    }
}
