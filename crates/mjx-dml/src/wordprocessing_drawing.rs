//! `wp:` — `dml-wordprocessingDrawing.xsd`, the schema `w:drawing` (`CT_Drawing`, `mjx-docx`) wraps:
//! a drawing's own placement (inline or anchored/floating), its wrap mode, and the small handful of
//! container shapes (`wp:graphicFrame`, `wp:wgp`, `wp:wpc`, `wp:contentPart`) that do not themselves
//! need WordprocessingML content to describe.
//!
//! **287 lines, 20 complexTypes, and — before this child — zero of them modeled anywhere in
//! `crates/`.** This module holds the ones that do not reach into WordprocessingML paragraph/table
//! content; three do and are modeled in `mjx-docx` instead — see "Where `wsp`/the text box live"
//! below, which is this child's own layering argument in full.
//!
//! # The 20 types, and where each one landed
//!
//! | XSD symbol | This module's name | Notes |
//! |---|---|---|
//! | `CT_EffectExtent` | [`EffectExtent`] | |
//! | `CT_Inline` | [`Inline`] | |
//! | `CT_Anchor` | [`Anchor`] | |
//! | `CT_PosH` | [`HorizontalPosition`] | |
//! | `CT_PosV` | [`VerticalPosition`] | |
//! | `CT_WrapNone` | [`WrapNone`] | |
//! | `CT_WrapSquare` | [`WrapSquare`] | |
//! | `CT_WrapTight` | [`WrapTight`] | |
//! | `CT_WrapThrough` | [`WrapThrough`] | |
//! | `CT_WrapTopBottom` | [`WrapTopAndBottom`] | |
//! | `CT_WrapPath` | [`WrapPath`] | |
//! | `CT_GraphicFrame` | [`WordDrawingFrame`] | an inline OLE-style frame; distinct from PowerPoint's own `p:graphicFrame` type |
//! | `CT_WordprocessingGroup` | [`WordprocessingGroup`] | member shapes preserved raw — see below |
//! | `CT_WordprocessingCanvas` | [`WordprocessingCanvas`] | member shapes preserved raw — see below |
//! | `CT_WordprocessingContentPart` | [`ContentPart`] | |
//! | `CT_WordprocessingContentPartNonVisual` | [`ContentPartNonVisual`] | |
//! | `CT_LinkedTextboxInformation` | [`LinkedTextboxInformation`] | |
//! | `CT_TextboxInfo` | `mjx_docx::document::TextboxInfo` | needs `CT_TxbxContent` — see below |
//! | `CT_TxbxContent` | `mjx_docx::document::TextBoxContent` | `EG_BlockLevelElts` — WordprocessingML content |
//! | `CT_WordprocessingShape` | `mjx_docx::document::WordprocessingShape` | optional `txbx` child needs `CT_TxbxContent` |
//!
//! # Where `wsp`/the text box live, and why
//!
//! `CT_TxbxContent`'s content is `w:EG_BlockLevelElts` — paragraphs, tables, the same vocabulary
//! [`crate`]'s own layering forbids it from naming (`mjx-dml` sits *below* `mjx-docx`; reaching for
//! `mjx-docx::document::Paragraph` from here would be the exact upward edge CLAUDE.md's layering
//! rule exists to catch). `CT_TextboxInfo` wraps a `CT_TxbxContent` directly, and
//! `CT_WordprocessingShape`'s own optional `txbx` child is a `CT_TextboxInfo` — so both are
//! transitively WordprocessingML-content-shaped too. All three live in `mjx-docx::document::drawing`
//! instead, reusing `mjx-docx`'s own `BlockContent`/`block_paragraph*` mechanism (MJXOFF-126's own
//! "extend, don't copy" instruction) for the text box's own paragraphs.
//!
//! [`WordprocessingGroup`] and [`WordprocessingCanvas`] make the same call **without** needing to
//! move: their own repeatable member choice (`wsp | grpSp | graphicFrame | pic | contentPart`) could
//! in principle be typed recursively, but doing so would either duplicate `CT_WordprocessingShape`'s
//! placement here (this module) *and* in `mjx-docx` (its real home), or force groups/canvases
//! themselves up into `mjx-docx` too — and grouped/canvas Word drawings are a materially rarer
//! feature this ticket's own "Done when" does not ask for. So their member content is preserved
//! **raw**, exactly as [`crate::geometry::CustomGeometry`] already keeps its own rarer internals
//! opaque: every byte round-trips, including a nested `wsp`, but nothing here decomposes it. A
//! caller that needs a group's own member shapes reaches them through the raw children directly.

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, Interner, Number, RawAttribute, RawElement, RawName,
    RawNode, Text, ToXml,
};
use mjx_ooxml_types::namespaces::DML_WORDPROCESSING_DRAWING;
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::wordprocessingdrawing::{
    HorizontalAlignment, HorizontalRelativeFrom, VerticalAlignment, VerticalRelativeFrom, WrapText,
};

use crate::codec::EmuCoordinate;
use crate::geometry::{Emu, Position, Size, Transform2D};
use crate::graphic::Graphic;
use crate::nonvisual::{
    NonVisualContentPartProperties, NonVisualDrawingProps, NonVisualGraphicFrameProperties,
};

/// Builds a `wp:local` qualified name.
fn wp_name(interner: &mut Interner, local: &str) -> RawName {
    RawName {
        prefix: Some(interner.intern("wp")),
        local: interner.intern(local),
        namespace: Some(interner.intern(DML_WORDPROCESSING_DRAWING.transitional)),
    }
}

/// Whether `name` is in the `wp:` namespace, matching both its Strict and Transitional URIs.
fn is_wp(name: &RawName, interner: &Interner) -> bool {
    let namespace = name.namespace.map(|symbol| interner.resolve(symbol));
    namespace == Some(DML_WORDPROCESSING_DRAWING.transitional)
        || namespace == DML_WORDPROCESSING_DRAWING.strict
}

/// The first `wp:`-namespaced element in `children` named `local`.
fn wp_child<'a>(
    children: &'a [RawNode],
    interner: &Interner,
    local: &str,
) -> Option<&'a RawElement> {
    children.iter().find_map(|node| match node {
        RawNode::Element(child)
            if is_wp(&child.name, interner) && interner.resolve(child.name.local) == local =>
        {
            Some(child)
        }
        _ => None,
    })
}

/// The `a:graphic` child of `children` — `a:` regardless of the host element's own `wp:` namespace,
/// since `a:graphic` is always DrawingML-main.
fn graphic_child<'a>(children: &'a [RawNode], interner: &Interner) -> Option<&'a RawElement> {
    children.iter().find_map(|node| match node {
        RawNode::Element(child) if interner.resolve(child.name.local) == "graphic" => Some(child),
        _ => None,
    })
}

// =================================================================================================
// CT_Point2D, read generically (wp:simplePos, wp:start, wp:lineTo all use it under a different name
// — the element is `wp:`-prefixed but the *type* behind it, `a:CT_Point2D`, is DrawingML-main's).
// =================================================================================================

#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "x", codec = EmuCoordinate, accessor = x, required))]
#[xml(attribute(local = "y", codec = EmuCoordinate, accessor = y, required))]
struct Point2DAttributes<A> {
    attributes: A,
}

/// Reads an `a:CT_Point2D`-shaped element. Never fails: an absent or malformed `x`/`y` reads as `0`
/// EMU, because a point this model cannot parse must still leave the file readable — the same
/// leniency [`crate::geometry::Transform2D::read`] applies to `a:off`.
fn read_point2d(element: &RawElement, interner: &Interner) -> Position {
    let attributes = Point2DAttributes {
        attributes: &element.attributes,
    };
    Position {
        x: attributes.x(interner).ok().unwrap_or(Emu::from_emu(0)),
        y: attributes.y(interner).ok().unwrap_or(Emu::from_emu(0)),
    }
}

fn point2d_element(interner: &mut Interner, local: &str, position: Position) -> RawElement {
    let mut attributes = Point2DAttributes {
        attributes: Vec::new(),
    };
    attributes.set_x(interner, position.x);
    attributes.set_y(interner, position.y);
    RawElement::new(
        wp_name(interner, local),
        attributes.attributes,
        Vec::new(),
        true,
    )
}

// =================================================================================================
// CT_EffectExtent (wp:effectExtent)
// =================================================================================================

/// `wp:effectExtent` (`CT_EffectExtent`) — the extra space a drawing's own effects (a shadow, a
/// glow) need beyond its plain extent, on each of the four sides. An attribute-only element (the
/// schema gives it no children at all), so — like [`super::TrackChangeMarker`] in `mjx-docx` — this
/// is `name`/`attributes`/`extra`/`empty` plus typed attribute accessors, not the `#[xml(children)]`
/// derive (which requires a content field this type has none of).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "l", codec = EmuCoordinate, accessor = left, required))]
#[xml(attribute(local = "t", codec = EmuCoordinate, accessor = top, required))]
#[xml(attribute(local = "r", codec = EmuCoordinate, accessor = right, required))]
#[xml(attribute(local = "b", codec = EmuCoordinate, accessor = bottom, required))]
pub struct EffectExtent {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

crate::build::fidelity_element_impls!(EffectExtent);

impl EffectExtent {
    /// Builds `<wp:effectExtent l="{left}" t="{top}" r="{right}" b="{bottom}"/>`.
    #[must_use]
    pub fn new(interner: &mut Interner, left: Emu, top: Emu, right: Emu, bottom: Emu) -> Self {
        let mut value = Self {
            name: wp_name(interner, "effectExtent"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        value.set_left(interner, left);
        value.set_top(interner, top);
        value.set_right(interner, right);
        value.set_bottom(interner, bottom);
        value
    }
}

fn read_effect_extent(children: &[RawNode], interner: &Interner) -> Option<EffectExtent> {
    wp_child(children, interner, "effectExtent")
        .and_then(|el| EffectExtent::from_xml(el, interner).ok())
}

// =================================================================================================
// CT_Inline (wp:inline)
// =================================================================================================

/// `wp:inline` (`CT_Inline`) — an inline drawing's own placement: extent, effect extent, non-visual
/// identity, then the `a:graphic` it wraps.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "distT", codec = EmuCoordinate, accessor = distance_top))]
#[xml(attribute(local = "distB", codec = EmuCoordinate, accessor = distance_bottom))]
#[xml(attribute(local = "distL", codec = EmuCoordinate, accessor = distance_left))]
#[xml(attribute(local = "distR", codec = EmuCoordinate, accessor = distance_right))]
pub struct Inline {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl Inline {
    /// Builds `<wp:inline><wp:extent cx="{width}" cy="{height}"/><wp:docPr .../>{graphic}</wp:inline>`.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        extent: Size,
        doc_properties: NonVisualDrawingProps,
        graphic: Graphic,
    ) -> Self {
        let mut children = vec![
            RawNode::Element(point2d_element(
                interner,
                "extent",
                Position {
                    x: extent.width,
                    y: extent.height,
                },
            )),
            RawNode::Element(doc_properties.to_xml(interner)),
        ];
        children.push(RawNode::Element(graphic.to_xml(interner)));
        Self {
            name: wp_name(interner, "inline"),
            attributes: Vec::new(),
            children,
            empty: false,
        }
    }

    /// The drawing's plain size (`wp:extent`) — `None` only when the element is malformed (the
    /// schema requires it).
    #[must_use]
    pub fn extent(&self, interner: &Interner) -> Option<Size> {
        wp_child(&self.children, interner, "extent").map(|el| {
            let position = read_point2d(el, interner);
            Size {
                width: position.x,
                height: position.y,
            }
        })
    }

    /// The extra space this drawing's own effects need (`wp:effectExtent`), or `None` if it declares
    /// none.
    #[must_use]
    pub fn effect_extent(&self, interner: &Interner) -> Option<EffectExtent> {
        read_effect_extent(&self.children, interner)
    }

    /// The drawing's own non-visual identity (`wp:docPr`) — `None` only when the element is
    /// malformed (the schema requires it).
    #[must_use]
    pub fn doc_properties(&self, interner: &Interner) -> Option<NonVisualDrawingProps> {
        wp_child(&self.children, interner, "docPr")
            .and_then(|el| NonVisualDrawingProps::from_xml(el, interner).ok())
    }

    /// The drawing's own non-visual graphic-frame properties (`wp:cNvGraphicFramePr`), or `None` if
    /// it declares none.
    #[must_use]
    pub fn frame_properties(&self, interner: &Interner) -> Option<NonVisualGraphicFrameProperties> {
        wp_child(&self.children, interner, "cNvGraphicFramePr")
            .and_then(|el| NonVisualGraphicFrameProperties::from_xml(el, interner).ok())
    }

    /// The `a:graphic` this drawing wraps — `None` only when the element is malformed (the schema
    /// requires it).
    #[must_use]
    pub fn graphic(&self, interner: &Interner) -> Option<Graphic> {
        graphic_child(&self.children, interner).and_then(|el| Graphic::from_xml(el, interner).ok())
    }
}

impl FromXml for Inline {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            children: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Inline {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        RawElement::rebuilt(
            self.name,
            self.attributes.clone(),
            self.children.clone(),
            false,
        )
    }
}

// =================================================================================================
// CT_PosH / CT_PosV (wp:positionH, wp:positionV)
// =================================================================================================

/// A simple-content leaf whose whole element body is text (`wp:align`, `wp:posOffset`) — the same
/// shape `mjx_docx::document::body::Text` uses for `w:t`/`w:delText`/…, reused here for a namespace
/// that crate cannot name. One struct serves both wire names via `with_local`, exactly as that type's
/// own doc comment explains for its own four.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = DML_WORDPROCESSING_DRAWING)]
pub struct TextLeaf {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(text)]
    text: String,
}

impl TextLeaf {
    /// Builds `<{local}>{text}</{local}>` (self-closing when `text` is empty).
    #[must_use]
    fn with_local(interner: &mut Interner, local: &str, text: &str) -> Self {
        Self {
            name: wp_name(interner, local),
            attributes: Vec::new(),
            empty: text.is_empty(),
            text: text.to_owned(),
        }
    }
}

/// A horizontal or vertical position's own value: an alignment keyword, or an explicit signed
/// offset in EMU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionValue<A> {
    /// `wp:align` — a named alignment (`ST_AlignH`/`ST_AlignV`).
    Align(A),
    /// `wp:posOffset` — an explicit offset from `relativeFrom`.
    Offset(Emu),
}

/// `wp:positionH` (`CT_PosH`) — a floating drawing's horizontal position: an alignment or an
/// explicit offset, relative to `@relativeFrom`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HorizontalPosition {
    relative_from: Option<HorizontalRelativeFrom>,
    value: Option<PositionValue<HorizontalAlignment>>,
}

impl HorizontalPosition {
    /// Builds a position relative to `relative_from`.
    #[must_use]
    pub fn new(
        relative_from: HorizontalRelativeFrom,
        value: PositionValue<HorizontalAlignment>,
    ) -> Self {
        Self {
            relative_from: Some(relative_from),
            value: Some(value),
        }
    }

    /// What this position is measured from (`@relativeFrom`) — `None` only when the element is
    /// malformed (the schema requires it).
    #[must_use]
    pub fn relative_from(&self) -> Option<HorizontalRelativeFrom> {
        self.relative_from
    }

    /// The alignment or offset itself — `None` only when the element is malformed (the schema
    /// requires exactly one of `wp:align`/`wp:posOffset`).
    #[must_use]
    pub fn value(&self) -> Option<PositionValue<HorizontalAlignment>> {
        self.value
    }
}

/// `wp:positionV` (`CT_PosV`) — the vertical counterpart of [`HorizontalPosition`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerticalPosition {
    relative_from: Option<VerticalRelativeFrom>,
    value: Option<PositionValue<VerticalAlignment>>,
}

impl VerticalPosition {
    /// Builds a position relative to `relative_from`.
    #[must_use]
    pub fn new(
        relative_from: VerticalRelativeFrom,
        value: PositionValue<VerticalAlignment>,
    ) -> Self {
        Self {
            relative_from: Some(relative_from),
            value: Some(value),
        }
    }

    /// What this position is measured from (`@relativeFrom`) — `None` only when the element is
    /// malformed (the schema requires it).
    #[must_use]
    pub fn relative_from(&self) -> Option<VerticalRelativeFrom> {
        self.relative_from
    }

    /// The alignment or offset itself — `None` only when the element is malformed (the schema
    /// requires exactly one of `wp:align`/`wp:posOffset`).
    #[must_use]
    pub fn value(&self) -> Option<PositionValue<VerticalAlignment>> {
        self.value
    }
}

#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "relativeFrom", codec = Enumeration<HorizontalRelativeFrom>, accessor = relative_from))]
struct PosHAttributes<A> {
    attributes: A,
}

#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "relativeFrom", codec = Enumeration<VerticalRelativeFrom>, accessor = relative_from))]
struct PosVAttributes<A> {
    attributes: A,
}

/// The typed value a `wp:align`/`wp:posOffset` leaf holds, generic over the alignment enum.
fn read_position_value<A: EnumeratedWireValue>(
    children: &[RawNode],
    interner: &Interner,
) -> Option<PositionValue<A>> {
    children.iter().find_map(|node| {
        let RawNode::Element(el) = node else {
            return None;
        };
        if !is_wp(&el.name, interner) {
            return None;
        }
        match interner.resolve(el.name.local) {
            "align" => {
                let leaf = TextLeaf::from_xml(el, interner).ok()?;
                A::from_wire_str(&leaf.text).map(PositionValue::Align)
            }
            "posOffset" => {
                let leaf = TextLeaf::from_xml(el, interner).ok()?;
                leaf.text
                    .trim()
                    .parse::<i64>()
                    .ok()
                    .map(|emu| PositionValue::Offset(Emu::from_emu(emu)))
            }
            _ => None,
        }
    })
}

/// A minimal bridge from the generated `from_wire`/`to_wire` methods (inherent, not a trait) to a
/// trait bound `read_position_value` can be generic over.
trait EnumeratedWireValue: Copy {
    fn from_wire_str(s: &str) -> Option<Self>;
    fn to_wire_str(self) -> &'static str;
}

impl EnumeratedWireValue for HorizontalAlignment {
    fn from_wire_str(s: &str) -> Option<Self> {
        Self::from_wire(s)
    }
    fn to_wire_str(self) -> &'static str {
        self.to_wire()
    }
}

impl EnumeratedWireValue for VerticalAlignment {
    fn from_wire_str(s: &str) -> Option<Self> {
        Self::from_wire(s)
    }
    fn to_wire_str(self) -> &'static str {
        self.to_wire()
    }
}

fn read_position_h(element: &RawElement, interner: &Interner) -> HorizontalPosition {
    let relative_from = PosHAttributes {
        attributes: &element.attributes,
    }
    .relative_from(interner)
    .ok()
    .flatten();
    HorizontalPosition {
        relative_from,
        value: read_position_value(&element.children, interner),
    }
}

fn read_position_v(element: &RawElement, interner: &Interner) -> VerticalPosition {
    let relative_from = PosVAttributes {
        attributes: &element.attributes,
    }
    .relative_from(interner)
    .ok()
    .flatten();
    VerticalPosition {
        relative_from,
        value: read_position_value(&element.children, interner),
    }
}

fn position_h_element(interner: &mut Interner, position: &HorizontalPosition) -> RawElement {
    let mut attributes = PosHAttributes {
        attributes: Vec::new(),
    };
    if let Some(relative_from) = position.relative_from {
        attributes.set_relative_from(interner, Some(relative_from));
    }
    let children = position_value_children(interner, position.value);
    RawElement::new(
        wp_name(interner, "positionH"),
        attributes.attributes,
        children,
        position.value.is_none(),
    )
}

fn position_v_element(interner: &mut Interner, position: &VerticalPosition) -> RawElement {
    let mut attributes = PosVAttributes {
        attributes: Vec::new(),
    };
    if let Some(relative_from) = position.relative_from {
        attributes.set_relative_from(interner, Some(relative_from));
    }
    let children = position_value_children(interner, position.value);
    RawElement::new(
        wp_name(interner, "positionV"),
        attributes.attributes,
        children,
        position.value.is_none(),
    )
}

fn position_value_children<A: EnumeratedWireValue>(
    interner: &mut Interner,
    value: Option<PositionValue<A>>,
) -> Vec<RawNode> {
    let Some(value) = value else {
        return Vec::new();
    };
    let leaf = match value {
        PositionValue::Align(a) => TextLeaf::with_local(interner, "align", a.to_wire_str()),
        PositionValue::Offset(emu) => {
            TextLeaf::with_local(interner, "posOffset", &emu.emu().to_string())
        }
    };
    vec![RawNode::Element(leaf.to_xml(interner))]
}

// =================================================================================================
// The five wrap modes (EG_WrapType) + CT_WrapPath
// =================================================================================================

/// `wp:wrapNone` (`CT_WrapNone`) — no text wrapping; the drawing floats over/under text with nothing
/// reflowing around it. An empty marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapNone {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

crate::build::fidelity_element_impls!(WrapNone);

impl WrapNone {
    /// Builds a self-closing `<wp:wrapNone/>`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wp_name(interner, "wrapNone"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }
}

/// `wp:wrapSquare` (`CT_WrapSquare`) — text wraps around the drawing's bounding box.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "wrapText", codec = Enumeration<WrapText>, accessor = wrap_text, required))]
#[xml(attribute(local = "distT", codec = EmuCoordinate, accessor = distance_top))]
#[xml(attribute(local = "distB", codec = EmuCoordinate, accessor = distance_bottom))]
#[xml(attribute(local = "distL", codec = EmuCoordinate, accessor = distance_left))]
#[xml(attribute(local = "distR", codec = EmuCoordinate, accessor = distance_right))]
pub struct WrapSquare {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

crate::build::fidelity_element_impls!(WrapSquare);

impl WrapSquare {
    /// The extra space this wrap's own effects need (`wp:effectExtent`), or `None` if it declares
    /// none.
    #[must_use]
    pub fn effect_extent(&self, interner: &Interner) -> Option<EffectExtent> {
        read_effect_extent(&self.children, interner)
    }
}

/// `wp:wrapTopAndBottom` (`CT_WrapTopBottom`) — text wraps above and below the drawing only.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "distT", codec = EmuCoordinate, accessor = distance_top))]
#[xml(attribute(local = "distB", codec = EmuCoordinate, accessor = distance_bottom))]
pub struct WrapTopAndBottom {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

crate::build::fidelity_element_impls!(WrapTopAndBottom);

impl WrapTopAndBottom {
    /// The extra space this wrap's own effects need (`wp:effectExtent`), or `None` if it declares
    /// none.
    #[must_use]
    pub fn effect_extent(&self, interner: &Interner) -> Option<EffectExtent> {
        read_effect_extent(&self.children, interner)
    }
}

/// `wp:start`/`wp:lineTo`, then `wp:wrapPolygon` itself (`CT_WrapPath`) — the outline text wraps
/// tightly or all the way through, as a closed polygon.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "edited", codec = OnOff, accessor = edited))]
pub struct WrapPath {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

crate::build::fidelity_element_impls!(WrapPath);

impl WrapPath {
    /// Builds `<wp:wrapPolygon><wp:start .../>{lines}</wp:wrapPolygon>` — `lines` must hold at least
    /// two points (the schema requires `wp:lineTo` `minOccurs="2"`).
    #[must_use]
    pub fn new(interner: &mut Interner, start: Position, lines: &[Position]) -> Self {
        let mut children = vec![RawNode::Element(point2d_element(interner, "start", start))];
        children.extend(
            lines
                .iter()
                .map(|point| RawNode::Element(point2d_element(interner, "lineTo", *point))),
        );
        Self {
            name: wp_name(interner, "wrapPolygon"),
            attributes: Vec::new(),
            children,
            empty: false,
        }
    }

    /// The polygon's starting point (`wp:start`) — `None` only when the element is malformed (the
    /// schema requires it).
    #[must_use]
    pub fn start(&self, interner: &Interner) -> Option<Position> {
        wp_child(&self.children, interner, "start").map(|el| read_point2d(el, interner))
    }

    /// The polygon's remaining points, in order (`wp:lineTo`, two or more per the schema).
    #[must_use]
    pub fn line_to(&self, interner: &Interner) -> Vec<Position> {
        self.children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(el)
                    if is_wp(&el.name, interner) && interner.resolve(el.name.local) == "lineTo" =>
                {
                    Some(read_point2d(el, interner))
                }
                _ => None,
            })
            .collect()
    }
}

/// `wp:wrapTight`/`wp:wrapThrough` (`CT_WrapTight`/`CT_WrapThrough`) — text wraps to the drawing's
/// own outline (`wp:wrapPolygon`) rather than its bounding box; `wrapThrough` additionally lets text
/// fill any interior holes the polygon encloses. The two schema types are structurally identical
/// (`wrapPolygon`, `@wrapText` required, `@distL`/`@distR` optional — neither carries `@distT`/
/// `@distB`, unlike [`WrapSquare`]), so one Rust type serves both, distinguished by
/// [`Wrap::Tight`]/[`Wrap::Through`]'s own wire-element name.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "wrapText", codec = Enumeration<WrapText>, accessor = wrap_text, required))]
#[xml(attribute(local = "distL", codec = EmuCoordinate, accessor = distance_left))]
#[xml(attribute(local = "distR", codec = EmuCoordinate, accessor = distance_right))]
pub struct WrapOutline {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

crate::build::fidelity_element_impls!(WrapOutline);

impl WrapOutline {
    /// Builds `<{local} wrapText="{wrap_text}"><wp:wrapPolygon>...</wp:wrapPolygon></{local}>` —
    /// `local` is `"wrapTight"` or `"wrapThrough"`.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        local: &str,
        wrap_text: WrapText,
        polygon: WrapPath,
    ) -> Self {
        let mut value = Self {
            name: wp_name(interner, local),
            attributes: Vec::new(),
            children: vec![RawNode::Element(polygon.to_xml(interner))],
            empty: false,
        };
        value.set_wrap_text(interner, wrap_text);
        value
    }

    /// The wrap's own outline (`wp:wrapPolygon`) — `None` only when the element is malformed (the
    /// schema requires it).
    #[must_use]
    pub fn polygon(&self, interner: &Interner) -> Option<WrapPath> {
        wp_child(&self.children, interner, "wrapPolygon")
            .and_then(|el| WrapPath::from_xml(el, interner).ok())
    }
}

/// `EG_WrapType` — the five wrap modes a floating (anchored) drawing chooses exactly one of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wrap {
    /// `wp:wrapNone`.
    None(WrapNone),
    /// `wp:wrapSquare`.
    Square(WrapSquare),
    /// `wp:wrapTight`.
    Tight(WrapOutline),
    /// `wp:wrapThrough`.
    Through(WrapOutline),
    /// `wp:wrapTopAndBottom`.
    TopAndBottom(WrapTopAndBottom),
}

impl Wrap {
    fn from_children(children: &[RawNode], interner: &Interner) -> Option<Self> {
        for node in children {
            let RawNode::Element(el) = node else { continue };
            if !is_wp(&el.name, interner) {
                continue;
            }
            return match interner.resolve(el.name.local) {
                "wrapNone" => WrapNone::from_xml(el, interner).ok().map(Wrap::None),
                "wrapSquare" => WrapSquare::from_xml(el, interner).ok().map(Wrap::Square),
                "wrapTight" => WrapOutline::from_xml(el, interner).ok().map(Wrap::Tight),
                "wrapThrough" => WrapOutline::from_xml(el, interner).ok().map(Wrap::Through),
                "wrapTopAndBottom" => WrapTopAndBottom::from_xml(el, interner)
                    .ok()
                    .map(Wrap::TopAndBottom),
                _ => continue,
            };
        }
        None
    }

    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        match self {
            Wrap::None(wrap) => wrap.to_xml(interner),
            Wrap::Square(wrap) => wrap.to_xml(interner),
            Wrap::Tight(wrap) => wrap.to_xml(interner),
            Wrap::Through(wrap) => wrap.to_xml(interner),
            Wrap::TopAndBottom(wrap) => wrap.to_xml(interner),
        }
    }
}

// =================================================================================================
// CT_Anchor (wp:anchor)
// =================================================================================================

/// `wp:anchor` (`CT_Anchor`) — a floating drawing's own placement: simple position, horizontal and
/// vertical position, extent, effect extent, wrap mode, non-visual identity, then the `a:graphic` it
/// wraps.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "distT", codec = EmuCoordinate, accessor = distance_top))]
#[xml(attribute(local = "distB", codec = EmuCoordinate, accessor = distance_bottom))]
#[xml(attribute(local = "distL", codec = EmuCoordinate, accessor = distance_left))]
#[xml(attribute(local = "distR", codec = EmuCoordinate, accessor = distance_right))]
#[xml(attribute(local = "simplePos", codec = OnOff, accessor = simple_pos_enabled))]
#[xml(attribute(local = "relativeHeight", codec = Number<u32>, accessor = relative_height, required))]
#[xml(attribute(local = "behindDoc", codec = OnOff, accessor = behind_doc, required))]
#[xml(attribute(local = "locked", codec = OnOff, accessor = locked, required))]
#[xml(attribute(local = "layoutInCell", codec = OnOff, accessor = layout_in_cell, required))]
#[xml(attribute(local = "hidden", codec = OnOff, accessor = hidden))]
#[xml(attribute(local = "allowOverlap", codec = OnOff, accessor = allow_overlap, required))]
pub struct Anchor {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl Anchor {
    /// The drawing's simple position (`wp:simplePos`) — only meaningful when
    /// [`Anchor::simple_pos_enabled`] is `Some(true)`; a schema-required child regardless.
    #[must_use]
    pub fn simple_pos(&self, interner: &Interner) -> Option<Position> {
        wp_child(&self.children, interner, "simplePos").map(|el| read_point2d(el, interner))
    }

    /// The drawing's horizontal position (`wp:positionH`) — `None` only when the element is
    /// malformed (the schema requires it).
    #[must_use]
    pub fn position_horizontal(&self, interner: &Interner) -> Option<HorizontalPosition> {
        wp_child(&self.children, interner, "positionH").map(|el| read_position_h(el, interner))
    }

    /// The drawing's vertical position (`wp:positionV`) — `None` only when the element is malformed
    /// (the schema requires it).
    #[must_use]
    pub fn position_vertical(&self, interner: &Interner) -> Option<VerticalPosition> {
        wp_child(&self.children, interner, "positionV").map(|el| read_position_v(el, interner))
    }

    /// The drawing's plain size (`wp:extent`) — `None` only when the element is malformed (the
    /// schema requires it).
    #[must_use]
    pub fn extent(&self, interner: &Interner) -> Option<Size> {
        wp_child(&self.children, interner, "extent").map(|el| {
            let position = read_point2d(el, interner);
            Size {
                width: position.x,
                height: position.y,
            }
        })
    }

    /// The extra space this drawing's own effects need (`wp:effectExtent`), or `None` if it declares
    /// none.
    #[must_use]
    pub fn effect_extent(&self, interner: &Interner) -> Option<EffectExtent> {
        read_effect_extent(&self.children, interner)
    }

    /// The drawing's own wrap mode (`EG_WrapType`) — `None` only when the element is malformed (the
    /// schema requires exactly one).
    #[must_use]
    pub fn wrap(&self, interner: &Interner) -> Option<Wrap> {
        Wrap::from_children(&self.children, interner)
    }

    /// The drawing's own non-visual identity (`wp:docPr`) — `None` only when the element is
    /// malformed (the schema requires it).
    #[must_use]
    pub fn doc_properties(&self, interner: &Interner) -> Option<NonVisualDrawingProps> {
        wp_child(&self.children, interner, "docPr")
            .and_then(|el| NonVisualDrawingProps::from_xml(el, interner).ok())
    }

    /// The drawing's own non-visual graphic-frame properties (`wp:cNvGraphicFramePr`), or `None` if
    /// it declares none.
    #[must_use]
    pub fn frame_properties(&self, interner: &Interner) -> Option<NonVisualGraphicFrameProperties> {
        wp_child(&self.children, interner, "cNvGraphicFramePr")
            .and_then(|el| NonVisualGraphicFrameProperties::from_xml(el, interner).ok())
    }

    /// The `a:graphic` this drawing wraps — `None` only when the element is malformed (the schema
    /// requires it).
    #[must_use]
    pub fn graphic(&self, interner: &Interner) -> Option<Graphic> {
        graphic_child(&self.children, interner).and_then(|el| Graphic::from_xml(el, interner).ok())
    }

    /// Builds a floating drawing: `simple_pos` is written both as the `wp:simplePos` element (always
    /// required, per the schema) and, when `simple_pos_enabled` is `true`, as the attribute that asks
    /// a consumer to honour it over `positionH`/`positionV`.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        interner: &mut Interner,
        simple_pos: Position,
        simple_pos_enabled: bool,
        position_horizontal: HorizontalPosition,
        position_vertical: VerticalPosition,
        extent: Size,
        wrap: Wrap,
        relative_height: u32,
        behind_doc: bool,
        locked: bool,
        layout_in_cell: bool,
        allow_overlap: bool,
        doc_properties: NonVisualDrawingProps,
        graphic: Graphic,
    ) -> Self {
        let mut children = vec![
            RawNode::Element(point2d_element(interner, "simplePos", simple_pos)),
            RawNode::Element(position_h_element(interner, &position_horizontal)),
            RawNode::Element(position_v_element(interner, &position_vertical)),
            RawNode::Element(point2d_element(
                interner,
                "extent",
                Position {
                    x: extent.width,
                    y: extent.height,
                },
            )),
            RawNode::Element(wrap.to_xml(interner)),
            RawNode::Element(doc_properties.to_xml(interner)),
        ];
        children.push(RawNode::Element(graphic.to_xml(interner)));
        let mut value = Self {
            name: wp_name(interner, "anchor"),
            attributes: Vec::new(),
            children,
            empty: false,
        };
        value.set_simple_pos_enabled(interner, Some(simple_pos_enabled));
        value.set_relative_height(interner, relative_height);
        value.set_behind_doc(interner, behind_doc);
        value.set_locked(interner, locked);
        value.set_layout_in_cell(interner, layout_in_cell);
        value.set_allow_overlap(interner, allow_overlap);
        value
    }
}

impl FromXml for Anchor {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            children: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Anchor {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        RawElement::rebuilt(
            self.name,
            self.attributes.clone(),
            self.children.clone(),
            false,
        )
    }
}

// =================================================================================================
// CT_LinkedTextboxInformation (wp:linkedTxbx)
// =================================================================================================

/// `wp:linkedTxbx` (`CT_LinkedTextboxInformation`) — a shape's text box overflows into another
/// shape's; this names that other shape's own text-box chain id/sequence rather than carrying content
/// of its own. A fidelity wrapper: `id`/`seq` are typed; `a:extLst` (its only possible child) is kept
/// opaque so the element round-trips byte-for-byte.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", codec = Number<u16>, accessor = id, required))]
#[xml(attribute(local = "seq", codec = Number<u16>, accessor = sequence, required))]
pub struct LinkedTextboxInformation {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

crate::build::fidelity_element_impls!(LinkedTextboxInformation);

// =================================================================================================
// CT_GraphicFrame (wp:graphicFrame) — an inline OLE-style frame, distinct from PowerPoint's own type
// of the same XSD symbol in a different namespace.
// =================================================================================================

/// `wp:graphicFrame` (`CT_GraphicFrame`) — an inline graphic frame: non-visual identity, non-visual
/// frame properties, transform, then the `a:graphic` it wraps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordDrawingFrame {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

crate::build::fidelity_element_impls!(WordDrawingFrame);

impl WordDrawingFrame {
    /// The frame's own non-visual identity (`wp:cNvPr`) — `None` only when the element is malformed
    /// (the schema requires it).
    #[must_use]
    pub fn doc_properties(&self, interner: &Interner) -> Option<NonVisualDrawingProps> {
        wp_child(&self.children, interner, "cNvPr")
            .and_then(|el| NonVisualDrawingProps::from_xml(el, interner).ok())
    }

    /// The frame's own non-visual frame properties (`wp:cNvFrPr`) — `None` only when the element is
    /// malformed (the schema requires it).
    #[must_use]
    pub fn frame_properties(&self, interner: &Interner) -> Option<NonVisualGraphicFrameProperties> {
        wp_child(&self.children, interner, "cNvFrPr")
            .and_then(|el| NonVisualGraphicFrameProperties::from_xml(el, interner).ok())
    }

    /// The frame's own transform (`wp:xfrm`) — `None` only when the element is malformed (the schema
    /// requires it).
    #[must_use]
    pub fn transform(&self, interner: &Interner) -> Option<Transform2D> {
        wp_child(&self.children, interner, "xfrm").map(|el| Transform2D::read(el, interner))
    }

    /// The `a:graphic` this frame wraps — `None` only when the element is malformed (the schema
    /// requires it).
    #[must_use]
    pub fn graphic(&self, interner: &Interner) -> Option<Graphic> {
        graphic_child(&self.children, interner).and_then(|el| Graphic::from_xml(el, interner).ok())
    }
}

// =================================================================================================
// CT_WordprocessingGroup / CT_WordprocessingCanvas — member shapes preserved raw; see module doc.
// =================================================================================================

/// `wp:wgp` (`CT_WordprocessingGroup`) — a group of Word shapes. Its own member choice (`wsp`/
/// `grpSp`/`graphicFrame`/`pic`/`contentPart`) is preserved as raw children rather than typed
/// recursively — see this module's own doc comment for why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordprocessingGroup {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

crate::build::fidelity_element_impls!(WordprocessingGroup);

/// `wp:wpc` (`CT_WordprocessingCanvas`) — a drawing canvas. Its own member choice is preserved as raw
/// children, exactly as [`WordprocessingGroup`]'s is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordprocessingCanvas {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

crate::build::fidelity_element_impls!(WordprocessingCanvas);

// =================================================================================================
// CT_WordprocessingContentPart / CT_WordprocessingContentPartNonVisual (ink)
// =================================================================================================

/// `wp:contentPart` (`CT_WordprocessingContentPart`) — a reference to an ink (digital pen) content
/// part: non-visual identity, then an optional transform.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "bwMode", codec = Text, accessor = black_and_white_mode_token))]
#[xml(attribute(local = "id", prefix = "r", codec = Text, accessor = relationship_id, required))]
pub struct ContentPart {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

crate::build::fidelity_element_impls!(ContentPart);

impl ContentPart {
    /// The content part's own non-visual identity (`wp:nvContentPartPr`), or `None` if it declares
    /// none.
    #[must_use]
    pub fn non_visual(&self, interner: &Interner) -> Option<ContentPartNonVisual> {
        wp_child(&self.children, interner, "nvContentPartPr")
            .and_then(|el| ContentPartNonVisual::from_xml(el, interner).ok())
    }

    /// The content part's own transform (`wp:xfrm`), or `None` if it declares none.
    #[must_use]
    pub fn transform(&self, interner: &Interner) -> Option<Transform2D> {
        wp_child(&self.children, interner, "xfrm").map(|el| Transform2D::read(el, interner))
    }
}

/// `wp:nvContentPartPr` (`CT_WordprocessingContentPartNonVisual`) — an ink content part's own
/// identity (`wp:cNvPr`) and lock list (`wp:cNvContentPartPr`), both optional.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentPartNonVisual {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

crate::build::fidelity_element_impls!(ContentPartNonVisual);

impl ContentPartNonVisual {
    /// The content part's own identity (`wp:cNvPr`), or `None` if it declares none.
    #[must_use]
    pub fn doc_properties(&self, interner: &Interner) -> Option<NonVisualDrawingProps> {
        wp_child(&self.children, interner, "cNvPr")
            .and_then(|el| NonVisualDrawingProps::from_xml(el, interner).ok())
    }

    /// The content part's own lock list (`wp:cNvContentPartPr`), or `None` if it declares none.
    #[must_use]
    pub fn content_part_properties(
        &self,
        interner: &Interner,
    ) -> Option<NonVisualContentPartProperties> {
        wp_child(&self.children, interner, "cNvContentPartPr")
            .and_then(|el| NonVisualContentPartProperties::from_xml(el, interner).ok())
    }
}
