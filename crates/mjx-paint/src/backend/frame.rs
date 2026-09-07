//! Building and encoding one frame: the [`Painter`] implementation, and the two phases it runs in.
//!
//! # Two phases, and why
//!
//! **Phase one is arithmetic.** It walks the [`FramePlan`], appends vertices, indices and uniform
//! blocks to three staging vectors, acquires the render targets each layer and each effect pass
//! needs, and produces a list of [`Pass`]es made of [`Record`]s. No command is encoded and no
//! `wgpu` render pass exists.
//!
//! **Phase two is encoding.** The three buffers are created and uploaded once, every pipeline the
//! records need is built, and the passes are encoded back to back into one command buffer and
//! submitted once.
//!
//! The split is what makes the buffers a single upload instead of one per draw, and it is what
//! keeps the borrow checker out of the way: a `wgpu::RenderPass` borrows its encoder for its whole
//! life, so a design that decided what to draw *while* a pass was open could not also open the pass
//! its children need.

use std::collections::HashMap;

use mjx_scene::{BitmapFormat, BlendMode, Color, SceneRect, SceneTransform};

use super::device::OFFSCREEN_FORMAT;
use super::{PaintKind, Pass, TexRef, WgpuPainter, UNIFORM_FLOATS};
use crate::error::PaintError;
use crate::gradient::{premultiplied, GradientRamp, RAMP_TEXELS};
use crate::plan::{apply, DrawOp, PaintProgram};
use crate::pool::{PoolHandle, TextureSize};
use crate::resources::{bytes_per_pixel, AtlasPage, AtlasVisitor, AtlasWrite};

/// One composite of a finished layer into its parent.
#[derive(Clone, Copy, Debug)]
pub(super) struct CompositeStep {
    pub(super) source: TexRef,
    pub(super) second: TexRef,
    pub(super) kind: PaintKind,
    pub(super) alpha: f32,
    pub(super) transform: SceneTransform,
    pub(super) color: Color,
    /// Start opacity, end opacity, start position, end position — a reflection's fade.
    pub(super) fade: [f32; 4],
    /// `0.0` for a fade along `x`, `1.0` along `y`.
    pub(super) fade_axis: f32,
    pub(super) blend: BlendMode,
}

impl CompositeStep {
    pub(super) fn blit(source: TexRef) -> Self {
        Self {
            source,
            second: TexRef::White,
            kind: PaintKind::Blit,
            alpha: 1.0,
            transform: SceneTransform::IDENTITY,
            color: Color {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 0,
            },
            fade: [1.0, 1.0, 0.0, 1.0],
            fade_axis: 1.0,
            blend: BlendMode::Over,
        }
    }
}

/// The staging a frame is built into.
pub(super) struct Staging {
    pub(super) vertices: Vec<f32>,
    pub(super) indices: Vec<u32>,
    pub(super) uniforms: Vec<f32>,
    pub(super) passes: Vec<Pass>,
    pub(super) ramps: Vec<u8>,
    pub(super) ramp_rows: HashMap<Vec<u8>, u32>,
    pub(super) acquired: Vec<PoolHandle>,
}

impl Staging {
    pub(super) fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            uniforms: Vec::new(),
            passes: Vec::new(),
            ramps: Vec::new(),
            ramp_rows: HashMap::new(),
            acquired: Vec::new(),
        }
    }

    /// Reserve a uniform slot and answer its index.
    pub(super) fn uniform(&mut self, block: [f32; UNIFORM_FLOATS]) -> usize {
        let slot = self.uniforms.len() / UNIFORM_FLOATS;
        self.uniforms.extend_from_slice(&block);
        slot
    }

    /// Append a quad and answer its index range and base vertex.
    pub(super) fn quad(
        &mut self,
        corners: [(f32, f32); 4],
        uv: [(f32, f32); 4],
    ) -> (std::ops::Range<u32>, i32) {
        let base = (self.vertices.len() / 4) as i32;
        for (position, texture) in corners.iter().zip(uv.iter()) {
            self.vertices
                .extend_from_slice(&[position.0, position.1, texture.0, texture.1]);
        }
        let start = self.indices.len() as u32;
        self.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        (start..self.indices.len() as u32, base)
    }

    /// Which row of the frame's ramp texture a resolved gradient occupies.
    pub(super) fn ramp_row(&mut self, ramp: &GradientRamp) -> u32 {
        let texels = ramp.texels().to_vec();
        if let Some(row) = self.ramp_rows.get(&texels) {
            return *row;
        }
        let row = (self.ramps.len() / (RAMP_TEXELS * 4)) as u32;
        self.ramps.extend_from_slice(&texels);
        self.ramp_rows.insert(texels, row);
        row
    }
}

/// The uniform block every draw shares, with everything at rest.
pub(super) fn uniform_block(
    transform: SceneTransform,
    viewport: (u32, u32),
    kind: PaintKind,
    opacity: f32,
) -> [f32; UNIFORM_FLOATS] {
    let mut block = [0.0f32; UNIFORM_FLOATS];
    block[0] = transform.scale_x;
    block[1] = transform.shear_y;
    block[2] = transform.shear_x;
    block[3] = transform.scale_y;
    block[4] = transform.translate_x;
    block[5] = transform.translate_y;
    block[6] = viewport.0 as f32;
    block[7] = viewport.1 as f32;
    block[24] = kind.wire();
    block[25] = opacity;
    // A picture with no adjustments: no brightness shift, unit contrast, no desaturation, opaque.
    block[33] = 1.0;
    block[35] = 1.0;
    block
}

/// Write a premultiplied colour into a uniform block at `offset`.
pub(super) fn write_color(block: &mut [f32; UNIFORM_FLOATS], offset: usize, color: Color) {
    let value = premultiplied(color);
    for (index, channel) in value.iter().enumerate() {
        if let Some(slot) = block.get_mut(offset + index) {
            *slot = *channel;
        }
    }
}

/// Little-endian bytes of a slice of `f32`, without a `bytemuck` dependency.
///
/// R07's hand-off 1 gives the same answer for a mesh's own buffers and for the same reason: the
/// bytes a GPU buffer wants are little-endian on every platform this ships to, and writing the
/// conversion out is four lines against a dependency in the shipped graph.
pub(super) fn le_bytes_f32(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

/// Little-endian bytes of a slice of `u32`.
pub(super) fn le_bytes_u32(values: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

/// The atlas uploader: what turns a glyph atlas's delta into texture writes.
pub(super) struct AtlasUploader<'a> {
    pub(super) device: &'a super::GraphicsDevice,
    pub(super) pages: &'a mut HashMap<u32, (wgpu::Texture, wgpu::TextureView)>,
    pub(super) created: usize,
    pub(super) released: usize,
    pub(super) bytes: usize,
}

impl AtlasVisitor for AtlasUploader<'_> {
    fn page_released(&mut self, page: u32) -> Result<(), PaintError> {
        self.pages.remove(&page);
        self.released += 1;
        Ok(())
    }

    fn page_created(&mut self, page: AtlasPage) -> Result<(), PaintError> {
        let format = match page.format {
            BitmapFormat::Coverage => wgpu::TextureFormat::R8Unorm,
            BitmapFormat::Rgba => wgpu::TextureFormat::Rgba8Unorm,
        };
        let side = u32::from(page.size);
        if side == 0 {
            return Err(PaintError::TextureUnavailable {
                width: side,
                height: side,
                detail: "an atlas page with no area".to_owned(),
            });
        }
        let texture = self
            .device
            .device()
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("mjx-paint glyph atlas"),
                size: wgpu::Extent3d {
                    width: side,
                    height: side,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.pages.insert(page.page, (texture, view));
        self.created += 1;
        Ok(())
    }

    fn write(&mut self, write: AtlasWrite<'_>) -> Result<(), PaintError> {
        let Some((texture, _)) = self.pages.get(&write.page) else {
            // A write for a page nobody created. The atlas reports creations before writes, so this
            // means a painter that lost its textures is being handed a partial delta; saying so is
            // better than writing into whichever page happens to hold that index.
            return Err(PaintError::TextureUnavailable {
                width: u32::from(write.width),
                height: u32::from(write.height),
                detail: format!("no storage for atlas page {}", write.page),
            });
        };
        let stride = u32::from(write.width) * bytes_per_pixel(write.format) as u32;
        if write.width == 0 || write.height == 0 || stride == 0 {
            return Ok(());
        }
        self.device.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: u32::from(write.x),
                    y: u32::from(write.y),
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            write.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(u32::from(write.height)),
            },
            wgpu::Extent3d {
                width: u32::from(write.width),
                height: u32::from(write.height),
                depth_or_array_layers: 1,
            },
        );
        self.bytes += write.pixels.len();
        Ok(())
    }
}

impl WgpuPainter {
    /// Acquire a viewport-sized single-sampled target out of the pool.
    pub(super) fn acquire_target(
        &mut self,
        width: u32,
        height: u32,
        staging: &mut Staging,
    ) -> Result<PoolHandle, PaintError> {
        let device = &self.device;
        let handle = self.pool.acquire(TextureSize::new(width, height), |size| {
            let texture = device.device().create_texture(&wgpu::TextureDescriptor {
                label: Some("mjx-paint layer"),
                size: wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
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
            Ok(texture)
        })?;
        staging.acquired.push(handle);
        Ok(handle)
    }

    /// Make sure every picture the plan names has a texture.
    pub(super) fn upload_images(
        &mut self,
        plan: &crate::plan::FramePlan,
        source: &dyn crate::resources::ImageSource,
    ) -> Result<(), PaintError> {
        for layer in plan.layers() {
            for op in &layer.ops {
                let handle = match op {
                    DrawOp::Picture { paint, .. } | DrawOp::Mesh { paint, .. } => match paint {
                        PaintProgram::Picture { handle, .. } => *handle,
                        _ => continue,
                    },
                    _ => continue,
                };
                if self.images.contains_key(&handle) {
                    continue;
                }
                let Some(pixels) = source.pixels(handle) else {
                    // A handle the caller has no picture for. The area is drawn as nothing rather
                    // than the frame failing: a missing image is a document problem, and refusing
                    // the frame would take the other forty shapes on the page with it.
                    continue;
                };
                if pixels.width == 0 || pixels.height == 0 {
                    continue;
                }
                let uploaded = super::upload_texture(
                    &self.device,
                    "mjx-paint picture",
                    pixels.width,
                    pixels.height,
                    wgpu::TextureFormat::Rgba8Unorm,
                    pixels.rgba,
                )?;
                self.images.insert(handle, uploaded);
            }
        }
        Ok(())
    }
}

/// The corners of a rectangle, clockwise from the top-left.
pub(super) fn rect_corners(rect: SceneRect) -> [(f32, f32); 4] {
    [
        (rect.left, rect.top),
        (rect.right, rect.top),
        (rect.right, rect.bottom),
        (rect.left, rect.bottom),
    ]
}

/// The unit texture coordinates of a quad, in the same order.
pub(super) const UNIT_UV: [(f32, f32); 4] = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];

/// Grow `bounds` to contain a transformed rectangle.
pub(super) fn absorb(bounds: &mut Option<SceneRect>, rect: SceneRect, transform: SceneTransform) {
    for (x, y) in rect_corners(rect) {
        let (x, y) = apply(transform, x, y);
        match bounds {
            Some(existing) => {
                existing.left = existing.left.min(x);
                existing.top = existing.top.min(y);
                existing.right = existing.right.max(x);
                existing.bottom = existing.bottom.max(y);
            }
            None => {
                *bounds = Some(SceneRect::new(x, y, x, y));
            }
        }
    }
}
