//! Navigation of a slide's shape tree (`p:sld > p:cSld > p:spTree > p:sp > p:txBody`).

use mjx_ooxml_core::{Interner, RawElement};
use mjx_ooxml_types::namespaces::PML;

use crate::error::PptxError;
use crate::nav;

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

/// The `p:sp` shape children of a shape tree, in order (skips `p:nvGrpSpPr` / `p:grpSpPr`).
pub(crate) fn shapes<'a>(
    sp_tree: &'a RawElement,
    interner: &'a Interner,
) -> impl Iterator<Item = &'a RawElement> {
    nav::children(sp_tree, interner, PML, "sp")
}

/// A shape's `p:txBody`, if it has one.
pub(crate) fn shape_txbody<'a>(
    shape: &'a RawElement,
    interner: &Interner,
) -> Option<&'a RawElement> {
    nav::child(shape, interner, PML, "txBody")
}
