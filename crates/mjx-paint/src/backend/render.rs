//! Phase one, continued: turning a layer's operations and an effect's DAG into records.
//!
//! Split from [`super::frame`] only for length — it is the same phase and the same walk. What lives
//! here is everything that decides *what a draw looks like*: the uniform block a paint resolves to,
//! the texture coordinates a picture is laid out with, and the passes an effect needs.
//!
//! # How an effect is executed
//!
//! Every node of the DAG is materialised into a texture, and then the **root** node's result is
//! composited into the parent in one of three ways, decided by the kind:
//!
//! * behind the subtree — a shadow, a glow, a reflection: the effect, then the subtree over it;
//! * instead of the subtree — a blur, a soft edge: the effect alone, because it already contains
//!   the subtree;
//! * over the subtree — an inner shadow, a fill overlay: the subtree, then the effect over it in
//!   the node's own blend mode.
//!
//! Getting the first of those backwards paints the shadow on top of the shape, which is a defect
//! that looks like a design decision, so [`crate::plan::draws_behind`] states it once and both this
//! painter and the software one read it from there.
//!
//! **That last sentence was false until MJXOFF-164.** R08 wrote it, and wrote
//! `let _ = draws_behind(root.effect.kind);` in `execute.rs` — the answer computed and thrown away —
//! while each arm of [`WgpuPainter::effect_steps`] pushed the subtree's own blit in a hard-coded
//! position of its own. Flipping `draws_behind` failed a test and changed no pixel; swapping the two
//! pushes in the `Glow | OuterShadow` arm painted every shadow **on top of its shape** and left the
//! whole suite green. The arms below now produce the effect's own output only, and the position of
//! the subtree comes from [`crate::plan::draws_behind`] and [`crate::plan::replaces_subtree`] in one
//! place at the foot of that function.

use super::device::OFFSCREEN_FORMAT;
use super::frame::{
    absorb, rect_corners, uniform_block, write_color, CompositeStep, Staging, UNIT_UV,
};
use super::{
    PaintKind, Pass, PipelineKey, Record, StencilMode, TexRef, WgpuPainter, PLACEHOLDER_WARNING,
    UNIFORM_FLOATS,
};
use crate::error::PaintError;
use crate::plan::{draws_behind, replaces_subtree, DrawOp, EffectNode, PaintProgram};
use crate::pool::PoolHandle;
use mjx_scene::{
    BitmapFormat, BlendMode, Color, EffectKind, ImageFillMode, SceneRect, SceneTransform,
};
use std::collections::HashMap;

/// A colour with nothing in it, for a draw whose kind reads none.
const TRANSPARENT: Color = Color {
    red: 0,
    green: 0,
    blue: 0,
    alpha: 0,
};

impl WgpuPainter {
    /// Phase one for one layer: turn its operations into records, appending to `staging`.
    ///
    /// Answers the device-space box the layer's content covers, which is what a reflection needs an
    /// axis to flip about — a reflection about the viewport's own bottom edge would put the copy
    /// wherever the page happens to end rather than under the shape.
    #[allow(
        clippy::too_many_arguments,
        reason = "one walk's whole state, threaded once"
    )]
    pub(super) fn build_layer(
        &mut self,
        plan: &crate::plan::FramePlan,
        index: usize,
        target: Option<PoolHandle>,
        viewport: (u32, u32),
        clear: bool,
        staging: &mut Staging,
        composites: &HashMap<usize, Vec<CompositeStep>>,
    ) -> Option<SceneRect> {
        let layer = plan.layer(index)?;
        let mut records = Vec::new();
        let mut bounds: Option<SceneRect> = None;
        // The clip depth. Every ordinary draw tests `stencil == depth`; a `PushClip` deepens it
        // inside its own shape and a `PopClip` shallows it again, which is what makes nested clips
        // intersect rather than replace.
        let mut depth = 0u32;
        let samples = self.device.sample_count();

        for op in &layer.ops {
            match op {
                DrawOp::Mesh {
                    mesh,
                    transform,
                    paint,
                    provenance,
                    ..
                } => {
                    if mesh.is_empty() {
                        continue;
                    }
                    absorb(&mut bounds, mesh.bounds(), *transform);
                    let destination = match paint {
                        PaintProgram::Picture { destination, .. } => Some(*destination),
                        _ => None,
                    };
                    let base = (staging.vertices.len() / 4) as i32;
                    let positions = mesh.positions();
                    for vertex in 0..mesh.vertex_count() {
                        let x = positions.get(vertex * 2).copied().unwrap_or_default();
                        let y = positions.get(vertex * 2 + 1).copied().unwrap_or_default();
                        let (u, v) = match destination {
                            Some(rect) => (
                                (x - rect.left) / (rect.right - rect.left).max(f32::EPSILON),
                                (y - rect.top) / (rect.bottom - rect.top).max(f32::EPSILON),
                            ),
                            None => (0.0, 0.0),
                        };
                        staging.vertices.extend_from_slice(&[x, y, u, v]);
                    }
                    let start = staging.indices.len() as u32;
                    staging.indices.extend_from_slice(mesh.indices());
                    let end = staging.indices.len() as u32;

                    // A stand-in shape, painted so that a reviewer can see it is a stand-in. Turned
                    // off for R10's golden images, which assert `DrawReport::placeholders` is zero
                    // instead — the stronger of the two checks, and the one a machine can make.
                    let warning = PaintProgram::Solid(PLACEHOLDER_WARNING);
                    let paint = if self.highlight_placeholders && provenance.is_placeholder() {
                        &warning
                    } else {
                        paint
                    };
                    let (_, block, textures) =
                        self.paint_uniform(paint, *transform, viewport, 1.0, staging);
                    let slot = staging.uniform(block);
                    records.push(Record {
                        uniform_slot: slot,
                        indices: start..end,
                        base_vertex: base,
                        textures,
                        key: PipelineKey {
                            blend: BlendMode::Over,
                            stencil: StencilMode::Test,
                            samples,
                            format: OFFSCREEN_FORMAT,
                        },
                        stencil_reference: depth,
                    });
                }
                DrawOp::Glyphs {
                    quads,
                    transform,
                    paint,
                    ..
                } => {
                    if quads.is_empty() {
                        continue;
                    }
                    let base = (staging.vertices.len() / 4) as i32;
                    let start = staging.indices.len() as u32;
                    for (position, quad) in quads.iter().enumerate() {
                        let rect = SceneRect::new(
                            quad.x,
                            quad.y,
                            quad.x + quad.width,
                            quad.y + quad.height,
                        );
                        absorb(&mut bounds, rect, *transform);
                        let corners = rect_corners(rect);
                        let uv = [
                            (quad.u, quad.v),
                            (quad.u + quad.texel_width, quad.v),
                            (quad.u + quad.texel_width, quad.v + quad.texel_height),
                            (quad.u, quad.v + quad.texel_height),
                        ];
                        for (corner, texel) in corners.iter().zip(uv.iter()) {
                            staging
                                .vertices
                                .extend_from_slice(&[corner.0, corner.1, texel.0, texel.1]);
                        }
                        let first = (position * 4) as u32;
                        staging.indices.extend_from_slice(&[
                            first,
                            first + 1,
                            first + 2,
                            first,
                            first + 2,
                            first + 3,
                        ]);
                    }
                    let end = staging.indices.len() as u32;
                    let (_, block, textures) =
                        self.paint_uniform(paint, *transform, viewport, 1.0, staging);
                    let slot = staging.uniform(block);
                    records.push(Record {
                        uniform_slot: slot,
                        indices: start..end,
                        base_vertex: base,
                        textures,
                        key: PipelineKey {
                            blend: BlendMode::Over,
                            stencil: StencilMode::Test,
                            samples,
                            format: OFFSCREEN_FORMAT,
                        },
                        stencil_reference: depth,
                    });
                }
                DrawOp::Picture {
                    destination,
                    transform,
                    paint,
                    ..
                } => {
                    absorb(&mut bounds, *destination, *transform);
                    let uv = self.picture_uv(paint, *destination);
                    let (indices, base) = staging.quad(rect_corners(*destination), uv);
                    let (_, block, textures) =
                        self.paint_uniform(paint, *transform, viewport, 1.0, staging);
                    let slot = staging.uniform(block);
                    records.push(Record {
                        uniform_slot: slot,
                        indices,
                        base_vertex: base,
                        textures,
                        key: PipelineKey {
                            blend: BlendMode::Over,
                            stencil: StencilMode::Test,
                            samples,
                            format: OFFSCREEN_FORMAT,
                        },
                        stencil_reference: depth,
                    });
                }
                DrawOp::PushClip {
                    mesh, transform, ..
                }
                | DrawOp::PopClip {
                    mesh, transform, ..
                } => {
                    let pushing = matches!(op, DrawOp::PushClip { .. });
                    if mesh.is_empty() {
                        // A clip whose shape has no triangles clips everything away. The depth still
                        // moves, so every draw inside it tests against a value the stencil never
                        // reaches and nothing is drawn — which is what an empty clip means.
                        depth = if pushing {
                            depth.saturating_add(1)
                        } else {
                            depth.saturating_sub(1)
                        };
                        continue;
                    }
                    let base = (staging.vertices.len() / 4) as i32;
                    let positions = mesh.positions();
                    for vertex in 0..mesh.vertex_count() {
                        let x = positions.get(vertex * 2).copied().unwrap_or_default();
                        let y = positions.get(vertex * 2 + 1).copied().unwrap_or_default();
                        staging.vertices.extend_from_slice(&[x, y, 0.0, 0.0]);
                    }
                    let start = staging.indices.len() as u32;
                    staging.indices.extend_from_slice(mesh.indices());
                    let end = staging.indices.len() as u32;
                    let block = uniform_block(*transform, viewport, PaintKind::Solid, 1.0);
                    let slot = staging.uniform(block);
                    // **Both test against `depth` as it stands, and that is not a coincidence.**
                    // Pushing deepens the stencil where it equals the current clip depth — which is
                    // `depth`, because the increment happens below. Popping shallows it where it
                    // equals the deeper value that push wrote — which is also `depth`, because the
                    // decrement has not happened yet either. The two are the same expression read
                    // at two different moments, and writing it as one is what makes the invariant
                    // legible: **the stencil inside the current clip always holds `depth`.**
                    let reference = depth;
                    records.push(Record {
                        uniform_slot: slot,
                        indices: start..end,
                        base_vertex: base,
                        textures: (TexRef::White, TexRef::White),
                        key: PipelineKey {
                            blend: BlendMode::Over,
                            stencil: if pushing {
                                StencilMode::Push
                            } else {
                                StencilMode::Pop
                            },
                            samples,
                            format: OFFSCREEN_FORMAT,
                        },
                        stencil_reference: reference,
                    });
                    depth = if pushing {
                        depth.saturating_add(1)
                    } else {
                        depth.saturating_sub(1)
                    };
                }
                DrawOp::Composite { layer: child, .. } => {
                    let Some(steps) = composites.get(child) else {
                        continue;
                    };
                    for step in steps {
                        let full = SceneRect::new(0.0, 0.0, viewport.0 as f32, viewport.1 as f32);
                        let (indices, base) = staging.quad(rect_corners(full), UNIT_UV);
                        let mut block =
                            uniform_block(step.transform, viewport, step.kind, step.alpha);
                        write_color(&mut block, 8, step.color);
                        block[28] = step.source_offset[0];
                        block[29] = step.source_offset[1];
                        block[31] = step.fade_axis;
                        block[36] = step.fade[0];
                        block[37] = step.fade[1];
                        block[38] = step.fade[2];
                        block[39] = step.fade[3];
                        let slot = staging.uniform(block);
                        records.push(Record {
                            uniform_slot: slot,
                            indices,
                            base_vertex: base,
                            textures: (step.source, step.second),
                            key: PipelineKey {
                                blend: step.blend,
                                stencil: StencilMode::Test,
                                samples,
                                format: OFFSCREEN_FORMAT,
                            },
                            stencil_reference: depth,
                        });
                    }
                }
            }
        }

        staging.passes.push(Pass {
            target,
            geometry: true,
            clear,
            records,
        });
        bounds
    }

    /// Texture coordinates for a picture drawn into `destination`.
    pub(super) fn picture_uv(
        &self,
        paint: &PaintProgram,
        destination: SceneRect,
    ) -> [(f32, f32); 4] {
        let PaintProgram::Picture { handle, image, .. } = paint else {
            return UNIT_UV;
        };
        match image.fill_mode {
            ImageFillMode::Stretch => {
                // `a:srcRect` is in fractions of the whole picture and names the part that is shown.
                // `SceneRect::UNIT` — the whole picture — falls out of the same arithmetic.
                let (u0, v0) = (image.crop.left, image.crop.top);
                let (u1, v1) = (image.crop.right, image.crop.bottom);
                [(u0, v0), (u1, v0), (u1, v1), (u0, v1)]
            }
            ImageFillMode::Tile => {
                // A tile repeats at its natural size, so how many times it fits is the destination
                // measured in tiles — which needs the picture's own size, and that is a fact about
                // the uploaded texture rather than about the display list.
                let (width, height) = self
                    .images
                    .get(handle)
                    .map(|(texture, _)| (texture.width() as f32, texture.height() as f32))
                    .unwrap_or((1.0, 1.0));
                let scale_x = finite_or(image.tile_scale_x, 1.0);
                let scale_y = finite_or(image.tile_scale_y, 1.0);
                let tile_width = (width * scale_x).abs().max(0.001);
                let tile_height = (height * scale_y).abs().max(0.001);
                let span_x = (destination.right - destination.left) / tile_width;
                let span_y = (destination.bottom - destination.top) / tile_height;
                let offset_x = -image.tile_offset.x / tile_width;
                let offset_y = -image.tile_offset.y / tile_height;
                [
                    (offset_x, offset_y),
                    (offset_x + span_x, offset_y),
                    (offset_x + span_x, offset_y + span_y),
                    (offset_x, offset_y + span_y),
                ]
            }
        }
    }

    /// The uniform block and the textures a paint needs.
    pub(super) fn paint_uniform(
        &mut self,
        paint: &PaintProgram,
        transform: SceneTransform,
        viewport: (u32, u32),
        opacity: f32,
        staging: &mut Staging,
    ) -> (PaintKind, [f32; UNIFORM_FLOATS], (TexRef, TexRef)) {
        match paint {
            PaintProgram::Solid(color) => {
                let mut block = uniform_block(transform, viewport, PaintKind::Solid, opacity);
                write_color(&mut block, 8, *color);
                (PaintKind::Solid, block, (TexRef::White, TexRef::White))
            }
            PaintProgram::Gradient { ramp, mapping } => {
                let row = staging.ramp_row(ramp);
                let mut block = uniform_block(transform, viewport, PaintKind::Gradient, opacity);
                block[16] = mapping.origin[0];
                block[17] = mapping.origin[1];
                block[18] = mapping.axis[0];
                block[19] = mapping.axis[1];
                block[20] = mapping.radii[0];
                block[21] = mapping.radii[1];
                block[22] = if mapping.radial { 1.0 } else { 0.0 };
                block[23] = row as f32;
                (PaintKind::Gradient, block, (TexRef::Ramps, TexRef::White))
            }
            PaintProgram::Pattern {
                preset,
                foreground,
                background,
            } => {
                let mut block = uniform_block(transform, viewport, PaintKind::Pattern, opacity);
                write_color(&mut block, 8, *foreground);
                write_color(&mut block, 12, *background);
                block[26] = preset.wire_value() as f32;
                (PaintKind::Pattern, block, (TexRef::Pattern, TexRef::White))
            }
            PaintProgram::Picture { handle, image, .. } => {
                let mut block = uniform_block(transform, viewport, PaintKind::Image, opacity);
                block[26] = if image.fill_mode == ImageFillMode::Tile {
                    1.0
                } else {
                    0.0
                };
                let adjustments = image.adjustments;
                block[32] = f32::from(adjustments.brightness_in_ten_thousandths) / 10_000.0;
                block[33] = 1.0 + f32::from(adjustments.contrast_in_ten_thousandths) / 10_000.0;
                block[34] = if adjustments.grayscale { 1.0 } else { 0.0 };
                block[35] = f32::from(adjustments.alpha_in_ten_thousandths) / 10_000.0;
                (
                    PaintKind::Image,
                    block,
                    (TexRef::Image(*handle), TexRef::White),
                )
            }
            PaintProgram::Glyphs {
                page,
                format,
                color,
            } => {
                let kind = match format {
                    BitmapFormat::Coverage => PaintKind::GlyphCoverage,
                    BitmapFormat::Rgba => PaintKind::GlyphColor,
                };
                let mut block = uniform_block(transform, viewport, kind, opacity);
                write_color(&mut block, 8, *color);
                (kind, block, (TexRef::Atlas(*page), TexRef::White))
            }
        }
    }

    /// One full-viewport effect pass into a fresh pooled target.
    #[allow(clippy::too_many_arguments, reason = "one pass's whole description")]
    pub(super) fn effect_pass(
        &mut self,
        viewport: (u32, u32),
        kind: PaintKind,
        source: TexRef,
        second: TexRef,
        color: Color,
        effect: [f32; 4],
        staging: &mut Staging,
    ) -> Result<TexRef, PaintError> {
        let target = self.acquire_target(viewport.0, viewport.1, staging)?;
        let full = SceneRect::new(0.0, 0.0, viewport.0 as f32, viewport.1 as f32);
        let (indices, base) = staging.quad(rect_corners(full), UNIT_UV);
        let mut block = uniform_block(SceneTransform::IDENTITY, viewport, kind, 1.0);
        write_color(&mut block, 8, color);
        block[28] = effect[0];
        block[29] = effect[1];
        block[30] = effect[2];
        block[31] = effect[3];
        let slot = staging.uniform(block);
        staging.passes.push(Pass {
            target: Some(target),
            geometry: false,
            clear: true,
            records: vec![Record {
                uniform_slot: slot,
                indices,
                base_vertex: base,
                textures: (source, second),
                key: PipelineKey {
                    blend: BlendMode::Over,
                    stencil: StencilMode::Absent,
                    samples: 1,
                    format: OFFSCREEN_FORMAT,
                },
                stencil_reference: 0,
            }],
        });
        Ok(TexRef::Pooled(target))
    }

    /// One pass that draws `fill` over `bounds` into a fresh pooled target.
    ///
    /// What a [`crate::LayerKind::Mask`]'s paint is drawn into before the stencil is applied to it.
    /// Separate from [`WgpuPainter::effect_pass`] because that one draws a *full-viewport* quad with
    /// the identity transform — right for a blur, wrong for a gradient, whose axis is resolved in
    /// the shape's own space and would run across the page instead of across the text.
    pub(super) fn fill_pass(
        &mut self,
        viewport: (u32, u32),
        paint: &PaintProgram,
        bounds: SceneRect,
        transform: SceneTransform,
        staging: &mut Staging,
    ) -> Result<TexRef, PaintError> {
        let target = self.acquire_target(viewport.0, viewport.1, staging)?;
        let uv = self.picture_uv(paint, bounds);
        let (indices, base) = staging.quad(rect_corners(bounds), uv);
        let (_, block, textures) = self.paint_uniform(paint, transform, viewport, 1.0, staging);
        let slot = staging.uniform(block);
        staging.passes.push(Pass {
            target: Some(target),
            geometry: false,
            clear: true,
            records: vec![Record {
                uniform_slot: slot,
                indices,
                base_vertex: base,
                textures,
                key: PipelineKey {
                    blend: BlendMode::Over,
                    stencil: StencilMode::Absent,
                    samples: 1,
                    format: OFFSCREEN_FORMAT,
                },
                stencil_reference: 0,
            }],
        });
        Ok(TexRef::Pooled(target))
    }

    /// Two separable passes: a blur of `source`.
    pub(super) fn blur(
        &mut self,
        viewport: (u32, u32),
        source: TexRef,
        radius: f32,
        staging: &mut Staging,
    ) -> Result<TexRef, PaintError> {
        // The tap spacing. Seventeen taps cover four standard deviations, so the step that puts the
        // outermost tap at `radius` is `radius / 8`. A radius that rounds to nothing is not a blur
        // and is answered with the input unchanged, because a step of zero makes every tap the same
        // texel and costs two passes to copy an image.
        let step = radius / 8.0;
        if !step.is_finite() || step <= 0.0 {
            return Ok(source);
        }
        let horizontal = self.effect_pass(
            viewport,
            PaintKind::Blur,
            source,
            TexRef::White,
            TRANSPARENT,
            [step / viewport.0.max(1) as f32, 1.0, 0.0, 0.0],
            staging,
        )?;
        self.effect_pass(
            viewport,
            PaintKind::Blur,
            horizontal,
            TexRef::White,
            TRANSPARENT,
            [step / viewport.1.max(1) as f32, 0.0, 1.0, 0.0],
            staging,
        )
    }

    /// Run one effect node over its input, and answer the composite steps its parent must draw.
    pub(super) fn effect_steps(
        &mut self,
        node: &EffectNode,
        input: TexRef,
        content: SceneRect,
        viewport: (u32, u32),
        staging: &mut Staging,
    ) -> Result<Vec<CompositeStep>, PaintError> {
        let radius = finite_or(node.effect.radius, 0.0).max(0.0);
        let direction = finite_or(node.effect.direction, 0.0);
        let distance = finite_or(node.effect.distance, 0.0).max(0.0);
        let offset = SceneTransform {
            scale_x: 1.0,
            shear_y: 0.0,
            shear_x: 0.0,
            scale_y: 1.0,
            translate_x: distance * direction.cos(),
            translate_y: distance * direction.sin(),
        };
        // **The effect's own output only.** Where the subtree goes relative to it is decided once,
        // below, out of `draws_behind` and `replaces_subtree` — see those two functions for why
        // that matters: until MJXOFF-164 the ordering was hard-coded in each arm here and
        // `draws_behind` was called and discarded, so the "single source of truth" decided nothing.
        let mut steps = Vec::new();
        match node.effect.kind {
            EffectKind::Blur => {
                let blurred = self.blur(viewport, input, radius, staging)?;
                steps.push(CompositeStep::blit(blurred));
            }
            EffectKind::Glow | EffectKind::OuterShadow => {
                // Blur first, then tint. A tint reads only alpha, and blurring alpha then colouring
                // it is the same picture as colouring then blurring — one pass fewer, and the pass
                // it saves is a full-viewport one.
                let blurred = self.blur(viewport, input, radius.max(1.0), staging)?;
                let mut tint = CompositeStep::blit(blurred);
                tint.kind = PaintKind::Tint;
                tint.color = node.color;
                if node.effect.kind == EffectKind::OuterShadow {
                    tint.transform = offset;
                }
                steps.push(tint);
            }
            EffectKind::InnerShadow => {
                let blurred = self.blur(viewport, input, radius.max(1.0), staging)?;
                let mut inner = CompositeStep::blit(blurred);
                inner.kind = PaintKind::InnerShadow;
                inner.second = input;
                inner.color = node.color;
                // The offset moves the **blurred copy** and not the mask. See `shaders.wgsl`'s
                // `KIND_INNER_SHADOW`: putting it on the quad, which is what R08 did, moves both
                // inputs and pushes the shadow outside the shape it is inside.
                inner.source_offset = [
                    -offset.translate_x / viewport.0.max(1) as f32,
                    -offset.translate_y / viewport.1.max(1) as f32,
                ];
                steps.push(inner);
            }
            EffectKind::SoftEdge => {
                let blurred = self.blur(viewport, input, radius.max(1.0), staging)?;
                let mut soft = CompositeStep::blit(blurred);
                soft.kind = PaintKind::MaskBySourceAlpha;
                soft.second = input;
                steps.push(soft);
            }
            EffectKind::Reflection => {
                // Flipped about the bottom of what the subtree actually covers. Flipping about the
                // viewport would put the reflection wherever the page happens to end rather than
                // under the shape, which is the whole of what a reflection is.
                let axis = content.bottom;
                let flip = SceneTransform {
                    scale_x: finite_or(node.effect.scale_x, 1.0),
                    shear_y: 0.0,
                    shear_x: 0.0,
                    scale_y: -finite_or(node.effect.scale_y, 1.0),
                    translate_x: distance * direction.cos(),
                    translate_y: 2.0 * axis + distance * direction.sin(),
                };
                let start_position = finite_or(node.effect.start_position, 0.0);
                let end_position = finite_or(node.effect.end_position, 1.0);
                let mut reflected = CompositeStep::blit(input);
                reflected.kind = PaintKind::Fade;
                reflected.transform = flip;
                reflected.fade = [
                    finite_or(node.effect.start_alpha, 1.0),
                    finite_or(node.effect.end_alpha, 0.0),
                    start_position,
                    end_position.max(start_position + 0.001),
                ];
                reflected.fade_axis = 1.0;
                steps.push(reflected);
            }
            EffectKind::FillOverlay => {
                let mut overlay = CompositeStep::blit(input);
                overlay.kind = PaintKind::Tint;
                overlay.color = node.color;
                overlay.blend = node.effect.blend;
                steps.push(overlay);
            }
        }
        // Where the subtree goes relative to what the effect produced, stated **once** for every
        // painter in this workspace. A blur and a soft edge already contain the subtree, so drawing
        // it again would put a sharp copy over the blurred one; a shadow, a glow and a reflection go
        // under it; an inner shadow and a fill overlay go over it.
        if !replaces_subtree(node.effect.kind) {
            if draws_behind(node.effect.kind) {
                steps.push(CompositeStep::blit(input));
            } else {
                steps.insert(0, CompositeStep::blit(input));
            }
        }
        Ok(steps)
    }
}

/// `value` if it is a number, and `fallback` if a document said something else.
fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}
