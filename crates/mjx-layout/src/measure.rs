//! Points, sizes, rectangles and affine transforms, all in [`Emu`].
//!
//! # Why EMU and not floating-point points
//!
//! Layout accumulates. A page of prose is hundreds of additions of a line height, a table is
//! hundreds of additions of a cell width, and a `f64` page that starts at the top margin and ends at
//! the bottom one does not land on the number the document says it should. EMU is an integer unit
//! into which every unit Office actually writes divides exactly — a point is `12700`, a twip is
//! `635`, a centimetre is `360000`, an inch is `914400` — so a fragment's edge is a *value* rather
//! than a value plus drift, and two boxes that should share an edge share it.
//!
//! The conversion from the text engine's `f64` points happens once, at the boundary
//! ([`Emu::from_points`]), where the rounding is visible and one EMU is 1/12700 of a point.
//!
//! # Which way is down
//!
//! **`y` increases downward**, and the origin of a page is its top-left corner. That is what OOXML
//! writes (`a:off@y` grows downward), what every surface this eventually reaches uses, and what
//! `mjx-text`'s [`place_run`](mjx_text::place_run) already assumes. A rectangle's `top` is therefore
//! numerically **less** than its `bottom`.
//!
//! # Why the geometry types are defined here and `Emu` is not
//!
//! `mjx-dml` has a `Point` and a `Rectangle`, and they are **not** these: both are built out of
//! `AdjustCoordinate`, which is "a literal EMU length *or* the name of a guide formula", because
//! they model `a:pt` and `a:rect` inside a custom geometry. They cannot be lifted into a
//! geometry-neutral tier, because a guide reference is DrawingML. `Emu` and `Angle` could be and
//! were: they now live in [`mjx_ooxml_core::measure`] and this crate uses that one definition rather
//! than writing a second.

use mjx_ooxml_core::measure::{Angle, Emu};

/// A position on a page, in EMU, with `y` increasing downward.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct LayoutPoint {
    /// How far right of the origin.
    pub x: Emu,
    /// How far **below** the origin.
    pub y: Emu,
}

impl LayoutPoint {
    /// The origin.
    pub const ORIGIN: Self = Self {
        x: Emu::ZERO,
        y: Emu::ZERO,
    };

    /// A point at `(x, y)`.
    #[must_use]
    pub const fn new(x: Emu, y: Emu) -> Self {
        Self { x, y }
    }

    /// The same point moved right by `dx` and down by `dy`.
    #[must_use]
    pub fn translated(self, dx: Emu, dy: Emu) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}

/// A width and a height, in EMU. Either may be negative, because a document may say so; a
/// [`LayoutRect`] built from one normalises it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct LayoutSize {
    /// How wide.
    pub width: Emu,
    /// How tall.
    pub height: Emu,
}

impl LayoutSize {
    /// Nothing at all.
    pub const ZERO: Self = Self {
        width: Emu::ZERO,
        height: Emu::ZERO,
    };

    /// A size of `width` by `height`.
    #[must_use]
    pub const fn new(width: Emu, height: Emu) -> Self {
        Self { width, height }
    }

    /// Whether either dimension is zero or negative, so that nothing can be inside it.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.width <= Emu::ZERO || self.height <= Emu::ZERO
    }
}

/// An axis-aligned rectangle, in EMU, with `y` increasing downward.
///
/// Stored as its four edges rather than an origin and a size, because every question asked of it —
/// does it contain this point, does it meet that one, what is the union — is an edge comparison, and
/// an origin-plus-size form has to recompute `right` and `bottom` to answer any of them.
///
/// **Containment is half-open**: `left <= x < right` and `top <= y < bottom`. That is what makes two
/// rectangles that share an edge unambiguous — a point on the shared edge belongs to exactly one of
/// them — and it is the rule the spatial index and every brute-force check of it use alike.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct LayoutRect {
    /// The left edge.
    pub left: Emu,
    /// The top edge, which is the **smaller** `y`.
    pub top: Emu,
    /// The right edge.
    pub right: Emu,
    /// The bottom edge, which is the **larger** `y`.
    pub bottom: Emu,
}

impl LayoutRect {
    /// The empty rectangle at the origin.
    pub const ZERO: Self = Self {
        left: Emu::ZERO,
        top: Emu::ZERO,
        right: Emu::ZERO,
        bottom: Emu::ZERO,
    };

    /// A rectangle from its four edges, normalised so that `left <= right` and `top <= bottom`.
    ///
    /// Normalising rather than refusing is deliberate: a document may write a negative extent, and
    /// the rectangle it means is the one with its corners swapped.
    #[must_use]
    pub fn from_edges(left: Emu, top: Emu, right: Emu, bottom: Emu) -> Self {
        Self {
            left: left.minimum(right),
            top: top.minimum(bottom),
            right: left.maximum(right),
            bottom: top.maximum(bottom),
        }
    }

    /// A rectangle at `origin`, `size` wide and tall, normalised.
    #[must_use]
    pub fn from_origin_and_size(origin: LayoutPoint, size: LayoutSize) -> Self {
        Self::from_edges(
            origin.x,
            origin.y,
            origin.x + size.width,
            origin.y + size.height,
        )
    }

    /// The top-left corner.
    #[must_use]
    pub const fn origin(self) -> LayoutPoint {
        LayoutPoint {
            x: self.left,
            y: self.top,
        }
    }

    /// How wide and how tall.
    #[must_use]
    pub fn size(self) -> LayoutSize {
        LayoutSize {
            width: self.right - self.left,
            height: self.bottom - self.top,
        }
    }

    /// How wide.
    #[must_use]
    pub fn width(self) -> Emu {
        self.right - self.left
    }

    /// How tall.
    #[must_use]
    pub fn height(self) -> Emu {
        self.bottom - self.top
    }

    /// Whether the rectangle encloses no area at all, so that nothing is inside it and it meets
    /// nothing.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    /// Whether `point` is inside, half-open: `left <= x < right` and `top <= y < bottom`.
    #[must_use]
    pub fn contains(self, point: LayoutPoint) -> bool {
        point.x >= self.left && point.x < self.right && point.y >= self.top && point.y < self.bottom
    }

    /// Whether the two rectangles share any area, half-open on every edge. An empty rectangle meets
    /// nothing, including itself.
    #[must_use]
    pub fn intersects(self, other: Self) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.left < other.right
            && other.left < self.right
            && self.top < other.bottom
            && other.top < self.bottom
    }

    /// The smallest rectangle containing both. An empty rectangle contributes nothing.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        Self {
            left: self.left.minimum(other.left),
            top: self.top.minimum(other.top),
            right: self.right.maximum(other.right),
            bottom: self.bottom.maximum(other.bottom),
        }
    }

    /// The area the two share, or `None` when they share none.
    #[must_use]
    pub fn intersection(self, other: Self) -> Option<Self> {
        if !self.intersects(other) {
            return None;
        }
        Some(Self {
            left: self.left.maximum(other.left),
            top: self.top.maximum(other.top),
            right: self.right.minimum(other.right),
            bottom: self.bottom.minimum(other.bottom),
        })
    }

    /// The same rectangle moved right by `dx` and down by `dy`.
    #[must_use]
    pub fn translated(self, dx: Emu, dy: Emu) -> Self {
        Self {
            left: self.left + dx,
            top: self.top + dy,
            right: self.right + dx,
            bottom: self.bottom + dy,
        }
    }

    /// The four corners, clockwise from the top-left.
    #[must_use]
    pub fn corners(self) -> [LayoutPoint; 4] {
        [
            LayoutPoint::new(self.left, self.top),
            LayoutPoint::new(self.right, self.top),
            LayoutPoint::new(self.right, self.bottom),
            LayoutPoint::new(self.left, self.bottom),
        ]
    }
}

/// An affine map from one coordinate space to another: `x' = a·x + c·y + e`, `y' = b·x + d·y + f`.
///
/// A box model needs more than a rotation. A PowerPoint group carries a child coordinate space
/// (`a:chOff`/`a:chExt`) that is a translate-and-scale, a shape carries a rotation (`a:xfrm@rot`)
/// and two flips (`@flipH`, `@flipV`), and they compose; a Word text frame carries a rotation of its
/// own. One 2×3 affine expresses every one of those and their composition, which a
/// rotation-plus-two-booleans could not.
///
/// Held in `f64` rather than in `Emu`, because the linear part of a transform is a *ratio* and has no
/// unit; only the translation is a length, and it is in EMU like everything else.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Transform {
    /// `a` — how much of the source `x` reaches the target `x`.
    pub scale_x: f64,
    /// `b` — how much of the source `x` reaches the target `y`.
    pub shear_y: f64,
    /// `c` — how much of the source `y` reaches the target `x`.
    pub shear_x: f64,
    /// `d` — how much of the source `y` reaches the target `y`.
    pub scale_y: f64,
    /// `e` — the translation along `x`, in EMU.
    pub translate_x: Emu,
    /// `f` — the translation along `y`, in EMU.
    pub translate_y: Emu,
}

impl Transform {
    /// The map that changes nothing.
    pub const IDENTITY: Self = Self {
        scale_x: 1.0,
        shear_y: 0.0,
        shear_x: 0.0,
        scale_y: 1.0,
        translate_x: Emu::ZERO,
        translate_y: Emu::ZERO,
    };

    /// Whether this is the identity, which the fragment tree uses to keep the common case free.
    #[must_use]
    pub fn is_identity(self) -> bool {
        self == Self::IDENTITY
    }

    /// A translation.
    #[must_use]
    pub const fn translation(dx: Emu, dy: Emu) -> Self {
        Self {
            translate_x: dx,
            translate_y: dy,
            ..Self::IDENTITY
        }
    }

    /// A scale about the origin. A negative factor is a flip, which is exactly what `a:xfrm@flipH`
    /// and `@flipV` are.
    #[must_use]
    pub const fn scale(x: f64, y: f64) -> Self {
        Self {
            scale_x: x,
            scale_y: y,
            ..Self::IDENTITY
        }
    }

    /// A rotation about the origin, **clockwise on the page**, because `y` grows downward.
    #[must_use]
    pub fn rotation(angle: Angle) -> Self {
        let (sine, cosine) = angle.radians().sin_cos();
        Self {
            scale_x: cosine,
            shear_y: sine,
            shear_x: -sine,
            scale_y: cosine,
            ..Self::IDENTITY
        }
    }

    /// A rotation about `centre`, which is what a shape's `a:xfrm@rot` actually is: the shape turns
    /// about the middle of its own box, not about the page's corner.
    #[must_use]
    pub fn rotation_about(angle: Angle, centre: LayoutPoint) -> Self {
        Self::translation(-centre.x, -centre.y)
            .then(Self::rotation(angle))
            .then(Self::translation(centre.x, centre.y))
    }

    /// This map followed by `next` — the composition a nested group produces.
    #[must_use]
    pub fn then(self, next: Self) -> Self {
        Self {
            scale_x: self.scale_x * next.scale_x + self.shear_y * next.shear_x,
            shear_y: self.scale_x * next.shear_y + self.shear_y * next.scale_y,
            shear_x: self.shear_x * next.scale_x + self.scale_y * next.shear_x,
            scale_y: self.shear_x * next.shear_y + self.scale_y * next.scale_y,
            translate_x: Emu::from_emu_rounded(
                self.translate_x.emu() as f64 * next.scale_x
                    + self.translate_y.emu() as f64 * next.shear_x,
            ) + next.translate_x,
            translate_y: Emu::from_emu_rounded(
                self.translate_x.emu() as f64 * next.shear_y
                    + self.translate_y.emu() as f64 * next.scale_y,
            ) + next.translate_y,
        }
    }

    /// Where `point` lands.
    #[must_use]
    pub fn apply(self, point: LayoutPoint) -> LayoutPoint {
        let x = point.x.emu() as f64;
        let y = point.y.emu() as f64;
        LayoutPoint {
            x: Emu::from_emu_rounded(x * self.scale_x + y * self.shear_x) + self.translate_x,
            y: Emu::from_emu_rounded(x * self.shear_y + y * self.scale_y) + self.translate_y,
        }
    }

    /// The determinant of the linear part. Zero means the map collapses the plane onto a line, which
    /// is what a scale of zero on either axis does, and is why [`Transform::inverse`] is fallible.
    #[must_use]
    pub fn determinant(self) -> f64 {
        self.scale_x * self.scale_y - self.shear_x * self.shear_y
    }

    /// The map back, or `None` when the determinant is zero or not finite.
    ///
    /// This is the narrow phase of a hit test through a rotated shape: the point comes back into the
    /// shape's own coordinates, where the rectangle is axis-aligned again.
    #[must_use]
    pub fn inverse(self) -> Option<Self> {
        let determinant = self.determinant();
        if !determinant.is_finite() || determinant == 0.0 {
            return None;
        }
        let scale_x = self.scale_y / determinant;
        let shear_y = -self.shear_y / determinant;
        let shear_x = -self.shear_x / determinant;
        let scale_y = self.scale_x / determinant;
        let translate_x = self.translate_x.emu() as f64;
        let translate_y = self.translate_y.emu() as f64;
        Some(Self {
            scale_x,
            shear_y,
            shear_x,
            scale_y,
            translate_x: Emu::from_emu_rounded(-(translate_x * scale_x + translate_y * shear_x)),
            translate_y: Emu::from_emu_rounded(-(translate_x * shear_y + translate_y * scale_y)),
        })
    }

    /// The axis-aligned bounding box of `rect` after this map.
    ///
    /// A rotated rectangle is not a rectangle, so this is the *broad phase* answer: the smallest
    /// axis-aligned box that certainly contains it. The spatial index stores exactly this, and an
    /// exact answer needs [`Transform::inverse`] and the untransformed rectangle.
    #[must_use]
    pub fn map_rect_bounds(self, rect: LayoutRect) -> LayoutRect {
        if self.is_identity() {
            return rect;
        }
        let mut corners = rect.corners().into_iter().map(|corner| self.apply(corner));
        let Some(first) = corners.next() else {
            return rect;
        };
        let mut bounds = LayoutRect {
            left: first.x,
            top: first.y,
            right: first.x,
            bottom: first.y,
        };
        for corner in corners {
            bounds.left = bounds.left.minimum(corner.x);
            bounds.top = bounds.top.minimum(corner.y);
            bounds.right = bounds.right.maximum(corner.x);
            bounds.bottom = bounds.bottom.maximum(corner.y);
        }
        bounds
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}
