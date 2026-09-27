//! Rendering one display list through two painters and comparing the results.
//!
//! # Why this is a property of the painters and not of a test
//!
//! **A second painter is the only honest way to test the first.** Two independent implementations
//! agreeing is evidence; one implementation agreeing with itself is not. That is the whole reason
//! MJXOFF-164 required a software painter rather than merely permitting one, and it is why the
//! comparison lives in this crate: R10's fidelity oracle is built on it, R11's plate regression
//! runs it, and a mechanism three children depend on should not be a helper inside one suite's
//! `tests/` directory.
//!
//! # The way this gate goes vacuous, and what stops it
//!
//! MJXOFF-155 §8 names this project's signature defect: *a gate that is green precisely when the
//! work is skipped.* A cross-painter comparison has a particularly sharp instance of it. The GPU
//! painter is unavailable on plenty of machines — no adapter, no driver, no display server — and
//! the obvious recovery is to carry on with the painter that *is* available. At that moment
//! "the painters agree" silently becomes "`tiny-skia` agrees with itself", which is true of every
//! painter that was ever written, including one that draws nothing at all.
//!
//! So [`compare_painters`] **refuses** two painters with the same [`Painter::name`], as
//! [`PaintError::PaintersNotDistinct`], and [`Agreement`] carries both names and both backend
//! reports so that a caller printing its result says *which two* agreed. The refusal is in the
//! library rather than in a suite because a suite can be written without it and this cannot.
//!
//! # What "agree" means when one painter multisamples and the other does not
//!
//! It cannot mean byte equality. `wgpu` rasterises with four-sample multisampling on whatever
//! hardware is present and `tiny-skia` computes analytic coverage; the two answer differently along
//! every antialiased edge on the page, by design, and a comparison that demanded equality would fail
//! on a correct pair and could only be satisfied by making one painter a copy of the other.
//!
//! What it means instead is **agreement away from edges, and bounded disagreement at them**:
//!
//! * [`Agreement::differing`] — how many pixels are further apart than a stated tolerance. An edge
//!   pixel is a handful of levels apart; a shape drawn in the wrong place, in the wrong colour, or
//!   not at all, moves *areas*.
//! * [`Agreement::mean_absolute_difference`] — the average over every channel of every pixel. Edges
//!   are a small fraction of a page, so this stays low unless something structural is wrong.
//! * [`Agreement::worst`] — where the largest disagreement is and what both painters put there,
//!   because *"the painters disagree"* is not a finding and *"at (312, 48) one drew the shadow and
//!   the other drew the shape"* is.
//!
//! # What a disagreement means, and which painter to suspect
//!
//! Usually the GPU one. R08 recorded four approximations in its effect passes and two upstream
//! defects in `mjx-scene`'s flattened stroke path, and four of its seven effect arms — `Glow`,
//! `Reflection`, `SoftEdge` and `InnerShadow` — had never been constructed by any test when this
//! painter was written. **A software painter is the first thing that exercises them**, so a
//! disagreement on a page with one of those on it is more likely to be the GPU painter than the
//! processor.
//!
//! The exception is a dashed or compound stroke: `mjx-scene`'s flattened stroke path loses the cap,
//! the join, the tolerance and the miter limit, and its compound band offset is winding-blind. That
//! is neither painter's fault and both will draw the same wrong thing.

use mjx_scene::DisplayList;

use crate::error::PaintError;
use crate::painter::{BackendReport, DrawReport, FrameReport, Painter, Pixels};
use crate::resources::Resources;
use crate::surface::{OffscreenSurface, SurfaceHost, Viewport};

/// How far apart two eight-bit channels may be and still count as the same pixel.
///
/// Sixteen levels of 255 — about six percent. Chosen against what the two rasterisers actually
/// produce rather than picked round: a multisampled edge quantises coverage to quarters and an
/// analytic one does not, so a pixel exactly half covered can be a full quarter-step apart on
/// **each** channel, and a gradient sampled at slightly different points adds a level or two more.
/// A tolerance below that would report every antialiased edge on every page; one far above it would
/// stop reporting a shape drawn in the wrong colour.
pub const DEFAULT_CHANNEL_TOLERANCE: u8 = 16;

/// What one painter did with one display list.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Render {
    /// Which painter drew it.
    pub painter: &'static str,
    /// What was actually running underneath.
    pub backend: BackendReport,
    /// What the draw did.
    pub drawn: DrawReport,
    /// What the frame did.
    pub frame: FrameReport,
    /// The pixels.
    pub pixels: Pixels,
}

/// Draw one display list through one painter and bring the pixels back.
///
/// # Errors
///
/// Whatever the painter fails with, and [`PaintError::NoPixels`] for a painter that drew a frame
/// and cannot hand its pixels back — a comparison must not treat an absent image as an equal one.
pub fn render_once(
    painter: &mut dyn Painter,
    list: &DisplayList,
    host: &mut dyn SurfaceHost,
    resources: &mut Resources<'_>,
) -> Result<Render, PaintError> {
    let name = painter.name();
    let backend = painter.backend();
    let viewport = Viewport::covering(host);
    let frame = painter.begin(host, viewport)?;
    let drawn = painter.draw(&frame, list, resources)?;
    let report = painter.end(frame)?;
    let Some(pixels) = painter.read_pixels()? else {
        return Err(PaintError::NoPixels { name });
    };
    Ok(Render {
        painter: name,
        backend,
        drawn,
        frame: report,
        pixels,
    })
}

/// The same, onto an offscreen target of a stated size and scale.
///
/// # Errors
///
/// As [`render_once`].
pub fn render_offscreen(
    painter: &mut dyn Painter,
    list: &DisplayList,
    width: u32,
    height: u32,
    scale_factor: f32,
    resources: &mut Resources<'_>,
) -> Result<Render, PaintError> {
    let mut host = OffscreenSurface::new(width, height, scale_factor);
    render_once(painter, list, &mut host, resources)
}

/// How much two painters' renders of the same list agree.
#[derive(Clone, PartialEq, Debug)]
pub struct Agreement {
    /// What the first painter calls itself, and what was running underneath it.
    pub left: (&'static str, BackendReport),
    /// The same for the second.
    pub right: (&'static str, BackendReport),
    /// How wide the compared images are.
    pub width: u32,
    /// How tall.
    pub height: u32,
    /// How many pixels were compared.
    pub pixels: usize,
    /// How many of them are further apart than the tolerance.
    pub differing: usize,
    /// The tolerance they were held to, per channel.
    pub tolerance: u8,
    /// The largest single-channel difference anywhere in the image.
    pub max_channel_difference: u8,
    /// The mean absolute difference across every channel of every pixel, in eight-bit levels.
    pub mean_absolute_difference: f64,
    /// Where the largest disagreement is, and what each painter put there.
    ///
    /// `None` only when the two images are identical. *"The painters disagree"* is not a finding;
    /// *"at (312, 48) one drew `#00000080` and the other drew `#3060c0ff`"* is one somebody can act
    /// on without re-running anything.
    pub worst: Option<Disagreement>,
    /// What each painter reported drawing, so that a comparison which passed because **both**
    /// painters drew nothing is visible rather than green.
    pub drawn: (DrawReport, DrawReport),
}

impl Agreement {
    /// What fraction of the pixels are further apart than the tolerance.
    #[must_use]
    pub fn differing_fraction(&self) -> f64 {
        if self.pixels == 0 {
            return 0.0;
        }
        self.differing as f64 / self.pixels as f64
    }

    /// Whether the two agree well enough to be called the same picture.
    ///
    /// `differing_fraction` under `allowed`, which is a fraction and not a count: a page twice the
    /// size has twice the edge pixels, and a bound in pixels would tighten as the page grew.
    #[must_use]
    pub fn within(&self, allowed: f64) -> bool {
        self.differing_fraction() <= allowed
    }

    /// Whether both painters actually drew something.
    ///
    /// **A comparison of two blank pages agrees perfectly**, which is the second way this gate goes
    /// vacuous after the two-painters-are-one way [`compare_painters`] refuses outright. A caller
    /// asserting agreement should assert this too.
    #[must_use]
    pub fn both_drew(&self) -> bool {
        self.drawn.0.draw_calls > 0 && self.drawn.1.draw_calls > 0
    }
}

/// One pixel where two painters disagree the most.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Disagreement {
    /// Pixels from the left edge.
    pub x: u32,
    /// Pixels from the top.
    pub y: u32,
    /// What the first painter put there, premultiplied `RGBA`.
    pub left: [u8; 4],
    /// What the second put there.
    pub right: [u8; 4],
}

impl core::fmt::Display for Agreement {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "`{}` ({}) against `{}` ({}) over {}x{}: {}/{} pixels differ by more than {} levels \
             ({:.4}%), worst {} levels, mean {:.3}",
            self.left.0,
            self.left.1,
            self.right.0,
            self.right.1,
            self.width,
            self.height,
            self.differing,
            self.pixels,
            self.tolerance,
            self.differing_fraction() * 100.0,
            self.max_channel_difference,
            self.mean_absolute_difference
        )?;
        if let Some(worst) = self.worst {
            write!(
                formatter,
                ", at ({}, {}) {:02x?} against {:02x?}",
                worst.x, worst.y, worst.left, worst.right
            )?;
        }
        Ok(())
    }
}

/// Compare two renders of the same list.
///
/// # Errors
///
/// [`PaintError::PaintersNotDistinct`] if both renders came from the same painter — see this
/// module's documentation for why that is refused rather than allowed — and
/// [`PaintError::IncomparableRenders`] for two images of different sizes.
pub fn compare_renders(
    left: &Render,
    right: &Render,
    tolerance: u8,
) -> Result<Agreement, PaintError> {
    if left.painter == right.painter {
        return Err(PaintError::PaintersNotDistinct { name: left.painter });
    }
    if left.pixels.width != right.pixels.width || left.pixels.height != right.pixels.height {
        return Err(PaintError::IncomparableRenders {
            left_width: left.pixels.width,
            left_height: left.pixels.height,
            right_width: right.pixels.width,
            right_height: right.pixels.height,
        });
    }

    let width = left.pixels.width;
    let height = left.pixels.height;
    let mut differing = 0usize;
    let mut compared = 0usize;
    let mut max_difference = 0u8;
    let mut total_difference = 0u64;
    let mut worst: Option<Disagreement> = None;

    for y in 0..height {
        for x in 0..width {
            let (Some(a), Some(b)) = (left.pixels.pixel(x, y), right.pixels.pixel(x, y)) else {
                continue;
            };
            compared += 1;
            let mut pixel_max = 0u8;
            for channel in 0..4 {
                let difference = a[channel].abs_diff(b[channel]);
                total_difference += u64::from(difference);
                pixel_max = pixel_max.max(difference);
            }
            if pixel_max > tolerance {
                differing += 1;
            }
            if pixel_max > max_difference {
                max_difference = pixel_max;
                worst = Some(Disagreement {
                    x,
                    y,
                    left: a,
                    right: b,
                });
            }
        }
    }

    let mean = if compared == 0 {
        0.0
    } else {
        total_difference as f64 / (compared as f64 * 4.0)
    };

    Ok(Agreement {
        left: (left.painter, left.backend.clone()),
        right: (right.painter, right.backend.clone()),
        width,
        height,
        pixels: compared,
        differing,
        tolerance,
        max_channel_difference: max_difference,
        mean_absolute_difference: mean,
        worst,
        drawn: (left.drawn, right.drawn),
    })
}

/// Where a painter's resources come from, once per painter.
///
/// # Why each painter needs its own, and why that is a trait rather than an argument
///
/// An [`crate::AtlasSource`] is a **taking** delta: it reports what changed since it was last asked
/// and then forgets. Hand one source to two painters and the second is told nothing changed, so it
/// draws a page whose text has no pixels — and the comparison then reports a large, entirely
/// artificial disagreement over every word on the page.
///
/// A `Resources` borrows three things mutably and immutably for a lifetime the caller owns, which a
/// plain `FnMut(&mut Resources)` cannot express portably. This trait is the smallest shape that
/// can: the implementor builds a fresh set inside its own stack frame and hands it to the painter.
pub trait ResourceFactory {
    /// Build one painter's resources and run `render` against them.
    ///
    /// # Errors
    ///
    /// Whatever `render` fails with.
    fn render(
        &mut self,
        render: &mut dyn FnMut(&mut Resources<'_>) -> Result<Render, PaintError>,
    ) -> Result<Render, PaintError>;
}

/// Render one display list through two painters and compare the results.
///
/// # Errors
///
/// [`PaintError::PaintersNotDistinct`] if the two are the same painter — checked **before** either
/// is asked to draw, so a caller that passed one painter twice is told what is wrong rather than
/// told how well it agrees with itself — whatever either painter fails with, and
/// [`PaintError::IncomparableRenders`] if they answer with different sizes.
#[allow(
    clippy::too_many_arguments,
    reason = "one comparison's whole description; splitting it would hide the tolerance"
)]
pub fn compare_painters(
    left: &mut dyn Painter,
    right: &mut dyn Painter,
    list: &DisplayList,
    width: u32,
    height: u32,
    scale_factor: f32,
    tolerance: u8,
    left_resources: &mut dyn ResourceFactory,
    right_resources: &mut dyn ResourceFactory,
) -> Result<Agreement, PaintError> {
    if left.name() == right.name() {
        return Err(PaintError::PaintersNotDistinct { name: left.name() });
    }
    let drawn_left = left_resources.render(&mut |resources| {
        render_offscreen(left, list, width, height, scale_factor, resources)
    })?;
    let drawn_right = right_resources.render(&mut |resources| {
        render_offscreen(right, list, width, height, scale_factor, resources)
    })?;
    compare_renders(&drawn_left, &drawn_right, tolerance)
}
