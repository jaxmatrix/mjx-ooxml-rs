//! The third artefact: type specimens whose *word boxes are the measurement*, and the fifty-four
//! preset hatches.
//!
//! # What this deck is for, and why it is the same sitting
//!
//! Four of the seven items waiting on one Windows morning are not about shapes at all:
//!
//! * **Cambria has no published metrics** — `mjx_text::reference` carries the family so the gap is
//!   queryable and carries no numbers, and its authority is
//!   [`Unverified`](mjx_text::ReferenceAuthority::Unverified).
//! * **Arial, Times New Roman and Courier New's advances are transcribed from (URW)++ Nimbus
//!   *clones***, not from Microsoft's faces. The table calls them
//!   [`Published`](mjx_text::ReferenceAuthority::Published) and MJXOFF-207's brief says that is
//!   wrong.
//! * **Calibri** has exactly ten characters, the digits, from a figure ECMA-376 states in passing.
//! * **The ten pictorial hatch masks** in `mjx_paint::PATTERN_MASKS` are drawn to the shape their
//!   names describe, because ECMA-376 gives a bitmap for none of the fifty-four.
//!
//! All four are answered by opening one file, exporting it, and reading the result mechanically.
//! **Nothing here asks the person to measure anything**, which is the whole design brief: the
//! sitting is short and mechanical or it does not happen.
//!
//! # How a word box becomes an advance
//!
//! `pdftotext -bbox-layout` reports a box around each word, in points, from the glyph positions the
//! exporter wrote. A single glyph's box is its *ink*, which is not its advance — so one box is never
//! enough. There are three here, and each answers a different objection:
//!
//! ```text
//! baseline: H H              span = adv(H) + ink(H)              = K,  once per family
//! probe:    H c H            span = adv(H) + adv(c) + ink(H)     = K + adv(c)
//! run:      H c c ... c H    span = adv(H) + N*adv(c) + ink(H)   = K + N*adv(c)
//! ```
//!
//! **`probe - baseline` is the advance, exactly**, because the two strings begin and end with the
//! same glyph at the same size and differ by exactly one inserted character: side bearings, hinting
//! and the exporter's rounding all cancel. The sentinel is also what makes the space character
//! measurable — ten spaces alone produce no word at all, while `H` `H` produces two words whose span
//! is still the number wanted.
//!
//! # Why the run is there anyway, and what it caught
//!
//! MJXOFF-207 built the run **first**, as `(run - probe) / (N - 1)`, and it was wrong for two of
//! Arial's ninety-two characters when read back from a real export. `f` came back at 260 thousandths
//! of an em against a published 278, and `1` at 482 against 556. Neither is a rounding error and
//! neither is a reader defect: a run of ten identical glyphs is *shaped*, and `ff` is a ligature.
//! **A probe that repeats a character measures the character in a context no ordinary text puts it
//! in.**
//!
//! So the run is kept as a **second, independent estimate** — [`Probe::run_advance_per_mille`] —
//! and the reported number is always the ligature-free one. The two share no term: one subtracts
//! the baseline, the other subtracts the probe. Where they agree the advance is confirmed twice;
//! where they disagree, the disagreement names a shaping behaviour of the producer, which is itself
//! worth carrying back from a Windows morning.
//!
//! The probe alphabet is **not written down here**: it is read out of `mjx_text::reference`'s own
//! Arial table, so the artefact measures exactly the characters the table has room for and cannot
//! drift from it.
//!
//! # And what a word box cannot answer
//!
//! Line pitch, yes: six lines at a known size give five baseline-to-baseline distances, which is the
//! number pagination actually rests on. **The `hhea` triple separately, no.** A word's box is ink,
//! so the first line's top is the height of an `H`, not the font's ascender. The specimen therefore
//! records the *pitch* and says so; splitting it into ascender, descender and line gap needs the
//! face, not a rendering of it.

use mjx_dml::{CharacterPropertiesSpec, ColorSpec, FillSpec, PatternType};
use mjx_pptx::{PptxError, Presentation};
use mjx_text::reference::REFERENCE_METRICS;

use crate::deck::{CAPTION_FONT, CAPTION_POINTS, HEADING_POINTS};
use crate::layout::{
    slide_size, PlateGeometry, Rect, ToShapeBounds, HEADING_HEIGHT, PLATES_PER_SLIDE, SLIDE,
};

/// The file this deck is written to.
pub const FILE_NAME: &str = "03-type-specimens-and-hatches.pptx";

/// The size every probe is set at, in points.
///
/// Twenty-four rather than a hundred: the widest advance in the table is Arial's `W` at 0.944 em, so
/// ten of them at 24 points is 227 points and fits a 320-point box without wrapping. The precision
/// cost is small — poppler reports two decimal places, so an advance derived from a 24-point span
/// over nine repeats is good to about a ten-thousandth of an em, against a tolerance of half a
/// thousandth.
pub const PROBE_POINTS: f64 = 24.0;

/// How many copies of the probe character the long box holds.
pub const PROBE_REPEATS: usize = 10;

/// The glyph that brackets every probe.
///
/// Its own ink cancels between the two boxes, and it is what gives the space character a word box at
/// all. `H` because it is unambiguous in every family here, has no side bearing surprises, and is
/// itself in the probe alphabet, so a family that got it wrong would be caught by its own row.
pub const SENTINEL: char = 'H';

/// The width of the one-copy box, in points.
pub const SINGLE_WIDTH: i64 = 70;

/// The width of the ten-copy box, in points.
pub const MULTI_WIDTH: i64 = 320;

/// The gap between the two boxes of one probe.
pub const PROBE_GAP: i64 = 10;

/// A probe row's height, in points.
pub const PROBE_ROW: i64 = 30;

/// How far in from the slide's left edge the first probe column starts.
pub const PROBE_MARGIN: i64 = 20;

/// How far apart the two probe columns are.
pub const PROBE_COLUMN: i64 = 440;

/// How many probe rows fit on a slide.
pub const PROBE_ROWS: usize = ((SLIDE.1 - HEADING_HEIGHT - 10) / PROBE_ROW) as usize;

/// How many probes fit on a slide — two columns of them.
pub const PROBES_PER_SLIDE: usize = PROBE_ROWS * 2;

/// How many lines the line-pitch specimen has.
pub const PITCH_LINES: usize = 6;

/// The size the line-pitch specimen is set at, in points. Large, because the quantity wanted is a
/// *ratio* and poppler's two decimal places go further on a big number.
pub const PITCH_POINTS: f64 = 60.0;

/// What the line-pitch specimen says on every line.
///
/// Ascender, x-height and descender in three letters, so the ink box is the tallest the family can
/// make and a reader can see at a glance that the specimen rendered in the family it names.
pub const PITCH_LINE: &str = "Hxg";

/// One probe: a character, and where its two boxes are.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Probe {
    /// The family, as a document names it.
    pub family: &'static str,
    /// The character whose advance this probe measures.
    pub character: char,
    /// Which page of the deck, zero-based.
    pub page: usize,
    /// The one-copy box, `HcH`. Its span less the family's baseline span **is** the advance.
    pub single: Rect,
    /// The ten-copy box. The second, shaping-exposed estimate.
    pub multiple: Rect,
}

impl Probe {
    /// What the one-copy box says.
    #[must_use]
    pub fn single_text(&self) -> String {
        format!("{SENTINEL}{}{SENTINEL}", self.character)
    }

    /// What the ten-copy box says.
    #[must_use]
    pub fn multiple_text(&self) -> String {
        let mut text = String::with_capacity(PROBE_REPEATS + 2);
        text.push(SENTINEL);
        for _ in 0..PROBE_REPEATS {
            text.push(self.character);
        }
        text.push(SENTINEL);
        text
    }

    /// **The advance**, in thousandths of an em: the probe's span less the family's baseline span.
    ///
    /// Exact, and free of any shaping between repeated glyphs — see the module documentation for
    /// what that cost when it was not.
    ///
    /// `None` when either span is missing, which is what a box poppler found no word in looks like.
    /// It must stay a `None` rather than a zero: a zero advance is a number, and a number goes into
    /// a table and is believed.
    #[must_use]
    pub fn advance_per_mille(probe_span: Option<f64>, baseline_span: Option<f64>) -> Option<f64> {
        Some((probe_span? - baseline_span?) / PROBE_POINTS * 1000.0)
    }

    /// The **second** estimate, from the run of [`PROBE_REPEATS`] copies, in thousandths of an em.
    ///
    /// Shares no term with [`advance_per_mille`](Self::advance_per_mille) — that one subtracts the
    /// baseline and this one subtracts the probe — so the two agreeing is a real cross-check, and
    /// the two disagreeing names a shaping behaviour rather than a defect.
    #[must_use]
    pub fn run_advance_per_mille(probe_span: Option<f64>, run_span: Option<f64>) -> Option<f64> {
        #[allow(clippy::cast_precision_loss, reason = "PROBE_REPEATS is ten")]
        let repeats = (PROBE_REPEATS - 1) as f64;
        Some((run_span? - probe_span?) / repeats / PROBE_POINTS * 1000.0)
    }
}

/// One family's baseline box: `HH`, the one measurement every probe of that family subtracts.
///
/// It sits in the first unused probe slot of the family's last page, which the probe alphabet always
/// leaves free: ninety-two probes over thirty-two slots a page fill twenty-eight of the last page's
/// thirty-two. Derived from the same arithmetic the probes are, so a change to the alphabet or to
/// the grid moves both together — and [`baselines`] refuses outright if a change ever fills the
/// last page exactly and leaves no slot.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BaselineBox {
    /// The family.
    pub family: &'static str,
    /// Which page, zero-based.
    pub page: usize,
    /// Where the box is.
    pub bounds: Rect,
}

impl BaselineBox {
    /// What it says: the sentinel, twice.
    #[must_use]
    pub fn text(&self) -> String {
        format!("{SENTINEL}{SENTINEL}")
    }
}

/// One family's line-pitch specimen.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PitchSpecimen {
    /// The family.
    pub family: &'static str,
    /// Which page, zero-based.
    pub page: usize,
    /// The box the six lines are in.
    pub bounds: Rect,
}

/// One hatch swatch.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Swatch {
    /// Its index in `ST_PresetPatternVal`, which is also its row in `mjx_paint::PATTERN_MASKS`.
    pub index: usize,
    /// Its `prst` token.
    pub token: &'static str,
    /// The preset.
    pub preset: PatternType,
    /// Which page of the deck, zero-based.
    pub page: usize,
    /// Where the swatch is.
    pub bounds: Rect,
    /// The window a comparison would crop, if a comparison of hatches were evidence — which under
    /// LibreOffice it is not.
    pub window: Rect,
}

/// The fifty-four preset patterns, in `ST_PresetPatternVal` order.
///
/// # Why this table exists and how it is held honest
///
/// `mjx_paint::PATTERN_MASKS` is indexed by position in `ST_PresetPatternVal`, and
/// `mjx_scene::PatternPreset` carries that position as a number — a display list holds no strings.
/// `mjx_ooxml_types`' `PatternType` carries the *token* and can parse one, but the generated
/// enumerations in this workspace expose no `ALL`, so nothing in the workspace can walk the fifty-four
/// tokens in order.
///
/// This is that walk, and it is checked three ways rather than trusted:
/// `tests/the_hatch_table_is_the_hatch_table.rs` asserts that every entry round-trips through
/// `PatternType::from_wire` and `to_wire` (so no token is a typo), that there are exactly
/// `PATTERN_PRESET_COUNT` of them and all distinct, and — the one that actually checks the
/// **order** — that the twelve percentage presets land at positions whose `PATTERN_MASKS` row has
/// the number of bits set that the name states. Twelve anchors spread through the table is what
/// catches a transposition; a list that merely parsed would not.
pub const HATCH_TOKENS: [&str; 54] = [
    "pct5",
    "pct10",
    "pct20",
    "pct25",
    "pct30",
    "pct40",
    "pct50",
    "pct60",
    "pct70",
    "pct75",
    "pct80",
    "pct90",
    "horz",
    "vert",
    "ltHorz",
    "ltVert",
    "dkHorz",
    "dkVert",
    "narHorz",
    "narVert",
    "dashHorz",
    "dashVert",
    "cross",
    "dnDiag",
    "upDiag",
    "ltDnDiag",
    "ltUpDiag",
    "dkDnDiag",
    "dkUpDiag",
    "wdDnDiag",
    "wdUpDiag",
    "dashDnDiag",
    "dashUpDiag",
    "diagCross",
    "smCheck",
    "lgCheck",
    "smGrid",
    "lgGrid",
    "dotGrid",
    "smConfetti",
    "lgConfetti",
    "horzBrick",
    "diagBrick",
    "solidDmnd",
    "openDmnd",
    "dotDmnd",
    "plaid",
    "sphere",
    "weave",
    "divot",
    "shingle",
    "wave",
    "trellis",
    "zigZag",
];

/// The ten hatches ECMA-376 gives no rule for and `mjx_paint` draws by hand.
///
/// **Ten, not nine.** `mjx_paint::pattern`'s own module documentation says *"Nine are pictorial"* and
/// then lists ten names — `smConfetti`, `lgConfetti`, `plaid`, `sphere`, `weave`, `divot`,
/// `shingle`, `wave`, `trellis`, `zigZag`. The list is the authority and the count was the typo;
/// MJXOFF-207 found it while writing this artefact, and `tests/the_hatch_table_is_the_hatch_table.rs`
/// now asserts the length so the two cannot disagree again.
///
/// These are the swatches the sitting exists to replace with measurements. The other forty-four are
/// derived from what their names state — a coverage, or a direction and a period — and are on the
/// sheet so that a *derivation* can be checked too.
pub const PICTORIAL_HATCHES: [&str; 10] = [
    "smConfetti",
    "lgConfetti",
    "plaid",
    "sphere",
    "weave",
    "divot",
    "shingle",
    "wave",
    "trellis",
    "zigZag",
];

/// Every family the reference table has an entry for, in its own order.
#[must_use]
pub fn families() -> Vec<&'static str> {
    REFERENCE_METRICS.iter().map(|entry| entry.family).collect()
}

/// The probe alphabet, read out of the reference table rather than restated.
///
/// Arial's row is the one used because it is the widest of the four that carry characters, and
/// because the point of the artefact is to fill in a table of exactly this shape.
#[must_use]
pub fn probe_alphabet() -> Vec<char> {
    REFERENCE_METRICS
        .iter()
        .find(|entry| entry.family == "Arial")
        .map(|entry| {
            entry
                .advances
                .iter()
                .map(|(character, _)| *character)
                .collect()
        })
        .unwrap_or_default()
}

/// Where the `position`-th probe row of a page sits: its one-copy box and its ten-copy box.
///
/// One function, called by the probes and by the baseline box, so a slot cannot mean two things.
#[must_use]
pub fn slot(position: usize) -> (Rect, Rect) {
    #[allow(
        clippy::cast_possible_wrap,
        reason = "a position is bounded by PROBES_PER_SLIDE"
    )]
    let (column, row) = (
        (position / PROBE_ROWS) as i64,
        (position % PROBE_ROWS) as i64,
    );
    let x = PROBE_MARGIN + column * PROBE_COLUMN;
    let y = HEADING_HEIGHT + row * PROBE_ROW;
    (
        Rect::new(x, y, SINGLE_WIDTH, PROBE_ROW),
        Rect::new(x + SINGLE_WIDTH + PROBE_GAP, y, MULTI_WIDTH, PROBE_ROW),
    )
}

/// How many pages one family's probes take.
#[must_use]
pub fn pages_per_family() -> usize {
    probe_alphabet().len().div_ceil(PROBES_PER_SLIDE)
}

/// Every probe in the deck, in page order.
#[must_use]
pub fn probes() -> Vec<Probe> {
    let alphabet = probe_alphabet();
    let mut probes = Vec::new();
    let mut page = 0usize;
    for family in families() {
        for chunk in alphabet.chunks(PROBES_PER_SLIDE) {
            for (position, character) in chunk.iter().enumerate() {
                let (single, multiple) = slot(position);
                probes.push(Probe {
                    family,
                    character: *character,
                    page,
                    single,
                    multiple,
                });
            }
            page += 1;
        }
    }
    probes
}

/// One baseline box per family, in the first free slot of that family's last probe page.
///
/// # Panics
///
/// If the probe alphabet fills its last page exactly, leaving no free slot. That is a change to the
/// reference table's Arial row or to the grid, not an input, and it must fail loudly rather than
/// silently put the baseline on top of a probe.
#[must_use]
pub fn baselines() -> Vec<BaselineBox> {
    let alphabet = probe_alphabet();
    let pages = pages_per_family();
    let used_on_last_page = alphabet.len() - (pages - 1) * PROBES_PER_SLIDE;
    assert!(
        used_on_last_page < PROBES_PER_SLIDE,
        "the {} probes fill their last page exactly, so there is no free slot for the baseline box \
         every advance is measured against",
        alphabet.len()
    );
    families()
        .into_iter()
        .enumerate()
        .map(|(index, family)| BaselineBox {
            family,
            page: index * pages + pages - 1,
            bounds: slot(used_on_last_page).0,
        })
        .collect()
}

/// One line-pitch specimen per family, all on the page after the probes.
#[must_use]
pub fn pitch_specimens() -> Vec<PitchSpecimen> {
    let first = probes().last().map_or(0, |probe| probe.page + 1);
    families()
        .into_iter()
        .enumerate()
        .map(|(index, family)| PitchSpecimen {
            family,
            page: first + index,
            bounds: Rect::new(
                PROBE_MARGIN,
                HEADING_HEIGHT,
                SLIDE.0 - 2 * PROBE_MARGIN,
                SLIDE.1 - HEADING_HEIGHT - 10,
            ),
        })
        .collect()
}

/// Every hatch swatch, laid out on the same plate grid the preset decks use.
///
/// # Panics
///
/// If a token in [`HATCH_TOKENS`] is not a `ST_PresetPatternVal` value. That is a typo in a `const`,
/// not an input, and `tests/the_hatch_table_is_the_hatch_table.rs` is what stops it reaching here.
#[must_use]
pub fn swatches() -> Vec<Swatch> {
    let first = pitch_specimens().last().map_or(0, |pitch| pitch.page + 1);
    HATCH_TOKENS
        .iter()
        .enumerate()
        .map(|(index, token)| {
            let geometry = PlateGeometry::at(index % PLATES_PER_SLIDE);
            Swatch {
                index,
                token,
                preset: PatternType::from_wire(token)
                    .unwrap_or_else(|| panic!("`{token}` is not a preset pattern token")),
                page: first + index / PLATES_PER_SLIDE,
                bounds: geometry.shape_box(),
                window: geometry.window(),
            }
        })
        .collect()
}

/// How many pages the deck has.
#[must_use]
pub fn page_count() -> usize {
    swatches().last().map_or(0, |swatch| swatch.page + 1)
}

/// Author the type-specimen and hatch deck.
///
/// # Errors
///
/// Whatever the presentation layer fails with; nothing here is input-driven.
pub fn specimen_deck() -> Result<Vec<u8>, PptxError> {
    let probes = probes();
    let baselines = baselines();
    let pitches = pitch_specimens();
    let swatches = swatches();
    let mut deck = Presentation::blank(slide_size())?;

    for page in 0..page_count() {
        let slide = deck.add_slide()?;
        let heading = heading_for(page, &probes, &pitches, &swatches);
        write_heading(&mut deck, slide, &heading)?;

        for probe in probes.iter().filter(|probe| probe.page == page) {
            write_probe(&mut deck, slide, probe)?;
        }
        for baseline in baselines.iter().filter(|baseline| baseline.page == page) {
            write_text(
                &mut deck,
                slide,
                &baseline.text(),
                baseline.bounds,
                baseline.family,
                PROBE_POINTS,
            )?;
        }
        for pitch in pitches.iter().filter(|pitch| pitch.page == page) {
            write_pitch(&mut deck, slide, pitch)?;
        }
        for swatch in swatches.iter().filter(|swatch| swatch.page == page) {
            write_swatch(&mut deck, slide, swatch)?;
        }
    }
    deck.save()
}

/// What the page says it is.
fn heading_for(
    page: usize,
    probes: &[Probe],
    pitches: &[PitchSpecimen],
    swatches: &[Swatch],
) -> String {
    if let Some(probe) = probes.iter().find(|probe| probe.page == page) {
        return format!(
            "MJXOFF-207 · advance ruler · {} · {PROBE_POINTS} pt · each row is one character, once \
             and {PROBE_REPEATS} times, bracketed by `{SENTINEL}`",
            probe.family
        );
    }
    if let Some(pitch) = pitches.iter().find(|pitch| pitch.page == page) {
        return format!(
            "MJXOFF-207 · line pitch · {} · {PITCH_LINES} lines at {PITCH_POINTS} pt, single spaced",
            pitch.family
        );
    }
    if swatches.iter().any(|swatch| swatch.page == page) {
        return "MJXOFF-207 · the fifty-four preset hatches, black on white — ECMA-376 publishes a \
                bitmap for none of them"
            .to_owned();
    }
    "MJXOFF-207".to_owned()
}

fn write_heading(deck: &mut Presentation, slide: usize, text: &str) -> Result<(), PptxError> {
    let bounds = Rect::new(12, 6, SLIDE.0 - 24, HEADING_HEIGHT - 10).to_shape_bounds();
    let index = deck.add_text_box(slide, text, bounds)?;
    deck.set_shape_run_properties(
        slide,
        index,
        &CharacterPropertiesSpec::new()
            .with_font(CAPTION_FONT)
            .with_size_points(HEADING_POINTS),
    )
}

fn write_probe(deck: &mut Presentation, slide: usize, probe: &Probe) -> Result<(), PptxError> {
    for (text, bounds) in [
        (probe.single_text(), probe.single),
        (probe.multiple_text(), probe.multiple),
    ] {
        write_text(deck, slide, &text, bounds, probe.family, PROBE_POINTS)?;
    }
    Ok(())
}

/// One text box in one family at one size — the whole of what a specimen is.
fn write_text(
    deck: &mut Presentation,
    slide: usize,
    text: &str,
    bounds: Rect,
    family: &str,
    points: f64,
) -> Result<(), PptxError> {
    let index = deck.add_text_box(slide, text, bounds.to_shape_bounds())?;
    deck.set_shape_run_properties(
        slide,
        index,
        &CharacterPropertiesSpec::new()
            .with_font(family)
            .with_size_points(points),
    )
}

fn write_pitch(
    deck: &mut Presentation,
    slide: usize,
    pitch: &PitchSpecimen,
) -> Result<(), PptxError> {
    let text = std::iter::repeat_n(PITCH_LINE, PITCH_LINES)
        .collect::<Vec<_>>()
        .join("\n");
    write_text(deck, slide, &text, pitch.bounds, pitch.family, PITCH_POINTS)
}

fn write_swatch(deck: &mut Presentation, slide: usize, swatch: &Swatch) -> Result<(), PptxError> {
    let shape = deck.add_shape(
        slide,
        mjx_geometry::PresetShapeType::Rectangle,
        swatch.bounds.to_shape_bounds(),
    )?;
    deck.set_shape_fill(
        slide,
        shape,
        &FillSpec::pattern(
            swatch.preset,
            ColorSpec::Srgb("000000".to_owned()),
            ColorSpec::Srgb("FFFFFF".to_owned()),
        ),
    )?;
    deck.set_shape_no_outline(slide, shape)?;

    let geometry = PlateGeometry::at(swatch.index % PLATES_PER_SLIDE);
    let caption = deck.add_text_box(
        slide,
        &format!("{}. {}", swatch.index, swatch.token),
        geometry.caption().to_shape_bounds(),
    )?;
    deck.set_shape_run_properties(
        slide,
        caption,
        &CharacterPropertiesSpec::new()
            .with_font(CAPTION_FONT)
            .with_size_points(CAPTION_POINTS),
    )
}
