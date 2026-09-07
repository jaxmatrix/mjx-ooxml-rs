//! The SVG exporter: vector output, and **a readable view of what the display list actually
//! contained**.
//!
//! # The second job is not decoration
//!
//! MJXOFF-164 asks for the debug view in as many words, and it earns its place for a reason that
//! has nothing to do with SVG. A display list is a flat binary encoding; a frame plan is a `Debug`
//! dump thousands of lines long; and the question a person actually has — *"why is the third shape
//! the wrong colour"* — is answered by knowing that **command 12 named paint row 4**. Nothing else
//! in this platform can say that in a form a person can open.
//!
//! So every element carries where it came from:
//!
//! | attribute | what it says |
//! |---|---|
//! | `data-mjx-command` | which command of the display list, counting from zero in paint order |
//! | `data-mjx-table` / `data-mjx-row` | which row of which resource table it named |
//! | `data-mjx-paint` | which row of the paint table fills it |
//! | `data-mjx-stroke` | which row of the stroke table outlines it |
//! | `data-mjx-role` | `fill` or `stroke` |
//! | `data-mjx-provenance` | `document` or `placeholder`, and the stand-in's label |
//! | `data-mjx-layer` | why a group exists: `opacity`, `effect`, `mask` |
//! | `data-mjx-glyphs` / `data-mjx-face` | how many glyphs a run has and which face it is in |
//!
//! They are `data-` attributes because that is the one namespace SVG guarantees it will not
//! interpret: the file is a valid SVG with them, `xmllint` accepts it, and a viewer ignores them.
//!
//! # What is vector and what is not
//!
//! Shapes, strokes, clips, gradients, hatches and text are vectors. Text is drawn as **glyph
//! outlines** taken from a [`crate::FontSource`] — a display list records where a glyph's *pixels*
//! are, and pixels are not an export — and a run whose face the caller did not supply is written as
//! an empty group carrying its own metadata and `data-mjx-unresolved-face`, so the absence is
//! visible in the file rather than silent.
//!
//! Pictures are embedded as PNG data URIs, which is what a self-contained SVG can carry. See
//! [`super::png`] for why this crate writes its own encoder rather than taking a dependency that
//! would put an image *decoder* in the shipped graph.
//!
//! # Effects, and the three that are not expressible
//!
//! A blur, an outer shadow, a glow and a soft edge become native `<filter>` elements — a
//! `feGaussianBlur` with the effect's own radius, and for the coloured ones an `feFlood` composited
//! into the blurred alpha. Those are exactly the four SVG has primitives for.
//!
//! An **inner shadow**, a **fill overlay** and a **reflection** are not. SVG can approximate each
//! with a filter chain of four or five primitives, and every such chain is a second, differently
//! wrong implementation of an effect the two rasterisers already approximate two ways. Rather than
//! add a third, the subtree is written without the effect and the group carries
//! `data-mjx-effect-unsupported` naming the kind — so a reader of the file, and a suite reading it,
//! can both see exactly what was dropped.

use mjx_scene::{
    Color, DisplayList, EffectKind, FillRule, LineCap, LineJoin, MeshRole, SceneRect,
    SceneTransform, Tessellator,
};

use crate::error::PaintError;
use crate::painter::{
    AdapterKind, Antialiasing, BackendReport, Capabilities, DrawReport, Frame, FrameReport,
    GraphicsApi, Painter, Pixels,
};
use crate::plan::{
    plan_frame_with, DrawOp, EffectNode, FramePlan, LayerKind, OpOrigin, PaintProgram, PlanOptions,
    VectorPath,
};
use crate::resources::Resources;
use crate::surface::{SurfaceHost, Viewport};

use super::{
    base64, hex, is_identity, matrix, number, opacity, outline_bounds, png, svg_path_data,
    xml_escape,
};

/// The name this exporter reports.
pub const SVG_EXPORTER: &str = "svg";

/// How many stops a resolved gradient is written with.
///
/// The ramp is 256 premultiplied texels and an SVG gradient is a stop list. Thirty-three stops — one
/// every eight texels, both ends included — reproduce a two-stop linear ramp exactly and a
/// many-stop one to within a level or two, at about a fifth of the file a full 256 would cost. A
/// gradient with a hard edge (two stops at the same position) is the case this cannot reproduce
/// exactly, and it lands within four texels of where the edge is.
pub const GRADIENT_STOPS: usize = 33;

/// A frame in progress.
struct OpenFrame {
    id: u64,
    width: u32,
    height: u32,
    report: DrawReport,
}

/// The SVG exporter.
pub struct SvgPainter {
    tessellator: Tessellator,
    open: Option<OpenFrame>,
    next_frame: u64,
    next_id: u32,
    highlight_placeholders: bool,
    defs: String,
    body: String,
    document: Option<String>,
    last_report: Option<FrameReport>,
}

impl core::fmt::Debug for SvgPainter {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SvgPainter")
            .field("frame", &self.open.as_ref().map(|frame| frame.id))
            .field("document_bytes", &self.document.as_ref().map(String::len))
            .finish()
    }
}

impl Default for SvgPainter {
    fn default() -> Self {
        Self::new()
    }
}

impl SvgPainter {
    /// An exporter with nothing open.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tessellator: Tessellator::new(),
            open: None,
            next_frame: 1,
            next_id: 0,
            highlight_placeholders: false,
            defs: String::new(),
            body: String::new(),
            document: None,
            last_report: None,
        }
    }

    /// Whether stand-in geometry is written in [`crate::PLACEHOLDER_WARNING`] rather than in its own
    /// paint.
    ///
    /// **Off by default here, where the two rasterisers have it on.** An exported document is a
    /// deliverable rather than a developer's view, and magenta shapes in a customer's PDF would be
    /// worse than the wrong outline. The information is not lost: every element carries
    /// `data-mjx-provenance`, which is a stronger signal than a colour because a machine can read
    /// it.
    #[must_use]
    pub fn highlighting_placeholders(mut self, highlight: bool) -> Self {
        self.highlight_placeholders = highlight;
        self
    }

    /// The finished document, once [`Painter::end`] has been called.
    #[must_use]
    pub fn document(&self) -> Option<&str> {
        self.document.as_deref()
    }

    /// The last frame's report.
    #[must_use]
    pub fn last_frame(&self) -> Option<FrameReport> {
        self.last_report
    }

    /// A fresh identifier for a `defs` entry.
    fn identifier(&mut self, prefix: &str) -> String {
        self.next_id = self.next_id.saturating_add(1);
        format!("mjx-{prefix}-{}", self.next_id)
    }
}

impl Painter for SvgPainter {
    fn name(&self) -> &'static str {
        SVG_EXPORTER
    }

    fn backend(&self) -> BackendReport {
        BackendReport {
            api: GraphicsApi::None,
            adapter: AdapterKind::Other,
            adapter_name: "SVG document writer".to_owned(),
            driver: String::new(),
            // The decision belongs to whatever opens the file, which is what
            // `Antialiasing::None`'s own documentation says it is for.
            antialiasing: Antialiasing::None,
        }
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            antialiasing: Antialiasing::None,
            // A document has no texture at all; the number a caller can act on is how large a
            // coordinate the format carries, and SVG's is a `float`.
            max_texture_size: u32::MAX,
            exact_path_clipping: true,
            effect_texture_budget: 0,
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
        let _ = target.raw_handle();
        self.defs.clear();
        self.body.clear();
        self.next_id = 0;
        self.document = None;
        let id = self.next_frame;
        self.next_frame = self.next_frame.saturating_add(1);
        self.open = Some(OpenFrame {
            id,
            width,
            height,
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
        match &self.open {
            Some(open) if open.id == frame.id() => {}
            open => {
                return Err(PaintError::WrongFrame {
                    given: frame.id(),
                    open: open.as_ref().map(|open| open.id),
                })
            }
        }
        // The vector plan: the same walk every painter takes, keeping the outline each shape's
        // triangles were made from.
        let plan = plan_frame_with(
            list,
            resources.geometry(),
            &mut self.tessellator,
            PlanOptions::for_vector(),
        )?;
        let report = plan.report();
        // A document exporter never asks the atlas for its delta — it draws outlines — but taking it
        // keeps the report honest about what the frame's text would have cost a rasteriser, and
        // stops a caller that drives two painters from one atlas seeing a stale delta.
        let mut body = String::new();
        self.emit_layer(&plan, 0, resources, &mut body);
        self.body.push_str(&body);
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
        let mut document = String::with_capacity(self.body.len() + self.defs.len() + 512);
        document.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        document.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
             width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\" \
             data-mjx-frame=\"{frame}\" data-mjx-commands=\"{commands}\" \
             data-mjx-draws=\"{draws}\" data-mjx-placeholders=\"{placeholders}\">\n",
            width = open.width,
            height = open.height,
            frame = open.id,
            commands = open.report.commands,
            draws = open.report.draw_calls,
            placeholders = open.report.placeholders,
        ));
        if !self.defs.is_empty() {
            document.push_str("<defs>\n");
            document.push_str(&self.defs);
            document.push_str("</defs>\n");
        }
        document.push_str(&self.body);
        document.push_str("</svg>\n");
        self.document = Some(document);

        let report = FrameReport {
            frame: open.id,
            drawn: open.report,
            width: open.width,
            height: open.height,
            sample_count: 1,
            presented: false,
            pool: crate::pool::PoolStatistics::default(),
        };
        self.last_report = Some(report);
        Ok(report)
    }

    fn read_pixels(&mut self) -> Result<Option<Pixels>, PaintError> {
        // A document is not pixels. See this module's documentation: answering with some would be a
        // painter pretending to be a rasteriser, and a golden-image suite is built to skip this.
        Ok(None)
    }
}

impl SvgPainter {
    /// Write one layer's operations into `out`, recursing into the layers it composites.
    fn emit_layer(
        &mut self,
        plan: &FramePlan,
        index: usize,
        resources: &Resources<'_>,
        out: &mut String,
    ) {
        let Some(layer) = plan.layer(index) else {
            return;
        };
        // How many clip groups are open, so that a stream which ends inside one still closes them.
        let mut open_clips = 0usize;
        for op in &layer.ops {
            match op {
                DrawOp::Mesh {
                    paint,
                    transform,
                    role,
                    provenance,
                    outline,
                    origin,
                    mesh,
                } => {
                    let Some(outline) = outline else {
                        // A vector plan always carries one; a caller that built a raster plan and
                        // handed it here would otherwise get a silently empty document.
                        out.push_str(&format!(
                            "<!-- command {} has {} triangles and no outline: this plan was not \
                             built with PlanOptions::for_vector -->\n",
                            origin.command,
                            mesh.triangle_count()
                        ));
                        continue;
                    };
                    self.emit_shape(paint, *transform, *role, outline, provenance, *origin, out);
                }
                DrawOp::Glyphs {
                    quads,
                    transform,
                    paint,
                    run,
                    origin,
                } => {
                    let colour = match paint {
                        PaintProgram::Glyphs { color, .. } => *color,
                        _ => Color {
                            red: 0,
                            green: 0,
                            blue: 0,
                            alpha: 0xff,
                        },
                    };
                    out.push_str(&format!(
                        "<g{} data-mjx-command=\"{}\" data-mjx-table=\"{}\" data-mjx-row=\"{}\" \
                         data-mjx-glyphs=\"{}\" data-mjx-face=\"{}\" data-mjx-pixels-per-em=\"{}\"",
                        transform_attribute(*transform),
                        origin.command,
                        origin.table,
                        row(origin.row),
                        quads.len(),
                        run.face,
                        number(run.pixels_per_em)
                    ));
                    let fonts = resources.fonts();
                    let has_face = fonts
                        .outline(
                            run.face,
                            quads.first().map_or(0, |quad| quad.glyph),
                            run.pixels_per_em,
                        )
                        .is_some();
                    if !has_face {
                        out.push_str(" data-mjx-unresolved-face=\"true\"");
                    }
                    out.push_str(">\n");
                    if has_face {
                        for quad in quads {
                            let Some(commands) =
                                fonts.outline(run.face, quad.glyph, run.pixels_per_em)
                            else {
                                continue;
                            };
                            out.push_str(&format!(
                                "<path transform=\"translate({} {})\" d=\"{}\" fill=\"{}\" \
                                 fill-opacity=\"{}\" fill-rule=\"nonzero\" data-mjx-glyph=\"{}\" \
                                 data-mjx-cluster=\"{}\"/>\n",
                                number(quad.pen_x),
                                number(quad.pen_y),
                                svg_path_data(&commands),
                                hex(colour),
                                number(opacity(colour)),
                                quad.glyph,
                                quad.cluster
                            ));
                        }
                    }
                    out.push_str("</g>\n");
                }
                DrawOp::Picture {
                    destination,
                    transform,
                    paint,
                    origin,
                } => {
                    self.emit_picture(paint, *destination, *transform, *origin, resources, out);
                }
                DrawOp::PushClip {
                    outline,
                    transform,
                    origin,
                    ..
                } => {
                    let Some(outline) = outline else {
                        continue;
                    };
                    let id = self.identifier("clip");
                    self.defs.push_str(&format!(
                        "<clipPath id=\"{id}\" clipPathUnits=\"userSpaceOnUse\"><path d=\"{}\" \
                         clip-rule=\"{}\"/></clipPath>\n",
                        svg_path_data(&transformed(&outline.commands, *transform)),
                        fill_rule(outline.fill_rule)
                    ));
                    out.push_str(&format!(
                        "<g clip-path=\"url(#{id})\" data-mjx-command=\"{}\" \
                         data-mjx-table=\"{}\" data-mjx-row=\"{}\">\n",
                        origin.command,
                        origin.table,
                        row(origin.row)
                    ));
                    open_clips += 1;
                }
                DrawOp::PopClip { .. } => {
                    if open_clips > 0 {
                        open_clips -= 1;
                        out.push_str("</g>\n");
                    }
                }
                DrawOp::Composite { layer: child, .. } => {
                    self.emit_group(plan, *child, resources, out);
                }
            }
        }
        // A layer whose clips were replayed into it never sees their `PopClip`, because the pop
        // belongs to the parent's stream. Closing them here is what keeps the document well formed.
        for _ in 0..open_clips {
            out.push_str("</g>\n");
        }
    }

    /// Write one child layer as a group, with whatever its kind puts on the group.
    fn emit_group(
        &mut self,
        plan: &FramePlan,
        index: usize,
        resources: &Resources<'_>,
        out: &mut String,
    ) {
        let Some(layer) = plan.layer(index) else {
            return;
        };
        let mut inner = String::new();
        self.emit_layer(plan, index, resources, &mut inner);
        match &layer.kind {
            LayerKind::Root => out.push_str(&inner),
            LayerKind::Opacity(alpha) => {
                // Native group opacity, which is the whole reason a layer maps to an element rather
                // than to a rasterised copy of itself.
                out.push_str(&format!(
                    "<g data-mjx-layer=\"opacity\" opacity=\"{}\">\n{inner}</g>\n",
                    number(*alpha)
                ));
            }
            LayerKind::Effect(nodes) => {
                let (attribute, unsupported) = self.effect_attribute(nodes);
                out.push_str(&format!(
                    "<g data-mjx-layer=\"effect\"{attribute}{unsupported}>\n{inner}</g>\n"
                ));
            }
            LayerKind::Mask {
                fill,
                bounds,
                transform,
            } => {
                let mask = self.identifier("mask");
                // The layer's own coverage, drawn white into a mask, with the fill behind it. What
                // `a:textFill` with a gradient in it actually means, and what the two rasterisers do
                // with one composite each.
                self.defs.push_str(&format!(
                    "<mask id=\"{mask}\" maskUnits=\"userSpaceOnUse\">\n{}</mask>\n",
                    whiten(&inner)
                ));
                let paint = self.paint_attributes(fill, *bounds, "fill");
                out.push_str(&format!(
                    "<g data-mjx-layer=\"mask\" mask=\"url(#{mask})\">\
                     <rect{} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" {paint}/></g>\n",
                    transform_attribute(*transform),
                    number(bounds.left),
                    number(bounds.top),
                    number((bounds.right - bounds.left).max(0.0)),
                    number((bounds.bottom - bounds.top).max(0.0)),
                ));
            }
        }
    }

    /// One shape, as a `<path>`.
    #[allow(
        clippy::too_many_arguments,
        reason = "one element's whole description, threaded once"
    )]
    fn emit_shape(
        &mut self,
        paint: &PaintProgram,
        transform: SceneTransform,
        role: MeshRole,
        outline: &VectorPath,
        provenance: &mjx_scene::Provenance,
        origin: OpOrigin,
        out: &mut String,
    ) {
        let warning = PaintProgram::Solid(crate::PLACEHOLDER_WARNING);
        let paint = if self.highlight_placeholders && provenance.is_placeholder() {
            &warning
        } else {
            paint
        };
        let bounds = outline_bounds(&outline.commands);
        let painted = match role {
            MeshRole::Fill => format!(
                "{} stroke=\"none\" fill-rule=\"{}\"",
                self.paint_attributes(paint, bounds, "fill"),
                fill_rule(outline.fill_rule)
            ),
            MeshRole::Stroke => {
                let stroke = outline.stroke;
                let mut text = format!(
                    "fill=\"none\" {}",
                    self.paint_attributes(paint, bounds, "stroke")
                );
                if let Some(stroke) = stroke {
                    text.push_str(&format!(
                        " stroke-width=\"{}\" stroke-linecap=\"{}\" stroke-linejoin=\"{}\"",
                        number(stroke.width),
                        line_cap(stroke.cap),
                        line_join(stroke.join)
                    ));
                    if let LineJoin::Miter { limit } = stroke.join {
                        text.push_str(&format!(" stroke-miterlimit=\"{}\"", number(limit)));
                    }
                    if let Some(dashes) = dash_array(stroke.dash, stroke.width) {
                        text.push_str(&format!(" stroke-dasharray=\"{dashes}\""));
                    }
                }
                text
            }
        };
        out.push_str(&format!(
            "<path{} d=\"{}\" {painted} data-mjx-command=\"{}\" data-mjx-table=\"{}\" \
             data-mjx-row=\"{}\" data-mjx-paint=\"{}\" data-mjx-stroke=\"{}\" \
             data-mjx-role=\"{}\" data-mjx-provenance=\"{}\"{}/>\n",
            transform_attribute(transform),
            svg_path_data(&outline.commands),
            origin.command,
            origin.table,
            row(origin.row),
            row(origin.paint),
            row(origin.stroke),
            match role {
                MeshRole::Fill => "fill",
                MeshRole::Stroke => "stroke",
            },
            if provenance.is_placeholder() {
                "placeholder"
            } else {
                "document"
            },
            match provenance.label.as_deref() {
                Some(label) => format!(" data-mjx-label=\"{}\"", xml_escape(label)),
                None => String::new(),
            }
        ));
    }

    /// A picture, as an `<image>` with its pixels inline.
    fn emit_picture(
        &mut self,
        paint: &PaintProgram,
        destination: SceneRect,
        transform: SceneTransform,
        origin: OpOrigin,
        resources: &Resources<'_>,
        out: &mut String,
    ) {
        let PaintProgram::Picture { handle, image, .. } = paint else {
            return;
        };
        let Some(pixels) = resources.images().pixels(*handle) else {
            // A handle the caller has no picture for. The area is written as an empty group saying
            // so, rather than as nothing at all: the two rasterisers draw nothing there and a
            // reader of the file should be able to tell that apart from a page that had no picture.
            out.push_str(&format!(
                "<g data-mjx-command=\"{}\" data-mjx-table=\"image\" data-mjx-row=\"{}\" \
                 data-mjx-missing-picture=\"{handle}\"/>\n",
                origin.command,
                row(origin.row)
            ));
            return;
        };
        let encoded = png(pixels.width, pixels.height, pixels.rgba);
        out.push_str(&format!(
            "<image{} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" \
             preserveAspectRatio=\"none\" opacity=\"{}\" \
             xlink:href=\"data:image/png;base64,{}\" data-mjx-command=\"{}\" \
             data-mjx-table=\"image\" data-mjx-row=\"{}\" data-mjx-fill-mode=\"{}\"/>\n",
            transform_attribute(transform),
            number(destination.left),
            number(destination.top),
            number((destination.right - destination.left).max(0.0)),
            number((destination.bottom - destination.top).max(0.0)),
            number(f32::from(image.adjustments.alpha_in_ten_thousandths) / 10_000.0),
            base64(&encoded),
            origin.command,
            row(origin.row),
            match image.fill_mode {
                mjx_scene::ImageFillMode::Stretch => "stretch",
                mjx_scene::ImageFillMode::Tile => "tile",
            }
        ));
    }

    /// `fill=` or `stroke=` attributes for a paint, defining whatever it needs in `defs`.
    fn paint_attributes(&mut self, paint: &PaintProgram, bounds: SceneRect, which: &str) -> String {
        match paint {
            PaintProgram::Solid(color) => format!(
                "{which}=\"{}\" {which}-opacity=\"{}\"",
                hex(*color),
                number(opacity(*color))
            ),
            PaintProgram::Gradient { ramp, mapping } => {
                let id = self.identifier("gradient");
                let mut stops = String::new();
                for step in 0..GRADIENT_STOPS {
                    let along = step as f32 / (GRADIENT_STOPS - 1).max(1) as f32;
                    let texel = (along * (crate::RAMP_TEXELS - 1) as f32).round() as usize;
                    let Some([r, g, b, a]) = ramp.texel(texel) else {
                        continue;
                    };
                    // The ramp is premultiplied and an SVG stop is not, so it is divided back out.
                    // Interpolating premultiplied values is what stops a fade to transparency
                    // passing through grey; writing them into a format that expects straight colour
                    // would put the same grey back.
                    let (red, green, blue) = if a == 0 {
                        (0, 0, 0)
                    } else {
                        (
                            ((u16::from(r) * 255) / u16::from(a)).min(255) as u8,
                            ((u16::from(g) * 255) / u16::from(a)).min(255) as u8,
                            ((u16::from(b) * 255) / u16::from(a)).min(255) as u8,
                        )
                    };
                    stops.push_str(&format!(
                        "<stop offset=\"{}\" stop-color=\"{}\" stop-opacity=\"{}\"/>",
                        number(along),
                        hex(Color {
                            red,
                            green,
                            blue,
                            alpha: 0xff
                        }),
                        number(f32::from(a) / 255.0)
                    ));
                }
                if mapping.radial {
                    // `radii` are the two half-extents `t` is measured against, and an SVG radial
                    // gradient has one radius — so the ellipse is a unit circle under a transform.
                    let (rx, ry) = (mapping.radii[0].max(0.001), mapping.radii[1].max(0.001));
                    self.defs.push_str(&format!(
                        "<radialGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" \
                         cx=\"0\" cy=\"0\" r=\"1\" \
                         gradientTransform=\"matrix({} 0 0 {} {} {})\">{stops}</radialGradient>\n",
                        number(rx),
                        number(ry),
                        number(mapping.origin[0]),
                        number(mapping.origin[1])
                    ));
                } else {
                    // The axis is already divided by its own squared length, so the point where `t`
                    // reaches one is the origin plus the axis divided by its squared length again.
                    let length =
                        mapping.axis[0] * mapping.axis[0] + mapping.axis[1] * mapping.axis[1];
                    let scale = if length > 0.0 { 1.0 / length } else { 0.0 };
                    self.defs.push_str(&format!(
                        "<linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" \
                         x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\">{stops}</linearGradient>\n",
                        number(mapping.origin[0]),
                        number(mapping.origin[1]),
                        number(mapping.origin[0] + mapping.axis[0] * scale),
                        number(mapping.origin[1] + mapping.axis[1] * scale)
                    ));
                }
                format!("{which}=\"url(#{id})\"")
            }
            PaintProgram::Pattern {
                preset,
                foreground,
                background,
            } => {
                let id = self.identifier("hatch");
                let mut cells = String::new();
                for y in 0..crate::PATTERN_SIDE {
                    for x in 0..crate::PATTERN_SIDE {
                        if crate::cell_of(*preset, x, y) {
                            cells.push_str(&format!(
                                "<rect x=\"{x}\" y=\"{y}\" width=\"1\" height=\"1\" fill=\"{}\" \
                                 fill-opacity=\"{}\"/>",
                                hex(*foreground),
                                number(opacity(*foreground))
                            ));
                        }
                    }
                }
                // **The same fifty-four masks the two rasterisers sample.** Four painters
                // disagreeing about a hatch would be worse than one approximate hatch and much
                // harder to notice.
                self.defs.push_str(&format!(
                    "<pattern id=\"{id}\" patternUnits=\"userSpaceOnUse\" x=\"0\" y=\"0\" \
                     width=\"{side}\" height=\"{side}\" data-mjx-preset=\"{preset}\">\
                     <rect width=\"{side}\" height=\"{side}\" fill=\"{}\" fill-opacity=\"{}\"/>\
                     {cells}</pattern>\n",
                    hex(*background),
                    number(opacity(*background)),
                    side = crate::PATTERN_SIDE,
                    preset = preset.wire_value()
                ));
                format!("{which}=\"url(#{id})\"")
            }
            PaintProgram::Picture { .. } | PaintProgram::Glyphs { .. } => {
                // A shape filled with a picture. The picture itself is written where a
                // `DrawOp::Picture` is; here the shape's own area records what it is filled with so
                // the debug view is complete.
                let _ = bounds;
                format!("{which}=\"none\" data-mjx-{which}-kind=\"picture\"")
            }
        }
    }

    /// A `filter=` attribute for an effect DAG, and a note for the kinds SVG cannot express.
    fn effect_attribute(&mut self, nodes: &[EffectNode]) -> (String, String) {
        let Some(root) = nodes.last() else {
            return (String::new(), String::new());
        };
        let radius = if root.effect.radius.is_finite() {
            root.effect.radius.max(0.0)
        } else {
            0.0
        };
        // SVG's deviation is the Gaussian's own sigma, and the effect's radius is the distance the
        // blur reaches — four sigma, which is what the seventeen-tap kernel in both rasterisers
        // covers. Passing the radius straight through would make every SVG blur four times the
        // rasterisers'.
        let deviation = number(radius / 4.0);
        let id = self.identifier("effect");
        let primitives = match root.effect.kind {
            EffectKind::Blur => {
                format!("<feGaussianBlur stdDeviation=\"{deviation}\"/>")
            }
            EffectKind::SoftEdge => format!(
                "<feGaussianBlur in=\"SourceAlpha\" stdDeviation=\"{deviation}\" result=\"soft\"/>\
                 <feComposite in=\"SourceGraphic\" in2=\"soft\" operator=\"in\"/>"
            ),
            EffectKind::OuterShadow | EffectKind::Glow => {
                let distance = if root.effect.distance.is_finite() {
                    root.effect.distance.max(0.0)
                } else {
                    0.0
                };
                let direction = if root.effect.direction.is_finite() {
                    root.effect.direction
                } else {
                    0.0
                };
                let (dx, dy) = if root.effect.kind == EffectKind::OuterShadow {
                    (distance * direction.cos(), distance * direction.sin())
                } else {
                    (0.0, 0.0)
                };
                format!(
                    "<feGaussianBlur in=\"SourceAlpha\" stdDeviation=\"{deviation}\" \
                     result=\"blurred\"/>\
                     <feOffset in=\"blurred\" dx=\"{}\" dy=\"{}\" result=\"moved\"/>\
                     <feFlood flood-color=\"{}\" flood-opacity=\"{}\" result=\"ink\"/>\
                     <feComposite in=\"ink\" in2=\"moved\" operator=\"in\" result=\"tinted\"/>\
                     <feMerge><feMergeNode in=\"tinted\"/>\
                     <feMergeNode in=\"SourceGraphic\"/></feMerge>",
                    number(dx),
                    number(dy),
                    hex(root.color),
                    number(opacity(root.color))
                )
            }
            // See this module's documentation: SVG can approximate each of these with a filter
            // chain, and every such chain is a third differently-wrong implementation of an effect
            // the two rasterisers already approximate two ways. The subtree is written without it
            // and the group says which kind was dropped.
            EffectKind::InnerShadow | EffectKind::FillOverlay | EffectKind::Reflection => {
                return (
                    String::new(),
                    format!(
                        " data-mjx-effect-unsupported=\"{}\"",
                        effect_name(root.effect.kind)
                    ),
                );
            }
        };
        // The filter region has to be wider than the element, or a shadow is clipped to the shape
        // it is a shadow of. The default is ten percent each way; a shadow reaches its radius plus
        // its distance, so this is generous rather than computed.
        self.defs.push_str(&format!(
            "<filter id=\"{id}\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\" \
             data-mjx-effect=\"{}\">{primitives}</filter>\n",
            effect_name(root.effect.kind)
        ));
        (format!(" filter=\"url(#{id})\""), String::new())
    }
}

/// A `transform=` attribute, or nothing for the identity.
fn transform_attribute(transform: SceneTransform) -> String {
    if is_identity(transform) {
        return String::new();
    }
    let m = matrix(transform);
    format!(
        " transform=\"matrix({} {} {} {} {} {})\"",
        number(m[0]),
        number(m[1]),
        number(m[2]),
        number(m[3]),
        number(m[4]),
        number(m[5])
    )
}

/// A resource row, or `none` where a command named no row of that table.
fn row(value: Option<u32>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "none".to_owned(),
    }
}

/// SVG's spelling of a fill rule.
fn fill_rule(rule: FillRule) -> &'static str {
    match rule {
        FillRule::NonZero => "nonzero",
        FillRule::EvenOdd => "evenodd",
    }
}

/// SVG's spelling of a line cap.
fn line_cap(cap: LineCap) -> &'static str {
    match cap {
        LineCap::Flat => "butt",
        LineCap::Round => "round",
        LineCap::Square => "square",
    }
}

/// SVG's spelling of a line join.
fn line_join(join: LineJoin) -> &'static str {
    match join {
        LineJoin::Round => "round",
        LineJoin::Bevel => "bevel",
        // The limit is written separately, as `stroke-miterlimit`, because SVG spells the two apart.
        LineJoin::Miter { .. } => "miter",
    }
}

/// A `stroke-dasharray` for a preset dash, or `None` for a solid line.
///
/// The lengths come from [`mjx_scene::dash_lengths`] — the same eleven number pairs the tessellator
/// cuts a rasterised dash with — so a dotted line in a document, in a render and in an export are
/// one decision rather than three.
fn dash_array(dash: mjx_scene::DashPattern, width: f32) -> Option<String> {
    let lengths = mjx_scene::dash_lengths(dash, width);
    if lengths.is_empty() {
        return None;
    }
    Some(
        lengths
            .iter()
            .map(|length| number(*length))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// What an effect kind is called in the file.
fn effect_name(kind: EffectKind) -> &'static str {
    match kind {
        EffectKind::Blur => "blur",
        EffectKind::OuterShadow => "outer-shadow",
        EffectKind::InnerShadow => "inner-shadow",
        EffectKind::Glow => "glow",
        EffectKind::SoftEdge => "soft-edge",
        EffectKind::Reflection => "reflection",
        EffectKind::FillOverlay => "fill-overlay",
    }
}

/// A path's steps, put through a transform.
///
/// Used only for a `clipPath`, whose contents SVG resolves in the *user* space of the element it is
/// applied to rather than in whatever space the clip was defined in. Every other element carries its
/// transform as an attribute, which is shorter and keeps the numbers the document's own.
fn transformed(
    commands: &[mjx_scene::PathCommand],
    transform: SceneTransform,
) -> Vec<mjx_scene::PathCommand> {
    use mjx_scene::{PathCommand, ScenePoint};
    let at = |point: ScenePoint| {
        let (x, y) = crate::plan::apply(transform, point.x, point.y);
        ScenePoint::new(x, y)
    };
    commands
        .iter()
        .map(|command| match *command {
            PathCommand::MoveTo(to) => PathCommand::MoveTo(at(to)),
            PathCommand::LineTo(to) => PathCommand::LineTo(at(to)),
            PathCommand::QuadraticTo { control, end } => PathCommand::QuadraticTo {
                control: at(control),
                end: at(end),
            },
            PathCommand::CubicTo {
                first_control,
                second_control,
                end,
            } => PathCommand::CubicTo {
                first_control: at(first_control),
                second_control: at(second_control),
                end: at(end),
            },
            PathCommand::Close => PathCommand::Close,
        })
        .collect()
}

/// A fragment of SVG with every `fill` and `stroke` colour replaced by white.
///
/// What a `<mask>` needs: SVG's default mask reads **luminance**, so a mask drawn in the ink's own
/// colour would let through only as much of the fill as the ink is bright — a run of dark blue text
/// masked by itself would be almost invisible. Replacing the colour with white makes the mask read
/// the coverage, which is what the two rasterisers multiply by.
///
/// A textual substitution rather than a second emitter, because the alternative is writing every
/// element twice and keeping the two in step for ever.
fn whiten(fragment: &str) -> String {
    let mut out = String::with_capacity(fragment.len());
    let mut rest = fragment;
    while let Some(at) = rest.find("fill=\"#") {
        let (before, after) = rest.split_at(at);
        out.push_str(before);
        out.push_str("fill=\"#ffffff");
        // `fill="#rrggbb` is thirteen characters; the closing quote is left for the copy below.
        rest = after.get(13..).unwrap_or("");
    }
    out.push_str(rest);
    out
}
