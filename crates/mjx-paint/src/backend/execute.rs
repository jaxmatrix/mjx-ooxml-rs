//! Phase two: the [`Painter`] implementation, and the encoder that turns a frame's records into one
//! command buffer.
//!
//! # Why every layer — including the frame's own — renders into a pooled texture
//!
//! Multisampling resolves *into* a target; it cannot load from one. So a second
//! [`Painter::draw`] on the same frame cannot open a pass that preserves what the first drew: the
//! multisample attachment would be loaded from its own previous contents, which are not the frame's.
//! Every layer therefore renders into a cleared pooled target, and each `draw` finishes with one
//! single-sampled blit that composites its root layer onto the frame's colour texture. That is one
//! full-viewport pass per call to `draw`, and it is what makes *"a page, then the selection, then
//! the in-canvas chrome"* three lists on one frame rather than three frames.

use std::collections::HashMap;

use mjx_scene::{BlendMode, DisplayList, SceneRect, SceneTransform};

use super::device::{OFFSCREEN_FORMAT, STENCIL_FORMAT};
use super::frame::{
    le_bytes_f32, le_bytes_u32, rect_corners, uniform_block, write_color, AtlasUploader,
    CompositeStep, Staging, UNIT_UV,
};
use super::{
    align_to, upload_texture, OpenFrame, PaintKind, Pass, PipelineKey, Record, StencilMode, TexRef,
    WgpuPainter, UNIFORM_FLOATS,
};
use crate::error::PaintError;
use crate::gradient::RAMP_TEXELS;
use crate::painter::{
    Antialiasing, BackendReport, Capabilities, DrawReport, Frame, FrameReport, Painter, Pixels,
};
use crate::plan::{draws_behind, plan_frame, LayerKind};
use crate::pool::PoolHandle;
use crate::resources::Resources;
use crate::surface::{SurfaceHost, Viewport};

/// How many bytes a row of a readback buffer must be a multiple of.
///
/// A `wgpu` requirement rather than a choice, and the reason a readback is a copy into a padded
/// buffer followed by a row-by-row repack rather than one `memcpy`.
const READBACK_ROW_ALIGNMENT: u32 = 256;

impl Painter for WgpuPainter {
    fn name(&self) -> &'static str {
        "wgpu"
    }

    fn backend(&self) -> BackendReport {
        self.device.report()
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            antialiasing: if self.device.sample_count() > 1 {
                Antialiasing::Multisample(self.device.sample_count())
            } else {
                Antialiasing::None
            },
            max_texture_size: self.device.max_texture_size(),
            exact_path_clipping: true,
            effect_texture_budget: self.pool.budget(),
        }
    }

    fn begin(
        &mut self,
        target: &mut dyn SurfaceHost,
        viewport: Viewport,
    ) -> Result<Frame, PaintError> {
        if let Some(open) = &self.open {
            return Err(PaintError::FrameAlreadyOpen { open: open.id });
        }
        let (width, height) = viewport.require_pixels()?;
        let limit = self.device.max_texture_size();
        if width > limit || height > limit {
            return Err(PaintError::TextureUnavailable {
                width,
                height,
                detail: format!(
                    "this device will not make a texture larger than {limit} on a side"
                ),
            });
        }

        // Reconfigure the swapchain when the window has changed size or scale since the last frame.
        // A surface whose configuration disagrees with the window is the ordinary cause of
        // `SurfaceLost`, and asking the host to redraw is the recovery.
        if let Some(surface) = &self.surface {
            let capabilities = surface.get_capabilities(self.device.adapter());
            surface.configure(
                self.device.device(),
                &wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: self.surface_format,
                    width,
                    height,
                    present_mode: capabilities
                        .present_modes
                        .first()
                        .copied()
                        .unwrap_or(wgpu::PresentMode::Fifo),
                    alpha_mode: capabilities
                        .alpha_modes
                        .first()
                        .copied()
                        .unwrap_or(wgpu::CompositeAlphaMode::Auto),
                    view_formats: Vec::new(),
                    desired_maximum_frame_latency: 2,
                    // `Auto` is what a document canvas wants: the swapchain stays in whatever
                    // space the display is in, and every colour in this painter is already
                    // eight-bit sRGB.
                    color_space: wgpu::SurfaceColorSpace::Auto,
                },
            );
        }

        let colour = self
            .device
            .device()
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("mjx-paint frame"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: OFFSCREEN_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
        let colour_view = colour.create_view(&wgpu::TextureViewDescriptor::default());
        let samples = self.device.sample_count();
        let multisample = if samples > 1 {
            Some(
                self.device
                    .device()
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("mjx-paint multisample"),
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: samples,
                        dimension: wgpu::TextureDimension::D2,
                        format: OFFSCREEN_FORMAT,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[],
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            )
        } else {
            None
        };
        let stencil = self
            .device
            .device()
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("mjx-paint clip stencil"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format: STENCIL_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Start the frame transparent. Not a formality: `read_pixels` on a frame nothing was drawn
        // into must answer "nothing", and a target that was never written holds whatever the driver
        // left there.
        let mut encoder =
            self.device
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("mjx-paint clear"),
                });
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("mjx-paint clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &colour_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        }));
        self.device.queue().submit([encoder.finish()]);

        let id = self.next_frame;
        self.next_frame = self.next_frame.saturating_add(1);
        let presenting = self.surface.is_some();
        // A host with no event loop answers `false`, and a painter that treated that as "a frame is
        // coming" would report success for a frame nothing will ever deliver. Nothing is being
        // recovered from here, so nothing is asked for — the call exists at `end`, where a lost
        // surface is.
        let _ = target.raw_handle();
        self.open = Some(OpenFrame {
            id,
            width,
            height,
            colour,
            colour_view,
            multisample,
            stencil,
            presenting,
            report: DrawReport::default(),
        });
        Ok(Frame::new(id, viewport))
    }

    fn draw(
        &mut self,
        frame: &Frame,
        list: &DisplayList,
        resources: &mut Resources<'_>,
    ) -> Result<DrawReport, PaintError> {
        let (width, height) = match &self.open {
            Some(open) if open.id == frame.id() => (open.width, open.height),
            open => {
                return Err(PaintError::WrongFrame {
                    given: frame.id(),
                    open: open.as_ref().map(|open| open.id),
                })
            }
        };
        let viewport = (width, height);

        let plan = plan_frame(list, resources.geometry(), &mut self.tessellator)?;
        let mut report = plan.report();

        // Upload only what changed. A frame that drew the same words as the last one takes a delta
        // with nothing in it and uploads nothing, which is what `atlas_bytes_uploaded` reports and
        // what `tests/the_atlas_delta_reaches_the_painter.rs` asserts over **two** frames — one
        // frame cannot tell an incremental upload from a wholesale one.
        {
            let mut uploader = AtlasUploader {
                device: &self.device,
                pages: &mut self.atlas,
                created: 0,
                released: 0,
                bytes: 0,
            };
            resources.glyphs().take_changes(&mut uploader)?;
            report.atlas_pages_created = uploader.created;
            report.atlas_pages_released = uploader.released;
            report.atlas_bytes_uploaded = uploader.bytes;
        }
        self.upload_images(&plan, resources.images())?;

        let mut staging = Staging::new();
        let mut composites: HashMap<usize, Vec<CompositeStep>> = HashMap::new();
        let mut targets: HashMap<usize, PoolHandle> = HashMap::new();

        // Descending, which is a topological order with no sort: a layer is opened while its parent
        // is being walked, so a child always has a higher index than its parent.
        for index in (0..plan.layers().len()).rev() {
            let handle = self.acquire_target(width, height, &mut staging)?;
            targets.insert(index, handle);
            let bounds = self.build_layer(
                &plan,
                index,
                Some(handle),
                viewport,
                true,
                &mut staging,
                &composites,
            );
            let content = bounds.unwrap_or(SceneRect::new(0.0, 0.0, width as f32, height as f32));
            let Some(layer) = plan.layer(index) else {
                continue;
            };
            let steps = match &layer.kind {
                LayerKind::Root => continue,
                LayerKind::Opacity(alpha) => {
                    let mut step = CompositeStep::blit(TexRef::Pooled(handle));
                    step.alpha = *alpha;
                    vec![step]
                }
                LayerKind::Effect(nodes) => self.evaluate_effects(
                    nodes,
                    TexRef::Pooled(handle),
                    content,
                    viewport,
                    &mut staging,
                )?,
            };
            composites.insert(index, steps);
        }

        // The root layer's target, composited onto the frame. See this module's documentation for
        // why the frame is not drawn into directly.
        if let Some(root) = targets.get(&0).copied() {
            let full = SceneRect::new(0.0, 0.0, width as f32, height as f32);
            let (indices, base) = staging.quad(rect_corners(full), UNIT_UV);
            let block = uniform_block(SceneTransform::IDENTITY, viewport, PaintKind::Blit, 1.0);
            let slot = staging.uniform(block);
            staging.passes.push(Pass {
                target: None,
                geometry: false,
                clear: false,
                records: vec![Record {
                    uniform_slot: slot,
                    indices,
                    base_vertex: base,
                    textures: (TexRef::Pooled(root), TexRef::White),
                    key: PipelineKey {
                        blend: BlendMode::Over,
                        stencil: StencilMode::Absent,
                        samples: 1,
                        format: OFFSCREEN_FORMAT,
                    },
                    stencil_reference: 0,
                }],
            });
        }

        self.encode(staging, viewport)?;

        if let Some(open) = &mut self.open {
            open.report.absorb(report);
        }
        Ok(report)
    }

    fn end(&mut self, frame: Frame) -> Result<FrameReport, PaintError> {
        let Some(open) = self.open.take() else {
            return Err(PaintError::WrongFrame {
                given: frame.id(),
                open: None,
            });
        };
        if open.id != frame.id() {
            let id = open.id;
            self.open = Some(open);
            return Err(PaintError::WrongFrame {
                given: frame.id(),
                open: Some(id),
            });
        }

        let mut presented = false;
        if open.presenting {
            match self.present(&open) {
                Ok(()) => presented = true,
                Err(error) => {
                    // A lost surface is a normal event — a window resized, minimised or moved to
                    // another display between one frame and the next. The frame's own pixels are
                    // intact, so they are kept for readback and the error is reported.
                    self.last_colour = Some(open.colour);
                    self.last_pixels = None;
                    return Err(error);
                }
            }
        }

        self.device.settle()?;
        let report = FrameReport {
            frame: open.id,
            drawn: open.report,
            width: open.width,
            height: open.height,
            sample_count: self.device.sample_count(),
            presented,
            pool: self.pool.statistics(),
        };
        self.last_colour = Some(open.colour);
        self.last_pixels = None;
        self.last_report = Some(report);
        Ok(report)
    }

    fn read_pixels(&mut self) -> Result<Option<Pixels>, PaintError> {
        if let Some(pixels) = &self.last_pixels {
            return Ok(Some(pixels.clone()));
        }
        let Some(texture) = &self.last_colour else {
            return Ok(None);
        };
        let width = texture.width();
        let height = texture.height();
        let padded = align_to(width * 4, READBACK_ROW_ALIGNMENT);
        let buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("mjx-paint readback"),
            size: u64::from(padded) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder =
            self.device
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("mjx-paint readback"),
                });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.device.queue().submit([encoder.finish()]);
        buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        if self
            .device
            .device()
            .poll(wgpu::PollType::wait_indefinitely())
            .is_err()
        {
            return Err(PaintError::Readback(
                "the device did not settle while a readback was mapped".to_owned(),
            ));
        }
        let mapped = buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|error| PaintError::Readback(error.to_string()))?;
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * padded) as usize;
            let end = start + (width * 4) as usize;
            match mapped.get(start..end) {
                Some(bytes) => rgba.extend_from_slice(bytes),
                None => {
                    return Err(PaintError::Readback(format!(
                        "the readback buffer is short of row {row}"
                    )))
                }
            }
        }
        drop(mapped);
        buffer.unmap();
        let pixels = Pixels {
            width,
            height,
            rgba,
        };
        self.last_pixels = Some(pixels.clone());
        Ok(Some(pixels))
    }
}

impl WgpuPainter {
    /// Evaluate an effect DAG over the subtree's texture and answer the parent's composite steps.
    ///
    /// Every node is materialised into a texture of its own so that a chain — a blur of a glow —
    /// works with no special case; only the root's steps reach the parent.
    fn evaluate_effects(
        &mut self,
        nodes: &[crate::plan::EffectNode],
        subtree: TexRef,
        content: SceneRect,
        viewport: (u32, u32),
        staging: &mut Staging,
    ) -> Result<Vec<CompositeStep>, PaintError> {
        let Some(root) = nodes.last() else {
            return Ok(vec![CompositeStep::blit(subtree)]);
        };
        let mut outputs: Vec<TexRef> = Vec::with_capacity(nodes.len());
        let mut root_steps = Vec::new();
        for (position, node) in nodes.iter().enumerate() {
            let input = match node.input {
                Some(index) => outputs.get(index).copied().unwrap_or(subtree),
                None => subtree,
            };
            let steps = self.effect_steps(node, input, content, viewport, staging)?;
            if position + 1 == nodes.len() {
                root_steps = steps;
                break;
            }
            // A node that is not the root has to become a texture, because the node above it takes a
            // texture. Its own composite steps are drawn into a fresh target in one pass.
            let target = self.acquire_target(viewport.0, viewport.1, staging)?;
            let mut records = Vec::new();
            for step in &steps {
                let full = SceneRect::new(0.0, 0.0, viewport.0 as f32, viewport.1 as f32);
                let (indices, base) = staging.quad(rect_corners(full), UNIT_UV);
                let mut block = uniform_block(step.transform, viewport, step.kind, step.alpha);
                write_color(&mut block, 8, step.color);
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
                        stencil: StencilMode::Absent,
                        samples: 1,
                        format: OFFSCREEN_FORMAT,
                    },
                    stencil_reference: 0,
                });
            }
            staging.passes.push(Pass {
                target: Some(target),
                geometry: false,
                clear: true,
                records,
            });
            outputs.push(TexRef::Pooled(target));
        }
        // Where the effect's result goes relative to the subtree. Behind for a shadow, a glow and a
        // reflection; instead of it for a blur and a soft edge; over it for an inner shadow and a
        // fill overlay — and `effect_steps` has already put the subtree's own blit in the right
        // place for the last two.
        let _ = draws_behind(root.effect.kind);
        Ok(root_steps)
    }

    /// Put the frame's colour on the window.
    fn present(&mut self, open: &OpenFrame) -> Result<(), PaintError> {
        let Some(surface) = self.surface.as_ref() else {
            return Ok(());
        };
        let surface_texture = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            // Suboptimal is still drawable: the swapchain would prefer to be reconfigured, and the
            // next `begin` does exactly that. Presenting it is better than dropping the frame.
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            other => {
                return Err(PaintError::SurfaceLost(format!(
                    "the swapchain answered {other:?}"
                )))
            }
        };
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let format = self.surface_format;
        self.blit(&view, &open.colour_view, format, open.width, open.height)?;
        self.device.queue().present(surface_texture);
        Ok(())
    }

    /// One full-target quad, sampling `source` into `into`.
    ///
    /// Its own tiny path rather than a [`Pass`], because it is the only draw in this painter whose
    /// colour attachment is not [`OFFSCREEN_FORMAT`]: a swapchain picks its own format, and a
    /// pipeline is built per format.
    fn blit(
        &mut self,
        into: &wgpu::TextureView,
        source: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Result<(), PaintError> {
        let key = PipelineKey {
            blend: BlendMode::Over,
            stencil: StencilMode::Absent,
            samples: 1,
            format,
        };
        self.ensure_pipeline(key);
        let block = uniform_block(
            SceneTransform::IDENTITY,
            (width, height),
            PaintKind::Blit,
            1.0,
        );
        let uniform = self.upload_uniforms(&block);
        let quad = SceneRect::new(0.0, 0.0, width as f32, height as f32);
        let (vertices, indices) = self.upload_quad(quad);
        let bind_uniform = self
            .device
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("mjx-paint blit uniform"),
                layout: &self.uniform_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &uniform,
                        offset: 0,
                        size: wgpu::BufferSize::new((UNIFORM_FLOATS * 4) as u64),
                    }),
                }],
            });
        let bind_textures = self
            .device
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("mjx-paint blit textures"),
                layout: &self.texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.white),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.clamped),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.repeating),
                    },
                ],
            });
        let Some(pipeline) = self.pipelines.get(&key) else {
            return Err(PaintError::Device(
                "the blit pipeline could not be built".to_owned(),
            ));
        };
        let mut encoder =
            self.device
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("mjx-paint blit"),
                });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mjx-paint blit"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: into,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_uniform, &[0]);
            pass.set_bind_group(1, &bind_textures, &[]);
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..6, 0, 0..1);
        }
        self.device.queue().submit([encoder.finish()]);
        Ok(())
    }

    /// One uniform block in a buffer of its own.
    fn upload_uniforms(&self, block: &[f32; UNIFORM_FLOATS]) -> wgpu::Buffer {
        let bytes = le_bytes_f32(block);
        let buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("mjx-paint uniform"),
            size: bytes.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.device.queue().write_buffer(&buffer, 0, &bytes);
        buffer
    }

    /// One quad's vertices and indices, in buffers of their own.
    fn upload_quad(&self, rect: SceneRect) -> (wgpu::Buffer, wgpu::Buffer) {
        let corners = rect_corners(rect);
        let mut vertices = Vec::with_capacity(16);
        for (corner, uv) in corners.iter().zip(UNIT_UV.iter()) {
            vertices.extend_from_slice(&[corner.0, corner.1, uv.0, uv.1]);
        }
        let vertex_bytes = le_bytes_f32(&vertices);
        let index_bytes = le_bytes_u32(&[0, 1, 2, 0, 2, 3]);
        let vertex_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("mjx-paint quad vertices"),
            size: vertex_bytes.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("mjx-paint quad indices"),
            size: index_bytes.len() as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.device
            .queue()
            .write_buffer(&vertex_buffer, 0, &vertex_bytes);
        self.device
            .queue()
            .write_buffer(&index_buffer, 0, &index_bytes);
        (vertex_buffer, index_buffer)
    }

    /// Phase two: create the frame's three buffers, build the pipelines the records need, and encode
    /// every pass into one command buffer.
    fn encode(&mut self, staging: Staging, viewport: (u32, u32)) -> Result<(), PaintError> {
        if staging.passes.is_empty() {
            self.release_all(&staging);
            return Ok(());
        }
        let (multisample, stencil, frame_view) = match &self.open {
            Some(open) => (
                open.multisample.clone(),
                open.stencil.clone(),
                open.colour_view.clone(),
            ),
            None => {
                self.release_all(&staging);
                return Err(PaintError::WrongFrame {
                    given: 0,
                    open: None,
                });
            }
        };

        // This frame's gradient ramps, one row each. A page whose theme uses one gradient on forty
        // shapes resolves it once and forty draws share a row, because `Staging::ramp_row` keys on
        // the resolved texels rather than on the record that produced them.
        let ramps = if staging.ramps.is_empty() {
            None
        } else {
            let rows = (staging.ramps.len() / (RAMP_TEXELS * 4)) as u32;
            Some(upload_texture(
                &self.device,
                "mjx-paint gradient ramps",
                RAMP_TEXELS as u32,
                rows,
                wgpu::TextureFormat::Rgba8Unorm,
                &staging.ramps,
            )?)
        };
        let ramp_rows = staging.ramps.len() / (RAMP_TEXELS * 4);

        let vertex_bytes = le_bytes_f32(&staging.vertices);
        let index_bytes = le_bytes_u32(&staging.indices);
        let stride = self.uniform_stride as usize;
        let slots = staging.uniforms.len() / UNIFORM_FLOATS;
        let mut uniform_bytes = vec![0u8; stride * slots.max(1)];
        for slot in 0..slots {
            let Some(source) = staging
                .uniforms
                .get(slot * UNIFORM_FLOATS..(slot + 1) * UNIFORM_FLOATS)
            else {
                continue;
            };
            let mut block = [0.0f32; UNIFORM_FLOATS];
            block.copy_from_slice(source);
            // The gradient path needs to know how many rows the ramp texture has in order to turn a
            // row index into a texture coordinate, and that is only known once every ramp in the
            // frame has been collected — which is after the record that reads it was built.
            block[27] = ramp_rows as f32;
            let bytes = le_bytes_f32(&block);
            if let Some(slice) = uniform_bytes.get_mut(slot * stride..slot * stride + bytes.len()) {
                slice.copy_from_slice(&bytes);
            }
        }

        if vertex_bytes.is_empty() || index_bytes.is_empty() {
            self.release_all(&staging);
            return Ok(());
        }

        let vertex_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("mjx-paint vertices"),
            size: vertex_bytes.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("mjx-paint indices"),
            size: index_bytes.len() as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("mjx-paint uniforms"),
            size: uniform_bytes.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.device
            .queue()
            .write_buffer(&vertex_buffer, 0, &vertex_bytes);
        self.device
            .queue()
            .write_buffer(&index_buffer, 0, &index_bytes);
        self.device
            .queue()
            .write_buffer(&uniform_buffer, 0, &uniform_bytes);

        for pass in &staging.passes {
            for record in &pass.records {
                self.ensure_pipeline(record.key);
            }
        }

        let bind_uniform = self
            .device
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("mjx-paint uniforms"),
                layout: &self.uniform_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &uniform_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new((UNIFORM_FLOATS * 4) as u64),
                    }),
                }],
            });

        // A view per pooled target, made once. Two records that sample the same layer share one.
        let mut pooled: HashMap<PoolHandle, wgpu::TextureView> = HashMap::new();
        for handle in &staging.acquired {
            if let Ok(texture) = self.pool.texture(*handle) {
                pooled.insert(
                    *handle,
                    texture.create_view(&wgpu::TextureViewDescriptor::default()),
                );
            }
        }

        let resolve = |reference: TexRef| -> &wgpu::TextureView {
            match reference {
                TexRef::White => &self.white,
                TexRef::Pattern => &self.pattern,
                TexRef::Ramps => ramps.as_ref().map(|(_, view)| view).unwrap_or(&self.white),
                TexRef::Atlas(page) => self
                    .atlas
                    .get(&page)
                    .map(|(_, view)| view)
                    .unwrap_or(&self.white),
                TexRef::Image(handle) => self
                    .images
                    .get(&handle)
                    .map(|(_, view)| view)
                    .unwrap_or(&self.white),
                TexRef::Pooled(handle) => pooled.get(&handle).unwrap_or(&self.white),
            }
        };

        // One bind group per distinct texture pair. A paragraph of a thousand glyphs on one atlas
        // page is one bind group, which is the whole reason the plan batched them.
        let mut bind_groups: HashMap<(TexRef, TexRef), wgpu::BindGroup> = HashMap::new();
        for pass in &staging.passes {
            for record in &pass.records {
                if bind_groups.contains_key(&record.textures) {
                    continue;
                }
                let group = self
                    .device
                    .device()
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("mjx-paint textures"),
                        layout: &self.texture_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(resolve(
                                    record.textures.0,
                                )),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(resolve(
                                    record.textures.1,
                                )),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::Sampler(&self.clamped),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: wgpu::BindingResource::Sampler(&self.repeating),
                            },
                        ],
                    });
                bind_groups.insert(record.textures, group);
            }
        }

        let mut encoder =
            self.device
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("mjx-paint frame"),
                });
        for pass in &staging.passes {
            let target_view = match pass.target {
                Some(handle) => match pooled.get(&handle) {
                    Some(view) => view,
                    None => continue,
                },
                None => &frame_view,
            };
            let load = if pass.clear {
                wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
            } else {
                wgpu::LoadOp::Load
            };
            let multisampled = pass.geometry && multisample.is_some();
            let colour = if multisampled {
                wgpu::RenderPassColorAttachment {
                    view: multisample.as_ref().unwrap_or(target_view),
                    depth_slice: None,
                    resolve_target: Some(target_view),
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                }
            } else {
                wgpu::RenderPassColorAttachment {
                    view: target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                }
            };
            let depth = if pass.geometry {
                Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &stencil,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    // Every layer's clip stack starts empty, because a layer is its own target and
                    // the clips that were open around it were re-emitted into it by the plan.
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Discard,
                    }),
                })
            } else {
                None
            };
            let mut render = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mjx-paint pass"),
                color_attachments: &[Some(colour)],
                depth_stencil_attachment: depth,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            render.set_vertex_buffer(0, vertex_buffer.slice(..));
            render.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            for record in &pass.records {
                let (Some(pipeline), Some(textures)) = (
                    self.pipelines.get(&record.key),
                    bind_groups.get(&record.textures),
                ) else {
                    continue;
                };
                render.set_pipeline(pipeline);
                let offset = (record.uniform_slot as u32).saturating_mul(self.uniform_stride);
                render.set_bind_group(0, &bind_uniform, &[offset]);
                render.set_bind_group(1, textures, &[]);
                if pass.geometry {
                    render.set_stencil_reference(record.stencil_reference);
                }
                render.draw_indexed(record.indices.clone(), record.base_vertex, 0..1);
            }
            drop(render);
        }
        self.device.queue().submit([encoder.finish()]);
        let _ = viewport;
        self.release_all(&staging);
        self.device.take_complaints()
    }

    /// Build the pipeline for `key` if it does not exist yet.
    fn ensure_pipeline(&mut self, key: PipelineKey) {
        if self.pipelines.contains_key(&key) {
            return;
        }
        let pipeline = super::build_pipeline(
            self.device.device(),
            &self.pipeline_layout,
            &self.shader,
            key,
        );
        self.pipelines.insert(key, pipeline);
    }

    /// Give every target this frame acquired back to the pool.
    ///
    /// **Where the byte budget is actually enforced**, because releasing is the only moment the pool
    /// decides to keep anything. A frame that forgot to do this would leave every target in flight
    /// for ever, the budget would bound nothing, and the pool would be a leak with statistics.
    fn release_all(&mut self, staging: &Staging) {
        for handle in &staging.acquired {
            // A handle the pool has already retired is not an error worth failing a frame over: it
            // means the same target was released twice, which is a bug in this file and not in the
            // document being drawn.
            let _ = self.pool.release(*handle);
        }
    }
}
