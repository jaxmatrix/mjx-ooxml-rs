//! The frame plan: a display list, lowered into layers and draw operations, **with no graphics API
//! anywhere in it**.
//!
//! # Why the lowering is separate from the drawing
//!
//! Two reasons, and the second is the one that decided it.
//!
//! **Render passes do not nest.** An opacity group and an effect are both *"draw this subtree
//! somewhere else, then bring it back"*, and on every graphics API that means ending the current
//! pass, opening one on another target, and opening a third to composite. A painter that tried to
//! do that while walking the command stream would be holding an open pass, which borrows its
//! encoder, while opening another — which is not a thing that can be written. Lowering first, and
//! then rendering layer by layer, turns a nesting problem into a list.
//!
//! **And a plan can be tested without a GPU.** Every question that is really about *understanding*
//! a display list — did the walk see all nine commands, does `PushOpacity(1.0)` cost a layer, does a
//! clip inside an effect reach the effect's own target, is a run of a thousand glyphs one draw call
//! or a thousand — is answered here, by a test that runs on a machine with no graphics stack at all.
//! What is left for the GPU suite is what only a GPU can answer: the pixels. Given that whether
//! continuous integration has a GPU is an open question, a design that put those two classes of
//! question in one place would have made the first class unaskable there.
//!
//! # What a layer is
//!
//! Layer zero is the frame's own target. Every other layer is an offscreen texture, and it exists
//! for exactly one of two reasons: a [`mjx_scene::Command::PushOpacity`] with a factor that is not
//! `1.0`, or a [`mjx_scene::Command::PushEffect`]. Its parent carries a [`DrawOp::Composite`] at the
//! position the group occupied, so paint order survives.
//!
//! **`PushOpacity(1.0)` opens no layer**, and that is not an optimisation, it is the difference
//! between a page that costs one target and a page that costs one per group: a scene builder emits
//! the command whenever a fragment carries an opacity, and the overwhelmingly common value is the
//! one that means "unchanged". `tests/the_identity_values_are_not_the_only_values.rs` holds both
//! halves — that `1.0` costs nothing, and that `0.5` costs a layer and changes the pixels.
//!
//! # Clips are replayed into a child layer
//!
//! A clip that was pushed outside a group is re-emitted inside it. Clipping only at composite time
//! would be correct for a plain opacity group and **wrong for every effect**: a blur would drag the
//! clipped-away part of the subtree back across the clip boundary, because it was drawn into the
//! offscreen target before the clip was applied. Re-emitting is cheap — the clip's mesh is already
//! tessellated and interned — and it makes the two cases the same case.

use std::sync::Arc;

use mjx_scene::{
    tessellate_scene, BitmapFormat, Clip, Color, Command, DisplayList, Effect, EffectKind,
    FillRule, Geometry, GeometryProvider, GlyphImage, Gradient, GradientKind, Image, Mesh,
    MeshRole, Paint, PathShade, PatternPreset, Provenance, ResourceIndex, SceneRect,
    SceneTransform, TessellationOptions, Tessellator,
};

use crate::error::PaintError;
use crate::gradient::GradientRamp;
use crate::painter::DrawReport;

/// Compose two transforms: `outer` applied to the result of `inner`.
///
/// A display list's `PushTransform` is **relative** — `mjx-scene` says so, and converts a fragment's
/// absolute transform into one relative to whatever is already installed — so a painter multiplies
/// rather than replaces, and a group inside a group works.
#[must_use]
pub fn compose(outer: SceneTransform, inner: SceneTransform) -> SceneTransform {
    SceneTransform {
        scale_x: outer.scale_x * inner.scale_x + outer.shear_x * inner.shear_y,
        shear_y: outer.shear_y * inner.scale_x + outer.scale_y * inner.shear_y,
        shear_x: outer.scale_x * inner.shear_x + outer.shear_x * inner.scale_y,
        scale_y: outer.shear_y * inner.shear_x + outer.scale_y * inner.scale_y,
        translate_x: outer.scale_x * inner.translate_x
            + outer.shear_x * inner.translate_y
            + outer.translate_x,
        translate_y: outer.shear_y * inner.translate_x
            + outer.scale_y * inner.translate_y
            + outer.translate_y,
    }
}

/// A point put through a transform.
#[must_use]
pub fn apply(transform: SceneTransform, x: f32, y: f32) -> (f32, f32) {
    (
        transform.scale_x * x + transform.shear_x * y + transform.translate_x,
        transform.shear_y * x + transform.scale_y * y + transform.translate_y,
    )
}

/// How a gradient's ramp is laid over the space a shape's vertices are in.
///
/// Resolved on the processor so the fragment shader is one dot product or one length, with no
/// branch on `angle_is_scaled`, no rectangle arithmetic and no stop list.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GradientMapping {
    /// Where the ramp starts, in the shape's own coordinates.
    pub origin: [f32; 2],
    /// For a linear ramp, the vector along which `t` runs from zero to one — already divided by its
    /// own squared length, so the shader's `t` is a plain dot product.
    pub axis: [f32; 2],
    /// For a radial or path ramp, the radii `t` is measured against.
    pub radii: [f32; 2],
    /// Whether the ramp is radial.
    pub radial: bool,
}

impl GradientMapping {
    /// Where `t` is at a point, exactly as the shader computes it.
    ///
    /// Public because it is what `tests/the_gradient_ramp_is_a_ramp.rs` checks the mapping with:
    /// a shader cannot be unit-tested and this arithmetic can, and the two are the same arithmetic
    /// written twice on purpose — the WGSL is three lines and this is its executable specification.
    #[must_use]
    pub fn at(&self, x: f32, y: f32) -> f32 {
        let dx = x - self.origin[0];
        let dy = y - self.origin[1];
        if self.radial {
            let rx = if self.radii[0] != 0.0 {
                dx / self.radii[0]
            } else {
                0.0
            };
            let ry = if self.radii[1] != 0.0 {
                dy / self.radii[1]
            } else {
                0.0
            };
            (rx * rx + ry * ry).sqrt().clamp(0.0, 1.0)
        } else {
            (dx * self.axis[0] + dy * self.axis[1]).clamp(0.0, 1.0)
        }
    }

    /// The mapping a gradient makes over `bounds`.
    #[must_use]
    pub fn resolve(gradient: &Gradient, bounds: SceneRect) -> Self {
        let width = (bounds.right - bounds.left).abs().max(f32::EPSILON);
        let height = (bounds.bottom - bounds.top).abs().max(f32::EPSILON);
        match gradient.kind {
            GradientKind::Linear => {
                let angle = if gradient.angle.is_finite() {
                    gradient.angle
                } else {
                    0.0
                };
                // `a:lin@scaled` measures the angle in the shape's own aspect ratio rather than in a
                // square, which is visible on every shape that is not square — and most are. The
                // difference is one multiplication, and skipping it is a diagonal gradient that runs
                // at the wrong diagonal.
                let (mut dx, mut dy) = (angle.cos(), angle.sin());
                if gradient.angle_is_scaled {
                    dx *= height;
                    dy *= width;
                    let length = (dx * dx + dy * dy).sqrt();
                    if length > 0.0 {
                        dx /= length;
                        dy /= length;
                    }
                }
                // The ramp spans the shape's box along that direction: start at whichever corner is
                // furthest back along it, and run to the projection of the far corner.
                let half_span = (dx.abs() * width + dy.abs() * height) / 2.0;
                let centre_x = (bounds.left + bounds.right) / 2.0;
                let centre_y = (bounds.top + bounds.bottom) / 2.0;
                let origin = [centre_x - dx * half_span, centre_y - dy * half_span];
                let span = (half_span * 2.0).max(f32::EPSILON);
                Self {
                    origin,
                    axis: [dx / span, dy / span],
                    radii: [0.0, 0.0],
                    radial: false,
                }
            }
            GradientKind::Radial | GradientKind::Path => {
                // `a:fillToRect` is in fractions of the shape's box, and names the rectangle the
                // ramp converges *on* — so its centre is where `t` is zero.
                let focus_x =
                    bounds.left + width * (gradient.focus.left + gradient.focus.right) / 2.0;
                let focus_y =
                    bounds.top + height * (gradient.focus.top + gradient.focus.bottom) / 2.0;
                let radii = match gradient.path_shade {
                    // A circle inscribed in the box: one radius, the smaller half-extent.
                    PathShade::Circle => {
                        let radius = (width.min(height) / 2.0).max(f32::EPSILON);
                        [radius, radius]
                    }
                    // The box itself, and the shape's own outline treated as its box — following a
                    // real outline inward is a distance field, which is a compute problem and
                    // therefore not available on the baseline backend.
                    PathShade::Rectangle | PathShade::Shape => [
                        (width / 2.0).max(f32::EPSILON),
                        (height / 2.0).max(f32::EPSILON),
                    ],
                };
                Self {
                    origin: [focus_x, focus_y],
                    axis: [0.0, 0.0],
                    radii,
                    radial: true,
                }
            }
        }
    }
}

/// What a draw is filled with, resolved out of the display list's tables.
#[derive(Clone, PartialEq, Debug)]
pub enum PaintProgram {
    /// One colour everywhere.
    Solid(Color),
    /// A ramp, and where it lies over the shape.
    Gradient {
        /// The resolved stops.
        ramp: Arc<GradientRamp>,
        /// Where the ramp lies.
        mapping: GradientMapping,
    },
    /// A two-colour preset hatch.
    Pattern {
        /// Which of the fifty-four.
        preset: PatternPreset,
        /// The colour its marks are drawn in.
        foreground: Color,
        /// The colour behind them.
        background: Color,
    },
    /// A picture.
    Picture {
        /// Which one, as the caller numbers them.
        handle: u64,
        /// How it fills its area, cropped, tiled and adjusted.
        image: Image,
        /// The rectangle it is laid into.
        destination: SceneRect,
    },
    /// A run of glyphs from one atlas page.
    Glyphs {
        /// Which page.
        page: u32,
        /// What its bytes mean.
        format: BitmapFormat,
        /// What colour the glyphs are.
        color: Color,
    },
}

/// One glyph's quad, in the run's own space before the run transform.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GlyphQuad {
    /// Left edge, in the run's space.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// How wide.
    pub width: f32,
    /// How tall.
    pub height: f32,
    /// Left edge in the atlas page, in **texels** — normalised by the shader against the page's own
    /// dimensions, because a display list records an atlas rectangle and never the page's size.
    pub u: f32,
    /// Top edge in the atlas page, in texels.
    pub v: f32,
    /// How many texels wide.
    pub texel_width: f32,
    /// How many texels tall.
    pub texel_height: f32,
}

/// One thing to draw.
#[derive(Clone, PartialEq, Debug)]
pub enum DrawOp {
    /// A tessellated path.
    Mesh {
        /// Its triangles, in the display list's own device-pixel space.
        mesh: Arc<Mesh>,
        /// What to put the triangles in.
        transform: SceneTransform,
        /// What to fill them with.
        paint: PaintProgram,
        /// Whether these are a fill or an outline. Carried for the report and for a painter that
        /// wants to order them; the pipeline is the same either way.
        role: MeshRole,
        /// Whether the outline behind these triangles is the document's own shape or a stand-in.
        ///
        /// **Every preset shape in the platform resolves to a stand-in today.** A painter that could
        /// not see this could not draw one in a warning colour and — the reason it matters — a
        /// golden-image gate could not refuse to call a page of framed rounded rectangles a fidelity
        /// render. It reaches a caller through [`DrawReport::placeholders`], because a field the
        /// painter reads and nobody can act on is the same defect one layer up.
        provenance: Provenance,
    },
    /// A batch of glyph quads sharing one atlas page and one colour.
    Glyphs {
        /// The quads.
        quads: Vec<GlyphQuad>,
        /// Where the run sits, and how much it is scaled by.
        transform: SceneTransform,
        /// Which page, what format, what colour.
        paint: PaintProgram,
    },
    /// A picture in a rectangle.
    Picture {
        /// Where it goes.
        destination: SceneRect,
        /// Under what transform.
        transform: SceneTransform,
        /// Which picture and how it fills the rectangle.
        paint: PaintProgram,
    },
    /// Narrow the clip to a shape.
    PushClip {
        /// The clip's triangles.
        mesh: Arc<Mesh>,
        /// Under what transform.
        transform: SceneTransform,
    },
    /// Widen it back. Carries the same triangles, because a stencil clip is undone by drawing the
    /// same shape again with the opposite operation.
    PopClip {
        /// The clip's triangles.
        mesh: Arc<Mesh>,
        /// Under what transform.
        transform: SceneTransform,
    },
    /// Bring a finished child layer back into this one.
    Composite {
        /// Which layer.
        layer: usize,
    },
}

/// One node of an effect DAG, with its colour already resolved.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct EffectNode {
    /// The node as the list stored it.
    pub effect: Effect,
    /// The colour the node draws in, resolved from its paint index. A gradient-filled glow is
    /// reduced to the gradient's first stop: an effect pass samples one colour per texel of the
    /// *input*, and there is nowhere in that pass for a second coordinate space to come from.
    pub color: Color,
    /// Which earlier node of the same DAG this one consumes, or `None` for the subtree.
    pub input: Option<usize>,
}

/// Why a layer exists.
#[derive(Clone, PartialEq, Debug)]
pub enum LayerKind {
    /// The frame's own target.
    Root,
    /// An opacity group. The factor is strictly below `1.0`: the identity opens no layer.
    Opacity(f32),
    /// An effect group, with its DAG in topological order. The last node is the root.
    Effect(Vec<EffectNode>),
}

/// One render target's worth of work.
#[derive(Clone, PartialEq, Debug)]
pub struct Layer {
    /// Why it exists.
    pub kind: LayerKind,
    /// What to draw into it, in paint order.
    pub ops: Vec<DrawOp>,
}

/// A display list, lowered.
#[derive(Clone, PartialEq, Debug)]
pub struct FramePlan {
    layers: Vec<Layer>,
    report: DrawReport,
}

impl FramePlan {
    /// The layers. Layer zero is the frame's target; every other is an offscreen one, and every one
    /// of those is named by exactly one [`DrawOp::Composite`] in a layer of a lower index.
    #[must_use]
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// One layer.
    #[must_use]
    pub fn layer(&self, index: usize) -> Option<&Layer> {
        self.layers.get(index)
    }

    /// What the lowering found: commands walked, draws produced, layers opened.
    #[must_use]
    pub const fn report(&self) -> DrawReport {
        self.report
    }

    /// How many draw operations there are in total, across every layer.
    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.layers.iter().map(|layer| layer.ops.len()).sum()
    }
}

/// The stack a walk keeps, one entry per `Push`.
enum Open {
    /// A transform, and what was installed before it.
    Transform(SceneTransform),
    /// A clip, and the triangles that undo it.
    Clip {
        mesh: Arc<Mesh>,
        transform: SceneTransform,
    },
    /// An opacity of exactly `1.0`. Nothing was opened, and the `Pop` has nothing to close — which
    /// is the whole point of tracking it separately rather than not pushing at all: the stream still
    /// contains a `Pop`, and a walk that ignored the push would close somebody else's group.
    IdentityOpacity,
    /// A layer, and the layer that was current before it.
    Layer { opened: usize, parent: usize },
}

/// Lower `list` into a plan.
///
/// # Errors
///
/// [`PaintError::MissingResource`] for an index the list's own tables do not hold,
/// [`PaintError::UnbalancedStack`] for a stream that pops more than it pushes, and whatever the
/// tessellator and the geometry provider fail with.
pub fn plan_frame(
    list: &DisplayList,
    provider: &dyn GeometryProvider,
    tessellator: &mut Tessellator,
) -> Result<FramePlan, PaintError> {
    // Hand-off 2 from R07: the whole page's triangles in one call. **The painter tessellates
    // nothing** — it does not mean it may not ask, it means it writes no tessellation code, and the
    // one place it asks for something this call does not cover is a clip's own outline, which is not
    // a command and so is not in the list this returns.
    let meshes = tessellate_scene(list, provider, tessellator)?;
    let mut mesh_cursor = 0usize;

    let options = TessellationOptions::for_bucket(mjx_scene::page_bucket(list.device_scale()));

    let mut layers = vec![Layer {
        kind: LayerKind::Root,
        ops: Vec::new(),
    }];
    let mut current = 0usize;
    let mut transform = SceneTransform::IDENTITY;
    let mut stack: Vec<Open> = Vec::new();
    let mut active_clips: Vec<(Arc<Mesh>, SceneTransform)> = Vec::new();
    let mut report = DrawReport::default();

    for (index, command) in list.commands().enumerate() {
        report.commands += 1;
        match command {
            Command::PushTransform(slot) => {
                let next = list.transform(slot).ok_or(PaintError::MissingResource {
                    table: "transform",
                    index: slot.index(),
                    command: index,
                })?;
                stack.push(Open::Transform(transform));
                transform = compose(transform, next);
            }
            Command::PushClip(slot) => {
                let clip = list.clip(slot).ok_or(PaintError::MissingResource {
                    table: "clip",
                    index: slot.index(),
                    command: index,
                })?;
                let mesh = clip_mesh(list, &clip, provider, tessellator, options, index)?;
                push_op(
                    &mut layers,
                    current,
                    DrawOp::PushClip {
                        mesh: Arc::clone(&mesh),
                        transform,
                    },
                );
                active_clips.push((Arc::clone(&mesh), transform));
                stack.push(Open::Clip { mesh, transform });
                report.clips += 1;
            }
            Command::PushOpacity(factor) => {
                // The identity. A scene builder emits this command for every fragment that carries
                // an opacity at all, and `1.0` is what most of them carry; opening a full-viewport
                // render target for each would be the single most expensive mistake in this file.
                if !factor.is_finite() || factor >= 1.0 {
                    stack.push(Open::IdentityOpacity);
                    continue;
                }
                let opened = open_layer(&mut layers, LayerKind::Opacity(factor.max(0.0)));
                stack.push(Open::Layer {
                    opened,
                    parent: current,
                });
                current = opened;
                report.layers += 1;
                replay_clips(&mut layers, current, &active_clips);
            }
            Command::PushEffect(slot) => {
                let dag = resolve_effects(list, slot, index)?;
                let opened = open_layer(&mut layers, LayerKind::Effect(dag));
                stack.push(Open::Layer {
                    opened,
                    parent: current,
                });
                current = opened;
                report.layers += 1;
                replay_clips(&mut layers, current, &active_clips);
            }
            Command::Pop => {
                let Some(open) = stack.pop() else {
                    return Err(PaintError::UnbalancedStack {
                        command: index,
                        detail: "a Pop with nothing pushed",
                    });
                };
                match open {
                    Open::Transform(previous) => transform = previous,
                    Open::Clip { mesh, transform } => {
                        active_clips.pop();
                        push_op(&mut layers, current, DrawOp::PopClip { mesh, transform });
                    }
                    Open::IdentityOpacity => {}
                    Open::Layer { opened, parent } => {
                        current = parent;
                        push_op(&mut layers, current, DrawOp::Composite { layer: opened });
                    }
                }
            }
            Command::FillPath { paint, .. } => {
                let Some((mesh, provenance)) =
                    take_mesh(&meshes, &mut mesh_cursor, index, MeshRole::Fill)
                else {
                    continue;
                };
                let paint = resolve_paint(list, paint, mesh.bounds(), index)?;
                report.triangles += mesh.triangle_count();
                report.draw_calls += 1;
                if provenance.is_placeholder() {
                    report.placeholders += 1;
                }
                push_op(
                    &mut layers,
                    current,
                    DrawOp::Mesh {
                        mesh,
                        transform,
                        paint,
                        role: MeshRole::Fill,
                        provenance,
                    },
                );
            }
            Command::StrokePath { stroke, .. } => {
                let Some((mesh, provenance)) =
                    take_mesh(&meshes, &mut mesh_cursor, index, MeshRole::Stroke)
                else {
                    continue;
                };
                let record = list.stroke(stroke).ok_or(PaintError::MissingResource {
                    table: "stroke",
                    index: stroke.index(),
                    command: index,
                })?;
                let paint = resolve_paint(list, record.paint, mesh.bounds(), index)?;
                report.triangles += mesh.triangle_count();
                report.draw_calls += 1;
                if provenance.is_placeholder() {
                    report.placeholders += 1;
                }
                push_op(
                    &mut layers,
                    current,
                    DrawOp::Mesh {
                        mesh,
                        transform,
                        paint,
                        role: MeshRole::Stroke,
                        provenance,
                    },
                );
            }
            Command::DrawGlyphs { run, paint } => {
                let record = list.glyph_run(run).ok_or(PaintError::MissingResource {
                    table: "glyph run",
                    index: run.index(),
                    command: index,
                })?;
                let colour = match resolve_paint(list, paint, SceneRect::UNIT, index)? {
                    PaintProgram::Solid(color) => color,
                    // Glyphs in a gradient, a hatch or a picture are a `a:textFill` this painter
                    // reduces to one colour rather than drawing wrongly: the alternative is a second
                    // pass that masks the fill by the run's coverage, which is R09's work and is
                    // recorded as such rather than half-built here.
                    other => representative_color(&other),
                };
                plan_glyph_run(
                    &record,
                    colour,
                    transform,
                    &mut layers,
                    current,
                    &mut report,
                    list,
                    provider,
                    tessellator,
                    index,
                )?;
            }
            Command::DrawImage { image, destination } => {
                let record = list.image(image).ok_or(PaintError::MissingResource {
                    table: "image",
                    index: image.index(),
                    command: index,
                })?;
                report.images += 1;
                report.draw_calls += 1;
                push_op(
                    &mut layers,
                    current,
                    DrawOp::Picture {
                        destination,
                        transform,
                        paint: PaintProgram::Picture {
                            handle: record.handle,
                            image: record,
                            destination,
                        },
                    },
                );
            }
        }
    }

    if let Some(remaining) = stack.pop() {
        let detail = match remaining {
            Open::Transform(_) => "a transform was never popped",
            Open::Clip { .. } => "a clip was never popped",
            Open::IdentityOpacity | Open::Layer { .. } => "a group was never popped",
        };
        return Err(PaintError::UnbalancedStack {
            command: report.commands,
            detail,
        });
    }

    Ok(FramePlan { layers, report })
}

/// Append an operation to a layer.
fn push_op(layers: &mut [Layer], index: usize, op: DrawOp) {
    if let Some(layer) = layers.get_mut(index) {
        layer.ops.push(op);
    }
}

/// Open a layer and answer its index.
fn open_layer(layers: &mut Vec<Layer>, kind: LayerKind) -> usize {
    layers.push(Layer {
        kind,
        ops: Vec::new(),
    });
    layers.len() - 1
}

/// Re-emit every clip that is open around a group, into the group's own target.
///
/// See the module documentation: clipping only when the group is composited is right for an opacity
/// and wrong for a blur, and one rule for both is better than two.
fn replay_clips(layers: &mut [Layer], into: usize, clips: &[(Arc<Mesh>, SceneTransform)]) {
    for (mesh, transform) in clips {
        push_op(
            layers,
            into,
            DrawOp::PushClip {
                mesh: Arc::clone(mesh),
                transform: *transform,
            },
        );
    }
}

/// The next mesh for `command`, if the tessellator produced one.
///
/// A linear cursor rather than a map: `tessellate_scene` answers in paint order, so the mesh for
/// command *n* is at or after the cursor and never before it. A command whose geometry the list did
/// not hold produces no mesh at all, which is why this can answer `None` without that being an
/// error — `tessellate_scene` skips it for the same reason and neither should fail the page.
fn take_mesh(
    meshes: &[mjx_scene::SceneMesh],
    cursor: &mut usize,
    command: usize,
    role: MeshRole,
) -> Option<(Arc<Mesh>, Provenance)> {
    while let Some(entry) = meshes.get(*cursor) {
        if entry.command > command {
            return None;
        }
        *cursor += 1;
        if entry.command == command && entry.role == role {
            return Some((Arc::clone(&entry.mesh), entry.provenance.clone()));
        }
    }
    None
}

/// The triangles a clip's region covers.
fn clip_mesh(
    list: &DisplayList,
    clip: &Clip,
    provider: &dyn GeometryProvider,
    tessellator: &mut Tessellator,
    options: TessellationOptions,
    command: usize,
) -> Result<Arc<Mesh>, PaintError> {
    let geometry = match clip.geometry {
        Some(slot) => list.geometry(slot).ok_or(PaintError::MissingResource {
            table: "geometry",
            index: slot.index(),
            command,
        })?,
        None => Geometry::Rectangle(clip.bounds),
    };
    Ok(tessellator.fill(&geometry, provider, options)?)
}

/// Resolve a paint index into something a pipeline can be handed.
fn resolve_paint(
    list: &DisplayList,
    slot: ResourceIndex,
    bounds: SceneRect,
    command: usize,
) -> Result<PaintProgram, PaintError> {
    let paint = list.paint(slot).ok_or(PaintError::MissingResource {
        table: "paint",
        index: slot.index(),
        command,
    })?;
    Ok(match paint {
        Paint::Solid(color) => PaintProgram::Solid(color),
        Paint::Gradient(slot) => {
            let gradient = list.gradient(slot).ok_or(PaintError::MissingResource {
                table: "gradient",
                index: slot.index(),
                command,
            })?;
            PaintProgram::Gradient {
                mapping: GradientMapping::resolve(&gradient, bounds),
                ramp: Arc::new(GradientRamp::resolve(&gradient)),
            }
        }
        Paint::Pattern {
            preset,
            foreground,
            background,
        } => PaintProgram::Pattern {
            preset,
            foreground,
            background,
        },
        Paint::Image(slot) => {
            let image = list.image(slot).ok_or(PaintError::MissingResource {
                table: "image",
                index: slot.index(),
                command,
            })?;
            PaintProgram::Picture {
                handle: image.handle,
                image,
                destination: bounds,
            }
        }
    })
}

/// One colour standing for a paint that is not one colour.
fn representative_color(paint: &PaintProgram) -> Color {
    match paint {
        PaintProgram::Solid(color) => *color,
        PaintProgram::Gradient { ramp, .. } => match ramp.texel(0) {
            // The ramp is premultiplied; un-premultiplying it is what turns it back into a colour a
            // solid draw can use.
            Some([r, g, b, a]) if a > 0 => Color {
                red: ((u16::from(r) * 255) / u16::from(a)).min(255) as u8,
                green: ((u16::from(g) * 255) / u16::from(a)).min(255) as u8,
                blue: ((u16::from(b) * 255) / u16::from(a)).min(255) as u8,
                alpha: a,
            },
            _ => Color {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 0,
            },
        },
        PaintProgram::Pattern { foreground, .. } => *foreground,
        PaintProgram::Picture { .. } | PaintProgram::Glyphs { .. } => Color {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0xff,
        },
    }
}

/// The effect DAG a `PushEffect` names, in topological order with every colour resolved.
fn resolve_effects(
    list: &DisplayList,
    root: ResourceIndex,
    command: usize,
) -> Result<Vec<EffectNode>, PaintError> {
    // The table is stored in topological order and a node may only name a node below it, which is
    // `mjx-scene`'s whole acyclicity check — so walking from zero to the root collects every node
    // the root can reach, in an order where each node's input is already present.
    let mut nodes = Vec::new();
    for index in 0..=root.index() {
        let slot = ResourceIndex::new(index);
        let effect = list.effect(slot).ok_or(PaintError::MissingResource {
            table: "effect",
            index,
            command,
        })?;
        let color = match effect.paint {
            Some(paint) => {
                let resolved = resolve_paint(list, paint, SceneRect::UNIT, command)?;
                representative_color(&resolved)
            }
            None => Color {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 0,
            },
        };
        nodes.push(EffectNode {
            effect,
            color,
            input: effect.input.map(|input| input.index() as usize),
        });
    }
    Ok(nodes)
}

/// Turn a run of placed glyphs into quads, grouped by atlas page.
#[allow(
    clippy::too_many_arguments,
    reason = "a walk's whole state, threaded once"
)]
fn plan_glyph_run(
    run: &mjx_scene::SceneGlyphRun,
    color: Color,
    transform: SceneTransform,
    layers: &mut [Layer],
    current: usize,
    report: &mut DrawReport,
    list: &DisplayList,
    provider: &dyn GeometryProvider,
    tessellator: &mut Tessellator,
    command: usize,
) -> Result<(), PaintError> {
    // The run's own space: positions and images are in the bucket's pixels, and the whole run is
    // multiplied by `residual_scale` to reach the size that was actually asked for. Folding that
    // into the transform is what lets every quad below be written in bucket pixels.
    let residual = if run.residual_scale.is_finite() && run.residual_scale > 0.0 {
        run.residual_scale
    } else {
        1.0
    };
    let run_transform = compose(
        transform,
        SceneTransform {
            scale_x: residual,
            shear_y: 0.0,
            shear_x: 0.0,
            scale_y: residual,
            translate_x: run.origin.x,
            translate_y: run.origin.y,
        },
    );

    // Grouped by page so a paragraph is a handful of draw calls rather than one per letter. Pages
    // are few — an atlas holds thousands of glyphs on one — so a linear scan beats a map.
    let mut batches: Vec<(u32, BitmapFormat, Vec<GlyphQuad>)> = Vec::new();

    for glyph in &run.glyphs {
        match glyph.image {
            GlyphImage::Blank => {}
            GlyphImage::Atlas(placement) => {
                // **The subpixel phase is not added here, and that is deliberate.** `mjx-text`
                // rasterises each phase as its own image — `swash`'s `Render::offset` — and takes
                // the placement off the offset outline, so `offset_from_origin_x` already contains
                // it. A painter that added `glyph.subpixel` again would shift every glyph by up to
                // three quarters of a pixel, and the text would merely look slightly wrong.
                let quad = GlyphQuad {
                    x: glyph.x as f32 + f32::from(placement.offset_from_origin_x),
                    y: glyph.y as f32 + f32::from(placement.offset_from_origin_y),
                    width: f32::from(placement.width),
                    height: f32::from(placement.height),
                    u: f32::from(placement.x),
                    v: f32::from(placement.y),
                    texel_width: f32::from(placement.width),
                    texel_height: f32::from(placement.height),
                };
                match batches.iter_mut().find(|(page, format, _)| {
                    *page == placement.page && *format == placement.format
                }) {
                    Some((_, _, quads)) => quads.push(quad),
                    None => batches.push((placement.page, placement.format, vec![quad])),
                }
                report.glyphs += 1;
            }
            GlyphImage::Outline(slot) => {
                // Hand-off 4 from R07: an outline glyph is an ordinary path and must not get a
                // second pipeline. It is glyph-local, exactly as an atlas placement is, so it is
                // positioned by its transform and never translated into place.
                let geometry = list.geometry(slot).ok_or(PaintError::MissingResource {
                    table: "geometry",
                    index: slot.index(),
                    command,
                })?;
                let mesh = tessellator.fill(
                    &geometry,
                    provider,
                    TessellationOptions::for_glyph_run(run),
                )?;
                let at = compose(
                    run_transform,
                    SceneTransform {
                        scale_x: 1.0,
                        shear_y: 0.0,
                        shear_x: 0.0,
                        scale_y: 1.0,
                        translate_x: glyph.x as f32,
                        translate_y: glyph.y as f32,
                    },
                );
                report.triangles += mesh.triangle_count();
                report.glyphs += 1;
                report.draw_calls += 1;
                push_op(
                    layers,
                    current,
                    DrawOp::Mesh {
                        mesh,
                        transform: at,
                        paint: PaintProgram::Solid(color),
                        role: MeshRole::Fill,
                        // A glyph outline is the face's own contour, never a stand-in: it came out
                        // of `mjx-text`'s rasteriser and through no geometry provider at all.
                        provenance: Provenance::document(),
                    },
                );
            }
        }
    }

    for (page, format, quads) in batches {
        report.draw_calls += 1;
        push_op(
            layers,
            current,
            DrawOp::Glyphs {
                quads,
                transform: run_transform,
                paint: PaintProgram::Glyphs {
                    page,
                    format,
                    color,
                },
            },
        );
    }
    Ok(())
}

/// Whether an effect kind draws its result **behind** the subtree rather than over it.
///
/// A shadow and a glow are both "a coloured, blurred copy of the shape, under the shape"; every
/// other kind replaces or overlays what it consumed. Getting this backwards paints the shadow on
/// top, which is a bug that looks like a design decision.
#[must_use]
pub const fn draws_behind(kind: EffectKind) -> bool {
    matches!(
        kind,
        EffectKind::OuterShadow | EffectKind::Glow | EffectKind::Reflection
    )
}

/// The fill rule a clip's own outline is filled with when the list did not say.
pub const CLIP_FILL_RULE: FillRule = FillRule::NonZero;
