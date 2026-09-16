//! What a shape's `a:prstGeom` is, read out of the document itself (MJXOFF-300).
//!
//! # Why this is here rather than in a test
//!
//! `mjx_scene_pptx::SlideGeometry` resolves an outline *handle* and never opens a package, because
//! a resolver that read a document would be a second reader beside the one that laid it out. The
//! other half of that join — *which* `a:prstGeom` the handle stands for — can only be answered by
//! whoever holds the file, and this crate is the one place in the workspace that may hold a
//! `mjx-pptx` (3.0) and a painter (5.5) at once.
//!
//! Before RC03 that half lived twice, as a private `fn outline_of` copied into two test files. A
//! journey that is supposed to prove *no test double* cannot also be the definition of the
//! production path it exercises, so the definition moved here and the tests call it.

use mjx_dml::{GuideContext, Size};
use mjx_geometry::{AdjustmentOverride, ShapeOutline};
use mjx_pptx::{Presentation, Surface};

/// The outline of the shape at `shape` on `surface`, sized to `extents`.
///
/// Two readers, because they answer two different questions: `shape_preset` says *which* preset —
/// the `ST_ShapeType` token the path tables are indexed by — and `shape_adjustments` says what its
/// guides have been moved to.
///
/// Only the **overridden** adjustments are carried across, for the reason
/// `ShapeOutline::from_preset_geometry` gives: the defaults are already in the generated table, and
/// copying them into every registry entry would put two sources of one number in the process.
///
/// A shape with a custom path, or with none of its own, answers `None` and reaches the provider's
/// policy — counted as a stand-in, never mistaken for the document's own geometry.
#[must_use]
pub fn shape_outline(
    presentation: &mut Presentation,
    surface: Surface,
    shape: &[u32],
    extents: Size,
) -> Option<ShapeOutline> {
    let path: Vec<usize> = shape.iter().map(|&index| index as usize).collect();
    let preset = presentation.shape_preset(surface, path.clone()).ok()??;
    let adjustments = presentation
        .shape_adjustments(surface, path, GuideContext::from_size(extents))
        .unwrap_or_default();
    Some(ShapeOutline {
        preset,
        extents,
        adjustments: adjustments
            .into_iter()
            .filter(|adjustment| adjustment.is_overridden)
            .map(|adjustment| AdjustmentOverride::new(adjustment.spec.wire_name, adjustment.value))
            .collect(),
    })
}
