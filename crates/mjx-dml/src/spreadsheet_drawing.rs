//! `xdr:` — `dml-spreadsheetDrawing.xsd`, the schema `xl/drawings/drawingN.xml` is rooted in:
//! everything that appears on a sheet but is not a cell, and the three ways it can be pinned to the
//! grid underneath it.
//!
//! # Where this lives, and why it is here rather than in `mjx-sml`
//!
//! `xdr` is a **DrawingML** schema, not a SpreadsheetML one: seventeen complex types, of which
//! twelve exist only to wrap something `dml-main.xsd` already declares
//! (`a:CT_ShapeProperties`, `a:CT_TextBody`, `a:CT_BlipFillProperties`, `a:CT_Transform2D`,
//! `a:graphic`). It sits here for the same reason [`crate::wordprocessing_drawing`] does — a
//! DrawingML satellite schema whose content is DrawingML, hosted by a format crate that reaches it
//! from above. `mjx-sml` (rank 2.1) sits above this crate and resolves an anchor against a sheet's
//! own column widths and row heights; `mjx-xlsx` (rank 3.0) owns the part, its relationship and the
//! image parts it names. Nothing here knows what a package is.
//!
//! # The seventeen types, and where each one landed
//!
//! | XSD symbol | This module's name | Notes |
//! |---|---|---|
//! | `CT_Drawing` | [`WorksheetDrawing`] | the part root, `xdr:wsDr` |
//! | `CT_TwoCellAnchor` | [`TwoCellAnchor`] | |
//! | `CT_OneCellAnchor` | [`OneCellAnchor`] | |
//! | `CT_AbsoluteAnchor` | [`AbsoluteAnchor`] | |
//! | `CT_Marker` | [`CellMarker`] | `xdr:from` / `xdr:to` |
//! | `CT_AnchorClientData` | [`AnchorClientData`] | **both attributes default to `true`** |
//! | `CT_Shape` | [`DrawingShape`] | `xdr:sp` |
//! | `CT_Picture` | [`DrawingPicture`] | `xdr:pic` |
//! | `CT_Connector` | [`DrawingConnector`] | `xdr:cxnSp` |
//! | `CT_GraphicalObjectFrame` | [`DrawingGraphicFrame`] | `xdr:graphicFrame` |
//! | `CT_GroupShape` | [`DrawingGroupShape`] | `xdr:grpSp`; member shapes preserved raw |
//! | `CT_Rel` | [`DrawingContentPart`] | `xdr:contentPart` |
//! | `CT_ShapeNonVisual`, `CT_PictureNonVisual`, `CT_ConnectorNonVisual`, `CT_GroupShapeNonVisual`, `CT_GraphicalObjectFrameNonVisual` | — | reached through [`AnchoredObject::identity`]; each is a two-child wrapper around `a:CT_NonVisualDrawingProps` plus one locking type [`crate::nonvisual`] already models |
//!
//! # Every type here is a fidelity wrapper
//!
//! The same shape as [`crate::shape_properties::ShapeProperties`]: the storage is the element's own
//! `attributes`/`children` lists, typed accessors read out of them, and a **new** child is placed
//! through the generated [`mjx_ooxml_types::child_order`] table for its type. So an
//! `xdr:twoCellAnchor` carrying a `mc:AlternateContent`, an `extLst`, or an `xdr:cxnSp` this module
//! does not decompose re-emits byte for byte, and modelling something only ever adds reach.
//!
//! # The trap in `xdr:clientData`
//!
//! `CT_AnchorClientData`'s two attributes are `xsd:boolean` with **`default="true"`** — the same
//! shape MJXOFF-127 recorded for `CT_ObjectPr`'s six. An anchor that writes no `clientData`
//! attributes at all locks *and* prints with the sheet, so a reader that assumed the family-wide
//! `false` reports both of them backwards. [`AnchorClientData::locks_with_sheet`] and
//! [`AnchorClientData::prints_with_sheet`] answer `true` for an attribute the file does not write,
//! and the pair is declared to the attribute derive with `default = true` so that the default is
//! stated once, beside the wire name, rather than remembered at each call site.
//!
//! # `editAs` is not the element name
//!
//! `xdr:twoCellAnchor@editAs` defaults to `twoCell`, but a producer is free to write
//! `editAs="oneCell"` on a `twoCellAnchor` — Apache POI does exactly that for
//! [`ClientAnchor.AnchorType.MOVE_DONT_RESIZE`], and `tests/fixtures/worksheet_drawings.xlsx`
//! carries one. The attribute says what happens to the object when the cells under it move; the
//! element name says how the object's *geometry* is stated. They are different questions, and
//! [`TwoCellAnchor::resizing_behavior`] answers the first from the attribute alone.
//!
//! [`ClientAnchor.AnchorType.MOVE_DONT_RESIZE`]: https://poi.apache.org/apidocs/dev/org/apache/poi/ss/usermodel/ClientAnchor.AnchorType.html

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, Interner, RawAttribute, RawElement, RawName, RawNode, Text,
    ToXml,
};
use mjx_ooxml_types::child_order::{
    ABSOLUTE_ANCHOR, CELL_MARKER, ONE_CELL_ANCHOR, SHEET_DRAWING_CONNECTOR,
    SHEET_DRAWING_GRAPHIC_FRAME, SHEET_DRAWING_PICTURE, SHEET_DRAWING_SHAPE, TWO_CELL_ANCHOR,
};
use mjx_ooxml_types::namespaces::{DML_MAIN, DML_SPREADSHEET_DRAWING};
use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;
use mjx_ooxml_types::support::OnOff;

use crate::build::fidelity_element_impls;
use crate::fill::{PictureFill, PictureFillMode};
use crate::geometry::{Emu, Position, Size, Transform2D};
use crate::graphic::Graphic;
use crate::nonvisual::NonVisualDrawingProps;
use crate::shape_properties::ShapeProperties;
use crate::text::TextBody;

/// The prefix every element this module *builds* is bound to. A file is read by resolved namespace,
/// never by prefix; this is only what an authored element is spelled with.
const XDR_PREFIX: &str = "xdr";

/// Builds an `xdr:local` qualified name in the SpreadsheetDrawingML namespace.
#[must_use]
pub fn xdr_name(interner: &mut Interner, local: &str) -> RawName {
    RawName {
        prefix: Some(interner.intern(XDR_PREFIX)),
        local: interner.intern(local),
        namespace: Some(interner.intern(DML_SPREADSHEET_DRAWING.transitional)),
    }
}

/// Whether `name` is in the `xdr:` namespace, matching both its Strict and Transitional URIs.
#[must_use]
fn is_xdr(name: &RawName, interner: &Interner) -> bool {
    let namespace = name.namespace.map(|symbol| interner.resolve(symbol));
    namespace == Some(DML_SPREADSHEET_DRAWING.transitional)
        || namespace == DML_SPREADSHEET_DRAWING.strict
}

/// Whether `name` is in the DrawingML-main namespace, matching both URIs.
fn is_dml_main(name: &RawName, interner: &Interner) -> bool {
    let namespace = name.namespace.map(|symbol| interner.resolve(symbol));
    namespace == Some(DML_MAIN.transitional) || namespace == DML_MAIN.strict
}

/// The first `xdr:`-namespaced element in `children` named `local`.
fn xdr_child<'a>(
    children: &'a [RawNode],
    interner: &Interner,
    local: &str,
) -> Option<&'a RawElement> {
    children.iter().find_map(|node| match node {
        RawNode::Element(child)
            if is_xdr(&child.name, interner) && interner.resolve(child.name.local) == local =>
        {
            Some(child)
        }
        _ => None,
    })
}

/// The first `xdr:`-namespaced element in `children` named `local`, mutably — so an edit reaches
/// through it in place and every attribute and child this module does not model survives.
fn xdr_child_mut<'a>(
    children: &'a mut [RawNode],
    interner: &Interner,
    local: &str,
) -> Option<&'a mut RawElement> {
    children.iter_mut().find_map(|node| match node {
        RawNode::Element(child)
            if is_xdr(&child.name, interner) && interner.resolve(child.name.local) == local =>
        {
            Some(child)
        }
        _ => None,
    })
}

/// The first `a:`-namespaced element in `children` named `local` — `a:graphic` under an
/// `xdr:graphicFrame`, `a:xfrm` under it too. `xdr:`'s own children are `xdr:`-namespaced even where
/// their *types* are DrawingML-main's, so the two searches are not interchangeable.
fn dml_main_child<'a>(
    children: &'a [RawNode],
    interner: &Interner,
    local: &str,
) -> Option<&'a RawElement> {
    children.iter().find_map(|node| match node {
        RawNode::Element(child)
            if is_dml_main(&child.name, interner)
                && interner.resolve(child.name.local) == local =>
        {
            Some(child)
        }
        _ => None,
    })
}

/// The decoded text of an element whose content model is a simple type (`xdr:col`, `xdr:row`, …).
fn element_text(element: &RawElement) -> Option<&str> {
    element.children.iter().find_map(|node| match node {
        RawNode::Text(bytes) | RawNode::CData(bytes) => std::str::from_utf8(bytes).ok(),
        _ => None,
    })
}

/// Replaces `element`'s text content with `text`, keeping any comment or processing instruction it
/// carried — a marker child is `<xdr:col>3</xdr:col>` and nothing else in every file this project
/// has read, but discarding what a producer put there would be a change nobody asked for.
fn set_element_text(element: &mut RawElement, text: &str) {
    let replacement = RawNode::Text(Box::from(text.as_bytes()));
    let existing = element
        .children
        .iter()
        .position(|node| matches!(node, RawNode::Text(_) | RawNode::CData(_)));
    match existing {
        Some(at) => element.children[at] = replacement,
        None => element.children.push(replacement),
    }
    element.empty = false;
}

// =================================================================================================
// CT_Marker — `xdr:from` / `xdr:to`
// =================================================================================================

/// `xdr:from` / `xdr:to` (`CT_Marker`, §20.5.2.15 / §20.5.2.32) — one anchor point, as a
/// **(column, offset-within-that-column, row, offset-within-that-row)** quadruple.
///
/// `colOff`/`rowOff` are offsets *within* the cell the marker names, measured from its top-left
/// corner — **not** from the sheet origin, which is what makes an anchor survive a column being
/// widened. Both are EMU (`a:ST_Coordinate`); `col` and `row` are **zero-based** indices
/// (`xdr:ST_ColID`/`ST_RowID`, `xsd:int` restricted to non-negative), so `col="0"` is column `A` and
/// `row="0"` is the row a file numbers `1`.
///
/// A value type rather than a fidelity wrapper: `CT_Marker` declares no attribute and exactly four
/// required children, and every edit goes back through [`apply`](Self::apply), which rewrites those
/// four children *in place* and leaves anything else the element carried where it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellMarker {
    /// `xdr:col` — the zero-based column index.
    pub column: i32,
    /// `xdr:colOff` — the offset into that column, from its left edge.
    pub column_offset: Emu,
    /// `xdr:row` — the zero-based row index.
    pub row: i32,
    /// `xdr:rowOff` — the offset into that row, from its top edge.
    pub row_offset: Emu,
}

impl CellMarker {
    /// A marker naming a cell and an offset into it.
    #[must_use]
    pub const fn new(column: i32, column_offset: i64, row: i32, row_offset: i64) -> Self {
        Self {
            column,
            column_offset: Emu::from_emu(column_offset),
            row,
            row_offset: Emu::from_emu(row_offset),
        }
    }

    /// Reads a marker out of an `xdr:from`/`xdr:to` element.
    ///
    /// `None` when any of the four children is absent or does not parse. All four are
    /// `minOccurs="1"` with no default, so a marker missing one names no cell at all, and inventing
    /// a zero for it would be inventing a position — the same reason
    /// [`Transform2D`] answers `None` for a partial transform.
    #[must_use]
    pub fn read(element: &RawElement, interner: &Interner) -> Option<Self> {
        let number = |local: &str| -> Option<i64> {
            element_text(xdr_child(&element.children, interner, local)?)?
                .trim()
                .parse::<i64>()
                .ok()
        };
        Some(Self {
            column: i32::try_from(number("col")?).ok()?,
            column_offset: Emu::from_emu(number("colOff")?),
            row: i32::try_from(number("row")?).ok()?,
            row_offset: Emu::from_emu(number("rowOff")?),
        })
    }

    /// Writes this marker into an `xdr:from`/`xdr:to` element, editing each of the four children in
    /// place when it is there and inserting it at its rank in `CT_Marker`'s `xsd:sequence` when it
    /// is not.
    ///
    /// The ranks come from the generated [`CELL_MARKER`] table, never from this file: `col`,
    /// `colOff`, `row` and `rowOff` are two pairs of the same shape, and the sequence position is
    /// the only thing that says a `colOff` may not follow a `row`.
    pub fn apply(&self, element: &mut RawElement, interner: &mut Interner) {
        for (local, value) in [
            ("col", i64::from(self.column)),
            ("colOff", self.column_offset.emu()),
            ("row", i64::from(self.row)),
            ("rowOff", self.row_offset.emu()),
        ] {
            let text = value.to_string();
            if let Some(child) = xdr_child_mut(&mut element.children, interner, local) {
                set_element_text(child, &text);
                continue;
            }
            let name = xdr_name(interner, local);
            let mut child = RawElement::new(name, Vec::new(), Vec::new(), false);
            set_element_text(&mut child, &text);
            CELL_MARKER.insert(&mut element.children, interner, child);
        }
        element.empty = false;
    }

    /// A fresh `xdr:{local}` element carrying this marker.
    #[must_use]
    pub fn to_element(self, interner: &mut Interner, local: &str) -> RawElement {
        let name = xdr_name(interner, local);
        let mut element = RawElement::new(name, Vec::new(), Vec::new(), false);
        self.apply(&mut element, interner);
        element
    }
}

// =================================================================================================
// CT_AnchorClientData — `xdr:clientData`
// =================================================================================================

/// `xdr:clientData` (`CT_AnchorClientData`, §20.5.2.3) — the two flags the spreadsheet application
/// itself acts on: whether the object is selectable while the sheet is protected, and whether it
/// prints.
///
/// **Both attributes are `xsd:boolean` with `default="true"`.** That is the opposite of nearly every
/// other flag in the formats this workspace reads, and an anchor whose `clientData` is written
/// `<xdr:clientData/>` — which is what Excel, LibreOffice and Apache POI all emit — is claiming
/// both. See this module's own documentation.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "fLocksWithSheet", codec = OnOff, accessor = locks_with_sheet_attribute, default = true))]
#[xml(attribute(local = "fPrintsWithSheet", codec = OnOff, accessor = prints_with_sheet_attribute, default = true))]
pub struct AnchorClientData {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl AnchorClientData {
    /// Builds `<xdr:clientData/>` — the spelling every producer writes, and the one that means
    /// *locks with the sheet and prints with the sheet*.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: xdr_name(interner, "clientData"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }

    /// Whether selection of this object is disabled while the sheet is protected
    /// (`@fLocksWithSheet`). **`true` when the attribute is absent.**
    #[must_use]
    pub fn locks_with_sheet(&self, interner: &Interner) -> bool {
        self.locks_with_sheet_attribute(interner).unwrap_or(true)
    }

    /// Whether this object is printed with the sheet (`@fPrintsWithSheet`). **`true` when the
    /// attribute is absent.**
    #[must_use]
    pub fn prints_with_sheet(&self, interner: &Interner) -> bool {
        self.prints_with_sheet_attribute(interner).unwrap_or(true)
    }
}

fidelity_element_impls!(AnchorClientData);

// =================================================================================================
// EG_ObjectChoices — the six things an anchor can hold
// =================================================================================================

/// Declares one anchored-object fidelity wrapper: the element by name, its typed attributes, and
/// the `nv*Pr` wrapper its identity (`xdr:cNvPr`) hangs under.
///
/// Six element kinds share these bodies. Writing them out six times would be six chances to reach
/// for the wrong `nv*Pr` local name, which is exactly the kind of mistake a reader of the file could
/// not see.
macro_rules! anchored_object {
    // The five kinds that carry at least one typed attribute.
    (
        $(#[$meta:meta])*
        $ty:ident, $local:literal, $non_visual:literal,
        attributes: [$(#[$attr:meta])+]
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
        $(#[$attr])+
        pub struct $ty {
            name: RawName,
            attributes: Vec<RawAttribute>,
            children: Vec<RawNode>,
            empty: bool,
        }

        anchored_object!(@body $ty, $local, $non_visual);
    };

    // The two that carry none: `xdr:grpSp` (whose only attributes are its members') and
    // `xdr:contentPart`, whose one attribute is an `r:id` and so is prefixed rather than typed.
    (
        $(#[$meta:meta])*
        $ty:ident, $local:literal, $non_visual:literal, attributes: []
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $ty {
            name: RawName,
            attributes: Vec<RawAttribute>,
            children: Vec<RawNode>,
            empty: bool,
        }

        anchored_object!(@body $ty, $local, $non_visual);
    };

    (@body $ty:ident, $local:literal, $non_visual:literal) => {
        impl $ty {
            /// The wire local name of this object's element.
            pub const LOCAL: &'static str = $local;

            /// The local name of the non-visual wrapper this object's identity hangs under.
            pub const NON_VISUAL_LOCAL: &'static str = $non_visual;

            /// This object's identity (`xdr:cNvPr` inside its own non-visual wrapper): the id and
            /// name a consumer shows, the alt text, and the hidden flag.
            ///
            /// `None` when the object writes no non-visual block, or one with no `cNvPr` — both are
            /// schema-invalid (`minOccurs="1"` twice over) and neither is repaired here.
            #[must_use]
            pub fn identity(&self, interner: &Interner) -> Option<NonVisualDrawingProps> {
                let wrapper = xdr_child(&self.children, interner, $non_visual)?;
                let props = xdr_child(&wrapper.children, interner, "cNvPr")?;
                NonVisualDrawingProps::from_xml(props, interner).ok()
            }

            /// The element as this model holds it, for a caller that needs the raw subtree.
            #[must_use]
            pub fn children(&self) -> &[RawNode] {
                &self.children
            }
        }

        fidelity_element_impls!($ty);
    };
}

anchored_object! {
    /// `xdr:sp` (`CT_Shape`, §20.5.2.29) — a shape drawn on the sheet: a preset or custom geometry,
    /// its fill and outline, and optionally text.
    ///
    /// `@fLocksText` is `xsd:boolean` with `default="true"` — the same trap `xdr:clientData` carries
    /// — so [`locks_text`](Self::locks_text) answers `true` for a shape that writes no attribute.
    /// `@fPublished` defaults to `false`, and `@macro`/`@textlink` are application-defined strings
    /// carried through exactly as written.
    DrawingShape, "sp", "nvSpPr",
    attributes: [
    #[xml(attribute(local = "macro", codec = Text, accessor = macro_reference))]
    #[xml(attribute(local = "textlink", codec = Text, accessor = text_link))]
    #[xml(attribute(local = "fLocksText", codec = OnOff, accessor = locks_text_attribute, default = true))]
    #[xml(attribute(local = "fPublished", codec = OnOff, accessor = is_published, default = false))]
    ]
}

anchored_object! {
    /// `xdr:pic` (`CT_Picture`, §20.5.2.25) — an image on the sheet: its identity, the
    /// `a:blipFill` naming the image part, and its shape properties.
    DrawingPicture, "pic", "nvPicPr",
    attributes: [
    #[xml(attribute(local = "macro", codec = Text, accessor = macro_reference))]
    #[xml(attribute(local = "fPublished", codec = OnOff, accessor = is_published, default = false))]
    ]
}

anchored_object! {
    /// `xdr:cxnSp` (`CT_Connector`, §20.5.2.13) — a line or connector between two other shapes.
    DrawingConnector, "cxnSp", "nvCxnSpPr",
    attributes: [
    #[xml(attribute(local = "macro", codec = Text, accessor = macro_reference))]
    #[xml(attribute(local = "fPublished", codec = OnOff, accessor = is_published, default = false))]
    ]
}

anchored_object! {
    /// `xdr:graphicFrame` (`CT_GraphicalObjectFrame`, §20.5.2.16) — the frame a chart, a diagram or
    /// a table is drawn in. **What goes inside it is MJXOFF-111's (E4)**; this reports the frame,
    /// its transform and the `a:graphic` reference, and decomposes neither.
    DrawingGraphicFrame, "graphicFrame", "nvGraphicFramePr",
    attributes: [
    #[xml(attribute(local = "macro", codec = Text, accessor = macro_reference))]
    #[xml(attribute(local = "fPublished", codec = OnOff, accessor = is_published, default = false))]
    ]
}

anchored_object! {
    /// `xdr:grpSp` (`CT_GroupShape`, §20.5.2.17) — several objects moved and sized as one.
    ///
    /// Its member shapes are preserved **raw**, exactly as
    /// [`WordprocessingGroup`](crate::wordprocessing_drawing::WordprocessingGroup)'s are and for the
    /// same reason: typing them recursively would either duplicate every object model inside itself
    /// or force the group's members into a second representation. Every byte round-trips, including
    /// a nested `xdr:grpSp`; nothing here decomposes one. [`members`](Self::members) hands back the
    /// raw child elements a caller needs.
    DrawingGroupShape, "grpSp", "nvGrpSpPr", attributes: []
}

anchored_object! {
    /// `xdr:contentPart` (`CT_Rel`, §20.5.2.12) — a reference to XML in a format ECMA-376 does not
    /// define (MathML, SMIL, SVG, InkML). One required `r:id`, and nothing else.
    DrawingContentPart, "contentPart", "contentPart", attributes: []
}

impl DrawingShape {
    /// Whether text inside this shape can still be edited while the sheet is protected
    /// (`@fLocksText`). **`true` when the attribute is absent** — see this type's own documentation.
    #[must_use]
    pub fn locks_text(&self, interner: &Interner) -> bool {
        self.locks_text_attribute(interner).unwrap_or(true)
    }

    /// This shape's visual properties (`xdr:spPr`) — transform, geometry, fill, outline, effects.
    #[must_use]
    pub fn shape_properties(&self, interner: &Interner) -> Option<ShapeProperties> {
        xdr_child(&self.children, interner, "spPr")
            .and_then(|element| ShapeProperties::from_xml(element, interner).ok())
    }

    /// This shape's text (`xdr:txBody`), or `None` when it carries none.
    #[must_use]
    pub fn text_body(&self, interner: &Interner) -> Option<TextBody> {
        xdr_child(&self.children, interner, "txBody")
            .and_then(|element| TextBody::from_xml(element, interner).ok())
    }

    /// Replaces this shape's visual properties, editing the existing `xdr:spPr` where it stands or
    /// inserting one at its rank in `CT_Shape`'s `xsd:sequence`.
    pub fn set_shape_properties(&mut self, interner: &mut Interner, properties: &ShapeProperties) {
        let mut element = properties.to_xml(interner);
        element.name = xdr_name(interner, "spPr");
        SHEET_DRAWING_SHAPE.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "spPr"
        });
        self.empty = false;
    }
}

impl DrawingPicture {
    /// This picture's image fill (`xdr:blipFill`), which holds the `a:blip@r:embed` relationship
    /// identifier naming the image part.
    #[must_use]
    pub fn fill(&self, interner: &Interner) -> Option<PictureFill> {
        xdr_child(&self.children, interner, "blipFill")
            .and_then(|element| PictureFill::from_xml(element, interner).ok())
    }

    /// The relationship identifier of the image this picture shows
    /// (`xdr:blipFill/a:blip@r:embed`), or `None` when it links an external image (`@r:link`) or
    /// names neither.
    ///
    /// **Resolving the identifier to a part is `mjx-xlsx`'s** — this crate knows nothing about
    /// packages, exactly as [`crate::picture::Picture`] does not.
    #[must_use]
    pub fn image_relationship_id(&self, interner: &Interner) -> Option<String> {
        self.fill(interner)?.image_rel_id(interner)
    }

    /// This picture's visual properties (`xdr:spPr`) — its transform, crop geometry and outline.
    #[must_use]
    pub fn shape_properties(&self, interner: &Interner) -> Option<ShapeProperties> {
        xdr_child(&self.children, interner, "spPr")
            .and_then(|element| ShapeProperties::from_xml(element, interner).ok())
    }

    /// Replaces this picture's visual properties, editing the existing `xdr:spPr` where it stands or
    /// inserting one at its rank in `CT_Picture`'s `xsd:sequence`.
    pub fn set_shape_properties(&mut self, interner: &mut Interner, properties: &ShapeProperties) {
        let mut element = properties.to_xml(interner);
        element.name = xdr_name(interner, "spPr");
        SHEET_DRAWING_PICTURE.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "spPr"
        });
        self.empty = false;
    }
}

impl DrawingConnector {
    /// This connector's visual properties (`xdr:spPr`).
    #[must_use]
    pub fn shape_properties(&self, interner: &Interner) -> Option<ShapeProperties> {
        xdr_child(&self.children, interner, "spPr")
            .and_then(|element| ShapeProperties::from_xml(element, interner).ok())
    }

    /// Replaces this connector's visual properties, at its rank in `CT_Connector`'s sequence.
    pub fn set_shape_properties(&mut self, interner: &mut Interner, properties: &ShapeProperties) {
        let mut element = properties.to_xml(interner);
        element.name = xdr_name(interner, "spPr");
        SHEET_DRAWING_CONNECTOR.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "spPr"
        });
        self.empty = false;
    }
}

impl DrawingGraphicFrame {
    /// The frame's own transform (`xdr:xfrm`, whose type is `a:CT_Transform2D`), or `None` when it
    /// writes none.
    #[must_use]
    pub fn transform(&self, interner: &Interner) -> Option<Transform2D> {
        xdr_child(&self.children, interner, "xfrm")
            .map(|element| Transform2D::read(element, interner))
    }

    /// Sets the frame's transform, editing the existing `xdr:xfrm` in place or inserting one at its
    /// rank in `CT_GraphicalObjectFrame`'s sequence.
    pub fn set_transform(&mut self, interner: &mut Interner, transform: Transform2D) {
        if let Some(element) = xdr_child_mut(&mut self.children, interner, "xfrm") {
            transform.apply(element, interner);
            self.empty = false;
            return;
        }
        let mut element = Transform2D::empty_element(interner);
        element.name = xdr_name(interner, "xfrm");
        transform.apply(&mut element, interner);
        SHEET_DRAWING_GRAPHIC_FRAME.insert(&mut self.children, interner, element);
        self.empty = false;
    }

    /// The graphical object this frame draws (`a:graphic` — DrawingML-main's own namespace, not
    /// `xdr:`), or `None` when the frame writes none.
    ///
    /// **What is inside it is not decomposed here**: a chart reference is MJXOFF-111's (E4).
    /// [`GraphicData::chart_relationship_id`](crate::graphic::GraphicData::chart_relationship_id)
    /// is what reads one.
    #[must_use]
    pub fn graphic(&self, interner: &Interner) -> Option<Graphic> {
        dml_main_child(&self.children, interner, "graphic")
            .and_then(|element| Graphic::from_xml(element, interner).ok())
    }
}

impl DrawingGroupShape {
    /// The group's member objects, as the raw elements the file wrote.
    ///
    /// Raw rather than typed: see this type's own documentation. A caller that wants one decoded
    /// hands it to [`AnchoredObject::read`].
    pub fn members<'a>(
        &'a self,
        interner: &'a Interner,
    ) -> impl Iterator<Item = &'a RawElement> + 'a {
        self.children.iter().filter_map(move |node| match node {
            RawNode::Element(child)
                if is_xdr(&child.name, interner)
                    && AnchoredObject::is_object_local(interner.resolve(child.name.local)) =>
            {
                Some(child)
            }
            _ => None,
        })
    }

    /// The group's own transform and fill (`xdr:grpSpPr`, an `a:CT_GroupShapeProperties`), as the
    /// raw element — the child coordinate space a member shape's own transform is stated in.
    #[must_use]
    pub fn group_shape_properties<'a>(&'a self, interner: &'a Interner) -> Option<&'a RawElement> {
        xdr_child(&self.children, interner, "grpSpPr")
    }
}

impl DrawingContentPart {
    /// The `r:id` this content part names, under `reference_prefix` — the prefix the part binds to
    /// the relationship-reference namespace.
    ///
    /// `None` when the attribute is absent. Resolving it to a part is `mjx-xlsx`'s.
    #[must_use]
    pub fn relationship_id(&self, interner: &Interner, reference_prefix: &str) -> Option<String> {
        self.attributes.iter().find_map(|attribute| {
            let prefix = attribute
                .name
                .prefix
                .map(|symbol| interner.resolve(symbol))?;
            (prefix == reference_prefix && interner.resolve(attribute.name.local) == "id")
                .then(|| String::from_utf8_lossy(&attribute.value).into_owned())
        })
    }
}

/// The `EG_ObjectChoices` group: the one thing an anchor holds.
///
/// A closed choice of six in the schema, and a closed enum here — an anchor holding something else
/// is not an anchored object at all, and [`Anchor::object`] answers `None` for it while the anchor
/// itself still round-trips byte for byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchoredObject {
    /// `xdr:sp` — a shape.
    Shape(DrawingShape),
    /// `xdr:grpSp` — a group of objects.
    GroupShape(DrawingGroupShape),
    /// `xdr:graphicFrame` — the frame a chart, diagram or table is drawn in.
    GraphicFrame(DrawingGraphicFrame),
    /// `xdr:cxnSp` — a connector.
    Connector(DrawingConnector),
    /// `xdr:pic` — a picture.
    Picture(DrawingPicture),
    /// `xdr:contentPart` — XML in a format ECMA-376 does not define.
    ContentPart(DrawingContentPart),
}

impl AnchoredObject {
    /// Whether `local` is one of the six element names `EG_ObjectChoices` allows.
    #[must_use]
    pub fn is_object_local(local: &str) -> bool {
        matches!(
            local,
            "sp" | "grpSp" | "graphicFrame" | "cxnSp" | "pic" | "contentPart"
        )
    }

    /// Reads an anchored object from the element the anchor holds, or `None` when the element is
    /// not one of the six.
    #[must_use]
    pub fn read(element: &RawElement, interner: &Interner) -> Option<Self> {
        if !is_xdr(&element.name, interner) {
            return None;
        }
        Some(match interner.resolve(element.name.local) {
            "sp" => Self::Shape(DrawingShape::from_xml(element, interner).ok()?),
            "grpSp" => Self::GroupShape(DrawingGroupShape::from_xml(element, interner).ok()?),
            "graphicFrame" => {
                Self::GraphicFrame(DrawingGraphicFrame::from_xml(element, interner).ok()?)
            }
            "cxnSp" => Self::Connector(DrawingConnector::from_xml(element, interner).ok()?),
            "pic" => Self::Picture(DrawingPicture::from_xml(element, interner).ok()?),
            "contentPart" => {
                Self::ContentPart(DrawingContentPart::from_xml(element, interner).ok()?)
            }
            _ => return None,
        })
    }

    /// This object's wire local name.
    #[must_use]
    pub fn local(&self) -> &'static str {
        match self {
            Self::Shape(_) => DrawingShape::LOCAL,
            Self::GroupShape(_) => DrawingGroupShape::LOCAL,
            Self::GraphicFrame(_) => DrawingGraphicFrame::LOCAL,
            Self::Connector(_) => DrawingConnector::LOCAL,
            Self::Picture(_) => DrawingPicture::LOCAL,
            Self::ContentPart(_) => DrawingContentPart::LOCAL,
        }
    }

    /// This object's identity (`xdr:cNvPr`) — the id and name a consumer shows.
    ///
    /// `None` for a [`ContentPart`](Self::ContentPart), which has no non-visual block at all: it is
    /// `CT_Rel`, one `r:id` and nothing else.
    #[must_use]
    pub fn identity(&self, interner: &Interner) -> Option<NonVisualDrawingProps> {
        match self {
            Self::Shape(value) => value.identity(interner),
            Self::GroupShape(value) => value.identity(interner),
            Self::GraphicFrame(value) => value.identity(interner),
            Self::Connector(value) => value.identity(interner),
            Self::Picture(value) => value.identity(interner),
            Self::ContentPart(_) => None,
        }
    }
}

impl ToXml for AnchoredObject {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        match self {
            Self::Shape(value) => value.to_xml(interner),
            Self::GroupShape(value) => value.to_xml(interner),
            Self::GraphicFrame(value) => value.to_xml(interner),
            Self::Connector(value) => value.to_xml(interner),
            Self::Picture(value) => value.to_xml(interner),
            Self::ContentPart(value) => value.to_xml(interner),
        }
    }
}

// =================================================================================================
// EG_Anchor — the three ways an object is pinned to the grid
// =================================================================================================

/// Reads an `xdr:pos` / `xdr:ext` child, whose *types* are `a:CT_Point2D` / `a:CT_PositiveSize2D`
/// but whose *elements* are declared inside `dml-spreadsheetDrawing.xsd` and are therefore `xdr:`.
fn read_point_pair(
    children: &[RawNode],
    interner: &Interner,
    local: &str,
    first: &str,
    second: &str,
) -> Option<(Emu, Emu)> {
    let element = xdr_child(children, interner, local)?;
    let read = |wanted: &str| -> Option<Emu> {
        element.attributes.iter().find_map(|attribute| {
            (attribute.name.prefix.is_none() && interner.resolve(attribute.name.local) == wanted)
                .then(|| {
                    std::str::from_utf8(&attribute.value)
                        .ok()?
                        .trim()
                        .parse::<i64>()
                        .ok()
                        .map(Emu::from_emu)
                })
                .flatten()
        })
    };
    Some((read(first)?, read(second)?))
}

/// Writes the two attributes of an `xdr:pos` / `xdr:ext` child, editing the element in place when
/// there is one and inserting it at `order`'s rank when there is not.
fn write_point_pair(
    children: &mut Vec<RawNode>,
    interner: &mut Interner,
    order: &mjx_ooxml_types::child_order::ChildOrder,
    local: &str,
    values: [(&str, Emu); 2],
) {
    if let Some(element) = xdr_child_mut(children, interner, local) {
        for (attribute, value) in values {
            mjx_xml::attribute::set(
                &mut element.attributes,
                interner,
                None,
                attribute,
                &value.emu().to_string(),
            );
        }
        return;
    }
    let name = xdr_name(interner, local);
    let mut element = RawElement::new(name, Vec::new(), Vec::new(), true);
    for (attribute, value) in values {
        mjx_xml::attribute::set(
            &mut element.attributes,
            interner,
            None,
            attribute,
            &value.emu().to_string(),
        );
    }
    order.insert(children, interner, element);
}

/// Declares the parts every anchor shares: the object it holds, its `xdr:clientData`, and the
/// setters that place a replacement at its rank in that anchor's own `xsd:sequence`.
macro_rules! anchor_body {
    ($ty:ident, $order:ident, $local:literal) => {
        impl $ty {
            /// The wire local name of this anchor's element.
            pub const LOCAL: &'static str = $local;

            /// The object this anchor holds, or `None` when it holds nothing this schema names.
            ///
            /// `EG_ObjectChoices` is `minOccurs="1"`, so `None` means the anchor is schema-invalid
            /// — which is a fact about the file, reported rather than repaired.
            #[must_use]
            pub fn object(&self, interner: &Interner) -> Option<AnchoredObject> {
                self.children.iter().find_map(|node| match node {
                    RawNode::Element(child) => AnchoredObject::read(child, interner),
                    _ => None,
                })
            }

            /// Replaces the object this anchor holds, keeping its position among the children when
            /// there is one and inserting the new one at `EG_ObjectChoices`' rank when there is not.
            pub fn set_object(&mut self, interner: &mut Interner, object: &AnchoredObject) {
                let element = object.to_xml(interner);
                $order.replace_or_insert(
                    &mut self.children,
                    interner,
                    element,
                    AnchoredObject::is_object_local,
                );
                self.empty = false;
            }

            /// This anchor's `xdr:clientData` — whether the object locks and prints with the sheet.
            ///
            /// `None` when the anchor writes none, which is schema-invalid
            /// (`minOccurs="1"`). It is **not** the same as an empty `<xdr:clientData/>`, which
            /// states both flags by their `true` defaults.
            #[must_use]
            pub fn client_data(&self, interner: &Interner) -> Option<AnchorClientData> {
                xdr_child(&self.children, interner, "clientData")
                    .and_then(|element| AnchorClientData::from_xml(element, interner).ok())
            }

            /// Replaces this anchor's `xdr:clientData`, at its rank in the sequence — last, after
            /// the object.
            pub fn set_client_data(&mut self, interner: &mut Interner, data: &AnchorClientData) {
                let mut element = data.to_xml(interner);
                element.name = xdr_name(interner, "clientData");
                $order.replace_or_insert(&mut self.children, interner, element, |local| {
                    local == "clientData"
                });
                self.empty = false;
            }

            /// This anchor's children, as the file wrote them.
            #[must_use]
            pub fn children(&self) -> &[RawNode] {
                &self.children
            }
        }

        fidelity_element_impls!($ty);
    };
}

/// `xdr:twoCellAnchor` (`CT_TwoCellAnchor`, §20.5.2.33) — an object pinned to **two** cells: its
/// top-left corner to the `from` marker and its bottom-right corner to the `to` marker.
///
/// The object has no extent of its own; the two markers *are* its geometry, which is why widening a
/// column between them makes the object wider. `@editAs` says what should happen when they move —
/// and it is an attribute, not an inference from the element name: see this module's own
/// documentation.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "editAs", codec = Enumeration<ResizingBehavior>, accessor = resizing_behavior_attribute))]
pub struct TwoCellAnchor {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

anchor_body!(TwoCellAnchor, TWO_CELL_ANCHOR, "twoCellAnchor");

impl TwoCellAnchor {
    /// The top-left anchor point (`xdr:from`), or `None` when the anchor writes none or one that
    /// does not parse.
    #[must_use]
    pub fn from_marker(&self, interner: &Interner) -> Option<CellMarker> {
        CellMarker::read(xdr_child(&self.children, interner, "from")?, interner)
    }

    /// The bottom-right anchor point (`xdr:to`).
    #[must_use]
    pub fn to_marker(&self, interner: &Interner) -> Option<CellMarker> {
        CellMarker::read(xdr_child(&self.children, interner, "to")?, interner)
    }

    /// Sets the top-left anchor point, editing the existing `xdr:from` in place.
    pub fn set_from_marker(&mut self, interner: &mut Interner, marker: CellMarker) {
        self.set_marker(interner, "from", marker);
    }

    /// Sets the bottom-right anchor point, editing the existing `xdr:to` in place.
    pub fn set_to_marker(&mut self, interner: &mut Interner, marker: CellMarker) {
        self.set_marker(interner, "to", marker);
    }

    fn set_marker(&mut self, interner: &mut Interner, local: &str, marker: CellMarker) {
        if let Some(element) = xdr_child_mut(&mut self.children, interner, local) {
            marker.apply(element, interner);
        } else {
            let element = marker.to_element(interner, local);
            TWO_CELL_ANCHOR.insert(&mut self.children, interner, element);
        }
        self.empty = false;
    }

    /// What this anchor promises to do to its object when the rows and columns under it move
    /// (`@editAs`).
    ///
    /// **`MoveAndResizeWithAnchorCells` when the attribute is absent** — `ST_EditAs`'s schema
    /// default is `twoCell` — and whatever the attribute says when it is present, which need not
    /// agree with the element's name. A producer writing `editAs="oneCell"` on a `twoCellAnchor` is
    /// saying *keep this object's size and move only its top-left corner*, and that is a shape
    /// Apache POI writes for every `MOVE_DONT_RESIZE` anchor.
    #[must_use]
    pub fn resizing_behavior(&self, interner: &Interner) -> ResizingBehavior {
        self.resizing_behavior_attribute(interner)
            .ok()
            .flatten()
            .unwrap_or(ResizingBehavior::MoveAndResizeWithAnchorCells)
    }
}

/// `xdr:oneCellAnchor` (`CT_OneCellAnchor`, §20.5.2.24) — an object pinned to **one** cell by its
/// top-left corner, carrying its own extent.
///
/// It moves with the cell it names and keeps its size, which is what `ST_EditAs`'s `oneCell` value
/// describes; unlike a two-cell anchor it has no `@editAs`, because there is nothing else it could
/// do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneCellAnchor {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

anchor_body!(OneCellAnchor, ONE_CELL_ANCHOR, "oneCellAnchor");

impl OneCellAnchor {
    /// The top-left anchor point (`xdr:from`).
    #[must_use]
    pub fn from_marker(&self, interner: &Interner) -> Option<CellMarker> {
        CellMarker::read(xdr_child(&self.children, interner, "from")?, interner)
    }

    /// Sets the top-left anchor point, editing the existing `xdr:from` in place.
    pub fn set_from_marker(&mut self, interner: &mut Interner, marker: CellMarker) {
        if let Some(element) = xdr_child_mut(&mut self.children, interner, "from") {
            marker.apply(element, interner);
        } else {
            let element = marker.to_element(interner, "from");
            ONE_CELL_ANCHOR.insert(&mut self.children, interner, element);
        }
        self.empty = false;
    }

    /// The object's own size (`xdr:ext`), in EMU.
    #[must_use]
    pub fn extent(&self, interner: &Interner) -> Option<Size> {
        let (width, height) = read_point_pair(&self.children, interner, "ext", "cx", "cy")?;
        Some(Size { width, height })
    }

    /// Sets the object's own size, editing the existing `xdr:ext` in place.
    pub fn set_extent(&mut self, interner: &mut Interner, size: Size) {
        write_point_pair(
            &mut self.children,
            interner,
            ONE_CELL_ANCHOR,
            "ext",
            [("cx", size.width), ("cy", size.height)],
        );
        self.empty = false;
    }
}

/// `xdr:absoluteAnchor` (`CT_AbsoluteAnchor`, §20.5.2.1) — an object pinned to the **sheet**, at an
/// absolute position and size in EMU.
///
/// It names no cell at all, so nothing that happens to the rows and columns changes what this
/// element says. That is the whole of `ST_EditAs`'s `absolute` behaviour, and it is why inserting a
/// row above such an object edits nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsoluteAnchor {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

anchor_body!(AbsoluteAnchor, ABSOLUTE_ANCHOR, "absoluteAnchor");

impl AbsoluteAnchor {
    /// The object's top-left corner (`xdr:pos`), measured from the sheet origin.
    #[must_use]
    pub fn position(&self, interner: &Interner) -> Option<Position> {
        let (x, y) = read_point_pair(&self.children, interner, "pos", "x", "y")?;
        Some(Position { x, y })
    }

    /// Sets the object's top-left corner, editing the existing `xdr:pos` in place.
    pub fn set_position(&mut self, interner: &mut Interner, position: Position) {
        write_point_pair(
            &mut self.children,
            interner,
            ABSOLUTE_ANCHOR,
            "pos",
            [("x", position.x), ("y", position.y)],
        );
        self.empty = false;
    }

    /// The object's size (`xdr:ext`), in EMU.
    #[must_use]
    pub fn extent(&self, interner: &Interner) -> Option<Size> {
        let (width, height) = read_point_pair(&self.children, interner, "ext", "cx", "cy")?;
        Some(Size { width, height })
    }

    /// Sets the object's size, editing the existing `xdr:ext` in place.
    pub fn set_extent(&mut self, interner: &mut Interner, size: Size) {
        write_point_pair(
            &mut self.children,
            interner,
            ABSOLUTE_ANCHOR,
            "ext",
            [("cx", size.width), ("cy", size.height)],
        );
        self.empty = false;
    }
}

/// The `EG_Anchor` choice: the three ways an object is pinned to a sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anchor {
    /// `xdr:twoCellAnchor` — pinned to two cells, so the cells give it its size.
    TwoCell(TwoCellAnchor),
    /// `xdr:oneCellAnchor` — pinned to one cell, carrying its own size.
    OneCell(OneCellAnchor),
    /// `xdr:absoluteAnchor` — pinned to the sheet, naming no cell.
    Absolute(AbsoluteAnchor),
}

impl Anchor {
    /// Whether `local` is one of the three element names `EG_Anchor` allows.
    #[must_use]
    pub fn is_anchor_local(local: &str) -> bool {
        matches!(local, "twoCellAnchor" | "oneCellAnchor" | "absoluteAnchor")
    }

    /// Reads an anchor from one child of `xdr:wsDr`, or `None` for anything else.
    #[must_use]
    pub fn read(element: &RawElement, interner: &Interner) -> Option<Self> {
        if !is_xdr(&element.name, interner) {
            return None;
        }
        Some(match interner.resolve(element.name.local) {
            "twoCellAnchor" => Self::TwoCell(TwoCellAnchor::from_xml(element, interner).ok()?),
            "oneCellAnchor" => Self::OneCell(OneCellAnchor::from_xml(element, interner).ok()?),
            "absoluteAnchor" => Self::Absolute(AbsoluteAnchor::from_xml(element, interner).ok()?),
            _ => return None,
        })
    }

    /// This anchor's wire local name.
    #[must_use]
    pub fn local(&self) -> &'static str {
        match self {
            Self::TwoCell(_) => TwoCellAnchor::LOCAL,
            Self::OneCell(_) => OneCellAnchor::LOCAL,
            Self::Absolute(_) => AbsoluteAnchor::LOCAL,
        }
    }

    /// What this anchor does to its object when the rows and columns under it move.
    ///
    /// The **element** decides it for a one-cell and an absolute anchor, which carry no `@editAs`
    /// at all; a two-cell anchor's own attribute decides it, and defaults to
    /// `MoveAndResizeWithAnchorCells`.
    #[must_use]
    pub fn resizing_behavior(&self, interner: &Interner) -> ResizingBehavior {
        match self {
            Self::TwoCell(anchor) => anchor.resizing_behavior(interner),
            Self::OneCell(_) => ResizingBehavior::MoveWithCellsButDoNotResize,
            Self::Absolute(_) => ResizingBehavior::DoNotMoveOrResizeWithRowsOrColumns,
        }
    }

    /// The object this anchor holds.
    #[must_use]
    pub fn object(&self, interner: &Interner) -> Option<AnchoredObject> {
        match self {
            Self::TwoCell(anchor) => anchor.object(interner),
            Self::OneCell(anchor) => anchor.object(interner),
            Self::Absolute(anchor) => anchor.object(interner),
        }
    }

    /// This anchor's `xdr:clientData`.
    #[must_use]
    pub fn client_data(&self, interner: &Interner) -> Option<AnchorClientData> {
        match self {
            Self::TwoCell(anchor) => anchor.client_data(interner),
            Self::OneCell(anchor) => anchor.client_data(interner),
            Self::Absolute(anchor) => anchor.client_data(interner),
        }
    }
}

impl ToXml for Anchor {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        match self {
            Self::TwoCell(value) => value.to_xml(interner),
            Self::OneCell(value) => value.to_xml(interner),
            Self::Absolute(value) => value.to_xml(interner),
        }
    }
}

// =================================================================================================
// CT_Drawing — `xdr:wsDr`, the part root
// =================================================================================================

/// Which axis a shift is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    Rows,
    Columns,
}

/// What one anchor did when rows or columns were inserted or removed under it.
///
/// A report, not a request: [`WorksheetDrawing::insert_rows`] and its three siblings return one of
/// these per anchor, and the fields say what the markers now hold rather than what the caller asked
/// for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnchorShift {
    /// The anchor's position among the drawing's anchors.
    pub index: usize,
    /// What the anchor promises to do when the cells under it move.
    pub promise: ResizingBehavior,
    /// Whether the object's top-left corner moved.
    pub moved: bool,
    /// Whether the object's extent changed — true only when the edit fell **between** a two-cell
    /// anchor's two markers, so its `to` moved and its `from` did not.
    pub resized: bool,
    /// Whether the markers alone could keep the anchor's own promise.
    ///
    /// `false` in exactly one case: a `xdr:twoCellAnchor` that was resized while its `@editAs` says
    /// its size must not change. A two-cell anchor has no extent of its own — the two markers *are*
    /// its geometry — so restoring the size means recomputing the `to` marker from the sheet's own
    /// row heights and column widths, which this crate cannot see. `mjx_sml`'s sheet-aware shift is
    /// what closes it for rows; see [`WorksheetDrawing::insert_rows`].
    pub promise_kept: bool,
}

/// `xdr:wsDr` (`CT_Drawing`, §20.5.2.35) — a worksheet's drawing part: every object on the sheet
/// that is not a cell, each in one of the three anchors.
///
/// *"It acts much like the `spTree` element within the DrawingML framework"* — with the difference
/// that a slide's shape tree positions a shape in slide coordinates and this one positions it
/// against a grid that moves.
///
/// A fidelity wrapper over the root element: unmodelled children, the root's own `xmlns:`
/// declarations and its attribute order all survive. **Parsing and serializing the part is the
/// caller's** — `mjx-xlsx` holds the package — exactly as it is for
/// [`WorksheetTable`](https://docs.rs/mjx-sml).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorksheetDrawing {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(WorksheetDrawing);

impl WorksheetDrawing {
    /// Reads a drawing part out of a parsed document.
    ///
    /// `Ok(None)` when the root is not an `xdr:wsDr` — the caller handed over some other part, which
    /// is a question rather than an error, exactly as `mjx_sml::WorksheetPart::read_document` treats
    /// it.
    ///
    /// # Errors
    /// [`FromXmlError`] if the root is an `xdr:wsDr` this model cannot read, which no well-formed
    /// XML can currently produce — every child is preserved rather than rejected.
    pub fn read_part(document: &mjx_ooxml_core::RawDocument) -> Result<Option<Self>, FromXmlError> {
        Self::read_root(&document.root, &document.interner)
    }

    /// [`read_part`](Self::read_part) for a caller holding the root element and its interner.
    ///
    /// # Errors
    /// As [`read_part`](Self::read_part).
    pub fn read_root(root: &RawElement, interner: &Interner) -> Result<Option<Self>, FromXmlError> {
        if !is_xdr(&root.name, interner) || interner.resolve(root.name.local) != "wsDr" {
            return Ok(None);
        }
        Ok(Some(Self::from_xml(root, interner)?))
    }

    /// Builds an empty `<xdr:wsDr>` declaring the three namespaces every drawing part needs:
    /// `xdr:` for its own elements, `a:` for the DrawingML types they carry, and `r:` for the
    /// `a:blip@r:embed` a picture names its image part with.
    ///
    /// Written as `<xdr:wsDr …></xdr:wsDr>` rather than self-closing, because the next thing that
    /// happens to a drawing part is that an anchor is put in it.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        let name = xdr_name(interner, "wsDr");
        let mut attributes = Vec::with_capacity(3);
        for (prefix, uri) in [
            (XDR_PREFIX, DML_SPREADSHEET_DRAWING.transitional),
            ("a", DML_MAIN.transitional),
            ("r", RELATIONSHIP_REFERENCE_NAMESPACE),
        ] {
            mjx_xml::attribute::set(&mut attributes, interner, Some("xmlns"), prefix, uri);
        }
        Self {
            name,
            attributes,
            children: Vec::new(),
            empty: false,
        }
    }

    /// The prefix this part binds to the relationship-reference namespace — `r` in every file this
    /// project has read, and the producer's choice rather than the schema's.
    ///
    /// `None` means the part binds the namespace nowhere, so no `a:blip` in it can carry an
    /// `@r:embed` at all.
    #[must_use]
    pub fn relationship_prefix<'a>(&'a self, interner: &'a Interner) -> Option<&'a str> {
        self.attributes.iter().find_map(|attribute| {
            let prefix = attribute
                .name
                .prefix
                .map(|symbol| interner.resolve(symbol))?;
            (prefix == "xmlns"
                && std::str::from_utf8(&attribute.value).ok()? == RELATIONSHIP_REFERENCE_NAMESPACE)
                .then(|| interner.resolve(attribute.name.local))
        })
    }

    /// Every anchor in the part, in document order.
    ///
    /// Anything that is not one of the three anchor elements is skipped — and preserved: a comment,
    /// an `mc:AlternateContent`, an element in a namespace `CT_Drawing` does not name.
    pub fn anchors<'a>(&'a self, interner: &'a Interner) -> impl Iterator<Item = Anchor> + 'a {
        self.children.iter().filter_map(move |node| match node {
            RawNode::Element(child) => Anchor::read(child, interner),
            _ => None,
        })
    }

    /// How many anchors the part holds.
    #[must_use]
    pub fn anchor_count(&self, interner: &Interner) -> usize {
        self.anchors(interner).count()
    }

    /// The anchor at `index` among the anchors, or `None`.
    #[must_use]
    pub fn anchor(&self, interner: &Interner, index: usize) -> Option<Anchor> {
        self.anchors(interner).nth(index)
    }

    /// Appends an anchor to the part.
    ///
    /// Appended rather than placed at a rank: `CT_Drawing`'s content model is one repeating
    /// `xsd:choice` of the three anchor elements, all at rank 0, so **document order is the only
    /// order there is** — and it is the paint order a consumer draws them in, which means moving one
    /// would change which object is on top.
    pub fn push_anchor(&mut self, interner: &mut Interner, anchor: &Anchor) {
        let element = anchor.to_xml(interner);
        self.children.push(RawNode::Element(element));
        self.empty = false;
    }

    /// Replaces the anchor at `index`, keeping its position — and so its paint order.
    ///
    /// `false` when there is no anchor there, and nothing is changed.
    pub fn replace_anchor(
        &mut self,
        interner: &mut Interner,
        index: usize,
        anchor: &Anchor,
    ) -> bool {
        let Some(at) = self.anchor_position(interner, index) else {
            return false;
        };
        let element = anchor.to_xml(interner);
        self.children[at] = RawNode::Element(element);
        true
    }

    /// Removes the anchor at `index`, reporting whether there was one.
    pub fn remove_anchor(&mut self, interner: &Interner, index: usize) -> bool {
        let Some(at) = self.anchor_position(interner, index) else {
            return false;
        };
        self.children.remove(at);
        true
    }

    /// The position among **all** children of the anchor at `index` among the anchors.
    fn anchor_position(&self, interner: &Interner, index: usize) -> Option<usize> {
        self.children
            .iter()
            .enumerate()
            .filter(|(_, node)| match node {
                RawNode::Element(child) => {
                    is_xdr(&child.name, interner)
                        && Anchor::is_anchor_local(interner.resolve(child.name.local))
                }
                _ => false,
            })
            .map(|(at, _)| at)
            .nth(index)
    }

    /// Moves every anchor for `count` rows inserted at the zero-based row `at`.
    ///
    /// Each anchor mode does what it promises, and the three answers differ:
    ///
    /// * a **`xdr:twoCellAnchor`** moves *and* sizes — both markers at or below `at` move down, so
    ///   an insertion above the object moves it and an insertion inside it makes it taller;
    /// * a **`xdr:oneCellAnchor`** moves and keeps its size — its `from` moves and its `xdr:ext` is
    ///   not touched;
    /// * a **`xdr:absoluteAnchor`** does neither — it names no cell, so there is nothing here to
    ///   move, and the returned report says `moved: false`.
    ///
    /// The one promise the markers alone cannot keep is a `xdr:twoCellAnchor` whose `@editAs` says
    /// its size must not change, when the insertion falls **between** its two markers: a two-cell
    /// anchor has no extent of its own, so restoring the size means recomputing the `to` marker from
    /// the sheet's own row heights, which this crate cannot see. Every such anchor comes back with
    /// `promise_kept: false`, naming itself, rather than being silently left wrong —
    /// `mjx_sml::SheetAnchors` is what closes it, because row heights are stated in points and can
    /// be resolved exactly.
    pub fn insert_rows(
        &mut self,
        interner: &mut Interner,
        at: i32,
        count: u32,
    ) -> Vec<AnchorShift> {
        self.shift(interner, Axis::Rows, at, i64::from(count))
    }

    /// Moves every anchor for `count` rows removed from the zero-based row `at`.
    ///
    /// A marker naming a row that is being removed is clamped to `at` with a zero offset — the cell
    /// it named is gone, and the nearest thing that still exists is the top of the row that has
    /// taken its place. Clamping rather than refusing, because **an anchor naming a row beyond the
    /// sheet is untrusted input, not a bug**.
    pub fn remove_rows(
        &mut self,
        interner: &mut Interner,
        at: i32,
        count: u32,
    ) -> Vec<AnchorShift> {
        self.shift(interner, Axis::Rows, at, -i64::from(count))
    }

    /// [`insert_rows`](Self::insert_rows) on the column axis.
    pub fn insert_columns(
        &mut self,
        interner: &mut Interner,
        at: i32,
        count: u32,
    ) -> Vec<AnchorShift> {
        self.shift(interner, Axis::Columns, at, i64::from(count))
    }

    /// [`remove_rows`](Self::remove_rows) on the column axis.
    pub fn remove_columns(
        &mut self,
        interner: &mut Interner,
        at: i32,
        count: u32,
    ) -> Vec<AnchorShift> {
        self.shift(interner, Axis::Columns, at, -i64::from(count))
    }

    /// The one implementation the four axis operations share.
    fn shift(
        &mut self,
        interner: &mut Interner,
        axis: Axis,
        at: i32,
        delta: i64,
    ) -> Vec<AnchorShift> {
        let mut report = Vec::new();
        for index in 0..self.anchor_count(interner) {
            let Some(position) = self.anchor_position(interner, index) else {
                break;
            };
            let RawNode::Element(element) = &self.children[position] else {
                continue;
            };
            let local = interner.resolve(element.name.local);
            let promise = match local {
                "oneCellAnchor" => ResizingBehavior::MoveWithCellsButDoNotResize,
                "absoluteAnchor" => ResizingBehavior::DoNotMoveOrResizeWithRowsOrColumns,
                _ => TwoCellAnchor::from_xml(element, interner)
                    .map(|anchor| anchor.resizing_behavior(interner))
                    .unwrap_or(ResizingBehavior::MoveAndResizeWithAnchorCells),
            };
            let two_cell = local == "twoCellAnchor";
            let markers: &[&str] = match local {
                "twoCellAnchor" => &["from", "to"],
                "oneCellAnchor" => &["from"],
                // An absolute anchor names no cell: `xdr:pos` is measured from the sheet origin, and
                // nothing that happens to a row or a column changes what it says.
                _ => &[],
            };

            let mut moved = false;
            let mut resized = false;
            for (which, marker_local) in markers.iter().enumerate() {
                let RawNode::Element(element) = &mut self.children[position] else {
                    continue;
                };
                let Some(child) = xdr_child_mut(&mut element.children, interner, marker_local)
                else {
                    continue;
                };
                let Some(marker) = CellMarker::read(child, interner) else {
                    continue;
                };
                let Some(shifted) = shift_marker(marker, axis, at, delta) else {
                    continue;
                };
                shifted.apply(child, interner);
                if which == 0 {
                    moved = true;
                } else {
                    resized = true;
                }
            }
            // `to` moved while `from` did not: the object's extent changed. When both moved by the
            // same amount the object was carried along whole, which is not a resize.
            let resized = resized && !moved;
            report.push(AnchorShift {
                index,
                promise,
                moved,
                resized,
                promise_kept: !(two_cell
                    && resized
                    && promise != ResizingBehavior::MoveAndResizeWithAnchorCells),
            });
        }
        report
    }
}

/// The relationship-reference namespace, restated here rather than imported: `mjx-dml` sits below
/// `mjx-opc` and cannot reach [`mjx_opc`]'s constant, and every other namespace in this crate is
/// resolved through [`mjx_ooxml_types::namespaces`], which does not carry this one either.
const RELATIONSHIP_REFERENCE_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// One marker under an insertion or a removal on `axis`, or `None` when nothing about it changes.
///
/// * **Insertion** (`delta > 0`): an index at or after `at` moves by `delta`; one before it does
///   not, because the rows above an insertion do not move.
/// * **Removal** (`delta < 0`): an index at or after the end of the removed run moves back by
///   `delta`; one *inside* the run is clamped to `at` with a zero offset, because the cell it named
///   no longer exists; one before the run does not move.
///
/// Saturating throughout. A file is free to write `col="2147483647"`, and a shift that overflowed
/// would be this library corrupting a part because somebody else wrote an implausible number.
fn shift_marker(marker: CellMarker, axis: Axis, at: i32, delta: i64) -> Option<CellMarker> {
    let (index, offset) = match axis {
        Axis::Rows => (marker.row, marker.row_offset),
        Axis::Columns => (marker.column, marker.column_offset),
    };
    let (shifted_index, shifted_offset) = if delta >= 0 {
        if index < at {
            return None;
        }
        (saturating_shift(index, delta), offset)
    } else {
        let removed_end = saturating_shift(at, -delta);
        if index >= removed_end {
            (saturating_shift(index, delta), offset)
        } else if index >= at {
            (at, Emu::from_emu(0))
        } else {
            return None;
        }
    };
    if shifted_index == index && shifted_offset == offset {
        return None;
    }
    Some(match axis {
        Axis::Rows => CellMarker {
            row: shifted_index,
            row_offset: shifted_offset,
            ..marker
        },
        Axis::Columns => CellMarker {
            column: shifted_index,
            column_offset: shifted_offset,
            ..marker
        },
    })
}

/// `index + delta`, clamped to the non-negative range `xdr:ST_ColID`/`ST_RowID` allow.
fn saturating_shift(index: i32, delta: i64) -> i32 {
    let shifted = i64::from(index).saturating_add(delta);
    i32::try_from(shifted.clamp(0, i64::from(i32::MAX))).unwrap_or(i32::MAX)
}

/// A fresh, minimally-complete anchored picture: `id`/`name` for its identity, `relationship_id`
/// naming the image part, and a preset rectangle geometry.
///
/// No transform is written. A picture in a `xdr:twoCellAnchor` takes its geometry from the two
/// markers, and one in a `xdr:oneCellAnchor` or `xdr:absoluteAnchor` from the anchor's own
/// `xdr:ext` — so an `a:xfrm` here would be a second, disagreeing statement of the same fact.
/// (`mjx_dml::new_picture` writes one because a `pic:pic` inside a `a:graphicData` has no anchor to
/// take it from.)
#[must_use]
pub fn new_anchored_picture(
    interner: &mut Interner,
    id: u32,
    name: &str,
    relationship_id: &str,
) -> DrawingPicture {
    let mut picture = DrawingPicture {
        name: xdr_name(interner, "pic"),
        attributes: Vec::new(),
        children: Vec::new(),
        empty: false,
    };

    // `xdr:nvPicPr` — `xdr:cNvPr` (the identity) then `xdr:cNvPicPr` (the lock list).
    let non_visual_name = xdr_name(interner, "nvPicPr");
    let cnv_pr_name = xdr_name(interner, "cNvPr");
    let cnv_pic_pr_name = xdr_name(interner, "cNvPicPr");
    let identity = NonVisualDrawingProps::with_name(interner, cnv_pr_name, id, name);
    let identity = RawNode::Element(identity.to_xml(interner));
    let picture_props = RawNode::Element(RawElement::new(
        cnv_pic_pr_name,
        Vec::new(),
        Vec::new(),
        true,
    ));
    picture.children.push(RawNode::Element(RawElement::new(
        non_visual_name,
        Vec::new(),
        vec![identity, picture_props],
        false,
    )));

    // `xdr:blipFill` — the `a:blip@r:embed` naming the image part, stretched to the anchor.
    let blip_fill_name = xdr_name(interner, "blipFill");
    let fill = PictureFill::with_name(
        interner,
        blip_fill_name,
        relationship_id,
        PictureFillMode::Stretch,
    );
    picture
        .children
        .push(RawNode::Element(fill.to_xml(interner)));

    // `xdr:spPr` — a rectangle, which is what a picture's outline is unless a caller says otherwise.
    let sp_pr_name = xdr_name(interner, "spPr");
    let mut properties = ShapeProperties::with_name(interner, sp_pr_name);
    let rectangle = crate::geometry::PresetGeometry::new(
        interner,
        mjx_ooxml_types::drawingml::PresetShapeType::Rectangle,
        None,
    );
    properties.set_geometry(
        interner,
        crate::shape_properties::ShapeGeometryChoice::Preset(rectangle),
    );
    picture
        .children
        .push(RawNode::Element(properties.to_xml(interner)));

    picture
}

/// A fresh `xdr:twoCellAnchor` holding `object`, pinned between `from` and `to`.
///
/// `behavior` is written as `@editAs` unless it is the schema's own default
/// (`MoveAndResizeWithAnchorCells`), which is left unwritten — the shorter spelling is what a
/// producer emits, and writing `editAs="twoCell"` would be authoring an attribute that says nothing.
#[must_use]
pub fn new_two_cell_anchor(
    interner: &mut Interner,
    from: CellMarker,
    to: CellMarker,
    object: &AnchoredObject,
    behavior: ResizingBehavior,
) -> TwoCellAnchor {
    let mut anchor = TwoCellAnchor {
        name: xdr_name(interner, "twoCellAnchor"),
        attributes: Vec::new(),
        children: Vec::new(),
        empty: false,
    };
    if behavior != ResizingBehavior::MoveAndResizeWithAnchorCells {
        anchor.set_resizing_behavior_attribute(interner, Some(behavior));
    }
    let from = from.to_element(interner, "from");
    let to = to.to_element(interner, "to");
    anchor.children.push(RawNode::Element(from));
    anchor.children.push(RawNode::Element(to));
    anchor
        .children
        .push(RawNode::Element(object.to_xml(interner)));
    let client_data = AnchorClientData::new(interner);
    anchor
        .children
        .push(RawNode::Element(client_data.to_xml(interner)));
    anchor
}

/// A fresh `xdr:oneCellAnchor` holding `object`, pinned to `from` and sized `extent`.
#[must_use]
pub fn new_one_cell_anchor(
    interner: &mut Interner,
    from: CellMarker,
    extent: Size,
    object: &AnchoredObject,
) -> OneCellAnchor {
    let mut anchor = OneCellAnchor {
        name: xdr_name(interner, "oneCellAnchor"),
        attributes: Vec::new(),
        children: Vec::new(),
        empty: false,
    };
    let from = from.to_element(interner, "from");
    anchor.children.push(RawNode::Element(from));
    anchor.set_extent(interner, extent);
    anchor
        .children
        .push(RawNode::Element(object.to_xml(interner)));
    let client_data = AnchorClientData::new(interner);
    anchor
        .children
        .push(RawNode::Element(client_data.to_xml(interner)));
    anchor
}

/// A fresh `xdr:absoluteAnchor` holding `object`, at `position` and sized `extent`.
#[must_use]
pub fn new_absolute_anchor(
    interner: &mut Interner,
    position: Position,
    extent: Size,
    object: &AnchoredObject,
) -> AbsoluteAnchor {
    let mut anchor = AbsoluteAnchor {
        name: xdr_name(interner, "absoluteAnchor"),
        attributes: Vec::new(),
        children: Vec::new(),
        empty: false,
    };
    anchor.set_position(interner, position);
    anchor.set_extent(interner, extent);
    anchor
        .children
        .push(RawNode::Element(object.to_xml(interner)));
    let client_data = AnchorClientData::new(interner);
    anchor
        .children
        .push(RawNode::Element(client_data.to_xml(interner)));
    anchor
}
