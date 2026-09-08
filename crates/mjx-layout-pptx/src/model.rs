//! [`SlideBoxModel`] — the first implementation of [`BoxModel`], and the code that turns a slide
//! into a [`FragmentTree`](mjx_layout::FragmentTree).
//!
//! # One slide is one page
//!
//! PowerPoint's layout discipline is the simplest in the programme, and every one of the four
//! contract methods is easier here than it will be for Word or Excel:
//!
//! * **[`BoxModel::estimate_extent`] is exact.** A deck has as many pages as it has slides, and the
//!   number is known without laying anything out — so this is the one box model that answers
//!   [`ExtentPrecision::Exact`] from a method whose whole purpose is to be allowed to guess. That is
//!   a useful contrast rather than a triviality: a caller that draws a scrollbar from an
//!   `Estimated` extent must let it move as real pages arrive, and one that draws it from an
//!   `Exact` extent must not.
//! * **[`BoxModel::invalidate`] names pages rather than a suffix.** Nothing reflows, so an edit to
//!   slide 7 dirties slide 7. A change to a *layout* or a *master* is the exception and is honest
//!   about it: this box model does not track which slides use which layout, so it answers
//!   [`DirtyPages::All`] rather than guessing.
//! * **The checkpoint carries four bytes** — which slide the next page is. It exists at all because
//!   the contract uses `continuation.is_none()` to mean *this was the last page*, and a caller
//!   iterating a deck needs that.
//! * **[`BoxModel::layout_page`] does not depend on the page before it.** The equivalence the
//!   contract asks for — laying out pages 1..=N in order and laying out page N alone give the same
//!   fragments — is free here, and a suite asserts it anyway, because "free" is a property of this
//!   implementation and not of the trait.
//!
//! # Why this holds a `GlyphRasteriser`
//!
//! A [`GlyphRunFragment`] names a [`FaceId`](mjx_text::FaceId), and the only thing that mints one is
//! `GlyphRasteriser::register`. A painter has to resolve the same identity to the same face, so the
//! box model and the painter must share **one** rasteriser — which means the box model owns one and
//! lends it ([`SlideBoxModel::rasteriser_mut`]) rather than each end keeping its own.
//!
//! That is a seam finding rather than a design: a box model that only ever *measures* has no reason
//! to hold a rasteriser, and `FaceId` being minted by the rasteriser rather than by the resolver is
//! what forces it. It is reported to R03/R04 as such.

use mjx_dml::{FillSpec, LineSpec};
use mjx_layout::{
    BoxFragment, BoxModel, ChangeSet, Checkpoint, Constraints, DecorationRef, DirtyPages, Extent,
    ExtentPrecision, Fragment, FragmentId, FragmentTreeBuilder, GeometryRef, GlyphRunFragment,
    LayoutError, LayoutPoint, LayoutRect, LineFragment, ModelSignature, PageFragments, PageIndex,
    ShapeFragment, SourcePath, Transform, TransformId,
};
use mjx_ooxml_core::measure::Emu;
use mjx_pptx::ShapeKind;
use mjx_text::{FeatureSet, FontResolver, GlyphRasteriser, Shaper};

use crate::address;
use crate::autofit::{AutofitOutcome, AutofitPolicy};
use crate::body::{self, PlacedBody, PlacedLine};
use crate::deck::{Shape, ShapeDecoration, SlideDeck};
use crate::error::SlideLayoutError;
use crate::text::TextEngine;

/// What a [`DecorationRef`] this box model issued resolves to.
///
/// The fragment tree carries only the number, deliberately: `mjx-layout` may never learn what a
/// DrawingML fill is. This is the table the layer that holds the box model reads it in — a scene
/// builder pairs a fragment with the entry its handle names, which is exactly the division
/// [`DecorationRef`]'s own documentation describes.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Decoration {
    /// What fills the shape, or `None` when nothing does.
    pub fill: Option<FillSpec>,
    /// What outlines it, or `None` when nothing does.
    pub outline: Option<LineSpec>,
}

/// What a [`GeometryRef`] this box model issued resolves to.
///
/// **Not a path.** `docs/UI_PLATFORM_PLAN.md` §4 L4's `GeometryProvider` is what turns a shape into
/// an outline, and it lives above this crate; a box model that generated preset paths would have
/// made the provider unswappable. So this says *which shape, at what size*, and stops.
#[derive(Clone, PartialEq, Debug)]
pub struct ShapeOutlineRequest {
    /// Which surface the shape is on.
    pub surface_index: u32,
    /// Its path through the shape tree.
    pub shape: Vec<u32>,
    /// The rectangle it is drawn at, in its own coordinate space.
    pub rect: LayoutRect,
}

/// The tables the handles in one page's fragments resolve through.
///
/// Rebuilt on every [`BoxModel::layout_page`], because a handle is only meaningful for the page it
/// was issued on — two pages numbering their decorations independently is what keeps the numbers
/// small and the table a `Vec`.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct PageCatalogue {
    decorations: Vec<Decoration>,
    geometries: Vec<ShapeOutlineRequest>,
    /// How deep each shape's path is, so a hit test can split a `SourceRef` correctly.
    shape_depths: Vec<(SourcePath, usize)>,
    autofit: Vec<(SourcePath, AutofitOutcome)>,
}

impl PageCatalogue {
    /// What a decoration handle resolves to.
    #[must_use]
    pub fn decoration(&self, handle: DecorationRef) -> Option<&Decoration> {
        usize::try_from(handle.number())
            .ok()
            .and_then(|index| self.decorations.get(index))
    }

    /// What a geometry handle resolves to.
    #[must_use]
    pub fn geometry(&self, handle: GeometryRef) -> Option<&ShapeOutlineRequest> {
        usize::try_from(handle.number())
            .ok()
            .and_then(|index| self.geometries.get(index))
    }

    /// How deep the shape path of the fragment at `path` is — the number
    /// [`TextHit::from_source`](crate::TextHit::from_source) needs and cannot infer.
    #[must_use]
    pub fn shape_depth(&self, path: &SourcePath) -> Option<usize> {
        self.shape_depths
            .iter()
            .filter(|(shape, _)| shape.contains(path))
            .map(|(_, depth)| *depth)
            .max()
    }

    /// What autofit did to the text body of the shape at `path`.
    #[must_use]
    pub fn autofit(&self, path: &SourcePath) -> Option<AutofitOutcome> {
        self.autofit
            .iter()
            .find(|(shape, _)| shape == path)
            .map(|(_, outcome)| *outcome)
    }

    /// Every autofit outcome on the page, with the shape it belongs to.
    #[must_use]
    pub fn autofit_outcomes(&self) -> &[(SourcePath, AutofitOutcome)] {
        &self.autofit
    }

    /// How many decoration handles were issued.
    #[must_use]
    pub fn decoration_count(&self) -> usize {
        self.decorations.len()
    }

    /// How many geometry handles were issued.
    #[must_use]
    pub fn geometry_count(&self) -> usize {
        self.geometries.len()
    }
}

/// PowerPoint's box model.
#[derive(Debug)]
pub struct SlideBoxModel {
    fonts: FontResolver,
    rasteriser: GlyphRasteriser,
    shaper: Shaper,
    features: FeatureSet,
    policy: AutofitPolicy,
    catalogue: PageCatalogue,
}

impl SlideBoxModel {
    /// This box model's signature.
    ///
    /// `"PPTXLAY1"` in ASCII. The trailing `1` is the version of the *continuation state*, not of
    /// the crate: if the four bytes a checkpoint carries ever change shape, this number changes and
    /// every old checkpoint is refused rather than misread.
    pub const SIGNATURE: ModelSignature = ModelSignature::new(0x5050_5458_4C41_5931);

    /// A box model that resolves faces through `fonts`.
    #[must_use]
    pub fn new(fonts: FontResolver) -> Self {
        Self {
            fonts,
            rasteriser: GlyphRasteriser::new(),
            shaper: Shaper::new(),
            features: FeatureSet::new(),
            policy: AutofitPolicy::default(),
            catalogue: PageCatalogue::default(),
        }
    }

    /// The same box model with a different autofit policy.
    #[must_use]
    pub fn with_autofit(mut self, policy: AutofitPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The same box model shaping every run with `features`.
    #[must_use]
    pub fn with_features(mut self, features: FeatureSet) -> Self {
        self.features = features;
        self
    }

    /// What it does about autofit.
    #[must_use]
    pub fn autofit_policy(&self) -> AutofitPolicy {
        self.policy
    }

    /// The font resolver, for the substitution manifest a document reports.
    #[must_use]
    pub fn fonts(&self) -> &FontResolver {
        &self.fonts
    }

    /// The rasteriser that minted every [`FaceId`](mjx_text::FaceId) in the fragments.
    ///
    /// A painter must draw through **this** rasteriser: a `FaceId` is an index into the rasteriser
    /// that issued it, so a second one would number the same faces differently and draw the wrong
    /// glyphs with no error anywhere.
    pub fn rasteriser_mut(&mut self) -> &mut GlyphRasteriser {
        &mut self.rasteriser
    }

    /// What the handles in the last page laid out resolve to.
    #[must_use]
    pub fn catalogue(&self) -> &PageCatalogue {
        &self.catalogue
    }
}

impl BoxModel for SlideBoxModel {
    type Content = SlideDeck;
    type Error = SlideLayoutError;

    fn signature(&self) -> ModelSignature {
        Self::SIGNATURE
    }

    fn layout_page(
        &mut self,
        content: &Self::Content,
        page: PageIndex,
        constraints: &Constraints,
        resume: Option<&Checkpoint>,
    ) -> Result<PageFragments, Self::Error> {
        let index = self.resume_from(page, resume)?;
        let last = content.slide_count().saturating_sub(1);
        let Some(slide) = content.slide(index) else {
            return Err(LayoutError::PageBeyondContent {
                requested: page,
                last: PageIndex::new(u32::try_from(last).unwrap_or(u32::MAX)),
            }
            .into());
        };

        let page_rect = LayoutRect::from_origin_and_size(LayoutPoint::ORIGIN, constraints.page);
        let mut catalogue = PageCatalogue::default();
        let mut builder = FragmentTreeBuilder::new();
        let surface = u32::try_from(index).unwrap_or(u32::MAX);

        let root = builder.push_simple(
            None,
            address::node(address::SLIDES, SourcePath::new(&[surface])),
            page_rect,
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        );

        // The shapes are already flattened into paint order, and a group's own box precedes its
        // members. Nesting them again would need a stack of builder identifiers keyed by path; a
        // map from path to identifier is the same thing without the stack, and it is what keeps a
        // malformed tree — a member whose group was skipped for having no bounds — from losing the
        // member as well.
        let mut parents: Vec<(Vec<u32>, FragmentId)> = Vec::new();
        for shape in slide.shapes() {
            let parent = parents
                .iter()
                .rev()
                .find(|(path, _)| shape.path.starts_with(path) && path.len() < shape.path.len())
                .map(|(_, id)| *id)
                .or(root);
            if let Some(id) = self.lay_out_shape(
                &mut builder,
                &mut catalogue,
                surface,
                shape,
                parent,
                constraints,
            )? {
                if shape.kind == ShapeKind::GroupShape {
                    parents.push((shape.path.clone(), id));
                }
            }
        }

        self.catalogue = catalogue;
        let continuation = if index >= last {
            None
        } else {
            let next = index.saturating_add(1);
            let state = u32::try_from(next).unwrap_or(u32::MAX).to_le_bytes();
            Some(Checkpoint::new(
                Self::SIGNATURE,
                page,
                address::node(
                    address::SLIDES,
                    SourcePath::new(&[u32::try_from(next).unwrap_or(u32::MAX)]),
                ),
                state.to_vec(),
            )?)
        };
        Ok(PageFragments::new(page, builder.finish(), continuation))
    }

    fn estimate_extent(&self, content: &Self::Content, constraints: &Constraints) -> Extent {
        Extent {
            // Not an estimate. A deck has as many pages as it has slides, and counting them costs
            // nothing — which is why this is the one box model whose "estimate" is the answer.
            pages: u32::try_from(content.slide_count()).unwrap_or(u32::MAX),
            page_size: constraints.page,
            precision: ExtentPrecision::Exact,
        }
    }

    fn invalidate(&mut self, change: &ChangeSet) -> DirtyPages {
        if change.is_empty() {
            return DirtyPages::None;
        }
        let mut pages: Vec<PageIndex> = Vec::new();
        for content_change in change.changes() {
            let source = &content_change.source;
            match source.part().number() {
                // A slide's own change dirties that slide and no other: nothing on a slide reflows
                // onto another one.
                0 => {
                    let Some(&index) = source.path().segments().first() else {
                        return DirtyPages::All;
                    };
                    let page = PageIndex::new(index);
                    if !pages.contains(&page) {
                        pages.push(page);
                    }
                }
                // A layout or a master is inherited by an unknown set of slides. This box model does
                // not track which, so it says so rather than guessing — a wrong `Pages` here would
                // leave a stale slide on screen with nothing to notice it.
                _ => return DirtyPages::All,
            }
        }
        pages.sort_unstable();
        DirtyPages::Pages(pages)
    }
}

impl SlideBoxModel {
    /// Which slide `page` lays out, checking the checkpoint it was handed.
    fn resume_from(
        &self,
        page: PageIndex,
        resume: Option<&Checkpoint>,
    ) -> Result<usize, SlideLayoutError> {
        let Some(checkpoint) = resume else {
            return Ok(page.number() as usize);
        };
        let state = checkpoint.state_for(Self::SIGNATURE, page)?;
        let bytes: [u8; 4] = state
            .try_into()
            .map_err(|_| SlideLayoutError::MalformedContinuation(state.len()))?;
        Ok(u32::from_le_bytes(bytes) as usize)
    }

    /// Lays one shape out and returns its fragment, or `None` when nothing places it.
    fn lay_out_shape(
        &mut self,
        builder: &mut FragmentTreeBuilder,
        catalogue: &mut PageCatalogue,
        surface: u32,
        shape: &Shape,
        parent: Option<FragmentId>,
        constraints: &Constraints,
    ) -> Result<Option<FragmentId>, SlideLayoutError> {
        // A shape no tier places has no rectangle, and drawing it at the origin would put something
        // on the slide that PowerPoint does not.
        let Some(rect) = shape.bounds else {
            return Ok(None);
        };
        let path = address::shape_path(surface, &shape.path);
        catalogue
            .shape_depths
            .push((path.clone(), shape.path.len()));

        let transform = builder.transform(fragment_transform(shape, rect));
        let decoration = decoration_handle(catalogue, &shape.decoration);
        let fragment = match shape.kind {
            ShapeKind::Shape | ShapeKind::ConnectionShape => {
                let geometry = GeometryRef::new(catalogue.geometries.len() as u64);
                catalogue.geometries.push(ShapeOutlineRequest {
                    surface_index: surface,
                    shape: shape.path.clone(),
                    rect,
                });
                Fragment::Shape(ShapeFragment {
                    geometry,
                    decoration,
                })
            }
            // A picture is an `ImageFragment` once there is a resource table to name its pixels
            // (R15). Until then it is the box it occupies, so it is hit-testable and takes up the
            // room it will take up, rather than being absent and moving nothing when it arrives.
            _ => Fragment::Box(BoxFragment {
                decoration,
                cell: None,
            }),
        };

        let id = builder.push(
            parent,
            address::node(address::SLIDES, path.clone()),
            rect,
            transform,
            None,
            fragment,
        );
        let Some(id) = id else {
            return Ok(None);
        };

        if let Some(body) = &shape.body {
            // A vertical text body is laid out in a **transposed** rectangle and then turned, so
            // every line-breaking decision is made against the measure the text actually has. See
            // `body::vertical_layout`.
            let vertical = body::vertical_layout(rect, &body.geometry);
            let placed = {
                let mut engine = TextEngine {
                    fonts: &mut self.fonts,
                    rasteriser: &mut self.rasteriser,
                    shaper: &mut self.shaper,
                    features: &self.features,
                };
                body::lay_out(&mut engine, vertical.rect, body, self.policy)?
            };
            catalogue.autofit.push((path.clone(), placed.outcome));

            // The body's own turn composes *under* the shape's rotation and flips, because a
            // vertical text body inside a rotated shape turns with the shape.
            let body_transform = match vertical.turn {
                None => transform,
                Some(turn) => builder.transform(turn.then(fragment_transform(shape, rect))),
            };
            build_body(
                builder,
                id,
                body_transform,
                surface,
                shape,
                &placed,
                constraints,
            );
        }
        Ok(Some(id))
    }
}

/// The affine map a shape's rotation and flips compose to.
///
/// Deliberately **not** called `shape_transform`: `mjx-pptx` has a reader of that name that answers
/// what a shape *declares* rather than what it renders at, and
/// `tests/the_ladder_is_consumed.rs` greps for exactly that spelling. A local helper that shadowed
/// it would make the grep unable to tell a re-derivation from a coincidence.
///
/// Every part of it turns about the **centre of the shape's own box**, because that is what
/// `a:xfrm@rot`, `@flipH` and `@flipV` all do. Composing them about the origin instead would move a
/// rotated shape to somewhere else on the slide, which is the classic way to get this wrong.
fn fragment_transform(shape: &Shape, rect: LayoutRect) -> Transform {
    let unrotated = shape.rotation.radians() == 0.0;
    if unrotated && !shape.flip_horizontal && !shape.flip_vertical {
        return Transform::IDENTITY;
    }
    let centre = LayoutPoint::new(
        rect.left + rect.width().divided_by(2),
        rect.top + rect.height().divided_by(2),
    );
    let scale = Transform::scale(
        if shape.flip_horizontal { -1.0 } else { 1.0 },
        if shape.flip_vertical { -1.0 } else { 1.0 },
    );
    Transform::translation(-centre.x, -centre.y)
        .then(scale)
        .then(Transform::rotation(shape.rotation))
        .then(Transform::translation(centre.x, centre.y))
}

/// The handle a shape's decoration is issued under, or `None` when it paints nothing.
fn decoration_handle(
    catalogue: &mut PageCatalogue,
    decoration: &ShapeDecoration,
) -> Option<DecorationRef> {
    if decoration.is_empty() {
        return None;
    }
    let handle = DecorationRef::new(catalogue.decorations.len() as u64);
    catalogue.decorations.push(Decoration {
        fill: decoration.fill.clone(),
        outline: decoration.outline.clone(),
    });
    Some(handle)
}

/// Adds a laid-out text body's fragments under its shape.
fn build_body(
    builder: &mut FragmentTreeBuilder,
    shape_id: FragmentId,
    transform: TransformId,
    surface: u32,
    shape: &Shape,
    placed: &PlacedBody,
    constraints: &Constraints,
) {
    let clip = clip_for(builder, placed, constraints);
    for column in &placed.columns {
        if column.lines.is_empty() {
            continue;
        }
        let Some(column_id) = builder.push(
            Some(shape_id),
            address::node(address::SLIDES, address::shape_path(surface, &shape.path)),
            column.rect,
            transform,
            clip,
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        ) else {
            continue;
        };

        // A paragraph's box is the union of the lines of it that landed in this column, and a
        // paragraph split across two columns therefore has one box in each. The union has to be
        // known before the box is pushed, because a fragment's rectangle is fixed when it is added.
        //
        // Every index here is a `get`: the loop bounds make an out-of-range one impossible, and it
        // is written this way anyway because *"this index cannot be out of range"* is a claim about
        // a document, and a document is untrusted input.
        let mut piece_start = 0_usize;
        while let Some(first) = column.lines.get(piece_start) {
            let paragraph = first.paragraph;
            let mut piece_end = piece_start;
            let mut bounds = LayoutRect::ZERO;
            while let Some(line) = column
                .lines
                .get(piece_end)
                .filter(|line| line.paragraph == paragraph)
            {
                bounds = bounds.union(line.rect);
                piece_end += 1;
            }
            let paragraph_path = address::paragraph_path(surface, &shape.path, paragraph);
            let Some(paragraph_id) = builder.push(
                Some(column_id),
                address::node(address::SLIDES, paragraph_path.clone()),
                bounds,
                transform,
                clip,
                Fragment::Box(BoxFragment {
                    decoration: None,
                    cell: None,
                }),
            ) else {
                piece_start = piece_end;
                continue;
            };
            for line in column.lines.get(piece_start..piece_end).unwrap_or_default() {
                build_line(
                    builder,
                    paragraph_id,
                    transform,
                    clip,
                    surface,
                    shape,
                    &paragraph_path,
                    line,
                );
            }
            piece_start = piece_end;
        }
    }
}

/// The clip a body's text is drawn under, or `None` when nothing needs clipping.
///
/// Text that overflowed its shape is still emitted — the fragments are what a reader would see in
/// PowerPoint, which does draw overflowing text — so nothing is clipped to the shape. The page is
/// clipped, because a shape placed partly off-slide draws only the part that is on it.
fn clip_for(
    builder: &mut FragmentTreeBuilder,
    placed: &PlacedBody,
    constraints: &Constraints,
) -> Option<mjx_layout::ClipId> {
    let page = LayoutRect::from_origin_and_size(LayoutPoint::ORIGIN, constraints.page);
    if page.contains(LayoutPoint::new(placed.content.left, placed.content.top))
        && page.contains(LayoutPoint::new(
            placed.content.right - Emu::from_emu(1),
            placed.content.bottom - Emu::from_emu(1),
        ))
    {
        return None;
    }
    builder.clip(page)
}

/// Adds one line and its glyph runs.
#[allow(clippy::too_many_arguments)]
fn build_line(
    builder: &mut FragmentTreeBuilder,
    paragraph_id: FragmentId,
    transform: TransformId,
    clip: Option<mjx_layout::ClipId>,
    surface: u32,
    shape: &Shape,
    paragraph_path: &SourcePath,
    line: &PlacedLine,
) {
    let Some(line_id) = builder.push(
        Some(paragraph_id),
        address::span(address::SLIDES, paragraph_path.clone(), line.range.clone()),
        line.rect,
        transform,
        clip,
        Fragment::Line(LineFragment {
            baseline: line.baseline,
            ascent: line.ascent,
            descent: line.descent,
            base_direction: line.base_direction,
            hanging_width: line.hanging_width,
        }),
    ) else {
        return;
    };

    // The marker is drawn first, so it is behind — and before — the text it marks, which is what
    // makes a hit test on the overlap answer the text.
    if let Some(marker) = &line.marker {
        builder.push(
            Some(line_id),
            address::node(address::SLIDES, paragraph_path.clone()),
            LayoutRect::from_edges(
                marker.origin.x,
                line.rect.top,
                marker.origin.x + marker.width,
                line.rect.bottom,
            ),
            transform,
            clip,
            Fragment::GlyphRun(GlyphRunFragment {
                face: marker.face,
                run: marker.shaped.clone(),
                origin: marker.origin,
                direction: mjx_text::TextDirection::LeftToRight,
                level: mjx_text::BidiLevel::LEFT_TO_RIGHT,
            }),
        );
    }

    for piece in &line.pieces {
        builder.push(
            Some(line_id),
            address::span(
                address::SLIDES,
                address::run_path(surface, &shape.path, line.paragraph, piece.run),
                piece.range_in_run.clone(),
            ),
            LayoutRect::from_edges(
                piece.origin.x,
                line.rect.top,
                piece.origin.x + piece.width,
                line.rect.bottom,
            ),
            transform,
            clip,
            Fragment::GlyphRun(GlyphRunFragment {
                face: piece.face,
                run: piece.shaped.clone(),
                origin: piece.origin,
                direction: piece.direction,
                level: piece.level,
            }),
        );
    }
}

/// The constraints a deck's own slide size gives.
///
/// A caller may lay a slide out at any size it likes — that is what `Constraints` is for — but the
/// size a deck states is the one it was authored at, and almost every caller wants exactly that.
#[must_use]
pub fn constraints_for(deck: &SlideDeck) -> Constraints {
    Constraints {
        page: deck.page(),
        // A slide has no margins: a shape is placed absolutely inside the whole slide, and the
        // insets that look like margins belong to each text body rather than to the page.
        content: LayoutRect::from_origin_and_size(LayoutPoint::ORIGIN, deck.page()),
        columns: 1,
        column_gap: Emu::ZERO,
        base_direction: mjx_text::TextDirection::LeftToRight,
        writing_mode: mjx_layout::WritingMode::HorizontalTopToBottom,
    }
}
