//! One scene, at one point of the state matrix, all the way to pixels — and the two overlays a
//! person turns on while judging it.
//!
//! # The route, and the fact that there is only one
//!
//! `FragmentTree -> build_scene -> DisplayList -> SoftwarePainter -> Pixels -> png::encode`. It is
//! the client platform's own rendering path below the box model, taken through the **software**
//! painter — pure Rust, no GPU, no window, no display server — so the audit surface and its plate
//! gate both run on a machine with nothing installed. Nothing here re-walks a display list:
//! `SoftwarePainter` consumes `mjx_paint::plan_frame`'s lowering like every other painter, which is
//! `CLAUDE.md`'s *"one lowering, four painters, and no painter re-walks a display list"*.
//!
//! # ⚠ Why a scene is materialised twice when the hit-test overlay is on
//!
//! MJXOFF-166: *"the hit-test visualiser is proved against the spatial index, not against a second
//! region computation."* A grab region is therefore
//! [`mjx_layout::SpatialIndex::bounds_of`] inflated by the input device's padding — and the index
//! has to be built from the scene **without** its own overlays, or the translucent rectangle would
//! be the topmost fragment at its own centre and the visualiser would be measuring itself. So
//! [`overlaid`] builds the base tree, indexes it, and only then adds the overlay nodes to a copy of
//! the canvas.

use mjx_layout::{FragmentTree, LayoutPoint, LayoutSize, SpatialIndex};
use mjx_ooxml_core::measure::Emu;
use mjx_paint::{DrawReport, NoGlyphs, NoImages, Pixels, Resources, SoftwarePainter};
use mjx_render_oracle::png::{self, Image};
use mjx_scene::{build_scene, Command, DashPattern, DisplayList, SceneOptions};
use mjx_text::{GlyphAtlas, GlyphRasteriser};
use mjx_tokens::Tokens;

use crate::canvas::{self, Canvas, Rect, PAGE, STAGE};
use crate::inventory::{Draws, Entry};
use crate::state::State;

/// Which overlays are drawn on top of a scene.
///
/// **They are not part of the element.** A plate is taken with both off, because a plate of an
/// element plus a debugging overlay is a plate of the overlay; they exist for the person sitting in
/// front of the harness, which is the instrument the inventory says is the audit surface.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Overlays {
    /// Draw every grab region as a translucent rectangle over the element it grabs.
    pub hit_test: bool,
    /// Draw the ruler down the stage's top and left edges.
    pub ruler: bool,
}

impl Overlays {
    /// Neither — what a plate is taken with.
    pub const NONE: Self = Self {
        hit_test: false,
        ruler: false,
    };
}

/// One scene, ready to be rendered or overlaid.
#[derive(Clone, PartialEq, Debug)]
pub struct Scene {
    /// Which inventory element.
    pub entry: &'static Entry,
    /// At which point of the state matrix.
    pub state: State,
    /// What was drawn.
    pub canvas: Canvas,
}

impl Scene {
    /// Build the scene for `entry` at `state`, with the palette `tokens` gives.
    #[must_use]
    pub fn build(entry: &'static Entry, tokens: &Tokens, state: State) -> Self {
        let mut canvas = Canvas::new(tokens, state);
        crate::scenes::draw(entry, &mut canvas);
        Self {
            entry,
            state,
            canvas,
        }
    }

    /// The page, in EMU, as the scene builder wants it.
    #[must_use]
    pub fn page(&self) -> LayoutSize {
        LayoutSize::new(Emu::from_points(STAGE.0), Emu::from_points(STAGE.1))
    }

    /// How many device pixels the stage becomes at this scene's density.
    #[must_use]
    pub fn device_size(&self) -> (u32, u32) {
        let scale = f64::from(self.canvas.pixels_per_point());
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the stage is 300 by 200 points and the largest density is 4 pixels a point"
        )]
        {
            (
                (STAGE.0 * scale).round() as u32,
                (STAGE.1 * scale).round() as u32,
            )
        }
    }
}

/// The scene's canvas with `overlays` added — see this module's own documentation for why the tree
/// is built twice.
#[must_use]
pub fn overlaid(scene: &Scene, overlays: Overlays) -> Canvas {
    if overlays == Overlays::NONE {
        return scene.canvas.clone();
    }
    let mut canvas = scene.canvas.clone();
    let scale = canvas.pixels_per_point();
    let ink = canvas.ink().clone();

    if overlays.hit_test {
        // **The index of the scene as it stands, before a single overlay node exists.**
        let (tree, ids) = scene.canvas.tree();
        let index = SpatialIndex::build(&tree);
        for grab in scene.canvas.grabs() {
            let Some(region) = grab_rectangle(&index, ids[grab.node.index()], grab.padding) else {
                continue;
            };
            canvas.rect(
                None,
                region,
                canvas::filled(canvas::with_alpha(ink.accent(), 0x2a)),
            );
            canvas.rect(
                None,
                region,
                canvas::dashed(scale, ink.accent(), 0.75, DashPattern::Dot),
            );
        }
    }

    if overlays.ruler {
        let rule = canvas::with_alpha(ink.muted(), 0xb0);
        for tick in 0..=((STAGE.0 / 10.0) as i32) {
            let x = f64::from(tick) * 10.0;
            let length = if tick % 5 == 0 { 7.0 } else { 3.5 };
            canvas.line(
                None,
                canvas::pt(x, 0.0),
                canvas::pt(x, length),
                "ruler tick",
                canvas::stroke(scale, rule, 0.5),
            );
        }
        for tick in 0..=((STAGE.1 / 10.0) as i32) {
            let y = f64::from(tick) * 10.0;
            let length = if tick % 5 == 0 { 7.0 } else { 3.5 };
            canvas.line(
                None,
                canvas::pt(0.0, y),
                canvas::pt(length, y),
                "ruler tick",
                canvas::stroke(scale, rule, 0.5),
            );
        }
        // The page's own rectangle, so the ruler says where the page is and not only where the
        // stage is: every measurement a reviewer takes off this overlay is a measurement inside a
        // document.
        canvas.rect(
            None,
            PAGE,
            canvas::dashed(
                scale,
                canvas::with_alpha(ink.muted(), 0x80),
                0.5,
                DashPattern::Dash,
            ),
        );
    }

    canvas
}

/// One grab region, in points: the fragment's own bounds out of the index, inflated.
fn grab_rectangle(
    index: &SpatialIndex,
    fragment: mjx_layout::FragmentId,
    padding: f64,
) -> Option<Rect> {
    let bounds = index.bounds_of(fragment)?;
    Some(
        Rect::new(
            bounds.origin().x.points(),
            bounds.origin().y.points(),
            bounds.width().points(),
            bounds.height().points(),
        )
        .inflated(padding),
    )
}

/// Where a grab region's centre is, for the assertion that the index answers with the fragment the
/// visualiser drew the region for.
#[must_use]
pub fn grab_centre(index: &SpatialIndex, fragment: mjx_layout::FragmentId) -> Option<LayoutPoint> {
    let bounds = index.bounds_of(fragment)?;
    Some(LayoutPoint::new(
        Emu::from_emu(bounds.origin().x.emu() + bounds.width().emu() / 2),
        Emu::from_emu(bounds.origin().y.emu() + bounds.height().emu() / 2),
    ))
}

/// Everything one render produced.
#[derive(Clone, PartialEq, Debug)]
pub struct Rendered {
    /// The tree that was drawn — tier one's subject, and what the spatial index is built from.
    pub tree: FragmentTree,
    /// The display list — tier two's subject, and what the [`Draws`] gate reads.
    pub list: DisplayList,
    /// The pixels, **premultiplied**, as the painter hands them back.
    pub pixels: Pixels,
    /// The same pixels as a straight-alpha image, ready to encode. See
    /// [`mjx_render_oracle::png`] for the whole of that decision.
    pub image: Image,
    /// What the draw did — `placeholders` and `draw_calls` both.
    pub report: DrawReport,
    /// How many pixels are not fully transparent.
    pub covered: usize,
    /// How many distinct colours the image holds — **the fourth counter**, and the one that refuses
    /// an element drawn in the page's own ink.
    pub distinct_colours: usize,
}

/// Render one canvas through the software painter.
///
/// # Errors
///
/// Whatever the scene builder or the painter fails with, as a sentence naming the element.
pub fn render(scene: &Scene, overlays: Overlays) -> Result<Rendered, String> {
    let canvas = overlaid(scene, overlays);
    let (tree, _) = canvas.tree();
    let mut rasteriser = GlyphRasteriser::new();
    let mut atlas = GlyphAtlas::new();
    let mut options = SceneOptions::new(scene.page());
    options.device_scale = scene.state.density.device_scale();
    let list = build_scene(&tree, &canvas, &mut rasteriser, &mut atlas, &options)
        .map_err(|error| format!("entry {}: building the scene: {error}", scene.entry.number))?;

    let (width, height) = scene.device_size();

    let mut painter = SoftwarePainter::new();
    let mut glyphs = NoGlyphs;
    let images = NoImages;
    let mut resources = Resources::new(&mut glyphs, &canvas, &images);
    let render =
        mjx_paint::render_offscreen(&mut painter, &list, width, height, 1.0, &mut resources)
            .map_err(|error| format!("entry {}: painting: {error}", scene.entry.number))?;

    let image = png::image_from_pixels(&render.pixels);
    let covered = image.covered();
    let distinct_colours = render.pixels.distinct_colors();
    Ok(Rendered {
        tree,
        list,
        pixels: render.pixels,
        image,
        report: render.drawn,
        covered,
        distinct_colours,
    })
}

/// The rendered scene as PNG bytes.
///
/// # Errors
///
/// Whatever the painter or `mjx_render_oracle::png::encode` fails with, as a sentence.
pub fn render_png(scene: &Scene, overlays: Overlays) -> Result<(Rendered, Vec<u8>), String> {
    let rendered = render(scene, overlays)?;
    let bytes = png::encode(&rendered.image)?;
    Ok((rendered, bytes))
}

/// One scene's display list, without painting it.
///
/// Tier two on its own. A gate that only asks about the *commands* — which stroke widths a scene
/// carries, which kinds of drawing it contains — has no business rasterising a 1200 × 800 frame to
/// find out, and `tests/the_axes_are_not_identities.rs` reads three densities per entry.
///
/// # Errors
///
/// Whatever the scene builder rejects, as a sentence naming the element.
pub fn display_list(scene: &Scene, overlays: Overlays) -> Result<DisplayList, String> {
    let canvas = overlaid(scene, overlays);
    let (tree, _) = canvas.tree();
    let mut rasteriser = GlyphRasteriser::new();
    let mut atlas = GlyphAtlas::new();
    let mut options = SceneOptions::new(scene.page());
    options.device_scale = scene.state.density.device_scale();
    build_scene(&tree, &canvas, &mut rasteriser, &mut atlas, &options)
        .map_err(|error| format!("entry {}: building the scene: {error}", scene.entry.number))
}

/// The display list of the **bare stage** — a backdrop and a page, and no element at all.
///
/// The floor `tests/every_entry_draws.rs` holds every entry to. It is a function rather than a
/// constant because it depends on the tokens in force, and it is here rather than in the test
/// because the test would otherwise have to reassemble the pipeline by hand and could drift from
/// the one [`render`] uses — at which point the floor would be measuring a different thing from the
/// number it is compared against.
///
/// # Errors
///
/// Whatever the scene builder rejects, as a sentence.
pub fn bare_stage_list(tokens: &Tokens) -> Result<DisplayList, String> {
    let mut canvas = Canvas::new(tokens, State::CANONICAL);
    crate::scenes::bare_stage(&mut canvas);
    let (tree, _) = canvas.tree();
    let mut rasteriser = GlyphRasteriser::new();
    let mut atlas = GlyphAtlas::new();
    let options = SceneOptions::new(LayoutSize::new(
        Emu::from_points(STAGE.0),
        Emu::from_points(STAGE.1),
    ));
    build_scene(&tree, &canvas, &mut rasteriser, &mut atlas, &options)
        .map_err(|error| format!("building the bare stage: {error}"))
}

/// Which [`Draws`] kinds a display list actually contains.
///
/// Read off the commands rather than off the image, so a scene that lost its stroke fails naming a
/// missing command instead of naming a pixel nobody can attribute.
#[must_use]
pub fn kinds_drawn(list: &DisplayList) -> Vec<Draws> {
    let mut kinds = Vec::new();
    let mut note = |kind: Draws| {
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    };
    for command in list.commands() {
        match command {
            Command::FillPath { .. } => note(Draws::Fill),
            Command::StrokePath { .. } => note(Draws::Stroke),
            Command::PushClip(_) => note(Draws::Clip),
            Command::PushOpacity(_) => note(Draws::Translucency),
            Command::PushEffect(_) => note(Draws::Effect),
            Command::PushTransform(_) => note(Draws::Transform),
            Command::DrawGlyphs { .. } | Command::DrawImage { .. } | Command::Pop => {}
        }
    }
    kinds.sort_unstable();
    kinds
}
