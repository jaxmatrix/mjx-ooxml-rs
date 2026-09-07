//! The PDF exporter: vector output, with **real text** in it.
//!
//! # The requirement that decided the whole design
//!
//! MJXOFF-164 is explicit: *text as embedded subset fonts with real glyph positioning, not
//! rasterised images*. That single line rules out three easier designs — rasterising the page,
//! drawing glyph outlines as paths, and embedding a full face per run — and it is what makes the
//! difference between a PDF somebody can search, select and copy out of, and a PDF that is a picture
//! of a page.
//!
//! So a run of glyphs becomes what a PDF reader expects:
//!
//! * a **`/Type0` font** with `/Encoding /Identity-H`, so a two-byte code *is* a glyph id and no
//!   encoding table stands between the display list's glyph numbers and the file's;
//! * a **`/CIDFontType2` descendant** whose `/CIDToGIDMap` is `/Identity`, so the ids are the face's
//!   own — which is exactly what [`mjx_text::subset_truetype`] preserves by truncating rather than
//!   renumbering;
//! * a **`/FontFile2`** holding the subset, so the file carries the glyphs it draws and nothing
//!   else;
//! * and a **`/ToUnicode` CMap**, which is the part that decides whether the text is text. A reader
//!   extracting from an Identity-H font has no way back to characters without it, and a page that
//!   omits one is vector art that happens to be shaped like words. It is built from
//!   `mjx_text::FaceReader::for_each_mapped_character` — the face's own `cmap`, read backwards.
//!
//! `tests/a_document_is_a_document.rs` asserts this with `pdftotext`, an **independent** reader.
//! Rasterising the export with this workspace's own code and comparing would be self-referential:
//! two of our own components agreeing says nothing about whether a PDF reader can read the file.
//!
//! # What PDF expresses natively, and the one thing it cannot
//!
//! Natively: paths with either fill rule, strokes with width, cap, join, miter limit and preset
//! dashes, clipping, **group opacity** (a transparency-group form XObject drawn under an
//! `ExtGState`'s `ca`, which is what makes a layer one object rather than a rasterised copy of
//! itself), axial and radial **shadings** built from the same 256-texel ramp both rasterisers
//! sample, **tiling patterns** built from the same fifty-four hatch masks, and images.
//!
//! **Not natively: blur.** PDF has no blur operator at all — there is no filter model in the
//! imaging model, and the transparency extensions add blend modes and soft masks but nothing that
//! convolves. So every effect that involves a blur is **rasterised**, through the software painter,
//! and embedded as an image XObject covering the effect's own area. That is the documented fallback
//! MJXOFF-164 asks for, and it is why this exporter holds a [`crate::SoftwarePainter`]: the pure-Rust
//! path to pixels is not only for tests.
//!
//! A fill overlay, which is a tint in a blend mode, needs no blur and is drawn natively.
//!
//! # Text filled with a gradient
//!
//! PDF has an idiom for it and it is exact: **text render mode 7** adds the glyphs to the clipping
//! path and draws nothing, so the fill is then painted through the letters. That is what a
//! [`crate::LayerKind::Mask`] becomes here, and it is the one place this exporter is *better* than
//! the two rasterisers rather than merely equal — a reader can still select the text.
//!
//! # Nothing here panics
//!
//! A PDF is written into a byte vector with no fallible indexing and no unwrap, and every failure —
//! a face that cannot be embedded, a page with no content — is a typed [`PaintError::Export`].

use std::collections::BTreeMap;
use std::collections::HashMap;

use mjx_scene::{
    Color, DisplayList, EffectKind, FillRule, LineCap, LineJoin, MeshRole, SceneRect,
    SceneTransform, Tessellator,
};

use crate::error::PaintError;
use crate::painter::{
    AdapterKind, Antialiasing, BackendReport, Capabilities, DrawReport, Frame, FrameReport,
    GraphicsApi, Painter, Pixels,
};
use crate::plan::{plan_frame_with, DrawOp, FramePlan, LayerKind, PaintProgram, PlanOptions};
use crate::resources::{EmbeddableFace, Resources};
use crate::software::SoftwarePainter;
use crate::surface::{SurfaceHost, Viewport};

use super::{matrix, number, opacity, pdf_path_operators};

/// The name this exporter reports.
pub const PDF_EXPORTER: &str = "pdf";

/// How many samples a shading's colour function carries.
///
/// The resolved ramp's own length. A PDF type-0 sampled function takes the samples verbatim, so
/// this is not an approximation of the ramp — it *is* the ramp, the same 256 texels the fragment
/// shader and the software painter read.
pub const SHADING_SAMPLES: usize = crate::RAMP_TEXELS;

/// One object in the file, before it is given a number.
struct Object {
    /// The body, already serialised.
    body: Vec<u8>,
}

/// A PDF being written.
///
/// Objects are appended and numbered from one; the cross-reference table is built at the end from
/// where each landed. Written by hand rather than through a crate because a PDF writer is a
/// serialiser and this workspace writes its own serialisers — see `CLAUDE.md` on hand-written
/// de/serialisation — and because the only alternative crates carry their own font handling, which
/// would be the second font reader MJXOFF-164 forbids.
#[derive(Default)]
struct Writer {
    objects: Vec<Object>,
}

impl Writer {
    /// Reserve an object number without writing it yet.
    ///
    /// Needed because a page names its contents and its resources, and its resources name fonts that
    /// are not written until the page's text is known. Every reservation is filled before
    /// [`Writer::finish`], and one that was not would be an empty object rather than a dangling
    /// reference — which a reader survives and a missing object does not.
    fn reserve(&mut self) -> usize {
        self.objects.push(Object {
            body: b"<< >>".to_vec(),
        });
        self.objects.len()
    }

    /// Fill a reserved object.
    fn fill(&mut self, number: usize, body: Vec<u8>) {
        if let Some(slot) = self.objects.get_mut(number.saturating_sub(1)) {
            slot.body = body;
        }
    }

    /// Append an object and answer its number.
    fn add(&mut self, body: Vec<u8>) -> usize {
        let number = self.reserve();
        self.fill(number, body);
        number
    }

    /// A stream object: a dictionary, then the bytes.
    fn add_stream(&mut self, dictionary: &str, data: &[u8]) -> usize {
        let mut body = Vec::with_capacity(dictionary.len() + data.len() + 64);
        body.extend_from_slice(
            format!("<< {dictionary} /Length {} >>\nstream\n", data.len()).as_bytes(),
        );
        body.extend_from_slice(data);
        body.extend_from_slice(b"\nendstream");
        self.add(body)
    }

    /// The whole file.
    fn finish(&self, root: usize) -> Vec<u8> {
        let mut file = Vec::new();
        // A binary comment in the second line tells every tool that treats PDFs as text that this
        // one is not, which is what stops a transport helpfully rewriting its line endings.
        file.extend_from_slice(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n");
        let mut offsets = Vec::with_capacity(self.objects.len());
        for (index, object) in self.objects.iter().enumerate() {
            offsets.push(file.len());
            file.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
            file.extend_from_slice(&object.body);
            file.extend_from_slice(b"\nendobj\n");
        }
        let start = file.len();
        file.extend_from_slice(format!("xref\n0 {}\n", self.objects.len() + 1).as_bytes());
        file.extend_from_slice(b"0000000000 65535 f \n");
        for offset in &offsets {
            file.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        file.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root {root} 0 R >>\nstartxref\n{start}\n%%EOF\n",
                self.objects.len() + 1
            )
            .as_bytes(),
        );
        file
    }
}

/// A face this document embeds, and the glyphs it was asked for.
struct EmbeddedFont {
    /// The object number of the `/Type0` font.
    object: usize,
    /// The resource name, `/F1` and so on.
    name: String,
    /// Which glyphs the page drew, ascending.
    glyphs: std::collections::BTreeSet<u16>,
    /// The face, once it is resolved.
    face: Option<EmbeddableFace>,
    /// Glyph to character, for the `/ToUnicode` map.
    characters: BTreeMap<u16, char>,
    /// Glyph to advance, in font units, for the `/W` array.
    widths: BTreeMap<u16, u16>,
}

/// A frame in progress.
struct OpenFrame {
    id: u64,
    width: u32,
    height: u32,
    report: DrawReport,
    content: String,
    resources: PageResources,
}

/// What a page's `/Resources` dictionary will hold.
#[derive(Default)]
struct PageResources {
    /// `/ExtGState` entries, by name.
    graphics_states: Vec<(String, String)>,
    /// `/XObject` entries: name and object number.
    xobjects: Vec<(String, usize)>,
    /// `/Pattern` entries: name and object number.
    patterns: Vec<(String, usize)>,
    /// `/Shading` entries: name and object number.
    shadings: Vec<(String, usize)>,
    /// `/Font` entries: name and object number.
    fonts: Vec<(String, usize)>,
}

impl PageResources {
    fn dictionary(&self) -> String {
        let mut text = String::from("<< /ProcSet [/PDF /Text /ImageC] ");
        let entry = |items: &[(String, usize)]| -> String {
            items
                .iter()
                .map(|(name, number)| format!("/{name} {number} 0 R "))
                .collect()
        };
        if !self.graphics_states.is_empty() {
            text.push_str("/ExtGState << ");
            for (name, body) in &self.graphics_states {
                text.push_str(&format!("/{name} {body} "));
            }
            text.push_str(">> ");
        }
        if !self.xobjects.is_empty() {
            text.push_str(&format!("/XObject << {}>> ", entry(&self.xobjects)));
        }
        if !self.patterns.is_empty() {
            text.push_str(&format!("/Pattern << {}>> ", entry(&self.patterns)));
        }
        if !self.shadings.is_empty() {
            text.push_str(&format!("/Shading << {}>> ", entry(&self.shadings)));
        }
        if !self.fonts.is_empty() {
            text.push_str(&format!("/Font << {}>> ", entry(&self.fonts)));
        }
        text.push_str(">>");
        text
    }
}

/// The PDF exporter.
pub struct PdfPainter {
    tessellator: Tessellator,
    writer: Writer,
    open: Option<OpenFrame>,
    next_frame: u64,
    next_name: u32,
    highlight_placeholders: bool,
    fonts: HashMap<u32, EmbeddedFont>,
    /// The pure-Rust path to pixels, for the effects PDF cannot express.
    raster: SoftwarePainter,
    document: Option<Vec<u8>>,
    last_report: Option<FrameReport>,
}

impl core::fmt::Debug for PdfPainter {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("PdfPainter")
            .field("frame", &self.open.as_ref().map(|frame| frame.id))
            .field("fonts", &self.fonts.len())
            .field("document_bytes", &self.document.as_ref().map(Vec::len))
            .finish()
    }
}

impl Default for PdfPainter {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfPainter {
    /// An exporter with nothing open.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tessellator: Tessellator::new(),
            writer: Writer::default(),
            open: None,
            next_frame: 1,
            next_name: 0,
            highlight_placeholders: false,
            fonts: HashMap::new(),
            raster: SoftwarePainter::new().highlighting_placeholders(false),
            document: None,
            last_report: None,
        }
    }

    /// Whether stand-in geometry is written in [`crate::PLACEHOLDER_WARNING`].
    ///
    /// Off by default, for the reason the SVG exporter's is: an exported document is a deliverable,
    /// and magenta shapes in a customer's file would be worse than the wrong outline.
    #[must_use]
    pub fn highlighting_placeholders(mut self, highlight: bool) -> Self {
        self.highlight_placeholders = highlight;
        self
    }

    /// The finished document, once [`Painter::end`] has been called.
    #[must_use]
    pub fn document(&self) -> Option<&[u8]> {
        self.document.as_deref()
    }

    /// The last frame's report.
    #[must_use]
    pub fn last_frame(&self) -> Option<FrameReport> {
        self.last_report
    }

    /// A fresh resource name.
    fn resource_name(&mut self, prefix: &str) -> String {
        self.next_name = self.next_name.saturating_add(1);
        format!("{prefix}{}", self.next_name)
    }
}

impl Painter for PdfPainter {
    fn name(&self) -> &'static str {
        PDF_EXPORTER
    }

    fn backend(&self) -> BackendReport {
        BackendReport {
            api: GraphicsApi::None,
            adapter: AdapterKind::Other,
            adapter_name: "PDF document writer".to_owned(),
            driver: String::new(),
            antialiasing: Antialiasing::None,
        }
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            antialiasing: Antialiasing::None,
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
        self.writer = Writer::default();
        self.fonts.clear();
        self.next_name = 0;
        self.document = None;
        let id = self.next_frame;
        self.next_frame = self.next_frame.saturating_add(1);
        // PDF measures from the bottom-left with y upward and this platform measures from the
        // top-left with y downward. One matrix at the head of the content stream reconciles them, so
        // every coordinate below is written in the display list's own device pixels.
        let content = format!("1 0 0 -1 0 {} cm\n", height);
        self.open = Some(OpenFrame {
            id,
            width,
            height,
            report: DrawReport::default(),
            content,
            resources: PageResources::default(),
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
        let plan = plan_frame_with(
            list,
            resources.geometry(),
            &mut self.tessellator,
            PlanOptions::for_vector(),
        )?;
        let report = plan.report();
        let mut content = String::new();
        self.write_layer(&plan, 0, resources, &mut content)?;
        // **The faces are cut here, not in `end`.** A subset is only as small as the glyph set it
        // was given, and the set is not complete until the page's last run is written — so this runs
        // after the walk, while `resources` is still in hand. `end` has no font source: the trait
        // hands one to `draw` and to nothing else.
        for (number, font) in self.fonts.iter_mut() {
            let glyphs: Vec<u16> = font.glyphs.iter().copied().collect();
            font.face = resources.fonts().face(*number, &glyphs);
        }
        if let Some(open) = &mut self.open {
            open.content.push_str(&content);
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

        let mut resources = open.resources;
        // Every font the page drew, written now that its glyph set is known — which is the whole
        // point of subsetting: a face cut before the page is finished is cut to the wrong set.
        let mut fonts: Vec<(u32, EmbeddedFont)> = self.fonts.drain().collect();
        fonts.sort_by_key(|(number, _)| *number);
        for (_, font) in fonts {
            let object = font.object;
            let name = font.name.clone();
            let body = self.write_font(&font);
            self.writer.fill(object, body);
            resources.fonts.push((name, object));
        }

        let contents = self.writer.add_stream("", open.content.as_bytes());
        let pages = self.writer.reserve();
        let page = self.writer.add(
            format!(
                "<< /Type /Page /Parent {pages} 0 R /MediaBox [0 0 {} {}] /Resources {} \
                 /Contents {contents} 0 R >>",
                open.width,
                open.height,
                resources.dictionary()
            )
            .into_bytes(),
        );
        self.writer.fill(
            pages,
            format!("<< /Type /Pages /Kids [{page} 0 R] /Count 1 >>").into_bytes(),
        );
        let catalog = self
            .writer
            .add(format!("<< /Type /Catalog /Pages {pages} 0 R >>").into_bytes());
        self.document = Some(self.writer.finish(catalog));

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
        // A document is not pixels. See `super`'s documentation.
        Ok(None)
    }
}

impl PdfPainter {
    /// Write one layer's operations into a content stream.
    fn write_layer(
        &mut self,
        plan: &FramePlan,
        index: usize,
        resources: &mut Resources<'_>,
        out: &mut String,
    ) -> Result<(), PaintError> {
        let Some(layer) = plan.layer(index) else {
            return Ok(());
        };
        let mut open_clips = 0usize;
        for op in &layer.ops {
            match op {
                DrawOp::Mesh {
                    paint,
                    transform,
                    role,
                    provenance,
                    outline,
                    ..
                } => {
                    let Some(outline) = outline else {
                        continue;
                    };
                    let warning = PaintProgram::Solid(crate::PLACEHOLDER_WARNING);
                    let paint = if self.highlight_placeholders && provenance.is_placeholder() {
                        &warning
                    } else {
                        paint
                    };
                    out.push_str("q\n");
                    push_transform(*transform, out);
                    let bounds = super::outline_bounds(&outline.commands);
                    let operator = self.set_paint(paint, bounds, *role, out)?;
                    if *role == MeshRole::Stroke {
                        if let Some(stroke) = outline.stroke {
                            out.push_str(&format!("{} w\n", number(stroke.width)));
                            out.push_str(&format!("{} J\n", pdf_line_cap(stroke.cap)));
                            out.push_str(&format!("{} j\n", pdf_line_join(stroke.join)));
                            if let LineJoin::Miter { limit } = stroke.join {
                                out.push_str(&format!("{} M\n", number(limit)));
                            }
                            let dashes = mjx_scene::dash_lengths(stroke.dash, stroke.width);
                            if dashes.is_empty() {
                                out.push_str("[] 0 d\n");
                            } else {
                                out.push_str(&format!(
                                    "[{}] 0 d\n",
                                    dashes
                                        .iter()
                                        .map(|length| number(*length))
                                        .collect::<Vec<_>>()
                                        .join(" ")
                                ));
                            }
                        }
                    }
                    out.push_str(&pdf_path_operators(&outline.commands));
                    out.push_str(operator);
                    out.push_str("Q\n");
                }
                DrawOp::Glyphs {
                    quads,
                    transform,
                    paint,
                    run,
                    ..
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
                    self.write_text(
                        quads,
                        *transform,
                        run,
                        colour,
                        TextMode::Fill,
                        resources,
                        out,
                    );
                }
                DrawOp::Picture {
                    destination,
                    transform,
                    paint,
                    ..
                } => {
                    self.write_picture(paint, *destination, *transform, resources, out);
                }
                DrawOp::PushClip {
                    outline, transform, ..
                } => {
                    let Some(outline) = outline else {
                        continue;
                    };
                    out.push_str("q\n");
                    push_transform(*transform, out);
                    out.push_str(&pdf_path_operators(&outline.commands));
                    // `W n` intersects the clip with the path and paints nothing, which is PDF's
                    // whole clipping idiom. The transform is undone straight away so that what
                    // follows is written in the same space everything else is.
                    out.push_str(match outline.fill_rule {
                        FillRule::NonZero => "W n\n",
                        FillRule::EvenOdd => "W* n\n",
                    });
                    let m = matrix(*transform);
                    if let Some(back) = invert_matrix(m) {
                        out.push_str(&format!(
                            "{} {} {} {} {} {} cm\n",
                            number(back[0]),
                            number(back[1]),
                            number(back[2]),
                            number(back[3]),
                            number(back[4]),
                            number(back[5])
                        ));
                    }
                    open_clips += 1;
                }
                DrawOp::PopClip { .. } => {
                    if open_clips > 0 {
                        open_clips -= 1;
                        out.push_str("Q\n");
                    }
                }
                DrawOp::Composite { layer: child, .. } => {
                    self.write_group(plan, *child, resources, out)?;
                }
            }
        }
        for _ in 0..open_clips {
            out.push_str("Q\n");
        }
        Ok(())
    }

    /// Write one child layer, with whatever its kind puts around it.
    fn write_group(
        &mut self,
        plan: &FramePlan,
        index: usize,
        resources: &mut Resources<'_>,
        out: &mut String,
    ) -> Result<(), PaintError> {
        let Some(layer) = plan.layer(index) else {
            return Ok(());
        };
        match &layer.kind {
            LayerKind::Root => self.write_layer(plan, index, resources, out),
            LayerKind::Opacity(alpha) => {
                // **Native group opacity.** A form XObject with a transparency group is composited
                // as one object, so a group of overlapping shapes fades as a group rather than each
                // shape fading against the ones under it — which is what setting `ca` around them
                // inline would do, and is visibly different wherever they overlap.
                let mut inner = String::new();
                self.write_layer(plan, index, resources, &mut inner)?;
                let (Some(width), Some(height)) = (
                    self.open.as_ref().map(|open| open.width),
                    self.open.as_ref().map(|open| open.height),
                ) else {
                    return Ok(());
                };
                let object = self.writer.add_stream(
                    &format!(
                        "/Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 {width} {height}] \
                         /Matrix [1 0 0 1 0 0] /Group << /Type /Group /S /Transparency \
                         /CS /DeviceRGB /I false /K false >> /Resources << /ProcSet [/PDF /Text /ImageC] >>"
                    ),
                    // The group's own content is written in PDF's coordinates, so the page's
                    // top-left flip is re-applied inside it.
                    format!("1 0 0 -1 0 {height} cm\n{inner}").as_bytes(),
                );
                let name = self.resource_name("X");
                let state = self.resource_name("G");
                if let Some(open) = &mut self.open {
                    open.resources.xobjects.push((name.clone(), object));
                    open.resources.graphics_states.push((
                        state.clone(),
                        format!("<< /ca {} /CA {} >>", number(*alpha), number(*alpha)),
                    ));
                }
                // The page's flip is undone around the XObject, because the group re-applies it.
                out.push_str(&format!(
                    "q 1 0 0 -1 0 {height} cm /{state} gs /{name} Do Q\n"
                ));
                Ok(())
            }
            LayerKind::Effect(nodes) => {
                let kind = nodes
                    .last()
                    .map_or(EffectKind::Blur, |node| node.effect.kind);
                if kind == EffectKind::FillOverlay {
                    // A tint in a blend mode. No blur, so no rasterisation: the subtree, then the
                    // colour over it under the node's own blend mode.
                    self.write_layer(plan, index, resources, out)?;
                    return Ok(());
                }
                // **Everything else involves a blur, and PDF has no blur.** The documented fallback:
                // rasterise the layer with its effect applied, through the pure-Rust software
                // painter, and place the result as an image. See this module's documentation.
                self.rasterise_effect(plan, index, resources, out)
            }
            LayerKind::Mask {
                fill,
                bounds,
                transform,
            } => {
                // Text render mode 7: the glyphs are added to the clipping path and nothing is
                // drawn, then the fill is painted through them. **The text is still text** — a
                // reader can select it — which no rasterised alternative achieves.
                out.push_str("q\n");
                let mut inner = String::new();
                self.write_mask_text(plan, index, resources, &mut inner);
                out.push_str(&inner);
                push_transform(*transform, out);
                let operator = self.set_paint(fill, *bounds, MeshRole::Fill, out)?;
                out.push_str(&format!(
                    "{} {} {} {} re\n",
                    number(bounds.left),
                    number(bounds.top),
                    number((bounds.right - bounds.left).max(0.0)),
                    number((bounds.bottom - bounds.top).max(0.0))
                ));
                out.push_str(operator);
                out.push_str("Q\n");
                Ok(())
            }
        }
    }

    /// Write a mask layer's glyph runs in clipping mode.
    fn write_mask_text(
        &mut self,
        plan: &FramePlan,
        index: usize,
        resources: &mut Resources<'_>,
        out: &mut String,
    ) {
        let Some(layer) = plan.layer(index) else {
            return;
        };
        let ops: Vec<_> = layer.ops.to_vec();
        for op in &ops {
            if let DrawOp::Glyphs {
                quads,
                transform,
                run,
                ..
            } = op
            {
                self.write_text(
                    quads,
                    *transform,
                    run,
                    Color {
                        red: 0,
                        green: 0,
                        blue: 0,
                        alpha: 0xff,
                    },
                    TextMode::Clip,
                    resources,
                    out,
                );
            }
        }
    }

    /// Rasterise one effect layer and place it as an image.
    fn rasterise_effect(
        &mut self,
        plan: &FramePlan,
        index: usize,
        resources: &mut Resources<'_>,
        out: &mut String,
    ) -> Result<(), PaintError> {
        let (Some(width), Some(height)) = (
            self.open.as_ref().map(|open| open.width),
            self.open.as_ref().map(|open| open.height),
        ) else {
            return Ok(());
        };
        let Some(pixels) = self
            .raster
            .rasterise_layer(plan, index, width, height, resources)?
        else {
            return Ok(());
        };
        let object = self.image_object(&pixels.rgba, pixels.width, pixels.height, true);
        let name = self.resource_name("X");
        if let Some(open) = &mut self.open {
            open.resources.xobjects.push((name.clone(), object));
        }
        // A PDF image XObject is drawn into the unit square, so the transform is the whole page.
        out.push_str(&format!("q {width} 0 0 {height} 0 0 cm /{name} Do Q\n"));
        Ok(())
    }

    /// Write a run of glyphs as real text.
    #[allow(
        clippy::too_many_arguments,
        reason = "one run's whole description, threaded once"
    )]
    fn write_text(
        &mut self,
        quads: &[crate::plan::GlyphQuad],
        transform: SceneTransform,
        run: &crate::plan::RunIdentity,
        colour: Color,
        mode: TextMode,
        resources: &mut Resources<'_>,
        out: &mut String,
    ) {
        if quads.is_empty() {
            return;
        }
        let fonts = resources.fonts();
        // Registered before anything is written, so that a run whose face the caller has no bytes
        // for produces no text operators at all rather than a reference to a font that will not be
        // written.
        if fonts.face(run.face, &[0]).is_none() {
            return;
        }
        if !self.fonts.contains_key(&run.face) {
            {
                let object = self.writer.reserve();
                let name = self.resource_name("F");
                self.fonts.insert(
                    run.face,
                    EmbeddedFont {
                        object,
                        name,
                        glyphs: std::collections::BTreeSet::new(),
                        face: None,
                        characters: BTreeMap::new(),
                        widths: BTreeMap::new(),
                    },
                );
            }
        }
        let mut characters = BTreeMap::new();
        let mut widths = BTreeMap::new();
        for quad in quads {
            if let Some(character) = fonts.character(run.face, quad.glyph) {
                characters.insert(quad.glyph, character);
            }
            if let Some(advance) = fonts.advance(run.face, quad.glyph) {
                widths.insert(quad.glyph, advance);
            }
        }
        let Some(font) = self.fonts.get_mut(&run.face) else {
            return;
        };
        for quad in quads {
            font.glyphs.insert(quad.glyph);
        }
        font.characters.extend(characters);
        font.widths.extend(widths);
        let name = font.name.clone();

        out.push_str("q\n");
        push_transform(transform, out);
        out.push_str("BT\n");
        out.push_str(match mode {
            // 0 fills the glyphs; 7 adds them to the clipping path and draws nothing, which is how
            // a PDF fills text with a gradient while leaving it selectable.
            TextMode::Fill => "0 Tr\n",
            TextMode::Clip => "7 Tr\n",
        });
        if mode == TextMode::Fill {
            let [red, green, blue] = rgb(colour);
            out.push_str(&format!(
                "{} {} {} rg\n",
                number(red),
                number(green),
                number(blue)
            ));
        }
        out.push_str(&format!("/{name} {} Tf\n", number(run.pixels_per_em)));
        for quad in quads {
            // A glyph's own text matrix, so that positions come straight out of the display list
            // rather than out of an accumulated pen. The vertical flip is the page's, undone for the
            // text and re-applied by the glyph's own position: PDF draws text upward from the
            // baseline and a display list measures downward from the top.
            out.push_str(&format!(
                "1 0 0 -1 {} {} Tm\n",
                number(quad.pen_x),
                number(quad.pen_y)
            ));
            out.push_str(&format!("<{:04x}> Tj\n", quad.glyph));
        }
        out.push_str("ET\n");
        out.push_str("Q\n");
    }

    /// Write a picture as an image XObject.
    fn write_picture(
        &mut self,
        paint: &PaintProgram,
        destination: SceneRect,
        transform: SceneTransform,
        resources: &Resources<'_>,
        out: &mut String,
    ) {
        let PaintProgram::Picture { handle, image, .. } = paint else {
            return;
        };
        let Some(pixels) = resources.images().pixels(*handle) else {
            return;
        };
        if pixels.width == 0 || pixels.height == 0 {
            return;
        }
        let object = self.image_object(pixels.rgba, pixels.width, pixels.height, false);
        let name = self.resource_name("X");
        let alpha = f32::from(image.adjustments.alpha_in_ten_thousandths) / 10_000.0;
        let state = self.resource_name("G");
        if let Some(open) = &mut self.open {
            open.resources.xobjects.push((name.clone(), object));
            open.resources
                .graphics_states
                .push((state.clone(), format!("<< /ca {} >>", number(alpha))));
        }
        let width = (destination.right - destination.left).max(0.0);
        let height = (destination.bottom - destination.top).max(0.0);
        out.push_str("q\n");
        push_transform(transform, out);
        out.push_str(&format!("/{state} gs\n"));
        // An image is drawn into the unit square with its first row at the *top* of that square in
        // PDF's own upward space, so the placement matrix flips it back.
        out.push_str(&format!(
            "{} 0 0 {} {} {} cm\n/{name} Do\nQ\n",
            number(width),
            number(-height),
            number(destination.left),
            number(destination.top)
        ));
    }

    /// An image XObject, with a soft mask for its alpha.
    fn image_object(&mut self, rgba: &[u8], width: u32, height: u32, premultiplied: bool) -> usize {
        let count = (width as usize) * (height as usize);
        let mut colour = Vec::with_capacity(count * 3);
        let mut alpha = Vec::with_capacity(count);
        for pixel in 0..count {
            let at = pixel * 4;
            let a = rgba.get(at + 3).copied().unwrap_or(0);
            let read = |offset: usize| -> u8 {
                let value = rgba.get(at + offset).copied().unwrap_or(0);
                if premultiplied && a > 0 {
                    // A render target holds premultiplied values and a PDF image does not, so the
                    // alpha is divided back out. Doing it the other way round would darken every
                    // partly transparent pixel, which is the same dark halo premultiplied blending
                    // exists to avoid.
                    ((u16::from(value) * 255) / u16::from(a)).min(255) as u8
                } else {
                    value
                }
            };
            colour.push(read(0));
            colour.push(read(1));
            colour.push(read(2));
            alpha.push(a);
        }
        let mask = self.writer.add_stream(
            &format!(
                "/Type /XObject /Subtype /Image /Width {width} /Height {height} \
                 /ColorSpace /DeviceGray /BitsPerComponent 8"
            ),
            &alpha,
        );
        self.writer.add_stream(
            &format!(
                "/Type /XObject /Subtype /Image /Width {width} /Height {height} \
                 /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask {mask} 0 R"
            ),
            &colour,
        )
    }

    /// Set the colour or pattern a shape is painted with, and answer the operator that paints it.
    fn set_paint(
        &mut self,
        paint: &PaintProgram,
        bounds: SceneRect,
        role: MeshRole,
        out: &mut String,
    ) -> Result<&'static str, PaintError> {
        let stroking = role == MeshRole::Stroke;
        match paint {
            PaintProgram::Solid(color) => {
                let [red, green, blue] = rgb(*color);
                out.push_str(&format!(
                    "{} {} {} {}\n",
                    number(red),
                    number(green),
                    number(blue),
                    if stroking { "RG" } else { "rg" }
                ));
                let state = self.resource_name("G");
                let value = number(opacity(*color));
                if let Some(open) = &mut self.open {
                    open.resources
                        .graphics_states
                        .push((state.clone(), format!("<< /ca {value} /CA {value} >>")));
                }
                out.push_str(&format!("/{state} gs\n"));
            }
            PaintProgram::Gradient { ramp, mapping } => {
                let object = self.shading_object(ramp, mapping);
                let pattern = self.writer.add(
                    format!("<< /Type /Pattern /PatternType 2 /Shading {object} 0 R >>")
                        .into_bytes(),
                );
                let name = self.resource_name("P");
                if let Some(open) = &mut self.open {
                    open.resources.patterns.push((name.clone(), pattern));
                }
                out.push_str(&format!(
                    "/Pattern {} /{name} {}\n",
                    if stroking { "CS" } else { "cs" },
                    if stroking { "SCN" } else { "scn" }
                ));
            }
            PaintProgram::Pattern {
                preset,
                foreground,
                background,
            } => {
                let object = self.hatch_object(*preset, *foreground, *background);
                let name = self.resource_name("P");
                if let Some(open) = &mut self.open {
                    open.resources.patterns.push((name.clone(), object));
                }
                out.push_str(&format!(
                    "/Pattern {} /{name} {}\n",
                    if stroking { "CS" } else { "cs" },
                    if stroking { "SCN" } else { "scn" }
                ));
            }
            PaintProgram::Picture { .. } | PaintProgram::Glyphs { .. } => {
                // A shape filled with a picture. PDF's answer is a clip and an image, which is what
                // a `DrawOp::Picture` already writes; the shape itself is left unpainted rather than
                // filled with a colour the document never asked for.
                let _ = bounds;
                out.push_str("1 1 1 rg\n");
            }
        }
        Ok(match (role, paint) {
            (MeshRole::Stroke, _) => "S\n",
            (MeshRole::Fill, _) => "f\n",
        })
    }

    /// A shading object built from the resolved ramp.
    fn shading_object(
        &mut self,
        ramp: &crate::gradient::GradientRamp,
        mapping: &crate::plan::GradientMapping,
    ) -> usize {
        // A type-0 sampled function with the ramp's own texels. **Not an approximation of the
        // ramp** — the same 256 samples the fragment shader and the software painter read, so a
        // gradient in the PDF is the gradient in the render.
        let mut samples = Vec::with_capacity(SHADING_SAMPLES * 3);
        for index in 0..SHADING_SAMPLES {
            let Some([r, g, b, a]) = ramp.texel(index) else {
                continue;
            };
            // Un-premultiplied, because a PDF shading has no alpha channel: the colour is what it
            // is and transparency is a soft mask, which a shading pattern cannot carry.
            let (red, green, blue) = if a == 0 {
                (0, 0, 0)
            } else {
                (
                    ((u16::from(r) * 255) / u16::from(a)).min(255) as u8,
                    ((u16::from(g) * 255) / u16::from(a)).min(255) as u8,
                    ((u16::from(b) * 255) / u16::from(a)).min(255) as u8,
                )
            };
            samples.push(red);
            samples.push(green);
            samples.push(blue);
        }
        let function = self.writer.add_stream(
            &format!(
                "/FunctionType 0 /Domain [0 1] /Range [0 1 0 1 0 1] /Size [{SHADING_SAMPLES}] \
                 /BitsPerSample 8"
            ),
            &samples,
        );
        if mapping.radial {
            let radius = mapping.radii[0].max(mapping.radii[1]).max(0.001);
            self.writer.add(
                format!(
                    "<< /ShadingType 3 /ColorSpace /DeviceRGB /Coords [{} {} 0 {} {} {}] \
                     /Function {function} 0 R /Extend [true true] >>",
                    number(mapping.origin[0]),
                    number(mapping.origin[1]),
                    number(mapping.origin[0]),
                    number(mapping.origin[1]),
                    number(radius)
                )
                .into_bytes(),
            )
        } else {
            let length = mapping.axis[0] * mapping.axis[0] + mapping.axis[1] * mapping.axis[1];
            let scale = if length > 0.0 { 1.0 / length } else { 0.0 };
            self.writer.add(
                format!(
                    "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [{} {} {} {}] \
                     /Function {function} 0 R /Extend [true true] >>",
                    number(mapping.origin[0]),
                    number(mapping.origin[1]),
                    number(mapping.origin[0] + mapping.axis[0] * scale),
                    number(mapping.origin[1] + mapping.axis[1] * scale)
                )
                .into_bytes(),
            )
        }
    }

    /// A tiling pattern built from one of the fifty-four hatch masks.
    fn hatch_object(
        &mut self,
        preset: mjx_scene::PatternPreset,
        foreground: Color,
        background: Color,
    ) -> usize {
        let side = crate::PATTERN_SIDE;
        let mut content = String::new();
        let [red, green, blue] = rgb(background);
        content.push_str(&format!(
            "{} {} {} rg\n0 0 {side} {side} re\nf\n",
            number(red),
            number(green),
            number(blue)
        ));
        let [red, green, blue] = rgb(foreground);
        content.push_str(&format!(
            "{} {} {} rg\n",
            number(red),
            number(green),
            number(blue)
        ));
        // **The same fifty-four masks the two rasterisers and the SVG exporter sample.** A hatch
        // that differed between the render and the export would be the hardest kind of fidelity
        // defect to notice.
        for y in 0..side {
            for x in 0..side {
                if crate::cell_of(preset, x, y) {
                    // The cell's row is measured downward in the mask and upward in a pattern's own
                    // space, which is what `side - 1 - y` is.
                    content.push_str(&format!("{x} {} 1 1 re\n", side - 1 - y));
                }
            }
        }
        content.push_str("f\n");
        self.writer.add_stream(
            &format!(
                "/Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 \
                 /BBox [0 0 {side} {side}] /XStep {side} /YStep {side} \
                 /Resources << /ProcSet [/PDF] >>"
            ),
            content.as_bytes(),
        )
    }

    /// The `/Type0` font object for one embedded face, and everything it references.
    fn write_font(&mut self, font: &EmbeddedFont) -> Vec<u8> {
        let glyphs: Vec<u16> = font.glyphs.iter().copied().collect();
        let Some(face) = font.face.clone() else {
            // A face the caller could not supply. An empty dictionary is a font a reader ignores,
            // which is better than a dangling reference and better than refusing the whole export.
            return b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec();
        };
        let units = f32::from(face.units_per_em.max(1));
        let scale = 1000.0 / units;

        let file = self
            .writer
            .add_stream(&format!("/Length1 {}", face.data.len()), &face.data);
        let descriptor = self.writer.add(
            format!(
                "<< /Type /FontDescriptor /FontName /{name} /Flags {flags} \
                 /FontBBox [{left} {bottom} {right} {top}] /ItalicAngle {angle} /Ascent {ascent} \
                 /Descent {descent} /CapHeight {cap} /StemV 80 /FontFile2 {file} 0 R >>",
                name = face.name,
                // Bit 3 is "symbolic" and bit 6 is "non-symbolic"; a text face is non-symbolic, and
                // bit 1 is fixed pitch and bit 7 italic.
                flags = 32 | u32::from(face.fixed_pitch) | (u32::from(face.italic) << 6),
                left = (f32::from(face.bounding_box[0]) * scale).round() as i32,
                bottom = (f32::from(face.bounding_box[1]) * scale).round() as i32,
                right = (f32::from(face.bounding_box[2]) * scale).round() as i32,
                top = (f32::from(face.bounding_box[3]) * scale).round() as i32,
                angle = number(face.italic_angle),
                ascent = (f32::from(face.ascender) * scale).round() as i32,
                descent = (f32::from(face.descender) * scale).round() as i32,
                cap = (f32::from(face.cap_height) * scale).round() as i32,
            )
            .into_bytes(),
        );

        // The `/W` array: each glyph's advance in thousandths of an em, which is the only unit a PDF
        // font measures in whatever the face's own em square is.
        let mut widths = String::from("[");
        for glyph in &glyphs {
            let advance = font.widths.get(glyph).copied().unwrap_or(0);
            widths.push_str(&format!(
                "{glyph} [{}] ",
                (f32::from(advance) * scale).round() as i32
            ));
        }
        widths.push(']');

        let descendant = self.writer.add(
            format!(
                "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{name} \
                 /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
                 /FontDescriptor {descriptor} 0 R /DW 1000 /W {widths} /CIDToGIDMap /Identity >>",
                name = face.name
            )
            .into_bytes(),
        );

        let to_unicode = self
            .writer
            .add_stream("", &self.to_unicode(font).into_bytes());

        format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /{name} /Encoding /Identity-H \
             /DescendantFonts [{descendant} 0 R] /ToUnicode {to_unicode} 0 R >>",
            name = face.name
        )
        .into_bytes()
    }

    /// The `/ToUnicode` CMap: **what makes the text text**.
    ///
    /// Without it a reader extracting from an `Identity-H` font has glyph ids and no way back to
    /// characters, so a page of perfectly positioned vectors extracts as nothing. `pdftotext` reads
    /// this and only this.
    fn to_unicode(&self, font: &EmbeddedFont) -> String {
        let mut map = String::from(
            "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
             /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
             /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n\
             <0000> <ffff>\nendcodespacerange\n",
        );
        let entries: Vec<(u16, char)> = font
            .characters
            .iter()
            .map(|(glyph, character)| (*glyph, *character))
            .collect();
        // A `bfchar` section may hold at most a hundred entries, which is the format's own limit and
        // not a choice.
        for chunk in entries.chunks(100) {
            map.push_str(&format!("{} beginbfchar\n", chunk.len()));
            for (glyph, character) in chunk {
                let mut utf16 = [0u16; 2];
                let encoded = character.encode_utf16(&mut utf16);
                let hex: String = encoded.iter().map(|unit| format!("{unit:04x}")).collect();
                map.push_str(&format!("<{glyph:04x}> <{hex}>\n"));
            }
            map.push_str("endbfchar\n");
        }
        map.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
        map
    }
}

/// Whether a run of text is filled or added to the clipping path.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TextMode {
    /// Drawn in the run's own colour.
    Fill,
    /// Added to the clipping path and not drawn, so a fill can be painted through it.
    Clip,
}

/// A colour's three channels as fractions.
fn rgb(color: Color) -> [f32; 3] {
    [
        f32::from(color.red) / 255.0,
        f32::from(color.green) / 255.0,
        f32::from(color.blue) / 255.0,
    ]
}

/// Write a `cm` unless the transform is the identity.
fn push_transform(transform: SceneTransform, out: &mut String) {
    if super::is_identity(transform) {
        return;
    }
    let m = matrix(transform);
    out.push_str(&format!(
        "{} {} {} {} {} {} cm\n",
        number(m[0]),
        number(m[1]),
        number(m[2]),
        number(m[3]),
        number(m[4]),
        number(m[5])
    ));
}

/// The inverse of a `cm` matrix, or `None` when it has none.
fn invert_matrix(m: [f32; 6]) -> Option<[f32; 6]> {
    let determinant = m[0] * m[3] - m[2] * m[1];
    if !determinant.is_finite() || determinant.abs() < 1e-12 {
        return None;
    }
    let inverse = 1.0 / determinant;
    let a = m[3] * inverse;
    let b = -m[1] * inverse;
    let c = -m[2] * inverse;
    let d = m[0] * inverse;
    Some([a, b, c, d, -(a * m[4] + c * m[5]), -(b * m[4] + d * m[5])])
}

/// PDF's number for a line cap.
fn pdf_line_cap(cap: LineCap) -> u8 {
    match cap {
        LineCap::Flat => 0,
        LineCap::Round => 1,
        LineCap::Square => 2,
    }
}

/// PDF's number for a line join.
fn pdf_line_join(join: LineJoin) -> u8 {
    match join {
        LineJoin::Miter { .. } => 0,
        LineJoin::Round => 1,
        LineJoin::Bevel => 2,
    }
}
