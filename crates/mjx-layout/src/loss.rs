//! What a box model could not lay out, counted where it happened and carried beside the fragments.

use crate::fragment::{ClipId, TransformId};
use crate::measure::LayoutRect;
use crate::source::SourceRef;

/// What a frame holds that a box model placed and did not lay out.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum FrameContent {
    /// A chart.
    Chart,
    /// A diagram, such as a SmartArt graphic.
    Diagram,
    /// An object embedded from another application.
    EmbeddedObject,
    /// Handwriting.
    Ink,
    /// A picture, such as a conditional-format icon.
    Picture,
    /// A drawn shape, such as an autoshape, a connector or a group of them.
    Shape,
}

impl FrameContent {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 6] = [
        Self::Chart,
        Self::Diagram,
        Self::EmbeddedObject,
        Self::Ink,
        Self::Picture,
        Self::Shape,
    ];

    /// What a placeholder for this content reads.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Chart => "Chart not rendered",
            Self::Diagram => "Diagram not rendered",
            Self::EmbeddedObject => "Embedded object not rendered",
            Self::Ink => "Ink not rendered",
            Self::Picture => "Picture not rendered",
            Self::Shape => "Shape not rendered",
        }
    }
}

/// How a box model fell short of the document.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum LayoutLossKind {
    /// A frame was placed and what it holds was not laid out.
    FrameContentNotLaidOut(FrameContent),
    /// Text was measured with nominal metrics rather than shaped.
    TextMeasuredNotShaped,
    /// The reader dropped content the file states, so nothing could lay it out.
    DroppedByReader,
    /// A value was laid out as the nearest thing the layout can express.
    ValueApproximated,
}

impl LayoutLossKind {
    /// Every kind, frame contents first in their own order.
    pub const ALL: [Self; 9] = [
        Self::FrameContentNotLaidOut(FrameContent::Chart),
        Self::FrameContentNotLaidOut(FrameContent::Diagram),
        Self::FrameContentNotLaidOut(FrameContent::EmbeddedObject),
        Self::FrameContentNotLaidOut(FrameContent::Ink),
        Self::FrameContentNotLaidOut(FrameContent::Picture),
        Self::FrameContentNotLaidOut(FrameContent::Shape),
        Self::TextMeasuredNotShaped,
        Self::DroppedByReader,
        Self::ValueApproximated,
    ];

    /// What a placeholder, a report or a ledger row calls this loss.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::FrameContentNotLaidOut(content) => content.label(),
            Self::TextMeasuredNotShaped => "Text not shaped",
            Self::DroppedByReader => "Content not read",
            Self::ValueApproximated => "Approximated",
        }
    }

    /// Whether content is missing, so the element gets a placeholder; an approximation still draws.
    #[must_use]
    pub const fn draws_placeholder(self) -> bool {
        matches!(
            self,
            Self::FrameContentNotLaidOut(_) | Self::DroppedByReader
        )
    }
}

/// Where a lost element sits, in its own space, so a placeholder can be drawn over it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LossArea {
    /// The element's rectangle, in the space `transform` names.
    pub rect: LayoutRect,
    /// The transform the element's fragments are in.
    pub transform: TransformId,
    /// The clip the element's fragments are drawn under.
    pub clip: Option<ClipId>,
}

/// One loss, at the address of the element that suffered it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LayoutLoss {
    /// The element's address in the document.
    pub source: SourceRef,
    /// What was lost.
    pub kind: LayoutLossKind,
    /// Where the element is, or `None` for an element with no area of its own.
    pub area: Option<LossArea>,
}

/// Every loss one page's layout recorded, in the order they were met.
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct LayoutLosses {
    losses: Vec<LayoutLoss>,
}

impl LayoutLosses {
    /// No losses.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a loss of an element with no area of its own.
    pub fn record(&mut self, source: SourceRef, kind: LayoutLossKind) {
        self.losses.push(LayoutLoss {
            source,
            kind,
            area: None,
        });
    }

    /// Records a loss of an element that occupies `area`.
    pub fn record_at(&mut self, source: SourceRef, kind: LayoutLossKind, area: LossArea) {
        self.losses.push(LayoutLoss {
            source,
            kind,
            area: Some(area),
        });
    }

    /// Every loss, in recording order.
    pub fn iter(&self) -> std::slice::Iter<'_, LayoutLoss> {
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
    pub fn count(&self, kind: LayoutLossKind) -> usize {
        self.losses.iter().filter(|loss| loss.kind == kind).count()
    }
}

impl<'a> IntoIterator for &'a LayoutLosses {
    type Item = &'a LayoutLoss;
    type IntoIter = std::slice::Iter<'a, LayoutLoss>;

    fn into_iter(self) -> Self::IntoIter {
        self.losses.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{PartId, SourcePath};

    fn at(segments: &[u32]) -> SourceRef {
        SourceRef::node(PartId::PRIMARY, SourcePath::new(segments))
    }

    #[test]
    fn a_count_names_one_kind_and_the_length_names_them_all() {
        let mut losses = LayoutLosses::new();
        losses.record(at(&[0]), LayoutLossKind::ValueApproximated);
        losses.record_at(
            at(&[1]),
            LayoutLossKind::FrameContentNotLaidOut(FrameContent::Ink),
            LossArea {
                rect: LayoutRect::default(),
                transform: TransformId::IDENTITY,
                clip: None,
            },
        );
        assert_eq!(losses.len(), 2);
        assert_eq!(losses.count(LayoutLossKind::ValueApproximated), 1);
        assert_eq!(
            losses.count(LayoutLossKind::FrameContentNotLaidOut(FrameContent::Chart)),
            0
        );
        assert!(losses.iter().nth(1).is_some_and(|loss| loss.area.is_some()));
    }

    #[test]
    fn only_missing_content_draws_a_placeholder() {
        let drawn: Vec<bool> = LayoutLossKind::ALL
            .iter()
            .map(|kind| kind.draws_placeholder())
            .collect();
        assert_eq!(
            drawn,
            [true, true, true, true, true, true, false, true, false]
        );
    }

    #[test]
    fn every_label_is_distinct() {
        let mut labels: Vec<&str> = LayoutLossKind::ALL
            .iter()
            .map(|kind| kind.label())
            .collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), LayoutLossKind::ALL.len());
    }
}
