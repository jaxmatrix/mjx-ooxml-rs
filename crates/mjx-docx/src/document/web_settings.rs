//! `word/webSettings.xml` (`CT_WebSettings`, the `w:webSettings` root, §17.15.2.44) — MJXOFF-136's
//! own file. Legacy web-layout markup: a saved-as-web-page document's frameset or `w:div` tree, and
//! the save-as-web flags Word itself still writes into every `.docx` that has ever been saved as a
//! web page even once. `w:divId` on a paragraph's own `w:pPr` (`CT_PPrBase`, C4) points into the
//! `w:div` tree this file models.
//!
//! # Recursive legacy nesting: `CT_Frameset`'s own `frameset`/`frame` choice, and `CT_Div`'s own
//! `divsChild`
//!
//! `CT_Frameset` can recursively contain further `w:frameset`s (modelled here with `Box` indirection
//! — [`FramesetContent::NestedFrameset`]); `CT_Div` can recursively contain further `w:divsChild`
//! (`CT_Divs`, itself a list of `CT_Div`). The `frameset` recursion gets a typed, boxed model because
//! `CT_Frameset` is one of this file's own named types; **`w:divsChild` is deliberately left
//! unmodelled** — every `w:div` this crate could plausibly need to address (`w:divId`'s own target)
//! is a *direct* `w:div` under `word/webSettings.xml`'s own `w:divs`, and an unbounded-depth typed
//! Rust structure for markup no real modern document nests more than one level deep would cost more
//! than it returns. `w:divsChild` still round-trips exactly — it simply falls into
//! [`DivContent::Raw`], this crate's ordinary unknown-element bucket, since no `child(..)` entry
//! names it.

use mjx_ooxml_core::{
    AttributeCodec, AttributeError, Enumeration, FromXml, FromXmlError, Interner,
    InvalidAttributeValue, RawAttribute, RawElement, RawName, RawNode, ToXml,
};
use mjx_ooxml_types::child_order::WEB_SETTINGS;
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::wordprocessingml::{
    FrameLayout, FrameScrollbarVisibility, PixelsMeasure, TargetScreenSize,
};

use super::body::wml_name;
use super::paragraph_properties::DecimalNumberValue;
use super::run_properties::{Border, Color, SignedTwips, Toggle};
use super::settings::TwipsMeasureValue;
use super::styles::StyleString;

/// `s:ST_SignedTwipsMeasure` (`w:marLeft`/`w:marRight`/`w:marTop`/`w:marBottom`) as an attribute
/// value — a signed measure in twentieths of a point.
type SignedTwipsMeasure = mjx_ooxml_types::wordprocessingml::SignedTwipsMeasure;

/// `CT_SignedTwipsMeasure`, reused across `w:div`'s four required margins — one wire shape under
/// four names, like [`super::settings::TwipsMeasureValue`] but signed.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = SignedTwips, accessor = twentieths_of_a_point, required))]
pub struct SignedTwipsMeasureElement {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl SignedTwipsMeasureElement {
    /// Builds a new `local` element of `value`.
    #[must_use]
    fn new(interner: &mut Interner, local: &str, value: SignedTwipsMeasure) -> Self {
        let mut item = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_twentieths_of_a_point(interner, value);
        item
    }
}

impl FromXml for SignedTwipsMeasureElement {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for SignedTwipsMeasureElement {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_PixelsMeasure`, reused across `w:frame`'s `marW`/`marH` — one wire shape under two names.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = mjx_ooxml_core::Number<PixelsMeasure>, accessor = pixels, required))]
pub struct PixelsMeasureValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl PixelsMeasureValue {
    /// Builds a new `local` element of `pixels`.
    #[must_use]
    pub(crate) fn new(interner: &mut Interner, local: &str, pixels: PixelsMeasure) -> Self {
        let mut item = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_pixels(interner, pixels);
        item
    }
}

impl FromXml for PixelsMeasureValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for PixelsMeasureValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `ST_TargetScreenSz` (`w:targetScreenSz/@val`) as an attribute value.
#[derive(Debug)]
struct TargetScreenSizeCodec;

impl AttributeCodec for TargetScreenSizeCodec {
    type Value<'a> = TargetScreenSize;
    type Input<'a> = TargetScreenSize;

    fn decode<'a>(raw: std::borrow::Cow<'a, str>) -> Result<TargetScreenSize, InvalidAttributeValue> {
        raw.parse().map_err(|_| InvalidAttributeValue::new("not a valid ST_TargetScreenSz"))
    }

    fn encode<'a>(value: TargetScreenSize) -> std::borrow::Cow<'a, str> {
        std::borrow::Cow::Owned(value.to_string())
    }
}

// =================================================================================================
// Attribute-only leaves: CT_FrameScrollbar, CT_FrameLayout, CT_TargetScreenSz.
// =================================================================================================

/// `w:scrollbar` inside `w:frame` (`CT_FrameScrollbar`, §17.15.2.34) — whether/when the frame shows
/// scrollbars.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<FrameScrollbarVisibility>, accessor = visibility, required))]
pub struct FrameScrollbarSetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FrameScrollbarSetting {
    /// Builds a new `w:scrollbar` of `visibility`.
    #[must_use]
    pub fn new(interner: &mut Interner, visibility: FrameScrollbarVisibility) -> Self {
        let mut item = Self {
            name: wml_name(interner, "scrollbar"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_visibility(interner, visibility);
        item
    }
}

impl FromXml for FrameScrollbarSetting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for FrameScrollbarSetting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:frameLayout` (`CT_FrameLayout`, §17.15.2.20) — how a frameset's own rows/columns divide the
/// page.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<FrameLayout>, accessor = kind, required))]
pub struct FrameLayoutSetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FrameLayoutSetting {
    /// Builds a new `w:frameLayout` of `kind`.
    #[must_use]
    pub fn new(interner: &mut Interner, kind: FrameLayout) -> Self {
        let mut item = Self {
            name: wml_name(interner, "frameLayout"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_kind(interner, kind);
        item
    }
}

impl FromXml for FrameLayoutSetting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for FrameLayoutSetting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:targetScreenSz` (`CT_TargetScreenSz`, §17.15.2.42) — the screen resolution web layout was
/// optimized for.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = TargetScreenSizeCodec, accessor = size, required))]
pub struct TargetScreenSizeSetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl TargetScreenSizeSetting {
    /// Builds a new `w:targetScreenSz` of `size`.
    #[must_use]
    pub fn new(interner: &mut Interner, size: TargetScreenSize) -> Self {
        let mut item = Self {
            name: wml_name(interner, "targetScreenSz"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_size(interner, size);
        item
    }
}

impl FromXml for TargetScreenSizeSetting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for TargetScreenSizeSetting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:optimizeForBrowser` (`CT_OptimizeForBrowser`, §17.15.2.28) — `CT_OnOff` extended with an
/// optional `target` browser string.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = OnOff, accessor = value))]
#[xml(attribute(local = "target", prefix = "w", codec = mjx_ooxml_core::Text, accessor = target))]
pub struct OptimizeForBrowserSetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl OptimizeForBrowserSetting {
    /// Builds a new, empty `w:optimizeForBrowser` — both attributes absent until a setter states
    /// one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "optimizeForBrowser"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for OptimizeForBrowserSetting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for OptimizeForBrowserSetting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// =================================================================================================
// CT_FramesetSplitbar, CT_Frame, CT_Frameset
// =================================================================================================

/// `w:framesetSplitbar` (`CT_FramesetSplitbar`, §17.15.2.19) — the divider between two frames.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct FramesetSplitbar {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "w", variant = Width, ty = TwipsMeasureValue),
        child(local = "color", variant = Color, ty = Color),
        child(local = "noBorder", variant = NoBorder, ty = Toggle),
        child(local = "flatBorders", variant = FlatBorders, ty = Toggle)
    )]
    content: Vec<FramesetSplitbarContent>,
}

/// One child of [`FramesetSplitbar`] — four slots, hand-ordered directly from `wml.xsd`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FramesetSplitbarContent {
    /// `w:w`.
    Width(TwipsMeasureValue),
    /// `w:color`.
    Color(Color),
    /// `w:noBorder`.
    NoBorder(Toggle),
    /// `w:flatBorders`.
    FlatBorders(Toggle),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl FramesetSplitbar {
    /// Builds a new, empty `w:framesetSplitbar`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "framesetSplitbar"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }
}

/// `w:frame` (`CT_Frame`, §17.15.2.18) — one frame's own source, sizing and scrollbar behaviour.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Frame {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "sz", variant = Size, ty = StyleString),
        child(local = "name", variant = FrameName, ty = StyleString),
        child(local = "title", variant = Title, ty = StyleString),
        child(local = "longDesc", variant = LongDescription, ty = super::body::RelationshipReference),
        child(local = "sourceFileName", variant = SourceFileName, ty = super::body::RelationshipReference),
        child(local = "marW", variant = MarginWidth, ty = PixelsMeasureValue),
        child(local = "marH", variant = MarginHeight, ty = PixelsMeasureValue),
        child(local = "scrollbar", variant = Scrollbar, ty = FrameScrollbarSetting),
        child(local = "noResizeAllowed", variant = NoResizeAllowed, ty = Toggle),
        child(local = "linkedToFile", variant = LinkedToFile, ty = Toggle)
    )]
    content: Vec<FrameContent>,
}

/// One child of [`Frame`] — `CT_Frame`'s own ten, hand-ordered directly from `wml.xsd`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameContent {
    /// `w:sz`.
    Size(StyleString),
    /// `w:name`.
    FrameName(StyleString),
    /// `w:title`.
    Title(StyleString),
    /// `w:longDesc` — `CT_Rel`.
    LongDescription(super::body::RelationshipReference),
    /// `w:sourceFileName` — `CT_Rel`.
    SourceFileName(super::body::RelationshipReference),
    /// `w:marW`.
    MarginWidth(PixelsMeasureValue),
    /// `w:marH`.
    MarginHeight(PixelsMeasureValue),
    /// `w:scrollbar`.
    Scrollbar(FrameScrollbarSetting),
    /// `w:noResizeAllowed`.
    NoResizeAllowed(Toggle),
    /// `w:linkedToFile`.
    LinkedToFile(Toggle),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl Frame {
    /// Builds a new, empty `w:frame`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "frame"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &FrameContent) -> Option<u16> {
        Some(match item {
            FrameContent::Size(_) => 0,
            FrameContent::FrameName(_) => 1,
            FrameContent::Title(_) => 2,
            FrameContent::LongDescription(_) => 3,
            FrameContent::SourceFileName(_) => 4,
            FrameContent::MarginWidth(_) => 5,
            FrameContent::MarginHeight(_) => 6,
            FrameContent::Scrollbar(_) => 7,
            FrameContent::NoResizeAllowed(_) => 8,
            FrameContent::LinkedToFile(_) => 9,
            FrameContent::Raw(_) => return None,
        })
    }

    fn insert_at_rank(&mut self, item: FrameContent) {
        let rank = Self::rank(&item);
        let mut at = self.content.len();
        for (index, existing) in self.content.iter().enumerate() {
            if let (Some(rank), Some(existing_rank)) = (rank, Self::rank(existing)) {
                if existing_rank > rank {
                    at = index;
                    break;
                }
            }
        }
        self.content.insert(at, item);
        self.empty = false;
    }

    /// `w:marW` — this frame's own margin width, in pixels.
    #[must_use]
    pub fn margin_width(&self) -> Option<&PixelsMeasureValue> {
        self.content.iter().find_map(|item| match item {
            FrameContent::MarginWidth(value) => Some(value),
            _ => None,
        })
    }

    /// Sets (or, with `None`, removes) `w:marW`.
    pub fn set_margin_width(&mut self, interner: &mut Interner, value: Option<PixelsMeasure>) {
        self.content
            .retain(|item| !matches!(item, FrameContent::MarginWidth(_)));
        if let Some(value) = value {
            let element = PixelsMeasureValue::new(interner, "marW", value);
            self.insert_at_rank(FrameContent::MarginWidth(element));
        }
    }

    /// `w:marH` — this frame's own margin height, in pixels.
    #[must_use]
    pub fn margin_height(&self) -> Option<&PixelsMeasureValue> {
        self.content.iter().find_map(|item| match item {
            FrameContent::MarginHeight(value) => Some(value),
            _ => None,
        })
    }

    /// Sets (or, with `None`, removes) `w:marH`.
    pub fn set_margin_height(&mut self, interner: &mut Interner, value: Option<PixelsMeasure>) {
        self.content
            .retain(|item| !matches!(item, FrameContent::MarginHeight(_)));
        if let Some(value) = value {
            let element = PixelsMeasureValue::new(interner, "marH", value);
            self.insert_at_rank(FrameContent::MarginHeight(element));
        }
    }
}

/// `w:frameset` (`CT_Frameset`, §17.15.2.21) — one frameset: its own size/split/layout/title, then
/// recursively nested framesets and/or frames. See the module's own doc comment for the recursion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frameset {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    content: Vec<FramesetContent>,
}

/// One child of [`Frameset`]: `sz, framesetSplitbar, frameLayout, title`, then a repeatable choice
/// of nested `frameset`/`frame` — five ranks, hand-ordered directly from `wml.xsd`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FramesetContent {
    /// `w:sz`.
    Size(StyleString),
    /// `w:framesetSplitbar`.
    SplitBar(FramesetSplitbar),
    /// `w:frameLayout`.
    Layout(FrameLayoutSetting),
    /// `w:title`.
    Title(StyleString),
    /// `w:frameset` — a nested frameset, boxed for the recursive type.
    NestedFrameset(Box<Frameset>),
    /// `w:frame`.
    Frame(Frame),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl Frameset {
    /// Builds a new, empty `w:frameset`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "frameset"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &FramesetContent) -> Option<u16> {
        match item {
            FramesetContent::Size(_) => Some(0),
            FramesetContent::SplitBar(_) => Some(1),
            FramesetContent::Layout(_) => Some(2),
            FramesetContent::Title(_) => Some(3),
            FramesetContent::NestedFrameset(_) | FramesetContent::Frame(_) => Some(4),
            FramesetContent::Raw(_) => None,
        }
    }

    fn insert_at_rank(&mut self, item: FramesetContent) {
        let rank = Self::rank(&item);
        let mut at = self.content.len();
        for (index, existing) in self.content.iter().enumerate() {
            if let (Some(rank), Some(existing_rank)) = (rank, Self::rank(existing)) {
                if existing_rank > rank {
                    at = index;
                    break;
                }
            }
        }
        self.content.insert(at, item);
        self.empty = false;
    }

    /// Every direct child frame, in document order.
    pub fn frames(&self) -> impl Iterator<Item = &Frame> {
        self.content.iter().filter_map(|item| match item {
            FramesetContent::Frame(value) => Some(value),
            _ => None,
        })
    }

    /// Appends a new child `w:frame`.
    pub fn add_frame(&mut self, value: Frame) {
        self.insert_at_rank(FramesetContent::Frame(value));
    }

    /// Every direct nested frameset, in document order.
    pub fn nested_framesets(&self) -> impl Iterator<Item = &Frameset> {
        self.content.iter().filter_map(|item| match item {
            FramesetContent::NestedFrameset(value) => Some(value.as_ref()),
            _ => None,
        })
    }

    /// Appends a new nested `w:frameset`.
    pub fn add_nested_frameset(&mut self, value: Frameset) {
        self.insert_at_rank(FramesetContent::NestedFrameset(Box::new(value)));
    }
}

impl FromXml for Frameset {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let mut content = Vec::with_capacity(element.children.len());
        for node in &element.children {
            let RawNode::Element(child) = node else {
                content.push(FramesetContent::Raw(node.clone()));
                continue;
            };
            let namespace = child.name.namespace.map(|s| interner.resolve(s));
            let is_wml = namespace == Some(mjx_ooxml_types::namespaces::WML.transitional)
                || namespace == mjx_ooxml_types::namespaces::WML.strict;
            let local = interner.resolve(child.name.local);
            let item = if is_wml && local == "sz" {
                FramesetContent::Size(StyleString::from_xml(child, interner)?)
            } else if is_wml && local == "framesetSplitbar" {
                FramesetContent::SplitBar(FramesetSplitbar::from_xml(child, interner)?)
            } else if is_wml && local == "frameLayout" {
                FramesetContent::Layout(FrameLayoutSetting::from_xml(child, interner)?)
            } else if is_wml && local == "title" {
                FramesetContent::Title(StyleString::from_xml(child, interner)?)
            } else if is_wml && local == "frameset" {
                FramesetContent::NestedFrameset(Box::new(Frameset::from_xml(child, interner)?))
            } else if is_wml && local == "frame" {
                FramesetContent::Frame(Frame::from_xml(child, interner)?)
            } else {
                FramesetContent::Raw(node.clone())
            };
            content.push(item);
        }
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            content,
        })
    }
}

impl ToXml for Frameset {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                FramesetContent::Size(value) => RawNode::Element(value.to_xml(interner)),
                FramesetContent::SplitBar(value) => RawNode::Element(value.to_xml(interner)),
                FramesetContent::Layout(value) => RawNode::Element(value.to_xml(interner)),
                FramesetContent::Title(value) => RawNode::Element(value.to_xml(interner)),
                FramesetContent::NestedFrameset(value) => RawNode::Element(value.to_xml(interner)),
                FramesetContent::Frame(value) => RawNode::Element(value.to_xml(interner)),
                FramesetContent::Raw(node) => node.clone(),
            })
            .collect::<Vec<_>>();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// =================================================================================================
// CT_DivBdr, CT_Div, CT_Divs
// =================================================================================================

/// `w:divBdr` (`CT_DivBdr`, §17.15.2.13) — the four borders around a `w:div`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct DivBorders {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "top", variant = Top, ty = Border),
        child(local = "left", variant = Left, ty = Border),
        child(local = "bottom", variant = Bottom, ty = Border),
        child(local = "right", variant = Right, ty = Border)
    )]
    content: Vec<DivBordersContent>,
}

/// One child of [`DivBorders`] — the same four-position pattern as
/// [`super::paragraph_properties::ParagraphBorders`]'s own `w:pBdr` (a subset: no `between`/`bar`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DivBordersContent {
    /// `w:top`.
    Top(Border),
    /// `w:left`.
    Left(Border),
    /// `w:bottom`.
    Bottom(Border),
    /// `w:right`.
    Right(Border),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl DivBorders {
    /// Builds a new, empty `w:divBdr`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "divBdr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &DivBordersContent) -> Option<u16> {
        Some(match item {
            DivBordersContent::Top(_) => 0,
            DivBordersContent::Left(_) => 1,
            DivBordersContent::Bottom(_) => 2,
            DivBordersContent::Right(_) => 3,
            DivBordersContent::Raw(_) => return None,
        })
    }

    fn side(&self, at: u16) -> Option<&Border> {
        self.content.iter().find_map(|item| {
            (Self::rank(item) == Some(at)).then_some(match item {
                DivBordersContent::Top(b)
                | DivBordersContent::Left(b)
                | DivBordersContent::Bottom(b)
                | DivBordersContent::Right(b) => b,
                DivBordersContent::Raw(_) => unreachable!(),
            })
        })
    }

    fn set_side(&mut self, interner: &mut Interner, at: u16, local: &str, value: Option<Border>) {
        self.content.retain(|item| Self::rank(item) != Some(at));
        if let Some(value) = value {
            let wrapped = value.renamed(interner, local);
            let item = match at {
                0 => DivBordersContent::Top(wrapped),
                1 => DivBordersContent::Left(wrapped),
                2 => DivBordersContent::Bottom(wrapped),
                _ => DivBordersContent::Right(wrapped),
            };
            let insert_at = self
                .content
                .iter()
                .position(|existing| Self::rank(existing).is_some_and(|rank| rank > at))
                .unwrap_or(self.content.len());
            self.content.insert(insert_at, item);
        }
        self.empty = false;
    }

    /// `w:top`.
    #[must_use]
    pub fn top(&self) -> Option<&Border> {
        self.side(0)
    }
    /// Sets (or removes) `w:top`.
    pub fn set_top(&mut self, interner: &mut Interner, value: Option<Border>) {
        self.set_side(interner, 0, "top", value);
    }
    /// `w:left`.
    #[must_use]
    pub fn left(&self) -> Option<&Border> {
        self.side(1)
    }
    /// Sets (or removes) `w:left`.
    pub fn set_left(&mut self, interner: &mut Interner, value: Option<Border>) {
        self.set_side(interner, 1, "left", value);
    }
    /// `w:bottom`.
    #[must_use]
    pub fn bottom(&self) -> Option<&Border> {
        self.side(2)
    }
    /// Sets (or removes) `w:bottom`.
    pub fn set_bottom(&mut self, interner: &mut Interner, value: Option<Border>) {
        self.set_side(interner, 2, "bottom", value);
    }
    /// `w:right`.
    #[must_use]
    pub fn right(&self) -> Option<&Border> {
        self.side(3)
    }
    /// Sets (or removes) `w:right`.
    pub fn set_right(&mut self, interner: &mut Interner, value: Option<Border>) {
        self.set_side(interner, 3, "right", value);
    }
}

/// `w:div` (`CT_Div`, §17.15.2.14) — one legacy HTML `<div>` this document's own paragraphs (via
/// `w:divId`, C4) may point into. `w:divsChild` (recursive nesting) is deliberately unmodelled — see
/// the module's own doc comment.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "w", codec = mjx_ooxml_core::Number<i64>, accessor = id, required))]
pub struct Div {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    content: Vec<DivContent>,
}

/// One child of [`Div`]: `blockQuote?, bodyDiv?, marLeft, marRight, marTop, marBottom, divBdr?`,
/// then `divsChild*` (unmodelled — falls to [`DivContent::Raw`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DivContent {
    /// `w:blockQuote`.
    BlockQuote(Toggle),
    /// `w:bodyDiv`.
    BodyDiv(Toggle),
    /// `w:marLeft` — required per the schema.
    MarginLeft(SignedTwipsMeasureElement),
    /// `w:marRight` — required per the schema.
    MarginRight(SignedTwipsMeasureElement),
    /// `w:marTop` — required per the schema.
    MarginTop(SignedTwipsMeasureElement),
    /// `w:marBottom` — required per the schema.
    MarginBottom(SignedTwipsMeasureElement),
    /// `w:divBdr`.
    Borders(DivBorders),
    /// Any other child — `w:divsChild` included; preserved verbatim (see the module's own doc
    /// comment).
    Raw(RawNode),
}

impl Div {
    /// Builds a new `w:div` identified by `id`, every child absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner, id: i64) -> Self {
        let mut item = Self {
            name: wml_name(interner, "div"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        item.set_id(interner, id);
        item
    }

    fn rank(item: &DivContent) -> Option<u16> {
        Some(match item {
            DivContent::BlockQuote(_) => 0,
            DivContent::BodyDiv(_) => 1,
            DivContent::MarginLeft(_) => 2,
            DivContent::MarginRight(_) => 3,
            DivContent::MarginTop(_) => 4,
            DivContent::MarginBottom(_) => 5,
            DivContent::Borders(_) => 6,
            DivContent::Raw(_) => return None,
        })
    }

    fn insert_at_rank(&mut self, item: DivContent) {
        let rank = Self::rank(&item);
        let mut at = self.content.len();
        for (index, existing) in self.content.iter().enumerate() {
            if let (Some(rank), Some(existing_rank)) = (rank, Self::rank(existing)) {
                if existing_rank > rank {
                    at = index;
                    break;
                }
            }
        }
        self.content.insert(at, item);
        self.empty = false;
    }

    fn margin(&self, at: u16) -> Option<&SignedTwipsMeasureElement> {
        self.content.iter().find_map(|item| {
            (Self::rank(item) == Some(at)).then_some(match item {
                DivContent::MarginLeft(v)
                | DivContent::MarginRight(v)
                | DivContent::MarginTop(v)
                | DivContent::MarginBottom(v) => v,
                _ => unreachable!(),
            })
        })
    }

    fn set_margin(&mut self, interner: &mut Interner, at: u16, local: &str, value: SignedTwipsMeasure) {
        self.content.retain(|item| Self::rank(item) != Some(at));
        let wrapped = SignedTwipsMeasureElement::new(interner, local, value);
        let item = match at {
            2 => DivContent::MarginLeft(wrapped),
            3 => DivContent::MarginRight(wrapped),
            4 => DivContent::MarginTop(wrapped),
            _ => DivContent::MarginBottom(wrapped),
        };
        self.insert_at_rank(item);
    }

    /// `w:marLeft`.
    #[must_use]
    pub fn margin_left(&self) -> Option<&SignedTwipsMeasureElement> {
        self.margin(2)
    }
    /// Sets `w:marLeft` (required per the schema — there is no "remove" for a required child).
    pub fn set_margin_left(&mut self, interner: &mut Interner, value: SignedTwipsMeasure) {
        self.set_margin(interner, 2, "marLeft", value);
    }
    /// `w:marRight`.
    #[must_use]
    pub fn margin_right(&self) -> Option<&SignedTwipsMeasureElement> {
        self.margin(3)
    }
    /// Sets `w:marRight`.
    pub fn set_margin_right(&mut self, interner: &mut Interner, value: SignedTwipsMeasure) {
        self.set_margin(interner, 3, "marRight", value);
    }
    /// `w:marTop`.
    #[must_use]
    pub fn margin_top(&self) -> Option<&SignedTwipsMeasureElement> {
        self.margin(4)
    }
    /// Sets `w:marTop`.
    pub fn set_margin_top(&mut self, interner: &mut Interner, value: SignedTwipsMeasure) {
        self.set_margin(interner, 4, "marTop", value);
    }
    /// `w:marBottom`.
    #[must_use]
    pub fn margin_bottom(&self) -> Option<&SignedTwipsMeasureElement> {
        self.margin(5)
    }
    /// Sets `w:marBottom`.
    pub fn set_margin_bottom(&mut self, interner: &mut Interner, value: SignedTwipsMeasure) {
        self.set_margin(interner, 5, "marBottom", value);
    }

    /// `w:divBdr`.
    #[must_use]
    pub fn borders(&self) -> Option<&DivBorders> {
        self.content.iter().find_map(|item| match item {
            DivContent::Borders(value) => Some(value),
            _ => None,
        })
    }
    /// Sets (or removes) `w:divBdr`.
    pub fn set_borders(&mut self, value: Option<DivBorders>) {
        self.content.retain(|item| !matches!(item, DivContent::Borders(_)));
        if let Some(value) = value {
            self.insert_at_rank(DivContent::Borders(value));
        }
    }
}

impl FromXml for Div {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let mut content = Vec::with_capacity(element.children.len());
        for node in &element.children {
            let RawNode::Element(child) = node else {
                content.push(DivContent::Raw(node.clone()));
                continue;
            };
            let namespace = child.name.namespace.map(|s| interner.resolve(s));
            let is_wml = namespace == Some(mjx_ooxml_types::namespaces::WML.transitional)
                || namespace == mjx_ooxml_types::namespaces::WML.strict;
            let local = interner.resolve(child.name.local);
            let item = if is_wml && local == "blockQuote" {
                DivContent::BlockQuote(Toggle::from_xml(child, interner)?)
            } else if is_wml && local == "bodyDiv" {
                DivContent::BodyDiv(Toggle::from_xml(child, interner)?)
            } else if is_wml && local == "marLeft" {
                DivContent::MarginLeft(SignedTwipsMeasureElement::from_xml(child, interner)?)
            } else if is_wml && local == "marRight" {
                DivContent::MarginRight(SignedTwipsMeasureElement::from_xml(child, interner)?)
            } else if is_wml && local == "marTop" {
                DivContent::MarginTop(SignedTwipsMeasureElement::from_xml(child, interner)?)
            } else if is_wml && local == "marBottom" {
                DivContent::MarginBottom(SignedTwipsMeasureElement::from_xml(child, interner)?)
            } else if is_wml && local == "divBdr" {
                DivContent::Borders(DivBorders::from_xml(child, interner)?)
            } else {
                DivContent::Raw(node.clone())
            };
            content.push(item);
        }
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            content,
        })
    }
}

impl ToXml for Div {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                DivContent::BlockQuote(value) => RawNode::Element(value.to_xml(interner)),
                DivContent::BodyDiv(value) => RawNode::Element(value.to_xml(interner)),
                DivContent::MarginLeft(value) => RawNode::Element(value.to_xml(interner)),
                DivContent::MarginRight(value) => RawNode::Element(value.to_xml(interner)),
                DivContent::MarginTop(value) => RawNode::Element(value.to_xml(interner)),
                DivContent::MarginBottom(value) => RawNode::Element(value.to_xml(interner)),
                DivContent::Borders(value) => RawNode::Element(value.to_xml(interner)),
                DivContent::Raw(node) => node.clone(),
            })
            .collect::<Vec<_>>();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:divs`/`w:divsChild` (`CT_Divs`, §17.15.2.15) — a list of `w:div`, at least one per the schema
/// (reading never rejects an empty one — fidelity-first).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Divs {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "div", variant = Entry, ty = Div))]
    content: Vec<DivsContent>,
}

/// One child of [`Divs`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DivsContent {
    /// `w:div`.
    Entry(Div),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl Divs {
    /// Builds a new, empty `w:divs`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "divs"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every direct `w:div`, in document order.
    pub fn entries(&self) -> impl Iterator<Item = &Div> {
        self.content.iter().filter_map(|item| match item {
            DivsContent::Entry(value) => Some(value),
            DivsContent::Raw(_) => None,
        })
    }

    /// The `w:div` whose `w:id` equals `id`, if one exists among the direct children (`w:divId`,
    /// C4's own reference target — nested `w:divsChild` entries are not searched; see the module's
    /// own doc comment).
    #[must_use]
    pub fn by_id(&self, interner: &Interner, id: i64) -> Option<&Div> {
        self.entries()
            .find(|div| div.id(interner).ok() == Some(id))
    }

    /// Appends `div` — the schema imposes no order among `w:div` siblings.
    pub fn add_entry(&mut self, div: Div) {
        self.content.push(DivsContent::Entry(div));
        self.empty = false;
    }
}

// =================================================================================================
// CT_WebSettings — the part root.
// =================================================================================================

/// `word/webSettings.xml`'s own root (`w:webSettings`, `CT_WebSettings`, §17.15.2.44).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct WebSettings {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "frameset", variant = Frameset, ty = Frameset),
        child(local = "divs", variant = Divs, ty = Divs),
        child(local = "encoding", variant = Encoding, ty = StyleString),
        child(local = "optimizeForBrowser", variant = OptimizeForBrowser, ty = OptimizeForBrowserSetting),
        child(local = "relyOnVML", variant = RelyOnVml, ty = Toggle),
        child(local = "allowPNG", variant = AllowPng, ty = Toggle),
        child(local = "doNotRelyOnCSS", variant = DoNotRelyOnCss, ty = Toggle),
        child(local = "doNotSaveAsSingleFile", variant = DoNotSaveAsSingleFile, ty = Toggle),
        child(local = "doNotOrganizeInFolder", variant = DoNotOrganizeInFolder, ty = Toggle),
        child(local = "doNotUseLongFileNames", variant = DoNotUseLongFileNames, ty = Toggle),
        child(local = "pixelsPerInch", variant = PixelsPerInch, ty = DecimalNumberValue),
        child(local = "targetScreenSz", variant = TargetScreenSize, ty = TargetScreenSizeSetting),
        child(local = "saveSmartTagsAsXml", variant = SaveSmartTagsAsXml, ty = Toggle)
    )]
    content: Vec<WebSettingsContent>,
}

/// One child of [`WebSettings`]: `CT_WebSettings`'s own thirteen, ranked from the generated
/// [`WEB_SETTINGS`] table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebSettingsContent {
    /// `w:frameset`.
    Frameset(Frameset),
    /// `w:divs`.
    Divs(Divs),
    /// `w:encoding`.
    Encoding(StyleString),
    /// `w:optimizeForBrowser`.
    OptimizeForBrowser(OptimizeForBrowserSetting),
    /// `w:relyOnVML`.
    RelyOnVml(Toggle),
    /// `w:allowPNG`.
    AllowPng(Toggle),
    /// `w:doNotRelyOnCSS`.
    DoNotRelyOnCss(Toggle),
    /// `w:doNotSaveAsSingleFile`.
    DoNotSaveAsSingleFile(Toggle),
    /// `w:doNotOrganizeInFolder`.
    DoNotOrganizeInFolder(Toggle),
    /// `w:doNotUseLongFileNames`.
    DoNotUseLongFileNames(Toggle),
    /// `w:pixelsPerInch`.
    PixelsPerInch(DecimalNumberValue),
    /// `w:targetScreenSz`.
    TargetScreenSize(TargetScreenSizeSetting),
    /// `w:saveSmartTagsAsXml`.
    SaveSmartTagsAsXml(Toggle),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl WebSettings {
    /// Builds a new, empty `w:webSettings`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "webSettings"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn local(item: &WebSettingsContent) -> Option<&'static str> {
        Some(match item {
            WebSettingsContent::Frameset(_) => "frameset",
            WebSettingsContent::Divs(_) => "divs",
            WebSettingsContent::Encoding(_) => "encoding",
            WebSettingsContent::OptimizeForBrowser(_) => "optimizeForBrowser",
            WebSettingsContent::RelyOnVml(_) => "relyOnVML",
            WebSettingsContent::AllowPng(_) => "allowPNG",
            WebSettingsContent::DoNotRelyOnCss(_) => "doNotRelyOnCSS",
            WebSettingsContent::DoNotSaveAsSingleFile(_) => "doNotSaveAsSingleFile",
            WebSettingsContent::DoNotOrganizeInFolder(_) => "doNotOrganizeInFolder",
            WebSettingsContent::DoNotUseLongFileNames(_) => "doNotUseLongFileNames",
            WebSettingsContent::PixelsPerInch(_) => "pixelsPerInch",
            WebSettingsContent::TargetScreenSize(_) => "targetScreenSz",
            WebSettingsContent::SaveSmartTagsAsXml(_) => "saveSmartTagsAsXml",
            WebSettingsContent::Raw(_) => return None,
        })
    }

    fn rank(item: &WebSettingsContent) -> Option<u16> {
        Self::local(item).and_then(|local| WEB_SETTINGS.rank_of(None, local))
    }

    fn remove(&mut self, is_target: impl Fn(&WebSettingsContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: WebSettingsContent) {
        let at = WEB_SETTINGS.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&WebSettingsContent) -> bool,
        value: Option<WebSettingsContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    super::property_macros::value_property!(WebSettingsContent, frameset, set_frameset, Frameset, Frameset, "frameset", "`w:frameset`.");
    super::property_macros::value_property!(WebSettingsContent, divs, set_divs, Divs, Divs, "divs", "`w:divs` — the `w:divId` (C4) resolution target.");
    super::property_macros::value_property!(WebSettingsContent, encoding, set_encoding, Encoding, StyleString, "encoding", "`w:encoding`.");
    super::property_macros::value_property!(WebSettingsContent, optimize_for_browser, set_optimize_for_browser, OptimizeForBrowser, OptimizeForBrowserSetting, "optimizeForBrowser", "`w:optimizeForBrowser`.");
    super::property_macros::toggle_property!(WebSettingsContent, rely_on_vml, set_rely_on_vml, RelyOnVml, "relyOnVML", "`w:relyOnVML`.");
    super::property_macros::toggle_property!(WebSettingsContent, allow_png, set_allow_png, AllowPng, "allowPNG", "`w:allowPNG`.");
    super::property_macros::toggle_property!(WebSettingsContent, do_not_rely_on_css, set_do_not_rely_on_css, DoNotRelyOnCss, "doNotRelyOnCSS", "`w:doNotRelyOnCSS`.");
    super::property_macros::toggle_property!(WebSettingsContent, do_not_save_as_single_file, set_do_not_save_as_single_file, DoNotSaveAsSingleFile, "doNotSaveAsSingleFile", "`w:doNotSaveAsSingleFile`.");
    super::property_macros::toggle_property!(WebSettingsContent, do_not_organize_in_folder, set_do_not_organize_in_folder, DoNotOrganizeInFolder, "doNotOrganizeInFolder", "`w:doNotOrganizeInFolder`.");
    super::property_macros::toggle_property!(WebSettingsContent, do_not_use_long_file_names, set_do_not_use_long_file_names, DoNotUseLongFileNames, "doNotUseLongFileNames", "`w:doNotUseLongFileNames`.");
    super::property_macros::value_property!(WebSettingsContent, pixels_per_inch, set_pixels_per_inch, PixelsPerInch, DecimalNumberValue, "pixelsPerInch", "`w:pixelsPerInch`.");
    super::property_macros::value_property!(WebSettingsContent, target_screen_size, set_target_screen_size, TargetScreenSize, TargetScreenSizeSetting, "targetScreenSz", "`w:targetScreenSz`.");
    super::property_macros::toggle_property!(WebSettingsContent, save_smart_tags_as_xml, set_save_smart_tags_as_xml, SaveSmartTagsAsXml, "saveSmartTagsAsXml", "`w:saveSmartTagsAsXml`.");
}
