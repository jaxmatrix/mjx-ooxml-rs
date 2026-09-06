//! The nine commands a display list is made of.
//!
//! # Why nine, and why these nine
//!
//! Four *state* commands and their `Pop`, and four *draw* commands. Everything a document can put on
//! a page reduces to them: a background is a `FillPath` over a rectangle, a shape is a `FillPath`
//! and a `StrokePath` over its outline, a paragraph is a run of `DrawGlyphs`, a picture is a
//! `DrawImage`, and a rotated group is all of that inside a `PushTransform`.
//!
//! The list is closed on purpose, for the same reason [`mjx_layout::Fragment`]'s six kinds are: it
//! is the vocabulary four painters and two exporters are written against, so a tenth command is a
//! change to six implementations. Anything a document wants that is not one of these is said with a
//! paint, a stroke, a geometry or an effect — all of which are *data* the painter interprets, not
//! control flow it has to grow a case for.
//!
//! # The stack
//!
//! `Push…` and `Pop` nest. A painter keeps a stack of state, and a display list that popped more
//! than it pushed, or that ended with something still pushed, does not decode: an unbalanced stream
//! is [`crate::SceneError::UnbalancedStack`], because a painter that met one would either draw the
//! rest of the page under a clip that should have ended, or crash.

use crate::encoding::ResourceIndex;
use crate::geometry::SceneRect;

/// A clip region: a rectangle, or a path with the rectangle that bounds it.
///
/// The bounding rectangle is stored even for a path clip, and it is not redundant. A viewport cull
/// asks *can anything under this clip be visible* thousands of times a frame, and answering it from
/// a rectangle costs four comparisons where answering it from the path costs a walk.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Clip {
    /// The geometry the clip follows, or `None` for a plain rectangle.
    pub geometry: Option<ResourceIndex>,
    /// The rectangle that bounds it, in the coordinate space of the enclosing transform.
    pub bounds: SceneRect,
}

impl Clip {
    /// A rectangular clip.
    #[must_use]
    pub fn rectangle(bounds: SceneRect) -> Self {
        Self {
            geometry: None,
            bounds,
        }
    }

    /// A clip that follows a path, with the box that bounds it.
    #[must_use]
    pub fn path(geometry: ResourceIndex, bounds: SceneRect) -> Self {
        Self {
            geometry: Some(geometry),
            bounds,
        }
    }
}

/// One command of a display list.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Command {
    /// Compose a transform onto the current one, until the matching [`Command::Pop`].
    ///
    /// **Relative, not absolute.** A painter multiplies rather than replaces, which is what makes a
    /// group inside a group work and is why the scene builder converts a fragment's absolute
    /// transform into one relative to whatever is already installed.
    PushTransform(ResourceIndex),
    /// Intersect a clip with the current one, until the matching [`Command::Pop`].
    ///
    /// Intersect, never replace: a clip that could widen the one around it would let a subtree draw
    /// outside the shape that contains it.
    PushClip(ResourceIndex),
    /// Multiply the current opacity, until the matching [`Command::Pop`].
    PushOpacity(f32),
    /// Apply the effect DAG rooted at this index to everything drawn until the matching
    /// [`Command::Pop`].
    PushEffect(ResourceIndex),
    /// Undo the most recent `Push`.
    Pop,
    /// Fill a geometry with a paint.
    FillPath {
        /// What shape.
        geometry: ResourceIndex,
        /// What colour, gradient, pattern or picture.
        paint: ResourceIndex,
    },
    /// Draw a geometry's outline.
    StrokePath {
        /// What shape.
        geometry: ResourceIndex,
        /// How the outline is drawn.
        stroke: ResourceIndex,
    },
    /// Draw a run of placed glyphs in a paint.
    DrawGlyphs {
        /// Which run.
        run: ResourceIndex,
        /// What colour the glyphs are.
        paint: ResourceIndex,
    },
    /// Draw a picture into a rectangle.
    DrawImage {
        /// Which picture, and how it fills the rectangle.
        image: ResourceIndex,
        /// Where it goes, in the coordinate space of the enclosing transform.
        destination: SceneRect,
    },
}

impl Command {
    /// The opcode this command is written as.
    #[must_use]
    pub const fn opcode(&self) -> u8 {
        use crate::encoding::opcode;
        match self {
            Self::PushTransform(_) => opcode::PUSH_TRANSFORM,
            Self::PushClip(_) => opcode::PUSH_CLIP,
            Self::PushOpacity(_) => opcode::PUSH_OPACITY,
            Self::PushEffect(_) => opcode::PUSH_EFFECT,
            Self::Pop => opcode::POP,
            Self::FillPath { .. } => opcode::FILL_PATH,
            Self::StrokePath { .. } => opcode::STROKE_PATH,
            Self::DrawGlyphs { .. } => opcode::DRAW_GLYPHS,
            Self::DrawImage { .. } => opcode::DRAW_IMAGE,
        }
    }

    /// How many bytes the command's record takes, its four-byte header included.
    #[must_use]
    pub const fn record_bytes(&self) -> usize {
        match self {
            Self::Pop => 4,
            Self::PushTransform(_)
            | Self::PushClip(_)
            | Self::PushOpacity(_)
            | Self::PushEffect(_) => 8,
            Self::FillPath { .. } | Self::StrokePath { .. } | Self::DrawGlyphs { .. } => 12,
            Self::DrawImage { .. } => 24,
        }
    }

    /// Whether this command pushes state that a later [`Command::Pop`] undoes.
    #[must_use]
    pub const fn pushes_state(&self) -> bool {
        matches!(
            self,
            Self::PushTransform(_) | Self::PushClip(_) | Self::PushOpacity(_) | Self::PushEffect(_)
        )
    }
}
