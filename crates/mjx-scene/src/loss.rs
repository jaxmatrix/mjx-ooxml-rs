//! What a scene could not resolve, the category every loss above the box model is named in, and the placeholder that stands where content is missing.

use mjx_layout::{FrameContent, LayoutLossKind, SourceRef};

use crate::geometry::{SceneRect, SceneTransform};

/// How a resolver fell short of what a fragment asked it for.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum SceneLossKind {
    /// A chart was laid out and nothing could resolve what it paints.
    ChartNotResolved,
    /// A colour could not be resolved to a value.
    ColourNotResolved,
    /// A fill names a picture the resolver holds no entry for.
    FillImageNotSupplied,
    /// A run of text was drawn in the default colour because its own paint was not answered.
    TextPaintDefaulted,
}

impl SceneLossKind {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 4] = [
        Self::ChartNotResolved,
        Self::ColourNotResolved,
        Self::FillImageNotSupplied,
        Self::TextPaintDefaulted,
    ];

    /// What a placeholder, a report or a ledger row calls this loss.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ChartNotResolved => "Chart not rendered",
            Self::ColourNotResolved => "Colour not resolved",
            Self::FillImageNotSupplied => "Fill picture not available",
            Self::TextPaintDefaulted => "Text colour approximated",
        }
    }

    /// Whether something is missing, so the element gets a placeholder; an approximation still draws.
    #[must_use]
    pub const fn draws_placeholder(self) -> bool {
        !matches!(self, Self::TextPaintDefaulted)
    }

    /// Whether the placeholder stands for everything inside the element, so nothing under it is drawn.
    #[must_use]
    pub const fn replaces_content(self) -> bool {
        matches!(self, Self::ChartNotResolved)
    }

    // The kind's number in a display list's loss section.
    pub(crate) const fn wire_value(self) -> u8 {
        match self {
            Self::ChartNotResolved => 1,
            Self::ColourNotResolved => 2,
            Self::FillImageNotSupplied => 3,
            Self::TextPaintDefaulted => 4,
        }
    }

    // The kind a loss section's number names.
    pub(crate) const fn from_wire_value(value: u8) -> Option<Self> {
        Some(match value {
            1 => Self::ChartNotResolved,
            2 => Self::ColourNotResolved,
            3 => Self::FillImageNotSupplied,
            4 => Self::TextPaintDefaulted,
            _ => return None,
        })
    }
}

/// One scene loss, at the address of the fragment that suffered it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SceneLoss {
    /// The fragment's address in the document.
    pub source: SourceRef,
    /// What was lost.
    pub kind: SceneLossKind,
}

/// Every scene loss a display list records, in paint order.
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct SceneLosses {
    losses: Vec<SceneLoss>,
}

impl SceneLosses {
    /// Wraps losses already in paint order.
    #[must_use]
    pub fn from_losses(losses: Vec<SceneLoss>) -> Self {
        Self { losses }
    }

    /// Every loss, in paint order.
    pub fn iter(&self) -> std::slice::Iter<'_, SceneLoss> {
        self.losses.iter()
    }

    /// How many losses there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.losses.len()
    }

    /// Whether nothing was lost.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.losses.is_empty()
    }

    /// How many losses are of `kind`.
    #[must_use]
    pub fn count(&self, kind: SceneLossKind) -> usize {
        self.losses.iter().filter(|loss| loss.kind == kind).count()
    }
}

impl<'a> IntoIterator for &'a SceneLosses {
    type Item = &'a SceneLoss;
    type IntoIter = std::slice::Iter<'a, SceneLoss>;

    fn into_iter(self) -> Self::IntoIter {
        self.losses.iter()
    }
}

/// Which stage lost something, and what: the one name a placeholder carries whichever stage recorded it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum LossCategory {
    /// The box model could not lay the content out.
    Layout(LayoutLossKind),
    /// The resolver could not answer what the content paints.
    Scene(SceneLossKind),
}

impl LossCategory {
    /// What a placeholder of this category reads.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Layout(kind) => kind.label(),
            Self::Scene(kind) => kind.label(),
        }
    }

    /// Whether a loss of this category draws a placeholder.
    #[must_use]
    pub const fn draws_placeholder(self) -> bool {
        match self {
            Self::Layout(kind) => kind.draws_placeholder(),
            Self::Scene(kind) => kind.draws_placeholder(),
        }
    }

    // The category as a loss record's stage byte and kind byte.
    pub(crate) const fn wire(self) -> (u8, u8) {
        match self {
            Self::Layout(kind) => (LAYOUT_STAGE, layout_wire_value(kind)),
            Self::Scene(kind) => (SCENE_STAGE, kind.wire_value()),
        }
    }

    // The category a loss record's stage and kind bytes name.
    pub(crate) const fn from_wire(stage: u8, kind: u8) -> Option<Self> {
        match stage {
            LAYOUT_STAGE => match layout_from_wire_value(kind) {
                Some(kind) => Some(Self::Layout(kind)),
                None => None,
            },
            SCENE_STAGE => match SceneLossKind::from_wire_value(kind) {
                Some(kind) => Some(Self::Scene(kind)),
                None => None,
            },
            _ => None,
        }
    }
}

// The stage byte of a layout loss.
const LAYOUT_STAGE: u8 = 1;
// The stage byte of a scene loss.
const SCENE_STAGE: u8 = 2;

// A layout loss kind's number in a display list's loss section.
const fn layout_wire_value(kind: LayoutLossKind) -> u8 {
    match kind {
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::Chart) => 1,
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::Diagram) => 2,
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::EmbeddedObject) => 3,
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::Ink) => 4,
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::Picture) => 5,
        LayoutLossKind::TextMeasuredNotShaped => 6,
        LayoutLossKind::DroppedByReader => 7,
        LayoutLossKind::ValueApproximated => 8,
    }
}

// The layout loss kind a loss section's number names.
const fn layout_from_wire_value(value: u8) -> Option<LayoutLossKind> {
    Some(match value {
        1 => LayoutLossKind::FrameContentNotLaidOut(FrameContent::Chart),
        2 => LayoutLossKind::FrameContentNotLaidOut(FrameContent::Diagram),
        3 => LayoutLossKind::FrameContentNotLaidOut(FrameContent::EmbeddedObject),
        4 => LayoutLossKind::FrameContentNotLaidOut(FrameContent::Ink),
        5 => LayoutLossKind::FrameContentNotLaidOut(FrameContent::Picture),
        6 => LayoutLossKind::TextMeasuredNotShaped,
        7 => LayoutLossKind::DroppedByReader,
        8 => LayoutLossKind::ValueApproximated,
        _ => return None,
    })
}

/// A labelled stand-in drawn over content that cannot be drawn, and what a hit test on it answers.
#[derive(Clone, PartialEq, Debug)]
pub struct Placeholder {
    /// The element's rectangle, in the space `transform` maps to the page.
    pub rect: SceneRect,
    /// The map from the element's space to the page, in device pixels.
    pub transform: SceneTransform,
    /// What the placeholder reads.
    pub label: String,
    /// What was lost.
    pub category: LossCategory,
    /// The element's address in the document.
    pub source: SourceRef,
    /// The position in the command stream the placeholder is drawn at.
    pub command: u32,
}

impl Placeholder {
    /// Whether the page point `(x, y)` falls inside the placeholder.
    #[must_use]
    pub fn contains(&self, x: f32, y: f32) -> bool {
        let Some(inverse) = invert(self.transform) else {
            return false;
        };
        let (local_x, local_y) = apply(inverse, x, y);
        local_x >= self.rect.left
            && local_x < self.rect.right
            && local_y >= self.rect.top
            && local_y < self.rect.bottom
    }
}

/// A resolver's answer: what to draw, that there is nothing to draw, or that it cannot say.
#[derive(Clone, PartialEq, Debug)]
pub enum Resolved<T> {
    /// The value to draw with.
    Answered(T),
    /// The fragment genuinely paints nothing.
    NothingToDraw,
    /// The fragment paints something the resolver cannot supply, and this is why.
    Unanswerable(SceneLossKind),
}

impl<T> Resolved<T> {
    /// The answer, if there is one.
    pub fn answered(self) -> Option<T> {
        match self {
            Self::Answered(value) => Some(value),
            Self::NothingToDraw | Self::Unanswerable(_) => None,
        }
    }
}

impl<T> From<Option<T>> for Resolved<T> {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::NothingToDraw, Self::Answered)
    }
}

// `outer` applied to the result of `inner`.
pub(crate) fn compose(outer: SceneTransform, inner: SceneTransform) -> SceneTransform {
    SceneTransform {
        scale_x: outer.scale_x * inner.scale_x + outer.shear_x * inner.shear_y,
        shear_y: outer.shear_y * inner.scale_x + outer.scale_y * inner.shear_y,
        shear_x: outer.scale_x * inner.shear_x + outer.shear_x * inner.scale_y,
        scale_y: outer.shear_y * inner.shear_x + outer.scale_y * inner.scale_y,
        translate_x: outer.scale_x * inner.translate_x
            + outer.shear_x * inner.translate_y
            + outer.translate_x,
        translate_y: outer.shear_y * inner.translate_x
            + outer.scale_y * inner.translate_y
            + outer.translate_y,
    }
}

// A point put through a transform.
fn apply(transform: SceneTransform, x: f32, y: f32) -> (f32, f32) {
    (
        transform.scale_x * x + transform.shear_x * y + transform.translate_x,
        transform.shear_y * x + transform.scale_y * y + transform.translate_y,
    )
}

// The map that undoes `transform`, or `None` when it collapses the plane.
fn invert(transform: SceneTransform) -> Option<SceneTransform> {
    let determinant = transform.scale_x * transform.scale_y - transform.shear_x * transform.shear_y;
    if determinant == 0.0 || !determinant.is_finite() {
        return None;
    }
    let scale_x = transform.scale_y / determinant;
    let shear_y = -transform.shear_y / determinant;
    let shear_x = -transform.shear_x / determinant;
    let scale_y = transform.scale_x / determinant;
    Some(SceneTransform {
        scale_x,
        shear_y,
        shear_x,
        scale_y,
        translate_x: -(scale_x * transform.translate_x + shear_x * transform.translate_y),
        translate_y: -(shear_y * transform.translate_x + scale_y * transform.translate_y),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_category_survives_its_wire_bytes() {
        let categories = LayoutLossKind::ALL
            .iter()
            .map(|kind| LossCategory::Layout(*kind))
            .chain(
                SceneLossKind::ALL
                    .iter()
                    .map(|kind| LossCategory::Scene(*kind)),
            );
        for category in categories {
            let (stage, kind) = category.wire();
            assert_eq!(LossCategory::from_wire(stage, kind), Some(category));
        }
        assert_eq!(LossCategory::from_wire(3, 1), None);
        assert_eq!(LossCategory::from_wire(2, 0), None);
    }

    #[test]
    fn a_placeholder_is_hit_inside_its_transformed_rectangle_and_nowhere_else() {
        let placeholder = Placeholder {
            rect: SceneRect::new(0.0, 0.0, 10.0, 10.0),
            transform: SceneTransform {
                translate_x: 100.0,
                translate_y: 50.0,
                ..SceneTransform::IDENTITY
            },
            label: SceneLossKind::ChartNotResolved.label().to_owned(),
            category: LossCategory::Scene(SceneLossKind::ChartNotResolved),
            source: SourceRef::node(mjx_layout::PartId::PRIMARY, mjx_layout::SourcePath::root()),
            command: 0,
        };
        assert!(placeholder.contains(105.0, 55.0));
        assert!(!placeholder.contains(5.0, 5.0));
    }

    #[test]
    fn only_a_chart_replaces_what_is_inside_it_and_only_text_paint_draws_no_placeholder() {
        let replaces: Vec<bool> = SceneLossKind::ALL
            .iter()
            .map(|kind| kind.replaces_content())
            .collect();
        assert_eq!(replaces, [true, false, false, false]);
        let placeholders: Vec<bool> = SceneLossKind::ALL
            .iter()
            .map(|kind| kind.draws_placeholder())
            .collect();
        assert_eq!(placeholders, [true, true, true, false]);
    }
}
