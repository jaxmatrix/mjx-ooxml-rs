//! [`ShapeCursor`] — one shape, addressed once, edited fluently.
//!
//! The type carries the documentation, since it is what a reader of this crate sees; this module is
//! private and holds only it and the [`ShapeEdit`] intent it records.

use mjx_dml::{
    CharacterPropertiesSpec, EffectListSpec, FillSpec, LineSpec, ParagraphPropertiesSpec,
    ShapeGeometry, Transform2D,
};

use crate::address::ShapePath;
use crate::error::PptxError;
use crate::geometry::ShapeBounds;
use crate::hyperlink::Hyperlink;
use crate::presentation::Presentation;
use crate::slide::ShapeKind;
use crate::surface::Surface;

/// One recorded edit. Each variant is the argument of exactly one `Presentation::set_shape_*` method,
/// owned rather than borrowed so a cursor can be held across statements.
///
/// The rel-bearing variants ([`Hyperlink`](ShapeEdit::Hyperlink), [`Image`](ShapeEdit::Image)) carry
/// the caller's intent, not a relationship id: the package work that turns one into the other happens
/// in `Presentation::apply_shape_edits`, so an unapplied cursor never touches the package.
#[derive(Debug)]
pub(crate) enum ShapeEdit {
    /// `set_shape_fill` (and `set_shape_no_fill`, as [`FillSpec::None`]).
    Fill(FillSpec),
    /// `set_shape_outline` (and `set_shape_no_outline`, as a line whose fill is [`FillSpec::None`]).
    Outline(LineSpec),
    /// `set_shape_effects` (and `set_shape_no_effects`, as an empty list).
    Effects(Box<EffectListSpec>),
    /// `set_shape_geometry`.
    Geometry(ShapeGeometry),
    /// `set_shape_transform` — and `set_shape_bounds`, which is a transform naming only offset and
    /// extent.
    Transform(Transform2D),
    /// `set_shape_text_content`.
    Text(String),
    /// `set_run_properties`.
    RunProperties {
        paragraph: usize,
        run: usize,
        spec: Box<CharacterPropertiesSpec>,
    },
    /// `set_paragraph_run_properties`.
    ParagraphRunProperties {
        paragraph: usize,
        spec: Box<CharacterPropertiesSpec>,
    },
    /// `set_shape_run_properties`.
    AllRunProperties(Box<CharacterPropertiesSpec>),
    /// `set_end_run_properties`.
    EndRunProperties {
        paragraph: usize,
        spec: Box<CharacterPropertiesSpec>,
    },
    /// `set_paragraph_properties`.
    ParagraphProperties {
        paragraph: usize,
        spec: Box<ParagraphPropertiesSpec>,
    },
    /// `set_text_range_properties`, or its `_by_grapheme` sibling when `graphemes` is set — the unit
    /// the offsets are counted in is part of the intent, and the conversion needs the paragraph's
    /// text, which is only in hand once the body is parsed.
    TextRangeProperties {
        paragraph: usize,
        range: core::ops::Range<usize>,
        spec: Box<CharacterPropertiesSpec>,
        graphemes: bool,
    },
    /// `set_shape_hyperlink`, or `clear_shape_hyperlink` when `None`.
    Hyperlink(Option<Hyperlink>),
    /// `set_picture_image`.
    Image(Vec<u8>),
}

impl ShapeEdit {
    /// Whether this edit is applied against the **parsed** text model rather than the raw tree.
    ///
    /// A consecutive run of these on one shape shares a single `TextBody` parse and rebuild, which is
    /// why formatting a paragraph and then a range of it costs one round trip, not two.
    /// [`Text`](ShapeEdit::Text) is deliberately *not* one of them: it rewrites the body's paragraphs
    /// on the raw tree, so it ends any run in progress and the next one sees the new paragraphs.
    pub(crate) fn edits_text_model(&self) -> bool {
        matches!(
            self,
            Self::RunProperties { .. }
                | Self::ParagraphRunProperties { .. }
                | Self::AllRunProperties(_)
                | Self::EndRunProperties { .. }
                | Self::ParagraphProperties { .. }
                | Self::TextRangeProperties { .. }
        )
    }
}

/// One shape, addressed once and edited fluently.
///
/// The flat API states an address on every call, which is right when a caller means one thing and
/// wrong the moment they mean several. Restyling a member of a group reads as a column of calls, each
/// repeating the surface and the path, each re-resolving the shape tree:
///
/// ```no_run
/// # use mjx_pptx::{Presentation, PptxError};
/// # use mjx_dml::{FillSpec, LineSpec};
/// # fn f(deck: &mut Presentation, navy: FillSpec, rule: LineSpec) -> Result<(), PptxError> {
/// deck.set_shape_fill(0, [2, 0], &navy)?;
/// deck.set_shape_outline(0, [2, 0], &rule)?;
/// deck.set_shape_text_content(0, [2, 0], "Q3")?;
/// # Ok(())
/// # }
/// ```
///
/// A cursor says the address once and the edits after it. [`member(i)`](Self::member) descends into a
/// group, [`sibling(i)`](Self::sibling) moves across to another member of the same container, and
/// [`parent()`](Self::parent) steps back out — so a whole group is restyled in one expression:
///
/// ```no_run
/// # use mjx_pptx::{Presentation, PptxError};
/// # use mjx_dml::{CharacterPropertiesSpec, EffectListSpec, FillSpec, LineSpec};
/// # fn f(
/// #     deck: &mut Presentation,
/// #     navy: FillSpec,
/// #     gold: FillSpec,
/// #     rule: LineSpec,
/// #     shadow: EffectListSpec,
/// #     bold: CharacterPropertiesSpec,
/// # ) -> Result<(), PptxError> {
/// deck.shape(0, 2)?                                  // the group at top-level index 2
///     .effects(shadow)
///     .member(0)?.fill(navy).outline(rule)           // its first member
///     .sibling(1)?.fill(gold).text("Q3").all_run_properties(bold)
///     .apply()?;                                     // one write pass, one dirty part
/// # Ok(())
/// # }
/// ```
///
/// # What a cursor is
///
/// It **records intent**. An edit method returns the cursor and writes nothing; [`apply`](Self::apply)
/// consumes it and applies the edits **in the order they were recorded**, marking the part dirty
/// exactly once. Dropping a cursor without applying it changes nothing at all — which is why it is
/// `#[must_use]`.
///
/// Every edit it records is executed by the same code the corresponding `Presentation::set_shape_*`
/// method calls; a cursor is a way of *saying* the edits, never a second way of *doing* them. Each
/// method below names the flat method it mirrors.
///
/// The cursor holds only `(surface, path)` — never a borrowed element — so navigating is free and
/// nothing is pinned between edits. Edits stay bound to the address they were recorded at, so one
/// `.apply()` commits work spread over a group and its members.
///
/// # What it is not
///
/// It does not **read**. A getter on a cursor holding unapplied edits would answer with the state
/// before them, which is a trap; reads stay on [`Presentation`], which always answers about the file
/// as it is. The exceptions are the three questions navigation itself needs — [`kind`](Self::kind),
/// [`member_count`](Self::member_count) and [`path`](Self::path).
///
/// Hyperlinks on a **run** or a text range are addressed by paragraph and run rather than by shape,
/// and stay on the flat API ([`Presentation::set_run_hyperlink`],
/// [`Presentation::set_text_range_hyperlink`]). A cursor's [`hyperlink`](Self::hyperlink) is the link
/// on the shape itself.
#[derive(Debug)]
#[must_use = "a ShapeCursor only records edits; call .apply() to write them"]
pub struct ShapeCursor<'deck> {
    deck: &'deck mut Presentation,
    surface: Surface,
    path: ShapePath,
    edits: Vec<(ShapePath, ShapeEdit)>,
}

impl<'deck> ShapeCursor<'deck> {
    /// Opens a cursor on an address that has already been checked to resolve.
    pub(crate) fn new(deck: &'deck mut Presentation, surface: Surface, path: ShapePath) -> Self {
        Self {
            deck,
            surface,
            path,
            edits: Vec::new(),
        }
    }

    // ---------------------------------------------------------------------------------------
    // Where the cursor is
    // ---------------------------------------------------------------------------------------

    /// The surface the cursor is on.
    #[must_use]
    pub fn surface(&self) -> Surface {
        self.surface
    }

    /// The address the cursor is currently on — what the next recorded edit will be bound to.
    #[must_use]
    pub fn path(&self) -> &ShapePath {
        &self.path
    }

    /// What kind of shape the cursor is on.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the part is malformed. The address itself is checked when the cursor
    /// is opened and by every move, so it cannot be stale.
    pub fn kind(&mut self) -> Result<ShapeKind, PptxError> {
        self.deck.shape_kind(self.surface, &self.path)
    }

    /// How many member shapes the addressed shape holds — `0` for anything that is not a group, since
    /// only a `p:grpSp` has members.
    ///
    /// This is the range [`member`](Self::member) accepts.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the part is malformed.
    pub fn member_count(&mut self) -> Result<usize, PptxError> {
        self.deck.shape_member_count(self.surface, &self.path)
    }

    // ---------------------------------------------------------------------------------------
    // Moving
    //
    // Each move validates against the live tree, so a bad address fails where it was written rather
    // than at `apply`. Edits already recorded keep the address they were recorded at.
    // ---------------------------------------------------------------------------------------

    /// Descends into the addressed group, onto its member `index`.
    ///
    /// # Errors
    /// Returns [`ShapeIsNotAGroup`](PptxError::ShapeIsNotAGroup) if the cursor is not on a `p:grpSp`,
    /// or [`ShapeIndexOutOfRange`](PptxError::ShapeIndexOutOfRange) if the group has no such member.
    pub fn member(self, index: usize) -> Result<Self, PptxError> {
        if self.deck.shape_kind(self.surface, &self.path)? != ShapeKind::GroupShape {
            return Err(PptxError::ShapeIsNotAGroup {
                surface: self.surface,
                path: self.path,
            });
        }
        let mut indices = self.path.indices().to_vec();
        indices.push(index);
        self.move_to(ShapePath::from(indices))
    }

    /// Moves to the shape at `index` in the **same container** — another member of the group this
    /// cursor is inside, or another top-level shape when it is not inside one.
    ///
    /// This is how several members of one group are edited in a single chain, without stepping back
    /// out through [`parent`](Self::parent) and in again.
    ///
    /// # Errors
    /// Returns [`ShapeIndexOutOfRange`](PptxError::ShapeIndexOutOfRange) if the container has no
    /// shape at `index`.
    pub fn sibling(self, index: usize) -> Result<Self, PptxError> {
        let Some((_, container)) = self.path.indices().split_last() else {
            return Err(PptxError::ShapeIndexOutOfRange {
                surface: self.surface,
                path: self.path,
                count: 0,
            });
        };
        let mut indices = container.to_vec();
        indices.push(index);
        self.move_to(ShapePath::from(indices))
    }

    /// Steps back out to the group this shape is a member of.
    ///
    /// # Errors
    /// Returns [`ShapeHasNoParent`](PptxError::ShapeHasNoParent) if the cursor is on a top-level
    /// shape: the shape tree is not itself a shape, so there is nothing to step out to.
    pub fn parent(self) -> Result<Self, PptxError> {
        let indices = self.path.indices();
        let Some(parent) = indices.len().checked_sub(1).filter(|len| *len > 0) else {
            return Err(PptxError::ShapeHasNoParent {
                surface: self.surface,
                path: self.path,
            });
        };
        let parent = ShapePath::from(&indices[..parent]);
        self.move_to(parent)
    }

    /// Re-aims the cursor at `path`, checking it resolves first.
    fn move_to(mut self, path: ShapePath) -> Result<Self, PptxError> {
        self.deck.shape_kind(self.surface, &path)?;
        self.path = path;
        Ok(self)
    }

    // ---------------------------------------------------------------------------------------
    // The `p:spPr` surface
    // ---------------------------------------------------------------------------------------

    /// Fills the shape. Mirrors [`Presentation::set_shape_fill`].
    pub fn fill(self, fill: FillSpec) -> Self {
        self.record(ShapeEdit::Fill(fill))
    }

    /// Gives the shape an explicit "no fill" (`a:noFill`). Mirrors
    /// [`Presentation::set_shape_no_fill`].
    pub fn no_fill(self) -> Self {
        self.record(ShapeEdit::Fill(FillSpec::None))
    }

    /// Outlines the shape. Mirrors [`Presentation::set_shape_outline`].
    pub fn outline(self, line: LineSpec) -> Self {
        self.record(ShapeEdit::Outline(line))
    }

    /// Gives the shape an explicit "no line". Mirrors [`Presentation::set_shape_no_outline`].
    pub fn no_outline(self) -> Self {
        self.record(ShapeEdit::Outline(LineSpec {
            fill: Some(FillSpec::None),
            ..LineSpec::new()
        }))
    }

    /// Applies an effect list to the shape. Mirrors [`Presentation::set_shape_effects`].
    pub fn effects(self, effects: EffectListSpec) -> Self {
        self.record(ShapeEdit::Effects(Box::new(effects)))
    }

    /// Gives the shape explicitly empty effects. Mirrors [`Presentation::set_shape_no_effects`].
    pub fn no_effects(self) -> Self {
        self.record(ShapeEdit::Effects(Box::new(EffectListSpec::new())))
    }

    /// Sets the shape's preset geometry. Mirrors [`Presentation::set_shape_geometry`].
    pub fn geometry(self, geometry: ShapeGeometry) -> Self {
        self.record(ShapeEdit::Geometry(geometry))
    }

    /// Moves and resizes the shape. Mirrors [`Presentation::set_shape_bounds`] — only offset and
    /// extent are written, so a rotation or a group's child coordinate space is left alone.
    pub fn bounds(self, bounds: ShapeBounds) -> Self {
        self.record(ShapeEdit::Transform(bounds.to_transform()))
    }

    /// Applies a transform to the shape. Mirrors [`Presentation::set_shape_transform`] — only the
    /// fields the transform names are written.
    pub fn transform(self, transform: Transform2D) -> Self {
        self.record(ShapeEdit::Transform(transform))
    }

    // ---------------------------------------------------------------------------------------
    // Text, and how it is formatted
    // ---------------------------------------------------------------------------------------

    /// Replaces the shape's whole text — one paragraph per line. Mirrors
    /// [`Presentation::set_shape_text_content`].
    ///
    /// Recorded edits apply in order, so a formatting call *after* this one formats the new text.
    pub fn text(self, text: impl Into<String>) -> Self {
        self.record(ShapeEdit::Text(text.into()))
    }

    /// Formats one run. Mirrors [`Presentation::set_run_properties`].
    pub fn run_properties(
        self,
        paragraph: usize,
        run: usize,
        spec: CharacterPropertiesSpec,
    ) -> Self {
        self.record(ShapeEdit::RunProperties {
            paragraph,
            run,
            spec: Box::new(spec),
        })
    }

    /// Formats every run of one paragraph, and its paragraph mark. Mirrors
    /// [`Presentation::set_paragraph_run_properties`].
    pub fn paragraph_run_properties(self, paragraph: usize, spec: CharacterPropertiesSpec) -> Self {
        self.record(ShapeEdit::ParagraphRunProperties {
            paragraph,
            spec: Box::new(spec),
        })
    }

    /// Formats every run of every paragraph. Mirrors [`Presentation::set_shape_run_properties`].
    pub fn all_run_properties(self, spec: CharacterPropertiesSpec) -> Self {
        self.record(ShapeEdit::AllRunProperties(Box::new(spec)))
    }

    /// Formats a paragraph's mark (`a:endParaRPr`) — how an empty paragraph is styled. Mirrors
    /// [`Presentation::set_end_run_properties`].
    pub fn end_run_properties(self, paragraph: usize, spec: CharacterPropertiesSpec) -> Self {
        self.record(ShapeEdit::EndRunProperties {
            paragraph,
            spec: Box::new(spec),
        })
    }

    /// Lays out one paragraph. Mirrors [`Presentation::set_paragraph_properties`].
    pub fn paragraph_properties(self, paragraph: usize, spec: ParagraphPropertiesSpec) -> Self {
        self.record(ShapeEdit::ParagraphProperties {
            paragraph,
            spec: Box::new(spec),
        })
    }

    /// Formats part of a paragraph, counted in **Unicode scalars**, splitting runs at the boundaries.
    /// Mirrors [`Presentation::set_text_range_properties`].
    pub fn text_range_properties(
        self,
        paragraph: usize,
        range: core::ops::Range<usize>,
        spec: CharacterPropertiesSpec,
    ) -> Self {
        self.record(ShapeEdit::TextRangeProperties {
            paragraph,
            range,
            spec: Box::new(spec),
            graphemes: false,
        })
    }

    /// Formats part of a paragraph, counted in **grapheme clusters** — what a text selection actually
    /// spans. Mirrors [`Presentation::set_text_range_properties_by_grapheme`].
    pub fn text_range_properties_by_grapheme(
        self,
        paragraph: usize,
        range: core::ops::Range<usize>,
        spec: CharacterPropertiesSpec,
    ) -> Self {
        self.record(ShapeEdit::TextRangeProperties {
            paragraph,
            range,
            spec: Box::new(spec),
            graphemes: true,
        })
    }

    // ---------------------------------------------------------------------------------------
    // Edits that reach the package
    // ---------------------------------------------------------------------------------------

    /// Links the shape itself. Mirrors [`Presentation::set_shape_hyperlink`] — the relationship is
    /// added, and the one any previous link named is removed, when the cursor is applied.
    pub fn hyperlink(self, link: Hyperlink) -> Self {
        self.record(ShapeEdit::Hyperlink(Some(link)))
    }

    /// Unlinks the shape. Mirrors [`Presentation::clear_shape_hyperlink`].
    pub fn clear_hyperlink(self) -> Self {
        self.record(ShapeEdit::Hyperlink(None))
    }

    /// Points the addressed picture at `bytes`. Mirrors [`Presentation::set_picture_image`] — the
    /// image part is added (identical bytes are stored once) when the cursor is applied.
    ///
    /// Takes the bytes by value so a caller who already owns them hands them over rather than
    /// copying an image to record the intent.
    pub fn image(self, bytes: impl Into<Vec<u8>>) -> Self {
        self.record(ShapeEdit::Image(bytes.into()))
    }

    // ---------------------------------------------------------------------------------------
    // Finishing
    // ---------------------------------------------------------------------------------------

    /// Binds `edit` to the address the cursor is on now.
    fn record(mut self, edit: ShapeEdit) -> Self {
        self.edits.push((self.path.clone(), edit));
        self
    }

    /// Writes every recorded edit, in the order it was recorded, and marks the part dirty once.
    ///
    /// An edit that fails (a shape with no `p:spPr` to fill, a paragraph index past the end) stops
    /// the pass with the edits recorded before it already written — the same place the equivalent
    /// column of `set_shape_*` calls would stop, since that is what a cursor is. Nothing is rolled
    /// back; a relationship added for an edit that never landed is swept rather than left orphaned.
    ///
    /// Addresses cannot fail here: the cursor checked each one as it moved onto it, held the deck
    /// exclusively while recording, and no edit it records adds or removes a shape.
    ///
    /// # Errors
    /// Returns whatever the mirrored flat method would: a malformed part, a shape whose kind cannot
    /// take the edit, an index within the shape out of range, or a failed package edit.
    pub fn apply(self) -> Result<(), PptxError> {
        let Self {
            deck,
            surface,
            edits,
            ..
        } = self;
        deck.apply_shape_edits(surface, edits)
    }
}
