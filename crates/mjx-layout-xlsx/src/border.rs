//! Border bands: the rectangles a cell's four edges are drawn as.
//!
//! # Why an edge is a filled band rather than a stroke
//!
//! R16 put a cell's four edges on its [`Decoration`](crate::model::Decoration) and stopped there,
//! which was right for a fragment-tier child and is **not consumable by a display list**. A
//! `mjx_scene::Decoration` carries exactly **one** stroke, and a cell has four edges that differ
//! in weight, colour and style; a resolver handed those four values can only pick one of them, and
//! picking one draws it on all four sides. MJXOFF-244 found that while planning the companion, and
//! the answer is the one `mjx-layout-pptx` already reached for a table cell: **each edge is its own
//! fragment** — a box the width of the line, filled with the line's colour.
//!
//! That is the only shape expressible in `mjx-layout`'s closed six-kind vocabulary that draws the
//! right colour at the right place at the right weight for every edge independently.
//!
//! # ⚠ What a band loses, and what keeps the loss recoverable
//!
//! **A filled band is solid**, so `dashed`, `dotted`, `dashDot`, `dashDotDot`, `mediumDashed`,
//! `mediumDashDot`, `mediumDashDotDot` and `slantDashDot` are all drawn as a solid line of the right
//! weight. A solid border of the right colour and weight is a far smaller error than no border, and
//! the alternative — a seventh fragment kind for an edge — is a change to every consumer of a
//! vocabulary `mjx-layout` closed on purpose.
//!
//! The information is **not** thrown away: the band's decoration carries the whole
//! [`BorderEdge`], style included, so the child that draws a dash has the
//! value waiting. The route it will take is a [`mjx_layout::ShapeFragment`] whose
//! `mjx_scene::GeometryProvider` answers with the edge's centre line and whose
//! decoration carries a dashed stroke; that is a design change to two crates rather than a value
//! this one failed to record.
//!
//! `crates/mjx-scene-xlsx/tests/the_dash_is_lost_at_the_band.rs` asserts the loss rather than
//! describing it, so that closing it is a test going red and being deleted.
//!
//! # ⚠ A hairline that rounds to nothing is the failure this module is written against
//!
//! R15 found `mjx-layout-pptx` flooring a `@w="0"` hairline at **one EMU** and then halving it to
//! zero — a border invisible at every zoom, in every export, with nothing anywhere to notice it.
//! **Excel is made of hairline borders**, so the same defect here would be worth more. Two things
//! stop it:
//!
//! * every width in [`band_width`] is a positive number of points and the match is **exhaustive**,
//!   so a `BorderStyle` added to the schema fails this file to compile rather than defaulting to
//!   zero; and
//! * `tests/no_border_rounds_to_nothing.rs` walks every variant of `BorderStyle`, asserts a
//!   strictly positive width *and* a band rectangle of strictly positive area, and
//!   `mjx-reference-pack`'s end-to-end gate asserts that a hairline-bordered cell puts ink on the
//!   page.
//!
//! # ⚠ Every width here is a GUESS
//!
//! ECMA-376 names the fourteen `ST_BorderStyle` tokens and defines the geometry of **none** of
//! them: §18.18.3 is a list of names. So the weights below are a reading — Excel draws `thin` at one
//! screen pixel, `medium` at two and `thick` at three at 100 %, which at 96 dpi is 0.75 pt, 1.5 pt
//! and 2.25 pt — and every one is marked at its site. Confirmation is the Windows sitting
//! (`docs/validation/07-the-reference-pack.md`); LibreOffice is a change detector and not a
//! reference.

use mjx_layout::LayoutRect;
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::spreadsheetml::BorderStyle;

use crate::merge::RegionEdge;
use crate::model::{BorderEdge, CellBorders};

/// One edge of one cell, as a rectangle to fill.
///
/// A `double` edge produces **two** of these for the same [`RegionEdge`]; everything else produces
/// one. See [`bands`].
#[derive(Clone, PartialEq, Debug)]
pub struct BorderBand {
    /// Which side of the cell it draws.
    pub edge: RegionEdge,
    /// Where it is, in the same coordinates the cell's own rectangle is in.
    pub rect: LayoutRect,
    /// The style and colour the file stated, carried whole so that the layer above has the
    /// information a band cannot draw.
    pub stated: BorderEdge,
}

/// How wide `style` is drawn.
///
/// GUESS, every arm. ECMA-376 §18.18.3 lists the fourteen tokens and defines no geometry for any of
/// them, so these are a reading of what Excel draws at 100 % on a 96 dpi screen: `thin` one pixel,
/// `medium` two, `thick` three, `hair` thinner than `thin`. A pixel at 96 dpi is 0.75 pt.
///
/// **Never zero, and never one EMU.** See the module documentation: a band of zero width covers no
/// pixels, and halving a one-EMU band rounds to zero, which is how a table of hairline borders
/// draws none of them with nothing to notice.
///
/// [`BorderStyle::None`] answers [`Emu::ZERO`] because it is not drawn at all — the one arm whose
/// zero is a decision rather than an accident, and [`bands`] never reaches it because
/// [`edge_of`](crate::model) drops a `none` edge before a decoration is built.
#[must_use]
pub fn band_width(style: BorderStyle) -> Emu {
    match style {
        // Not drawn. The only zero in this table, and it is unreachable from `bands`.
        BorderStyle::None => Emu::ZERO,
        // GUESS: the thinnest weight Excel's border gallery offers, drawn thinner than `thin` so
        // that the two are told apart. Half a point is two thirds of a pixel at 96 dpi, which the
        // painter draws with partial coverage rather than dropping — the end-to-end gate asserts
        // exactly that, because "thinner than a pixel" is where a hairline goes missing.
        BorderStyle::Hair => Emu::from_points(0.5),
        // GUESS: one screen pixel at 96 dpi. The dashed family at this weight draws solid; see the
        // module documentation for what that costs and why the style is still carried.
        BorderStyle::Thin
        | BorderStyle::Dashed
        | BorderStyle::Dotted
        | BorderStyle::DashDot
        | BorderStyle::DashDotDot => Emu::from_points(0.75),
        // GUESS: two screen pixels, which is what `medium` means beside `thin`.
        BorderStyle::Medium
        | BorderStyle::MediumDashed
        | BorderStyle::MediumDashDot
        | BorderStyle::MediumDashDotDot
        | BorderStyle::SlantDashDot => Emu::from_points(1.5),
        // GUESS: three screen pixels.
        BorderStyle::Thick => Emu::from_points(2.25),
        // A `double` is two thin lines with a gap; this is the width of **one** of them, and
        // [`bands`] is what emits the pair. See [`DOUBLE_SEPARATION`].
        BorderStyle::Double => DOUBLE_LINE_WIDTH,
    }
}

/// How wide each of a `double` edge's two lines is.
///
/// GUESS: the same weight as `thin`, which is what a double rule looks like in Excel's gallery.
const DOUBLE_LINE_WIDTH: Emu = Emu::from_emu(9_525);

/// How far apart the centres of a `double` edge's two lines sit.
///
/// GUESS: one line width of clear space between them, so the pair occupies the same total as
/// `thick` — which is what makes a double border read as heavier than a single one at a glance.
const DOUBLE_SEPARATION: Emu = Emu::from_emu(19_050);

/// Every band `borders` draws around `rect`, in [`RegionEdge::ALL`] order.
///
/// Empty for a cell with no stated border, which is the overwhelming majority of a worksheet — the
/// allocation only happens for a cell that has one.
///
/// GUESS: a band is **centred on the cell's own edge**, so two adjacent cells that both state a
/// border draw it in the same place rather than side by side. That is what makes a grid of bordered
/// cells look like a grid rather than like a grid drawn twice, and whether Excel biases the line to
/// one side of the gridline is a question for the Windows sitting.
#[must_use]
pub fn bands(rect: LayoutRect, borders: &CellBorders) -> Vec<BorderBand> {
    let mut out = Vec::new();
    for edge in RegionEdge::ALL {
        let Some(stated) = borders.edge(edge) else {
            continue;
        };
        if stated.style == BorderStyle::Double {
            let offset = DOUBLE_SEPARATION.divided_by(2);
            for shift in [-offset, offset] {
                out.push(BorderBand {
                    edge,
                    rect: band_rect(rect, edge, shift, DOUBLE_LINE_WIDTH),
                    stated: stated.clone(),
                });
            }
            continue;
        }
        let width = band_width(stated.style);
        if width <= Emu::ZERO {
            continue;
        }
        out.push(BorderBand {
            edge,
            rect: band_rect(rect, edge, Emu::ZERO, width),
            stated: stated.clone(),
        });
    }
    out
}

/// The rectangle a band of `width` occupies on `edge` of `rect`, its centre moved by `shift`.
///
/// `shift` is along the axis the band is thin in, and it is signed the same way for all four edges
/// — a `double` edge's two lines straddle the cell's edge symmetrically, so neither line has to be
/// called the inner one.
fn band_rect(rect: LayoutRect, edge: RegionEdge, shift: Emu, width: Emu) -> LayoutRect {
    let half = half_of(width);
    match edge {
        RegionEdge::Left => LayoutRect::from_edges(
            rect.left + shift - half,
            rect.top,
            rect.left + shift + half,
            rect.bottom,
        ),
        RegionEdge::Right => LayoutRect::from_edges(
            rect.right + shift - half,
            rect.top,
            rect.right + shift + half,
            rect.bottom,
        ),
        RegionEdge::Top => LayoutRect::from_edges(
            rect.left,
            rect.top + shift - half,
            rect.right,
            rect.top + shift + half,
        ),
        RegionEdge::Bottom => LayoutRect::from_edges(
            rect.left,
            rect.bottom + shift - half,
            rect.right,
            rect.bottom + shift + half,
        ),
    }
}

/// Half of `width`, never rounded down to nothing.
///
/// **This function is the R15 defect, written out so it cannot recur.** `Emu::divided_by(2)` on a
/// one-EMU width answers zero, and a band whose two edges are the same coordinate covers no area at
/// all — so the thinnest border in the file becomes the one that is not drawn. Every width
/// [`band_width`] produces is far above that today; this floor is what keeps that true if one of
/// them is ever tuned downward.
fn half_of(width: Emu) -> Emu {
    let half = width.divided_by(2);
    if half <= Emu::ZERO {
        Emu::from_emu(1)
    } else {
        half
    }
}
