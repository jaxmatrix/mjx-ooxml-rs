//! The diagram data part (`dgm:dataModel`, `CT_DataModel`) — the point-and-connection graph a
//! SmartArt diagram draws, and the property set (`dgm:prSet`) that binds each point to a layout,
//! quick style and colour transform.
//!
//! This is the load-bearing model in this module: a diagram is a graph, and this is the graph.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{
    Enumeration, Interner, Number, RawAttribute, RawElement, RawName, RawNode, Text,
};
use mjx_ooxml_types::child_order::LAYOUT_VARIABLE_PROPERTY_SET;
use mjx_ooxml_types::diagram::{ConnectionType, PointType};
use mjx_ooxml_types::support::OnOff;

use crate::build::fidelity_element_impls;
use crate::text::TextBody;

use super::support::{dgm_element, dgm_name, is_dgm};

// ---------------------------------------------------------------------------------------------
// Points
// ---------------------------------------------------------------------------------------------

/// `dgm:prSet` (`CT_ElemPropSet`) — the customization and binding properties a point carries.
///
/// ECMA-376 Part 1 §21.4.3.4 groups its attributes into four families: presentation properties
/// (`presAssocID`, …), document properties (`loTypeId`, …, the layout/style/colours binding),
/// semantic element properties (`phldrT`, `phldr`), and customization properties (`custAng`, …). All
/// four families are typed here; the two child elements are `presLayoutVars` (typed —
/// [`LayoutVariables`]) and `style` (`a:CT_ShapeStyle`, preserved opaque — this crate does not yet
/// model a standalone shape-style type outside a shape's own surface).
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_DIAGRAM)]
#[xml(attribute(local = "presAssocID", codec = Text, accessor = presentation_association_id))]
#[xml(attribute(local = "presName", codec = Text, accessor = presentation_name))]
#[xml(attribute(local = "presStyleLbl", codec = Text, accessor = presentation_style_label))]
#[xml(attribute(local = "presStyleIdx", codec = Number<i32>, accessor = presentation_style_index))]
#[xml(attribute(local = "presStyleCnt", codec = Number<i32>, accessor = presentation_style_count))]
#[xml(attribute(local = "loTypeId", codec = Text, accessor = layout_type_id))]
#[xml(attribute(local = "loCatId", codec = Text, accessor = layout_category_id))]
#[xml(attribute(local = "qsTypeId", codec = Text, accessor = quick_style_type_id))]
#[xml(attribute(local = "qsCatId", codec = Text, accessor = quick_style_category_id))]
#[xml(attribute(local = "csTypeId", codec = Text, accessor = colors_type_id))]
#[xml(attribute(local = "csCatId", codec = Text, accessor = colors_category_id))]
#[xml(attribute(local = "coherent3DOff", codec = OnOff, accessor = coherent_3d_off))]
#[xml(attribute(local = "phldrT", codec = Text, accessor = placeholder_text))]
#[xml(attribute(local = "phldr", codec = OnOff, accessor = is_placeholder))]
#[xml(attribute(local = "custAng", codec = Number<i32>, accessor = custom_angle))]
#[xml(attribute(local = "custFlipVert", codec = OnOff, accessor = custom_flip_vertical))]
#[xml(attribute(local = "custFlipHor", codec = OnOff, accessor = custom_flip_horizontal))]
#[xml(attribute(local = "custSzX", codec = Number<i32>, accessor = custom_width_override))]
#[xml(attribute(local = "custSzY", codec = Number<i32>, accessor = custom_height_override))]
#[xml(attribute(local = "custScaleX", codec = Text, accessor = custom_width_scale))]
#[xml(attribute(local = "custScaleY", codec = Text, accessor = custom_height_scale))]
#[xml(attribute(local = "custT", codec = OnOff, accessor = custom_text_changed))]
#[xml(attribute(local = "custLinFactX", codec = Text, accessor = custom_linear_factor_x))]
#[xml(attribute(local = "custLinFactY", codec = Text, accessor = custom_linear_factor_y))]
#[xml(attribute(
    local = "custLinFactNeighborX",
    codec = Text,
    accessor = custom_linear_factor_neighbor_x
))]
#[xml(attribute(
    local = "custLinFactNeighborY",
    codec = Text,
    accessor = custom_linear_factor_neighbor_y
))]
#[xml(attribute(local = "custRadScaleRad", codec = Text, accessor = custom_radius_scale_radius))]
#[xml(attribute(
    local = "custRadScaleInc",
    codec = Text,
    accessor = custom_radius_scale_increment
))]
pub struct ElementPropertySet {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "presLayoutVars", variant = Variables, ty = LayoutVariables))]
    content: Vec<ElementPropertySetContent>,
}

/// One ordered child of an [`ElementPropertySet`]: the typed [`LayoutVariables`], or an opaque node
/// (`style`, or an unknown element).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementPropertySetContent {
    /// `presLayoutVars` — the presentation layout variables this element overrides.
    Variables(LayoutVariables),
    /// Any other child — `style` (`a:CT_ShapeStyle`), whitespace, or an unknown element.
    Raw(RawNode),
}

impl ElementPropertySet {
    /// A fresh, empty `dgm:prSet` — no attribute set, no children. Callers layer bindings on with
    /// the generated `set_*` accessors.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: dgm_name(interner, "prSet"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The property set's presentation layout variable overrides (`presLayoutVars`), or `None`.
    #[must_use]
    pub fn layout_variables(&self) -> Option<&LayoutVariables> {
        self.content.iter().find_map(|item| match item {
            ElementPropertySetContent::Variables(vars) => Some(vars),
            ElementPropertySetContent::Raw(_) => None,
        })
    }
}

/// `dgm:pt` (`CT_Pt`) — one point: a node, a transition, the document root, or a presentation node
/// (see [`PointType`]).
///
/// ECMA-376 Part 1 §21.4.3.5: "A point in DiagramML is defined to hold data associated with a
/// particular point or node in a diagram." `spPr` (shape-property overrides) and `extLst` are
/// preserved opaque; `prSet` and `t` (the point's text, wire-identical to a shape's `a:txBody` — see
/// [`TextBody`]) are typed.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_DIAGRAM)]
#[xml(attribute(local = "modelId", codec = Text, accessor = model_id, required))]
#[xml(attribute(local = "type", codec = Enumeration<PointType>, accessor = point_type, default = PointType::Node))]
#[xml(attribute(local = "cxnId", codec = Text, accessor = connection_id))]
pub struct Point {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "prSet", variant = Properties, ty = ElementPropertySet),
        child(local = "t", variant = Text, ty = TextBody)
    )]
    content: Vec<PointContent>,
}

/// One ordered child of a [`Point`]: a typed [`ElementPropertySet`] or [`TextBody`], or an opaque
/// node (`spPr`, `extLst`, or an unknown element).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PointContent {
    /// `prSet` — the point's property set (layout/style/colours binding, customizations).
    Properties(ElementPropertySet),
    /// `t` — the point's text body, wire-identical to `a:txBody`.
    Text(TextBody),
    /// Any other child — `spPr` (`a:CT_ShapeProperties`), `extLst`, whitespace, or an unknown
    /// element.
    Raw(RawNode),
}

impl Point {
    /// A fresh `dgm:pt` of `model_id`, typed `point_type`, holding no properties or text yet.
    #[must_use]
    pub fn new(interner: &mut Interner, model_id: &str, point_type: PointType) -> Self {
        let mut pt = Self {
            name: dgm_name(interner, "pt"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        pt.set_model_id(interner, model_id);
        pt.set_point_type(interner, Some(point_type));
        pt
    }

    /// This point's property set (`prSet`), or `None` if it declares one carrying nothing (or none
    /// at all — the two read alike, since an empty `prSet` and an absent one both bind nothing).
    #[must_use]
    pub fn properties(&self) -> Option<&ElementPropertySet> {
        self.content.iter().find_map(|item| match item {
            PointContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// This point's text body (`t`), or `None` if it carries none.
    #[must_use]
    pub fn text(&self) -> Option<&TextBody> {
        self.content.iter().find_map(|item| match item {
            PointContent::Text(text) => Some(text),
            _ => None,
        })
    }

    /// The point's text, read through [`TextBody::text`], or `None` if it carries no text body.
    #[must_use]
    pub fn text_content(&self) -> Option<String> {
        self.text().map(TextBody::text)
    }
}

/// One ordered child of a [`PointList`]: a typed [`Point`], or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PointListContent {
    /// A point (`dgm:pt`).
    Point(Point),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `dgm:ptLst` (`CT_PtList`) — every point of a data model, in document order (§21.4.3.6).
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_DIAGRAM)]
pub struct PointList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "pt", variant = Point, ty = Point))]
    content: Vec<PointListContent>,
}

impl PointList {
    /// A fresh `dgm:ptLst` of `points`.
    #[must_use]
    pub fn new(interner: &mut Interner, points: Vec<Point>) -> Self {
        let content: Vec<PointListContent> =
            points.into_iter().map(PointListContent::Point).collect();
        let empty = content.is_empty();
        Self {
            name: dgm_name(interner, "ptLst"),
            attributes: Vec::new(),
            empty,
            content,
        }
    }

    /// The list's points, in document order (opaque children are skipped).
    pub fn points(&self) -> impl Iterator<Item = &Point> {
        self.content.iter().filter_map(|item| match item {
            PointListContent::Point(pt) => Some(pt),
            _ => None,
        })
    }

    /// The point whose `modelId` is `model_id`, or `None` — including when a point's `modelId` fails
    /// to decode (a required attribute is still read fallibly: a malformed file is read as it is,
    /// never rejected, so a decode failure just means that point cannot match).
    #[must_use]
    pub fn point_by_id<'a>(&'a self, interner: &Interner, model_id: &str) -> Option<&'a Point> {
        self.points().find(|pt| {
            pt.model_id(interner)
                .is_ok_and(|id| id.as_ref() == model_id)
        })
    }

    /// The list's ordered content (typed points interleaved with opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[PointListContent] {
        &self.content
    }
}

// ---------------------------------------------------------------------------------------------
// Connections
// ---------------------------------------------------------------------------------------------

/// `dgm:cxn` (`CT_Cxn`) — a directed edge between two points, ranked among its source's and its
/// destination's siblings.
///
/// ECMA-376 Part 1 §21.4.3.2: "This element defines a connection between two points. A connection
/// defines a relationship between two points in a diagram." `extLst` is the only child, preserved
/// opaque (rare in practice — this project has never seen one).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "modelId", codec = Text, accessor = model_id, required))]
#[xml(attribute(local = "type", codec = Enumeration<ConnectionType>, accessor = connection_type, default = ConnectionType::ParentOf))]
#[xml(attribute(local = "srcId", codec = Text, accessor = source_id, required))]
#[xml(attribute(local = "destId", codec = Text, accessor = destination_id, required))]
#[xml(attribute(local = "srcOrd", codec = Number<u32>, accessor = source_order, required))]
#[xml(attribute(local = "destOrd", codec = Number<u32>, accessor = destination_order, required))]
#[xml(attribute(local = "parTransId", codec = Text, accessor = parent_transition_id))]
#[xml(attribute(local = "sibTransId", codec = Text, accessor = sibling_transition_id))]
#[xml(attribute(local = "presId", codec = Text, accessor = presentation_id))]
pub struct Connection {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(Connection);

impl Connection {
    /// A fresh `parOf` `dgm:cxn` from `source_id` to `destination_id`, ranked `source_order` among
    /// the source's children and `destination_order` among the destination's siblings.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        model_id: &str,
        source_id: &str,
        destination_id: &str,
        source_order: u32,
        destination_order: u32,
    ) -> Self {
        let mut cxn = Self {
            name: dgm_name(interner, "cxn"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        cxn.set_model_id(interner, model_id);
        cxn.set_source_id(interner, source_id);
        cxn.set_destination_id(interner, destination_id);
        cxn.set_source_order(interner, source_order);
        cxn.set_destination_order(interner, destination_order);
        cxn
    }
}

/// One ordered child of a [`ConnectionList`]: a typed [`Connection`], or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionListContent {
    /// A connection (`dgm:cxn`).
    Connection(Connection),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `dgm:cxnLst` (`CT_CxnList`) — every connection of a data model, in document order (§21.4.3.3).
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_DIAGRAM)]
pub struct ConnectionList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "cxn", variant = Connection, ty = Connection))]
    content: Vec<ConnectionListContent>,
}

impl ConnectionList {
    /// A fresh `dgm:cxnLst` of `connections`.
    #[must_use]
    pub fn new(interner: &mut Interner, connections: Vec<Connection>) -> Self {
        let content: Vec<ConnectionListContent> = connections
            .into_iter()
            .map(ConnectionListContent::Connection)
            .collect();
        let empty = content.is_empty();
        Self {
            name: dgm_name(interner, "cxnLst"),
            attributes: Vec::new(),
            empty,
            content,
        }
    }

    /// The list's connections, in document order (opaque children are skipped).
    pub fn connections(&self) -> impl Iterator<Item = &Connection> {
        self.content.iter().filter_map(|item| match item {
            ConnectionListContent::Connection(cxn) => Some(cxn),
            _ => None,
        })
    }

    /// Every connection whose `srcId` is `source_id`, in `srcOrd` order — the outgoing edges of one
    /// point, which is what makes the connection list a graph's adjacency rather than a flat list.
    pub fn connections_from<'a>(
        &'a self,
        interner: &'a Interner,
        source_id: &'a str,
    ) -> impl Iterator<Item = &'a Connection> {
        self.connections().filter(move |cxn| {
            cxn.source_id(interner)
                .is_ok_and(|id| id.as_ref() == source_id)
        })
    }

    /// The list's ordered content (typed connections interleaved with opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[ConnectionListContent] {
        &self.content
    }
}

// ---------------------------------------------------------------------------------------------
// The data model itself
// ---------------------------------------------------------------------------------------------

/// One ordered child of a [`DataModel`]: the typed [`PointList`] or [`ConnectionList`], or an opaque
/// node (`bg`, `whole`, `extLst`, or an unknown element).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataModelContent {
    /// `ptLst` — every point.
    Points(PointList),
    /// `cxnLst` — every connection.
    Connections(ConnectionList),
    /// Any other child — `bg` (background formatting), `whole` (whole-diagram formatting),
    /// `extLst`, whitespace, or an unknown element.
    Raw(RawNode),
}

/// `dgm:dataModel` (`CT_DataModel`) — the diagram's data: "the nodes and the connections between
/// them" (§21.4.2.10 *dataModel (Data Model)*: "The data for this instance of the diagram.").
///
/// This is the root element of the Diagram Data part — what
/// [`DiagramContent::data`](https://docs.rs/mjx-pptx) holds as bytes, and what a caller reading a
/// diagram back parses to get the point-and-connection graph. `bg` and `whole` (whole-diagram
/// formatting) are preserved opaque; they carry no graph information.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_DIAGRAM)]
pub struct DataModel {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "ptLst", variant = Points, ty = PointList),
        child(local = "cxnLst", variant = Connections, ty = ConnectionList)
    )]
    content: Vec<DataModelContent>,
}

impl DataModel {
    /// A fresh `dgm:dataModel` of `points` and `connections`, in schema order (`ptLst` before
    /// `cxnLst`).
    #[must_use]
    pub fn new(interner: &mut Interner, points: PointList, connections: ConnectionList) -> Self {
        Self {
            name: dgm_name(interner, "dataModel"),
            attributes: Vec::new(),
            empty: false,
            content: vec![
                DataModelContent::Points(points),
                DataModelContent::Connections(connections),
            ],
        }
    }

    /// The data model's points (`ptLst`), or `None` if the document omits the (schema-required)
    /// element — a malformed file is read as it is, never rejected.
    #[must_use]
    pub fn points(&self) -> Option<&PointList> {
        self.content.iter().find_map(|item| match item {
            DataModelContent::Points(points) => Some(points),
            _ => None,
        })
    }

    /// The data model's connections (`cxnLst`), or `None` if it declares none.
    #[must_use]
    pub fn connections(&self) -> Option<&ConnectionList> {
        self.content.iter().find_map(|item| match item {
            DataModelContent::Connections(connections) => Some(connections),
            _ => None,
        })
    }

    /// The data model's ordered content (typed points/connections interleaved with opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[DataModelContent] {
        &self.content
    }
}

// ---------------------------------------------------------------------------------------------
// Layout variables
// ---------------------------------------------------------------------------------------------

/// Drops [`LayoutVariables`]'s child named `local`, if it has one — an absent variable is "not set",
/// distinct from a present one whose `val` is absent (which the schema gives its own default).
fn remove_variable(children: &mut Vec<RawNode>, interner: &Interner, local: &str) {
    children.retain(|node| match node {
        RawNode::Element(element) => {
            !(is_dgm(&element.name, interner) && interner.resolve(element.name.local) == local)
        }
        _ => true,
    });
}

/// The child named `local`, or `None`.
fn find_variable<'a>(
    children: &'a [RawNode],
    interner: &Interner,
    local: &str,
) -> Option<&'a RawElement> {
    children.iter().find_map(|node| match node {
        RawNode::Element(element)
            if is_dgm(&element.name, interner) && interner.resolve(element.name.local) == local =>
        {
            Some(element)
        }
        _ => None,
    })
}

/// Reads `local`'s `val` attribute as `T`, or `None` if the child is absent or its value does not
/// parse (a malformed file is read as it is, never rejected).
fn read_variable<T>(children: &[RawNode], interner: &Interner, local: &str) -> Option<T>
where
    T: core::str::FromStr,
{
    let element = find_variable(children, interner, local)?;
    mjx_xml::attribute::find(&element.attributes, interner, None, "val")
        .and_then(|attribute| mjx_xml::attribute::decoded_value(attribute, "val").ok())
        .and_then(|value| value.parse::<T>().ok())
}

/// Reads `local`'s `val` attribute as a boolean, accepting every `ST_OnOff`-family spelling the
/// schema's plain `xsd:boolean` does not itself require but a real file may still carry.
fn read_variable_bool(children: &[RawNode], interner: &Interner, local: &str) -> Option<bool> {
    let element = find_variable(children, interner, local)?;
    let attribute = mjx_xml::attribute::find(&element.attributes, interner, None, "val")?;
    let raw = mjx_xml::attribute::decoded_value(attribute, "val").ok()?;
    <OnOff as mjx_ooxml_core::AttributeCodec>::decode(raw).ok()
}

/// Sets or clears `local`'s `val` attribute, creating the child (at its rank in
/// [`LAYOUT_VARIABLE_PROPERTY_SET`]'s `xsd:sequence` — never appended or prepended blindly) or
/// removing it to match.
fn write_variable<T: ToString>(
    children: &mut Vec<RawNode>,
    interner: &mut Interner,
    local: &'static str,
    value: Option<T>,
) {
    let Some(value) = value else {
        remove_variable(children, interner, local);
        return;
    };
    let mut element = find_variable(children, interner, local)
        .cloned()
        .unwrap_or_else(|| dgm_element(interner, local, Vec::new(), Vec::new()));
    mjx_xml::attribute::set(
        &mut element.attributes,
        interner,
        None,
        "val",
        &value.to_string(),
    );
    LAYOUT_VARIABLE_PROPERTY_SET
        .replace_or_insert(children, interner, element, |candidate| candidate == local);
}

/// Sets or clears `local`'s `val` attribute as the canonical `ST_OnOff` spelling, placed the same
/// way [`write_variable`] places it.
fn write_variable_bool(
    children: &mut Vec<RawNode>,
    interner: &mut Interner,
    local: &'static str,
    value: Option<bool>,
) {
    let Some(value) = value else {
        remove_variable(children, interner, local);
        return;
    };
    let mut element = find_variable(children, interner, local)
        .cloned()
        .unwrap_or_else(|| dgm_element(interner, local, Vec::new(), Vec::new()));
    let encoded = <OnOff as mjx_ooxml_core::AttributeCodec>::encode(value);
    mjx_xml::attribute::set(&mut element.attributes, interner, None, "val", &encoded);
    LAYOUT_VARIABLE_PROPERTY_SET
        .replace_or_insert(children, interner, element, |candidate| candidate == local);
}

/// `dgm:varLst` (`CT_LayoutVariablePropertySet`) — "a list of variables which interact with user
/// interface components" (§21.4.2.31).
///
/// The schema wraps each of the nine variables in its own single-attribute element
/// (`CT_ChildMax`, `CT_Direction`, …); this type collapses that ceremony into nine plain accessor
/// pairs over the element's own retained children, the same discipline
/// `#[xml(attribute(..))]` applies to attributes — one call to read, one to write, and an absent
/// variable stays absent rather than being synthesized from a default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutVariables {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(LayoutVariables);

impl LayoutVariables {
    /// A fresh, empty `dgm:varLst` — no variables set.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: dgm_name(interner, "varLst"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }

    /// `orgChart@val` — whether the organization-chart layout's user-interface controls are shown
    /// (§21.4.6.8).
    #[must_use]
    pub fn organization_chart(&self, interner: &Interner) -> Option<bool> {
        read_variable(&self.children, interner, "orgChart")
    }

    /// Sets or clears [`organization_chart`](Self::organization_chart).
    pub fn set_organization_chart(&mut self, interner: &mut Interner, value: Option<bool>) {
        write_variable(&mut self.children, interner, "orgChart", value);
        self.empty = self.children.is_empty();
    }

    /// `chMax@val` — the maximum number of children a node may have; `-1` means unbounded
    /// (§21.4.6.4).
    #[must_use]
    pub fn maximum_children(&self, interner: &Interner) -> Option<i32> {
        read_variable(&self.children, interner, "chMax")
    }

    /// Sets or clears [`maximum_children`](Self::maximum_children).
    pub fn set_maximum_children(&mut self, interner: &mut Interner, value: Option<i32>) {
        write_variable(&mut self.children, interner, "chMax", value);
        self.empty = self.children.is_empty();
    }

    /// `chPref@val` — the preferred number of children a node should have (§21.4.6.5).
    #[must_use]
    pub fn preferred_children(&self, interner: &Interner) -> Option<i32> {
        read_variable(&self.children, interner, "chPref")
    }

    /// Sets or clears [`preferred_children`](Self::preferred_children).
    pub fn set_preferred_children(&mut self, interner: &mut Interner, value: Option<i32>) {
        write_variable(&mut self.children, interner, "chPref", value);
        self.empty = self.children.is_empty();
    }

    /// `bulletEnabled@val` — whether the "insert bullet as node" user-interface control is shown
    /// (§21.4.6.3). The schema types `@val` as plain `xsd:boolean`, but every wire spelling this
    /// crate's `ST_OnOff` family accepts is read leniently here too, matching this project's
    /// two-valued-type convention (see `mjx_ooxml_types::support`).
    #[must_use]
    pub fn bullets_enabled(&self, interner: &Interner) -> Option<bool> {
        read_variable_bool(&self.children, interner, "bulletEnabled")
    }

    /// Sets or clears [`bullets_enabled`](Self::bullets_enabled), writing the one canonical
    /// `ST_OnOff` spelling.
    pub fn set_bullets_enabled(&mut self, interner: &mut Interner, value: Option<bool>) {
        write_variable_bool(&mut self.children, interner, "bulletEnabled", value);
        self.empty = self.children.is_empty();
    }

    /// `dir@val` — the diagram's traversal direction (§21.4.6.6, [`TraversalDirection`](super::TraversalDirection)).
    #[must_use]
    pub fn direction(&self, interner: &Interner) -> Option<super::TraversalDirection> {
        let wire = read_variable::<String>(&self.children, interner, "dir")?;
        super::TraversalDirection::from_wire(&wire)
    }

    /// Sets or clears [`direction`](Self::direction).
    pub fn set_direction(
        &mut self,
        interner: &mut Interner,
        value: Option<super::TraversalDirection>,
    ) {
        write_variable(
            &mut self.children,
            interner,
            "dir",
            value.map(|direction| direction.to_wire().to_owned()),
        );
        self.empty = self.children.is_empty();
    }

    /// `hierBranch@val` — the hierarchy diagram's branch style (§21.4.6.7,
    /// [`HierarchyBranchStyle`](super::HierarchyBranchStyle)).
    #[must_use]
    pub fn hierarchy_branch(&self, interner: &Interner) -> Option<super::HierarchyBranchStyle> {
        let wire = read_variable::<String>(&self.children, interner, "hierBranch")?;
        super::HierarchyBranchStyle::from_wire(&wire)
    }

    /// Sets or clears [`hierarchy_branch`](Self::hierarchy_branch).
    pub fn set_hierarchy_branch(
        &mut self,
        interner: &mut Interner,
        value: Option<super::HierarchyBranchStyle>,
    ) {
        write_variable(
            &mut self.children,
            interner,
            "hierBranch",
            value.map(|style| style.to_wire().to_owned()),
        );
        self.empty = self.children.is_empty();
    }

    /// `animOne@val` — the one-by-one animation string a consumer offers in its user interface
    /// (§21.4.6.2... actually §21.4.6, see [`OneByOneAnimation`](super::OneByOneAnimation)).
    #[must_use]
    pub fn one_by_one_animation(&self, interner: &Interner) -> Option<super::OneByOneAnimation> {
        let wire = read_variable::<String>(&self.children, interner, "animOne")?;
        super::OneByOneAnimation::from_wire(&wire)
    }

    /// Sets or clears [`one_by_one_animation`](Self::one_by_one_animation).
    pub fn set_one_by_one_animation(
        &mut self,
        interner: &mut Interner,
        value: Option<super::OneByOneAnimation>,
    ) {
        write_variable(
            &mut self.children,
            interner,
            "animOne",
            value.map(|animation| animation.to_wire().to_owned()),
        );
        self.empty = self.children.is_empty();
    }

    /// `animLvl@val` — the level-animation string a consumer offers ([`LevelAnimation`](super::LevelAnimation)).
    #[must_use]
    pub fn level_animation(&self, interner: &Interner) -> Option<super::LevelAnimation> {
        let wire = read_variable::<String>(&self.children, interner, "animLvl")?;
        super::LevelAnimation::from_wire(&wire)
    }

    /// Sets or clears [`level_animation`](Self::level_animation).
    pub fn set_level_animation(
        &mut self,
        interner: &mut Interner,
        value: Option<super::LevelAnimation>,
    ) {
        write_variable(
            &mut self.children,
            interner,
            "animLvl",
            value.map(|animation| animation.to_wire().to_owned()),
        );
        self.empty = self.children.is_empty();
    }

    /// `resizeHandles@val` — how a consumer resizes shapes within the diagram
    /// ([`ResizeHandleBehavior`](super::ResizeHandleBehavior)).
    #[must_use]
    pub fn resize_handles(&self, interner: &Interner) -> Option<super::ResizeHandleBehavior> {
        let wire = read_variable::<String>(&self.children, interner, "resizeHandles")?;
        super::ResizeHandleBehavior::from_wire(&wire)
    }

    /// Sets or clears [`resize_handles`](Self::resize_handles).
    pub fn set_resize_handles(
        &mut self,
        interner: &mut Interner,
        value: Option<super::ResizeHandleBehavior>,
    ) {
        write_variable(
            &mut self.children,
            interner,
            "resizeHandles",
            value.map(|behavior| behavior.to_wire().to_owned()),
        );
        self.empty = self.children.is_empty();
    }
}
