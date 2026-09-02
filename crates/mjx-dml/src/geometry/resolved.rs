//! Custom geometry with every guide reference turned into a number.
//!
//! [`CustomGeometry`] and [`CustomGeometrySpec`] keep a coordinate exactly as the file wrote it — a
//! literal, or the name of a guide ([`AdjustCoordinate`] / [`AdjustAngle`]). This module evaluates the
//! shape's `a:avLst` and `a:gdLst` against a shape size (see [`formula`](super::formula)) and hands
//! back the same geometry with each coordinate an [`Emu`] and each angle an [`Angle`].
//!
//! Resolution is a **read-side** capability. Nothing here mutates a geometry, and nothing here is
//! written back: the stored `fmla` text is the file's, and it stays the file's, so resolving a shape
//! cannot change a single byte of what is serialized.
//!
//! Coordinates come back in the space the file wrote them in. A path's own `@w`/`@h` box
//! ([`Path2DSpec::width`](super::Path2DSpec)) is a rendering transform onto the shape's extents, and
//! this module does not apply it.

use super::custom::{
    AdjustAngle, AdjustCoordinate, AdjustHandle, ConnectionSite, CustomGeometry,
    CustomGeometrySpec, DrawCommand, GuideSpec, Path2DSpec, PathFillMode, Point, Rectangle,
};
use super::formula::{value_as_angle, value_as_emu, GuideContext, GuideError, ResolvedGuides};
use super::measures::{Angle, Emu};
use mjx_ooxml_core::Interner;

/// The `(name, formula)` pairs of a guide list, for [`ResolvedGuides::extend`].
fn pairs(guides: &[GuideSpec]) -> impl Iterator<Item = (&str, &str)> {
    guides
        .iter()
        .map(|guide| (guide.name.as_str(), guide.formula.as_str()))
}

impl AdjustCoordinate {
    /// This coordinate as a length: the literal, or the value of the guide it names.
    ///
    /// # Errors
    ///
    /// [`GuideError::UndefinedGuide`] if it names a guide the shape does not define and that is not a
    /// built-in variable.
    pub fn resolve(&self, guides: &ResolvedGuides<'_>) -> Result<Emu, GuideError> {
        match self {
            Self::Emu(emu) => Ok(*emu),
            Self::Guide(name) => guides.resolve(name).map(value_as_emu),
        }
    }
}

impl AdjustAngle {
    /// This angle as an [`Angle`]: the literal, or the value of the guide it names (read in the wire
    /// scale of 60000ths of a degree).
    ///
    /// # Errors
    ///
    /// [`GuideError::UndefinedGuide`] if it names a guide the shape does not define and that is not a
    /// built-in variable.
    pub fn resolve(&self, guides: &ResolvedGuides<'_>) -> Result<Angle, GuideError> {
        match self {
            Self::Angle(angle) => Ok(*angle),
            Self::Guide(name) => guides.resolve(name).map(value_as_angle),
        }
    }
}

/// A [`Point`] with both coordinates resolved to lengths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedPoint {
    /// The horizontal coordinate (`a:pt@x`).
    pub x: Emu,
    /// The vertical coordinate (`a:pt@y`).
    pub y: Emu,
}

impl Point {
    /// This point with both coordinates resolved.
    ///
    /// # Errors
    ///
    /// [`GuideError::UndefinedGuide`] if either coordinate names an unknown guide.
    pub fn resolve(&self, guides: &ResolvedGuides<'_>) -> Result<ResolvedPoint, GuideError> {
        Ok(ResolvedPoint {
            x: self.x.resolve(guides)?,
            y: self.y.resolve(guides)?,
        })
    }
}

/// A [`Rectangle`] (`a:rect`) with all four edges resolved to lengths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedRectangle {
    /// The left edge (`@l`).
    pub left: Emu,
    /// The top edge (`@t`).
    pub top: Emu,
    /// The right edge (`@r`).
    pub right: Emu,
    /// The bottom edge (`@b`).
    pub bottom: Emu,
}

impl Rectangle {
    /// This rectangle with all four edges resolved.
    ///
    /// # Errors
    ///
    /// [`GuideError::UndefinedGuide`] if any edge names an unknown guide.
    pub fn resolve(&self, guides: &ResolvedGuides<'_>) -> Result<ResolvedRectangle, GuideError> {
        Ok(ResolvedRectangle {
            left: self.left.resolve(guides)?,
            top: self.top.resolve(guides)?,
            right: self.right.resolve(guides)?,
            bottom: self.bottom.resolve(guides)?,
        })
    }
}

/// A [`DrawCommand`] with every coordinate, radius and angle resolved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResolvedDrawCommand {
    /// `a:close` — close the current subpath back to its start.
    Close,
    /// `a:moveTo` — start a new subpath at a point, drawing nothing.
    MoveTo(ResolvedPoint),
    /// `a:lnTo` — draw a straight line to a point.
    LineTo(ResolvedPoint),
    /// `a:arcTo` — draw an elliptical arc, given the ellipse radii and the start / swing angles.
    ArcTo {
        /// The ellipse's horizontal radius (`@wR`).
        width_radius: Emu,
        /// The ellipse's vertical radius (`@hR`).
        height_radius: Emu,
        /// The angle the arc starts at (`@stAng`).
        start_angle: Angle,
        /// The angle the arc sweeps through (`@swAng`).
        swing_angle: Angle,
    },
    /// `a:quadBezTo` — a quadratic Bézier curve: one control point, then the end point.
    QuadBezierTo(ResolvedPoint, ResolvedPoint),
    /// `a:cubicBezTo` — a cubic Bézier curve: two control points, then the end point.
    CubicBezierTo(ResolvedPoint, ResolvedPoint, ResolvedPoint),
}

impl DrawCommand {
    /// This command with every coordinate, radius and angle resolved.
    ///
    /// # Errors
    ///
    /// [`GuideError::UndefinedGuide`] if any of them names an unknown guide.
    pub fn resolve(&self, guides: &ResolvedGuides<'_>) -> Result<ResolvedDrawCommand, GuideError> {
        Ok(match self {
            Self::Close => ResolvedDrawCommand::Close,
            Self::MoveTo(point) => ResolvedDrawCommand::MoveTo(point.resolve(guides)?),
            Self::LineTo(point) => ResolvedDrawCommand::LineTo(point.resolve(guides)?),
            Self::ArcTo {
                width_radius,
                height_radius,
                start_angle,
                swing_angle,
            } => ResolvedDrawCommand::ArcTo {
                width_radius: width_radius.resolve(guides)?,
                height_radius: height_radius.resolve(guides)?,
                start_angle: start_angle.resolve(guides)?,
                swing_angle: swing_angle.resolve(guides)?,
            },
            Self::QuadBezierTo(control, end) => {
                ResolvedDrawCommand::QuadBezierTo(control.resolve(guides)?, end.resolve(guides)?)
            }
            Self::CubicBezierTo(first, second, end) => ResolvedDrawCommand::CubicBezierTo(
                first.resolve(guides)?,
                second.resolve(guides)?,
                end.resolve(guides)?,
            ),
        })
    }
}

/// A [`Path2DSpec`] with every command resolved. The flags are carried through unchanged.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResolvedPath {
    /// The width of the path's coordinate box (`@w`).
    pub width: Option<Emu>,
    /// The height of the path's coordinate box (`@h`).
    pub height: Option<Emu>,
    /// How the path is filled (`@fill`).
    pub fill: Option<PathFillMode>,
    /// Whether the path is stroked (`@stroke`).
    pub stroke: Option<bool>,
    /// Whether the path may be extruded (`@extrusionOk`).
    pub extrusion_ok: Option<bool>,
    /// The path's drawing commands, in order.
    pub commands: Vec<ResolvedDrawCommand>,
}

impl Path2DSpec {
    /// This path with every command resolved.
    ///
    /// # Errors
    ///
    /// [`GuideError::UndefinedGuide`] if a command names an unknown guide.
    pub fn resolve(&self, guides: &ResolvedGuides<'_>) -> Result<ResolvedPath, GuideError> {
        let mut commands = Vec::with_capacity(self.commands.len());
        for command in &self.commands {
            commands.push(command.resolve(guides)?);
        }
        Ok(ResolvedPath {
            width: self.width,
            height: self.height,
            fill: self.fill,
            stroke: self.stroke,
            extrusion_ok: self.extrusion_ok,
            commands,
        })
    }
}

/// A [`ConnectionSite`] with its position and outgoing angle resolved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedConnectionSite {
    /// The angle a connector leaves the site at (`@ang`).
    pub angle: Angle,
    /// Where the site sits (`a:pos`).
    pub position: ResolvedPoint,
}

impl ConnectionSite {
    /// This connection site with its position and angle resolved.
    ///
    /// # Errors
    ///
    /// [`GuideError::UndefinedGuide`] if either names an unknown guide.
    pub fn resolve(
        &self,
        guides: &ResolvedGuides<'_>,
    ) -> Result<ResolvedConnectionSite, GuideError> {
        Ok(ResolvedConnectionSite {
            angle: self.angle.resolve(guides)?,
            position: self.position.resolve(guides)?,
        })
    }
}

/// An [`AdjustHandle`] with its position and clamp range resolved. The `gdRef*` names stay names —
/// they say which guide the drag *writes to*, not a value to read.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedAdjustHandle {
    /// A Cartesian (`x`/`y`) adjust handle (`a:ahXY`).
    Xy {
        /// The handle's position (`a:pos`).
        position: ResolvedPoint,
        /// The guide the horizontal drag drives (`@gdRefX`).
        guide_ref_x: Option<String>,
        /// The minimum horizontal value (`@minX`).
        min_x: Option<Emu>,
        /// The maximum horizontal value (`@maxX`).
        max_x: Option<Emu>,
        /// The guide the vertical drag drives (`@gdRefY`).
        guide_ref_y: Option<String>,
        /// The minimum vertical value (`@minY`).
        min_y: Option<Emu>,
        /// The maximum vertical value (`@maxY`).
        max_y: Option<Emu>,
    },
    /// A polar (radius/angle) adjust handle (`a:ahPolar`).
    Polar {
        /// The handle's position (`a:pos`).
        position: ResolvedPoint,
        /// The guide the radial drag drives (`@gdRefR`).
        guide_ref_radius: Option<String>,
        /// The minimum radius (`@minR`).
        min_radius: Option<Emu>,
        /// The maximum radius (`@maxR`).
        max_radius: Option<Emu>,
        /// The guide the angular drag drives (`@gdRefAng`).
        guide_ref_angle: Option<String>,
        /// The minimum angle (`@minAng`).
        min_angle: Option<Angle>,
        /// The maximum angle (`@maxAng`).
        max_angle: Option<Angle>,
    },
}

impl AdjustHandle {
    /// This handle with its position and clamp range resolved.
    ///
    /// # Errors
    ///
    /// [`GuideError::UndefinedGuide`] if any of them names an unknown guide.
    pub fn resolve(&self, guides: &ResolvedGuides<'_>) -> Result<ResolvedAdjustHandle, GuideError> {
        Ok(match self {
            Self::Xy {
                position,
                guide_ref_x,
                min_x,
                max_x,
                guide_ref_y,
                min_y,
                max_y,
            } => ResolvedAdjustHandle::Xy {
                position: position.resolve(guides)?,
                guide_ref_x: guide_ref_x.clone(),
                min_x: resolve_optional_coordinate(min_x, guides)?,
                max_x: resolve_optional_coordinate(max_x, guides)?,
                guide_ref_y: guide_ref_y.clone(),
                min_y: resolve_optional_coordinate(min_y, guides)?,
                max_y: resolve_optional_coordinate(max_y, guides)?,
            },
            Self::Polar {
                position,
                guide_ref_radius,
                min_radius,
                max_radius,
                guide_ref_angle,
                min_angle,
                max_angle,
            } => ResolvedAdjustHandle::Polar {
                position: position.resolve(guides)?,
                guide_ref_radius: guide_ref_radius.clone(),
                min_radius: resolve_optional_coordinate(min_radius, guides)?,
                max_radius: resolve_optional_coordinate(max_radius, guides)?,
                guide_ref_angle: guide_ref_angle.clone(),
                min_angle: resolve_optional_angle(min_angle, guides)?,
                max_angle: resolve_optional_angle(max_angle, guides)?,
            },
        })
    }
}

/// Resolves an optional coordinate, keeping "absent" absent.
fn resolve_optional_coordinate(
    coordinate: &Option<AdjustCoordinate>,
    guides: &ResolvedGuides<'_>,
) -> Result<Option<Emu>, GuideError> {
    coordinate
        .as_ref()
        .map(|value| value.resolve(guides))
        .transpose()
}

/// Resolves an optional angle, keeping "absent" absent.
fn resolve_optional_angle(
    angle: &Option<AdjustAngle>,
    guides: &ResolvedGuides<'_>,
) -> Result<Option<Angle>, GuideError> {
    angle
        .as_ref()
        .map(|value| value.resolve(guides))
        .transpose()
}

/// A whole custom geometry with every guide reference resolved to a number — what a renderer, a
/// hit-test or a bounding-box calculation reads.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResolvedCustomGeometry {
    /// The drawing paths (`a:pathLst`), in order.
    pub paths: Vec<ResolvedPath>,
    /// The rectangle text is laid out in (`a:rect`), if the geometry states one.
    pub text_rectangle: Option<ResolvedRectangle>,
    /// The connection sites (`a:cxnLst`), in order.
    pub connection_sites: Vec<ResolvedConnectionSite>,
    /// The adjust handles (`a:ahLst`), in order.
    pub adjust_handles: Vec<ResolvedAdjustHandle>,
}

impl CustomGeometrySpec {
    /// Evaluates the geometry's guides against a shape size: the `a:avLst` seeds first, then the
    /// `a:gdLst` formulas, each in declaration order.
    ///
    /// # Errors
    ///
    /// [`GuideError::Guide`] naming the first guide that could not be evaluated.
    pub fn guide_values(&self, context: GuideContext) -> Result<ResolvedGuides<'_>, GuideError> {
        let mut resolved = ResolvedGuides::new(context);
        resolved.extend(pairs(&self.adjust_values))?;
        resolved.extend(pairs(&self.guides))?;
        Ok(resolved)
    }

    /// This geometry with every coordinate and angle resolved against a shape size.
    ///
    /// # Errors
    ///
    /// [`GuideError`] if a guide cannot be evaluated, or if a coordinate names one the shape does not
    /// define.
    pub fn resolve(&self, context: GuideContext) -> Result<ResolvedCustomGeometry, GuideError> {
        let guides = self.guide_values(context)?;
        let mut paths = Vec::with_capacity(self.paths.len());
        for path in &self.paths {
            paths.push(path.resolve(&guides)?);
        }
        let mut connection_sites = Vec::with_capacity(self.connection_sites.len());
        for site in &self.connection_sites {
            connection_sites.push(site.resolve(&guides)?);
        }
        let mut adjust_handles = Vec::with_capacity(self.adjust_handles.len());
        for handle in &self.adjust_handles {
            adjust_handles.push(handle.resolve(&guides)?);
        }
        let text_rectangle = self
            .text_rectangle
            .as_ref()
            .map(|rect| rect.resolve(&guides))
            .transpose()?;
        Ok(ResolvedCustomGeometry {
            paths,
            text_rectangle,
            connection_sites,
            adjust_handles,
        })
    }
}

impl CustomGeometry {
    /// This geometry with every coordinate and angle resolved against a shape size — the whole of
    /// [`CustomGeometrySpec::resolve`] straight off the wire wrapper.
    ///
    /// # Errors
    ///
    /// [`GuideError`] if a guide cannot be evaluated, or if a coordinate names one the shape does not
    /// define.
    pub fn resolve(
        &self,
        interner: &Interner,
        context: GuideContext,
    ) -> Result<ResolvedCustomGeometry, GuideError> {
        self.spec(interner).resolve(context)
    }
}
