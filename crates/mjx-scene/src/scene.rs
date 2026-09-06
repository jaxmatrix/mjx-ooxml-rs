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
    DecorationRef, Fragment, FragmentId, FragmentTree, GeometryRef, ImageRef, LayoutSize,
    SourceRef, Transform,
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
/// decoration returns `None` from every method and produces a scene of plain text, which is exactly
/// what the foreign box model in this crate's tests does.
pub trait ResourceResolver {
    /// What paints the box or shape that carries `reference`.
    fn decoration(&self, reference: DecorationRef) -> Option<Decoration>;

    /// What paints the text at `source`.
    ///
    /// **Addressed by [`SourceRef`], not by a handle**, because [`mjx_layout::GlyphRunFragment`]
    /// carries no decoration of its own — see this crate's hand-off notes. A `SourceRef` is the
    /// fragment vocabulary's own link back to the document, so answering from it reaches around
    /// nothing; a run's fill, outline and shadow are `a:rPr`'s in DrawingML and `w:rPr`'s in
    /// WordprocessingML, and the resolver is the layer that can read either.
    fn text_decoration(&self, source: &SourceRef) -> Option<Decoration>;

    /// Which picture `reference` names, and how it is drawn.
    fn image(&self, reference: ImageRef) -> Option<Image>;
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

/// Build the display list for one page.
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
        let decoration = decoration_of(node.fragment(), resolver);
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

        stack.push(Step::Leave { pops });
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

    builder.finish()
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
fn decoration_of(fragment: &Fragment, resolver: &dyn ResourceResolver) -> Option<Decoration> {
    let reference = match fragment {
        Fragment::Box(box_fragment) => box_fragment.decoration?,
        Fragment::Shape(shape) => shape.decoration?,
        Fragment::Line(_) | Fragment::GlyphRun(_) | Fragment::Image(_) | Fragment::Table(_) => {
            return None
        }
    };
    resolver
        .decoration(reference)
        .filter(|decoration| !decoration.is_invisible())
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
            let Some(mut image) = resolver.image(picture.image) else {
                return Ok(());
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
            let paint = text_paint(builder, resolver, node.source())?;
            builder.push(Command::DrawGlyphs { run: index, paint })
        }
    }
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
) -> Result<ResourceIndex, SceneError> {
    let fill = resolver
        .text_decoration(source)
        .map(|decoration| decoration.fill)
        .filter(|fill| !fill.is_none())
        .unwrap_or(FillStyle::Solid(DEFAULT_TEXT_COLOR));
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
