//! Navigation of a slide's shape tree (`p:sld > p:cSld > p:spTree > p:sp > p:txBody`).

use mjx_dml::{ColorMap, ColorSchemeSlot, Fill, StyleMatrixReference};
use mjx_ooxml_core::{FromXml, Interner, RawElement, RawNode};
use mjx_ooxml_types::namespaces::{SchemaNamespace, DML_MAIN, PML};
use mjx_ooxml_types::presentationml::{Orientation, PlaceholderSize, PlaceholderType};

use crate::build;
use crate::error::PptxError;
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
    /// `p:grpSp` — a group of shapes (`CT_GroupShape`). Its children are not themselves enumerated.
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
/// `p:grpSp`: a group is one shape, its members are not separately addressable.
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

    // Insert before everything the transform must precede, rather than appending: in both
    // `CT_ShapeProperties` and `CT_GroupShapeProperties` the transform is the *first* member of the
    // sequence, and in `CT_GraphicalObjectFrame` it follows only `p:nvGraphicFramePr`. Order is
    // validity here, not style.
    let namespace = name.namespace();
    let existing = holder.children.iter().position(|node| match node {
        RawNode::Element(element) => nav::name_is(&element.name, interner, namespace, "xfrm"),
        _ => false,
    });
    let index = match existing {
        Some(index) => index,
        None => {
            let at = transform_insert_index(holder, interner, container.is_none());
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

/// Where a new transform child belongs in `holder`.
///
/// In a `p:spPr` / `p:grpSpPr` the transform precedes every other element, so it goes before the
/// first one. In a `p:graphicFrame` (`is_graphic_frame`) it goes after the required
/// `p:nvGraphicFramePr` and before the `a:graphic`.
fn transform_insert_index(
    holder: &RawElement,
    interner: &Interner,
    is_graphic_frame: bool,
) -> usize {
    let first_element = holder
        .children
        .iter()
        .position(|node| matches!(node, RawNode::Element(_)));
    if !is_graphic_frame {
        return first_element.unwrap_or(holder.children.len());
    }
    holder
        .children
        .iter()
        .position(|node| match node {
            RawNode::Element(element) => {
                !nav::name_is(&element.name, interner, PML, "nvGraphicFramePr")
            }
            _ => false,
        })
        .unwrap_or(holder.children.len())
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
}
