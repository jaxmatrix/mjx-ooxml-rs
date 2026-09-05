//! The three graded conditional formats and the threshold vocabulary they share: `CT_Cfvo`,
//! `CT_ColorScale`, `CT_DataBar` and `CT_IconSet`.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_ColorScale` | 2769 | `x:cfRule/colorScale` |
//! | `CT_DataBar` | 2775 | `x:cfRule/dataBar` |
//! | `CT_IconSet` | 2784 | `x:cfRule/iconSet` |
//! | `CT_Cfvo` | 2793 | `…/cfvo` |
//!
//! # A threshold is a value object, and its `@type` says how to read `@val`
//!
//! ECMA-376 Part 1 §18.3.1.11 calls `cfvo` a *"Conditional Format Value Object"* and describes it as
//! *"the values of the interpolation points in a gradient scale"*. `@val` is an `ST_Xstring` — text,
//! whatever the cell's own type is — and `@type` says what the text means: a number, a percentage, a
//! percentile, a formula, or the range's own minimum or maximum. [`ConditionalValueObject`]
//! therefore reports [`value_kind`](ConditionalValueObject::value_kind) and
//! [`value`](ConditionalValueObject::value) separately, and decodes neither into a number: `min` and
//! `max` carry a `@val` that Excel writes and ignores, and a `formula` carries an expression this
//! library never evaluates.
//!
//! # The three cardinalities are different, and each is the schema's
//!
//! * a colour scale takes **2 or more** `cfvo` and **2 or more** `color`, paired by position —
//!   §18.3.1.11: *"The first `<cfvo>` element corresponds with the first `<color>` definition, and
//!   so on."*
//! * a data bar takes **exactly 2** `cfvo` and **exactly 1** `color`;
//! * an icon set takes **2 or more** `cfvo` and no colour at all.
//!
//! None of the three is enforced on read. A file that writes three colours beside two value objects
//! is a file this library reports as it stands, exactly as `crate::worksheet` reports an overlapping
//! merge rather than repairing it. What the accessors do instead is refuse to *pretend*:
//! [`ColorScale::pairs`] stops at the shorter of the two lists and
//! [`ColorScale::is_balanced`] says whether it had to.
//!
//! # The colours are `CT_Color`, which is not DrawingML's
//!
//! Every colour here is [`ColorElement`] — SpreadsheetML's one-element-five-attributes colour, with
//! `@indexed`, `@theme` (an `xsd:unsignedInt` *position*) and `@tint`, none of which
//! [`mjx_dml::ColorSpec`] can spell. The reasoning is written out once, at
//! [`crate::font::color`]. Resolving a `@theme` position to a scheme slot is
//! [`crate::styles::palette`]'s, and *that* half is shared with DrawingML.
//!
//! # `@iconSet`'s default is `3TrafficLights1`, and nothing here spells it by hand
//!
//! The schema declares `default="3TrafficLights1"` — a digit-leading token that no Rust identifier
//! may begin with. `mjx-ooxml-types`' generator sanitised it deterministically into
//! [`IconSetType::ThreeTrafficLights`], with the exact wire token in that variant's own
//! documentation, and this module names the generated variant rather than writing a table of its
//! own.

use mjx_ooxml_core::{
    Enumeration, Interner, Number, RawAttribute, RawElement, RawName, RawNode, Text, ToXml,
};
use mjx_ooxml_types::spreadsheetml::{ConditionalFormatValueObjectType, IconSetType};
use mjx_ooxml_types::support::OnOff;

use crate::font::ColorElement;
use crate::leaf::attribute_bag;
use crate::worksheet::rebuild_element;

attribute_bag! {
    /// `x:cfvo` (`CT_Cfvo`, `sml.xsd:2793`) — one threshold of a colour scale, data bar or icon set.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_Cfvo`. Wire element: `cfvo`.
    ///
    /// `@type` is the only `use="required"` attribute. `@val` is an `ST_Xstring` and stays text:
    /// §18.3.1.11 defines it as *"The value of this conditional formatting value object"* and says
    /// nothing about its lexical space, so a `formula` threshold and a `num` threshold arrive here
    /// the same way and neither is decoded.
    ///
    /// `@gte` is named from §18.3.1.11's own sentence — *"For icon sets, determines whether this
    /// threshold value uses the greater than or equal to operator. 0 indicates 'greater than' is
    /// used instead of 'greater than or equal to'"* — and defaults to `true`. It is meaningful only
    /// inside an [`IconSet`]; a colour scale and a data bar interpolate rather than compare.
    #[xml(attribute(
        local = "type",
        codec = Enumeration<ConditionalFormatValueObjectType>,
        accessor = value_kind,
        required
    ))]
    #[xml(attribute(local = "val", codec = Text, accessor = value))]
    #[xml(attribute(local = "gte", codec = OnOff, accessor = is_greater_than_or_equal, default = true))]
    ConditionalValueObject, "cfvo"
}

/// `x:colorScale` (`CT_ColorScale`, `sml.xsd:2769`) — a gradated colour scale: two or more
/// thresholds, and one colour for each.
///
/// **`ST_`/`CT_` symbol:** `CT_ColorScale`. Wire element: `colorScale`. The type carries **no
/// attributes at all**; it is entirely its two child lists, which are paired by position.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
#[xml(namespace = SML)]
pub struct ColorScale {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "cfvo", variant = Threshold, ty = ConditionalValueObject),
        child(local = "color", variant = Color, ty = ColorElement)
    )]
    content: Vec<ColorScaleContent>,
}

/// One child of [`ColorScale`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorScaleContent {
    /// `x:cfvo` (rank 0) — one interpolation point.
    Threshold(ConditionalValueObject),
    /// `x:color` (rank 1) — the colour that point is drawn in.
    Color(ColorElement),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl ColorScale {
    /// Builds an empty `x:colorScale`, bound to `prefix` or to the default namespace.
    ///
    /// The schema requires two of each child, so an empty scale is invalid markup; it is still
    /// constructible, because a caller builds one and then fills it.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "colorScale"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[ColorScaleContent] {
        &self.content
    }

    /// Every `x:cfvo`, in document order.
    pub fn thresholds(&self) -> impl Iterator<Item = &ConditionalValueObject> + '_ {
        self.content.iter().filter_map(|item| match item {
            ColorScaleContent::Threshold(value) => Some(value),
            _ => None,
        })
    }

    /// Every `x:color`, in document order.
    pub fn colors(&self) -> impl Iterator<Item = &ColorElement> + '_ {
        self.content.iter().filter_map(|item| match item {
            ColorScaleContent::Color(color) => Some(color),
            _ => None,
        })
    }

    /// The thresholds paired with their colours, by position.
    ///
    /// §18.3.1.11: *"The first `<cfvo>` element corresponds with the first `<color>` definition, and
    /// so on."* The pairing stops at the shorter list, so a scale the file wrote unbalanced yields
    /// only the pairs it actually states — [`is_balanced`](Self::is_balanced) is how a caller finds
    /// out that it had to.
    pub fn pairs(&self) -> impl Iterator<Item = (&ConditionalValueObject, &ColorElement)> + '_ {
        self.thresholds().zip(self.colors())
    }

    /// Whether the file wrote as many colours as thresholds.
    ///
    /// `false` is a description of the file, never a refusal: nothing here repairs an unbalanced
    /// scale, and [`pairs`](Self::pairs) simply reports the pairs that exist.
    #[must_use]
    pub fn is_balanced(&self) -> bool {
        self.thresholds().count() == self.colors().count()
    }

    /// Appends a threshold at its rank in `CT_ColorScale`'s sequence — after the last `cfvo`, and
    /// **before** the first `color`.
    pub fn push_threshold(&mut self, threshold: ConditionalValueObject) {
        let at = self.insert_index("cfvo");
        self.content
            .insert(at, ColorScaleContent::Threshold(threshold));
        self.empty = false;
    }

    /// Appends a colour after the colours already present.
    pub fn push_color(&mut self, color: ColorElement) {
        let at = self.insert_index("color");
        self.content.insert(at, ColorScaleContent::Color(color));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                ColorScaleContent::Threshold(value) => RawNode::Element(value.as_raw_element()),
                ColorScaleContent::Color(color) => RawNode::Element(color.as_raw_element()),
                ColorScaleContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }

    /// Where a child named `local` belongs, from the generated table.
    fn insert_index(&self, local: &str) -> usize {
        mjx_ooxml_types::child_order::CONDITIONAL_FORMAT_COLOR_SCALE
            .insert_index_of_names(self.content.iter().map(ColorScaleContent::rank), local)
    }
}

impl ColorScaleContent {
    /// This child's rank in `CT_ColorScale`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        let local = match self {
            Self::Threshold(_) => "cfvo",
            Self::Color(_) => "color",
            Self::Raw(_) => return None,
        };
        mjx_ooxml_types::child_order::CONDITIONAL_FORMAT_COLOR_SCALE.rank_of(None, local)
    }
}

impl ToXml for ColorScale {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:dataBar` (`CT_DataBar`, `sml.xsd:2775`) — an in-cell bar whose length tracks the value.
///
/// **`ST_`/`CT_` symbol:** `CT_DataBar`. Wire element: `dataBar`. Exactly two `cfvo` and exactly one
/// `color`, per the schema's own `minOccurs`/`maxOccurs`.
///
/// `@minLength` and `@maxLength` are *"as a percentage of the cell width"* (§18.3.1.28) and default
/// to `10` and `90`. The bar length §18.3.1.28 gives a formula for is a rendering question and is
/// not computed here.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "minLength", codec = Number<u32>, accessor = minimum_length, default = 10))]
#[xml(attribute(local = "maxLength", codec = Number<u32>, accessor = maximum_length, default = 90))]
#[xml(attribute(local = "showValue", codec = OnOff, accessor = shows_cell_value, default = true))]
pub struct DataBar {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "cfvo", variant = Threshold, ty = ConditionalValueObject),
        child(local = "color", variant = Color, ty = ColorElement)
    )]
    content: Vec<DataBarContent>,
}

/// One child of [`DataBar`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataBarContent {
    /// `x:cfvo` (rank 0) — the shorter and longer ends of the bar, in that order.
    Threshold(ConditionalValueObject),
    /// `x:color` (rank 1) — the one colour the bar is drawn in.
    Color(ColorElement),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl DataBar {
    /// Builds an empty `x:dataBar`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "dataBar"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[DataBarContent] {
        &self.content
    }

    /// Every `x:cfvo`, in document order — the schema declares exactly two.
    pub fn thresholds(&self) -> impl Iterator<Item = &ConditionalValueObject> + '_ {
        self.content.iter().filter_map(|item| match item {
            DataBarContent::Threshold(value) => Some(value),
            _ => None,
        })
    }

    /// The bar's one `x:color`, or `None` for a bar the file wrote without one.
    #[must_use]
    pub fn color(&self) -> Option<&ColorElement> {
        self.content.iter().find_map(|item| match item {
            DataBarContent::Color(color) => Some(color),
            _ => None,
        })
    }

    /// Appends a threshold before the colour.
    pub fn push_threshold(&mut self, threshold: ConditionalValueObject) {
        let at = self.insert_index("cfvo");
        self.content
            .insert(at, DataBarContent::Threshold(threshold));
        self.empty = false;
    }

    /// Sets the bar's colour: replacing the existing element where it is, or inserting one at its
    /// rank after the thresholds.
    pub fn set_color(&mut self, color: Option<ColorElement>) {
        let existing = self
            .content
            .iter()
            .position(|item| matches!(item, DataBarContent::Color(_)));
        match (existing, color) {
            (Some(at), Some(color)) => self.content[at] = DataBarContent::Color(color),
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(color)) => {
                let at = self.insert_index("color");
                self.content.insert(at, DataBarContent::Color(color));
                self.empty = false;
            }
            (None, None) => {}
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                DataBarContent::Threshold(value) => RawNode::Element(value.as_raw_element()),
                DataBarContent::Color(color) => RawNode::Element(color.as_raw_element()),
                DataBarContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }

    /// Where a child named `local` belongs, from the generated table.
    fn insert_index(&self, local: &str) -> usize {
        mjx_ooxml_types::child_order::CONDITIONAL_FORMAT_DATA_BAR
            .insert_index_of_names(self.content.iter().map(DataBarContent::rank), local)
    }
}

impl DataBarContent {
    /// This child's rank in `CT_DataBar`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        let local = match self {
            Self::Threshold(_) => "cfvo",
            Self::Color(_) => "color",
            Self::Raw(_) => return None,
        };
        mjx_ooxml_types::child_order::CONDITIONAL_FORMAT_DATA_BAR.rank_of(None, local)
    }
}

impl ToXml for DataBar {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:iconSet` (`CT_IconSet`, `sml.xsd:2784`) — one icon per band, and the thresholds between them.
///
/// **`ST_`/`CT_` symbol:** `CT_IconSet`. Wire element: `iconSet`. Two or more `cfvo` and no colour:
/// the icons come from `@iconSet`, whose schema default is the wire token `3TrafficLights1` —
/// [`IconSetType::ThreeTrafficLights`].
///
/// `@percent` says the thresholds are percentiles rather than numbers (§18.3.1.49) and defaults to
/// `true`; `@reverse` *"reverses the default order of the icons in this icon set"*.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(
    local = "iconSet",
    codec = Enumeration<IconSetType>,
    accessor = icons,
    default = IconSetType::ThreeTrafficLights
))]
#[xml(attribute(local = "showValue", codec = OnOff, accessor = shows_cell_value, default = true))]
#[xml(attribute(local = "percent", codec = OnOff, accessor = thresholds_are_percentiles, default = true))]
#[xml(attribute(local = "reverse", codec = OnOff, accessor = icons_are_reversed, default = false))]
pub struct IconSet {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "cfvo", variant = Threshold, ty = ConditionalValueObject))]
    content: Vec<IconSetContent>,
}

/// One child of [`IconSet`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconSetContent {
    /// `x:cfvo` — one band boundary.
    Threshold(ConditionalValueObject),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl IconSet {
    /// Builds an empty `x:iconSet`, bound to `prefix` or to the default namespace.
    ///
    /// Writes no `@iconSet`, which means the schema default `3TrafficLights1` — the same "absent is
    /// not a value" rule every table in this crate follows.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "iconSet"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[IconSetContent] {
        &self.content
    }

    /// Every `x:cfvo`, in document order.
    pub fn thresholds(&self) -> impl Iterator<Item = &ConditionalValueObject> + '_ {
        self.content.iter().filter_map(|item| match item {
            IconSetContent::Threshold(value) => Some(value),
            IconSetContent::Raw(_) => None,
        })
    }

    /// Appends a threshold after the ones already present.
    ///
    /// `CT_IconSet` declares one repeating child, so there is no order to place against and this
    /// really is an append.
    pub fn push_threshold(&mut self, threshold: ConditionalValueObject) {
        self.content.push(IconSetContent::Threshold(threshold));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                IconSetContent::Threshold(value) => RawNode::Element(value.as_raw_element()),
                IconSetContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for IconSet {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}
