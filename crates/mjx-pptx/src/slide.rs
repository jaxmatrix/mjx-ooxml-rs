//! Navigation of a slide's shape tree (`p:sld > p:cSld > p:spTree > p:sp > p:txBody`).

use mjx_dml::{
    ColorMap, ColorSchemeSlot, EffectListSpec, Fill, FillSpec, LineSpec, PresetGeometry,
    Scene3DSpec, Shape3DSpec, StyleMatrixReference,
};
use mjx_mce::MARKUP_COMPATIBILITY_2006;
use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawElement, RawNode, Symbol, ToXml};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_ooxml_types::namespaces::{SchemaNamespace, DML_CHART, DML_DIAGRAM, DML_MAIN, PML};
use mjx_ooxml_types::presentationml::{Orientation, PlaceholderSize, PlaceholderType};

use mjx_ooxml_types::child_order::{
    ChildOrder, GRAPHIC_FRAME, GROUP_SHAPE_PROPERTIES, SHAPE_PROPERTIES,
};

use crate::address::ShapePath;
use crate::build;
use crate::error::PptxError;
use crate::geometry::Geometry;
use crate::nav;

/// The `p:spPr` child elements that a fill must precede, per `CT_ShapeProperties`'s content order
/// (line, effects, 3-D, extensions). A new fill is inserted before the first of these.
const AFTER_FILL_LOCALS: [&str; 6] = ["ln", "effectLst", "effectDag", "scene3d", "sp3d", "extLst"];

/// The `p:spPr` child elements that the outline (`a:ln`) must precede, per `CT_ShapeProperties`'s
/// content order (effects, 3-D, extensions). This is [`AFTER_FILL_LOCALS`] without the leading `ln`,
/// so a new outline lands after any geometry and fill (neither is in the set) and before effects.
const AFTER_LINE_LOCALS: [&str; 5] = ["effectLst", "effectDag", "scene3d", "sp3d", "extLst"];

/// The `p:spPr` child elements that the effect list (`a:effectLst`) must precede, per
/// `CT_ShapeProperties`'s content order (3-D, extensions). This is [`AFTER_LINE_LOCALS`] without the
/// leading effect-container names, so a new effect list lands after any geometry, fill, and outline
/// (none of which is in the set) and before the 3-D/extension children.
const AFTER_EFFECT_LOCALS: [&str; 3] = ["scene3d", "sp3d", "extLst"];

/// The `p:spPr` child elements that the 3-D scene (`a:scene3d`) must precede, per
/// `CT_ShapeProperties`'s content order (the shape's own 3-D, extensions). A new `a:scene3d` lands
/// after any geometry, fill, outline, and effects (none of which is in the set) and before `a:sp3d`.
const AFTER_SCENE3D_LOCALS: [&str; 2] = ["sp3d", "extLst"];

/// The `p:spPr` child elements that the shape's 3-D properties (`a:sp3d`) must precede, per
/// `CT_ShapeProperties`'s content order (extensions only). `a:sp3d` is the last visual property, so a
/// new one lands after everything else and before only any `a:extLst`.
const AFTER_SP3D_LOCALS: [&str; 1] = ["extLst"];

/// The `p:spTree` of a slide (`slide_root` is the `p:sld`).
pub(crate) fn sp_tree<'a>(
    slide_root: &'a RawElement,
    interner: &Interner,
) -> Result<&'a RawElement, PptxError> {
    let c_sld = nav::child(slide_root, interner, PML, "cSld")
        .ok_or(PptxError::MalformedSlide("missing p:cSld"))?;
    nav::child(c_sld, interner, PML, "spTree").ok_or(PptxError::MalformedSlide("missing p:spTree"))
}

/// The `p:spTree` of a slide, mutably.
pub(crate) fn sp_tree_mut<'a>(
    slide_root: &'a mut RawElement,
    interner: &Interner,
) -> Result<&'a mut RawElement, PptxError> {
    let c_sld = nav::child_mut(slide_root, interner, PML, "cSld")
        .ok_or(PptxError::MalformedSlide("missing p:cSld"))?;
    nav::child_mut(c_sld, interner, PML, "spTree")
        .ok_or(PptxError::MalformedSlide("missing p:spTree"))
}

/// What kind of shape a `p:spTree` child is — the six alternatives of `CT_GroupShape`'s child choice
/// (`EG_ShapeElements`).
///
/// Shapes of every kind share one index space (see
/// [`Presentation::shape_count`](crate::Presentation::shape_count)), so this is how a caller tells
/// what it is addressing: a picture accepts the `p:spPr` surface (fill, outline, effects, geometry)
/// but has no text body, a group has no `p:spPr` at all, and so on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShapeKind {
    /// `p:sp` — an autoshape, text box, or placeholder (`CT_Shape`).
    Shape,
    /// `p:pic` — a picture (`CT_Picture`).
    Picture,
    /// `p:grpSp` — a group of shapes (`CT_GroupShape`). Its members are not on the top-level index
    /// space; they are addressed by descending into it with a [`ShapePath`](crate::ShapePath).
    GroupShape,
    /// `p:graphicFrame` — a frame holding a table, chart, or other graphical object
    /// (`CT_GraphicalObjectFrame`).
    GraphicFrame,
    /// `p:cxnSp` — a connector between two shapes (`CT_Connector`).
    ConnectionShape,
    /// `p:contentPart` — a reference to an external content part (`CT_Rel`).
    ContentPart,
}

impl ShapeKind {
    /// The kind named by a `p:spTree` child's local name, or `None` for anything that is not a shape
    /// (`p:nvGrpSpPr`, `p:grpSpPr`, `p:extLst`).
    pub(crate) fn from_local(local: &str) -> Option<Self> {
        match local {
            "sp" => Some(Self::Shape),
            "pic" => Some(Self::Picture),
            "grpSp" => Some(Self::GroupShape),
            "graphicFrame" => Some(Self::GraphicFrame),
            "cxnSp" => Some(Self::ConnectionShape),
            "contentPart" => Some(Self::ContentPart),
            _ => None,
        }
    }

    /// The element name this kind is written as, without its `p:` prefix (e.g. `pic`).
    #[must_use]
    pub fn wire(self) -> &'static str {
        match self {
            Self::Shape => "sp",
            Self::Picture => "pic",
            Self::GroupShape => "grpSp",
            Self::GraphicFrame => "graphicFrame",
            Self::ConnectionShape => "cxnSp",
            Self::ContentPart => "contentPart",
        }
    }
}

/// What a `p:graphicFrame` frames, told from its `a:graphicData@uri` — so a caller can distinguish
/// "not a table" from "a graphical object this library does not model yet".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GraphicFrameKind {
    /// A table (`a:tbl`) — the one kind with a full surface here.
    Table,
    /// A chart — modeled in Phase 6 (`mjx-chart`), not reachable through this crate yet.
    Chart,
    /// A SmartArt diagram — unscheduled.
    Diagram,
    /// A legacy embedded **OLE object** (`p:oleObj`) — an embedded document or OLE stream in a separate
    /// part, drawn from a fallback image snapshot. Preserve-first: recognized and readable, not modeled.
    OleObject,
    /// Some other graphical object (an unknown extension): the frame states a `uri` this does not
    /// recognize.
    Other,
}

impl GraphicFrameKind {
    /// The kind a graphic frame's `a:graphicData@uri` names.
    #[must_use]
    pub(crate) fn from_uri(uri: &str) -> Self {
        match uri {
            TABLE_GRAPHIC_URI => Self::Table,
            CHART_GRAPHIC_URI => Self::Chart,
            DIAGRAM_GRAPHIC_URI => Self::Diagram,
            OLE_GRAPHIC_URI => Self::OleObject,
            _ => Self::Other,
        }
    }
}

/// The kind of a shape-tree child, or `None` if it is not a PresentationML shape element.
pub(crate) fn shape_kind(element: &RawElement, interner: &Interner) -> Option<ShapeKind> {
    let namespace = element
        .name
        .namespace
        .map(|symbol| interner.resolve(symbol));
    if namespace != Some(PML.transitional) && namespace != PML.strict {
        return None;
    }
    ShapeKind::from_local(interner.resolve(element.name.local))
}

/// The shape children of a shape tree, of **every** [`ShapeKind`], in document order — one index
/// space, so a picture is simply shape *n* on the slide.
///
/// Skips the tree's own `p:nvGrpSpPr` / `p:grpSpPr` / `p:extLst`, and does not descend into a
/// `p:grpSp`: a group is one shape here. Because the same shape-kind filter enumerates a group's own
/// members, this doubles as the member enumerator that [`resolve_shape`] descends through.
pub(crate) fn shapes<'a>(
    sp_tree: &'a RawElement,
    interner: &'a Interner,
) -> impl Iterator<Item = &'a RawElement> {
    sp_tree.children.iter().filter_map(move |node| match node {
        RawNode::Element(element) if shape_kind(element, interner).is_some() => Some(element),
        _ => None,
    })
}

/// The `n`-th shape of a shape tree, mutably — the same index space [`shapes`] enumerates.
pub(crate) fn nth_shape_mut<'a>(
    sp_tree: &'a mut RawElement,
    interner: &Interner,
    n: usize,
) -> Option<&'a mut RawElement> {
    nav::nth_child_matching_mut(sp_tree, interner, n, |element, interner| {
        shape_kind(element, interner).is_some()
    })
}

/// The position of the `n`-th shape in `sp_tree.children` — the same index space [`shapes`]
/// enumerates, mapped back onto the raw child list so the shape can be removed.
pub(crate) fn nth_shape_position(
    sp_tree: &RawElement,
    interner: &Interner,
    n: usize,
) -> Option<usize> {
    nav::nth_child_matching_position(sp_tree, interner, n, |element, interner| {
        shape_kind(element, interner).is_some()
    })
}

// ---------------------------------------------------------------------------------------------
// Descent — resolving a `ShapePath` through nested groups
// ---------------------------------------------------------------------------------------------
//
// A group's members are exactly `shapes(group_element)`: the same shape-kind filter that enumerates
// a shape tree's top-level children enumerates a `p:grpSp`'s members, because `shape_kind` already
// rejects the group's own `p:nvGrpSpPr` / `p:grpSpPr` / `p:extLst`. A shape that is *not* a group has
// no shape-kind children at all, so descending into one finds zero members — which is exactly the
// "that index has nothing to descend into" answer we want, with no special-casing. Every resolver
// therefore treats "the members of X" as "the shapes of X", uniformly, at every depth.
//
// On a miss each resolver returns the number of addressable shapes at the container where the walk
// stopped, so the caller can report `0..count` for the level that was actually out of range.

/// Resolves a [`ShapePath`] to the shape it addresses, descending through nested groups.
///
/// `Err(count)` carries the number of shapes in the container where the walk ran out of range — the
/// top-level tree for a length-1 path, or the group reached for a deeper one. Stepping into a
/// non-group leaves a zero-member container, so the next index misses with `count == 0`.
pub(crate) fn resolve_shape<'a>(
    sp_tree: &'a RawElement,
    interner: &'a Interner,
    path: &ShapePath,
) -> Result<&'a RawElement, usize> {
    let indices = path.indices();
    if indices.is_empty() {
        return Err(shapes(sp_tree, interner).count());
    }
    let mut container: &'a RawElement = sp_tree;
    for (depth, &index) in indices.iter().enumerate() {
        match shapes(container, interner).nth(index) {
            Some(shape) if depth + 1 == indices.len() => return Ok(shape),
            Some(shape) => container = shape,
            None => return Err(shapes(container, interner).count()),
        }
    }
    unreachable!("the loop returns once it reaches the last index")
}

/// Resolves a [`ShapePath`] to the shape it addresses, mutably — the write counterpart of
/// [`resolve_shape`], with the same `Err(count)` contract.
pub(crate) fn resolve_shape_mut<'a>(
    sp_tree: &'a mut RawElement,
    interner: &Interner,
    path: &ShapePath,
) -> Result<&'a mut RawElement, usize> {
    let indices = path.indices();
    if indices.is_empty() {
        return Err(shapes(sp_tree, interner).count());
    }
    let mut container: &'a mut RawElement = sp_tree;
    for (depth, &index) in indices.iter().enumerate() {
        // The count is read before the mutable borrow, since the miss branch cannot borrow the
        // container again once `nth_shape_mut` has taken it.
        let count = shapes(container, interner).count();
        let last = depth + 1 == indices.len();
        match nth_shape_mut(container, interner, index) {
            Some(shape) if last => return Ok(shape),
            Some(shape) => container = shape,
            None => return Err(count),
        }
    }
    unreachable!("the loop returns once it reaches the last index")
}

/// Resolves a [`ShapePath`] to the shape's **parent container** and its position in that container's
/// raw child list — what removal needs, since a shape is dropped from its parent's `children`, not
/// edited in place. For a top-level shape the parent is the shape tree itself.
///
/// `Err(count)` follows [`resolve_shape`]: the addressable-shape count at the level that missed.
pub(crate) fn resolve_shape_position<'a>(
    sp_tree: &'a mut RawElement,
    interner: &Interner,
    path: &ShapePath,
) -> Result<(&'a mut RawElement, usize), usize> {
    let indices = path.indices();
    let Some((&last_index, parent_indices)) = indices.split_last() else {
        return Err(shapes(sp_tree, interner).count());
    };
    let parent: &'a mut RawElement = if parent_indices.is_empty() {
        sp_tree
    } else {
        resolve_shape_mut(sp_tree, interner, &ShapePath::from(parent_indices))?
    };
    let count = shapes(parent, interner).count();
    match nth_shape_position(parent, interner, last_index) {
        Some(position) => Ok((parent, position)),
        None => Err(count),
    }
}

/// A shape's `p:txBody`, if it has one.
pub(crate) fn shape_txbody<'a>(
    shape: &'a RawElement,
    interner: &Interner,
) -> Option<&'a RawElement> {
    nav::child(shape, interner, PML, "txBody")
}

/// The `a:graphicData@uri` a graphic frame carries when what it frames is a table. A frame holding
/// a chart or a SmartArt diagram names a different one, which is how they are told apart without
/// looking at the payload.
pub(crate) const TABLE_GRAPHIC_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/table";

/// The `a:graphicData@uri` of a frame holding a chart.
pub(crate) const CHART_GRAPHIC_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";

/// The `a:graphicData@uri` of a frame holding a SmartArt diagram.
pub(crate) const DIAGRAM_GRAPHIC_URI: &str =
    "http://schemas.openxmlformats.org/drawingml/2006/diagram";

/// The `a:graphicData@uri` of a frame holding a legacy OLE object (`p:oleObj`).
pub(crate) const OLE_GRAPHIC_URI: &str =
    "http://schemas.openxmlformats.org/presentationml/2006/ole";

/// The Markup Compatibility namespace as a [`SchemaNamespace`] (single URI, no Strict variant) so the
/// `nav` helpers can match `mc:AlternateContent` / `mc:Choice` / `mc:Fallback` the same way they match
/// schema elements. An OLE object's `p:oleObj` is wrapped in this MCE machinery in real decks.
pub(crate) const MCE: SchemaNamespace = SchemaNamespace {
    transitional: MARKUP_COMPATIBILITY_2006,
    strict: None,
};

/// The `a:graphicData@uri` a graphic frame declares — what kind of object it frames — or `None` when
/// the shape is not a `p:graphicFrame` or the frame states no `uri`.
pub(crate) fn graphic_frame_uri<'a>(
    shape: &'a RawElement,
    interner: &'a Interner,
) -> Option<&'a str> {
    let graphic = nav::child(shape, interner, DML_MAIN, "graphic")?;
    let data = nav::child(graphic, interner, DML_MAIN, "graphicData")?;
    nav::attr_value(data, interner, "uri")
}

/// The table a shape frames (`p:graphicFrame > a:graphic > a:graphicData > a:tbl`), if it is a
/// graphic frame and what it frames is a table.
///
/// `None` for any other shape kind, and for a frame holding a chart or diagram instead — the
/// element is looked for rather than the `uri` trusted, since it is the payload that decides.
pub(crate) fn shape_table<'a>(
    shape: &'a RawElement,
    interner: &Interner,
) -> Option<&'a RawElement> {
    let graphic = nav::child(shape, interner, DML_MAIN, "graphic")?;
    let data = nav::child(graphic, interner, DML_MAIN, "graphicData")?;
    nav::child(data, interner, DML_MAIN, "tbl")
}

/// The table a shape frames, mutably.
pub(crate) fn shape_table_mut<'a>(
    shape: &'a mut RawElement,
    interner: &Interner,
) -> Option<&'a mut RawElement> {
    let graphic = nav::child_mut(shape, interner, DML_MAIN, "graphic")?;
    let data = nav::child_mut(graphic, interner, DML_MAIN, "graphicData")?;
    nav::child_mut(data, interner, DML_MAIN, "tbl")
}

/// The relationship id a chart frame names (`p:graphicFrame > a:graphic > a:graphicData >
/// c:chart@r:id`), or `None` for any other shape kind and for a frame holding a table or diagram
/// instead — the `c:chart` element is looked for rather than the `uri` trusted, since it is the
/// payload that decides.
///
/// The `r:id` is matched by local name — a `c:chart` in a graphic frame carries no other `id`-local
/// attribute — so the relationships prefix need not be resolved (as with `a:hlinkClick`, see
/// [`shape_hyperlink_rel_id`]).
pub(crate) fn chart_rel_id<'a>(shape: &'a RawElement, interner: &Interner) -> Option<&'a str> {
    let graphic = nav::child(shape, interner, DML_MAIN, "graphic")?;
    let data = nav::child(graphic, interner, DML_MAIN, "graphicData")?;
    let chart = nav::child(data, interner, DML_CHART, "chart")?;
    chart
        .attributes
        .iter()
        .find(|attr| interner.resolve(attr.name.local) == "id")
        .and_then(|attr| std::str::from_utf8(&attr.value).ok())
}

/// The `p:oleObj` a graphic frame frames, or `None` for any other shape.
///
/// Unlike a chart (`c:chart` directly under `a:graphicData`), an OLE object is wrapped in the MCE
/// machinery: `a:graphicData > mc:AlternateContent > mc:Choice`/`mc:Fallback > p:oleObj`. This finds
/// it whether it sits directly under `graphicData` (rare) or in an `mc:Choice` (preferred, the live
/// object) or `mc:Fallback` branch. Preserve-first: the element is returned raw, never resolved.
pub(crate) fn ole_object<'a>(
    shape: &'a RawElement,
    interner: &'a Interner,
) -> Option<&'a RawElement> {
    let graphic = nav::child(shape, interner, DML_MAIN, "graphic")?;
    let data = nav::child(graphic, interner, DML_MAIN, "graphicData")?;
    if let Some(ole) = nav::child(data, interner, PML, "oleObj") {
        return Some(ole);
    }
    let alternate = nav::child(data, interner, MCE, "AlternateContent")?;
    ["Choice", "Fallback"].into_iter().find_map(|branch| {
        nav::children(alternate, interner, MCE, branch)
            .find_map(|candidate| nav::child(candidate, interner, PML, "oleObj"))
    })
}

/// The `p:oleObj` a graphic frame frames, mutably — the write counterpart of [`ole_object`], with the
/// same MCE-aware walk (`a:graphicData` directly, or through an `mc:Choice` / `mc:Fallback` branch).
pub(crate) fn ole_object_mut<'a>(
    shape: &'a mut RawElement,
    interner: &Interner,
) -> Option<&'a mut RawElement> {
    let graphic = nav::child_mut(shape, interner, DML_MAIN, "graphic")?;
    let data = nav::child_mut(graphic, interner, DML_MAIN, "graphicData")?;
    if nav::child(data, interner, PML, "oleObj").is_some() {
        return nav::child_mut(data, interner, PML, "oleObj");
    }
    let alternate = nav::child_mut(data, interner, MCE, "AlternateContent")?;
    let position = alternate.children.iter().position(|node| match node {
        RawNode::Element(branch) => {
            (nav::name_is(&branch.name, interner, MCE, "Choice")
                || nav::name_is(&branch.name, interner, MCE, "Fallback"))
                && nav::child(branch, interner, PML, "oleObj").is_some()
        }
        _ => false,
    })?;
    let RawNode::Element(branch) = &mut alternate.children[position] else {
        return None;
    };
    nav::child_mut(branch, interner, PML, "oleObj")
}

/// The relationship id an OLE frame names for its embedded object part (`p:oleObj@r:id`), or `None` for
/// any other shape. Matched by local name, as [`chart_rel_id`].
pub(crate) fn ole_object_rel_id<'a>(
    shape: &'a RawElement,
    interner: &'a Interner,
) -> Option<&'a str> {
    ole_object(shape, interner)?
        .attributes
        .iter()
        .find(|attr| interner.resolve(attr.name.local) == "id")
        .and_then(|attr| std::str::from_utf8(&attr.value).ok())
}

/// The relationship id an OLE frame names for its object *data* — its embedded object
/// (`p:oleObj@r:id`) or a linked one (`p:oleObj@r:link`), whichever is present — or `None` for any
/// other shape. Matched by local name (the relationships prefix is left unresolved), as
/// [`ole_object_rel_id`].
pub(crate) fn ole_object_data_rel_id<'a>(
    shape: &'a RawElement,
    interner: &'a Interner,
) -> Option<&'a str> {
    ole_object(shape, interner)?
        .attributes
        .iter()
        .find(|attr| matches!(interner.resolve(attr.name.local), "id" | "link"))
        .and_then(|attr| std::str::from_utf8(&attr.value).ok())
}

/// The relationship id of a **fallback snapshot** image nested directly in `container`
/// (`container > p:pic > p:blipFill > a:blip@r:embed`), or `None` if there is no such snapshot. Shared
/// by OLE objects and ActiveX controls, which both carry a `p:pic` snapshot the same way — the image a
/// renderer draws in place of the (unexecuted) object or control.
pub(crate) fn pic_snapshot_rel_id<'a>(
    container: &'a RawElement,
    interner: &'a Interner,
) -> Option<&'a str> {
    let picture = nav::child(container, interner, PML, "pic")?;
    let blip_fill = nav::child(picture, interner, PML, "blipFill")?;
    let blip = nav::child(blip_fill, interner, DML_MAIN, "blip")?;
    blip.attributes
        .iter()
        .find(|attr| interner.resolve(attr.name.local) == "embed")
        .and_then(|attr| std::str::from_utf8(&attr.value).ok())
}

/// The relationship id of an OLE object's fallback snapshot image
/// (`p:oleObj > p:pic > p:blipFill > a:blip@r:embed`), or `None` if the frame carries no such snapshot.
pub(crate) fn ole_snapshot_rel_id<'a>(
    shape: &'a RawElement,
    interner: &'a Interner,
) -> Option<&'a str> {
    pic_snapshot_rel_id(ole_object(shape, interner)?, interner)
}

/// The `progId` an OLE frame declares (e.g. `Excel.Sheet.12`) — the app that owns the embedded object —
/// or `None` if absent. An unprefixed attribute, matched by local name.
pub(crate) fn ole_prog_id<'a>(shape: &'a RawElement, interner: &'a Interner) -> Option<&'a str> {
    nav::attr_value(ole_object(shape, interner)?, interner, "progId")
}

/// The ActiveX form controls a slide carries (`p:sld > p:cSld > p:controls > p:control`), in order.
///
/// A `p:control` is a sibling of `p:spTree` under `p:cSld`, so it is **not** in the shape index space —
/// controls are addressed by their own per-slide index. Empty when the slide has no `p:controls`.
pub(crate) fn controls<'a>(
    slide_root: &'a RawElement,
    interner: &'a Interner,
) -> impl Iterator<Item = &'a RawElement> {
    nav::child(slide_root, interner, PML, "cSld")
        .and_then(|c_sld| nav::child(c_sld, interner, PML, "controls"))
        .into_iter()
        .flat_map(move |controls| nav::children(controls, interner, PML, "control"))
}

/// The `idx`-th ActiveX `p:control` on a slide, or `None` when the slide has fewer.
pub(crate) fn nth_control<'a>(
    slide_root: &'a RawElement,
    interner: &'a Interner,
    idx: usize,
) -> Option<&'a RawElement> {
    controls(slide_root, interner).nth(idx)
}

/// The relationship id a `p:control` names for its ActiveX control part (`p:control@r:id`), or `None`.
/// Matched by local name, as [`chart_rel_id`].
pub(crate) fn control_rel_id<'a>(control: &'a RawElement, interner: &Interner) -> Option<&'a str> {
    control
        .attributes
        .iter()
        .find(|attr| interner.resolve(attr.name.local) == "id")
        .and_then(|attr| std::str::from_utf8(&attr.value).ok())
}

/// The `name` a `p:control` declares (e.g. `CommandButton1`), or `None`. An unprefixed attribute,
/// matched by local name.
pub(crate) fn control_name<'a>(control: &'a RawElement, interner: &'a Interner) -> Option<&'a str> {
    nav::attr_value(control, interner, "name")
}

/// The `p:controls` container under a slide's `p:cSld`, mutably, creating it when the slide has
/// none — inserted directly after `p:spTree`, which is where `CT_CommonSlideData`'s content order
/// puts it (`bg?`, `spTree`, `custDataLst?`, `controls?`, `extLst?`).
pub(crate) fn controls_mut<'a>(
    slide_root: &'a mut RawElement,
    interner: &mut Interner,
) -> Result<&'a mut RawElement, PptxError> {
    let c_sld = nav::child_mut(slide_root, interner, PML, "cSld")
        .ok_or(PptxError::MalformedSlide("missing p:cSld"))?;
    if nav::child(c_sld, interner, PML, "controls").is_none() {
        let controls = build::node(interner, "p", PML, "controls", Vec::new(), Vec::new());
        // After `p:spTree` (and any `p:custDataLst`), before `p:extLst`.
        let at = c_sld
            .children
            .iter()
            .position(|node| match node {
                RawNode::Element(child) => nav::name_is(&child.name, interner, PML, "extLst"),
                _ => false,
            })
            .unwrap_or(c_sld.children.len());
        c_sld.children.insert(at, RawNode::Element(controls));
        c_sld.empty = false;
    }
    nav::child_mut(c_sld, interner, PML, "controls")
        .ok_or(PptxError::MalformedSlide("missing p:controls"))
}

/// The `idx`-th ActiveX `p:control` on a slide, mutably.
pub(crate) fn nth_control_mut<'a>(
    slide_root: &'a mut RawElement,
    interner: &Interner,
    idx: usize,
) -> Option<&'a mut RawElement> {
    let c_sld = nav::child_mut(slide_root, interner, PML, "cSld")?;
    let controls = nav::child_mut(c_sld, interner, PML, "controls")?;
    nav::nth_child_matching_mut(controls, interner, idx, |element, interner| {
        nav::name_is(&element.name, interner, PML, "control")
    })
}

/// The position of the `idx`-th `p:control` in its `p:controls` container's child list, so the
/// control can be removed with its own indentation.
pub(crate) fn nth_control_position(
    controls: &RawElement,
    interner: &Interner,
    idx: usize,
) -> Option<usize> {
    nav::nth_child_matching_position(controls, interner, idx, |element, interner| {
        nav::name_is(&element.name, interner, PML, "control")
    })
}

/// The `spid` a `p:control` or a `p:oleObj` names — the `id` of the VML shape that draws it in a
/// legacy consumer (`a:ST_ShapeID`). An unprefixed attribute, matched by local name.
pub(crate) fn legacy_shape_id<'a>(
    element: &'a RawElement,
    interner: &'a Interner,
) -> Option<&'a str> {
    nav::attr_value(element, interner, "spid")
}

/// Whether `element` is a content part that references a separate part by relationship id — either
/// PresentationML's own `p:contentPart` (`CT_Rel`) or the Office 2010 `p14:contentPart` producers
/// wrap in `mc:AlternateContent`.
fn is_content_part(element: &RawElement, interner: &Interner) -> bool {
    if interner.resolve(element.name.local) != "contentPart" {
        return false;
    }
    let namespace = element
        .name
        .namespace
        .map(|symbol| interner.resolve(symbol));
    namespace == Some(PML.transitional)
        || namespace == PML.strict
        || namespace == Some(crate::constants::POWERPOINT_2010_NAMESPACE)
}

/// The relationship id a content part names (`@r:id`), or `None`. Matched by local name — a
/// `contentPart` carries no other `id`-local attribute.
pub(crate) fn content_part_rel_id<'a>(
    element: &'a RawElement,
    interner: &Interner,
) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attr| interner.resolve(attr.name.local) == "id")
        .and_then(|attr| std::str::from_utf8(&attr.value).ok())
}

/// Every content part a shape tree references, as `(shape index, relationship id)`.
///
/// A `p:contentPart` is a shape like any other, so it has an index in the one shape index space. A
/// `p14:contentPart` is not: producers wrap it in `mc:AlternateContent`, which sits beside the shapes
/// rather than among them, so its index is `None`. Both are found, because both are how a deck
/// references ink.
pub(crate) fn content_part_references(
    sp_tree: &RawElement,
    interner: &Interner,
) -> Vec<(Option<usize>, String)> {
    let mut found = Vec::new();
    let mut shape_index = 0usize;
    for node in &sp_tree.children {
        let RawNode::Element(element) = node else {
            continue;
        };
        if shape_kind(element, interner).is_some() {
            if is_content_part(element, interner) {
                if let Some(rel_id) = content_part_rel_id(element, interner) {
                    found.push((Some(shape_index), rel_id.to_owned()));
                }
            }
            shape_index += 1;
            continue;
        }
        if nav::name_is(&element.name, interner, MCE, "AlternateContent") {
            collect_alternate_content_parts(element, interner, &mut found);
        }
    }
    found
}

/// Appends every content part inside an `mc:AlternateContent`'s branches to `found`, with no shape
/// index — the branches are not in the shape index space.
fn collect_alternate_content_parts(
    alternate: &RawElement,
    interner: &Interner,
    found: &mut Vec<(Option<usize>, String)>,
) {
    for branch in ["Choice", "Fallback"] {
        for candidate in nav::children(alternate, interner, MCE, branch) {
            for node in &candidate.children {
                let RawNode::Element(element) = node else {
                    continue;
                };
                if is_content_part(element, interner) {
                    if let Some(rel_id) = content_part_rel_id(element, interner) {
                        found.push((None, rel_id.to_owned()));
                    }
                }
            }
        }
    }
}

/// The `dgm:relIds` a SmartArt frame carries (`p:graphicFrame > a:graphic > a:graphicData >
/// dgm:relIds`), or `None` for any other shape — the element is looked for rather than the frame's
/// `uri` trusted, since it is the payload that decides.
pub(crate) fn diagram_rel_ids<'a>(
    shape: &'a RawElement,
    interner: &Interner,
) -> Option<&'a RawElement> {
    let graphic = nav::child(shape, interner, DML_MAIN, "graphic")?;
    let data = nav::child(graphic, interner, DML_MAIN, "graphicData")?;
    nav::child(data, interner, DML_DIAGRAM, "relIds")
}

/// The value of the relationship-reference attribute `local` on a `dgm:relIds` (`r:dm`, `r:lo`,
/// `r:qs`, `r:cs`).
///
/// Unlike a `c:chart@r:id`, these four share their element with each other, so matching by local name
/// alone is enough to tell them apart and the relationships prefix need not be resolved.
pub(crate) fn diagram_rel_id<'a>(
    rel_ids: &'a RawElement,
    interner: &Interner,
    local: &str,
) -> Option<&'a str> {
    rel_ids
        .attributes
        .iter()
        .find(|attr| interner.resolve(attr.name.local) == local)
        .and_then(|attr| std::str::from_utf8(&attr.value).ok())
}

/// The `n`-th `a:tr` of a table, mutably.
pub(crate) fn nth_row_mut<'a>(
    table: &'a mut RawElement,
    interner: &Interner,
    n: usize,
) -> Option<&'a mut RawElement> {
    nav::nth_child_matching_mut(table, interner, n, |element, interner| {
        nav::name_is(&element.name, interner, DML_MAIN, "tr")
    })
}

/// The `n`-th `a:tc` of a row, mutably. A cell covered by a merge is a cell like any other, so the
/// index space has no holes.
pub(crate) fn nth_cell_mut<'a>(
    row: &'a mut RawElement,
    interner: &Interner,
    n: usize,
) -> Option<&'a mut RawElement> {
    nav::nth_child_matching_mut(row, interner, n, |element, interner| {
        nav::name_is(&element.name, interner, DML_MAIN, "tc")
    })
}

/// The `n`-th child of `parent` named `(DML_MAIN, local)`.
pub(crate) fn nth_dml_child<'a>(
    parent: &'a RawElement,
    interner: &Interner,
    local: &str,
    n: usize,
) -> Option<&'a RawElement> {
    parent
        .children
        .iter()
        .filter_map(|node| match node {
            RawNode::Element(element) if nav::name_is(&element.name, interner, DML_MAIN, local) => {
                Some(element)
            }
            _ => None,
        })
        .nth(n)
}

/// A shape's preset geometry (`p:spPr > a:prstGeom`), if it has one. A shape with custom geometry
/// (`a:custGeom`) or an inherited placeholder geometry returns `None`.
pub(crate) fn shape_prstgeom<'a>(
    shape: &'a RawElement,
    interner: &Interner,
) -> Option<&'a RawElement> {
    let sp_pr = nav::child(shape, interner, PML, "spPr")?;
    nav::child(sp_pr, interner, DML_MAIN, "prstGeom")
}

/// A shape's custom geometry (`p:spPr > a:custGeom`), if it has one. A preset or inherited-geometry
/// shape returns `None`. The mutually-exclusive counterpart to [`shape_prstgeom`].
pub(crate) fn shape_custgeom<'a>(
    shape: &'a RawElement,
    interner: &Interner,
) -> Option<&'a RawElement> {
    let sp_pr = nav::child(shape, interner, PML, "spPr")?;
    nav::child(sp_pr, interner, DML_MAIN, "custGeom")
}

// ---------------------------------------------------------------------------------------------
// The transform (`a:xfrm`) — the one property that is not in the same place for every shape kind
// ---------------------------------------------------------------------------------------------
//
// Fill, outline, effects and geometry all live in `p:spPr`, so their accessors can go straight
// there. A transform cannot: a group keeps its own in `p:grpSpPr` (as a `CT_GroupTransform2D`, with
// the child coordinate space its members are laid out in), and a graphic frame keeps its own as a
// **`p:xfrm`** — PresentationML's namespace, a direct child, and required by the schema rather than
// optional. Only its wrapper differs; the `a:off`/`a:ext` inside are DrawingML in every case, which
// is why one `Transform2D` reads them all.
//
// A `p:contentPart` is a reference to an external part (`CT_Rel`) and has no transform at all.

/// Where a shape of `kind` keeps its transform: the local name of the container to look inside
/// (`None` means the shape element itself), and the qualified name of the transform element.
///
/// `None` for a kind that cannot carry one.
fn transform_location(kind: ShapeKind) -> Option<(Option<&'static str>, TransformName)> {
    match kind {
        // `p:spPr > a:xfrm` — `CT_Transform2D`.
        ShapeKind::Shape | ShapeKind::Picture | ShapeKind::ConnectionShape => {
            Some((Some("spPr"), TransformName::DrawingMl))
        }
        // `p:grpSpPr > a:xfrm` — `CT_GroupTransform2D`, carrying `a:chOff` / `a:chExt`.
        ShapeKind::GroupShape => Some((Some("grpSpPr"), TransformName::DrawingMl)),
        // `p:graphicFrame > p:xfrm` — a direct child, in PresentationML's own namespace.
        ShapeKind::GraphicFrame => Some((None, TransformName::PresentationMl)),
        // `CT_Rel` has nowhere to put one.
        ShapeKind::ContentPart => None,
    }
}

/// Which namespace a shape kind's transform element is written in. Its children are DrawingML
/// either way — this is only about the wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransformName {
    /// `a:xfrm`, the usual case.
    DrawingMl,
    /// `p:xfrm`, which only a `p:graphicFrame` uses.
    PresentationMl,
}

impl TransformName {
    /// The namespace the transform element is named in.
    fn namespace(self) -> SchemaNamespace {
        match self {
            Self::DrawingMl => DML_MAIN,
            Self::PresentationMl => PML,
        }
    }

    /// The prefix a newly built transform element is written with.
    fn prefix(self) -> &'static str {
        match self {
            Self::DrawingMl => "a",
            Self::PresentationMl => "p",
        }
    }
}

/// A shape's transform element, whichever of the three places its kind keeps it in, or `None` when
/// it declares none (its position is then inherited) or its kind cannot carry one.
pub(crate) fn shape_transform<'a>(
    shape: &'a RawElement,
    interner: &Interner,
) -> Option<&'a RawElement> {
    let kind = shape_kind(shape, interner)?;
    let (container, name) = transform_location(kind)?;
    let holder = match container {
        Some(local) => nav::child(shape, interner, PML, local)?,
        None => shape,
    };
    nav::child(holder, interner, name.namespace(), "xfrm")
}

/// A shape's transform element, mutably — **creating an empty one** in the right container, at its
/// rank in that container's sequence, when the shape declares none.
///
/// This is the whole write path's knowledge of where a transform lives, so no caller repeats it.
/// The returned element is ready for
/// [`Transform2D::apply`](mjx_dml::Transform2D::apply), which fills in only the fields being set.
///
/// # Errors
/// [`ShapeCannotBePositioned`](PptxError::ShapeCannotBePositioned) for a `p:contentPart`, which has
/// no transform in its schema, and [`ShapeHasNoProperties`](PptxError::ShapeHasNoProperties) for a
/// shape missing the `p:spPr` / `p:grpSpPr` its transform would live in.
pub(crate) fn shape_transform_slot_mut<'a>(
    shape: &'a mut RawElement,
    interner: &mut Interner,
) -> Result<&'a mut RawElement, PptxError> {
    let kind = shape_kind(shape, interner).ok_or(PptxError::MalformedSlide("not a shape"))?;
    let (container, name) =
        transform_location(kind).ok_or(PptxError::ShapeCannotBePositioned { kind })?;

    let holder = match container {
        Some(local) => {
            nav::child_mut(shape, interner, PML, local).ok_or(PptxError::ShapeHasNoProperties)?
        }
        None => shape,
    };

    // Insert at the transform's rank in the holder's generated `xsd:sequence`, rather than
    // appending: in both `CT_ShapeProperties` and `CT_GroupShapeProperties` the transform is the
    // *first* member, and in `CT_GraphicalObjectFrame` it follows only `p:nvGraphicFramePr`. Order
    // is validity here, not style.
    let namespace = name.namespace();
    let existing = holder.children.iter().position(|node| match node {
        RawNode::Element(element) => nav::name_is(&element.name, interner, namespace, "xfrm"),
        _ => false,
    });
    let index = match existing {
        Some(index) => index,
        None => {
            let order =
                transform_holder_order(kind).ok_or(PptxError::ShapeCannotBePositioned { kind })?;
            let at = order.insert_index(&holder.children, interner, "xfrm");
            let element = build::node(interner, name.prefix(), namespace, "xfrm", vec![], vec![]);
            holder.children.insert(at, RawNode::Element(element));
            holder.empty = false;
            at
        }
    };
    match &mut holder.children[index] {
        RawNode::Element(element) => Ok(element),
        _ => Err(PptxError::MalformedSlide(
            "transform slot is not an element",
        )),
    }
}

/// The generated child order of the element a shape of `kind` keeps its transform in.
///
/// `p:spPr` is `CT_ShapeProperties`, `p:grpSpPr` is `CT_GroupShapeProperties`, and a
/// `p:graphicFrame` holds its `p:xfrm` directly, so the order is `CT_GraphicalObjectFrame`'s.
fn transform_holder_order(kind: ShapeKind) -> Option<&'static ChildOrder> {
    match kind {
        ShapeKind::Shape | ShapeKind::Picture | ShapeKind::ConnectionShape => {
            Some(SHAPE_PROPERTIES)
        }
        ShapeKind::GroupShape => Some(GROUP_SHAPE_PROPERTIES),
        ShapeKind::GraphicFrame => Some(GRAPHIC_FRAME),
        ShapeKind::ContentPart => None,
    }
}

/// Parses a `p:clrMap` / `a:overrideClrMapping` element into a [`ColorMap`] — the twelve logical
/// color-name attributes, each a `ST_ColorSchemeIndex` token. Returns `None` if any of the twelve is
/// absent or unrecognized (e.g. the schema-loose attribute-less `overrideClrMapping` some files emit).
pub(crate) fn parse_color_map(element: &RawElement, interner: &Interner) -> Option<ColorMap> {
    let slot =
        |local| nav::attr_value(element, interner, local).and_then(ColorSchemeSlot::from_wire);
    Some(ColorMap {
        background1: slot("bg1")?,
        text1: slot("tx1")?,
        background2: slot("bg2")?,
        text2: slot("tx2")?,
        accent1: slot("accent1")?,
        accent2: slot("accent2")?,
        accent3: slot("accent3")?,
        accent4: slot("accent4")?,
        accent5: slot("accent5")?,
        accent6: slot("accent6")?,
        hyperlink: slot("hlink")?,
        followed_hyperlink: slot("folHlink")?,
    })
}

/// The local name of `node` if it is a DrawingML-main element (accepting both URIs), else `None`.
fn dml_element_local<'a>(node: &'a RawNode, interner: &'a Interner) -> Option<&'a str> {
    match node {
        RawNode::Element(element) => {
            let namespace = element
                .name
                .namespace
                .map(|symbol| interner.resolve(symbol));
            if namespace == Some(DML_MAIN.transitional) || namespace == DML_MAIN.strict {
                Some(interner.resolve(element.name.local))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// A shape's explicit fill element (`p:spPr`'s `EG_FillProperties` child), if present. Returns `None`
/// when the shape has no `p:spPr` or no explicit fill (its fill is then inherited).
pub(crate) fn shape_fill<'a>(shape: &'a RawElement, interner: &Interner) -> Option<&'a RawElement> {
    let sp_pr = nav::child(shape, interner, PML, "spPr")?;
    sp_pr.children.iter().find_map(|node| match node {
        RawNode::Element(element)
            if dml_element_local(node, interner).is_some_and(Fill::is_fill_local) =>
        {
            Some(element)
        }
        _ => None,
    })
}

/// A shape's fill style reference (`p:sp > p:style > a:fillRef`), if it has one — the theme
/// fill-style index and the color that substitutes the style's `phClr`.
pub(crate) fn shape_fill_ref(
    shape: &RawElement,
    interner: &Interner,
) -> Option<StyleMatrixReference> {
    let style = nav::child(shape, interner, PML, "style")?;
    let fill_ref = nav::child(style, interner, DML_MAIN, "fillRef")?;
    StyleMatrixReference::from_xml(fill_ref, interner).ok()
}

/// A shape's outline style reference (`p:sp > p:style > a:lnRef`), if it has one — the theme
/// line-style index and the color that substitutes the style's `phClr`.
pub(crate) fn shape_line_ref(
    shape: &RawElement,
    interner: &Interner,
) -> Option<StyleMatrixReference> {
    let style = nav::child(shape, interner, PML, "style")?;
    let line_ref = nav::child(style, interner, DML_MAIN, "lnRef")?;
    StyleMatrixReference::from_xml(line_ref, interner).ok()
}

/// A shape's effect style reference (`p:sp > p:style > a:effectRef`), if it has one — the theme
/// effect-style index and the color that substitutes the style's `phClr`.
pub(crate) fn shape_effect_ref(
    shape: &RawElement,
    interner: &Interner,
) -> Option<StyleMatrixReference> {
    let style = nav::child(shape, interner, PML, "style")?;
    let effect_ref = nav::child(style, interner, DML_MAIN, "effectRef")?;
    StyleMatrixReference::from_xml(effect_ref, interner).ok()
}

/// A shape's placeholder identity (`p:nv*Pr > p:nvPr > p:ph`): what it holds (`@type`) and which
/// slot it occupies (`@idx`). Used to match a shape against the same-slot placeholder on the slide
/// layout / master when its own properties are inherited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Placeholder {
    /// What the placeholder holds (`@type`, default [`PlaceholderType::Object`] per the schema).
    pub kind: PlaceholderType,
    /// The placeholder index (`@idx`, default `0`).
    pub idx: u32,
}

impl Placeholder {
    /// Whether this is a title placeholder — `title` and `ctrTitle` share one inheritance slot.
    pub(crate) fn is_title_family(self) -> bool {
        matches!(
            self.kind,
            PlaceholderType::Title | PlaceholderType::CenteredTitle
        )
    }

    /// Whether `self` and `other` name the same inheritance slot: title-family placeholders match each
    /// other regardless of index; any other placeholder matches by index. (A documented heuristic —
    /// PowerPoint's exact matching has more nuance around body/object placeholders.)
    pub(crate) fn matches(self, other: Placeholder) -> bool {
        if self.is_title_family() || other.is_title_family() {
            self.is_title_family() && other.is_title_family()
        } else {
            self.idx == other.idx
        }
    }
}

/// The non-visual properties container of a shape of any kind — `p:nvSpPr` on a `p:sp`, `p:nvPicPr`
/// on a `p:pic`, and so on. Every kind names it differently but each holds the `p:nvPr` that carries
/// the placeholder.
fn non_visual_properties<'a>(shape: &'a RawElement, interner: &Interner) -> Option<&'a RawElement> {
    const CONTAINERS: [&str; 5] = [
        "nvSpPr",
        "nvPicPr",
        "nvGrpSpPr",
        "nvCxnSpPr",
        "nvGraphicFramePr",
    ];
    CONTAINERS
        .iter()
        .find_map(|local| nav::child(shape, interner, PML, local))
}

/// The non-visual properties container of a shape, mutably — the writer's counterpart to
/// [`non_visual_properties`].
fn non_visual_properties_mut<'a>(
    shape: &'a mut RawElement,
    interner: &Interner,
) -> Option<&'a mut RawElement> {
    const CONTAINERS: [&str; 5] = [
        "nvSpPr",
        "nvPicPr",
        "nvGrpSpPr",
        "nvCxnSpPr",
        "nvGraphicFramePr",
    ];
    // Find which container this shape kind uses (an immutable probe), then borrow it mutably.
    let local = CONTAINERS
        .iter()
        .find(|local| nav::child(shape, interner, PML, local).is_some())?;
    nav::child_mut(shape, interner, PML, local)
}

/// The relationship id of a shape's click hyperlink (`p:cNvPr > a:hlinkClick@r:id`), or `None` if the
/// shape has no hyperlink. The `r:id` is matched by local name — an `a:hlinkClick` carries no other
/// `id`-local attribute, so the relationship prefix need not be resolved.
pub(crate) fn shape_hyperlink_rel_id<'a>(
    shape: &'a RawElement,
    interner: &Interner,
) -> Option<&'a str> {
    let nv = non_visual_properties(shape, interner)?;
    let c_nv_pr = nav::child(nv, interner, PML, "cNvPr")?;
    let hlink = nav::child(c_nv_pr, interner, DML_MAIN, "hlinkClick")?;
    hlink
        .attributes
        .iter()
        .find(|attr| interner.resolve(attr.name.local) == "id")
        .and_then(|attr| std::str::from_utf8(&attr.value).ok())
}

/// Sets (or, when both are `None`, clears) a shape's click hyperlink (`p:cNvPr > a:hlinkClick`),
/// naming relationship `rel_id` — written with the literal `rel_prefix` bound to the relationships
/// namespace — and/or carrying `action`. Any existing `a:hlinkClick` is replaced. `a:hlinkClick` is
/// the first child `CT_NonVisualDrawingProps` allows, so a fresh one is inserted at the front.
pub(crate) fn set_shape_hyperlink(
    shape: &mut RawElement,
    interner: &mut Interner,
    rel_prefix: Symbol,
    rel_id: Option<&str>,
    action: Option<&str>,
) -> Result<(), PptxError> {
    let nv = non_visual_properties_mut(shape, interner).ok_or(PptxError::MalformedSlide(
        "shape has no non-visual properties",
    ))?;
    let c_nv_pr = nav::child_mut(nv, interner, PML, "cNvPr")
        .ok_or(PptxError::MalformedSlide("shape has no p:cNvPr"))?;
    c_nv_pr.children.retain(|node| {
        !matches!(node, RawNode::Element(child)
            if nav::name_is(&child.name, interner, DML_MAIN, "hlinkClick"))
    });
    if rel_id.is_none() && action.is_none() {
        c_nv_pr.empty = c_nv_pr.children.is_empty();
        return Ok(());
    }
    let mut attributes = Vec::new();
    if let Some(rel_id) = rel_id {
        attributes.push(build::attr_prefixed(interner, rel_prefix, "id", rel_id));
    }
    if let Some(action) = action {
        attributes.push(build::attr(interner, "action", action));
    }
    let hlink = build::leaf(interner, "a", DML_MAIN, "hlinkClick", attributes);
    c_nv_pr.children.insert(0, RawNode::Element(hlink));
    c_nv_pr.empty = false;
    Ok(())
}

/// The placeholder identity of `shape` (`p:nv*Pr > p:nvPr > p:ph`), or `None` if it is not a
/// placeholder shape. Pictures and graphic frames can be placeholders too, so this accepts the
/// non-visual container of any shape kind.
pub(crate) fn shape_placeholder(shape: &RawElement, interner: &Interner) -> Option<Placeholder> {
    let nv_sp_pr = non_visual_properties(shape, interner)?;
    let nv_pr = nav::child(nv_sp_pr, interner, PML, "nvPr")?;
    let ph = nav::child(nv_pr, interner, PML, "ph")?;
    let kind = nav::attr_value(ph, interner, "type")
        .and_then(PlaceholderType::from_wire)
        .unwrap_or(PlaceholderType::Object);
    let idx = nav::attr_value(ph, interner, "idx")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    Some(Placeholder { kind, idx })
}

/// Whether `shape` is a **text box** (`p:cNvSpPr@txBox="1"`) rather than an ordinary shape.
///
/// PowerPoint styles the two differently: ECMA-376 Part 1 §19.3.1.35 says a master's `p:otherStyle`
/// applies "within a slide shape but not within a text box. Text box styling is handled from within
/// the `bodyStyle` element." A shape kind that has no `p:cNvSpPr` at all is not a text box.
pub(crate) fn shape_is_text_box(shape: &RawElement, interner: &Interner) -> bool {
    let Some(nv_container) = non_visual_properties(shape, interner) else {
        return false;
    };
    nav::child(nv_container, interner, PML, "cNvSpPr")
        .and_then(|c_nv_sp_pr| nav::attr_value(c_nv_sp_pr, interner, "txBox"))
        .is_some_and(|value| matches!(value, "1" | "true"))
}

/// Everything a shape's `p:ph` declares: what the placeholder holds, which slot it occupies, how much
/// of the layout it fills, which way its text runs, and the shape's own name.
///
/// This is what a layout offers a slide to fill in. The slot — [`kind`](PlaceholderInfo::kind) plus
/// [`index`](PlaceholderInfo::index) — is what inheritance matches on, so a slide placeholder with the
/// same slot as one on its layout takes that layout shape's properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceholderInfo {
    /// What the placeholder holds (`@type`; `obj` when unstated, per the schema).
    pub kind: PlaceholderType,
    /// The slot index (`@idx`, default `0`) that inheritance matches on.
    pub index: u32,
    /// How much of the layout the placeholder covers (`@sz`, default `full`).
    pub size: PlaceholderSize,
    /// Which way the placeholder's text runs (`@orient`, default `horz`).
    pub orientation: Orientation,
    /// The shape's non-visual name (`p:cNvPr@name`, e.g. `Title 1`), or `None` if unnamed.
    pub name: Option<String>,
}

/// One shape of a surface, as an inventory entry: where it sits, what it is, and the placeholder
/// slot it fills.
///
/// [`Presentation::shapes`](crate::Presentation::shapes) answers with a whole surface's worth of
/// these in one call, so asking *what is on this slide* is one read of the part rather than an index
/// loop over [`shape_count`](crate::Presentation::shape_count) with a fallible accessor inside it.
///
/// [`index`](Self::index) is on the surface's **top-level** index space, the one every shape method
/// takes as an address; a group is one entry, and its members are reached by descending into it
/// (`[2, 1]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeInfo {
    /// The shape's address on the surface's top-level index space.
    pub index: usize,
    /// What kind of shape it is.
    pub kind: ShapeKind,
    /// The placeholder slot the shape fills, or `None` if it is not a placeholder.
    pub placeholder: Option<PlaceholderInfo>,
}

/// The full placeholder metadata of `shape`, or `None` if it is not a placeholder.
pub(crate) fn shape_placeholder_info(
    shape: &RawElement,
    interner: &Interner,
) -> Option<PlaceholderInfo> {
    let nv_container = non_visual_properties(shape, interner)?;
    let nv_pr = nav::child(nv_container, interner, PML, "nvPr")?;
    let ph = nav::child(nv_pr, interner, PML, "ph")?;
    let Placeholder { kind, idx } = shape_placeholder(shape, interner)?;
    let name = nav::child(nv_container, interner, PML, "cNvPr")
        .and_then(|c_nv_pr| nav::attr_value(c_nv_pr, interner, "name"))
        .filter(|name| !name.is_empty())
        .map(str::to_owned);
    Some(PlaceholderInfo {
        kind,
        index: idx,
        size: nav::attr_value(ph, interner, "sz")
            .and_then(PlaceholderSize::from_wire)
            .unwrap_or(PlaceholderSize::Full),
        orientation: nav::attr_value(ph, interner, "orient")
            .and_then(Orientation::from_wire)
            .unwrap_or(Orientation::Horizontal),
        name,
    })
}

/// The first `p:sp` in `sp_tree` whose placeholder matches `target` (see [`Placeholder::matches`]).
pub(crate) fn find_placeholder<'a>(
    sp_tree: &'a RawElement,
    target: Placeholder,
    interner: &'a Interner,
) -> Option<&'a RawElement> {
    shapes(sp_tree, interner)
        .find(|shape| shape_placeholder(shape, interner).is_some_and(|ph| ph.matches(target)))
}

/// The index of `sp_pr`'s existing fill child (`EG_FillProperties`), if any.
pub(crate) fn fill_child_index(sp_pr: &RawElement, interner: &Interner) -> Option<usize> {
    sp_pr
        .children
        .iter()
        .position(|node| dml_element_local(node, interner).is_some_and(Fill::is_fill_local))
}

/// Where a new fill child should be inserted in `sp_pr`: before the first line/effect/3-D/extension
/// child (so it lands after any geometry), or at the end when none is present.
pub(crate) fn fill_insert_index(sp_pr: &RawElement, interner: &Interner) -> usize {
    sp_pr
        .children
        .iter()
        .position(|node| {
            dml_element_local(node, interner)
                .is_some_and(|local| AFTER_FILL_LOCALS.contains(&local))
        })
        .unwrap_or(sp_pr.children.len())
}

/// A shape's explicit outline element (`p:spPr > a:ln`), if present. Returns `None` when the shape has
/// no `p:spPr` or no `a:ln` (its outline is then inherited).
pub(crate) fn shape_line<'a>(shape: &'a RawElement, interner: &Interner) -> Option<&'a RawElement> {
    let sp_pr = nav::child(shape, interner, PML, "spPr")?;
    nav::child(sp_pr, interner, DML_MAIN, "ln")
}

/// The index of `sp_pr`'s existing outline child (`a:ln`), if any.
pub(crate) fn line_child_index(sp_pr: &RawElement, interner: &Interner) -> Option<usize> {
    sp_pr
        .children
        .iter()
        .position(|node| dml_element_local(node, interner) == Some("ln"))
}

/// Where a new outline child (`a:ln`) should be inserted in `sp_pr`: before the first
/// effect/3-D/extension child (so it lands after any geometry and fill), or at the end when none is
/// present.
pub(crate) fn line_insert_index(sp_pr: &RawElement, interner: &Interner) -> usize {
    sp_pr
        .children
        .iter()
        .position(|node| {
            dml_element_local(node, interner)
                .is_some_and(|local| AFTER_LINE_LOCALS.contains(&local))
        })
        .unwrap_or(sp_pr.children.len())
}

/// A shape's explicit effect list (`p:spPr > a:effectLst`), if present. Returns `None` when the shape
/// has no `p:spPr` or no `a:effectLst` (its effects are then inherited). The rarer `a:effectDag`
/// alternative is not modeled and reads as `None`.
pub(crate) fn shape_effects<'a>(
    shape: &'a RawElement,
    interner: &Interner,
) -> Option<&'a RawElement> {
    let sp_pr = nav::child(shape, interner, PML, "spPr")?;
    nav::child(sp_pr, interner, DML_MAIN, "effectLst")
}

/// The index of `sp_pr`'s existing effect-container child, if any. Matches both `a:effectLst` and its
/// mutually-exclusive `a:effectDag` alternative (`EG_EffectProperties` permits at most one), so setting
/// effects replaces whichever is present rather than emitting a second effect container.
pub(crate) fn effect_child_index(sp_pr: &RawElement, interner: &Interner) -> Option<usize> {
    sp_pr.children.iter().position(|node| {
        matches!(
            dml_element_local(node, interner),
            Some("effectLst") | Some("effectDag")
        )
    })
}

/// Where a new effect list (`a:effectLst`) should be inserted in `sp_pr`: before the first
/// 3-D/extension child (so it lands after any geometry, fill, and outline), or at the end when none is
/// present.
pub(crate) fn effect_insert_index(sp_pr: &RawElement, interner: &Interner) -> usize {
    sp_pr
        .children
        .iter()
        .position(|node| {
            dml_element_local(node, interner)
                .is_some_and(|local| AFTER_EFFECT_LOCALS.contains(&local))
        })
        .unwrap_or(sp_pr.children.len())
}

/// The largest `p:cNvPr@id` anywhere under `sp_tree` (0 if none). Non-visual ids are unique per
/// slide, so the next free id is one past this maximum.
pub(crate) fn max_cnvpr_id(sp_tree: &RawElement, interner: &Interner) -> u32 {
    fn walk(element: &RawElement, interner: &Interner, max: &mut u32) {
        if nav::name_is(&element.name, interner, PML, "cNvPr") {
            if let Some(id) = element
                .attributes
                .iter()
                .find(|attr| {
                    attr.name.prefix.is_none() && interner.resolve(attr.name.local) == "id"
                })
                .and_then(|attr| std::str::from_utf8(&attr.value).ok())
                .and_then(|value| value.parse::<u32>().ok())
            {
                *max = (*max).max(id);
            }
        }
        for child in &element.children {
            if let RawNode::Element(child) = child {
                walk(child, interner, max);
            }
        }
    }
    let mut max = 0;
    walk(sp_tree, interner, &mut max);
    max
}

// ---------------------------------------------------------------------------------------------
// The element edits themselves
// ---------------------------------------------------------------------------------------------
//
// Each of these is the *whole* of one edit, expressed against an already-resolved shape and the
// part's interner — nothing above them knows how the element is built or where it is inserted.
// `Presentation`'s flat setters resolve an address and call one; the shape cursor resolves once and
// calls several. There is exactly one implementation of each edit, and this is it.

/// Writes `fill` into `shape`'s `p:spPr`, replacing an existing fill child in place or inserting a
/// new one after any geometry and before `a:ln`.
///
/// `rel_declaration` is the `xmlns:r` binding a picture fill's `r:embed` needs when the part root
/// does not already bind the relationships namespace to that prefix; it is computed from the part
/// root (which this function cannot see) and simply stamped on the built element.
pub(crate) fn set_fill(
    shape: &mut RawElement,
    interner: &mut Interner,
    fill: &FillSpec,
    rel_declaration: Option<RawAttribute>,
) -> Result<(), PptxError> {
    let sp_pr =
        nav::child_mut(shape, interner, PML, "spPr").ok_or(PptxError::ShapeHasNoProperties)?;
    let mut element = fill.to_fill(interner).to_xml(interner);
    if let Some(declaration) = rel_declaration {
        element.attributes.push(declaration);
    }
    let node = RawNode::Element(element);
    match fill_child_index(sp_pr, interner) {
        Some(index) => sp_pr.children[index] = node,
        None => {
            let at = fill_insert_index(sp_pr, interner);
            sp_pr.children.insert(at, node);
            sp_pr.empty = false;
        }
    }
    Ok(())
}

/// Writes `line` into `shape`'s `p:spPr` as its `a:ln`, replacing an existing one in place or
/// inserting a new one after any geometry and fill, before effects.
pub(crate) fn set_line(
    shape: &mut RawElement,
    interner: &mut Interner,
    line: &LineSpec,
) -> Result<(), PptxError> {
    let sp_pr =
        nav::child_mut(shape, interner, PML, "spPr").ok_or(PptxError::ShapeHasNoProperties)?;
    let node = RawNode::Element(line.to_line(interner).to_xml(interner));
    match line_child_index(sp_pr, interner) {
        Some(index) => sp_pr.children[index] = node,
        None => {
            let at = line_insert_index(sp_pr, interner);
            sp_pr.children.insert(at, node);
            sp_pr.empty = false;
        }
    }
    Ok(())
}

/// Writes `effects` into `shape`'s `p:spPr` as its `a:effectLst`, replacing whichever effect
/// container is present (an `a:effectDag` is overwritten) or inserting a new one after any geometry,
/// fill and outline, before the 3-D and extension children.
pub(crate) fn set_effects(
    shape: &mut RawElement,
    interner: &mut Interner,
    effects: &EffectListSpec,
) -> Result<(), PptxError> {
    let sp_pr =
        nav::child_mut(shape, interner, PML, "spPr").ok_or(PptxError::ShapeHasNoProperties)?;
    let node = RawNode::Element(effects.to_effect_list(interner).to_xml(interner));
    match effect_child_index(sp_pr, interner) {
        Some(index) => sp_pr.children[index] = node,
        None => {
            let at = effect_insert_index(sp_pr, interner);
            sp_pr.children.insert(at, node);
            sp_pr.empty = false;
        }
    }
    Ok(())
}

/// A shape's explicit 3-D scene (`p:spPr > a:scene3d`), if present. Returns `None` when the shape has
/// no `p:spPr` or no `a:scene3d` (3-D has no inheritance chain — an absent scene is simply flat).
pub(crate) fn shape_scene_3d<'a>(
    shape: &'a RawElement,
    interner: &Interner,
) -> Option<&'a RawElement> {
    let sp_pr = nav::child(shape, interner, PML, "spPr")?;
    nav::child(sp_pr, interner, DML_MAIN, "scene3d")
}

/// A shape's explicit 3-D properties (`p:spPr > a:sp3d`), if present. Returns `None` when the shape
/// has no `p:spPr` or no `a:sp3d`.
pub(crate) fn shape_sp3d<'a>(shape: &'a RawElement, interner: &Interner) -> Option<&'a RawElement> {
    let sp_pr = nav::child(shape, interner, PML, "spPr")?;
    nav::child(sp_pr, interner, DML_MAIN, "sp3d")
}

/// The index of `sp_pr`'s existing 3-D scene child (`a:scene3d`), if any. Unique in
/// `CT_ShapeProperties`, so setting a scene replaces it in place.
pub(crate) fn scene3d_child_index(sp_pr: &RawElement, interner: &Interner) -> Option<usize> {
    sp_pr
        .children
        .iter()
        .position(|node| dml_element_local(node, interner) == Some("scene3d"))
}

/// Where a new 3-D scene (`a:scene3d`) should be inserted in `sp_pr`: before the first
/// `a:sp3d`/extension child (so it lands after any geometry, fill, outline, and effects), or at the
/// end when none is present.
pub(crate) fn scene3d_insert_index(sp_pr: &RawElement, interner: &Interner) -> usize {
    sp_pr
        .children
        .iter()
        .position(|node| {
            dml_element_local(node, interner)
                .is_some_and(|local| AFTER_SCENE3D_LOCALS.contains(&local))
        })
        .unwrap_or(sp_pr.children.len())
}

/// The index of `sp_pr`'s existing 3-D properties child (`a:sp3d`), if any. Unique in
/// `CT_ShapeProperties`, so setting them replaces the element in place.
pub(crate) fn sp3d_child_index(sp_pr: &RawElement, interner: &Interner) -> Option<usize> {
    sp_pr
        .children
        .iter()
        .position(|node| dml_element_local(node, interner) == Some("sp3d"))
}

/// Where a new 3-D properties element (`a:sp3d`) should be inserted in `sp_pr`: before any
/// `a:extLst` (so it lands after everything else), or at the end when none is present.
pub(crate) fn sp3d_insert_index(sp_pr: &RawElement, interner: &Interner) -> usize {
    sp_pr
        .children
        .iter()
        .position(|node| {
            dml_element_local(node, interner)
                .is_some_and(|local| AFTER_SP3D_LOCALS.contains(&local))
        })
        .unwrap_or(sp_pr.children.len())
}

/// Writes `scene` into `shape`'s `p:spPr` as its `a:scene3d`, replacing an existing one in place or
/// inserting a new one after any geometry, fill, outline, and effects, before `a:sp3d`.
pub(crate) fn set_scene_3d(
    shape: &mut RawElement,
    interner: &mut Interner,
    scene: &Scene3DSpec,
) -> Result<(), PptxError> {
    let sp_pr =
        nav::child_mut(shape, interner, PML, "spPr").ok_or(PptxError::ShapeHasNoProperties)?;
    let node = RawNode::Element(scene.to_scene_3d(interner).to_xml(interner));
    match scene3d_child_index(sp_pr, interner) {
        Some(index) => sp_pr.children[index] = node,
        None => {
            let at = scene3d_insert_index(sp_pr, interner);
            sp_pr.children.insert(at, node);
            sp_pr.empty = false;
        }
    }
    Ok(())
}

/// Writes `sp3d` into `shape`'s `p:spPr` as its `a:sp3d`, replacing an existing one in place or
/// inserting a new one after every other visual property, before any `a:extLst`.
pub(crate) fn set_sp3d(
    shape: &mut RawElement,
    interner: &mut Interner,
    sp3d: &Shape3DSpec,
) -> Result<(), PptxError> {
    let sp_pr =
        nav::child_mut(shape, interner, PML, "spPr").ok_or(PptxError::ShapeHasNoProperties)?;
    let node = RawNode::Element(sp3d.to_shape_3d(interner).to_xml(interner));
    match sp3d_child_index(sp_pr, interner) {
        Some(index) => sp_pr.children[index] = node,
        None => {
            let at = sp3d_insert_index(sp_pr, interner);
            sp_pr.children.insert(at, node);
            sp_pr.empty = false;
        }
    }
    Ok(())
}

/// Removes `shape`'s `a:scene3d` if present. A shape with no `p:spPr` or no scene has nothing to
/// remove, so this is a no-op that succeeds — 3-D has no inheritance to override, so clearing simply
/// drops the element rather than writing an (schema-invalid) empty one.
pub(crate) fn remove_scene_3d(shape: &mut RawElement, interner: &Interner) {
    if let Some(sp_pr) = nav::child_mut(shape, interner, PML, "spPr") {
        if let Some(index) = scene3d_child_index(sp_pr, interner) {
            sp_pr.children.remove(index);
        }
    }
}

/// Removes `shape`'s `a:sp3d` if present. As [`remove_scene_3d`], a no-op that succeeds when there is
/// nothing to remove.
pub(crate) fn remove_sp3d(shape: &mut RawElement, interner: &Interner) {
    if let Some(sp_pr) = nav::child_mut(shape, interner, PML, "spPr") {
        if let Some(index) = sp3d_child_index(sp_pr, interner) {
            sp_pr.children.remove(index);
        }
    }
}

/// The index of `sp_pr`'s geometry child — the mutually-exclusive `a:prstGeom` or `a:custGeom` — if
/// any.
fn geometry_child_index(sp_pr: &RawElement, interner: &Interner) -> Option<usize> {
    sp_pr.children.iter().position(|node| {
        matches!(
            dml_element_local(node, interner),
            Some("prstGeom" | "custGeom")
        )
    })
}

/// Where a new geometry child should be inserted in `sp_pr`: before the first fill / line / effect /
/// 3-D / extension child (so it lands after any `a:xfrm`, which precedes it), or at the end when none
/// is present. Geometry is the second `CT_ShapeProperties` child, right after the transform.
fn geometry_insert_index(sp_pr: &RawElement, interner: &Interner) -> usize {
    sp_pr
        .children
        .iter()
        .position(|node| {
            dml_element_local(node, interner).is_some_and(|local| {
                Fill::is_fill_local(local) || AFTER_FILL_LOCALS.contains(&local)
            })
        })
        .unwrap_or(sp_pr.children.len())
}

/// Writes `geometry` into `shape`'s `p:spPr`, unifying preset and custom geometry: it replaces
/// whichever geometry element is present (`a:prstGeom` or `a:custGeom`) in place, inserts a new one at
/// the geometry slot (after any `a:xfrm`, before fill), or — for [`Geometry::Inherited`] — removes the
/// shape's own geometry so an inherited one takes over. The two kinds are mutually exclusive, so
/// setting one drops the other.
pub(crate) fn set_geometry(
    shape: &mut RawElement,
    interner: &mut Interner,
    geometry: &Geometry,
) -> Result<(), PptxError> {
    let new_element = match geometry {
        Geometry::Preset(shape_geometry) => {
            // `set_shape` overwrites `prst` and the adjustments, so the placeholder preset is only a
            // seed; the built element carries exactly what `shape_geometry` names.
            let mut geom = PresetGeometry::new(interner, PresetShapeType::Rectangle, None);
            geom.set_shape(interner, *shape_geometry);
            Some(geom.to_xml(interner))
        }
        Geometry::Custom(spec) => Some(spec.to_custom_geometry(interner).to_xml(interner)),
        Geometry::Inherited => None,
    };

    let sp_pr =
        nav::child_mut(shape, interner, PML, "spPr").ok_or(PptxError::ShapeHasNoProperties)?;
    match (geometry_child_index(sp_pr, interner), new_element) {
        (Some(index), Some(element)) => sp_pr.children[index] = RawNode::Element(element),
        (Some(index), None) => {
            sp_pr.children.remove(index);
        }
        (None, Some(element)) => {
            let at = geometry_insert_index(sp_pr, interner);
            sp_pr.children.insert(at, RawNode::Element(element));
            sp_pr.empty = false;
        }
        (None, None) => {}
    }
    Ok(())
}

/// Points a picture's `p:blipFill > a:blip` at relationship `rel_id`, written with the literal
/// `rel_prefix` bound to the relationships namespace. Any `@r:link` is dropped — the picture now
/// embeds its image — and the rest of the `p:blipFill` (source rect, tile/stretch) is preserved.
pub(crate) fn set_blip_embed(
    picture: &mut RawElement,
    interner: &mut Interner,
    rel_prefix: Symbol,
    rel_id: &str,
) -> Result<(), PptxError> {
    let blip_fill =
        nav::child_mut(picture, interner, PML, "blipFill").ok_or(PptxError::PictureHasNoImage)?;
    let blip = nav::child_mut(blip_fill, interner, DML_MAIN, "blip")
        .ok_or(PptxError::PictureHasNoImage)?;

    // Attribute namespaces are unresolved, so the embed/link attributes are matched by local name.
    blip.attributes
        .retain(|attr| interner.resolve(attr.name.local) != "link");
    let embed = build::attr_prefixed(interner, rel_prefix, "embed", rel_id);
    match blip
        .attributes
        .iter()
        .position(|attr| interner.resolve(attr.name.local) == "embed")
    {
        Some(index) => blip.attributes[index] = embed,
        None => blip.attributes.push(embed),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_xml::fidelity;

    const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
    const P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";

    fn sp_pr(inner: &str) -> (RawElement, mjx_ooxml_core::Interner) {
        let fragment = format!(r#"<p:spPr xmlns:p="{P}" xmlns:a="{A}">{inner}</p:spPr>"#);
        let doc = fidelity::parse(fragment.as_bytes()).expect("parse");
        (doc.root, doc.interner)
    }

    fn element(fragment: String) -> (RawElement, mjx_ooxml_core::Interner) {
        let doc = fidelity::parse(fragment.as_bytes()).expect("parse");
        (doc.root, doc.interner)
    }

    #[test]
    fn finds_existing_fill_of_any_kind() {
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/><a:gradFill/><a:ln/>"#);
        assert_eq!(fill_child_index(&el, &interner), Some(2));
    }

    #[test]
    fn no_fill_child_when_absent() {
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/>"#);
        assert_eq!(fill_child_index(&el, &interner), None);
    }

    #[test]
    fn insert_index_lands_after_geometry_before_line() {
        // With an a:ln present, the fill inserts right before it (after geometry).
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/><a:ln/>"#);
        assert_eq!(fill_insert_index(&el, &interner), 2);
    }

    #[test]
    fn insert_index_appends_when_no_trailing_children() {
        // No line/effect/3-D/ext child: the fill appends after geometry.
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/>"#);
        assert_eq!(fill_insert_index(&el, &interner), 2);
    }

    #[test]
    fn finds_existing_line() {
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/><a:solidFill/><a:ln/>"#);
        assert_eq!(line_child_index(&el, &interner), Some(3));
    }

    #[test]
    fn no_line_child_when_absent() {
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/><a:solidFill/>"#);
        assert_eq!(line_child_index(&el, &interner), None);
    }

    #[test]
    fn line_insert_index_lands_after_fill_before_effects() {
        // The outline inserts after geometry+fill and before the effect list.
        let (el, interner) =
            sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/><a:solidFill/><a:effectLst/>"#);
        assert_eq!(line_insert_index(&el, &interner), 3);
    }

    #[test]
    fn line_insert_index_appends_when_no_trailing_children() {
        // No effect/3-D/ext child: the outline appends after geometry and fill.
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/><a:solidFill/>"#);
        assert_eq!(line_insert_index(&el, &interner), 3);
    }

    #[test]
    fn finds_existing_effect_list_or_dag() {
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:solidFill/><a:ln/><a:effectLst/>"#);
        assert_eq!(effect_child_index(&el, &interner), Some(3));
        // The mutually-exclusive a:effectDag alternative is matched too, so it is replaced on set.
        let (dag, interner) = sp_pr(r#"<a:xfrm/><a:ln/><a:effectDag/>"#);
        assert_eq!(effect_child_index(&dag, &interner), Some(2));
    }

    #[test]
    fn no_effect_child_when_absent() {
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/><a:solidFill/><a:ln/>"#);
        assert_eq!(effect_child_index(&el, &interner), None);
    }

    #[test]
    fn effect_insert_index_lands_after_line_before_3d() {
        // The effect list inserts after geometry+fill+outline and before the 3-D children.
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:solidFill/><a:ln/><a:sp3d/>"#);
        assert_eq!(effect_insert_index(&el, &interner), 3);
    }

    #[test]
    fn effect_insert_index_appends_when_no_trailing_children() {
        // No 3-D/ext child: the effect list appends after geometry, fill, and outline.
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:solidFill/><a:ln/>"#);
        assert_eq!(effect_insert_index(&el, &interner), 3);
    }

    #[test]
    fn parse_color_map_reads_twelve_slots() {
        let (map_el, interner) = element(format!(
            concat!(
                r#"<p:clrMap xmlns:p="{P}" bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" "#,
                r#"accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" "#,
                r#"accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>"#
            ),
            P = P
        ));
        let map = parse_color_map(&map_el, &interner).expect("color map");
        assert_eq!(
            map.resolve(mjx_dml::SchemeColor::Background1),
            Some(ColorSchemeSlot::Light1)
        );
        assert_eq!(
            map.resolve(mjx_dml::SchemeColor::Text1),
            Some(ColorSchemeSlot::Dark1)
        );
    }

    #[test]
    fn parse_color_map_rejects_attribute_less_override() {
        // A schema-loose, attribute-less overrideClrMapping yields no map (caller falls back).
        let (map_el, interner) = element(format!(r#"<a:overrideClrMapping xmlns:a="{A}"/>"#));
        assert!(parse_color_map(&map_el, &interner).is_none());
    }

    fn sp(inner: &str) -> (RawElement, mjx_ooxml_core::Interner) {
        let fragment = format!(r#"<p:sp xmlns:p="{P}" xmlns:a="{A}">{inner}</p:sp>"#);
        let doc = fidelity::parse(fragment.as_bytes()).expect("parse");
        (doc.root, doc.interner)
    }

    /// A shape tree holding one child of each interesting kind, in a deliberate order.
    fn mixed_sp_tree() -> (RawElement, mjx_ooxml_core::Interner) {
        element(format!(
            concat!(
                r#"<p:spTree xmlns:p="{P}" xmlns:a="{A}">"#,
                r#"<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>"#,
                r#"<p:grpSpPr/>"#,
                r#"<p:sp><p:spPr/></p:sp>"#,
                r#"<p:pic><p:spPr/></p:pic>"#,
                r#"<p:grpSp><p:sp><p:spPr/></p:sp><p:pic/></p:grpSp>"#,
                r#"<p:cxnSp><p:spPr/></p:cxnSp>"#,
                r#"<p:graphicFrame/>"#,
                r#"<p:extLst/>"#,
                r#"</p:spTree>"#
            ),
            P = P,
            A = A
        ))
    }

    #[test]
    fn shapes_enumerates_every_kind_in_document_order() {
        let (tree, interner) = mixed_sp_tree();
        let kinds: Vec<ShapeKind> = shapes(&tree, &interner)
            .map(|shape| shape_kind(shape, &interner).expect("a shape kind"))
            .collect();
        assert_eq!(
            kinds,
            vec![
                ShapeKind::Shape,
                ShapeKind::Picture,
                ShapeKind::GroupShape,
                ShapeKind::ConnectionShape,
                ShapeKind::GraphicFrame,
            ],
            "the group's own members must not be enumerated, and non-shape children must be skipped"
        );
    }

    #[test]
    fn nth_shape_mut_addresses_the_same_index_space() {
        let (mut tree, interner) = mixed_sp_tree();
        // Index 1 is the picture, index 3 the connector — a `p:sp`-only lookup would return neither.
        let picture = nth_shape_mut(&mut tree, &interner, 1).expect("shape 1");
        assert_eq!(shape_kind(picture, &interner), Some(ShapeKind::Picture));
        let connector = nth_shape_mut(&mut tree, &interner, 3).expect("shape 3");
        assert_eq!(
            shape_kind(connector, &interner),
            Some(ShapeKind::ConnectionShape)
        );
        assert!(nth_shape_mut(&mut tree, &interner, 5).is_none());
    }

    #[test]
    fn resolve_shape_descends_into_a_group() {
        // In `mixed_sp_tree` the group is top-level index 2, holding a `p:sp` (member 0) and a
        // `p:pic` (member 1).
        let (tree, interner) = mixed_sp_tree();
        let member0 =
            resolve_shape(&tree, &interner, &ShapePath::from([2, 0])).expect("group member 0");
        assert_eq!(shape_kind(member0, &interner), Some(ShapeKind::Shape));
        let member1 =
            resolve_shape(&tree, &interner, &ShapePath::from([2, 1])).expect("group member 1");
        assert_eq!(shape_kind(member1, &interner), Some(ShapeKind::Picture));
        // A bare top-level index still resolves the group itself.
        let group = resolve_shape(&tree, &interner, &ShapePath::from(2)).expect("the group");
        assert_eq!(shape_kind(group, &interner), Some(ShapeKind::GroupShape));
    }

    #[test]
    fn resolve_shape_reports_the_failing_containers_count() {
        let (tree, interner) = mixed_sp_tree();
        // Past the end of the top-level space (five shapes: sp, pic, grpSp, cxnSp, graphicFrame).
        assert_eq!(resolve_shape(&tree, &interner, &ShapePath::from(5)), Err(5));
        // Past the end of the group's two members.
        assert_eq!(
            resolve_shape(&tree, &interner, &ShapePath::from([2, 9])),
            Err(2)
        );
        // Descending into a non-group (index 0 is a `p:sp`) finds a zero-member container.
        assert_eq!(
            resolve_shape(&tree, &interner, &ShapePath::from([0, 0])),
            Err(0)
        );
    }

    #[test]
    fn resolve_shape_mut_reaches_the_same_member() {
        let (mut tree, interner) = mixed_sp_tree();
        let member1 =
            resolve_shape_mut(&mut tree, &interner, &ShapePath::from([2, 1])).expect("member 1");
        assert_eq!(shape_kind(member1, &interner), Some(ShapeKind::Picture));
        assert_eq!(
            resolve_shape_mut(&mut tree, &interner, &ShapePath::from([0, 0])),
            Err(0)
        );
    }

    #[test]
    fn resolve_shape_position_names_the_parent_container_and_child_slot() {
        let (mut tree, interner) = mixed_sp_tree();
        // The member's parent is the group, not the shape tree — so removal drops it from the group.
        let (parent, position) =
            resolve_shape_position(&mut tree, &interner, &ShapePath::from([2, 1]))
                .expect("member 1 position");
        assert_eq!(shape_kind(parent, &interner), Some(ShapeKind::GroupShape));
        let removed = match &parent.children[position] {
            RawNode::Element(element) => shape_kind(element, &interner),
            _ => None,
        };
        assert_eq!(removed, Some(ShapeKind::Picture));

        // A top-level shape's parent is the shape tree itself.
        let (parent, _) = resolve_shape_position(&mut tree, &interner, &ShapePath::from(0))
            .expect("top-level position");
        assert!(nav::name_is(&parent.name, &interner, PML, "spTree"));
    }

    #[test]
    fn shape_kind_rejects_a_same_named_element_in_another_namespace() {
        let (foreign, interner) = element(format!(r#"<a:pic xmlns:a="{A}"/>"#));
        assert_eq!(shape_kind(&foreign, &interner), None);
    }

    #[test]
    fn placeholder_is_read_from_a_pictures_non_visual_container() {
        // A picture placeholder keeps its p:ph under p:nvPicPr, not p:nvSpPr.
        let (pic, interner) = element(format!(
            concat!(
                r#"<p:pic xmlns:p="{P}" xmlns:a="{A}"><p:nvPicPr><p:cNvPr id="5" name="P"/>"#,
                r#"<p:cNvPicPr/><p:nvPr><p:ph type="pic" idx="2"/></p:nvPr></p:nvPicPr></p:pic>"#
            ),
            P = P,
            A = A
        ));
        let ph = shape_placeholder(&pic, &interner).expect("picture placeholder");
        assert_eq!(ph.kind, PlaceholderType::Picture);
        assert!(!ph.is_title_family());
        assert_eq!(ph.idx, 2);
    }

    #[test]
    fn shape_placeholder_reads_type_and_idx_with_defaults() {
        let (shape, interner) = sp(
            r#"<p:nvSpPr><p:cNvPr id="2" name="T"/><p:cNvSpPr/><p:nvPr><p:ph type="ctrTitle"/></p:nvPr></p:nvSpPr><p:spPr/>"#,
        );
        let ph = shape_placeholder(&shape, &interner).expect("placeholder");
        assert_eq!(ph.kind, PlaceholderType::CenteredTitle);
        assert!(ph.is_title_family());
        assert_eq!(ph.idx, 0);

        // A body placeholder with an explicit idx.
        let (shape, interner) = sp(
            r#"<p:nvSpPr><p:cNvPr id="3" name="B"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr>"#,
        );
        let ph = shape_placeholder(&shape, &interner).expect("placeholder");
        assert_eq!(ph.kind, PlaceholderType::Body);
        assert!(!ph.is_title_family());
        assert_eq!(ph.idx, 1);

        // A p:ph with no @type defaults to the schema's `obj`.
        let (shape, interner) = sp(
            r#"<p:nvSpPr><p:cNvPr id="4" name="C"/><p:cNvSpPr/><p:nvPr><p:ph idx="2"/></p:nvPr></p:nvSpPr>"#,
        );
        let ph = shape_placeholder(&shape, &interner).expect("placeholder");
        assert_eq!(ph.kind, PlaceholderType::Object);
        assert_eq!(ph.idx, 2);

        // No p:ph -> not a placeholder.
        let (shape, interner) =
            sp(r#"<p:nvSpPr><p:cNvPr id="4" name="X"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>"#);
        assert!(shape_placeholder(&shape, &interner).is_none());
    }

    #[test]
    fn placeholder_matching_rules() {
        let title = Placeholder {
            kind: PlaceholderType::Title,
            idx: 0,
        };
        let ctr_title = Placeholder {
            kind: PlaceholderType::CenteredTitle,
            idx: 5,
        };
        let body0 = Placeholder {
            kind: PlaceholderType::Body,
            idx: 0,
        };
        let body1 = Placeholder {
            kind: PlaceholderType::Body,
            idx: 1,
        };
        // Title-family match regardless of idx.
        assert!(title.matches(ctr_title));
        // Body matches by idx.
        assert!(body0.matches(Placeholder {
            kind: PlaceholderType::Object,
            idx: 0
        }));
        assert!(!body0.matches(body1));
        // Title never matches body.
        assert!(!title.matches(body0));
    }

    #[test]
    fn find_placeholder_picks_the_matching_shape() {
        let (sp_tree, interner) = element(format!(
            concat!(
                r#"<p:spTree xmlns:p="{P}" xmlns:a="{A}">"#,
                r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="B"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr></p:sp>"#,
                r#"<p:sp><p:nvSpPr><p:cNvPr id="3" name="T"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr></p:sp>"#,
                r#"</p:spTree>"#
            ),
            P = P,
            A = A
        ));
        let title_target = Placeholder {
            kind: PlaceholderType::Title,
            idx: 0,
        };
        let found = find_placeholder(&sp_tree, title_target, &interner).expect("title match");
        assert!(shape_placeholder(found, &interner)
            .expect("placeholder")
            .is_title_family());

        // No matching body idx.
        assert!(find_placeholder(
            &sp_tree,
            Placeholder {
                kind: PlaceholderType::Body,
                idx: 9
            },
            &interner
        )
        .is_none());
    }

    // -----------------------------------------------------------------------------------------
    // The transform locator — the three places a transform lives, and the kind that has none
    // -----------------------------------------------------------------------------------------

    /// One shape of each kind, each carrying a transform in the place its schema puts it. The `x`
    /// coordinate identifies which transform was found, so a locator that reaches into the wrong
    /// container fails loudly instead of silently finding *a* transform.
    fn shape_of_each_kind() -> Vec<(ShapeKind, String)> {
        vec![
            (
                ShapeKind::Shape,
                r#"<p:sp><p:spPr><a:xfrm><a:off x="1" y="0"/></a:xfrm></p:spPr></p:sp>"#.to_owned(),
            ),
            (
                ShapeKind::Picture,
                r#"<p:pic><p:spPr><a:xfrm><a:off x="2" y="0"/></a:xfrm></p:spPr></p:pic>"#
                    .to_owned(),
            ),
            (
                ShapeKind::ConnectionShape,
                r#"<p:cxnSp><p:spPr><a:xfrm><a:off x="3" y="0"/></a:xfrm></p:spPr></p:cxnSp>"#
                    .to_owned(),
            ),
            (
                // A group's is in `p:grpSpPr`, and carries the child coordinate space.
                ShapeKind::GroupShape,
                concat!(
                    r#"<p:grpSp><p:grpSpPr><a:xfrm><a:off x="4" y="0"/>"#,
                    r#"<a:chOff x="40" y="0"/></a:xfrm></p:grpSpPr>"#,
                    r#"<p:sp><p:spPr><a:xfrm><a:off x="999" y="0"/></a:xfrm></p:spPr></p:sp>"#,
                    r#"</p:grpSp>"#
                )
                .to_owned(),
            ),
            (
                // A graphic frame's is a `p:xfrm`, a direct child.
                ShapeKind::GraphicFrame,
                concat!(
                    r#"<p:graphicFrame><p:nvGraphicFramePr/>"#,
                    r#"<p:xfrm><a:off x="5" y="0"/></p:xfrm>"#,
                    r#"<a:graphic/></p:graphicFrame>"#
                )
                .to_owned(),
            ),
            (ShapeKind::ContentPart, r#"<p:contentPart/>"#.to_owned()),
        ]
    }

    /// Wraps a shape fragment so its namespaces resolve.
    fn shape(fragment: &str) -> (RawElement, mjx_ooxml_core::Interner) {
        let (tree, interner) = element(format!(
            r#"<p:spTree xmlns:p="{P}" xmlns:a="{A}">{fragment}</p:spTree>"#
        ));
        let shape = tree
            .children
            .iter()
            .find_map(|node| match node {
                RawNode::Element(element) => Some(element.clone()),
                _ => None,
            })
            .expect("one shape");
        (shape, interner)
    }

    #[test]
    fn the_locator_finds_each_kinds_transform_where_its_schema_keeps_it() {
        for (kind, fragment) in shape_of_each_kind() {
            let (element, interner) = shape(&fragment);
            let found = shape_transform(&element, &interner);

            if kind == ShapeKind::ContentPart {
                assert!(found.is_none(), "{kind:?} has no transform in its schema");
                continue;
            }
            let found = found.unwrap_or_else(|| panic!("{kind:?}: no transform found"));
            let off = nav::child(found, &interner, DML_MAIN, "off").expect("a:off");
            let x = nav::attr_value(off, &interner, "x").expect("x");
            let expected = match kind {
                ShapeKind::Shape => "1",
                ShapeKind::Picture => "2",
                ShapeKind::ConnectionShape => "3",
                ShapeKind::GroupShape => "4",
                ShapeKind::GraphicFrame => "5",
                ShapeKind::ContentPart => unreachable!(),
            };
            assert_eq!(x, expected, "{kind:?} found the wrong transform");
        }
    }

    #[test]
    fn a_groups_own_transform_is_not_its_first_members() {
        // `p:grpSp` holds both a `p:grpSpPr > a:xfrm` and member shapes with their own — a locator
        // that searched descendants rather than the named container would find the member's.
        let (_, fragment) = shape_of_each_kind()
            .into_iter()
            .find(|(kind, _)| *kind == ShapeKind::GroupShape)
            .expect("the group case");
        let (element, interner) = shape(&fragment);

        let found = shape_transform(&element, &interner).expect("the group's own transform");
        let transform = mjx_dml::Transform2D::read(found, &interner);
        assert_eq!(transform.position.map(|p| p.x.emu()), Some(4));
        assert_eq!(
            transform.child_position.map(|p| p.x.emu()),
            Some(40),
            "the child coordinate space comes with it"
        );
    }

    #[test]
    fn a_shape_declaring_no_transform_reads_as_none() {
        let (element, interner) = shape(r#"<p:sp><p:spPr/></p:sp>"#);
        assert!(shape_transform(&element, &interner).is_none());
    }

    #[test]
    fn the_slot_creates_a_transform_before_everything_it_must_precede() {
        // `a:xfrm` is the first member of `CT_ShapeProperties`'s sequence.
        let (mut element, mut interner) =
            shape(r#"<p:sp><p:spPr><a:prstGeom prst="rect"/><a:ln/></p:spPr></p:sp>"#);
        let slot = shape_transform_slot_mut(&mut element, &mut interner).expect("slot");
        assert_eq!(interner.resolve(slot.name.local), "xfrm");

        let sp_pr = nav::child(&element, &interner, PML, "spPr").expect("spPr");
        let locals: Vec<&str> = sp_pr
            .children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(el) => Some(interner.resolve(el.name.local)),
                _ => None,
            })
            .collect();
        assert_eq!(locals, ["xfrm", "prstGeom", "ln"]);
    }

    #[test]
    fn the_slot_creates_a_graphic_frames_transform_in_presentationml() {
        // A `p:graphicFrame`'s transform is `p:xfrm`, and sits after `p:nvGraphicFramePr`.
        let (mut element, mut interner) =
            shape(r#"<p:graphicFrame><p:nvGraphicFramePr/><a:graphic/></p:graphicFrame>"#);
        let slot = shape_transform_slot_mut(&mut element, &mut interner).expect("slot");
        assert_eq!(interner.resolve(slot.name.local), "xfrm");
        assert!(
            nav::name_is(&slot.name, &interner, PML, "xfrm"),
            "a graphic frame's transform is PresentationML's, not DrawingML's"
        );

        let locals: Vec<&str> = element
            .children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(el) => Some(interner.resolve(el.name.local)),
                _ => None,
            })
            .collect();
        assert_eq!(locals, ["nvGraphicFramePr", "xfrm", "graphic"]);
    }

    #[test]
    fn the_slot_returns_an_existing_transform_rather_than_a_second_one() {
        let (mut element, mut interner) =
            shape(r#"<p:sp><p:spPr><a:xfrm><a:off x="7" y="0"/></a:xfrm></p:spPr></p:sp>"#);
        {
            let slot = shape_transform_slot_mut(&mut element, &mut interner).expect("slot");
            assert!(nav::child(slot, &interner, DML_MAIN, "off").is_some());
        }
        let sp_pr = nav::child(&element, &interner, PML, "spPr").expect("spPr");
        let count = sp_pr
            .children
            .iter()
            .filter(|node| {
                matches!(node, RawNode::Element(el)
                if interner.resolve(el.name.local) == "xfrm")
            })
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn a_content_part_cannot_be_positioned() {
        let (mut element, mut interner) = shape(r#"<p:contentPart/>"#);
        assert!(matches!(
            shape_transform_slot_mut(&mut element, &mut interner),
            Err(PptxError::ShapeCannotBePositioned {
                kind: ShapeKind::ContentPart
            })
        ));
    }

    #[test]
    fn a_shape_missing_its_properties_element_is_a_typed_error() {
        let (mut element, mut interner) = shape(r#"<p:sp/>"#);
        assert!(matches!(
            shape_transform_slot_mut(&mut element, &mut interner),
            Err(PptxError::ShapeHasNoProperties)
        ));
    }

    #[test]
    fn graphic_frame_kind_is_told_from_the_graphic_data_uri() {
        assert_eq!(
            GraphicFrameKind::from_uri(TABLE_GRAPHIC_URI),
            GraphicFrameKind::Table
        );
        assert_eq!(
            GraphicFrameKind::from_uri(CHART_GRAPHIC_URI),
            GraphicFrameKind::Chart
        );
        assert_eq!(
            GraphicFrameKind::from_uri(DIAGRAM_GRAPHIC_URI),
            GraphicFrameKind::Diagram
        );
        assert_eq!(
            GraphicFrameKind::from_uri("urn:something:else"),
            GraphicFrameKind::Other
        );
    }
}
