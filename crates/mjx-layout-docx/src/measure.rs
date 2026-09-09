//! Turning WordprocessingML's measures into [`Emu`], and the one rounding rule that has bitten
//! before.
//!
//! # Every `w:` length is a wire string
//!
//! `ST_TwipsMeasure`, `ST_SignedTwipsMeasure` and `ST_HpsMeasure` are all `xsd:union`s of a number
//! and a *universal measure* (`"0.5in"`, `"12pt"`, `"3mm"`), and `mjx-ooxml-types` keeps them as the
//! exact wire string because that is what fidelity requires. Turning one into a length is
//! `mjx_ooxml_types::support::universal_measure`'s job — MJXOFF-174 put it there rather than here
//! precisely because `mjx-docx`'s residency reads two of them and this crate reads five more, and
//! two parsers for one grammar is one parser too many.
//!
//! What is here is the *last* step: twips and half-points into [`Emu`], and the guards a document
//! can provoke.
//!
//! # A hairline can round to nothing
//!
//! `mjx-layout-xlsx`'s `border::half_of` learned this and it is the same arithmetic here: a border
//! or a paragraph rule whose width is floored at one EMU and then halved is **zero**, invisible
//! everywhere, on a path no test reaches unless somebody writes one. Word has paragraph borders
//! (`w:pBdr`, all six of them) and this crate places them, so [`half_of`] is here for the same
//! reason and `tests/no_rule_rounds_to_nothing.rs` holds it.

use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::shared::TwipsMeasure;
use mjx_ooxml_types::support::half_point_measure::half_points_from_wire;
use mjx_ooxml_types::support::universal_measure::twips_from_wire;
use mjx_ooxml_types::wordprocessingml::{EighthPointMeasure, HalfPointMeasure, SignedTwipsMeasure};

/// The narrowest visible line, in EMU.
///
/// One EMU is 1/914,400 inch and is far below one device pixel at any zoom a reader uses, so this
/// is not "one EMU": it is a quarter of a point, which is Word's own hairline and is a hair at 100 %
/// on a 96-DPI screen. **GUESS: Word's hairline is a quarter point.** ECMA-376 says a `w:sz` of `2`
/// is a quarter point and says nothing about what a renderer does with a border it rounds below
/// that; a stroke that vanishes is the failure this constant exists to prevent, so the floor is
/// visible rather than infinitesimal.
pub const HAIRLINE: Emu = Emu::from_emu(3175);

/// A `ST_TwipsMeasure` wire string as a length, or `None` when it is neither a number nor a
/// universal measure.
#[must_use]
pub fn twips_measure(value: &TwipsMeasure) -> Option<Emu> {
    twips_from_wire(value.to_wire()).map(Emu::from_twips)
}

/// The same for `ST_SignedTwipsMeasure`, which may be negative — a negative `w:ind@left` pulls a
/// paragraph into the margin, which real documents do.
#[must_use]
pub fn signed_twips_measure(value: &SignedTwipsMeasure) -> Option<Emu> {
    twips_from_wire(value.to_wire()).map(Emu::from_twips)
}

/// A `ST_HpsMeasure` wire string as a size in **points**, or `None` when it will not parse.
#[must_use]
pub fn half_points(value: &HalfPointMeasure) -> Option<f64> {
    #[allow(clippy::cast_precision_loss)]
    half_points_from_wire(value.to_wire()).map(|halves| halves as f64 / 2.0)
}

// `ST_SignedHpsMeasure` — `w:position`'s baseline offset — has **no** converter here, and that is
// deliberate rather than an omission: nothing in this child raises or lowers a baseline, so a
// converter for it would be a public function with no caller, which is the dead surface this project
// is written against. `w:position` and `w:vertAlign` arrive when something draws a superscript.

/// A border's `w:sz`, in eighths of a point, as a length — never narrower than [`HAIRLINE`].
///
/// `w:sz="0"` is legal and means *the thinnest line the renderer can draw*, not *no line*: the
/// absence of a line is `w:val="none"`, which is a different attribute. A zero here that produced a
/// zero-width stroke would silently drop a border a reader can see in Word.
#[must_use]
pub fn border_width(value: Option<&EighthPointMeasure>) -> Emu {
    let eighths = value
        .and_then(|measure| i64::try_from(*measure).ok())
        .unwrap_or(0);
    let width = Emu::from_points(
        #[allow(clippy::cast_precision_loss)]
        {
            eighths as f64 / 8.0
        },
    );
    width.maximum(HAIRLINE)
}

/// Half of `width`, but never nothing.
///
/// A border sits **astride** the box's edge, so each side of it is half the stroke — and a stroke
/// already floored at one EMU halves to zero, which draws nothing at all with no error anywhere.
/// This is `mjx-layout-xlsx`'s `border::half_of` run again on Word's paragraph rules and table
/// borders, and it is written out rather than shared because the two crates sit at the same rank and
/// an edge between them is refused by name.
#[must_use]
pub fn half_of(width: Emu) -> Emu {
    if width <= Emu::ZERO {
        return Emu::ZERO;
    }
    let half = width.divided_by(2);
    if half <= Emu::ZERO {
        // The stroke is one EMU or narrower and still visible, so its half must be too. Rounding
        // down here is what makes a hairline disappear.
        return Emu::from_emu(1);
    }
    half
}
