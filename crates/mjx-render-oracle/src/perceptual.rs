//! Perceptual diffing: **a structural metric and a per-specimen tolerance, never a byte compare.**
//!
//! # Why a byte compare is the wrong instrument, even where it would pass
//!
//! Two renders of the same page by the same rasteriser on the same machine *are* byte-identical, so
//! it is tempting to say so and stop. Three things make that a trap:
//!
//! 1. **The pixel tier's real job is a comparison across rasterisers.** `pdftoppm` rasterises our
//!    PDF and PowerPoint's, one rasteriser for both sides — but the two documents describe their
//!    edges differently, and an antialiased edge one level apart on one channel is not a finding. A
//!    byte compare reports every edge on every page and therefore reports nothing.
//! 2. **A count of differing pixels does not say what kind of difference it is.** A shape moved
//!    three pixels and a shape drawn in a slightly wrong colour can produce the same count. The
//!    structural metric separates them: a positional error destroys local covariance and a tint does
//!    not.
//! 3. **A byte compare has no per-fixture tolerance to state**, so the number that decides a
//!    failure ends up as a constant nobody argued about.
//!
//! So a difference here carries four numbers — a differing fraction, a mean absolute difference, a
//! **mean structural similarity** and the similarity of the **least similar window** — and a
//! [`Tolerance`] states what each specimen may spend of each.
//!
//! The fourth is there because the first three all divide by the whole page, and this tier caught
//! them doing it: a word displaced eight points is 0.85 % of a page and drags the mean similarity
//! from 1.0 to 0.98, both of which a cross-producer tolerance allows. See
//! [`Tolerance::worst_window_similarity`].
//!
//! # What the structural metric is
//!
//! Mean SSIM over the luminance of the two images, in eight-by-eight windows stepped by four, with
//! the constants Wang *et al.* give: `C1 = (0.01 * 255)^2`, `C2 = (0.03 * 255)^2`. Luminance is
//! Rec. 709 over the image **composited on white**, because that is what a reader looking at the
//! plate in R11's gallery sees, and because a difference in alpha alone is already caught by the
//! per-channel pass which looks at all four channels.
//!
//! Eight by eight rather than the eleven-tap Gaussian of the original paper: the windows here are
//! over synthetic plates with hard edges rather than over photographs, the Gaussian's only purpose
//! is to avoid blocking artefacts in a *displayed* SSIM map, and a box window makes the number
//! reproducible in one screenful of arithmetic instead of two.
//!
//! # ⚠ What this does **not** do
//!
//! It does not decide whether a comparison is *evidence*. That is
//! [`ReferenceProvider::excludes`](crate::authority::ReferenceProvider::excludes)'s job and it runs
//! first: a gradient specimen compared against a LibreOffice reference has no perceptual verdict at
//! all, because the reference is wrong in a way no metric can see.

use crate::authority::Verdict;
use crate::png::Image;

/// How far apart two eight-bit channels may be and still count as the same pixel.
///
/// `mjx_paint`'s own figure rather than a second one: the workspace has already decided how far
/// apart two channels may be, and a second answer to the same question is the beginning of two
/// answers that disagree.
pub const CHANNEL_TOLERANCE: u8 = mjx_paint::DEFAULT_CHANNEL_TOLERANCE;

/// What a specimen's pixel tier may spend before a difference is a finding.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Tolerance {
    /// How far apart two channels may be and still count as the same pixel.
    pub channel: u8,
    /// What fraction of pixels may be further apart than that.
    pub differing_fraction: f64,
    /// How structurally similar the two must be **on average**, from `0.0` to `1.0`.
    pub structural_similarity: f64,
    /// How structurally similar the *least similar window* must be.
    ///
    /// # ⚠ Why a mean and a fraction were both not enough, measured rather than reasoned
    ///
    /// The first version of this tier had two numbers, a differing fraction and a mean similarity,
    /// and `tests/the_pdf_tiers_work_on_our_own_exports.rs` **caught them both absorbing a real
    /// defect**: a word displaced eight points on a 480 × 160 page moves about 655 pixels, which is
    /// 0.85 % of the page and inside the two percent a cross-producer comparison is allowed, and it
    /// drags the mean similarity only from 1.0 to 0.98.
    ///
    /// Both numbers have the same denominator problem — *the whole page* — and a local defect
    /// divided by a whole page is small however wrong it is. The minimum over windows has no
    /// denominator: one window that disagrees answers for itself, and a word that moved destroys
    /// the covariance in its own windows and scores near zero. It is what makes this tier able to
    /// see a thing the layout tier would have had to catch alone.
    pub worst_window_similarity: f64,
}

impl Tolerance {
    /// What a baseline taken and checked by **the same rasteriser** is held to.
    ///
    /// One in ten thousand pixels and a similarity of 0.999 — not zero and not one, on purpose. A
    /// tolerance of exactly zero would be a byte compare wearing a metric's clothes, and this tier
    /// has to keep meaning something when the reference is `pdftoppm`'s rasterisation of somebody
    /// else's PDF. What it will *not* absorb is a shape in the wrong place or the wrong colour: the
    /// smallest panel on any specimen is over two thousand pixels, which is two hundred times this.
    pub const EXACT_RASTER: Self = Self {
        channel: CHANNEL_TOLERANCE,
        differing_fraction: 0.000_1,
        structural_similarity: 0.999,
        worst_window_similarity: 0.99,
    };

    /// What a comparison against a PDF rasterised from another producer is held to.
    ///
    /// Two percent of pixels and a similarity of 0.95. The looser number is the honest one for a
    /// tier where both sides came from different document generators: a one-point stroke around a
    /// hundred-point box is a few hundred device pixels of edge, and two producers can disagree
    /// along all of it without either being wrong.
    pub const CROSS_PRODUCER: Self = Self {
        channel: CHANNEL_TOLERANCE,
        differing_fraction: 0.02,
        structural_similarity: 0.95,
        // Six tenths. A window whose content two producers drew in slightly different places still
        // correlates strongly; a window where one of them drew nothing at all does not. This is the
        // number that makes the tier local, and it is the reason the tier can speak about a moved
        // word rather than leaving it entirely to `pdftotext`.
        worst_window_similarity: 0.6,
    };
}

/// One pixel where two images disagree the most.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Worst {
    /// Pixels from the left.
    pub x: u32,
    /// Pixels from the top.
    pub y: u32,
    /// What the first image put there, straight RGBA.
    pub left: [u8; 4],
    /// What the second put there.
    pub right: [u8; 4],
}

/// The least similar window of a comparison, and where it is.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WorstWindow {
    /// Pixels from the left, at the window's top-left corner.
    pub x: u32,
    /// Pixels from the top.
    pub y: u32,
    /// How similar it is, from `0.0` to `1.0`.
    pub similarity: f64,
}

/// How much two images differ, in four numbers that answer different questions.
#[derive(Clone, PartialEq, Debug)]
pub struct Difference {
    /// How many pixels were compared.
    pub pixels: usize,
    /// How many are further apart than the tolerance on some channel.
    pub differing: usize,
    /// The largest single-channel difference anywhere.
    pub max_channel_difference: u8,
    /// The mean absolute difference across every channel of every pixel, in eight-bit levels.
    pub mean_absolute_difference: f64,
    /// Mean structural similarity, `1.0` for identical images.
    pub structural_similarity: f64,
    /// The least similar window, and where it is. `None` only for an image too small to window.
    pub worst_window: Option<WorstWindow>,
    /// Where the largest difference is, and what each side put there. `None` only for identical
    /// images — *"they differ"* is not a finding and *"at (312, 48) one drew `#00000080` and the
    /// other `#3060c0ff`"* is one somebody can act on.
    pub worst: Option<Worst>,
    /// How many pixels of the first image are not fully transparent, and how many of the second.
    ///
    /// **Two blank images agree perfectly**, which is the way this whole family of gate goes
    /// hollow. Carried beside the agreement rather than checked inside it, because whether ink is
    /// expected is the caller's knowledge.
    pub ink: (usize, usize),
}

impl Difference {
    /// What fraction of the compared pixels differ.
    #[must_use]
    pub fn differing_fraction(&self) -> f64 {
        if self.pixels == 0 {
            return 1.0;
        }
        #[allow(
            clippy::cast_precision_loss,
            reason = "a plate is tens of thousands of pixels"
        )]
        {
            self.differing as f64 / self.pixels as f64
        }
    }

    /// Whether both sides drew something.
    #[must_use]
    pub const fn both_drew(&self) -> bool {
        self.ink.0 > 0 && self.ink.1 > 0
    }

    /// The verdict this difference supports, **before** any provider exclusion is applied.
    ///
    /// Three ways to be [`Verdict::NotEvidence`] and they are all the same mistake in different
    /// clothes: nothing to compare, nothing drawn on one side, nothing drawn on either.
    #[must_use]
    pub fn verdict(&self, tolerance: Tolerance, expect_ink: bool) -> Verdict {
        if self.pixels == 0 {
            return Verdict::NotEvidence {
                reason: "the two images have no pixels in common, so nothing was compared"
                    .to_owned(),
            };
        }
        if expect_ink && !self.both_drew() {
            return Verdict::NotEvidence {
                reason: format!(
                    "one side drew nothing ({} and {} covered pixels of {}), and two blank images \
                     agree perfectly",
                    self.ink.0, self.ink.1, self.pixels
                ),
            };
        }
        let fraction = self.differing_fraction();
        let structural = self.structural_similarity >= tolerance.structural_similarity;
        // **The local half.** A fraction and a mean both divide by the whole page, so neither can
        // see a defect that is confined to part of it; this one has no denominator.
        let local = self
            .worst_window
            .is_none_or(|window| window.similarity >= tolerance.worst_window_similarity);
        if fraction <= tolerance.differing_fraction && structural && local {
            return Verdict::Agreed {
                differing_fraction: fraction,
                allowed: tolerance.differing_fraction,
            };
        }
        let local = self.worst_window.map_or_else(String::new, |window| {
            format!(
                "; least similar window at ({}, {}) scores {:.5} (needs {:.5})",
                window.x, window.y, window.similarity, tolerance.worst_window_similarity
            )
        });
        let worst = match self.worst {
            Some(worst) => format!(
                "worst at ({}, {}): {} against {}; mean structural similarity {:.5} (needs \
                 {:.5}){local}",
                worst.x,
                worst.y,
                hex(worst.left),
                hex(worst.right),
                self.structural_similarity,
                tolerance.structural_similarity
            ),
            None => format!(
                "no pixel differs, but mean structural similarity is {:.5} (needs {:.5}){local}",
                self.structural_similarity, tolerance.structural_similarity
            ),
        };
        Verdict::Disagreed {
            differing_fraction: fraction,
            allowed: tolerance.differing_fraction,
            worst,
        }
    }
}

/// `#rrggbbaa`.
fn hex(pixel: [u8; 4]) -> String {
    format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        pixel[0], pixel[1], pixel[2], pixel[3]
    )
}

/// Compare two straight-alpha images.
///
/// # Errors
///
/// A sentence, for two images of different sizes. Refused rather than compared over the overlap: a
/// size change is a finding in itself and reporting it as a pile of differing pixels would bury it.
pub fn compare(left: &Image, right: &Image, tolerance: Tolerance) -> Result<Difference, String> {
    if left.width != right.width || left.height != right.height {
        return Err(format!(
            "a {} by {} image cannot be compared against a {} by {} one",
            left.width, left.height, right.width, right.height
        ));
    }
    let pixels = (left.width as usize) * (left.height as usize);
    let mut differing = 0usize;
    let mut max_channel_difference = 0u8;
    let mut total_absolute = 0u64;
    let mut worst: Option<Worst> = None;
    let mut worst_gap = 0u16;

    for (index, (a, b)) in left
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(right.rgba.as_chunks::<4>().0.iter())
        .enumerate()
    {
        let mut gap = 0u8;
        for channel in 0..4 {
            let difference = a[channel].abs_diff(b[channel]);
            total_absolute += u64::from(difference);
            gap = gap.max(difference);
        }
        max_channel_difference = max_channel_difference.max(gap);
        if gap > tolerance.channel {
            differing += 1;
        }
        if u16::from(gap) > worst_gap {
            worst_gap = u16::from(gap);
            let width = left.width.max(1) as usize;
            #[allow(
                clippy::cast_possible_truncation,
                reason = "an index inside an image whose dimensions are `u32`"
            )]
            {
                worst = Some(Worst {
                    x: (index % width) as u32,
                    y: (index / width) as u32,
                    left: *a,
                    right: *b,
                });
            }
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "a plate is tens of thousands of pixels and the sum fits a f64 exactly"
    )]
    let mean_absolute_difference = if pixels == 0 {
        0.0
    } else {
        total_absolute as f64 / (pixels as f64 * 4.0)
    };

    let (mean, worst_window) = structural_detail(left, right);
    Ok(Difference {
        pixels,
        differing,
        max_channel_difference,
        mean_absolute_difference,
        structural_similarity: mean,
        worst_window,
        worst,
        ink: (left.covered(), right.covered()),
    })
}

/// How wide an SSIM window is.
const WINDOW: usize = 8;

/// How far apart two windows start. Half the window, so every pixel is inside two of them in each
/// axis and a difference at a window boundary cannot fall between the cracks.
const STEP: usize = 4;

/// `(0.01 * 255)^2`, the stabiliser for the luminance term.
const C1: f64 = 6.5025;

/// `(0.03 * 255)^2`, the stabiliser for the contrast and structure terms.
const C2: f64 = 58.5225;

/// Mean SSIM over the luminance of two same-sized images.
///
/// `1.0` for identical images, including two identical *blank* ones — which is why a caller checks
/// [`Difference::both_drew`] rather than believing a similarity of one.
#[must_use]
pub fn structural_similarity(left: &Image, right: &Image) -> f64 {
    structural_detail(left, right).0
}

/// The mean similarity **and the least similar window**, which are two different questions.
///
/// The mean answers *"is this the same picture"*; the minimum answers *"is any part of it a
/// different picture"*. A page is mostly background, so the first is dominated by the parts nothing
/// happened to and the second is not — which is the whole reason both are carried.
#[must_use]
pub fn structural_detail(left: &Image, right: &Image) -> (f64, Option<WorstWindow>) {
    if left.width != right.width || left.height != right.height {
        return (0.0, None);
    }
    let a = luminance(left);
    let b = luminance(right);
    let width = left.width as usize;
    let height = left.height as usize;
    if width < WINDOW || height < WINDOW {
        // Too small to window: compare the whole thing as one window rather than answering an
        // arbitrary number. Every specimen here is hundreds of pixels across, so this is the path a
        // caller with a tiny crop takes.
        return (
            window_similarity(&a, &b, width, height, 0, 0, width, height),
            None,
        );
    }

    let mut total = 0.0f64;
    let mut windows = 0usize;
    let mut worst: Option<WorstWindow> = None;
    let mut y = 0usize;
    while y + WINDOW <= height {
        let mut x = 0usize;
        while x + WINDOW <= width {
            let similarity = window_similarity(&a, &b, width, height, x, y, WINDOW, WINDOW);
            total += similarity;
            windows += 1;
            if worst.is_none_or(|current| similarity < current.similarity) {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "a window origin inside an image whose dimensions are `u32`"
                )]
                {
                    worst = Some(WorstWindow {
                        x: x as u32,
                        y: y as u32,
                        similarity,
                    });
                }
            }
            x += STEP;
        }
        y += STEP;
    }
    if windows == 0 {
        return (1.0, None);
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "a page has thousands of windows, not billions"
    )]
    {
        (total / windows as f64, worst)
    }
}

/// The SSIM of one window.
#[allow(
    clippy::too_many_arguments,
    reason = "one window's whole description; a struct for it would be read once"
)]
fn window_similarity(
    a: &[f64],
    b: &[f64],
    stride: usize,
    height: usize,
    x: usize,
    y: usize,
    width: usize,
    tall: usize,
) -> f64 {
    let mut count = 0.0f64;
    let (mut sum_a, mut sum_b) = (0.0f64, 0.0f64);
    let (mut sum_aa, mut sum_bb, mut sum_ab) = (0.0f64, 0.0f64, 0.0f64);
    for row in y..(y + tall).min(height) {
        for column in x..(x + width).min(stride) {
            let index = row * stride + column;
            let (Some(va), Some(vb)) = (a.get(index), b.get(index)) else {
                continue;
            };
            count += 1.0;
            sum_a += va;
            sum_b += vb;
            sum_aa += va * va;
            sum_bb += vb * vb;
            sum_ab += va * vb;
        }
    }
    if count < 1.0 {
        return 1.0;
    }
    let mean_a = sum_a / count;
    let mean_b = sum_b / count;
    let variance_a = (sum_aa / count) - mean_a * mean_a;
    let variance_b = (sum_bb / count) - mean_b * mean_b;
    let covariance = (sum_ab / count) - mean_a * mean_b;
    let numerator = (2.0 * mean_a * mean_b + C1) * (2.0 * covariance + C2);
    let denominator = (mean_a * mean_a + mean_b * mean_b + C1) * (variance_a + variance_b + C2);
    if denominator == 0.0 {
        return 1.0;
    }
    (numerator / denominator).clamp(-1.0, 1.0)
}

/// Rec. 709 luminance of an image composited on white, one `f64` a pixel.
fn luminance(image: &Image) -> Vec<f64> {
    image
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| {
            let alpha = f64::from(pixel[3]) / 255.0;
            let over_white = |channel: u8| f64::from(channel) * alpha + 255.0 * (1.0 - alpha);
            0.2126 * over_white(pixel[0])
                + 0.7152 * over_white(pixel[1])
                + 0.0722 * over_white(pixel[2])
        })
        .collect()
}

/// A picture of where two images differ: black where they agree, and the disagreement amplified
/// where they do not.
///
/// **A failure a reviewer cannot see is a failure nobody fixes** — MJXOFF-165's own constraint — and
/// a diff image is the difference between a number in a log and a finding. The channel difference is
/// multiplied by four and clamped, so a one-level disagreement is visible rather than being a black
/// image that looks like a pass.
///
/// # Errors
///
/// A sentence, for two images of different sizes.
pub fn diff_image(left: &Image, right: &Image) -> Result<Image, String> {
    if left.width != right.width || left.height != right.height {
        return Err(format!(
            "a {} by {} image cannot be differenced against a {} by {} one",
            left.width, left.height, right.width, right.height
        ));
    }
    let mut rgba = Vec::with_capacity(left.rgba.len());
    for (a, b) in left
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(right.rgba.as_chunks::<4>().0.iter())
    {
        let amplify = |channel: usize| a[channel].abs_diff(b[channel]).saturating_mul(4);
        rgba.extend_from_slice(&[amplify(0), amplify(1), amplify(2), 0xff]);
    }
    Ok(Image {
        width: left.width,
        height: left.height,
        rgba,
    })
}
