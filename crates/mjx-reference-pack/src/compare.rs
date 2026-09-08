//! Comparing two rasters of the same page, one plate at a time.
//!
//! # Why per plate and not per page
//!
//! *"The page differs"* is not a finding. A whole-page diff over 187 shapes says something is wrong
//! and nothing about what, and the caption strip — which our side does not draw at all — would
//! dominate it. Cropping [`PlateGeometry::window`](crate::layout::PlateGeometry::window) out of both
//! rasters turns the answer into one row per preset, which is what MJXOFF-207 asks for: *the
//! preliminary pass reports per shape.*
//!
//! # The two ways this goes vacuous, and what stops each
//!
//! 1. **Two blank crops agree perfectly.** Every plate that draws is asserted to have *ink* on the
//!    reference side before an agreement is believed; a window with no ink is
//!    [`Verdict::NotEvidence`], never an agreement. This is the same guard
//!    `mjx_paint::Agreement::both_drew` exists for, one level up.
//! 2. **A tolerance chosen to make the report green.** [`PLATE_TOLERANCE`] is stated with its
//!    reasoning and is *not* a gate: the preliminary pass **prints** every row and asserts only that
//!    every plate has one. A LibreOffice disagreement is a thing to look at, not a build failure —
//!    treating it as one would be the first step towards editing correct code until it matches a
//!    non-authoritative reference.
//!
//! # What the numbers are
//!
//! Both sides are rasterised by **one** rasteriser (`pdftoppm`) at one DPI, so antialiasing cancels
//! and a remaining difference is real. [`CHANNEL_TOLERANCE`] is `mjx_paint`'s own
//! [`DEFAULT_CHANNEL_TOLERANCE`](mjx_paint::DEFAULT_CHANNEL_TOLERANCE) rather than a second number:
//! the workspace already decided how far apart two eight-bit channels may be and still be the same
//! pixel, and a second answer here would be a second answer to the same question.

use crate::authority::{Baseline, ReferenceProvider, RenderedContent, Verdict};
use crate::layout::Rect;
use crate::plates::Plate;
use crate::tools::{Raster, RASTER_DPI};

/// How far apart two eight-bit channels may be and still count as the same pixel.
///
/// `mjx_paint`'s own figure, not a second one.
pub const CHANNEL_TOLERANCE: u8 = mjx_paint::DEFAULT_CHANNEL_TOLERANCE;

/// What fraction of a plate's window may differ and still be called an agreement.
///
/// **Two percent, and the number is a statement about outlines rather than a tuning knob.** A plate
/// is a 152 × 103 point window at 96 dpi — about 27 800 pixels — of which the shape's own antialiased
/// outline occupies a few hundred: a 1-point stroke around a 100 × 70 box is roughly 460 device
/// pixels of edge, and two renderers can disagree along all of it and half as much again for corner
/// joins and the fill's own edge. Two percent is 556 pixels, which such a disagreement fits inside
/// and which a shape drawn in the wrong place, at the wrong size, or not at all does not: any of
/// those moves *areas*, and the smallest plate on the sheet is 7 000 pixels of fill.
///
/// It is a reporting threshold and nothing gates on it. See this module's documentation.
pub const PLATE_TOLERANCE: f64 = 0.02;

/// The part of one raster inside `window`.
#[must_use]
pub fn crop(raster: &Raster, window: Rect) -> Raster {
    let pixels = window.to_pixels(RASTER_DPI, raster.width, raster.height);
    let mut rgb = Vec::with_capacity(pixels.area() * 3);
    for y in pixels.y..pixels.y + pixels.height {
        for x in pixels.x..pixels.x + pixels.width {
            rgb.extend_from_slice(&raster.pixel(x, y).unwrap_or([0xff, 0xff, 0xff]));
        }
    }
    Raster {
        width: pixels.width,
        height: pixels.height,
        rgb,
    }
}

/// How much two crops of the same window agree.
#[derive(Clone, PartialEq, Debug)]
pub struct WindowAgreement {
    /// How many pixels were compared.
    pub pixels: usize,
    /// How many are further apart than [`CHANNEL_TOLERANCE`] on some channel.
    pub differing: usize,
    /// Where the worst difference is, and what each side put there.
    pub worst: Option<(u32, u32, [u8; 3], [u8; 3])>,
    /// How many non-white pixels the reference crop has.
    pub reference_ink: usize,
    /// How many our own crop has.
    pub ours_ink: usize,
}

impl WindowAgreement {
    /// What fraction of the compared pixels differ.
    #[must_use]
    pub fn differing_fraction(&self) -> f64 {
        if self.pixels == 0 {
            return 1.0;
        }
        #[allow(
            clippy::cast_precision_loss,
            reason = "a plate window is tens of thousands of pixels"
        )]
        {
            self.differing as f64 / self.pixels as f64
        }
    }

    /// The verdict this agreement supports, before any provider exclusion is applied.
    ///
    /// A window with no ink on the reference side is **not evidence**: two blank crops agree
    /// perfectly, and reporting that as a pass is the oldest way this kind of gate goes hollow.
    #[must_use]
    pub fn verdict(&self, expected_ink: bool) -> Verdict {
        if self.pixels == 0 {
            return Verdict::NotEvidence {
                reason: "the two pages are different sizes, so this window has no pixels in common"
                    .to_owned(),
            };
        }
        if expected_ink && self.reference_ink == 0 {
            return Verdict::NotEvidence {
                reason: format!(
                    "the reference drew nothing in this window ({} pixels, all white), and two \
                     blank crops agree perfectly",
                    self.pixels
                ),
            };
        }
        let fraction = self.differing_fraction();
        if fraction <= PLATE_TOLERANCE {
            return Verdict::Agreed {
                differing_fraction: fraction,
                allowed: PLATE_TOLERANCE,
            };
        }
        Verdict::Disagreed {
            differing_fraction: fraction,
            allowed: PLATE_TOLERANCE,
            worst: match self.worst {
                Some((x, y, reference, ours)) => format!(
                    "at ({x}, {y}) in the window the reference has {reference:02x?} and we have \
                     {ours:02x?}; the reference has {} inked pixels and we have {}",
                    self.reference_ink, self.ours_ink
                ),
                None => "the two crops are identical, which contradicts the count".to_owned(),
            },
        }
    }
}

/// Compare one window of two rasters.
#[must_use]
pub fn compare_window(reference: &Raster, ours: &Raster, window: Rect) -> WindowAgreement {
    let left = crop(reference, window);
    let right = crop(ours, window);
    let width = left.width.min(right.width);
    let height = left.height.min(right.height);

    let mut differing = 0usize;
    let mut pixels = 0usize;
    let mut worst: Option<(u32, u32, [u8; 3], [u8; 3])> = None;
    let mut worst_distance = 0u8;

    for y in 0..height {
        for x in 0..width {
            let (Some(a), Some(b)) = (left.pixel(x, y), right.pixel(x, y)) else {
                continue;
            };
            pixels += 1;
            let distance = (0..3).map(|c| a[c].abs_diff(b[c])).max().unwrap_or(0);
            if distance > CHANNEL_TOLERANCE {
                differing += 1;
            }
            if distance > worst_distance {
                worst_distance = distance;
                worst = Some((x, y, a, b));
            }
        }
    }

    WindowAgreement {
        pixels,
        differing,
        worst,
        reference_ink: left.ink(),
        ours_ink: right.ink(),
    }
}

/// The baseline one plate produces, with the provider's exclusions already applied.
///
/// A plate that has nothing to draw on our side — `upArrow`, or a shape at a singular point — never
/// reaches the pixel comparison at all: its own reason is the verdict, because comparing a window
/// PowerPoint filled against a window we deliberately left blank would report a difference that is
/// not a defect on either side.
#[must_use]
pub fn plate_baseline(
    plate: &Plate,
    reference: &Raster,
    ours: &Raster,
    provider: ReferenceProvider,
    artefact: &str,
) -> Baseline {
    let provenance = format!(
        "{artefact}, page {}, plate {}",
        plate.page() + 1,
        plate.index + 1
    );
    if let Some(reason) = plate.kind.not_evidence() {
        return Baseline {
            subject: plate.token.to_owned(),
            provenance,
            provider,
            authority: provider.authority(),
            content: RenderedContent::Outline,
            verdict: Verdict::NotEvidence { reason },
        };
    }
    let agreement = compare_window(reference, ours, plate.geometry().window());
    Baseline::new(
        plate.token,
        provenance,
        provider,
        RenderedContent::Outline,
        agreement.verdict(true),
    )
}
