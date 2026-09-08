//! The pages the oracle asserts over, and the two perturbations that prove its tiers localise.
//!
//! # A specimen crosses the seam exactly once
//!
//! Each specimen is a [`FragmentTree`] built by hand, a palette that answers the handles in it, and
//! nothing else. From there it goes `FragmentTree -> build_scene -> DisplayList -> SoftwarePainter
//! -> Pixels`, which is the whole of the client platform's rendering path below the box model. That
//! single route is what makes three *independent* tiers possible: tier one reads the tree, tier two
//! reads the list, tier three reads the pixels, and a defect introduced at one stage is invisible to
//! the tiers above it.
//!
//! **The trees are built rather than laid out**, and that is deliberate. No box model for a `.pptx`
//! exists yet — R14 onward — and a baseline whose input came from one would be a baseline of the box
//! model as well as of the renderer, so the day the box model changed, every image would move and
//! nobody would be able to say which half moved.
//!
//! # ⚠ No specimen contains text, and that is a decision rather than an omission
//!
//! A golden image containing glyph coverage is a golden image of **a rasteriser version and an
//! installed face**. `mjx-text` rasterises through `swash`, and a patch release of it that changed
//! one pixel of one stem would expire every approval in this crate for a reason that has nothing to
//! do with fidelity. So the pixel tier is font-free, and text is measured where it is exactly
//! meaningful and exactly stable: the **layout tier**, which asks `pdftotext -bbox-layout` where the
//! words landed in a PDF we exported. That is the tier MJXOFF-165 calls the strong one, it is
//! rasteriser-independent by construction, and no exclusion touches it. See [`crate::pdf`].
//!
//! # The two perturbations
//!
//! [`Perturbation::FragmentPosition`] moves one fragment. It is visible in all three tiers, because
//! a rectangle is carried from the tree into the list into the pixels.
//!
//! [`Perturbation::PaintColour`] changes one answer the **palette** gives. The fragment tree does not
//! carry colours — a `BoxFragment` carries a [`DecorationRef`], a bare number — so tier one cannot
//! see it and tiers two and three can. That asymmetry is the entire proof that the tiers are
//! independent rather than three views of one comparison, and
//! `tests/the_tiers_fail_independently.rs` is where it is asserted.

use mjx_dml::geometry::{GeometryGuideList, PresetGeometry};
use mjx_geometry::{PresetGeometryProvider, PresetShapeType, ShapeOutline, Size};
use mjx_layout::{
    BoxFragment, DecorationRef, Fragment, FragmentTree, FragmentTreeBuilder, GeometryRef, ImageRef,
    LayoutRect, LayoutSize, PartId, ShapeFragment, SourcePath, SourceRef,
};
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_core::Interner;
use mjx_paint::{
    DrawReport, NoGlyphs, NoImages, OffscreenSurface, PaintError, Painter, PdfPainter, Pixels,
    Resources, SoftwarePainter, Viewport,
};
use mjx_scene::{
    build_scene, Color, Decoration, DisplayList, FillStyle, Gradient, GradientStop, Image,
    ResourceResolver, SceneOptions, StrokeStyle,
};
use mjx_text::{GlyphAtlas, GlyphRasteriser};

use crate::authority::RenderedContent;
use crate::perceptual::Tolerance;

/// Every specimen's page, in points. One page size for the whole corpus, so a plate's dimensions are
/// never the thing that differs between two baselines.
pub const PAGE: (i64, i64) = (240, 160);

/// The handle the preset specimen's shape is registered under.
///
/// Deliberately not `0`, and deliberately not the fragment's index. A provider that ignored the
/// registry and answered from a table in order would still draw a plausible shape; with a scattered
/// handle it draws nothing and `DrawReport::placeholders` says so.
pub const PRESET_HANDLE: u64 = 0x0165_0000_0000_002b;

/// Which preset the geometry specimen draws.
///
/// A five-pointed star: it has ten vertices, no straight run longer than a few points, and a
/// singular case at neither of its two adjustments — so a shape drawn from a stand-in rectangle, a
/// shape drawn at the wrong size and a shape drawn not at all are three visibly different images
/// rather than three grey squares.
pub const PRESET: PresetShapeType = PresetShapeType::FivePointStar;

/// One page the oracle holds a baseline for.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Specimen {
    /// A stable, filesystem-safe name. It is the baseline directory's name and the plate's name in
    /// the manifest R11's gallery loads, so it may not change without expiring an approval.
    pub name: &'static str,
    /// What the page is for, in one sentence, printed in the gallery.
    pub description: &'static str,
    /// What kind of drawing it is — **the axis the provider exclusions are declared along**. A
    /// specimen whose content is [`RenderedContent::GradientFill`] cannot be compared against a
    /// LibreOffice reference, and this is the field that says so.
    pub content: RenderedContent,
    /// How far its pixel tier may differ before the difference is a finding.
    pub tolerance: Tolerance,
}

impl Specimen {
    /// The page, as the scene builder wants it.
    #[must_use]
    pub fn page(&self) -> LayoutSize {
        #[allow(
            clippy::cast_precision_loss,
            reason = "240 and 160 are exact in f64 and in f32"
        )]
        LayoutSize::new(
            Emu::from_points(PAGE.0 as f64),
            Emu::from_points(PAGE.1 as f64),
        )
    }

    /// The specimen with that name, or `None`.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        SPECIMENS.iter().copied().find(|s| s.name == name)
    }
}

/// **The corpus.** Five pages, and the set is chosen so that no gate in this crate can be satisfied
/// by measuring one thing five times.
///
/// * Three different [`RenderedContent`] kinds, so the provider exclusion has something to exclude
///   **and** something to leave alone. A corpus that was all gradients would make the exclusion
///   vacuous; one that was all solid fills would make it unexercised.
/// * A page with no stroke and a page with one, because `DrawReport::placeholders` is incremented in
///   two places and a corpus whose every shape is filled proves one of them — which is exactly what
///   MJXOFF-206 found to be true of this workspace.
/// * A page whose commands nest and pop, so the display list is not five `FillPath`s in a row.
pub const SPECIMENS: &[Specimen] = &[
    Specimen {
        name: "solid-panels",
        description: "Three solid-filled boxes at different positions, with no stroke and no \
                      nesting — the simplest page on which all three tiers can disagree.",
        content: RenderedContent::SolidFill,
        tolerance: Tolerance::EXACT_RASTER,
    },
    Specimen {
        name: "stroked-frames",
        description: "The same boxes filled and stroked, so that the stroke half of every counter \
                      in the pipeline executes.",
        content: RenderedContent::SolidFill,
        tolerance: Tolerance::EXACT_RASTER,
    },
    Specimen {
        name: "preset-star",
        description: "A five-pointed star resolved through `mjx-geometry`'s own provider, filled \
                      and stroked. The page on which `placeholders == 0` means something.",
        content: RenderedContent::Outline,
        tolerance: Tolerance::EXACT_RASTER,
    },
    Specimen {
        name: "gradient-panel",
        description: "A linear gradient across the page. Excluded from any LibreOffice pixel \
                      comparison — by the provider, not by this list.",
        content: RenderedContent::GradientFill,
        tolerance: Tolerance::EXACT_RASTER,
    },
    Specimen {
        name: "nested-groups",
        description: "A clipped, half-opaque group inside another, so the command stream pushes \
                      and pops rather than being a row of fills.",
        content: RenderedContent::SolidFill,
        tolerance: Tolerance::EXACT_RASTER,
    },
];

/// What was deliberately changed before a comparison was run.
///
/// Not a debugging aid: `tests/the_tiers_fail_independently.rs` is the gate MJXOFF-165 states in its
/// *Done when*, and these are its two inputs.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Perturbation {
    /// Nothing. What a baseline is taken and checked at.
    None,
    /// One fragment moved eight points to the right. Visible to **all three** tiers.
    FragmentPosition,
    /// One decoration answered in a different colour. Invisible to tier one, visible to tiers two
    /// and three — because a fragment tree carries handles and not colours.
    PaintColour,
}

impl Perturbation {
    /// Every perturbation, so a sweep cannot miss one.
    pub const ALL: [Self; 3] = [Self::None, Self::FragmentPosition, Self::PaintColour];

    /// How it is named in a report.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "unperturbed",
            Self::FragmentPosition => "one fragment moved",
            Self::PaintColour => "one paint recoloured",
        }
    }
}

/// The colour the palette answers with when it has not been perturbed.
const INK: Color = Color {
    red: 0x24,
    green: 0x4a,
    blue: 0x8f,
    alpha: 0xff,
};

/// The colour [`Perturbation::PaintColour`] substitutes. Far enough from [`INK`] that a channel
/// tolerance cannot swallow it, and a different hue rather than a different lightness, so a
/// comparison that only looked at luminance would still see it.
const PERTURBED_INK: Color = Color {
    red: 0xa8,
    green: 0x3c,
    blue: 0x1e,
    alpha: 0xff,
};

/// A rectangle in points.
fn rect(x: f64, y: f64, width: f64, height: f64) -> LayoutRect {
    LayoutRect::from_edges(
        Emu::from_points(x),
        Emu::from_points(y),
        Emu::from_points(x + width),
        Emu::from_points(y + height),
    )
}

/// A source reference for the `index`-th thing on the page.
fn source(index: u32) -> SourceRef {
    SourceRef::node(PartId::new(1), SourcePath::new(&[index]))
}

/// The fragment tree for `specimen`, with `perturbation` applied.
///
/// # Panics
///
/// Never on any input this crate offers: the builder refuses only a parent it did not mint and a
/// tree of more than `u32::MAX` nodes, and every push below names a parent it has just been given.
#[must_use]
pub fn fragments(specimen: &Specimen, perturbation: Perturbation) -> FragmentTree {
    let nudge = if perturbation == Perturbation::FragmentPosition {
        8.0
    } else {
        0.0
    };
    let mut builder = FragmentTreeBuilder::new();
    let page = builder
        .push_simple(
            None,
            source(0),
            rect(0.0, 0.0, PAGE.0 as f64, PAGE.1 as f64),
            Fragment::Box(BoxFragment {
                decoration: Some(DecorationRef::new(DECORATION_PAPER)),
                cell: None,
            }),
        )
        .expect("a root fragment");

    match specimen.name {
        "solid-panels" | "stroked-frames" => {
            let decoration = if specimen.name == "stroked-frames" {
                DECORATION_STROKED
            } else {
                DECORATION_INK
            };
            for (index, (x, y)) in [(20.0, 24.0), (96.0, 56.0), (168.0, 92.0)]
                .into_iter()
                .enumerate()
            {
                // Only the **first** panel moves. A perturbation that moved everything would be
                // indistinguishable from a change of page size, and a diff of it would name the
                // whole page rather than one fragment.
                let offset = if index == 0 { nudge } else { 0.0 };
                builder
                    .push_simple(
                        Some(page),
                        source(index as u32 + 1),
                        rect(x + offset, y, 52.0, 40.0),
                        Fragment::Box(BoxFragment {
                            decoration: Some(DecorationRef::new(decoration)),
                            cell: None,
                        }),
                    )
                    .expect("a panel");
            }
        }
        "preset-star" => {
            builder
                .push_simple(
                    Some(page),
                    source(1),
                    rect(60.0 + nudge, 20.0, 120.0, 120.0),
                    Fragment::Shape(ShapeFragment {
                        geometry: GeometryRef::new(PRESET_HANDLE),
                        decoration: Some(DecorationRef::new(DECORATION_STROKED)),
                    }),
                )
                .expect("the star");
        }
        "gradient-panel" => {
            builder
                .push_simple(
                    Some(page),
                    source(1),
                    rect(24.0 + nudge, 24.0, 192.0, 112.0),
                    Fragment::Box(BoxFragment {
                        decoration: Some(DecorationRef::new(DECORATION_GRADIENT)),
                        cell: None,
                    }),
                )
                .expect("the gradient panel");
        }
        "nested-groups" => {
            let clip = builder.clip(rect(20.0, 20.0, 140.0, 100.0));
            let outer = builder
                .push(
                    Some(page),
                    source(1),
                    rect(20.0 + nudge, 20.0, 140.0, 100.0),
                    mjx_layout::TransformId::IDENTITY,
                    clip,
                    Fragment::Box(BoxFragment {
                        decoration: Some(DecorationRef::new(DECORATION_TRANSLUCENT)),
                        cell: None,
                    }),
                )
                .expect("the outer group");
            let inner = builder
                .push_simple(
                    Some(outer),
                    source(2),
                    rect(60.0, 50.0, 140.0, 100.0),
                    Fragment::Box(BoxFragment {
                        decoration: Some(DecorationRef::new(DECORATION_INK)),
                        cell: None,
                    }),
                )
                .expect("the inner group");
            builder
                .push_simple(
                    Some(inner),
                    source(3),
                    rect(70.0, 60.0, 40.0, 30.0),
                    Fragment::Box(BoxFragment {
                        decoration: Some(DecorationRef::new(DECORATION_ACCENT)),
                        cell: None,
                    }),
                )
                .expect("the innermost box");
        }
        other => panic!("`{other}` is not a specimen this crate knows"),
    }
    builder.finish()
}

/// The paper the page is drawn on.
const DECORATION_PAPER: u64 = 1;
/// One flat colour, no outline.
const DECORATION_INK: u64 = 2;
/// The same colour with a two-point outline.
const DECORATION_STROKED: u64 = 3;
/// A linear gradient across the shape.
const DECORATION_GRADIENT: u64 = 4;
/// A flat colour at half opacity, which is what makes the nested page nest.
const DECORATION_TRANSLUCENT: u64 = 5;
/// A colour nothing else on any page uses, with an outline.
///
/// **It exists because the first version of the nested page did not have it**, and the innermost box
/// was drawn in the same ink as the group containing it — so it was invisible, and a change to it
/// would have moved no pixel. That is R09's own hand-off 14 in a different costume: a fixture whose
/// effect is not visible is a fixture that proves the code ran and nothing about what it did. The
/// nested page is now three distinguishable colours deep.
const DECORATION_ACCENT: u64 = 6;

/// Every handle the palette answers, so a reachability gate can ask for all of them.
pub const DECORATIONS: [u64; 6] = [
    DECORATION_PAPER,
    DECORATION_INK,
    DECORATION_STROKED,
    DECORATION_GRADIENT,
    DECORATION_TRANSLUCENT,
    DECORATION_ACCENT,
];

/// What paints the handles a specimen's fragments carry.
///
/// The **only** place a colour enters the pipeline, which is what makes
/// [`Perturbation::PaintColour`] invisible to tier one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Palette {
    perturbation: Perturbation,
}

impl Palette {
    /// The palette for a run under `perturbation`.
    #[must_use]
    pub const fn new(perturbation: Perturbation) -> Self {
        Self { perturbation }
    }

    /// The ink, which is the one colour the perturbation changes.
    #[must_use]
    pub const fn ink(&self) -> Color {
        match self.perturbation {
            Perturbation::PaintColour => PERTURBED_INK,
            _ => INK,
        }
    }
}

impl ResourceResolver for Palette {
    fn decoration(&self, reference: DecorationRef) -> Option<Decoration> {
        let paper = Color {
            red: 0xff,
            green: 0xff,
            blue: 0xff,
            alpha: 0xff,
        };
        let edge = Color {
            red: 0x10,
            green: 0x14,
            blue: 0x1c,
            alpha: 0xff,
        };
        match reference.number() {
            DECORATION_PAPER => Some(Decoration::filled(FillStyle::Solid(paper))),
            DECORATION_INK => Some(Decoration::filled(FillStyle::Solid(self.ink()))),
            DECORATION_STROKED => Some(Decoration {
                fill: FillStyle::Solid(self.ink()),
                stroke: Some(StrokeStyle::solid(2.0, edge)),
                ..Decoration::none()
            }),
            DECORATION_GRADIENT => Some(Decoration::filled(FillStyle::Gradient(Gradient::linear(
                vec![
                    GradientStop::new(0.0, self.ink()),
                    GradientStop::new(0.5, paper),
                    GradientStop::new(1.0, edge),
                ],
                0.0,
            )))),
            DECORATION_TRANSLUCENT => Some(Decoration {
                fill: FillStyle::Solid(edge),
                opacity: 0.5,
                ..Decoration::none()
            }),
            DECORATION_ACCENT => Some(Decoration {
                fill: FillStyle::Solid(Color {
                    red: 0xe0,
                    green: 0xa8,
                    blue: 0x30,
                    alpha: 0xff,
                }),
                stroke: Some(StrokeStyle::solid(2.0, edge)),
                ..Decoration::none()
            }),
            _ => None,
        }
    }

    fn text_decoration(&self, _source: &SourceRef) -> Option<Decoration> {
        // No specimen contains text; see this module's own documentation for why. Answering `None`
        // rather than a colour keeps that true: a specimen that grew a glyph run would draw it in
        // `mjx_scene::DEFAULT_TEXT_COLOR` and the page would visibly change, rather than quietly
        // acquiring a font dependency.
        None
    }

    fn image(&self, _reference: ImageRef) -> Option<Image> {
        None
    }
}

/// The geometry provider for `specimen` — populated for the preset specimen and empty for the rest.
///
/// # Errors
///
/// A sentence, when the preset's own `a:prstGeom` will not cross the document bridge, which would
/// mean the generated table and `mjx-dml`'s reader disagree about a shape.
pub fn provider(specimen: &Specimen) -> Result<PresetGeometryProvider, String> {
    let mut provider = PresetGeometryProvider::new();
    if specimen.name != "preset-star" {
        return Ok(provider);
    }
    let mut interner = Interner::new();
    let empty = GeometryGuideList::new(&mut interner, Vec::new());
    let document = PresetGeometry::new(&mut interner, PRESET, Some(empty));
    let extents = Size::from_emu(
        120 * crate::geom::EMU_PER_POINT,
        120 * crate::geom::EMU_PER_POINT,
    );
    let outline = ShapeOutline::from_preset_geometry(&document, &interner, extents)
        .ok_or_else(|| format!("`{PRESET:?}` did not cross the document bridge"))?;
    provider.register(PRESET_HANDLE, outline);
    Ok(provider)
}

/// The display list for `specimen` under `perturbation` — **tier two's subject**.
///
/// # Errors
///
/// Whatever the scene builder rejects, as a sentence.
pub fn display_list(
    specimen: &Specimen,
    perturbation: Perturbation,
) -> Result<DisplayList, String> {
    let tree = fragments(specimen, perturbation);
    let palette = Palette::new(perturbation);
    let mut rasteriser = GlyphRasteriser::new();
    let mut atlas = GlyphAtlas::new();
    build_scene(
        &tree,
        &palette,
        &mut rasteriser,
        &mut atlas,
        &SceneOptions::new(specimen.page()),
    )
    .map_err(|error| format!("building `{}`'s scene: {error}", specimen.name))
}

/// How many device pixels a specimen's page becomes, at the unzoomed scale.
///
/// One point is 96/72 pixels, so a 240 × 160 point page is 320 × 213⅓ — and the third of a pixel is
/// why this rounds rather than truncates, and why it is stated once here instead of at three call
/// sites that could round differently.
#[must_use]
pub fn device_size() -> (u32, u32) {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a 240 by 160 point page is 320 by 214 pixels"
    )]
    {
        (
            ((PAGE.0 as f64) * 96.0 / 72.0).round() as u32,
            ((PAGE.1 as f64) * 96.0 / 72.0).round() as u32,
        )
    }
}

/// What one render of a specimen produced.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Rendered {
    /// The pixels, **premultiplied**, as the painter hands them back.
    pub pixels: Pixels,
    /// What the draw did — `placeholders` and `draw_calls` both.
    pub report: DrawReport,
}

/// Render `specimen` through the software painter — **tier three's subject**.
///
/// Through `SoftwarePainter` rather than through whatever is available, because it needs no GPU, no
/// window and no display server, and `SoftwarePainter::new()` answers `Self` rather than `Result`.
/// A baseline that could only be taken on a machine with a graphics stack would be a baseline
/// continuous integration could not check.
///
/// # Errors
///
/// Whatever the painter or the scene builder fails with, as a sentence.
pub fn render(specimen: &Specimen, perturbation: Perturbation) -> Result<Rendered, String> {
    let list = display_list(specimen, perturbation)?;
    let provider = provider(specimen)?;
    let (width, height) = device_size();
    let mut painter = SoftwarePainter::new();
    let mut glyphs = NoGlyphs;
    let images = NoImages;
    let mut host = OffscreenSurface::new(width, height, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = painter
        .begin(&mut host, viewport)
        .map_err(|error| describe(specimen, &error))?;
    let mut resources = Resources::new(&mut glyphs, &provider, &images);
    let report = painter
        .draw(&frame, &list, &mut resources)
        .map_err(|error| describe(specimen, &error))?;
    painter
        .end(frame)
        .map_err(|error| describe(specimen, &error))?;
    let pixels = painter
        .read_pixels()
        .map_err(|error| describe(specimen, &error))?
        .ok_or_else(|| {
            format!(
                "the software painter kept no pixels for `{}`",
                specimen.name
            )
        })?;
    Ok(Rendered { pixels, report })
}

/// The same page as a PDF, which is what the layout and pixel tiers of [`crate::pdf`] compare.
///
/// # Errors
///
/// Whatever the exporter fails with, as a sentence.
pub fn export_pdf(specimen: &Specimen, perturbation: Perturbation) -> Result<Vec<u8>, String> {
    let list = display_list(specimen, perturbation)?;
    let provider = provider(specimen)?;
    let (width, height) = device_size();
    let mut painter = PdfPainter::new();
    let mut glyphs = NoGlyphs;
    let images = NoImages;
    let mut host = OffscreenSurface::new(width, height, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = painter
        .begin(&mut host, viewport)
        .map_err(|error| describe(specimen, &error))?;
    let mut resources = Resources::new(&mut glyphs, &provider, &images);
    painter
        .draw(&frame, &list, &mut resources)
        .map_err(|error| describe(specimen, &error))?;
    painter
        .end(frame)
        .map_err(|error| describe(specimen, &error))?;
    painter.document().map(<[u8]>::to_vec).ok_or_else(|| {
        format!(
            "the PDF exporter finished `{}` with no document",
            specimen.name
        )
    })
}

/// A painter's failure, with the specimen it happened on named.
fn describe(specimen: &Specimen, error: &PaintError) -> String {
    format!("`{}`: {error}", specimen.name)
}
