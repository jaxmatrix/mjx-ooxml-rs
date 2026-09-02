//! The element builders shared by more than one authoring surface: the non-visual property
//! blocks, the shape-property block, and a text body's paragraphs and runs.

use mjx_dml::Transform2D;
use mjx_ooxml_core::{Interner, RawElement, RawNode};
use mjx_ooxml_types::namespaces::{DML_MAIN, PML};

use crate::build;
use crate::geometry::ShapeBounds;

/// `p:nvGraphicFramePr` — the non-visual furniture every graphic frame carries: a `p:cNvPr` with the
/// id and name, a `p:cNvGraphicFramePr` locking the frame against grouping, and an empty `p:nvPr`.
pub(super) fn build_nv_graphic_frame_pr(
    interner: &mut Interner,
    id: u32,
    name: &str,
) -> RawElement {
    let cnvpr_attrs = vec![
        build::attr(interner, "id", &id.to_string()),
        build::attr(interner, "name", name),
    ];
    let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
    let lock_attrs = vec![build::attr(interner, "noGrp", "1")];
    let frame_locks = build::leaf(interner, "a", DML_MAIN, "graphicFrameLocks", lock_attrs);
    let c_nv_frame_pr = build::node(
        interner,
        "p",
        PML,
        "cNvGraphicFramePr",
        Vec::new(),
        vec![RawNode::Element(frame_locks)],
    );
    let nv_pr = build::leaf(interner, "p", PML, "nvPr", Vec::new());
    build::node(
        interner,
        "p",
        PML,
        "nvGraphicFramePr",
        Vec::new(),
        vec![
            RawNode::Element(c_nv_pr),
            RawNode::Element(c_nv_frame_pr),
            RawNode::Element(nv_pr),
        ],
    )
}

/// Builds a plain text-box `p:sp` with non-visual id `id`, laid out at `bounds`, whose text body
/// holds one paragraph per line of `text`.
/// `p:nvSpPr` — non-visual shape properties: `p:cNvPr@id,name`, `p:cNvSpPr` (with `txBox="1"` iff
/// `tx_box`), and an empty `p:nvPr`.
pub(super) fn build_nv_sp_pr(
    interner: &mut Interner,
    id: u32,
    name: &str,
    tx_box: bool,
) -> RawElement {
    let cnvpr_attrs = vec![
        build::attr(interner, "id", &id.to_string()),
        build::attr(interner, "name", name),
    ];
    let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
    let cnvsppr_attrs = if tx_box {
        vec![build::attr(interner, "txBox", "1")]
    } else {
        Vec::new()
    };
    let c_nv_sp_pr = build::leaf(interner, "p", PML, "cNvSpPr", cnvsppr_attrs);
    let nv_pr = build::leaf(interner, "p", PML, "nvPr", Vec::new());
    build::node(
        interner,
        "p",
        PML,
        "nvSpPr",
        Vec::new(),
        vec![
            RawNode::Element(c_nv_pr),
            RawNode::Element(c_nv_sp_pr),
            RawNode::Element(nv_pr),
        ],
    )
}

/// `p:spPr` — visual shape properties: an `a:xfrm` transform at `bounds` plus `a:prstGeom@prst` with
/// an empty `a:avLst` (the preset's default adjustments).
pub(super) fn build_sp_pr(interner: &mut Interner, prst: &str, bounds: ShapeBounds) -> RawElement {
    // One spelling of an `a:xfrm` in the crate: creating a shape and moving one go through the
    // same writer, so a built transform and an edited transform cannot drift apart.
    let mut xfrm = Transform2D::empty_element(interner);
    bounds.to_transform().apply(&mut xfrm, interner);
    let av_lst = build::leaf(interner, "a", DML_MAIN, "avLst", Vec::new());
    let prstgeom_attrs = vec![build::attr(interner, "prst", prst)];
    let prst_geom = build::node(
        interner,
        "a",
        DML_MAIN,
        "prstGeom",
        prstgeom_attrs,
        vec![RawNode::Element(av_lst)],
    );
    build::node(
        interner,
        "p",
        PML,
        "spPr",
        Vec::new(),
        vec![RawNode::Element(xfrm), RawNode::Element(prst_geom)],
    )
}

/// `p:txBody` — the required `a:bodyPr` + `a:lstStyle`, then `paragraphs`.
pub(super) fn build_text_body(interner: &mut Interner, paragraphs: Vec<RawElement>) -> RawElement {
    build_body(interner, "p", PML, paragraphs)
}

/// A `CT_TextBody` under whichever name its container gives it — `p:txBody` in a shape, `a:txBody`
/// in a table cell. The content is identical, which is the whole reason one text model serves both.
fn build_body(
    interner: &mut Interner,
    prefix: &str,
    namespace: mjx_ooxml_types::namespaces::SchemaNamespace,
    paragraphs: Vec<RawElement>,
) -> RawElement {
    let body_pr = build::leaf(interner, "a", DML_MAIN, "bodyPr", Vec::new());
    let lst_style = build::leaf(interner, "a", DML_MAIN, "lstStyle", Vec::new());
    let mut children = vec![RawNode::Element(body_pr), RawNode::Element(lst_style)];
    children.extend(paragraphs.into_iter().map(RawNode::Element));
    build::node(interner, prefix, namespace, "txBody", Vec::new(), children)
}

/// One fresh `a:tc`: an `a:txBody` with one empty paragraph and an empty `a:tcPr` — what both a
/// created table's cells and a cell inserted by a row/column edit are born as. A caller's first act
/// is `set_cell_text`; formatting is added afterwards with `format_cells`, never inherited here.
pub(super) fn build_table_cell(interner: &mut Interner) -> RawElement {
    let paragraph = build_paragraph(interner, "");
    let body = build_body(interner, "a", DML_MAIN, vec![paragraph]);
    let tc_pr = build::leaf(interner, "a", DML_MAIN, "tcPr", Vec::new());
    build::node(
        interner,
        "a",
        DML_MAIN,
        "tc",
        Vec::new(),
        vec![RawNode::Element(body), RawNode::Element(tc_pr)],
    )
}

/// Builds one `a:p` holding exactly one run (`a:r > a:t`) carrying the line's text — **including when
/// the line is empty**, which yields an empty run rather than an empty paragraph.
///
/// That is what makes a newly added shape fillable: [`set_shape_text`](Presentation::set_shape_text)
/// *replaces* the `run_idx`-th run, so a paragraph with no runs could not be filled in at all (it
/// answered [`RunIndexOutOfRange`](PptxError::RunIndexOutOfRange)). An empty run renders exactly like
/// an empty paragraph, so the blank line a caller asked for still looks blank.
pub(super) fn build_paragraph(interner: &mut Interner, line: &str) -> RawElement {
    let run = build_run(interner, line);
    build::node(
        interner,
        "a",
        DML_MAIN,
        "p",
        Vec::new(),
        vec![RawNode::Element(run)],
    )
}

/// One `a:r` text run carrying `text` (which may be empty — an empty run is what makes a shape
/// fillable by [`set_shape_text`](Presentation::set_shape_text), which replaces an existing run).
pub(super) fn build_run(interner: &mut Interner, text: &str) -> RawElement {
    let t = build::text_leaf(interner, "a", DML_MAIN, "t", Vec::new(), text);
    build::node(
        interner,
        "a",
        DML_MAIN,
        "r",
        Vec::new(),
        vec![RawNode::Element(t)],
    )
}
