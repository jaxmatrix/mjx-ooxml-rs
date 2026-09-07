//! Shape + extents + adjustments → guides → `DrawCommand`s → device pixels.
//!
//! # The environment is built the way the format builds it
//!
//! ECMA-376 Part 1 §20.1.9.11 makes declaration order evaluation order, and `mjx-dml`'s
//! [`PresetGeometry::adjustments_for_size`](mjx_dml::geometry::PresetGeometry::adjustments_for_size)
//! already builds a preset's environment in exactly two stages: every adjustment bound to its
//! current value — an `a:avLst` override where there is one and the generated default otherwise —
//! and then the shape's own `gdLst`, in order. `guide_environment` is that, with two differences,
//! both of which are what MJXOFF-201 is about:
//!
//! * the **whole** `gdLst` from [`crate::table`], instead of the subset an adjustment's domain
//!   needed; and
//! * the **whole** `a:avLst`, instead of the handled subset
//!   [`mjx_ooxml_types::drawingml::adjustments_of`] exposes. An `avLst` entry no
//!   adjust handle references is not a user-facing adjustment — and it is still a name the shape's
//!   `gdLst` reads. `pentagon` has two of them and cannot evaluate its first guide without them.
//!
//! Note what is **not** here: no clamping of an adjustment into its domain. The shapes do that
//! themselves, in their own first guide — `a = pin 0 adj 50000` — because that is where the format
//! puts it, and a second clamp up here would silently disagree with a shape whose bound is a guide.
//!
//! # Two coordinate spaces, one affine map
//!
//! Guides evaluate in the shape's **own** space, whose `w` and `h` are the extents the document
//! states (`a:xfrm/a:ext`). A path may then declare a coordinate box of its own (`@w`/`@h`), in
//! which case its points are fractions of that box rather than lengths in the shape's space. Either
//! way the result is mapped onto the device-pixel rectangle the seam supplied, by one affine map
//! per path.
//!
//! **Why the extents are taken and not inferred from the pixel box.** They could have been: a fixed
//! number of EMU per device pixel would make the provider a pure function of the preset and the
//! box. It would also be wrong twice. A guide such as `sqrt` is not homogeneous in `w` and `h`, so
//! a shape using one would resolve differently at different zoom levels; and every coordinate
//! `mjx-dml` resolves is rounded to a whole [`mjx_ooxml_core::measure::Emu`], so a shape
//! resolved in a space a hundred and twenty units wide would be quantised to a hundred and
//! twentieth of its own width. Resolving in the document's own units and mapping afterwards has
//! neither problem, and the caller registering a handle always knows the extents.
//!
//! # One shape, several contours, and one command list
//!
//! An `a:pathLst` is a *list*, and each `a:path` in it declares its own `@fill` and `@stroke`. That
//! is not decoration: `arc` draws its visible curve in a `fill="none"` path and its filled body in
//! a `stroke="false"` sibling, and treating the two alike fills an open contour or outlines a
//! region nothing asked to be outlined.
//!
//! So this module answers twice. [`contours_of_definition`] is the full answer — one
//! [`PresetContour`] per `a:path`, each carrying its own treatment — and it is what a scene builder
//! that wants to paint a preset correctly should read. [`outline_of_definition`] is the projection
//! onto what the display-list seam takes, which is *one* command list and *one*
//! [`FillRule`]: it concatenates the contours and **drops the ones that draw nothing at all**,
//! neither filled nor stroked. Exactly one path in ECMA-376's geometry file is in that state, and
//! `tests/the_flags_reach_a_consumer.rs` names it.
//!
//! The projection is lossy and stated to be: two contours with different fill treatments arrive at
//! the seam as one list, because [`ResolvedOutline`] has nowhere to put the difference. Widening
//! that is `mjx-scene`'s decision and not this crate's — see MJXOFF-211.

use std::collections::HashSet;

use mjx_dml::geometry::{
    Emu, GuideContext, GuideError, ResolvedDrawCommand, ResolvedGuides, ResolvedPoint,
    ResolvedRectangle, Size,
};
use mjx_ooxml_core::measure::Angle;
use mjx_ooxml_types::drawingml::{
    adjustments_of, AdjustmentBound, AdjustmentSpec, PathFillMode, PresetShapeType,
};
use mjx_scene::{FillRule, OutlineProvenance, PathCommand, ResolvedOutline, ScenePoint, SceneRect};

use crate::arc::{arc_to_cubics, ShapePoint};
use crate::provider::AdjustmentOverride;
use crate::table::{definition_of, PresetPath, PresetShapeDefinition};
use crate::GeometryError;

/// One adjustment of a shape, with its value and the numeric domain that value is allowed to move
/// in at a given size.
///
/// The same thing `mjx-dml`'s [`BoundedAdjustment`](mjx_dml::geometry::BoundedAdjustment) is, and
/// it exists here rather than being reused for one reason: `mjx-dml`'s is computed against the
/// *bound* guides alone, because that is all the generated table held. This one is computed against
/// the shape's whole `gdLst`, so a bound that depends on a guide the subset omitted still resolves.
/// Every field is in native spec units — fractions in 1000ths of a percent, angles in 60000ths of a
/// degree.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AdjustmentDomain {
    /// The adjustment's static metadata, from the generated table.
    pub spec: &'static AdjustmentSpec,
    /// The value in effect: the override if there is one, else the spec's default.
    pub value: f64,
    /// Whether [`value`](Self::value) came from an override rather than from the default.
    pub is_overridden: bool,
    /// The domain's lower bound, evaluated at this size.
    pub minimum: f64,
    /// The domain's upper bound, evaluated at this size.
    pub maximum: f64,
}

/// The guide environment a preset's paths resolve against — its adjustments, then its whole
/// `gdLst` — together with the names of the guides that had no finite value here.
///
/// # Errors
///
/// [`GeometryError::Guides`] naming the first guide that would not evaluate. A guide that
/// evaluates to no finite number is **not** one of those: it is reported in the second half of the
/// answer instead, for the reason spelled out in the body.
pub(crate) fn guide_environment(
    definition: &PresetShapeDefinition,
    extents: Size,
    adjustments: &[AdjustmentOverride],
) -> Result<(ResolvedGuides<'static>, HashSet<&'static str>), GeometryError> {
    let shape = definition.preset.to_wire();
    let mut environment = ResolvedGuides::new(GuideContext::from_size(extents));

    // **The whole `a:avLst`, not the handled subset.** `adjustments_of` holds only the entries some
    // adjust handle references; the ones it drops are constants *to the user* and are still names
    // the shape's own `gdLst` reads. `pentagon`'s first guide is `*/ wd2 hf 100000`, and `hf` is
    // exactly such an entry — seeding from the handled subset alone leaves nine shapes unable to
    // evaluate a single guide. Evaluated rather than parsed: an `avLst` entry is a `a:gd` like any
    // other, and `val N` is the formula language's own literal.
    environment
        .extend(
            definition
                .adjustment_values
                .iter()
                .map(|value| (value.wire_name, value.formula)),
        )
        .map_err(|source| GeometryError::Guides { shape, source })?;

    // The document's `a:avLst` overrides, on top of the file's seeds. Filtered to names the shape
    // actually declares, so a caller cannot introduce a guide the preset does not have — an
    // override is a *re*-statement of one of the shape's own adjustments and nothing else.
    for override_ in adjustments {
        if definition
            .adjustment_values
            .iter()
            .any(|value| value.wire_name == override_.wire_name)
        {
            environment.define(override_.wire_name.clone(), override_.value);
        }
    }

    // **The `gdLst`, one guide at a time, because some of them have poles.** ECMA-376's own
    // formulas divide and take square roots, and at the ends of an adjustment's domain the divisor
    // can be zero: `circularArrow`'s `dxF1 = "+/ q11 q10 q4"` has no value at `adj5 = 0`, which is
    // that adjustment's own *minimum* and therefore a place a handle drag reaches. `mjx-dml`
    // refuses a non-finite guide, rightly — an infinity in a display list is worse than a wrong
    // number — but refusing the whole *shape* over it is wrong twice over: sixty (shape, size,
    // adjustment) combinations across ten presets are in that position, and in most of them the
    // guide with no value is one the shape's paths never read. **Which shapes are singular depends
    // on who is reading**, and MJXOFF-204 measured all three answers: six presets have a singular
    // *path*, three a singular *text rectangle* (all in `il`, all at an `adj2` of zero, and all
    // three draw perfectly well there), six a singular *connection site*. `noSmoking` is in the
    // first list alone and `parallelogram` in the third alone.
    //
    // So a guide with no finite value is left *undefined* rather than fatal, and so is every later
    // guide that names it — an undefined name propagates exactly as far as it is used and no
    // further. Nothing is invented and nothing is silenced: if a path then names one of them,
    // `emit_path` fails with [`GeometryError::PathCommand`], which is the loud answer, and it names
    // the guide. Every other guide failure — a malformed formula, a name nothing defines — stays
    // fatal here, because those are table defects rather than singularities.
    let mut singular: HashSet<&'static str> = HashSet::new();
    for guide in definition.guides {
        if guide
            .formula
            .split_whitespace()
            .skip(1)
            .any(|argument| singular.contains(argument))
        {
            singular.insert(guide.wire_name);
            continue;
        }
        match environment.extend(std::iter::once((guide.wire_name, guide.formula))) {
            Ok(()) => {}
            Err(error) if is_a_singularity(&error) => {
                singular.insert(guide.wire_name);
            }
            Err(source) => return Err(GeometryError::Guides { shape, source }),
        }
    }
    Ok((environment, singular))
}

/// Whether a guide failure is the shape's own arithmetic leaving the reals, rather than a defect in
/// the table.
///
/// [`GuideError::NotFinite`] is a division by zero, a square root of a negative or an overflow —
/// something a *correct* formula does at a particular size and adjustment. A malformed formula or
/// an undefined name is not: those say the table is wrong, and [`guide_environment`] keeps them
/// fatal.
fn is_a_singularity(error: &GuideError) -> bool {
    match error {
        GuideError::Guide { source, .. } => is_a_singularity(source),
        GuideError::NotFinite { .. } => true,
        GuideError::Malformed { .. } | GuideError::UndefinedGuide { .. } => false,
    }
}

/// An adjustment's current value, and whether it was overridden.
fn value_of(spec: &AdjustmentSpec, adjustments: &[AdjustmentOverride]) -> (f64, bool) {
    match adjustments
        .iter()
        .find(|override_| override_.wire_name == spec.wire_name)
    {
        Some(override_) => (override_.value, true),
        None => (f64::from(spec.default), false),
    }
}

/// A domain bound as a number: a literal as itself, a guide name looked up in the environment.
fn bound(
    kind: AdjustmentBound,
    environment: &ResolvedGuides<'_>,
    shape: &'static str,
) -> Result<f64, GeometryError> {
    match kind {
        AdjustmentBound::Literal(literal) => Ok(f64::from(literal)),
        AdjustmentBound::Guide(name) => environment
            .resolve(name)
            .map_err(|source| GeometryError::Guides { shape, source }),
    }
}

/// Every adjustment a preset exposes, with its value and its evaluated domain at `extents`.
///
/// Empty for a shape with no adjustments (`rect`, `ellipse`). This is what a sweep over an
/// adjustment's whole range needs in order to know what the range *is*, and
/// `tests/an_adjustment_moves_the_shape.rs` is its consumer.
///
/// # Errors
///
/// [`GeometryError::UnseededShape`] if this build has no table for the preset, and
/// [`GeometryError::Guides`] if the shape's guide list or one of its bounds would not evaluate.
///
/// **A degenerate size is a failure here, and is not one for [`preset_outline`]**, which is a
/// deliberate difference rather than an inconsistency. A shape with a zero side makes `ss` zero and
/// `*/ 100000 w ss` infinite, so `rightArrow`'s `maxAdj2` has no value — and *"what is this
/// adjustment's domain"* genuinely has no answer at that size, while *"what does this shape draw"*
/// has an obvious one: nothing.
pub fn adjustment_domains(
    preset: PresetShapeType,
    extents: Size,
    adjustments: &[AdjustmentOverride],
) -> Result<Vec<AdjustmentDomain>, GeometryError> {
    let definition = definition_of(preset).ok_or(GeometryError::UnseededShape {
        shape: preset.to_wire(),
    })?;
    let (environment, _) = guide_environment(definition, extents, adjustments)?;
    let shape = preset.to_wire();
    adjustments_of(preset)
        .iter()
        .map(|spec| {
            let (value, is_overridden) = value_of(spec, adjustments);
            Ok(AdjustmentDomain {
                spec,
                value,
                is_overridden,
                minimum: bound(spec.min, &environment, shape)?,
                maximum: bound(spec.max, &environment, shape)?,
            })
        })
        .collect()
}

/// The outline a preset shape draws, in the device pixels of `within`.
///
/// The whole pipeline in one call: the guide environment, the table's steps as
/// [`DrawCommand`](mjx_dml::geometry::DrawCommand)s, `mjx-dml`'s resolver, this crate's arc
/// decomposition, and the affine map onto `within`. The answer carries
/// [`OutlineProvenance::Document`] — never [`Placeholder`](OutlineProvenance::Placeholder) — which
/// is the field R10 gates a fidelity render on.
///
/// # Errors
///
/// [`GeometryError::UnseededShape`] for a preset this build has no table for,
/// [`GeometryError::Guides`] for a guide list that will not evaluate, and
/// [`GeometryError::PathCommand`] for a coordinate naming a guide the shape does not define.
pub fn preset_outline(
    preset: PresetShapeType,
    extents: Size,
    adjustments: &[AdjustmentOverride],
    within: SceneRect,
) -> Result<ResolvedOutline, GeometryError> {
    let definition = definition_of(preset).ok_or(GeometryError::UnseededShape {
        shape: preset.to_wire(),
    })?;
    outline_of_definition(definition, extents, adjustments, within)
}

/// One `a:path` of a preset, resolved into device pixels together with the treatment ECMA-376
/// declares for it.
///
/// The full answer [`ResolvedOutline`] cannot hold: the seam carries one command list and one
/// [`FillRule`] per outline, and a preset shape is a *list* of paths each of which says for itself
/// whether it is filled, how, and whether it is stroked.
#[derive(Clone, PartialEq, Debug)]
pub struct PresetContour {
    /// The steps, in the device pixels of the `within` rectangle they were resolved against.
    pub commands: Vec<PathCommand>,
    /// How this contour is filled (`a:path@fill`).
    pub fill: PathFillMode,
    /// Whether this contour is stroked (`a:path@stroke`).
    pub stroke: bool,
    /// Whether this contour may be extruded in 3-D (`a:path@extrusionOk`).
    ///
    /// Carried through from the table and acted on by nobody in this crate — MJXOFF-201 §7 puts
    /// 3-D outside this epic, and MJXOFF-211 names its reader.
    pub extrusion_ok: bool,
}

impl PresetContour {
    /// Whether this contour contributes a filled region.
    #[must_use]
    pub fn is_filled(&self) -> bool {
        self.fill != PathFillMode::None
    }

    /// Whether this contour draws nothing at all — neither filled nor stroked.
    ///
    /// [`outline_of_definition`] leaves such a contour out of the single command list it builds,
    /// which is what makes `@fill` and `@stroke` flags something acts on. One path of one shape in
    /// ECMA-376's geometry file answers `true`.
    #[must_use]
    pub fn draws_nothing(&self) -> bool {
        !self.is_filled() && !self.stroke
    }
}

/// Every contour a preset shape draws, each with its own fill and stroke treatment.
///
/// # Errors
///
/// As [`preset_outline`].
pub fn preset_contours(
    preset: PresetShapeType,
    extents: Size,
    adjustments: &[AdjustmentOverride],
    within: SceneRect,
) -> Result<Vec<PresetContour>, GeometryError> {
    let definition = definition_of(preset).ok_or(GeometryError::UnseededShape {
        shape: preset.to_wire(),
    })?;
    contours_of_definition(definition, extents, adjustments, within)
}

/// Every contour a *given* definition draws — [`preset_contours`] without the table lookup.
///
/// **A shape whose extents have no area draws no contours at all**, which is the one empty answer
/// in this crate and is not a failure; [`outline_of_definition`] says why at length.
///
/// # Errors
///
/// As [`outline_of_definition`].
pub fn contours_of_definition(
    definition: &PresetShapeDefinition,
    extents: Size,
    adjustments: &[AdjustmentOverride],
    within: SceneRect,
) -> Result<Vec<PresetContour>, GeometryError> {
    match resolve_contours(definition, extents, adjustments, within) {
        Ok(contours) => Ok(contours),
        // **A shape with no extent, whose own formulas divided by it.** `ss` is `min(w, h)`, so a
        // shape with a zero side makes `*/ 100000 w ss` infinite and `mjx-dml`'s evaluator refuses
        // a non-finite guide — correctly, because a non-finite coordinate in a display list is
        // worse than a wrong one. A path then naming that guide fails the same way, one step later.
        //
        // Neither is a defect in the table and neither must fail the page. A `<a:ext cx="0"
        // cy="0"/>` is a real thing a real deck contains (a collapsed placeholder, a shape scaled
        // to nothing by an animation), and there is exactly one right answer for it: **no
        // commands**. This is not the *"silently drew nothing"* failure the seam warns about —
        // that one is a shape with area whose geometry could not be found — it is a shape with no
        // area, and `PlaceholderGeometry` answers a zero-size box the same way.
        //
        // The condition is on the *extents* rather than on the error, so a table defect at an
        // ordinary size still fails loudly: only a shape that has nothing to draw is allowed to
        // draw nothing. Note that most shapes reach here *without* failing — `rect` has no guides
        // at all and `triangle`'s multiply through by a zero width — and those take the ordinary
        // path, where `scale` collapses them onto the box's corner.
        Err(error) if !extents_have_area(extents) => {
            let _ = error;
            Ok(Vec::new())
        }
        Err(error) => Err(error),
    }
}

/// [`contours_of_definition`] without the degenerate-size answer.
fn resolve_contours(
    definition: &PresetShapeDefinition,
    extents: Size,
    adjustments: &[AdjustmentOverride],
    within: SceneRect,
) -> Result<Vec<PresetContour>, GeometryError> {
    let (environment, singular) = guide_environment(definition, extents, adjustments)?;
    let mut contours = Vec::with_capacity(definition.paths.len());
    for path in definition.paths {
        let map = ShapeToDevice::new(within, path, extents);
        let mut commands = Vec::new();
        emit_path(
            definition,
            path,
            &environment,
            &singular,
            &map,
            &mut commands,
        )?;
        contours.push(PresetContour {
            commands,
            fill: path.fill,
            stroke: path.stroke,
            extrusion_ok: path.extrusion_ok,
        });
    }
    Ok(contours)
}

/// The outline a *given* definition draws — [`preset_outline`] without the table lookup.
///
/// Public for two reasons, both of them gates rather than conveniences. A suite that wants to prove
/// a bounding-box tolerance is not vacuous has to resolve a *deliberately wrong* shape and show the
/// tolerance catches it, and a table it can only read cannot be made wrong. And a path's `@w`/`@h`
/// coordinate box is a branch of `ShapeToDevice::new` a suite must be able to take with a
/// non-identity value, which MJXOFF-155 §6's identity-value probe exists to refuse leaving untaken.
///
/// **What it does with the contours' flags.** It concatenates every contour
/// [`contours_of_definition`] produced *except* the ones that
/// [`draw nothing`](PresetContour::draws_nothing) — neither filled nor stroked. That is the only
/// reduction available without losing geometry: a `fill="none"` contour still has to be stroked and
/// a `stroke="false"` one still has to be filled, so both stay, and the difference between them is
/// what the seam cannot carry.
///
/// # Errors
///
/// As [`preset_outline`], minus [`GeometryError::UnseededShape`] which cannot arise.
pub fn outline_of_definition(
    definition: &PresetShapeDefinition,
    extents: Size,
    adjustments: &[AdjustmentOverride],
    within: SceneRect,
) -> Result<ResolvedOutline, GeometryError> {
    let mut commands = Vec::new();
    for contour in contours_of_definition(definition, extents, adjustments, within)? {
        if contour.draws_nothing() {
            continue;
        }
        commands.extend(contour.commands);
    }

    Ok(ResolvedOutline {
        commands,
        // DrawingML fills by the non-zero winding rule; the even-odd rule is what the *placeholder*
        // uses, to make its hollow frame read as a frame.
        fill_rule: FillRule::NonZero,
        label: definition.preset.to_wire().to_owned(),
        provenance: OutlineProvenance::Document,
    })
}

// -------------------------------------------------------------------------------------------
// The text rectangle (`a:rect`)
// -------------------------------------------------------------------------------------------

/// Where text goes inside a preset shape — or why there is no answer.
///
/// # Why this is four answers and not a rectangle
///
/// A shape's `a:rect` is what keeps a rounded rectangle's text off its corners, a chevron's out of
/// its notch and a callout's out of its tail. The tempting signature is
/// `fn text_rectangle(..) -> SceneRect`, falling back to the shape's own box wherever the answer is
/// missing — **and that is the exact defect MJXOFF-201 §6 is about.** Text would still render, in
/// the wrong place, and every test that asks *"did text appear"* would pass. So the three ways of
/// having no answer are three *values*, each of which a caller has to look at:
///
/// * [`NotDeclared`](Self::NotDeclared) — the shape says nothing. Five of the 186 (`chartPlus`,
///   `chartStar`, `chartX`, `line`, `lineInv`): three tick marks and two bare lines.
/// * [`Singular`](Self::Singular) — the shape says, and one of the guides it says it in has no
///   finite value at this size and these adjustments. **Four presets are singular *only* here**,
///   which is why `mjx-dml`'s evaluator is run one guide at a time: `leftRightUpArrow`,
///   `leftUpArrow` and `quadArrow` lose `il`, and `parallelogram` loses the `q3` its `il` is built
///   from, each at one end of one adjustment's own domain. Their paths draw perfectly well there.
/// * [`Inverted`](Self::Inverted) — the shape says, the guides all have values, and the edges have
///   **crossed**. There is no text area at all, and the crossed rectangle is a diagnosis rather
///   than a box.
///
/// [`or_bounding_box`](Self::or_bounding_box) is the fallback, and it is a **named call** rather
/// than a default: a caller that lays text against the shape's own box where there is no rectangle
/// has said so at the call site, in one word a reviewer can grep for.
#[derive(Clone, PartialEq, Debug)]
pub enum TextRectangle {
    /// The shape declares an `a:rect` and it resolved, in the device pixels of the box the shape
    /// was drawn in.
    Declared(SceneRect),
    /// The shape declares no `a:rect` at all.
    NotDeclared,
    /// The shape declares one, and a guide it is written in has no finite value here.
    Singular {
        /// The guide whose own formula has no finite value at this size and these adjustments.
        guide: String,
    },
    /// The shape declares one and its edges have crossed — left past right, or top past bottom.
    Inverted {
        /// The rectangle those crossed edges describe, with its own edges put back in order.
        ///
        /// **Not a text area.** It is here so that a failure can print where the edges went, which
        /// is what identified `pie`'s transposed `a:rect` in ECMA-376's own file — its top edge is
        /// `hc + idx`, a *horizontal* guide, which on a 160 × 120 shape lands 16.6 points below the
        /// shape's own bottom. `xtask`'s `RECT_ERRATA` corrects that one; this variant is what
        /// would catch the next.
        crossed: SceneRect,
    },
}

impl TextRectangle {
    /// The rectangle, if there is one — `None` for all three of the ways there is not.
    #[must_use]
    pub fn declared(&self) -> Option<SceneRect> {
        match *self {
            Self::Declared(rectangle) => Some(rectangle),
            Self::NotDeclared | Self::Singular { .. } | Self::Inverted { .. } => None,
        }
    }

    /// The rectangle, or the shape's own box where there is none.
    ///
    /// **The stated fallback policy, and the reason it is a method.** A text layout must lay text
    /// somewhere, and the only other box it has is the one the shape was drawn in — so the fallback
    /// is right, and burying it inside the resolver would make a shape whose rectangle silently
    /// went missing indistinguishable from one that never had a rectangle to lose. Here the caller
    /// names it, and [`declared`](Self::declared) is what a gate asks instead.
    #[must_use]
    pub fn or_bounding_box(&self, within: SceneRect) -> SceneRect {
        self.declared().unwrap_or(within)
    }
}

/// Where text goes inside a preset shape, in the device pixels of `within`.
///
/// # Errors
///
/// [`GeometryError::UnseededShape`] for a preset this build has no table for,
/// [`GeometryError::Guides`] for a guide list that will not evaluate, and
/// [`GeometryError::TextRectangle`] for an edge naming a guide the shape does not define. A guide
/// that has *no finite value* is none of those — it is [`TextRectangle::Singular`], which is an
/// answer and not a failure.
pub fn preset_text_rectangle(
    preset: PresetShapeType,
    extents: Size,
    adjustments: &[AdjustmentOverride],
    within: SceneRect,
) -> Result<TextRectangle, GeometryError> {
    let definition = definition_of(preset).ok_or(GeometryError::UnseededShape {
        shape: preset.to_wire(),
    })?;
    text_rectangle_of_definition(definition, extents, adjustments, within)
}

/// Where text goes inside a *given* definition — [`preset_text_rectangle`] without the table lookup.
///
/// Public for the reason [`outline_of_definition`] is: a suite that wants to prove the
/// [`Inverted`](TextRectangle::Inverted) arm is reachable has to resolve a deliberately transposed
/// rectangle, and a table it can only read cannot be made wrong.
///
/// # Errors
///
/// As [`preset_text_rectangle`], minus [`GeometryError::UnseededShape`] which cannot arise.
pub fn text_rectangle_of_definition(
    definition: &PresetShapeDefinition,
    extents: Size,
    adjustments: &[AdjustmentOverride],
    within: SceneRect,
) -> Result<TextRectangle, GeometryError> {
    let Some(rectangle) = definition.text_rectangle else {
        return Ok(TextRectangle::NotDeclared);
    };
    let shape = definition.preset.to_wire();
    let (environment, singular) = guide_environment(definition, extents, adjustments)?;
    let resolved = match rectangle.to_rectangle().resolve(&environment) {
        Ok(resolved) => resolved,
        Err(source) => {
            return match singular_name(&singular, &source) {
                Some(guide) => Ok(TextRectangle::Singular { guide }),
                None => Err(GeometryError::TextRectangle { shape, source }),
            }
        }
    };
    let placed = ShapeToDevice::in_shape_space(within, extents).place_rectangle(resolved);
    // Crossed edges are detected **before** the map, in the shape's own EMU, because `SceneRect`
    // normalises: a rectangle whose left is past its right arrives on the page looking like a
    // perfectly ordinary small box, and the one thing that says otherwise is the order the guides
    // resolved in.
    if resolved.left.emu() > resolved.right.emu() || resolved.top.emu() > resolved.bottom.emu() {
        return Ok(TextRectangle::Inverted { crossed: placed });
    }
    Ok(TextRectangle::Declared(placed))
}

// -------------------------------------------------------------------------------------------
// The connection sites (`a:cxnLst`)
// -------------------------------------------------------------------------------------------

/// One place a connector attaches to a shape, resolved into device pixels.
///
/// Named for what it *is* rather than after [`mjx_dml::geometry::ConnectionSite`], which is the
/// unresolved form and which this crate maps from rather than redefining.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ConnectionPoint {
    /// Where the connector attaches, in the device pixels of the box the shape was drawn in.
    pub position: ScenePoint,
    /// The direction a connector leaves in, clockwise from the positive `x` axis.
    ///
    /// **This is why a site is not merely a point.** An elbow connector leaving the top of a box
    /// must travel up before it turns and a curved one must leave along its tangent; without the
    /// angle both would leave along the straight line to the other shape and cut through the one
    /// they started in.
    pub angle: Angle,
}

/// Every place a connector can attach to a preset shape, in the device pixels of `within`.
///
/// Empty for the thirteen presets that declare no `a:cxnLst`, nine of which are themselves
/// connectors — and that emptiness is a fact about the shape rather than a failure to find
/// anything, which is why it is not an error.
///
/// # Errors
///
/// [`GeometryError::UnseededShape`], [`GeometryError::Guides`],
/// [`GeometryError::SingularGeometry`] and [`GeometryError::ConnectionSite`].
///
/// **A site whose guides have no finite value fails the whole list**, unlike a text rectangle,
/// which answers [`TextRectangle::Singular`] for itself. The asymmetry is deliberate: a text
/// rectangle is one thing that has an answer or has not, while a connector names a site *by index*
/// — `a:cxn@idx` — so dropping the fourth site silently renumbers the fifth onwards and attaches
/// every connector after it to the wrong side of the shape.
pub fn preset_connection_sites(
    preset: PresetShapeType,
    extents: Size,
    adjustments: &[AdjustmentOverride],
    within: SceneRect,
) -> Result<Vec<ConnectionPoint>, GeometryError> {
    let definition = definition_of(preset).ok_or(GeometryError::UnseededShape {
        shape: preset.to_wire(),
    })?;
    connection_sites_of_definition(definition, extents, adjustments, within)
}

/// Every connection site of a *given* definition — [`preset_connection_sites`] without the lookup.
///
/// # Errors
///
/// As [`preset_connection_sites`], minus [`GeometryError::UnseededShape`].
pub fn connection_sites_of_definition(
    definition: &PresetShapeDefinition,
    extents: Size,
    adjustments: &[AdjustmentOverride],
    within: SceneRect,
) -> Result<Vec<ConnectionPoint>, GeometryError> {
    if definition.connection_sites.is_empty() {
        return Ok(Vec::new());
    }
    let shape = definition.preset.to_wire();
    let (environment, singular) = guide_environment(definition, extents, adjustments)?;
    let map = ShapeToDevice::in_shape_space(within, extents);
    let mut sites = Vec::with_capacity(definition.connection_sites.len());
    for (index, site) in definition.connection_sites.iter().enumerate() {
        let resolved = site
            .to_connection_site()
            .resolve(&environment)
            .map_err(|source| match singular_name(&singular, &source) {
                Some(guide) => GeometryError::SingularGeometry { shape, guide },
                None => GeometryError::ConnectionSite {
                    shape,
                    index,
                    source,
                },
            })?;
        sites.push(ConnectionPoint {
            position: map.place(shape_point(resolved.position)),
            angle: resolved.angle,
        });
    }
    Ok(sites)
}

/// The affine map from one path's coordinate space onto the device-pixel box the shape is drawn in.
#[derive(Clone, Copy, Debug)]
struct ShapeToDevice {
    left: f32,
    top: f32,
    horizontal: f64,
    vertical: f64,
}

impl ShapeToDevice {
    /// The map for `path`, whose coordinates are in its own `@w`/`@h` box when it declares one and
    /// in the shape's own extents otherwise.
    fn new(within: SceneRect, path: &PresetPath, extents: Size) -> Self {
        let span = |declared: Option<i64>, shape: Emu| {
            declared
                .filter(|value| *value > 0)
                .map_or_else(|| shape.emu() as f64, |value| value as f64)
        };
        Self {
            left: within.left,
            top: within.top,
            horizontal: scale(within.width(), span(path.width, extents.width)),
            vertical: scale(within.height(), span(path.height, extents.height)),
        }
    }

    /// The map for coordinates written in the shape's **own** space, with no coordinate box.
    ///
    /// An `a:rect` and an `a:cxn` are exactly that: neither is inside an `a:path` and neither can
    /// therefore inherit a `@w`/`@h`. That is not a simplification — `CT_Path2D`'s box is an
    /// attribute of the *path*, and a text rectangle written in one path's box would be undefined
    /// for a shape with three paths and three different boxes. Thirty-one of the 186 have such a
    /// box and every one of them puts its text rectangle in the shape's space regardless.
    fn in_shape_space(within: SceneRect, extents: Size) -> Self {
        Self {
            left: within.left,
            top: within.top,
            horizontal: scale(within.width(), extents.width.emu() as f64),
            vertical: scale(within.height(), extents.height.emu() as f64),
        }
    }

    /// Where a point of the shape's space lands on the page.
    fn place(&self, point: ShapePoint) -> ScenePoint {
        ScenePoint::new(
            self.left + (point.x * self.horizontal) as f32,
            self.top + (point.y * self.vertical) as f32,
        )
    }

    /// Where a rectangle of the shape's space lands on the page.
    fn place_rectangle(&self, rectangle: ResolvedRectangle) -> SceneRect {
        let top_left = self.place(ShapePoint::new(
            rectangle.left.emu() as f64,
            rectangle.top.emu() as f64,
        ));
        let bottom_right = self.place(ShapePoint::new(
            rectangle.right.emu() as f64,
            rectangle.bottom.emu() as f64,
        ));
        SceneRect::new(top_left.x, top_left.y, bottom_right.x, bottom_right.y)
    }
}

/// Whether a shape's extents describe an area at all.
///
/// Both sides strictly positive. Zero is what a collapsed placeholder writes; a negative is out of
/// schema (`ST_PositiveCoordinate`) and arrives from files that lie, which is every file.
fn extents_have_area(extents: Size) -> bool {
    extents.width.emu() > 0 && extents.height.emu() > 0
}

/// How many device pixels one unit of a shape's space covers.
///
/// Zero for a shape with no extent on that axis, which is a real thing a document contains and
/// which would otherwise divide by zero and put an infinity into every coordinate. Zero collapses
/// the shape onto the box's corner, which is what a zero-width shape *is*. Reached by the shapes
/// whose guide lists survive a degenerate size — `rect` has no guides and `triangle`'s multiply
/// through by a zero width — while the ones whose guides do not survive it are answered above.
fn scale(pixels: f32, units: f64) -> f64 {
    if units > 0.0 {
        f64::from(pixels) / units
    } else {
        0.0
    }
}

/// A resolved point as a real-valued point in the shape's space.
fn shape_point(point: ResolvedPoint) -> ShapePoint {
    ShapePoint::new(point.x.emu() as f64, point.y.emu() as f64)
}

/// Which failure a path command's guide lookup is: a shape that has no geometry at these
/// adjustments, or a defect in the table.
///
/// The two are the same `GuideError` and are told apart by *why* the name is undefined. A name in
/// `singular` was defined by the table and left out because its own formula had no finite value
/// here — the shape genuinely has no geometry at this point of its adjustment domain, and a counted
/// stand-in is the right answer. Any other undefined name means the table's path reads a guide the
/// table's guide list does not define, which is a defect no policy may paper over.
fn path_command_error(
    shape: &'static str,
    singular: &HashSet<&'static str>,
    source: GuideError,
) -> GeometryError {
    match singular_name(singular, &source) {
        Some(guide) => GeometryError::SingularGeometry { shape, guide },
        None => GeometryError::PathCommand { shape, source },
    }
}

/// The name of the singular guide a resolution failure is *about*, if that is what it is about.
///
/// The one place the distinction between *"this shape has no geometry here"* and *"this table is
/// wrong"* is made, so that the path steps, the text rectangle and the connection sites cannot
/// drift into three different readings of the same `GuideError`. `Some(name)` means the table
/// defined that guide and [`guide_environment`] left it out because its own formula had no finite
/// value at this size and these adjustments; `None` means the name is not the table's at all, which
/// is a defect.
fn singular_name(singular: &HashSet<&'static str>, source: &GuideError) -> Option<String> {
    match source {
        GuideError::UndefinedGuide { name } if singular.contains(name.as_str()) => {
            Some(name.clone())
        }
        _ => None,
    }
}

/// Walk one path's steps, appending the device-pixel commands they draw.
fn emit_path(
    definition: &PresetShapeDefinition,
    path: &PresetPath,
    environment: &ResolvedGuides<'_>,
    singular: &HashSet<&'static str>,
    map: &ShapeToDevice,
    into: &mut Vec<PathCommand>,
) -> Result<(), GeometryError> {
    let shape = definition.preset.to_wire();
    // Where the pen is, and where the current subpath began — an `a:arcTo` derives its ellipse from
    // the first and an `a:close` returns to the second.
    let mut pen = ShapePoint::default();
    let mut subpath_start = ShapePoint::default();
    // Whether a contour is open, and whether one was ever opened. The pair is what makes a step
    // *after* an `a:close` draw from the right place; the comment inside the loop says how.
    let mut open = false;
    let mut ever_opened = false;

    for step in path.steps {
        let resolved = step
            .to_draw_command()
            .resolve(environment)
            .map_err(|source| path_command_error(shape, singular, source))?;
        // **A step after an `a:close` continues from where the closed contour began.** ECMA-376
        // says `a:close` returns the pen to the subpath's start point, and the schema permits a
        // drawing element to follow it without an intervening `a:moveTo` — a shape drawn as one
        // outline and one closed hole with a shared corner does exactly that. A
        // `mjx_scene::PathCommand` list has no way to say *"reopen the contour I
        // just closed"*, and `mjx-scene` drops a step that arrives before any `MoveTo`, so the
        // faithful translation is to state the start point again.
        //
        // `ever_opened` is what stops that becoming an invention: a path whose *first* step is not
        // a `MoveTo` is malformed, and beginning it at the shape's own origin would draw a line
        // from the corner of the box that nothing in the document asks for. Such a step is left to
        // be dropped, which is what `mjx-scene` already does with it.
        if !open && ever_opened && !matches!(resolved, ResolvedDrawCommand::MoveTo(_)) {
            if matches!(resolved, ResolvedDrawCommand::Close) {
                // A second `a:close` on an already-closed contour draws nothing. Emitting one would
                // close an empty contour, which a tessellator counts and a stroker draws a dot for.
                continue;
            }
            into.push(PathCommand::MoveTo(map.place(pen)));
            open = true;
        }
        match resolved {
            ResolvedDrawCommand::Close => {
                into.push(PathCommand::Close);
                pen = subpath_start;
                open = false;
            }
            ResolvedDrawCommand::MoveTo(point) => {
                pen = shape_point(point);
                subpath_start = pen;
                open = true;
                ever_opened = true;
                into.push(PathCommand::MoveTo(map.place(pen)));
            }
            ResolvedDrawCommand::LineTo(point) => {
                pen = shape_point(point);
                into.push(PathCommand::LineTo(map.place(pen)));
            }
            ResolvedDrawCommand::QuadBezierTo(control, end) => {
                pen = shape_point(end);
                into.push(PathCommand::QuadraticTo {
                    control: map.place(shape_point(control)),
                    end: map.place(pen),
                });
            }
            ResolvedDrawCommand::CubicBezierTo(first, second, end) => {
                pen = shape_point(end);
                into.push(PathCommand::CubicTo {
                    first_control: map.place(shape_point(first)),
                    second_control: map.place(shape_point(second)),
                    end: map.place(pen),
                });
            }
            ResolvedDrawCommand::ArcTo {
                width_radius,
                height_radius,
                start_angle,
                swing_angle,
            } => {
                for segment in arc_to_cubics(
                    pen,
                    width_radius.emu() as f64,
                    height_radius.emu() as f64,
                    start_angle.radians(),
                    swing_angle.radians(),
                ) {
                    pen = segment.end;
                    into.push(PathCommand::CubicTo {
                        first_control: map.place(segment.first_control),
                        second_control: map.place(segment.second_control),
                        end: map.place(pen),
                    });
                }
            }
        }
    }
    Ok(())
}
