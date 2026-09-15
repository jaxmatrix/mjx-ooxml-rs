//! [`build_scene`] — a [`FragmentTree`] in, a [`DisplayList`] out, and nothing else consulted.
//!
//! # Driven only by fragments
//!
//! The walk below reads six fragment kinds, a rectangle, a transform, a clip and a [`SourceRef`].
//! It has never heard of a slide, a paragraph property, a theme or a package, and it cannot: this
//! crate depends on `mjx-layout` and on no format crate, and `xtask/tests/layering.rs` refuses the
//! edge that would let it. That is the seam `docs/UI_PLATFORM_PLAN.md` §2 draws, seen from above —
//! and `tests/fragments_alone_drive_the_builder.rs` builds a scene from the **foreign** box model
//! `mjx-layout`'s own gate carries, which shares no ancestry with any OOXML one.
//!
//! # The three opaque handles, and who resolves them
//!
//! A fragment says *this box is decorated*, *this shape has that outline*, *this is picture 7* —
//! and says it with a [`DecorationRef`], a [`GeometryRef`] and an [`ImageRef`], all of which are
//! bare integers. `mjx-layout` is explicit about why: decoration is where a box model's own
//! vocabulary would leak into the seam, and DrawingML's fill/line/effect model is not CSS's is not a
//! plain-text renderer's.
//!
//! So a [`ResourceResolver`] is handed in, and it is **the box model's companion** — the thing that
//! issued those numbers. A PowerPoint scene pairs a PowerPoint fragment tree with a resolver that
//! reads `mjx-dml`; a Markdown one pairs its tree with a resolver that returns three colours. The
//! builder never learns which.
//!
//! A [`GeometryRef`] is the exception and is deliberately *not* resolved here: it becomes a
//! [`Geometry::Unresolved`] carrying the handle and the box, because §4 L4's `GeometryProvider` is
//! R07's and resolving a preset shape into paths is what R07 does.
//!
//! # What a glyph run costs here that it did not cost in layout
//!
//! Everything resolution-dependent. [`place_run`] is called with the device scale, every glyph is
//! given an image out of the atlas, and a glyph too large for a bitmap becomes an ordinary path in
//! the geometry table. A fragment tree survives a zoom; a display list is rebuilt by it, and this
//! function is the rebuild.
//!
//! # The one interpretation this builder makes, and the three claims it is made of
//!
//! A fragment's clip is *absolute*: each node states the clip it is drawn under, and the contract
//! does not say what a child that states none is drawn under. A display list's clips **nest** — a
//! `PushClip` intersects, it never widens — so the builder installs a clip for a node and its whole
//! subtree, and a descendant that states none inherits it. That is the reading a clipped shape and
//! the text inside it need, and it can only ever narrow, never widen. It is written down here rather
//! than assumed because it is a choice, and `docs/UI_PLATFORM_PLAN.md`'s contract owner may want it
//! stated in `mjx-layout` instead.
//!
//! **That paragraph was once the only thing asserting any of it.** MJXOFF-161's review deleted
//! `node.clip().or(enclosing.clip)` outright and the crate stayed green at 53 passed, 0 failed; a
//! probe that aborted the process whenever an ancestor clipped and a descendant did not **never
//! fired**, so the rule was not weakly covered — nothing in the crate constructed the case at all.
//!
//! It is three claims, and they fail independently. Each is now gated against the emitted command
//! stream in `tests/fragments_alone_drive_the_builder.rs`, and each was proved by its own mutation:
//!
//! 1. **Inheritance is honoured** — a descendant naming no clip is drawn inside its ancestor's.
//!    Structural: it follows from the `Pop` below being emitted after the subtree rather than after
//!    the node. Popping early makes `a_descendant_that_states_no_clip_is_drawn_inside_its_ancestors`
//!    red.
//! 2. **An inherited clip is not re-installed** — `enclosing.clip` is read *only* by the
//!    `Some(clip_id) != enclosing.clip` comparison below, so `.or(enclosing.clip)` is observable
//!    only when a descendant **re-states** the ancestor's clip through an intervening node that
//!    states none. That needs a tree three deep, which is exactly why the review's mutation was
//!    green; it now makes `a_descendant_that_restates_an_inherited_clip_does_not_install_it_twice`
//!    red.
//! 3. **A clip is re-installed when the transform changes** — the `|| transform_changed` below. The
//!    same rectangle in a new space is a different region on the page, and a clip that silently
//!    failed to re-install would cut against the wrong rectangle, which is invisible until
//!    something is actually clipped away.

use mjx_layout::{
    DecorationRef, Fragment, FragmentId, FragmentTree, GeometryRef, ImageRef, LayoutLosses,
    LayoutSize, PageFragments, SourceRef, Transform,
};
use mjx_text::{place_run, DeviceScale, GlyphAtlas, GlyphRasteriser, Hinting, PreparedImage};
use mjx_tokens::Color;

use crate::build::SceneBuilder;
use crate::command::{Clip, Command};
use crate::effect::EffectStyle;
use crate::encoding::ResourceIndex;
use crate::error::SceneError;
use crate::geometry::{
    finite, pixels_from_emu, FillRule, Geometry, PathCommand, ScenePoint, SceneRect, SceneTransform,
};
use crate::glyphs::{AtlasPlacement, GlyphImage, SceneGlyph, SceneGlyphRun};
use crate::list::DisplayList;
use crate::loss::{LossCategory, Resolved, SceneLossKind};
use crate::paint::{FillStyle, Image, StrokeStyle};

/// What text is drawn in when the resolver says nothing about it.
///
/// A box model that carries no colour — the plain-text one in `mjx-layout`'s own tests is exactly
/// that — still has to produce a page a reader can see, and opaque black on the page's own
/// background is what "no colour was specified" has always meant in a document. The alternative,
/// refusing to draw, would make a scene from a colourless box model empty rather than plain.
pub const DEFAULT_TEXT_COLOR: Color = Color {
    red: 0,
    green: 0,
    blue: 0,
    alpha: 0xff,
};

/// Everything painting one thing needs, in a vocabulary with no OOXML in it.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Decoration {
    /// What fills it.
    pub fill: FillStyle,
    /// What outlines it, if anything.
    pub stroke: Option<StrokeStyle>,
    /// How opaque the whole thing and everything inside it is, `0.0` to `1.0`.
    pub opacity: f32,
    /// The effect DAG, in topological order; the **last** entry is the root.
    pub effects: Vec<EffectStyle>,
}

impl Decoration {
    /// A decoration that paints nothing and changes nothing.
    #[must_use]
    pub fn none() -> Self {
        Self {
            fill: FillStyle::None,
            stroke: None,
            opacity: 1.0,
            effects: Vec::new(),
        }
    }

    /// A decoration that is one fill and nothing else.
    #[must_use]
    pub fn filled(fill: FillStyle) -> Self {
        Self {
            fill,
            ..Self::none()
        }
    }

    /// Whether this draws nothing and wraps nothing, so that the node can be skipped entirely.
    #[must_use]
    pub fn is_invisible(&self) -> bool {
        self.fill.is_none()
            && self.stroke.is_none()
            && self.effects.is_empty()
            && self.opacity >= 1.0
    }
}

/// Turns the opaque handles a fragment tree carries into paints, strokes, effects and pictures.
///
/// Implemented by **the box model's companion** — the layer that issued the handles. Nothing here
/// can be answered by this crate, and nothing here has to be answered at all: a box model with no
/// decoration answers [`Resolved::NothingToDraw`] from every method and produces a scene of plain
/// text, which is exactly what the foreign box model in this crate's tests does.
pub trait ResourceResolver {
    /// What paints the box or shape that carries `reference`.
    fn decoration(&self, reference: DecorationRef) -> Resolved<Decoration>;

    /// What paints the text at `source`.
    ///
    /// **Addressed by [`SourceRef`], not by a handle**, because [`mjx_layout::GlyphRunFragment`]
    /// carries no decoration of its own — see this crate's hand-off notes. A `SourceRef` is the
    /// fragment vocabulary's own link back to the document, so answering from it reaches around
    /// nothing; a run's fill, outline and shadow are `a:rPr`'s in DrawingML and `w:rPr`'s in
    /// WordprocessingML, and the resolver is the layer that can read either.
    fn text_decoration(&self, source: &SourceRef) -> Resolved<Decoration>;

    /// Which picture `reference` names, and how it is drawn.
    fn image(&self, reference: ImageRef) -> Resolved<Image>;

    /// Why the content inside the box at `source` cannot be drawn as a whole, or `None` when it can.
    fn unanswerable_content(&self, _source: &SourceRef) -> Option<SceneLossKind> {
        None
    }
}

/// How a scene is built: at what scale, with what hinting, onto how large a page.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SceneOptions {
    /// How many device pixels one typographic point covers — the display's density, the operating
    /// system's factor and the reader's zoom, in one number.
    pub device_scale: DeviceScale,
    /// How the glyphs are grid-fitted. [`Hinting::Unhinted`] is what a rotated or animated page
    /// wants, because grid-fitting a glyph that is about to be transformed fits it to the wrong
    /// grid.
    pub hinting: Hinting,
    /// The page, in EMU, as the constraints that laid it out gave it.
    pub page: LayoutSize,
}

impl SceneOptions {
    /// A page of `page` at the unzoomed scale, grid-fitted.
    #[must_use]
    pub fn new(page: LayoutSize) -> Self {
        Self {
            device_scale: DeviceScale::UNZOOMED,
            hinting: Hinting::GridFitted,
            page,
        }
    }
}

/// Build the display list for one laid-out page: its fragments, every layout loss, and a placeholder for each over missing content with an area.
///
/// # Errors
///
/// As [`build_scene`].
pub fn build_page(
    page: &PageFragments,
    resolver: &dyn ResourceResolver,
    rasteriser: &mut GlyphRasteriser,
    atlas: &mut GlyphAtlas,
    options: &SceneOptions,
) -> Result<DisplayList, SceneError> {
    build(
        page.fragments(),
        page.losses(),
        resolver,
        rasteriser,
        atlas,
        options,
    )
}

/// Build the display list for a fragment tree that carries no layout losses.
///
/// # Errors
///
/// [`SceneError::Text`] when a glyph cannot be rasterised or the atlas cannot hold it, and whatever
/// [`SceneBuilder::finish`] rejects.
pub fn build_scene(
    tree: &FragmentTree,
    resolver: &dyn ResourceResolver,
    rasteriser: &mut GlyphRasteriser,
    atlas: &mut GlyphAtlas,
    options: &SceneOptions,
) -> Result<DisplayList, SceneError> {
    build(
        tree,
        &LayoutLosses::new(),
        resolver,
        rasteriser,
        atlas,
        options,
    )
}

// The walk both entry points share.
fn build(
    tree: &FragmentTree,
    layout_losses: &LayoutLosses,
    resolver: &dyn ResourceResolver,
    rasteriser: &mut GlyphRasteriser,
    atlas: &mut GlyphAtlas,
    options: &SceneOptions,
) -> Result<DisplayList, SceneError> {
    let scale = options.device_scale;
    let mut builder = SceneBuilder::new(
        scale,
        pixels_from_emu(options.page.width, scale),
        pixels_from_emu(options.page.height, scale),
    );

    // An explicit stack, never recursion: a fragment tree's depth is a document's, and a document
    // can nest a group inside a group inside a table cell as often as it likes. `mjx-layout` walks
    // its own tree the same way and for the same reason.
    let mut stack: Vec<Step> = Vec::new();
    for root in tree.roots().iter().rev() {
        stack.push(Step::Enter {
            node: *root,
            enclosing: Enclosing::PAGE,
        });
    }

    while let Some(step) = stack.pop() {
        let (node_id, enclosing) = match step {
            Step::Leave { pops } => {
                for _ in 0..pops {
                    builder.push(Command::Pop)?;
                }
                continue;
            }
            Step::Enter { node, enclosing } => (node, enclosing),
        };
        let Some(node) = tree.node(node_id) else {
            continue;
        };

        let absolute = tree.transform(node.transform());
        let rect = SceneRect::from_layout(node.rect(), scale);
        let content_loss = match node.fragment() {
            Fragment::Box(_) => resolver.unanswerable_content(node.source()),
            _ => None,
        };
        let (decoration, decoration_losses) = match content_loss {
            Some(_) => (None, Vec::new()),
            None => match decoration_of(node.fragment(), resolver) {
                Resolved::Answered(decoration) => (Some(decoration), Vec::new()),
                Resolved::NothingToDraw => (None, Vec::new()),
                Resolved::Unanswerable(kind) => (None, vec![kind]),
                // What survives is drawn, and a decoration with nothing left in it draws nothing.
                Resolved::Partial(decoration, lost) => {
                    ((!decoration.is_invisible()).then_some(decoration), lost)
                }
            },
        };
        let mut pops = 0_usize;

        let transform_changed = absolute != enclosing.transform;
        if transform_changed {
            let relative = relative_transform(enclosing.transform, absolute);
            let index = builder.add_transform(SceneTransform::from_layout(relative, scale))?;
            builder.push(Command::PushTransform(index))?;
            pops += 1;
        }
        if let Some(clip_id) = node.clip() {
            if Some(clip_id) != enclosing.clip || transform_changed {
                if let Some(rect) = tree.clip(clip_id) {
                    let index =
                        builder.add_clip(Clip::rectangle(SceneRect::from_layout(rect, scale)))?;
                    builder.push(Command::PushClip(index))?;
                    pops += 1;
                }
            }
        }
        if let Some(decoration) = &decoration {
            if decoration.opacity < 1.0 {
                builder.push(Command::PushOpacity(finite(decoration.opacity).max(0.0)))?;
                pops += 1;
            }
            if let Some(root) = builder.add_effect_styles(&decoration.effects)? {
                builder.push(Command::PushEffect(root))?;
                pops += 1;
            }
        }

        if let Some(kind) = content_loss {
            builder.add_loss(
                LossCategory::Scene(kind),
                node.source(),
                placeholder_over(kind, rect),
            )?;
        } else {
            draw(
                &mut builder,
                tree,
                node_id,
                rect,
                decoration.as_ref(),
                resolver,
                rasteriser,
                atlas,
                options,
            )?;
            record_losses(
                &mut builder,
                &decoration_losses,
                node.source(),
                rect,
                decoration.is_none(),
            )?;
        }

        stack.push(Step::Leave { pops });
        if content_loss.is_some_and(SceneLossKind::replaces_content)
            || decoration_losses.iter().any(|kind| kind.replaces_content())
        {
            continue;
        }
        let enclosing = Enclosing {
            transform: absolute,
            clip: node.clip().or(enclosing.clip),
        };
        let children: Vec<FragmentId> = tree.children(node_id).collect();
        for child in children.into_iter().rev() {
            stack.push(Step::Enter {
                node: child,
                enclosing,
            });
        }
    }

    // Every layout loss is carried; the ones over missing content are drawn last, each in its element's own space.
    for loss in layout_losses {
        let drawn = loss
            .area
            .filter(|_| loss.kind.draws_placeholder())
            .map(|area| (area, SceneRect::from_layout(area.rect, scale)))
            .filter(|(_, rect)| !rect.is_empty());
        let Some((area, rect)) = drawn else {
            builder.add_loss(LossCategory::Layout(loss.kind), &loss.source, None)?;
            continue;
        };
        let mut pops = 0_usize;
        let transform = tree.transform(area.transform);
        if !transform.is_identity() {
            let index = builder.add_transform(SceneTransform::from_layout(transform, scale))?;
            builder.push(Command::PushTransform(index))?;
            pops += 1;
        }
        if let Some(clip) = area.clip.and_then(|clip| tree.clip(clip)) {
            let index = builder.add_clip(Clip::rectangle(SceneRect::from_layout(clip, scale)))?;
            builder.push(Command::PushClip(index))?;
            pops += 1;
        }
        builder.add_loss(LossCategory::Layout(loss.kind), &loss.source, Some(rect))?;
        for _ in 0..pops {
            builder.push(Command::Pop)?;
        }
    }

    builder.finish()
}

// The rectangle a loss of `kind` draws its placeholder over, or `None` for an approximation or an element with no area.
fn placeholder_over(kind: SceneLossKind, rect: SceneRect) -> Option<SceneRect> {
    (kind.draws_placeholder() && !rect.is_empty()).then_some(rect)
}

/// What is already installed around the node about to be entered.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Enclosing {
    transform: Transform,
    clip: Option<mjx_layout::ClipId>,
}

impl Enclosing {
    /// Page space, unclipped — what a root fragment is drawn in.
    const PAGE: Self = Self {
        transform: Transform::IDENTITY,
        clip: None,
    };
}

/// One item of the walk.
#[derive(Clone, Copy, Debug)]
enum Step {
    Enter {
        node: FragmentId,
        enclosing: Enclosing,
    },
    Leave {
        pops: usize,
    },
}

/// The map that, composed onto `enclosing`, gives `absolute`.
///
/// A display list's `PushTransform` composes rather than replaces, so a node whose absolute map is
/// `A` inside an enclosing map `E` has to push `A · E⁻¹`. When `E` has no inverse it has collapsed
/// the plane onto a line and nothing inside it is visible whatever is pushed, so the node's own map
/// is installed: an honest choice for a subtree that draws nothing either way, and one that never
/// panics.
fn relative_transform(enclosing: Transform, absolute: Transform) -> Transform {
    if enclosing.is_identity() {
        return absolute;
    }
    match enclosing.inverse() {
        Some(inverse) => absolute.then(inverse),
        None => absolute,
    }
}

/// The decoration a fragment carries, if it carries one at all.
fn decoration_of(fragment: &Fragment, resolver: &dyn ResourceResolver) -> Resolved<Decoration> {
    let reference = match fragment {
        Fragment::Box(box_fragment) => box_fragment.decoration,
        Fragment::Shape(shape) => shape.decoration,
        Fragment::Line(_) | Fragment::GlyphRun(_) | Fragment::Image(_) | Fragment::Table(_) => None,
    };
    let Some(reference) = reference else {
        return Resolved::NothingToDraw;
    };
    match resolver.decoration(reference) {
        Resolved::Answered(decoration) if decoration.is_invisible() => Resolved::NothingToDraw,
        answer => answer,
    }
}

/// Emit whatever one node draws for itself. Its children are the walk's business, not this one's.
#[allow(
    clippy::too_many_arguments,
    reason = "one call site, and every argument is a \
                                              different kind of thing the draw genuinely needs"
)]
fn draw(
    builder: &mut SceneBuilder,
    tree: &FragmentTree,
    node_id: FragmentId,
    rect: SceneRect,
    decoration: Option<&Decoration>,
    resolver: &dyn ResourceResolver,
    rasteriser: &mut GlyphRasteriser,
    atlas: &mut GlyphAtlas,
    options: &SceneOptions,
) -> Result<(), SceneError> {
    let Some(node) = tree.node(node_id) else {
        return Ok(());
    };
    match node.fragment() {
        // A line and a table draw nothing of their own: a line's ink is its glyph runs and a
        // table's is its cells, and both are children. They are fragments because everything that
        // *acts* on them acts on all of them at once, which is a matter for the interaction layer.
        Fragment::Line(_) | Fragment::Table(_) => Ok(()),
        Fragment::Box(_) => {
            let Some(decoration) = decoration else {
                return Ok(());
            };
            let geometry = builder.add_geometry(&Geometry::Rectangle(rect))?;
            paint_geometry(builder, geometry, decoration)
        }
        Fragment::Shape(shape) => {
            let Some(decoration) = decoration else {
                return Ok(());
            };
            let geometry = builder.add_geometry(&unresolved_geometry(shape.geometry, rect))?;
            paint_geometry(builder, geometry, decoration)
        }
        Fragment::Image(picture) => {
            let mut image = match resolver.image(picture.image) {
                Resolved::Answered(image) => image,
                Resolved::NothingToDraw => return Ok(()),
                Resolved::Unanswerable(kind) => {
                    return builder.add_loss(
                        LossCategory::Scene(kind),
                        node.source(),
                        placeholder_over(kind, rect),
                    )
                }
                Resolved::Partial(image, lost) => {
                    record_losses(builder, &lost, node.source(), rect, false)?;
                    image
                }
            };
            // A fragment's crop is the box model's answer and outranks the resolver's, because it
            // is what the *layout* was computed against.
            if let Some(crop) = picture.crop {
                image.crop = SceneRect::new(
                    crop.left as f32,
                    crop.top as f32,
                    crop.right as f32,
                    crop.bottom as f32,
                );
            }
            let index = builder.add_image(image)?;
            builder.push(Command::DrawImage {
                image: index,
                destination: rect,
            })
        }
        Fragment::GlyphRun(run) => {
            let placement = place_run(
                &run.run,
                run.face,
                options.device_scale,
                options.hinting,
                (0.0, 0.0),
            );
            let prepared = atlas.prepare_run(rasteriser, &placement)?;
            let mut glyphs = Vec::with_capacity(prepared.len());
            for glyph in prepared.glyphs() {
                let image = match &glyph.image {
                    PreparedImage::Atlas(entry) => GlyphImage::Atlas(AtlasPlacement {
                        page: entry.page.as_u32(),
                        format: entry.format,
                        x: entry.x,
                        y: entry.y,
                        width: entry.width,
                        height: entry.height,
                        offset_from_origin_x: entry.offset_from_origin_x,
                        offset_from_origin_y: entry.offset_from_origin_y,
                    }),
                    PreparedImage::Outline(outline) => {
                        let geometry = builder.add_geometry(&Geometry::path(
                            outline_to_path(outline),
                            FillRule::NonZero,
                        ))?;
                        GlyphImage::Outline(geometry)
                    }
                    PreparedImage::Blank => GlyphImage::Blank,
                };
                glyphs.push(SceneGlyph {
                    x: glyph.placed.x,
                    y: glyph.placed.y,
                    cluster: glyph.placed.cluster,
                    glyph: glyph.placed.glyph().0,
                    subpixel: glyph.placed.key.subpixel.index(),
                    image,
                });
            }
            let scene_run = SceneGlyphRun {
                face: run.face.as_u32(),
                bucket_steps: prepared.bucket().steps(),
                residual_scale: prepared.residual_scale(),
                origin: ScenePoint::new(
                    pixels_from_emu(run.origin.x, options.device_scale),
                    pixels_from_emu(run.origin.y, options.device_scale),
                ),
                direction: run.direction,
                hinting: options.hinting,
                level: run.level.0,
                glyphs,
            };
            let index = builder.add_glyph_run(&scene_run)?;
            let paint = text_paint(builder, resolver, node.source(), rect)?;
            builder.push(Command::DrawGlyphs { run: index, paint })
        }
    }
}

// Counts each part the resolver could not answer; when nothing was drawn, the first missing part places the element's one placeholder.
fn record_losses(
    builder: &mut SceneBuilder,
    losses: &[SceneLossKind],
    source: &SourceRef,
    rect: SceneRect,
    nothing_drawn: bool,
) -> Result<(), SceneError> {
    let mut placed = !nothing_drawn;
    for kind in losses {
        let over = if placed {
            None
        } else {
            placeholder_over(*kind, rect)
        };
        placed |= over.is_some();
        builder.add_loss(LossCategory::Scene(*kind), source, over)?;
    }
    Ok(())
}

/// A shape's outline, as the handle a box model issued and the box it is resolved against.
fn unresolved_geometry(geometry: GeometryRef, bounds: SceneRect) -> Geometry {
    Geometry::Unresolved {
        outline: geometry.number(),
        bounds,
    }
}

/// Fill then stroke, which is the order every one of these formats paints in: an outline sits on
/// top of the fill it surrounds.
fn paint_geometry(
    builder: &mut SceneBuilder,
    geometry: ResourceIndex,
    decoration: &Decoration,
) -> Result<(), SceneError> {
    if let Some(paint) = builder.add_fill_style(&decoration.fill)? {
        builder.push(Command::FillPath { geometry, paint })?;
    }
    if let Some(style) = &decoration.stroke {
        if let Some(stroke) = builder.add_stroke_style(style)? {
            builder.push(Command::StrokePath { geometry, stroke })?;
        }
    }
    Ok(())
}

/// What a run of glyphs is drawn in: whatever the resolver says about its address, or
/// [`DEFAULT_TEXT_COLOR`].
fn text_paint(
    builder: &mut SceneBuilder,
    resolver: &dyn ResourceResolver,
    source: &SourceRef,
    rect: SceneRect,
) -> Result<ResourceIndex, SceneError> {
    let fill = match resolver.text_decoration(source) {
        Resolved::Answered(decoration) if !decoration.fill.is_none() => decoration.fill,
        Resolved::Answered(_) | Resolved::NothingToDraw => FillStyle::Solid(DEFAULT_TEXT_COLOR),
        Resolved::Unanswerable(kind) => {
            builder.add_loss(
                LossCategory::Scene(kind),
                source,
                placeholder_over(kind, rect),
            )?;
            FillStyle::Solid(DEFAULT_TEXT_COLOR)
        }
        Resolved::Partial(decoration, lost) => {
            record_losses(builder, &lost, source, rect, false)?;
            if decoration.fill.is_none() {
                FillStyle::Solid(DEFAULT_TEXT_COLOR)
            } else {
                decoration.fill
            }
        }
    };
    // The filter above removed the one fill that interns to no paint, so the `ok_or` cannot fire.
    // It is written rather than assumed because there is no `expect` in this crate — and because it
    // is what makes deleting the fallback a **red**. It used to be a green mutation: a second
    // fallback sat underneath this one and absorbed the loss of the first, so removing the default
    // text colour changed nothing and no test noticed.
    builder
        .add_fill_style(&fill)?
        .ok_or(SceneError::UnpaintableText)
}

/// A glyph's outline, in the glyph's own coordinates.
///
/// **Not translated to the run's origin**, deliberately: an [`crate::glyphs::AtlasPlacement`] is
/// glyph-local too, so a painter positions a glyph the same way whichever route it took out of the
/// rasteriser — which is the property `mjx-text`'s [`mjx_text::GlyphRoute`] exists to make true.
/// Keeping it glyph-local is also what lets the same letter at the same size, drawn a thousand
/// times on a page, intern down to one geometry.
fn outline_to_path(outline: &mjx_text::GlyphOutline) -> Vec<PathCommand> {
    use mjx_text::OutlineCommand;
    outline
        .commands()
        .iter()
        .map(|command| match *command {
            OutlineCommand::MoveTo(point) => PathCommand::MoveTo(ScenePoint::new(point.x, point.y)),
            OutlineCommand::LineTo(point) => PathCommand::LineTo(ScenePoint::new(point.x, point.y)),
            OutlineCommand::QuadraticTo(control, end) => PathCommand::QuadraticTo {
                control: ScenePoint::new(control.x, control.y),
                end: ScenePoint::new(end.x, end.y),
            },
            OutlineCommand::CubicTo(first, second, end) => PathCommand::CubicTo {
                first_control: ScenePoint::new(first.x, first.y),
                second_control: ScenePoint::new(second.x, second.y),
                end: ScenePoint::new(end.x, end.y),
            },
            OutlineCommand::Close => PathCommand::Close,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::Image;
    use mjx_layout::{
        BoxFragment, FragmentTreeBuilder, ImageFragment, LayoutLossKind, LayoutRect, LossArea,
        PageIndex, PartId, ShapeFragment, SourcePath, TransformId,
    };
    use mjx_ooxml_core::measure::Emu;

    // A resolver whose every answer the test chooses.
    struct Answers {
        decoration: Resolved<Decoration>,
        image: Resolved<Image>,
        content: Option<(Vec<u32>, SceneLossKind)>,
    }

    impl ResourceResolver for Answers {
        fn decoration(&self, _reference: DecorationRef) -> Resolved<Decoration> {
            self.decoration.clone()
        }

        fn text_decoration(&self, _source: &SourceRef) -> Resolved<Decoration> {
            Resolved::NothingToDraw
        }

        fn image(&self, _reference: ImageRef) -> Resolved<Image> {
            self.image.clone()
        }

        fn unanswerable_content(&self, source: &SourceRef) -> Option<SceneLossKind> {
            let (path, kind) = self.content.as_ref()?;
            (source.path().segments() == path.as_slice()).then_some(*kind)
        }
    }

    fn at(segments: &[u32]) -> SourceRef {
        SourceRef::node(PartId::PRIMARY, SourcePath::new(segments))
    }

    // A rectangle of 72 by 36 points, which is 96 by 48 unzoomed pixels.
    fn a_box() -> LayoutRect {
        LayoutRect::from_edges(
            Emu::ZERO,
            Emu::ZERO,
            Emu::from_points(72.0),
            Emu::from_points(36.0),
        )
    }

    fn solid() -> Decoration {
        Decoration::filled(FillStyle::Solid(DEFAULT_TEXT_COLOR))
    }

    fn scene(tree: &FragmentTree, resolver: &Answers) -> DisplayList {
        build_scene(
            tree,
            resolver,
            &mut GlyphRasteriser::new(),
            &mut GlyphAtlas::new(),
            &SceneOptions::new(LayoutSize::new(
                Emu::from_points(200.0),
                Emu::from_points(200.0),
            )),
        )
        .expect("the scene builds")
    }

    fn fills(list: &DisplayList) -> usize {
        list.commands()
            .filter(|command| matches!(command, Command::FillPath { .. }))
            .count()
    }

    fn one_shape() -> FragmentTree {
        let mut builder = FragmentTreeBuilder::new();
        builder.push_simple(
            None,
            at(&[0, 0]),
            a_box(),
            Fragment::Shape(ShapeFragment {
                geometry: GeometryRef::new(1),
                decoration: Some(DecorationRef::new(1)),
            }),
        );
        builder.finish()
    }

    #[test]
    fn a_shape_whose_decoration_is_unanswerable_is_counted_under_a_placeholder() {
        let list = scene(
            &one_shape(),
            &Answers {
                decoration: Resolved::Unanswerable(SceneLossKind::ColourNotResolved),
                image: Resolved::NothingToDraw,
                content: None,
            },
        );
        assert_eq!(list.losses().count(SceneLossKind::ColourNotResolved), 1);
        let placeholders = list.placeholders();
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].rect, SceneRect::new(0.0, 0.0, 96.0, 48.0));
        assert_eq!(placeholders[0].source, at(&[0, 0]));
        assert_eq!(fills(&list), 0);
    }

    #[test]
    fn a_shape_that_paints_nothing_is_no_loss() {
        let list = scene(
            &one_shape(),
            &Answers {
                decoration: Resolved::NothingToDraw,
                image: Resolved::NothingToDraw,
                content: None,
            },
        );
        assert!(list.losses().is_empty());
        assert!(list.placeholders().is_empty());
    }

    #[test]
    fn unanswerable_content_is_one_placeholder_and_nothing_inside_it_is_drawn() {
        let mut builder = FragmentTreeBuilder::new();
        let frame = builder.push_simple(
            None,
            at(&[0, 0]),
            a_box(),
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        );
        for bar in 0..3 {
            builder.push_simple(
                frame,
                at(&[0, 0, 3, bar]),
                a_box(),
                Fragment::Shape(ShapeFragment {
                    geometry: GeometryRef::new(2),
                    decoration: Some(DecorationRef::new(2)),
                }),
            );
        }
        let list = scene(
            &builder.finish(),
            &Answers {
                decoration: Resolved::Answered(solid()),
                image: Resolved::NothingToDraw,
                content: Some((vec![0, 0], SceneLossKind::ChartNotResolved)),
            },
        );
        assert_eq!(list.losses().len(), 1);
        assert_eq!(list.losses().count(SceneLossKind::ChartNotResolved), 1);
        assert_eq!(list.placeholders().len(), 1);
        assert_eq!(fills(&list), 0, "the bars inside the frame are not drawn");
        assert_eq!(
            list.placeholder_at(10.0, 10.0).map(|found| found.category),
            Some(LossCategory::Scene(SceneLossKind::ChartNotResolved))
        );
        assert_eq!(list.placeholder_at(150.0, 150.0), None);
    }

    #[test]
    fn a_picture_the_resolver_cannot_supply_is_a_placeholder() {
        let mut builder = FragmentTreeBuilder::new();
        builder.push_simple(
            None,
            at(&[0, 4]),
            a_box(),
            Fragment::Image(ImageFragment {
                image: ImageRef::new(0),
                crop: None,
            }),
        );
        let list = scene(
            &builder.finish(),
            &Answers {
                decoration: Resolved::NothingToDraw,
                image: Resolved::Unanswerable(SceneLossKind::FillImageNotSupplied),
                content: None,
            },
        );
        assert_eq!(list.losses().count(SceneLossKind::FillImageNotSupplied), 1);
        assert_eq!(list.placeholders().len(), 1);
    }

    #[test]
    fn a_layout_loss_with_an_area_is_drawn_in_its_own_space_and_an_approximation_is_not() {
        let mut builder = FragmentTreeBuilder::new();
        let doubled = builder.transform(Transform::scale(2.0, 2.0));
        let mut losses = LayoutLosses::new();
        losses.record_at(
            at(&[1, 1]),
            LayoutLossKind::DroppedByReader,
            LossArea {
                rect: a_box(),
                transform: doubled,
                clip: None,
            },
        );
        losses.record_at(
            at(&[1, 2]),
            LayoutLossKind::ValueApproximated,
            LossArea {
                rect: a_box(),
                transform: TransformId::IDENTITY,
                clip: None,
            },
        );
        let page = PageFragments::new(PageIndex::FIRST, builder.finish(), None).with_losses(losses);
        let list = build_page(
            &page,
            &Answers {
                decoration: Resolved::NothingToDraw,
                image: Resolved::NothingToDraw,
                content: None,
            },
            &mut GlyphRasteriser::new(),
            &mut GlyphAtlas::new(),
            &SceneOptions::new(LayoutSize::new(
                Emu::from_points(200.0),
                Emu::from_points(200.0),
            )),
        )
        .expect("the page builds");
        let placeholders = list.placeholders();
        assert_eq!(placeholders.len(), 1);
        assert_eq!(
            placeholders[0].category,
            LossCategory::Layout(LayoutLossKind::DroppedByReader)
        );
        assert_eq!(placeholders[0].transform.scale_x, 2.0);
        assert!(list.placeholder_at(150.0, 80.0).is_some());
        assert_eq!(
            (
                list.losses().count(LayoutLossKind::DroppedByReader),
                list.losses().count(LayoutLossKind::ValueApproximated),
                list.losses().len()
            ),
            (1, 1, 2),
            "both layout losses are carried, and only the dropped content draws"
        );
    }
}
