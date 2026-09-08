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
//! # ⚠ Opacity is already gone by the time a value gets here
//!
//! `ColorSpec::Srgb` is a six-digit hex triplet and has no alpha channel, and `resolve_fill` /
//! `resolve_line` / `resolve_effects` are documented as dropping the resolved alpha for exactly that
//! reason. So [`color_of`] returns an **opaque** colour for every colour a document can state, and
//! the `<a:alpha val="63000"/>` the standard Office theme puts on every shadow is not here to be
//! read. That is a loss at the seam below this crate; it is asserted rather than described in
//! `tests/the_opacity_is_lost_at_the_spec_boundary.rs`.

use mjx_dml::{ColorSpec, FillSpec, LineDash, LineSpec};
use mjx_ooxml_types::drawingml::{
    CompoundLine, LineCap as DrawingLineCap, LineEndLength, LineEndType, LineEndWidth, PatternType,
    PenAlignment, PresetLineDash,
};
use mjx_scene::{
    Color, CompoundStroke, DashPattern, DeviceScale, FillStyle, Gradient, GradientStop, Image,
    LineCap, LineEnd, LineEndShape, LineEndSize, LineJoin, PatternPreset, StrokeAlignment,
    StrokeStyle,
};

/// The colour a resolved [`ColorSpec`] names, or `None` when it names none this build can read.
///
/// **Always opaque.** See the module's own warning: the alpha was dropped one crate below.
#[must_use]
pub fn color_of(spec: &ColorSpec) -> Option<Color> {
    let hex = match spec {
        ColorSpec::Srgb(hex) => hex.as_str(),
        // A scheme colour that reaches here is one `mjx-dml` could not resolve — there was no
        // theme, or the slot was empty — and inventing a colour for it would paint a shape in a
        // colour no tier of the document states. Drawing nothing is the honest answer and it is
        // visible, which is what makes it reportable.
        ColorSpec::Scheme(_) => return None,
        ColorSpec::Other { value, .. } => value.as_deref()?,
    };
    let digits = hex.strip_prefix('#').unwrap_or(hex);
    if digits.len() != 6 {
        return None;
    }
    let channel = |from: usize| u8::from_str_radix(digits.get(from..from + 2)?, 16).ok();
    Some(Color {
        red: channel(0)?,
        green: channel(2)?,
        blue: channel(4)?,
        alpha: 0xff,
    })
}

/// The fill a resolved [`FillSpec`] names.
///
/// `image` answers what handle a picture fill's relationship id was issued under; a caller with no
/// image table hands one that always answers `None`, and a picture fill then paints nothing rather
/// than painting a wrong colour.
#[must_use]
pub fn fill_style(spec: &FillSpec, image: &dyn Fn(&str) -> Option<u64>) -> FillStyle {
    match spec {
        FillSpec::None => FillStyle::None,
        // `a:grpFill` says *take the group's fill*, and the group's fill is not in the catalogue:
        // `mjx-pptx` answers `effective_shape_fill` for a `p:sp` and a `p:cxnSp`, and a group's own
        // `a:grpSpPr` is a question it does not take. So this paints nothing, which is what a group
        // with no fill of its own means and is the common case. A group that *does* state a fill
        // needs a reader in `mjx-pptx` before it can be consumed here.
        FillSpec::Group => FillStyle::None,
        FillSpec::Solid(color) => color_of(color).map_or(FillStyle::None, FillStyle::Solid),
        FillSpec::Gradient { stops, angle } => {
            let stops: Vec<GradientStop> = stops
                .iter()
                .filter_map(|stop| {
                    Some(GradientStop::new(
                        stop.position.ratio() as f32,
                        color_of(&stop.color)?,
                    ))
                })
                .collect();
            if stops.is_empty() {
                return FillStyle::None;
            }
            FillStyle::Gradient(Gradient::linear(
                stops,
                angle.map_or(0.0, |angle| angle.radians() as f32),
            ))
        }
        FillSpec::Pattern {
            preset,
            foreground,
            background,
        } => {
            // A pattern with no preset, or with a colour this build cannot read, is a pattern that
            // cannot be drawn as one. Its foreground is used as a solid fill instead, which is what
            // a hatch reduces to as its scale falls below one pixel and is the closest thing to it
            // that is not nothing.
            let foreground = foreground.as_ref().and_then(color_of);
            let background = background.as_ref().and_then(color_of);
            match (preset.map(pattern_preset), foreground, background) {
                (Some(preset), Some(foreground), Some(background)) => FillStyle::Pattern {
                    preset,
                    foreground,
                    background,
                },
                (_, Some(foreground), _) => FillStyle::Solid(foreground),
                (_, None, Some(background)) => FillStyle::Solid(background),
                (_, None, None) => FillStyle::None,
            }
        }
        FillSpec::Picture { rel_id, mode } => match image(rel_id) {
            None => FillStyle::None,
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
                FillStyle::Image(picture)
            }
        },
    }
}

/// The stroke a resolved [`LineSpec`] names, at `scale`, or `None` when it outlines nothing.
///
/// A width of zero is DrawingML's hairline and is drawn as the thinnest visible line rather than as
/// nothing, which is what every renderer does with it and what makes a table's default border
/// appear at all.
#[must_use]
pub fn stroke_style(
    spec: &LineSpec,
    scale: DeviceScale,
    image: &dyn Fn(&str) -> Option<u64>,
) -> Option<StrokeStyle> {
    let fill = spec
        .fill
        .as_ref()
        .map_or(FillStyle::None, |fill| fill_style(fill, image));
    if fill.is_none() {
        return None;
    }
    let width = spec.width.map_or(0.0, |width| {
        mjx_scene::pixels_from_emu(mjx_ooxml_core::measure::Emu::from_emu(width.emu()), scale)
    });
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
    })
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
