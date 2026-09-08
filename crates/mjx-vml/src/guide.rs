// The last page of the upper shared markup's guide, whose other five live in `mjx-chart` and whose
// index is `crates/mjx-chart/docs/guide/README.md`. It is hosted here rather than there because
// `mjx-chart` and `mjx-vml` are the same layering rank: neither may depend on the other, so a page in
// that directory could not resolve a single intra-doc link into this crate (MJXOFF-221).
//
// One page, so this module *is* the page rather than an index over submodules — the shape
// `mjx_mce::guide` uses for the same reason. It is also where **VML's weaker guarantee** is stated
// for a caller who has a VML part in their hands: no schema gate, no XSD-derived child order, and the
// round trip as the only real check.
#![doc = include_str!("../docs/legacy_vml.md")]

// Imported so the page's intra-doc links resolve, exactly as the other guide sets do.
#[allow(unused_imports)]
use crate::{
    is_vml_content_type, shape_identifier_for_number, AttachedObjectData, AttachedObjectKind,
    DiagramText, Drawing, DrawingContent, DrawingPart, EmbeddedOleObject, Fill, ImageData, Ink,
    OleDrawAspect, OleObjectKind, OleUpdateMode, Shape, ShapeContent, ShapeGroup,
    ShapeGroupContent, ShapeIdMap, ShapeLayout, ShapeLayoutContent, ShapePath, ShapeProtections,
    ShapeTemplate, Stroke, TextBox, CONTENT_TYPE_VML, REL_VML_DRAWING, VML_DEFAULT_EXTENSION,
};
