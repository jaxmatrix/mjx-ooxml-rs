//! [`PresentationSession`] — a `.pptx` held open.

use mjx_layout::LayoutRect;
use mjx_layout::{PartId, SourceRef};
use mjx_ooxml_core::measure::Emu;
use mjx_pptx::{Presentation, ShapeBounds, Surface};

use crate::document::{Applied, Committed, Invalidation, ResidentDocument};
use crate::error::SessionError;
use crate::ooxml::DirtyEstimate;
use crate::operation::{Operation, OperationKind, Value};

/// A presentation a session holds open.
///
/// # The addressing scheme
///
/// A `SourceRef` means nothing without the box model that issued it, so this is that box model's
/// half of the contract, written down:
///
/// | Piece | Meaning |
/// |---|---|
/// | [`PartId`] `0` | slides · `1` layouts · `2` masters · `3` notes slides |
/// | path segment `0` | which surface of that kind |
/// | path segments `1 ..` | the shape path — one index for a top-level shape, more to descend into `p:grpSp` groups |
/// | the last **two** segments, for a text operation | the paragraph index and the run index inside the shape |
///
/// So `[2, 4]` under [`PartId::PRIMARY`] is slide 2's fifth top-level shape, and `[2, 4, 0, 1]` is
/// that shape's first paragraph's second run. A text operation needs at least four segments; a
/// bounds operation needs at least two.
///
/// The surface *kind* is the part number rather than a path segment because it is the one thing that
/// selects a different content stream: a slide and a layout are different parts of the package, and a
/// `PartId` is exactly "which part".
///
/// # What it can do
///
/// [`Value::Text`] and [`Value::Empty`] set a run's text; [`OperationKind::SetBounds`] places a
/// shape. Anything else is refused rather than approximated — a slide has no string pool and no
/// error values, and inventing a mapping for them would be this crate authoring markup.
///
/// Everything else `mjx-pptx` can do is still reachable through
/// [`Session::document_mut`](crate::Session::document_mut); it is simply not journalled, because a
/// journal entry this crate cannot invert is an undo that would lose work.
#[derive(Debug)]
pub struct PresentationSession {
    presentation: Presentation,
    dirty: DirtyEstimate,
}

impl PresentationSession {
    /// Holds `presentation` open.
    #[must_use]
    pub fn new(presentation: Presentation) -> Self {
        Self {
            presentation,
            dirty: DirtyEstimate::default(),
        }
    }

    /// Opens a deck from its container bytes and holds it open.
    ///
    /// # Errors
    /// [`SessionError::Document`] carrying the `PptxError` if the package is unreadable.
    pub fn open(bytes: &[u8]) -> Result<Self, SessionError> {
        Presentation::open(bytes)
            .map(Self::new)
            .map_err(SessionError::document)
    }

    /// The deck, for reading.
    #[must_use]
    pub const fn presentation(&self) -> &Presentation {
        &self.presentation
    }

    /// The deck, mutably. Anything done through here is not journalled — see
    /// [`Session::document_mut`](crate::Session::document_mut).
    pub fn presentation_mut(&mut self) -> &mut Presentation {
        &mut self.presentation
    }

    /// Gives the deck back.
    #[must_use]
    pub fn into_presentation(self) -> Presentation {
        self.presentation
    }

    /// The surface an address names.
    fn surface(address: &SourceRef) -> Result<Surface, SessionError> {
        let segments = address.path().segments();
        let Some(&index) = segments.first() else {
            return Err(SessionError::no_such_node(describe(address)));
        };
        let index = index as usize;
        match address.part().number() {
            0 => Ok(Surface::Slide(index)),
            1 => Ok(Surface::Layout(index)),
            2 => Ok(Surface::Master(index)),
            3 => Ok(Surface::Notes(index)),
            other => Err(SessionError::no_such_node(format!(
                "part {other} — a presentation numbers slides 0, layouts 1, masters 2 and notes 3"
            ))),
        }
    }

    /// The shape path a bounds operation names: everything after the surface index.
    fn shape_path(address: &SourceRef) -> Result<Vec<usize>, SessionError> {
        let segments = address.path().segments();
        if segments.len() < 2 {
            return Err(SessionError::no_such_node(format!(
                "{} — a shape needs a surface index and at least one shape index",
                describe(address)
            )));
        }
        Ok(segments[1..].iter().map(|&it| it as usize).collect())
    }

    /// The shape path, paragraph and run a text operation names.
    fn text_path(address: &SourceRef) -> Result<(Vec<usize>, usize, usize), SessionError> {
        let segments = address.path().segments();
        if segments.len() < 4 {
            return Err(SessionError::no_such_node(format!(
                "{} — a run needs a surface index, a shape path, a paragraph index and a run index",
                describe(address)
            )));
        }
        let split = segments.len() - 2;
        let shape = segments[1..split].iter().map(|&it| it as usize).collect();
        Ok((
            shape,
            segments[split] as usize,
            segments[split + 1] as usize,
        ))
    }

    /// The index `set_shape_text` wants: runs flattened over the shape's paragraphs.
    ///
    /// `set_shape_text` counts runs across the whole shape while `run_text` counts them inside one
    /// paragraph, so the two are not the same number and cannot be used against each other without
    /// this. Reading `run_count` per paragraph does not dirty the part.
    fn flattened_run_index(
        &mut self,
        surface: Surface,
        shape: &[usize],
        paragraph: usize,
        run: usize,
    ) -> Result<usize, SessionError> {
        let mut flattened = run;
        for earlier in 0..paragraph {
            flattened += self
                .presentation
                .run_count(surface, shape.to_vec(), earlier)
                .map_err(SessionError::document)?;
        }
        Ok(flattened)
    }

    fn apply_text(&mut self, address: &SourceRef, text: &str) -> Result<Applied, SessionError> {
        let surface = Self::surface(address)?;
        let (shape, paragraph, run) = Self::text_path(address)?;
        let was = self
            .presentation
            .run_text(surface, shape.clone(), paragraph, run)
            .map_err(SessionError::document)?;
        let flattened = self.flattened_run_index(surface, &shape, paragraph, run)?;
        self.presentation
            .set_shape_text(surface, shape, flattened, text)
            .map_err(SessionError::document)?;
        self.dirty.note(text.len());
        Ok(Applied {
            inverse: Operation::set_value(address.clone(), Value::text(was)),
            invalidation: Invalidation::at(address),
        })
    }

    fn apply_bounds(
        &mut self,
        address: &SourceRef,
        bounds: LayoutRect,
    ) -> Result<Applied, SessionError> {
        let surface = Self::surface(address)?;
        let shape = Self::shape_path(address)?;
        let was = self
            .presentation
            .shape_bounds(surface, shape.clone())
            .map_err(SessionError::document)?
            .ok_or_else(|| {
                // A shape with no transform at all has no rectangle to put back, so placing it would
                // be an edit this crate could not undo exactly.
                SessionError::no_exact_inverse(format!(
                    "{} — the shape states no transform, so there is no rectangle to restore",
                    describe(address)
                ))
            })?;
        self.presentation
            .set_shape_bounds(surface, shape, to_shape_bounds(bounds))
            .map_err(SessionError::document)?;
        self.dirty.note(0);
        Ok(Applied {
            inverse: Operation::set_bounds(address.clone(), from_shape_bounds(was)),
            invalidation: Invalidation::at(address),
        })
    }
}

impl ResidentDocument for PresentationSession {
    fn apply(&mut self, operation: &Operation) -> Result<Applied, SessionError> {
        match operation.kind() {
            OperationKind::SetValue(Value::Text(text)) => {
                self.apply_text(operation.address(), text)
            }
            OperationKind::SetValue(Value::Empty) => self.apply_text(operation.address(), ""),
            OperationKind::SetValue(other) => Err(SessionError::unsupported(format!(
                "put {other:?} into a run — a slide holds text and nothing else this vocabulary spells"
            ))),
            OperationKind::SetBounds(bounds) => self.apply_bounds(operation.address(), *bounds),
        }
    }

    fn dirty_bytes(&self) -> usize {
        self.dirty.bytes()
    }

    fn commit(&mut self) -> Result<Committed, SessionError> {
        let serialised = self.presentation.settle_dirty_parts().len();
        let bytes = self.presentation.save().map_err(SessionError::document)?;
        self.dirty.clear();
        Ok(Committed {
            parts_serialised: serialised,
            bytes,
        })
    }
}

/// How an address reads in a message.
fn describe(address: &SourceRef) -> String {
    format!(
        "part {} path {:?}",
        address.part().number(),
        address.path().segments()
    )
}

fn to_shape_bounds(rect: LayoutRect) -> ShapeBounds {
    ShapeBounds::new(
        rect.left.emu(),
        rect.top.emu(),
        rect.width().emu(),
        rect.height().emu(),
    )
}

fn from_shape_bounds(bounds: ShapeBounds) -> LayoutRect {
    LayoutRect::from_edges(
        Emu::from_emu(bounds.offset_x_emu),
        Emu::from_emu(bounds.offset_y_emu),
        Emu::from_emu(bounds.offset_x_emu.saturating_add(bounds.width_emu)),
        Emu::from_emu(bounds.offset_y_emu.saturating_add(bounds.height_emu)),
    )
}

/// The part numbers this residency answers to, named rather than spelled at every call site.
impl PresentationSession {
    /// Slides.
    pub const SLIDES: PartId = PartId::new(0);
    /// Slide layouts.
    pub const LAYOUTS: PartId = PartId::new(1);
    /// Slide masters.
    pub const MASTERS: PartId = PartId::new(2);
    /// Notes slides.
    pub const NOTES: PartId = PartId::new(3);
}
