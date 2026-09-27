//! Text wrapping: what a line's measure actually is when something floats beside it.
//!
//! # Why this is not a rectangle subtraction
//!
//! A wrapped line's available width is **not** the column less the object's bounding box. `w:drawing`
//! carries `wp:wrapTight` and `wp:wrapThrough`, and both take a real polygon
//! (`wp:wrapPolygon`) — so the width a line gets depends on **where down the object that line
//! sits**, and a triangle standing on its point leaves nearly the whole measure at its apex and
//! nearly none at its base. An engine that subtracted the bounding box would produce the same
//! answer for every line beside the object, which is *almost* right and is why this is the most
//! under-estimated feature in a Word renderer.
//!
//! [`free_runs`] is therefore a **per-band query**, not a per-object one: given the vertical band one
//! line occupies, it returns the horizontal runs of the measure that line may use. It is exact for a
//! polygon rather than sampled, and the reason it can be is stated at [`span_in_band`].
//!
//! # The four decisions this module makes, and which are readings
//!
//! * **`tight` collapses to one interval and `through` does not.** That is the whole difference
//!   between the two elements — §20.4.2.15 and §20.4.2.17 differ in one sentence, *"text can flow
//!   into the wrap polygon"* — so `tight` takes the polygon's outermost crossings in the band and
//!   `through` takes every covered interval separately. `SpecCode`-adjacent: the schema gives the
//!   two elements the same content model and the prose gives them different behaviour.
//! * **The largest-side rule.** `wrapText="largest"` is Word's default behaviour for an off-centre
//!   object, and §20.4.3.7's `ST_WrapText` names the value without saying what happens when the two
//!   sides tie. **`GUESS:`** a tie goes to the *left* run, because a tie means the object is centred
//!   and Word's own dialog labels that case "left only".
//! * **`topAndBottom` clears the whole measure.** The band the object occupies has no runs at all,
//!   which is what makes a line fall past it rather than beside it.
//! * **What unit `wp:wrapPolygon`'s coordinates are in.** See [`Exclusion::from_polygon`]; this is
//!   the sharpest guess in the module and the one a sitting should check first.
//!
//! # ⚠ Provenance
//!
//! Nobody has run Word. The polygon geometry is arithmetic and is not a reading; *which* polygon
//! Word actually uses, whether it insets by half a line, and how it rounds are all unchecked. This
//! is weaker evidence than R19's UAX #14 rows and no stronger than R20's footnote-area rows: there
//! is **no external standard for text wrapping at all**, so every row this module contributes to
//! `tests/the_provenance_is_declared.rs` is `EngineDerived` or `SpecCode`, and none is
//! `DocumentedBehaviour`.

use mjx_ooxml_core::measure::Emu;

/// Which sides of an object text may flow down.
///
/// [`mjx_ooxml_types::wordprocessingdrawing::WrapText`]'s four values, kept as this crate's own
/// enumeration so that a floating **table** — whose `w:tblpPr` has no `wrapText` at all — can be
/// described in the same vocabulary.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WrapSide {
    /// `bothSides` — text flows on either side of the object.
    #[default]
    BothSides,
    /// `left` — only the run to the object's left survives.
    Left,
    /// `right` — only the run to its right.
    Right,
    /// `largest` — whichever run is wider, and no other.
    Largest,
}

impl WrapSide {
    /// The value `wp:wrapSquare@wrapText` and friends carry.
    #[must_use]
    pub fn of(value: mjx_ooxml_types::wordprocessingdrawing::WrapText) -> Self {
        use mjx_ooxml_types::wordprocessingdrawing::WrapText;
        match value {
            WrapText::BothSides => Self::BothSides,
            WrapText::Left => Self::Left,
            WrapText::Right => Self::Right,
            WrapText::Largest => Self::Largest,
        }
    }
}

/// One thing text has to flow around, in the coordinates of the column it sits in.
///
/// `x` grows to the right from the column's left edge and `y` grows downward from the column's top,
/// which is the space [`crate::paginate::fill_column`] already places blocks in.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Exclusion {
    /// The left edge of the object, its `distL` already subtracted.
    pub left: Emu,
    /// The right edge, `distR` already added.
    pub right: Emu,
    /// The top edge, `distT` already subtracted.
    pub top: Emu,
    /// The bottom edge, `distB` already added.
    pub bottom: Emu,
    /// The wrap polygon, in the same column coordinates, when the object has one. Empty for a
    /// square wrap, which is the bounding box and needs none.
    pub polygon: Vec<(Emu, Emu)>,
    /// Which sides text may use.
    pub side: WrapSide,
    /// Whether text may enter a concavity of the polygon (`wp:wrapThrough`) or only follow its
    /// outline (`wp:wrapTight`).
    pub through: bool,
    /// Whether this is a `wp:wrapTopAndBottom`, which clears the whole measure rather than part of
    /// it.
    pub clears_the_band: bool,
}

/// How many units a `wp:wrapPolygon` coordinate space is wide when Word writes one.
///
/// See [`Exclusion::from_polygon`].
pub const WRAP_POLYGON_UNITS: i64 = 21600;

impl Exclusion {
    /// A square wrap: the object's box, with its four distances already applied.
    #[must_use]
    pub fn rectangle(left: Emu, top: Emu, right: Emu, bottom: Emu, side: WrapSide) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
            polygon: Vec::new(),
            side,
            through: false,
            clears_the_band: false,
        }
    }

    /// A `wp:wrapTight` or `wp:wrapThrough`: the object's box **and** its polygon, resolved into
    /// column coordinates.
    ///
    /// # ⚠ What unit the polygon is in — the sharpest `GUESS:` in this crate
    ///
    /// ECMA-376 Part 1 §20.4.2.16 types `wp:wrapPolygon`'s `wp:start` and `wp:lineTo` as
    /// `a:CT_Point2D`, whose `x`/`y` are `ST_Coordinate` — **EMU**. Word does not write EMU there: it
    /// writes the shape's own `0..21600` drawing space, which is what every `.docx` with a tight wrap
    /// in it actually contains. Twenty-one thousand six hundred EMU is `0.06 mm`, so an object whose
    /// polygon really were in EMU would have a wrap outline six hundredths of a millimetre across,
    /// which nobody has ever authored.
    ///
    /// So: **when every coordinate lies within `0..=`[`WRAP_POLYGON_UNITS`] and the object is larger
    /// than that on the same axis, the coordinates are read as `21600`ths of the extent.** Otherwise
    /// they are read as EMU offsets from the object's top-left corner, which is what the schema says.
    /// Both readings are implemented, the choice is made per object, and it is the first thing a
    /// sitting against real Word should check.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_polygon(
        left: Emu,
        top: Emu,
        right: Emu,
        bottom: Emu,
        width: Emu,
        height: Emu,
        raw: &[(i64, i64)],
        side: WrapSide,
        through: bool,
    ) -> Self {
        let relative_x = width.emu() > WRAP_POLYGON_UNITS
            && raw
                .iter()
                .all(|(x, _)| (0..=WRAP_POLYGON_UNITS).contains(x));
        let relative_y = height.emu() > WRAP_POLYGON_UNITS
            && raw
                .iter()
                .all(|(_, y)| (0..=WRAP_POLYGON_UNITS).contains(y));
        let polygon = raw
            .iter()
            .map(|&(x, y)| {
                let x = if relative_x {
                    scaled(x, width.emu())
                } else {
                    x
                };
                let y = if relative_y {
                    scaled(y, height.emu())
                } else {
                    y
                };
                (left + Emu::from_emu(x), top + Emu::from_emu(y))
            })
            .collect();
        Self {
            left,
            right,
            top,
            bottom,
            polygon,
            side,
            through,
            clears_the_band: false,
        }
    }

    /// A `wp:wrapTopAndBottom`: no text beside it at all.
    #[must_use]
    pub fn band(top: Emu, bottom: Emu) -> Self {
        Self {
            left: Emu::from_emu(i64::MIN / 4),
            right: Emu::from_emu(i64::MAX / 4),
            top,
            bottom,
            polygon: Vec::new(),
            side: WrapSide::BothSides,
            through: false,
            clears_the_band: true,
        }
    }

    /// Whether this exclusion overlaps the vertical band `top..bottom` at all.
    #[must_use]
    pub fn meets(&self, top: Emu, bottom: Emu) -> bool {
        self.top < bottom && self.bottom > top
    }
}

/// `value` of `WRAP_POLYGON_UNITS`, expressed out of `extent`.
fn scaled(value: i64, extent: i64) -> i64 {
    value.saturating_mul(extent) / WRAP_POLYGON_UNITS
}

/// The horizontal runs of `measure_left..measure_right` a line occupying `top..bottom` may use.
///
/// Returned left to right, non-overlapping, and possibly empty — an empty answer means *no text fits
/// beside these objects at this height at all*, which is what a `wp:wrapTopAndBottom` produces and is
/// the caller's signal to move the line down rather than to compose it into nothing.
///
/// # What it costs
///
/// One pass over the exclusions per call, and for each polygon one pass over its edges per sample
/// row. A page with no floats on it never calls this — [`crate::flow::lay_out`] checks for an empty
/// exclusion list first — which is what keeps a document with no drawings in it exactly as fast as
/// it was before this module existed.
#[must_use]
pub fn free_runs(
    measure_left: Emu,
    measure_right: Emu,
    top: Emu,
    bottom: Emu,
    exclusions: &[Exclusion],
) -> Vec<(Emu, Emu)> {
    let mut runs = vec![(measure_left, measure_right)];
    for exclusion in exclusions {
        if !exclusion.meets(top, bottom) {
            continue;
        }
        if exclusion.clears_the_band {
            return Vec::new();
        }
        let blocked = span_in_band(exclusion, top, bottom);
        if blocked.is_empty() {
            continue;
        }
        runs = subtract(&runs, &blocked);
        runs = keep_side(runs, exclusion.side, &blocked);
    }
    runs.retain(|(start, end)| end > start);
    runs
}

/// Which x intervals `exclusion` actually covers in the band `top..bottom`.
///
/// # Why this is exact rather than sampled
///
/// A polygon's edges are straight lines, so between two consecutive vertex rows the x of every
/// crossing is a **linear** function of y — and the extreme values of a linear function on an
/// interval are at its endpoints. Sampling at the band's own top and bottom **and at every polygon
/// vertex inside the band** therefore visits every extreme there is: no crossing can bulge further
/// out between two samples than it does at one of them. That is why this is not an approximation and
/// why a finer sample would find nothing new.
#[must_use]
pub fn span_in_band(exclusion: &Exclusion, top: Emu, bottom: Emu) -> Vec<(Emu, Emu)> {
    if exclusion.polygon.len() < 3 {
        return vec![(exclusion.left, exclusion.right)];
    }
    let mut samples: Vec<Emu> = vec![top, bottom];
    for &(_, y) in &exclusion.polygon {
        if y > top && y < bottom {
            samples.push(y);
        }
    }
    samples.sort_unstable();
    samples.dedup();

    let mut covered: Vec<(Emu, Emu)> = Vec::new();
    for sample in samples {
        // A band's bottom edge is exclusive: a polygon that ends exactly where the line begins does
        // not touch it. Nudging the last sample inward keeps a zero-height band from reporting the
        // whole outline.
        let at = if sample == bottom && bottom > top {
            bottom - Emu::from_emu(1)
        } else {
            sample
        };
        for interval in crossings(&exclusion.polygon, at) {
            covered.push(interval);
        }
    }
    if covered.is_empty() {
        return Vec::new();
    }
    let merged = merged(covered);
    if exclusion.through {
        merged
    } else {
        // `wp:wrapTight`: the outline, with nothing allowed inside it.
        let left = merged.first().map_or(exclusion.left, |run| run.0);
        let right = merged.last().map_or(exclusion.right, |run| run.1);
        vec![(left, right)]
    }
}

/// The x intervals the polygon covers on the scan row `y`, by the even-odd rule.
fn crossings(polygon: &[(Emu, Emu)], y: Emu) -> Vec<(Emu, Emu)> {
    let mut xs: Vec<i64> = Vec::new();
    for index in 0..polygon.len() {
        let (x0, y0) = polygon[index];
        let (x1, y1) = polygon[(index + 1) % polygon.len()];
        if y0 == y1 {
            continue;
        }
        let (top_y, bottom_y) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
        if y < top_y || y >= bottom_y {
            continue;
        }
        let span = (y1 - y0).emu();
        if span == 0 {
            continue;
        }
        let along = (y - y0).emu();
        let x = x0.emu() + (x1 - x0).emu().saturating_mul(along) / span;
        xs.push(x);
    }
    xs.sort_unstable();
    // Crossings come in pairs by the even-odd rule: an odd one out would mean an unclosed
    // polygon, and dropping it is what keeps a malformed file readable.
    xs.as_chunks::<2>()
        .0
        .iter()
        .map(|pair| (Emu::from_emu(pair[0]), Emu::from_emu(pair[1])))
        .collect()
}

/// `intervals`, sorted and with every overlap merged.
fn merged(mut intervals: Vec<(Emu, Emu)>) -> Vec<(Emu, Emu)> {
    intervals.sort_by_key(|run| run.0);
    let mut out: Vec<(Emu, Emu)> = Vec::with_capacity(intervals.len());
    for (start, end) in intervals {
        match out.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.maximum(end),
            _ => out.push((start, end)),
        }
    }
    out
}

/// `runs` with every interval of `blocked` removed.
fn subtract(runs: &[(Emu, Emu)], blocked: &[(Emu, Emu)]) -> Vec<(Emu, Emu)> {
    let mut out: Vec<(Emu, Emu)> = runs.to_vec();
    for &(start, end) in blocked {
        let mut next: Vec<(Emu, Emu)> = Vec::with_capacity(out.len() + 1);
        for &(run_start, run_end) in &out {
            if end <= run_start || start >= run_end {
                next.push((run_start, run_end));
                continue;
            }
            if start > run_start {
                next.push((run_start, start));
            }
            if end < run_end {
                next.push((end, run_end));
            }
        }
        out = next;
    }
    out
}

/// The largest-side rule, and the two one-sided ones.
///
/// `blocked` is what the object covered, so *left of the object* means *ending at or before its
/// leftmost blocked edge* — which is the only definition that survives a polygon, whose left edge is
/// not its bounding box's.
fn keep_side(runs: Vec<(Emu, Emu)>, side: WrapSide, blocked: &[(Emu, Emu)]) -> Vec<(Emu, Emu)> {
    let Some(&(object_left, _)) = blocked.first() else {
        return runs;
    };
    let Some(&(_, object_right)) = blocked.last() else {
        return runs;
    };
    match side {
        WrapSide::BothSides => runs,
        WrapSide::Left => runs
            .into_iter()
            .filter(|run| run.1 <= object_left)
            .collect(),
        WrapSide::Right => runs
            .into_iter()
            .filter(|run| run.0 >= object_right)
            .collect(),
        WrapSide::Largest => {
            let left: Emu = runs
                .iter()
                .filter(|run| run.1 <= object_left)
                .fold(Emu::ZERO, |widest, run| widest.maximum(run.1 - run.0));
            let right: Emu = runs
                .iter()
                .filter(|run| run.0 >= object_right)
                .fold(Emu::ZERO, |widest, run| widest.maximum(run.1 - run.0));
            // GUESS: a tie goes left. See this module's own documentation.
            if right > left {
                runs.into_iter()
                    .filter(|run| run.0 >= object_right)
                    .collect()
            } else {
                runs.into_iter()
                    .filter(|run| run.1 <= object_left)
                    .collect()
            }
        }
    }
}

/// The first run of `runs` at least `wanted` wide, or the widest one when none is.
///
/// A line is composed against **one** run — text does not jump a float and carry on beside it on the
/// same line — so this is how a line picks which of its available runs it lives in.
#[must_use]
pub fn run_for(runs: &[(Emu, Emu)], wanted: Emu) -> Option<(Emu, Emu)> {
    runs.iter()
        .copied()
        .find(|run| run.1 - run.0 >= wanted)
        .or_else(|| runs.iter().copied().max_by_key(|run| (run.1 - run.0).emu()))
}

/// The lowest edge of any exclusion that blocks the band `top..bottom` entirely, or `None` when
/// something still fits.
///
/// **This is what terminates the vertical search.** A line that has no run at all must move down,
/// and it must move to somewhere that is strictly lower than where it was — the bottom of whatever
/// blocked it. Every exclusion is finite and there are finitely many, so the search visits each at
/// most once and then stops.
#[must_use]
pub fn next_clear_edge(top: Emu, bottom: Emu, exclusions: &[Exclusion]) -> Option<Emu> {
    exclusions
        .iter()
        .filter(|exclusion| exclusion.meets(top, bottom))
        .map(|exclusion| exclusion.bottom)
        .filter(|edge| *edge > top)
        .min()
}
