//! **The gate this whole child exists to make possible**, and the way it goes vacuous.
//!
//! MJX-STAND-IN: this crate may not name `mjx-geometry` — `tests/the_seam_holds.rs` forbids it — so
//! the stand-in is the only `GeometryProvider` a painter's own suite can construct. Two painters
//! handed the same provider is what makes the comparison a comparison of rasterisers.
//!
//! # Why a second painter at all
//!
//! Two independent implementations agreeing is evidence. One implementation agreeing with itself is
//! not — it is true of every painter ever written, including one that draws nothing. So MJXOFF-164
//! *required* a software painter rather than permitting one, and this file is where the two are
//! made to answer the same question.
//!
//! # The specific way this gate degrades into nothing
//!
//! The `wgpu` painter is unavailable on plenty of machines: no adapter, no driver, no display
//! server. The obvious recovery is to carry on with the painter that is available — and at that
//! moment *"the painters agree"* has silently become *"`tiny-skia` agrees with itself"* and passes
//! for ever.
//!
//! Three things stop it, and they are deliberately at three different levels:
//!
//! 1. **The library refuses.** [`compare_painters`] answers [`PaintError::PaintersNotDistinct`] when
//!    both sides report the same [`Painter::name`], *before* either is asked to draw. A caller
//!    cannot reach the vacuous comparison at all, in this suite or in R10's.
//! 2. **The gate names both painters and both backends**, and prints them. A run that says
//!    *"`wgpu` (Vulkan on ...) against `tiny-skia` (none, CPU)"* cannot be mistaken for a run that
//!    skipped one.
//! 3. **A skip is loud.** `common::skip` prints the case's own name and the reason, and **fails**
//!    under `MJX_REQUIRE_GPU=1`, which is what continuous integration sets. The `render` job
//!    installs `lavapipe`, so an absence there is a failure and never a pass.
//!
//! [`its_own_refusal_is_proved`] is the fourth: it *demands* the refusal, by asking for exactly the
//! comparison that would be vacuous and asserting it is rejected. A gate that only ever ran the
//! valid case would not notice if the refusal were deleted.
//!
//! # What "agree" is allowed to mean
//!
//! Not byte equality. `wgpu` rasterises with four-sample multisampling and `tiny-skia` computes
//! analytic coverage, so the two differ along every antialiased edge **by design** — a comparison
//! that demanded equality could only be satisfied by making one painter a copy of the other, which
//! would destroy the independence the exercise is for.
//!
//! So the bound is on *area*: at most [`ALLOWED_DIFFERING_FRACTION`] of the page may be further
//! apart than [`mjx_paint::DEFAULT_CHANNEL_TOLERANCE`]. An edge is a line and a mistake is a region:
//! a shape drawn in the wrong place, in the wrong colour, or not at all moves percent, not
//! thousandths.
//!
//! # Proved by mutation
//!
//! * Passing the software painter as both sides → `its_own_refusal_is_proved` is the case, and the
//!   valid case fails with `PaintersNotDistinct` rather than passing.
//! * Removing `Placement::texels` from the software painter's glyph shading — mapping a glyph's quad
//!   onto the whole atlas page rather than onto its own rectangle — took the differing fraction from
//!   0.40 % to 1.51 %, which `ALLOWED_DIFFERING_FRACTION` refuses. **That is a real defect this
//!   gate found**, in this painter, before it was written down.
//! * Swapping the two pushes in `effect_steps`'s shadow arm in either painter → the shadow lands on
//!   top of its shape in one and behind it in the other, and the fraction goes to several percent.

mod common;

use mjx_paint::{
    compare_painters, PaintError, Painter, Render, ResourceFactory, Resources, SoftwarePainter,
    DEFAULT_CHANNEL_TOLERANCE,
};
use mjx_scene::{EffectKind, PlaceholderGeometry};

/// How much of a page the two painters may disagree about.
///
/// One and a half percent. Measured rather than picked: over `common::every_command` — a page with a
/// background, a sheared transform, a clip, an opacity group, a gradient, a hatch, a stroke, a drop
/// shadow, a picture and a run of glyphs on it — the two painters differ on **0.40 %** of pixels,
/// all of them on edges. The bound is a little under four times that, which leaves room for a
/// different GPU's sample positions and refuses anything structural: a shape in the wrong place, in
/// the wrong colour, or missing, moves several percent at least.
pub const ALLOWED_DIFFERING_FRACTION: f64 = 0.015;

/// A fresh set of resources, because an atlas delta is **taken** and not peeked.
///
/// Handing one source to both painters would tell the second that nothing changed, so it would draw
/// a page whose text has no pixels — and the comparison would then report a large, entirely
/// artificial disagreement over every word. See [`ResourceFactory`].
struct Fresh;

impl ResourceFactory for Fresh {
    fn render(
        &mut self,
        render: &mut dyn FnMut(&mut Resources<'_>) -> Result<Render, PaintError>,
    ) -> Result<Render, PaintError> {
        let mut glyphs = common::ChequeredAtlas::new();
        let images = common::OnePicture::new();
        let geometry = PlaceholderGeometry::new();
        let mut resources = Resources::new(&mut glyphs, &geometry, &images);
        render(&mut resources)
    }
}

#[test]
fn the_two_rasterisers_draw_the_same_page() {
    const CASE: &str = "the two rasterisers draw the same page";
    let mut software = SoftwarePainter::new();
    let mut gpu = match common::painter() {
        Ok(painter) => painter,
        Err(why) => {
            common::skip(CASE, &why);
            return;
        }
    };
    common::announce(CASE, &gpu);
    println!(
        "{CASE}: against painter `{}` on {}",
        software.name(),
        software.backend()
    );

    let list = common::every_command(200.0, 150.0);
    let agreement = compare_painters(
        &mut gpu,
        &mut software,
        &list,
        200,
        150,
        1.0,
        DEFAULT_CHANNEL_TOLERANCE,
        &mut Fresh,
        &mut Fresh,
    )
    .expect("two distinct painters render the same list");

    println!("{agreement}");

    // **Both painters, named.** A comparison that could not say which two ran would pass on a run
    // where one was skipped, which is the defect this whole file is written against.
    assert_eq!(agreement.left.0, "wgpu");
    assert_eq!(agreement.right.0, "tiny-skia");
    assert_ne!(
        agreement.left.1.api, agreement.right.1.api,
        "the two painters must be running on different things — {} against {}. If they are not, one \
         of them is not the painter it says it is.",
        agreement.left.1, agreement.right.1
    );

    // **And both drew.** Two blank pages agree perfectly, which is the second way this goes vacuous
    // after the two-painters-are-one way the library refuses outright.
    assert!(
        agreement.both_drew(),
        "one of the painters issued no draw calls at all: {:?}",
        agreement.drawn
    );
    assert!(
        agreement.drawn.0.commands == agreement.drawn.1.commands,
        "the two painters walked different numbers of commands ({} against {}), which means they \
         are not lowering the same list — the whole comparison rests on `plan_frame` being the one \
         lowering",
        agreement.drawn.0.commands,
        agreement.drawn.1.commands
    );

    assert!(
        agreement.within(ALLOWED_DIFFERING_FRACTION),
        "the painters disagree about {:.4}% of the page, which is more than the {:.2}% an \
         antialiasing difference accounts for. {agreement}",
        agreement.differing_fraction() * 100.0,
        ALLOWED_DIFFERING_FRACTION * 100.0
    );
}

#[test]
fn every_effect_kind_is_drawn_the_same_way_by_both() {
    const CASE: &str = "every effect kind is drawn the same way by both";
    let mut gpu = match common::painter() {
        Ok(painter) => painter,
        Err(why) => {
            common::skip(CASE, &why);
            return;
        }
    };
    common::announce(CASE, &gpu);

    // **Four of these seven arms had never executed anywhere before this child.** R08's own suite
    // constructed `OuterShadow` and `Blur`; `Glow`, `Reflection`, `SoftEdge` and `InnerShadow` were
    // written, reviewed, merged and never run. A disagreement here is more likely to be the GPU
    // painter than the processor.
    for kind in EffectKind::ALL {
        let mut software = SoftwarePainter::new();
        let list = common::one_shape_under(120.0, 100.0, kind);
        let agreement = compare_painters(
            &mut gpu,
            &mut software,
            &list,
            120,
            100,
            1.0,
            DEFAULT_CHANNEL_TOLERANCE,
            &mut Fresh,
            &mut Fresh,
        )
        .expect("two distinct painters render the same list");
        println!("{kind:?}: {agreement}");
        assert!(
            agreement.both_drew(),
            "{kind:?}: one of the painters drew nothing"
        );
        assert!(
            agreement.within(EFFECT_ALLOWED_DIFFERING_FRACTION),
            "{kind:?}: the painters disagree about {:.4}% of the page. {agreement}",
            agreement.differing_fraction() * 100.0
        );
    }
}

/// How much of a page the two may disagree about **on an effect**.
///
/// Wider than [`ALLOWED_DIFFERING_FRACTION`], and the reason is not slack. An effect is a
/// full-viewport convolution of a full-viewport target, so where the ordinary page has a *line* of
/// edge pixels, a blurred one has an *area* of them — every pixel a shadow reaches is a pixel whose
/// value came out of seventeen taps of an image the two rasterisers had already drawn slightly
/// differently, and the difference is spread rather than amplified. Six percent is measured against
/// the seven kinds; a wrong ordering still moves far more than that, because it changes which of two
/// opaque things is on top.
pub const EFFECT_ALLOWED_DIFFERING_FRACTION: f64 = 0.06;

#[test]
fn its_own_refusal_is_proved() {
    // The vacuous comparison, demanded rather than avoided. Without this case a deleted refusal
    // would be invisible: every other assertion in this file would still pass.
    let mut one = SoftwarePainter::new();
    let mut two = SoftwarePainter::new();
    let list = common::one_rectangle(
        40.0,
        40.0,
        mjx_scene::SceneRect::new(5.0, 5.0, 35.0, 35.0),
        common::rgb(0x20, 0x40, 0x80),
    );
    let error = compare_painters(
        &mut one,
        &mut two,
        &list,
        40,
        40,
        1.0,
        DEFAULT_CHANNEL_TOLERANCE,
        &mut Fresh,
        &mut Fresh,
    )
    .expect_err("comparing a painter with itself proves nothing and must be refused");
    match error {
        PaintError::PaintersNotDistinct { name } => assert_eq!(name, "tiny-skia"),
        other => panic!("the refusal must name what is wrong, and it said: {other}"),
    }
}

#[test]
fn a_painter_with_no_pixels_is_refused_rather_than_treated_as_equal() {
    // A document exporter drew a frame and has no pixels. A comparison that treated `None` as an
    // empty image would report perfect agreement with a blank page, which is the same defect as
    // comparing a painter with itself wearing a different hat.
    let mut svg = mjx_paint::SvgPainter::new();
    let list = common::one_rectangle(
        30.0,
        30.0,
        mjx_scene::SceneRect::new(2.0, 2.0, 28.0, 28.0),
        common::rgb(0x80, 0x20, 0x20),
    );
    let mut glyphs = common::ChequeredAtlas::new();
    let images = common::OnePicture::new();
    let geometry = PlaceholderGeometry::new();
    let mut resources = Resources::new(&mut glyphs, &geometry, &images);
    let error = mjx_paint::render_offscreen(&mut svg, &list, 30, 30, 1.0, &mut resources)
        .expect_err("an exporter has no pixels");
    match error {
        PaintError::NoPixels { name } => assert_eq!(name, "svg"),
        other => panic!("the refusal must name the painter, and it said: {other}"),
    }
}
