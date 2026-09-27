//! [`SlideGeometry`] — what a slide's outline handles resolve to.
//!
//! # Why the box model does not do this itself
//!
//! `crates/mjx-layout-pptx/tests/the_seam_holds.rs` refuses `mjx-geometry` **by name**, and its
//! reasoning is the whole point of `docs/UI_PLATFORM_PLAN.md` §4 L4's provider seam: a box model
//! that generated preset paths would put a path table inside a [`FragmentTree`] and make the
//! provider unswappable. So a [`mjx_layout::ShapeFragment`] carries a bare number, and this is where
//! the number becomes a shape.
//!
//! # MJX-STAND-IN: this crate's default provider stands in, and says so where a reader will look
//!
//! MJX-STAND-IN: [`SlideGeometry::new`] takes `UnknownShapePolicy::StandIn`, so a shape whose preset
//! this build has no table for draws a framed crossed rectangle rather than failing the page. That
//! is the policy's own documented answer for an *application*, and it is only safe because it is
//! **counted**: [`SlideGeometry::unregistered`] says how many before a pixel is drawn and
//! `mjx_paint::DrawReport::placeholders` says how many were. A gate uses
//! [`SlideGeometry::refusing_unknown_shapes`] instead, which fails rather than rendering a lie.
//! `crates/mjx-geometry/tests/the_stand_in_is_named_wherever_it_is_used.rs` does not require this
//! note — it greps for `PlaceholderGeometry` constructions, and this reaches the stand-in through
//! the provider's own fall-through — and it is written anyway, because the next reader of this file
//! is asking exactly the question that gate exists to answer.
//!
//! [`FragmentTree`]: mjx_layout::FragmentTree

use mjx_geometry::{PresetGeometryProvider, ShapeOutline};
use mjx_layout_pptx::PageCatalogue;
use mjx_scene::{GeometryProvider, ResolvedOutline, SceneError, SceneRect};

/// The provider for one page's outline handles.
///
/// Built by [`SlideGeometry::register`] from the catalogue the box model produced and the
/// document's own `a:prstGeom`, which the caller reads: this crate does not name `mjx-pptx` in
/// `[dependencies]`, because a resolver that opened a package would be a second reader of a document
/// beside the one that laid it out.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct SlideGeometry {
    presets: PresetGeometryProvider,
    registered: usize,
    unregistered: usize,
}

impl SlideGeometry {
    /// A provider that stands in — visibly — for a shape it cannot draw.
    ///
    /// [`mjx_geometry::UnknownShapePolicy::StandIn`] rather than
    /// [`Refuse`](mjx_geometry::UnknownShapePolicy::Refuse), and the reason is the one that
    /// policy's own documentation gives: a deck whose every shape must draw is better
    /// served by an obviously-wrong shape it can *count* than by a page that refuses to render.
    /// The count is what makes it safe — every stand-in raises
    /// `mjx_paint::DrawReport::placeholders`, so no image taken against this can be recorded as
    /// fidelity — and [`SlideGeometry::unregistered`] says how many before a pixel is drawn at all.
    #[must_use]
    pub fn new() -> Self {
        Self {
            presets: PresetGeometryProvider::standing_in_for_unknown_shapes(),
            registered: 0,
            unregistered: 0,
        }
    }

    /// A provider that **refuses** a shape it cannot draw.
    ///
    /// What a gate uses: a page that must render every shape from the document's own geometry says
    /// so by failing rather than by standing in.
    #[must_use]
    pub fn refusing_unknown_shapes() -> Self {
        Self {
            presets: PresetGeometryProvider::new(),
            registered: 0,
            unregistered: 0,
        }
    }

    /// Records what the handle at `index` of `catalogue`'s geometry table draws.
    ///
    /// `outline` is what the caller read out of the document for that shape — `None` for a shape
    /// with custom geometry, with none of its own, or with a preset this build has no table for,
    /// each of which reaches the policy rather than a wrong outline.
    pub fn register(&mut self, index: usize, outline: Option<ShapeOutline>) {
        match outline {
            Some(outline) => {
                self.presets.register(index as u64, outline);
                self.registered += 1;
            }
            None => self.unregistered += 1,
        }
    }

    /// Registers every handle of `catalogue`, asking `read` what each one draws.
    ///
    /// The shape of the join: the catalogue says *which shape, at what size*, and `read` — which is
    /// the caller's, because only the caller holds the package — says what that shape's
    /// `a:prstGeom` is. Neither half knows the other's vocabulary, which is what keeps the provider
    /// swappable.
    pub fn register_all(
        &mut self,
        catalogue: &PageCatalogue,
        mut read: impl FnMut(&mjx_layout_pptx::ShapeOutlineRequest) -> Option<ShapeOutline>,
    ) {
        for index in 0..catalogue.geometry_count() {
            let Some(request) = catalogue.geometry(mjx_layout::GeometryRef::new(index as u64))
            else {
                continue;
            };
            let outline = read(request);
            self.register(index, outline);
        }
    }

    /// How many handles resolve to the document's own geometry.
    #[must_use]
    pub fn registered(&self) -> usize {
        self.registered
    }

    /// How many do not, and will therefore reach the policy.
    ///
    /// **Read this before trusting a render.** A non-zero count is not an error and is not fidelity
    /// either: it is the number of shapes whose outline came from somewhere other than the document.
    #[must_use]
    pub fn unregistered(&self) -> usize {
        self.unregistered
    }
}

impl GeometryProvider for SlideGeometry {
    fn outline(&self, outline: u64, within: SceneRect) -> Result<ResolvedOutline, SceneError> {
        self.presets.outline(outline, within)
    }
}
