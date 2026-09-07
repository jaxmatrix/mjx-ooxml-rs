//! Shape + extents + adjustments → guides → `DrawCommand`s → device pixels.
//!
//! # The environment is built the way the format builds it
//!
//! ECMA-376 Part 1 §20.1.9.11 makes declaration order evaluation order, and `mjx-dml`'s
//! [`PresetGeometry::adjustments_for_size`](mjx_dml::geometry::PresetGeometry::adjustments_for_size)
//! already builds a preset's environment in exactly two stages: every adjustment bound to its
//! current value — an `a:avLst` override where there is one and the generated default otherwise —
//! and then the shape's own `gdLst`, in order. `guide_environment` is that, with the *whole*
//! `gdLst` from [`crate::table`] instead of the subset the adjustment bounds needed. That is the
//! only difference between the two, and it is the whole of what MJXOFF-201 is about.
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

use mjx_dml::geometry::{
    Emu, GuideContext, ResolvedDrawCommand, ResolvedGuides, ResolvedPoint, Size,
};
use mjx_ooxml_types::drawingml::{
    adjustments_of, AdjustmentBound, AdjustmentSpec, PresetShapeType,
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

/// The guide environment a preset's paths resolve against: its adjustments, then its whole `gdLst`.
///
/// # Errors
///
/// [`GeometryError::Guides`] naming the first guide that would not evaluate.
pub(crate) fn guide_environment(
    definition: &PresetShapeDefinition,
    extents: Size,
    adjustments: &[AdjustmentOverride],
) -> Result<ResolvedGuides<'static>, GeometryError> {
    let mut environment = ResolvedGuides::new(GuideContext::from_size(extents));
    for spec in adjustments_of(definition.preset) {
        environment.define(spec.wire_name, value_of(spec, adjustments).0);
    }
    environment
        .extend(
            definition
                .guides
                .iter()
                .map(|guide| (guide.wire_name, guide.formula)),
        )
        .map_err(|source| GeometryError::Guides {
            shape: definition.preset.to_wire(),
            source,
        })?;
    Ok(environment)
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
    let environment = guide_environment(definition, extents, adjustments)?;
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

/// The outline a *given* definition draws — [`preset_outline`] without the table lookup.
///
/// Public for two reasons, both of them gates rather than conveniences. A suite that wants to prove
/// a bounding-box tolerance is not vacuous has to resolve a *deliberately wrong* shape and show the
/// tolerance catches it, and a table it can only read cannot be made wrong. And a path's `@w`/`@h`
/// coordinate box — which no seeded shape uses, because none of the six needs one — is a branch of
/// `ShapeToDevice::new` that would otherwise never be taken with a non-identity value, which
/// MJXOFF-155 §6's identity-value probe exists to refuse.
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
    let environment = match guide_environment(definition, extents, adjustments) {
        Ok(environment) => environment,
        // **A shape with no extent, whose own formulas divided by it.** `ss` is `min(w, h)`, so a
        // shape with a zero side makes `*/ 100000 w ss` infinite and `mjx-dml`'s evaluator refuses
        // a non-finite guide — correctly, because a non-finite coordinate in a display list is
        // worse than a wrong one.
        //
        // That is not a defect in the table and it must not fail the page. A `<a:ext cx="0"
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
        // path below, where [`scale`] collapses them onto the box's corner.
        Err(error) if !extents_have_area(extents) => {
            let _ = error;
            return Ok(ResolvedOutline {
                commands: Vec::new(),
                fill_rule: FillRule::NonZero,
                label: definition.preset.to_wire().to_owned(),
                provenance: OutlineProvenance::Document,
            });
        }
        Err(error) => return Err(error),
    };

    let mut commands = Vec::new();
    for path in definition.paths {
        let map = ShapeToDevice::new(within, path, extents);
        emit_path(definition, path, &environment, &map, &mut commands)?;
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

    /// Where a point of the shape's space lands on the page.
    fn place(&self, point: ShapePoint) -> ScenePoint {
        ScenePoint::new(
            self.left + (point.x * self.horizontal) as f32,
            self.top + (point.y * self.vertical) as f32,
        )
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

/// Walk one path's steps, appending the device-pixel commands they draw.
fn emit_path(
    definition: &PresetShapeDefinition,
    path: &PresetPath,
    environment: &ResolvedGuides<'_>,
    map: &ShapeToDevice,
    into: &mut Vec<PathCommand>,
) -> Result<(), GeometryError> {
    let shape = definition.preset.to_wire();
    // Where the pen is, and where the current subpath began — an `a:arcTo` derives its ellipse from
    // the first and an `a:close` returns to the second.
    let mut pen = ShapePoint::default();
    let mut subpath_start = ShapePoint::default();
    // Whether a contour is open, and whether one was ever opened. The pair is what makes a step
    // *after* an `a:close` draw from the right place — see [`reopen`].
    let mut open = false;
    let mut ever_opened = false;

    for step in path.steps {
        let resolved = step
            .to_draw_command()
            .resolve(environment)
            .map_err(|source| GeometryError::PathCommand { shape, source })?;
        // **A step after an `a:close` continues from where the closed contour began.** ECMA-376
        // says `a:close` returns the pen to the subpath's start point, and the schema permits a
        // drawing element to follow it without an intervening `a:moveTo` — a shape drawn as one
        // outline and one closed hole with a shared corner does exactly that. A
        // [`PathCommand`](mjx_scene::PathCommand) list has no way to say *"reopen the contour I
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
