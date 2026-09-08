//! Reading the answers back out of an exported PDF.
//!
//! # The one rule every reader here obeys
//!
//! **A reader that cannot see what it expected reports that it could not, and never a number.** A
//! probe whose words did not come back is [`Verdict::NotEvidence`], not an advance of zero — because
//! a zero is a number, and a number goes into a table and is believed.
//!
//! That is not hypothetical, and it was found rather than anticipated. `pdftotext` reading
//! LibreOffice's export of this crate's own specimen deck loses the trailing sentinel of Arial's `%`
//! probe: the line comes back as `H%` where the document says `H%H`. Nothing about the geometry of
//! the extraction says so — the box is a perfectly plausible width — and a reader that took the span
//! at face value would record Arial's `%` advance short by a whole `H`, which is a wrong number that
//! looks like a right one. So [`span_of`] compares the extracted text against what the document says
//! and refuses the row when they differ, naming both.
//!
//! # What each reader can and cannot answer
//!
//! | Reader | Answers | Does not answer |
//! |---|---|---|
//! | [`read_advances`] | each character's advance, in thousandths of an em | anything about vertical metrics |
//! | [`read_line_pitch`] | baseline-to-baseline distance as a fraction of the type size | the `hhea` triple separately: a word box is *ink*, so the top of a line is the top of an `H` and not the font's ascender |
//! | [`read_hanging`] | which characters end a line **past** that paragraph's own measure | why, or what Word would do at a different measure |
//! | [`read_hatch_tiles`] | each hatch's ink coverage, and its tile when the render is clean enough to find a period | the bitmap when the export's own resampling has blurred the tile — in which case it says so |

use std::path::Path;

use mjx_paint::PATTERN_MASKS;

use crate::authority::{Baseline, ReferenceProvider, RenderedContent, Verdict};
use crate::hanging::{paragraphs, HangingRole, HANGABLE};
use crate::layout::Rect;
use crate::tools::{rasterise, word_boxes, Raster, WordBox};
use crate::typography::{
    baselines, pitch_specimens, probes, swatches, BaselineBox, Probe, PITCH_LINES, PITCH_POINTS,
};

/// How far apart two spans may be, in points, and still be called the same edge.
///
/// Poppler reports six decimal places of a number it computed from glyph positions, so the noise is
/// far below a tenth of a point; a tenth is generous and is well under the smallest advance
/// difference the reference table can express.
pub const EDGE_TOLERANCE_POINTS: f64 = 0.1;

/// One character's measured advance.
#[derive(Clone, PartialEq, Debug)]
pub struct AdvanceReading {
    /// The family the probe was set in.
    pub family: &'static str,
    /// The character.
    pub character: char,
    /// **The advance**, in thousandths of an em: the `HcH` box less the family's `HH` box. `None`
    /// when the reader refused the row.
    pub advance_per_mille: Option<f64>,
    /// The **second** estimate, from the run of ten copies. It shares no term with the first, so
    /// the two agreeing is a cross-check — and the two disagreeing names a shaping behaviour of the
    /// producer rather than a defect in either. `f` and `1` disagree under LibreOffice today,
    /// because `ff` is a ligature and a run of `1`s is shaped.
    pub run_advance_per_mille: Option<f64>,
    /// What was concluded, and — when nothing was — why.
    pub verdict: Verdict,
}

impl AdvanceReading {
    /// How far the two independent estimates are apart, in thousandths of an em, or `None` when
    /// there are not two of them.
    #[must_use]
    pub fn estimates_disagree_by(&self) -> Option<f64> {
        Some((self.advance_per_mille? - self.run_advance_per_mille?).abs())
    }
}

/// One family's measured line pitch.
#[derive(Clone, PartialEq, Debug)]
pub struct PitchReading {
    /// The family.
    pub family: &'static str,
    /// Baseline-to-baseline distance as a fraction of the type size, or `None` when the specimen
    /// did not come back as [`PITCH_LINES`] lines.
    pub pitch_per_em: Option<f64>,
    /// What was concluded.
    pub verdict: Verdict,
}

/// What one paragraph of the hanging document did.
#[derive(Clone, PartialEq, Debug)]
pub struct HangingReading {
    /// Which paragraph.
    pub role: HangingRole,
    /// The measure the reader inferred, in points: the *median* line width, which is what a
    /// paragraph of many lines settles on.
    pub measure_points: f64,
    /// How many lines it broke into.
    pub lines: usize,
    /// The characters observed ending a line **past** that measure — the hanging set, as this
    /// producer renders it.
    pub hangs: Vec<char>,
    /// What was concluded.
    pub verdict: Verdict,
}

/// One hatch swatch as it was actually drawn.
#[derive(Clone, PartialEq, Debug)]
pub struct HatchReading {
    /// Its index in `ST_PresetPatternVal`, which is its row in `mjx_paint::PATTERN_MASKS`.
    pub index: usize,
    /// Its token.
    pub token: &'static str,
    /// What fraction of the swatch is inked.
    pub coverage: f64,
    /// What fraction of `PATTERN_MASKS`' row for it is set — the number a measurement would replace.
    pub mask_coverage: f64,
    /// The tile the reader recovered, if the render was periodic enough to find one: a square of
    /// rows, most significant bit leftmost, in the same shape `PATTERN_MASKS` holds.
    pub tile: Option<Vec<Vec<bool>>>,
    /// What was concluded.
    pub verdict: Verdict,
}

/// The span of the words whose centres fall inside `rect`, and the text they spell.
///
/// `None` when no word is in the rectangle at all, which is a different answer from a span of zero.
#[must_use]
pub fn span_in(words: &[WordBox], page: usize, rect: Rect) -> Option<(f64, f64, String)> {
    let inside: Vec<&WordBox> = words
        .iter()
        .filter(|word| word.page == page && word.is_inside(rect))
        .collect();
    if inside.is_empty() {
        return None;
    }
    let minimum = inside
        .iter()
        .map(|word| word.x_min)
        .fold(f64::MAX, f64::min);
    let maximum = inside
        .iter()
        .map(|word| word.x_max)
        .fold(f64::MIN, f64::max);
    let text: String = inside.iter().map(|word| word.text.as_str()).collect();
    Some((minimum, maximum, text))
}

/// Whitespace removed, so a probe whose words poppler split at a space still compares equal.
fn squeezed(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

/// Every probe's advance, out of an exported specimen deck.
///
/// # Errors
///
/// A sentence naming what `pdftotext` said. A *probe* that could not be read is a row, not an error.
pub fn read_advances(pdf: &Path) -> Result<Vec<AdvanceReading>, String> {
    let words = word_boxes(pdf)?;
    let baselines = baselines();
    Ok(probes()
        .into_iter()
        .map(|probe| {
            let baseline = baselines
                .iter()
                .find(|baseline| baseline.family == probe.family);
            advance_of(&probe, baseline, &words)
        })
        .collect())
}

/// The span of a box whose words spell exactly `expected`, or the sentence saying why not.
///
/// **The text check is what stops a wrong number looking like a right one.** `pdftotext` reading
/// LibreOffice's export of this crate's own deck loses the trailing sentinel of Arial's `%` probe:
/// the line comes back as `H%` where the document says `H%H`. Nothing about the geometry says so —
/// the box is a perfectly plausible width — and a span taken at face value would record `%` short
/// by a whole `H`.
/// # Errors
///
/// A sentence naming what was wrong: no words in the box, or words that spell something else.
pub fn span_of(
    words: &[WordBox],
    page: usize,
    rect: Rect,
    expected: &str,
    which: &str,
) -> Result<f64, String> {
    let Some((minimum, maximum, extracted)) = span_in(words, page, rect) else {
        return Err(format!(
            "the {which} box has no words in it, so there is no span to take"
        ));
    };
    if squeezed(&extracted) != squeezed(expected) {
        return Err(format!(
            "the {which} box extracted as {extracted:?} where the document says {expected:?}; a \
             span taken from a truncated extraction is a wrong number that looks like a right one"
        ));
    }
    Ok(maximum - minimum)
}

/// One probe's row.
fn advance_of(probe: &Probe, baseline: Option<&BaselineBox>, words: &[WordBox]) -> AdvanceReading {
    let refuse = |reason: String| AdvanceReading {
        family: probe.family,
        character: probe.character,
        advance_per_mille: None,
        run_advance_per_mille: None,
        verdict: Verdict::NotEvidence {
            reason: format!("`{}` {:?}: {reason}", probe.family, probe.character),
        },
    };

    let Some(baseline) = baseline else {
        return refuse("the family has no baseline box, so nothing can be subtracted".to_owned());
    };
    // Poppler numbers pages from one.
    let baseline_span = match span_of(
        words,
        baseline.page + 1,
        baseline.bounds,
        &baseline.text(),
        "baseline",
    ) {
        Ok(span) => span,
        Err(reason) => return refuse(reason),
    };
    let probe_span = match span_of(
        words,
        probe.page + 1,
        probe.single,
        &probe.single_text(),
        "one-copy",
    ) {
        Ok(span) => span,
        Err(reason) => return refuse(reason),
    };
    // The run is the *second* estimate and its absence is not fatal to the first: a family whose
    // run box was mis-extracted still has an exact advance.
    let run_span = span_of(
        words,
        probe.page + 1,
        probe.multiple,
        &probe.multiple_text(),
        "ten-copy",
    )
    .ok();

    let advance = Probe::advance_per_mille(Some(probe_span), Some(baseline_span));
    let run = Probe::run_advance_per_mille(Some(probe_span), run_span);
    match advance {
        Some(value) if value.is_finite() && value >= 0.0 => AdvanceReading {
            family: probe.family,
            character: probe.character,
            advance_per_mille: Some(value),
            run_advance_per_mille: run,
            verdict: Verdict::Agreed {
                differing_fraction: 0.0,
                allowed: 0.0,
            },
        },
        other => refuse(format!("the two spans imply an advance of {other:?}")),
    }
}

/// Each family's line pitch, as a fraction of the type size.
///
/// # Errors
///
/// A sentence naming what `pdftotext` said.
pub fn read_line_pitch(pdf: &Path) -> Result<Vec<PitchReading>, String> {
    let words = word_boxes(pdf)?;
    Ok(pitch_specimens()
        .into_iter()
        .map(|specimen| {
            let page = specimen.page + 1;
            let mut tops: Vec<f64> = words
                .iter()
                .filter(|word| word.page == page && word.is_inside(specimen.bounds))
                .map(|word| word.y_min)
                .collect();
            tops.sort_by(f64::total_cmp);
            tops.dedup_by(|left, right| (*left - *right).abs() < EDGE_TOLERANCE_POINTS);
            if tops.len() != PITCH_LINES {
                return PitchReading {
                    family: specimen.family,
                    pitch_per_em: None,
                    verdict: Verdict::NotEvidence {
                        reason: format!(
                            "`{}`'s specimen came back as {} distinct line tops where the document \
                             writes {PITCH_LINES}; a pitch taken from the wrong number of lines is \
                             a wrong number",
                            specimen.family,
                            tops.len()
                        ),
                    },
                };
            }
            #[allow(clippy::cast_precision_loss, reason = "PITCH_LINES is six")]
            let steps = (PITCH_LINES - 1) as f64;
            let pitch = (tops[PITCH_LINES - 1] - tops[0]) / steps / PITCH_POINTS;
            PitchReading {
                family: specimen.family,
                pitch_per_em: Some(pitch),
                verdict: Verdict::Agreed {
                    differing_fraction: 0.0,
                    allowed: 0.0,
                },
            }
        })
        .collect())
}

/// What each paragraph of the hanging document did with its punctuation.
///
/// # Errors
///
/// A sentence naming what `pdftotext` said.
pub fn read_hanging(pdf: &Path) -> Result<Vec<HangingReading>, String> {
    let words = word_boxes(pdf)?;
    // The three paragraphs run down one page, so they are separated by their vertical order rather
    // than by a rectangle: a paragraph's own height depends on how many lines it broke into, which
    // is one of the things being measured.
    let mut lines: Vec<(f64, f64, String)> = Vec::new();
    for word in &words {
        match lines
            .iter_mut()
            .find(|(top, _, _)| (*top - word.y_min).abs() < EDGE_TOLERANCE_POINTS)
        {
            Some((_, right, text)) => {
                *right = right.max(word.x_max);
                text.push_str(&word.text);
            }
            None => lines.push((word.y_min, word.x_max, word.text.clone())),
        }
    }
    lines.sort_by(|left, right| left.0.total_cmp(&right.0));

    let roles = paragraphs();
    let per_paragraph = lines.len() / roles.len().max(1);
    Ok(roles
        .into_iter()
        .enumerate()
        .map(|(position, paragraph)| {
            let slice: Vec<&(f64, f64, String)> = lines
                .iter()
                .skip(position * per_paragraph)
                .take(per_paragraph)
                .collect();
            if slice.len() < 4 {
                return HangingReading {
                    role: paragraph.role,
                    measure_points: 0.0,
                    lines: slice.len(),
                    hangs: Vec::new(),
                    verdict: Verdict::NotEvidence {
                        reason: format!(
                            "`{}` came back as {} line(s); a measure inferred from that few is not \
                             a measure",
                            paragraph.role.label(),
                            slice.len()
                        ),
                    },
                };
            }
            let mut rights: Vec<f64> = slice.iter().map(|(_, right, _)| *right).collect();
            rights.sort_by(f64::total_cmp);
            let measure = rights[rights.len() / 2];
            let hangs: Vec<char> = slice
                .iter()
                .filter(|(_, right, _)| *right > measure + EDGE_TOLERANCE_POINTS)
                .filter_map(|(_, _, text)| text.chars().next_back())
                .filter(|character| HANGABLE.contains(character))
                .collect::<std::collections::BTreeSet<char>>()
                .into_iter()
                .collect();
            HangingReading {
                role: paragraph.role,
                measure_points: measure,
                lines: slice.len(),
                hangs,
                verdict: Verdict::Agreed {
                    differing_fraction: 0.0,
                    allowed: 0.0,
                },
            }
        })
        .collect())
}

/// Every hatch swatch's coverage, and its tile where one can be recovered.
///
/// # Errors
///
/// A sentence naming what `pdftoppm` said.
pub fn read_hatch_tiles(pdf: &Path) -> Result<Vec<HatchReading>, String> {
    let mut readings = Vec::new();
    let mut rasters: std::collections::BTreeMap<usize, Raster> = std::collections::BTreeMap::new();
    for swatch in swatches() {
        let raster = match rasters.entry(swatch.page) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(rasterise(pdf, swatch.page + 1)?)
            }
        };
        let crop = crate::compare::crop(raster, swatch.bounds);
        let inked = crop.ink();
        #[allow(
            clippy::cast_precision_loss,
            reason = "a swatch crop is tens of thousands of pixels"
        )]
        let coverage = if crop.width * crop.height == 0 {
            0.0
        } else {
            inked as f64 / (crop.width as f64 * crop.height as f64)
        };
        let set: u32 = PATTERN_MASKS[swatch.index]
            .iter()
            .map(|row| u32::from(row.count_ones()))
            .sum();
        let tile = recover_tile(&crop);
        readings.push(HatchReading {
            index: swatch.index,
            token: swatch.token,
            coverage,
            mask_coverage: f64::from(set) / 64.0,
            verdict: if crop.width < 16 || crop.height < 16 {
                Verdict::NotEvidence {
                    reason: format!(
                        "`{}`'s swatch rasterised to {}x{} pixels, which cannot hold a tile",
                        swatch.token, crop.width, crop.height
                    ),
                }
            } else {
                Verdict::Agreed {
                    differing_fraction: 0.0,
                    allowed: 0.0,
                }
            },
            tile,
        });
    }
    Ok(readings)
}

/// The smallest square period the crop repeats at, as a bitmap, or `None` when nothing repeats
/// cleanly enough to be called a tile.
///
/// A hatch is periodic by construction, so the period is found rather than assumed: the exporter
/// chooses its own tile size and the raster's DPI is ours, so *"eight pixels"* is not a thing this
/// reader may take on faith. A period is accepted when shifting the crop by it disagrees with itself
/// on under [`TILE_AGREEMENT`] of its pixels — a blurred or resampled hatch fails that and reports
/// `None`, which is the honest answer for a bitmap nobody can read off the page.
fn recover_tile(crop: &Raster) -> Option<Vec<Vec<bool>>> {
    /// How much of the crop a candidate period must agree with itself over.
    const TILE_AGREEMENT: f64 = 0.98;
    /// The largest period worth looking for. A hatch tile is small; a "period" larger than this is
    /// the swatch's own edge, not a pattern.
    const LARGEST_PERIOD: u32 = 48;

    if crop.width < LARGEST_PERIOD * 2 || crop.height < LARGEST_PERIOD * 2 {
        return None;
    }
    let inked = |x: u32, y: u32| {
        crop.pixel(x, y)
            .is_some_and(|pixel| pixel.iter().any(|channel| *channel < 128))
    };
    // Only the middle of the swatch, so the shape's own antialiased border is not a "pattern".
    let (x0, y0) = (crop.width / 8, crop.height / 8);
    let (x1, y1) = (crop.width - crop.width / 8, crop.height - crop.height / 8);

    for period in 2..=LARGEST_PERIOD {
        let mut agreed = 0usize;
        let mut compared = 0usize;
        for y in y0..y1.saturating_sub(period) {
            for x in x0..x1.saturating_sub(period) {
                compared += 1;
                if inked(x, y) == inked(x + period, y) && inked(x, y) == inked(x, y + period) {
                    agreed += 1;
                }
            }
        }
        #[allow(
            clippy::cast_precision_loss,
            reason = "a swatch crop is tens of thousands of pixels"
        )]
        let fraction = if compared == 0 {
            0.0
        } else {
            agreed as f64 / compared as f64
        };
        if fraction >= TILE_AGREEMENT {
            return Some(
                (0..period)
                    .map(|y| (0..period).map(|x| inked(x0 + x, y0 + y)).collect())
                    .collect(),
            );
        }
    }
    None
}

/// Every reading above as [`Baseline`]s, so the typography and hatch items land in the same report
/// the plates do — with the same provider, the same authority and the same exclusions.
#[must_use]
pub fn as_baselines(
    advances: &[AdvanceReading],
    pitches: &[PitchReading],
    hanging: &[HangingReading],
    hatches: &[HatchReading],
    provider: ReferenceProvider,
    artefact: &str,
) -> Vec<Baseline> {
    let mut rows = Vec::new();
    for reading in advances {
        rows.push(Baseline::new(
            format!("{} {:?}", reading.family, reading.character),
            artefact,
            provider,
            RenderedContent::TextLayout,
            reading.verdict.clone(),
        ));
    }
    for reading in pitches {
        rows.push(Baseline::new(
            format!("{} line pitch", reading.family),
            artefact,
            provider,
            RenderedContent::TextLayout,
            reading.verdict.clone(),
        ));
    }
    for reading in hanging {
        rows.push(Baseline::new(
            reading.role.label(),
            artefact,
            provider,
            RenderedContent::TextLayout,
            reading.verdict.clone(),
        ));
    }
    for reading in hatches {
        rows.push(Baseline::new(
            reading.token,
            artefact,
            provider,
            // The exclusion that makes every one of these `not evidence` under LibreOffice, and
            // lifts by itself the day an Office export arrives.
            RenderedContent::PatternFill,
            reading.verdict.clone(),
        ));
    }
    rows
}
