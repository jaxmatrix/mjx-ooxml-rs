//! Fills and outlines: `mjx-dml`'s [`FillSpec`] and [`LineSpec`] as `mjx-scene`'s [`FillStyle`] and
//! [`StrokeStyle`].
//!
//! # A translation, not a resolution
//!
//! Every value arriving here has already been through `mjx-pptx`'s seven-tier ladder and
//! `mjx-dml`'s colour resolution: a `a:schemeClr` is already a hex triplet, a theme fill style has
//! already had its `phClr` substituted, and a placeholder has already inherited. So nothing in this
//! module reads a document, and nothing decides what a shape *says* — it decides only how to write
//! the same thing in the other vocabulary.
//!
//! # The opacity arrives, and it arrives as a transform
//!
//! `ColorSpec::Srgb` is a six-digit hex triplet with no alpha channel, so `mjx-dml`'s resolution
//! hands a non-opaque colour over as `Transformed { base: Srgb(hex), transforms: [Alpha(x)] }` and
//! bakes every *other* transform into the hex (MJXOFF-243). [`color_of`] therefore reads exactly
//! two shapes — a triplet, and a triplet under one `a:alpha` — and refuses anything else, because a
//! transform chain reaching here is one nothing resolved and painting its base would put a shape on
//! screen in a colour the document does not state.
//!
//! **The colour stays straight all the way down.** A `mjx_scene::Color` is the channels as the
//! document states them with the opacity beside them; `mjx_paint::Pixels::rgba` is premultiplied,
//! and that conversion belongs to the painter. Confusing the two darkens every translucent thing on
//! the page by exactly the factor nobody notices, which is why
//! `crates/mjx-reference-pack/tests/render_rc04_alpha.rs` pins both conventions in one test.

use mjx_dml::{ColorSpec, ColorTransform, FillSpec, LineDash, LineSpec};
use mjx_ooxml_types::drawingml::{
    CompoundLine, LineCap as DrawingLineCap, LineEndLength, LineEndType, LineEndWidth, PatternType,
    PenAlignment, PresetLineDash,
};
use mjx_scene::{
    Color, CompoundStroke, DashPattern, DeviceScale, FillStyle, Gradient, GradientStop, Image,
    LineCap, LineEnd, LineEndShape, LineEndSize, LineJoin, PatternPreset, Resolved, SceneLossKind,
    StrokeAlignment, StrokeStyle,
};

/// The colour a resolved [`ColorSpec`] names, or `None` when it names none this build can read.
///
/// A bare triplet is opaque; a triplet under one `a:alpha` carries that opacity as
/// `round(ratio * 255)`, which is the conversion [`mjx_scene::Color`] documents on the other side.
pub fn color_of(spec: &ColorSpec) -> Result<Color, SceneLossKind> {
    let hex = match spec {
        ColorSpec::Srgb(hex) => hex.as_str(),
        // A scheme colour that reaches here is one `mjx-dml` could not resolve — there was no
        // theme, or the slot was empty — and inventing a colour for it would paint a shape in a
        // colour no tier of the document states. Drawing nothing is the honest answer and it is
        // visible, which is what makes it reportable.
        ColorSpec::Scheme(_) => return Err(SceneLossKind::ColourNotResolved),
        ColorSpec::Other { value, .. } => {
            value.as_deref().ok_or(SceneLossKind::ColourNotResolved)?
        }
        // A resolved colour keeps exactly one transform, its `a:alpha`, and everything else is
        // already in the triplet underneath (MJXOFF-243). So this arm reads the opacity and defers
        // the colour to the base; any other transform arrived from somewhere that did not resolve
        // it, and painting the base while dropping the arithmetic would put a shape on screen in a
        // colour the document does not state — the silent-wrong-colour failure this crate exists to
        // avoid. Drawing nothing is honest, visible and reportable, so that is what it does.
        ColorSpec::Transformed { base, transforms } => {
            let [ColorTransform::Alpha(alpha)] = transforms.as_slice() else {
                return Err(SceneLossKind::ColourNotResolved);
            };
            let mut color = color_of(base)?;
            color.alpha = (alpha.ratio().clamp(0.0, 1.0) * 255.0).round() as u8;
            return Ok(color);
        }
    };
    let digits = hex.strip_prefix('#').unwrap_or(hex);
    let unreadable = SceneLossKind::ColourNotResolved;
    if digits.len() != 6 {
        return Err(unreadable);
    }
    let channel = |from: usize| u8::from_str_radix(digits.get(from..from + 2)?, 16).ok();
    Ok(Color {
        red: channel(0).ok_or(unreadable)?,
        green: channel(2).ok_or(unreadable)?,
        blue: channel(4).ok_or(unreadable)?,
        alpha: 0xff,
    })
}

/// A value with the parts of it that could not be resolved: whole when nothing was lost.
pub(crate) fn resolved_with<T>(value: T, lost: Vec<SceneLossKind>) -> Resolved<T> {
    if lost.is_empty() {
        Resolved::Answered(value)
    } else {
        Resolved::Partial(value, lost)
    }
}

/// The value of one part of a decoration, with what it lost added to `lost`, or `nothing` when the part cannot be drawn.
pub(crate) fn part<T>(answer: Resolved<T>, nothing: T, lost: &mut Vec<SceneLossKind>) -> T {
    match answer {
        Resolved::Answered(value) => value,
        Resolved::NothingToDraw => nothing,
        Resolved::Unanswerable(kind) => {
            lost.push(kind);
            nothing
        }
        Resolved::Partial(value, parts) => {
            lost.extend(parts);
            value
        }
    }
}

/// The fill a resolved [`FillSpec`] names.
///
/// `image` answers what handle a picture fill's relationship id was issued under; a caller with no
/// image table hands one that always answers `None`, and a picture fill then paints nothing rather
/// than painting a wrong colour.
pub fn fill_style(spec: &FillSpec, image: &dyn Fn(&str) -> Option<u64>) -> Resolved<FillStyle> {
    match spec {
        FillSpec::None => Resolved::Answered(FillStyle::None),
        // `a:grpFill` takes the group's fill, which nothing reads yet, so the shape paints nothing and says so.
        // Owned by MJXOFF-328 (RC35), group fill.
        FillSpec::Group => {
            Resolved::Partial(FillStyle::None, vec![SceneLossKind::PaintApproximated])
        }
        FillSpec::Solid(color) => match color_of(color) {
            Ok(color) => Resolved::Answered(FillStyle::Solid(color)),
            Err(kind) => Resolved::Unanswerable(kind),
        },
        FillSpec::Gradient { stops, angle } => {
            let stops = stops
                .iter()
                .map(|stop| {
                    Ok(GradientStop::new(
                        stop.position.ratio() as f32,
                        color_of(&stop.color)?,
                    ))
                })
                .collect::<Result<Vec<GradientStop>, SceneLossKind>>();
            match stops {
                Err(kind) => Resolved::Unanswerable(kind),
                Ok(stops) if stops.is_empty() => Resolved::Answered(FillStyle::None),
                Ok(stops) => Resolved::Answered(FillStyle::Gradient(Gradient::linear(
                    stops,
                    angle.map_or(0.0, |angle| angle.radians() as f32),
                ))),
            }
        }
        FillSpec::Pattern {
            preset,
            foreground,
            background,
        } => {
            // A colour it states and cannot be resolved loses the whole fill, and is counted.
            let foreground = match foreground.as_ref().map(color_of).transpose() {
                Ok(foreground) => foreground,
                Err(kind) => return Resolved::Unanswerable(kind),
            };
            let background = match background.as_ref().map(color_of).transpose() {
                Ok(background) => background,
                Err(kind) => return Resolved::Unanswerable(kind),
            };
            // A hatch with no preset or a missing colour is drawn as the colour it has, which is what it reduces to below one pixel, and counted.
            let approximated = vec![SceneLossKind::PaintApproximated];
            match (preset.map(pattern_preset), foreground, background) {
                (Some(preset), Some(foreground), Some(background)) => {
                    Resolved::Answered(FillStyle::Pattern {
                        preset,
                        foreground,
                        background,
                    })
                }
                (_, Some(foreground), _) => {
                    Resolved::Partial(FillStyle::Solid(foreground), approximated)
                }
                (_, None, Some(background)) => {
                    Resolved::Partial(FillStyle::Solid(background), approximated)
                }
                (_, None, None) => Resolved::Partial(FillStyle::None, approximated),
            }
        }
        FillSpec::Picture { rel_id, mode } => match image(rel_id) {
            // A picture fill the page's image table holds no entry for is counted and stood in for.
            None => Resolved::Unanswerable(SceneLossKind::FillImageNotSupplied),
            Some(handle) => {
                let mut picture = Image::stretched(handle);
                picture.fill_mode = match mode {
                    mjx_dml::PictureFillMode::Tile => mjx_scene::ImageFillMode::Tile,
                    // `a:stretch`, and the case where the fill states neither: an unadorned
                    // `a:blipFill` stretches, which is what `ImageFillMode`'s own default says.
                    mjx_dml::PictureFillMode::Stretch | mjx_dml::PictureFillMode::None => {
                        mjx_scene::ImageFillMode::Stretch
                    }
                };
                Resolved::Answered(FillStyle::Image(picture))
            }
        },
    }
}

/// The stroke a resolved [`LineSpec`] names, at `scale`, or `None` when it outlines nothing.
///
/// A width of zero is DrawingML's hairline and is drawn as the thinnest visible line rather than as
/// nothing, which is what every renderer does with it and what makes a table's default border
/// appear at all.
pub fn stroke_style(
    spec: &LineSpec,
    scale: DeviceScale,
    image: &dyn Fn(&str) -> Option<u64>,
) -> Resolved<Option<StrokeStyle>> {
    let mut lost = Vec::new();
    let fill = match spec.fill.as_ref() {
        Some(fill) => match fill_style(fill, image) {
            Resolved::Unanswerable(kind) => return Resolved::Unanswerable(kind),
            answer => part(answer, FillStyle::None, &mut lost),
        },
        None => FillStyle::None,
    };
    if fill.is_none() {
        return resolved_with(None, lost);
    }
    let width = spec.width.map_or(0.0, |width| {
        mjx_scene::pixels_from_emu(mjx_ooxml_core::measure::Emu::from_emu(width.emu()), scale)
    });
    resolved_with(
        Some(StrokeStyle {
            fill,
            width: width.max(HAIRLINE_PIXELS),
            cap: match spec.cap {
                Some(DrawingLineCap::Round) => LineCap::Round,
                Some(DrawingLineCap::Square) => LineCap::Square,
                Some(DrawingLineCap::Flat) | None => LineCap::Flat,
            },
            join: match spec.join {
                Some(mjx_dml::LineJoin::Bevel) => LineJoin::Bevel,
                // `a:miter@lim` is a fraction of the line's width; DrawingML's own default when a
                // mitre states none is eight, which is what PowerPoint writes.
                Some(mjx_dml::LineJoin::Miter { limit }) => LineJoin::Miter {
                    limit: limit.map_or(8.0, |limit| limit.ratio() as f32),
                },
                Some(mjx_dml::LineJoin::Round) | None => LineJoin::Round,
            },
            dash: dash_pattern(spec.dash.as_ref()),
            alignment: match spec.pen_alignment {
                Some(PenAlignment::Inset) => StrokeAlignment::Inset,
                Some(PenAlignment::Center) | None => StrokeAlignment::Centered,
            },
            compound: match spec.compound {
                Some(CompoundLine::Double) => CompoundStroke::Double,
                Some(CompoundLine::ThickThin) => CompoundStroke::ThickThin,
                Some(CompoundLine::ThinThick) => CompoundStroke::ThinThick,
                Some(CompoundLine::Triple) => CompoundStroke::Triple,
                Some(CompoundLine::Single) | None => CompoundStroke::Single,
            },
            head: line_end(spec.head_end.as_ref()),
            tail: line_end(spec.tail_end.as_ref()),
        }),
        lost,
    )
}

/// How wide a hairline is drawn, in device pixels.
///
/// One device pixel. DrawingML states a hairline as `@w="0"`, and a stroke of zero width covers no
/// pixels at all — so a table whose style states hairline borders would have none, which is the
/// visible half of getting this wrong.
const HAIRLINE_PIXELS: f32 = 1.0;

/// The dash a line's [`LineDash`] names.
///
/// A `a:custDash` becomes [`DashPattern::Dash`] rather than nothing: `LineSpec` does not model a
/// custom dash's stops (`mjx-dml`'s own documentation says a `LineDash::Custom` rebuilds an empty
/// `<a:custDash/>`), so the choice is between a dashed line with the wrong rhythm and a solid line
/// where the document asked for dashes. The first is closer.
fn dash_pattern(dash: Option<&LineDash>) -> DashPattern {
    match dash {
        None => DashPattern::Solid,
        Some(LineDash::Custom) => DashPattern::Dash,
        Some(LineDash::Preset(preset)) => match preset {
            PresetLineDash::Solid => DashPattern::Solid,
            PresetLineDash::Dot => DashPattern::Dot,
            PresetLineDash::Dash => DashPattern::Dash,
            PresetLineDash::LargeDash => DashPattern::LargeDash,
            PresetLineDash::DashDot => DashPattern::DashDot,
            PresetLineDash::LargeDashDot => DashPattern::LargeDashDot,
            PresetLineDash::LargeDashDotDot => DashPattern::LargeDashDotDot,
            PresetLineDash::SystemDash => DashPattern::SystemDash,
            PresetLineDash::SystemDot => DashPattern::SystemDot,
            PresetLineDash::SystemDashDot => DashPattern::SystemDashDot,
            PresetLineDash::SystemDashDotDot => DashPattern::SystemDashDotDot,
        },
    }
}

/// The decoration one end of a line carries.
fn line_end(end: Option<&mjx_dml::LineEnd>) -> LineEnd {
    let Some(end) = end else {
        return LineEnd::default();
    };
    LineEnd {
        shape: match end.kind {
            Some(LineEndType::Triangle) => LineEndShape::Triangle,
            Some(LineEndType::Stealth) => LineEndShape::Stealth,
            Some(LineEndType::Diamond) => LineEndShape::Diamond,
            Some(LineEndType::Oval) => LineEndShape::Oval,
            Some(LineEndType::Arrow) => LineEndShape::Arrow,
            Some(LineEndType::None) | None => LineEndShape::None,
        },
        width: match end.width {
            Some(LineEndWidth::Small) => LineEndSize::Small,
            Some(LineEndWidth::Large) => LineEndSize::Large,
            Some(LineEndWidth::Medium) | None => LineEndSize::Medium,
        },
        length: match end.length {
            Some(LineEndLength::Small) => LineEndSize::Small,
            Some(LineEndLength::Large) => LineEndSize::Large,
            Some(LineEndLength::Medium) | None => LineEndSize::Medium,
        },
    }
}

/// The scene's name for a DrawingML preset pattern.
///
/// # Why fifty-four arms and not an arithmetic cast
///
/// The two enumerations are generated from the same `ST_PresetPatternVal` and happen to declare the
/// same fifty-four names in the same order, so `pattern as u32` would work today. It would also
/// keep working, silently and wrongly, the day either side reorders or inserts — and a hatch drawn
/// as the wrong hatch is a defect nobody reports because the page still looks like a page.
///
/// Written out, the match is **exhaustive on both sides**: adding a preset to either enumeration
/// fails this file to compile, which is the strongest gate available and costs nothing to keep.
#[must_use]
pub fn pattern_preset(pattern: PatternType) -> PatternPreset {
    match pattern {
        PatternType::Percent5 => PatternPreset::Percent5,
        PatternType::Percent10 => PatternPreset::Percent10,
        PatternType::Percent20 => PatternPreset::Percent20,
        PatternType::Percent25 => PatternPreset::Percent25,
        PatternType::Percent30 => PatternPreset::Percent30,
        PatternType::Percent40 => PatternPreset::Percent40,
        PatternType::Percent50 => PatternPreset::Percent50,
        PatternType::Percent60 => PatternPreset::Percent60,
        PatternType::Percent70 => PatternPreset::Percent70,
        PatternType::Percent75 => PatternPreset::Percent75,
        PatternType::Percent80 => PatternPreset::Percent80,
        PatternType::Percent90 => PatternPreset::Percent90,
        PatternType::Horizontal => PatternPreset::Horizontal,
        PatternType::Vertical => PatternPreset::Vertical,
        PatternType::LightHorizontal => PatternPreset::LightHorizontal,
        PatternType::LightVertical => PatternPreset::LightVertical,
        PatternType::DarkHorizontal => PatternPreset::DarkHorizontal,
        PatternType::DarkVertical => PatternPreset::DarkVertical,
        PatternType::NarrowHorizontal => PatternPreset::NarrowHorizontal,
        PatternType::NarrowVertical => PatternPreset::NarrowVertical,
        PatternType::DashedHorizontal => PatternPreset::DashedHorizontal,
        PatternType::DashedVertical => PatternPreset::DashedVertical,
        PatternType::Cross => PatternPreset::Cross,
        PatternType::DownwardDiagonal => PatternPreset::DownwardDiagonal,
        PatternType::UpwardDiagonal => PatternPreset::UpwardDiagonal,
        PatternType::LightDownwardDiagonal => PatternPreset::LightDownwardDiagonal,
        PatternType::LightUpwardDiagonal => PatternPreset::LightUpwardDiagonal,
        PatternType::DarkDownwardDiagonal => PatternPreset::DarkDownwardDiagonal,
        PatternType::DarkUpwardDiagonal => PatternPreset::DarkUpwardDiagonal,
        PatternType::WideDownwardDiagonal => PatternPreset::WideDownwardDiagonal,
        PatternType::WideUpwardDiagonal => PatternPreset::WideUpwardDiagonal,
        PatternType::DashedDownwardDiagonal => PatternPreset::DashedDownwardDiagonal,
        PatternType::DashedUpwardDiagonal => PatternPreset::DashedUpwardDiagonal,
        PatternType::DiagonalCross => PatternPreset::DiagonalCross,
        PatternType::SmallCheckerboard => PatternPreset::SmallCheckerboard,
        PatternType::LargeCheckerboard => PatternPreset::LargeCheckerboard,
        PatternType::SmallGrid => PatternPreset::SmallGrid,
        PatternType::LargeGrid => PatternPreset::LargeGrid,
        PatternType::DottedGrid => PatternPreset::DottedGrid,
        PatternType::SmallConfetti => PatternPreset::SmallConfetti,
        PatternType::LargeConfetti => PatternPreset::LargeConfetti,
        PatternType::HorizontalBrick => PatternPreset::HorizontalBrick,
        PatternType::DiagonalBrick => PatternPreset::DiagonalBrick,
        PatternType::SolidDiamond => PatternPreset::SolidDiamond,
        PatternType::OpenDiamond => PatternPreset::OpenDiamond,
        PatternType::DottedDiamond => PatternPreset::DottedDiamond,
        PatternType::Plaid => PatternPreset::Plaid,
        PatternType::Sphere => PatternPreset::Sphere,
        PatternType::Weave => PatternPreset::Weave,
        PatternType::Divot => PatternPreset::Divot,
        PatternType::Shingle => PatternPreset::Shingle,
        PatternType::Wave => PatternPreset::Wave,
        PatternType::Trellis => PatternPreset::Trellis,
        PatternType::ZigZag => PatternPreset::ZigZag,
    }
}
