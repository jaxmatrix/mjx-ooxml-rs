//! [`SheetGeometry`] — the provider a worksheet needs, which is the one that answers nothing.
//!
//! # Why a provider at all, when a worksheet has no shapes
//!
//! `mjx-paint`'s `Resources` takes a [`GeometryProvider`] because a display list can carry a
//! [`Geometry::Unresolved`](mjx_scene::Geometry::Unresolved), and every painter resolves one the
//! same way. `mjx-layout-xlsx` issues **no** [`GeometryRef`](mjx_layout::GeometryRef): a cell is a
//! rectangle, a border is a rectangle, and a rectangle needs no path table. So a worksheet's scene
//! contains no unresolved outline and this provider is never called.
//!
//! That is exactly why it **refuses** rather than standing in. `mjx-scene`'s own
//! [`PlaceholderGeometry`](mjx_scene::PlaceholderGeometry) answers every handle with a framed
//! crossed rectangle, which is right for a deck of shapes that must render; here it would turn *a
//! handle nobody issued* — which can only be a defect in the box model or a resolver paired with
//! the wrong page — into a visible box that a reader would report as a rendering bug rather than as
//! the plumbing error it is. Refusing surfaces it at the first frame, in the crate that caused it.
//!
//! # What this means for `DrawReport::placeholders`
//!
//! [`unregistered`](SheetGeometry::unregistered) is zero and will stay zero until a worksheet grows
//! a shape, so an Excel render's placeholder count is a genuine but *narrow* assertion: it says the
//! painter drew no stand-in, computed independently of the render, and it would go red the moment a
//! handle appeared from anywhere. It is not the same weight of evidence as PowerPoint's, where the
//! number is the count of shapes whose `a:prstGeom` this build has no table for, and this paragraph
//! is here so that nobody reads it as if it were.
//!
//! # Where the drawings arrive — and why they still register nothing
//!
//! MJXOFF-173 has now put `xdr:twoCellAnchor` drawings on a sheet, and **this type did not change**.
//! That is worth saying plainly, because the paragraph this replaces predicted the opposite.
//!
//! A drawing reaches the fragment tree as a **box**: its rectangle, resolved against the box model's
//! own row heights and column widths, and its source address. Its *content* does not, and cannot
//! yet: a shape's `a:prstGeom` is laid out by `mjx-layout-pptx`, which sits at rank 3.6 — the same
//! rank as `mjx-layout-xlsx` — so the edge between the two box models is *sideways* and
//! `xtask/tests/layering.rs` refuses it by name. No `GeometryRef` is issued for a worksheet, so
//! there is still nothing to register, and a registry that claimed otherwise would be worse than a
//! refusal.
//!
//! When a crate below both box models owns DrawingML shape layout, this type grows a registry and
//! the `mjx-geometry` edge that rank 3.7 already permits. That crate does not exist, and naming
//! which child creates it is a decision for whoever schedules it.

use mjx_scene::{GeometryProvider, ResolvedOutline, SceneError, SceneRect};

/// The provider for a worksheet: it answers no handle, because a worksheet issues none.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SheetGeometry;

impl SheetGeometry {
    /// The provider.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// How many handles resolve to the document's own geometry. Zero, and see the module
    /// documentation for why that is a statement rather than a gap.
    #[must_use]
    pub fn registered(&self) -> usize {
        0
    }

    /// How many do not, and would therefore reach a stand-in.
    ///
    /// Zero, and it is the number a render gate compares `mjx_paint::DrawReport::placeholders`
    /// against — computed here, before a pixel is drawn, so the two are independent.
    #[must_use]
    pub fn unregistered(&self) -> usize {
        0
    }
}

impl GeometryProvider for SheetGeometry {
    fn outline(&self, outline: u64, _within: SceneRect) -> Result<ResolvedOutline, SceneError> {
        // See the module documentation: a stand-in here would hide a defect behind a shape.
        Err(SceneError::UnresolvedOutline { outline })
    }
}
