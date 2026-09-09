//! **The string a paragraph is actually laid out from** — the document's own text with generated
//! content spliced in, deleted content dropped, and a map back to the document's offsets.
//!
//! # Why a second string exists at all
//!
//! Until MJXOFF-177 this crate laid out `mjx_docx::ParagraphFormatting::text` directly, and four
//! things a reader sees were therefore not on the line:
//!
//! | what | what the document holds | what a reader sees |
//! |---|---|---|
//! | a footnote reference | nothing — [`mjx_docx::NoteReference`] is a position | a superscript `1` |
//! | a list's number | nothing — `w:numPr` is a reference | `2.1.3` and a tab |
//! | a field | the value Word last computed | the value **now** |
//! | an inline picture or equation | nothing — a `w:drawing` is a position | a box of a known width |
//!
//! and one thing a reader may or may not see depending on a setting: a tracked deletion.
//!
//! Every one of those changes the **width of a line**, so every one changes where the line breaks,
//! where the page breaks, and which page every later paragraph lands on. They are not decoration
//! that a later stage can add; they have to be in the measure. So the paragraph is *composed* first
//! and laid out second, and this module is the composition.
//!
//! # The offset map, and why it is not optional
//!
//! Every [`mjx_layout::SourceRef`] this crate produces carries a **character range into the
//! document**, because a hit test that answered "layout offset 47" would name a position in a string
//! no part of the file contains — and a selection built from it would move when a field's value
//! changed length. So a [`Composition`] keeps [`Piece`]s: contiguous stretches of the layout string,
//! each either a range of the document's own text or **generated**, and generated pieces map to the
//! empty range at their anchor. That is the same answer [`crate::model`] already gives for a
//! hyphen, a tab leader and a line number — a glyph the document does not contain takes the address
//! of the position it sits at.
//!
//! # What an inline object is on the line
//!
//! One `U+FFFC OBJECT REPLACEMENT CHARACTER`, in a run of its own with a fixed
//! [`mjx_layout::TextRun::advance`]. UAX #14 gives `U+FFFC` line-break class `CB`, which is the class
//! that exists for exactly this — an object whose breaking behaviour is the *embedder's*, so the
//! breaker treats it as an opaque unit and lets the surrounding context decide. One character means
//! the run is all-or-nothing, which is what an object with no interior needs.
//!
//! **This is what closes MJXOFF-175's and MJXOFF-176's declared gaps**, and both are closed by the
//! same mechanism: a footnote mark became real text, and an inline drawing became a real advance.

use std::ops::Range;

use mjx_docx::{ParagraphFormatting, RevisionKind};
use mjx_ooxml_core::measure::Emu;

use crate::lists::Marker;
use crate::math::MathBox;
use crate::revision::RevisionView;
use crate::style::{ParagraphStyle, RunStyle};

/// `U+FFFC OBJECT REPLACEMENT CHARACTER` — one character standing for one inline object.
pub const OBJECT_REPLACEMENT: char = '\u{FFFC}';

/// Where a stretch of the layout string came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PieceKind {
    /// The document's own text.
    Document,
    /// A field's computed value, replacing its cached result.
    FieldResult,
    /// A list's marker and its suffix.
    ListMarker,
    /// A footnote or endnote reference mark.
    NoteMark,
    /// The single character standing for an inline drawing or an equation.
    InlineObject,
}

/// What a piece's tracked-change state is, for whatever draws it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PieceRevision {
    /// Which container it sits in.
    pub kind: RevisionKind,
    /// `w:author`.
    pub author: Option<String>,
}

/// One contiguous stretch of the layout string.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Piece {
    /// Its bytes in [`Composition::text`].
    pub layout: Range<usize>,
    /// Its bytes in the document's own paragraph text. **Empty** for generated content, at the
    /// document offset it is anchored to.
    pub document: Range<usize>,
    /// What it is.
    pub kind: PieceKind,
    /// The innermost tracked change covering it, if any.
    pub revision: Option<PieceRevision>,
}

/// What an inline object is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InlineObjectKind {
    /// A `w:drawing` with `wp:inline` — index into
    /// [`mjx_docx::ParagraphFormatting::drawings`].
    Drawing(usize),
    /// An `m:oMath` — index into [`mjx_docx::ParagraphFormatting::equations`].
    Equation(usize),
}

/// One atomic box on a line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InlineObject {
    /// The byte of [`Composition::text`] its `U+FFFC` sits at.
    pub at: usize,
    /// What it is.
    pub kind: InlineObjectKind,
    /// Its advance — the width the line composer reserves for it.
    pub width: Emu,
    /// How far above the baseline it reaches.
    pub ascent: Emu,
    /// How far below.
    pub descent: Emu,
}

/// A paragraph, composed: the string it is laid out from, the runs over it, and the map back.
#[derive(Clone, PartialEq, Debug)]
pub struct Composition {
    text: String,
    runs: Vec<RunStyle>,
    pieces: Vec<Piece>,
    objects: Vec<InlineObject>,
    equations: Vec<MathBox>,
    style: ParagraphStyle,
    change_bar: bool,
}

impl Composition {
    /// The paragraph's own resolved style — carried here so that one value reaches the flow engine
    /// rather than a composition and a paragraph that could disagree about which paragraph they are.
    #[must_use]
    pub fn style(&self) -> &ParagraphStyle {
        &self.style
    }

    /// The string that is measured, broken and shaped.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The runs over it, in order, covering it without gaps.
    #[must_use]
    pub fn runs(&self) -> &[RunStyle] {
        &self.runs
    }

    /// Its pieces, in order.
    #[must_use]
    pub fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    /// The inline objects on it, in order.
    #[must_use]
    pub fn objects(&self) -> &[InlineObject] {
        &self.objects
    }

    /// The equations on it, laid out, in the order [`InlineObjectKind::Equation`] indexes them.
    ///
    /// **Kept rather than re-derived at emission time.** An equation's box tree is what a painter
    /// draws and what the line reserved width for; laying it out twice would mean two trees free to
    /// disagree, which is the same failure `crate::flow` avoids by carrying the composition.
    #[must_use]
    pub fn equations(&self) -> &[MathBox] {
        &self.equations
    }

    /// Whether a change bar belongs beside this paragraph — the paragraph carries a tracked content
    /// change and the view draws bars.
    #[must_use]
    pub fn change_bar(&self) -> bool {
        self.change_bar
    }

    /// The object whose `U+FFFC` is at layout offset `at`, if there is one.
    #[must_use]
    pub fn object_at(&self, at: usize) -> Option<&InlineObject> {
        self.objects.iter().find(|object| object.at == at)
    }

    /// The document range a layout range names.
    ///
    /// The smallest document range covering every *document* piece the layout range touches. A
    /// layout range made entirely of generated content answers the empty range at its anchor, which
    /// is what puts a caret beside a field's value rather than inside it.
    #[must_use]
    pub fn document_range(&self, layout: &Range<usize>) -> Range<usize> {
        let mut start: Option<usize> = None;
        let mut end: Option<usize> = None;
        let mut anchor = 0_usize;
        for piece in &self.pieces {
            if piece.layout.end <= layout.start {
                anchor = piece.document.end;
                continue;
            }
            if piece.layout.start >= layout.end {
                break;
            }
            if piece.kind == PieceKind::Document {
                // Clip to the part of this piece the range actually touches, so that a line ending
                // in the middle of a run names the bytes on the line and not the whole run.
                let offset = piece.document.start;
                let from = offset + layout.start.saturating_sub(piece.layout.start);
                let to = offset
                    + layout
                        .end
                        .min(piece.layout.end)
                        .saturating_sub(piece.layout.start);
                start = Some(start.map_or(from, |existing: usize| existing.min(from)));
                end = Some(end.map_or(to, |existing: usize| existing.max(to)));
            } else {
                anchor = piece.document.start;
            }
        }
        match (start, end) {
            (Some(start), Some(end)) => start..end.max(start),
            _ => anchor..anchor,
        }
    }

    /// A composition holding one paragraph's own text and nothing generated.
    ///
    /// What every caller had before MJXOFF-177, and what a paragraph with no field, no marker, no
    /// note reference, no equation, no inline object **and no tracked change** still gets: the map
    /// is the identity and nothing is spliced or dropped.
    ///
    /// # It shows every revision, and that is why it is only for a paragraph that has none
    ///
    /// "Plain" means *the document's own text*, so this hides nothing — and a paragraph with a
    /// tracked deletion in it therefore must **not** come through here, because which text such a
    /// paragraph is laid out from is a question about the view. [`compose`] is the entry point that
    /// takes one. The first version of this function defaulted the view instead, which read the
    /// document in Word's *Simple Markup* and hid every deletion from a caller that had asked for
    /// *All Markup* — the same document, laid out from a different string, with no error anywhere.
    #[must_use]
    pub fn plain(paragraph: &ParagraphFormatting) -> Self {
        let mut builder = Builder::new(paragraph);
        builder.view = RevisionView::AllMarkup;
        builder.finish()
    }
}

/// The generated content one paragraph gets.
///
/// A struct of four optional inputs rather than four arguments, because the builder's own order —
/// marker, then the document's text with its edits applied — is the same whichever are present, and
/// a caller supplying none gets [`Composition::plain`].
#[derive(Default, Debug)]
pub struct Generated<'a> {
    /// The list marker, when the paragraph is in a list.
    pub marker: Option<&'a Marker>,
    /// What each field renders, by field index — `None` keeps the cached result that is already in
    /// the text.
    pub field_values: &'a [Option<String>],
    /// The reference mark for each of the paragraph's note references, in order.
    pub note_marks: &'a [String],
    /// Each of the paragraph's equations, laid out, in order.
    pub equations: &'a [MathBox],
    /// Which review view the document is being laid out in.
    pub view: RevisionView,
}

/// Composes one paragraph.
#[must_use]
pub fn compose(paragraph: &ParagraphFormatting, generated: &Generated<'_>) -> Composition {
    let mut builder = Builder::new(paragraph);
    builder.view = generated.view;
    builder.marker = generated.marker;
    builder.field_values = generated.field_values;
    builder.note_marks = generated.note_marks;
    builder.equations = generated.equations;
    builder.finish()
}

/// How much smaller a footnote reference mark is set than the run it sits in.
///
/// **`GUESS:`** 65 %, which is the ratio Word's own `Footnote Reference` character style produces
/// through `w:vertAlign="superscript"` in the faces this repository ships. ECMA-376 §17.3.2.42
/// defines `superscript` as *"raised above the baseline and smaller"* and states neither number, so
/// this is a reading — and it is a reading that changes a **width**, which is why it is here and
/// marked rather than left to a painter.
pub const SUPERSCRIPT_SCALE: f64 = 0.65;

/// One composition, in progress.
struct Builder<'a> {
    paragraph: &'a ParagraphFormatting,
    view: RevisionView,
    marker: Option<&'a Marker>,
    field_values: &'a [Option<String>],
    note_marks: &'a [String],
    equations: &'a [MathBox],
    text: String,
    runs: Vec<RunStyle>,
    pieces: Vec<Piece>,
    objects: Vec<InlineObject>,
}

/// One thing that happens at a document offset while the text is walked.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Event {
    /// A field's result runs from here to `end` and renders `value` instead.
    Field { end: usize, value: String },
    /// A note reference mark.
    NoteMark(String),
    /// An inline drawing.
    Drawing(usize, Emu, Emu),
    /// An equation.
    Equation(usize, Emu, Emu, Emu),
    /// A tracked-change span this view does not show runs from here to `end`.
    Hidden { end: usize },
}

impl<'a> Builder<'a> {
    fn new(paragraph: &'a ParagraphFormatting) -> Self {
        Self {
            paragraph,
            view: RevisionView::default(),
            marker: None,
            field_values: &[],
            note_marks: &[],
            equations: &[],
            text: String::new(),
            runs: Vec::new(),
            pieces: Vec::new(),
            objects: Vec::new(),
        }
    }

    /// The run style at document offset `at`, or the paragraph's first run's.
    fn style_at(&self, at: usize) -> Option<RunStyle> {
        let runs = self.paragraph.runs();
        runs.iter()
            .find(|run| run.range.start <= at && at < run.range.end)
            .or_else(|| runs.iter().find(|run| run.range.end == at))
            .or_else(|| runs.first())
            .map(|run| RunStyle::of(run.range.clone(), &run.properties))
    }

    /// Appends generated text, styled like the run at `anchor`.
    fn push_generated(&mut self, text: &str, anchor: usize, kind: PieceKind, scale: f64) {
        if text.is_empty() {
            return;
        }
        let Some(mut style) = self.style_at(anchor) else {
            return;
        };
        let start = self.text.len();
        self.text.push_str(text);
        style.range = start..self.text.len();
        if (scale - 1.0).abs() > f64::EPSILON {
            style.size = mjx_text::FontSize::from_points(style.size.in_points() * scale);
        }
        // Generated content is never hidden: `w:vanish` belongs to the run the document wrote, and a
        // list's number is not that run. A marker inherited from a hidden run would be a list that
        // silently loses its numbers.
        style.hidden = false;
        self.runs.push(style);
        self.pieces.push(Piece {
            layout: start..self.text.len(),
            document: anchor..anchor,
            kind,
            revision: self.revision_at(anchor),
        });
    }

    /// Appends a stretch of the document's own text.
    fn push_document(&mut self, range: Range<usize>) {
        let Some(slice) = self.paragraph.text().get(range.clone()) else {
            return;
        };
        if slice.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(slice);
        self.pieces.push(Piece {
            layout: start..self.text.len(),
            document: range.clone(),
            kind: PieceKind::Document,
            revision: self.revision_at(range.start),
        });
        // One layout run per document run the stretch overlaps, clipped — so a stretch that spans a
        // formatting boundary keeps both formats rather than taking the first one's.
        for run in self.paragraph.runs() {
            let from = run.range.start.max(range.start);
            let to = run.range.end.min(range.end);
            if from >= to {
                continue;
            }
            let mut style = RunStyle::of(run.range.clone(), &run.properties);
            style.range = (start + (from - range.start))..(start + (to - range.start));
            self.runs.push(style);
        }
    }

    /// Appends one inline object: its `U+FFFC`, its run and its entry.
    fn push_object(
        &mut self,
        at: usize,
        kind: InlineObjectKind,
        width: Emu,
        ascent: Emu,
        descent: Emu,
    ) {
        let Some(mut style) = self.style_at(at) else {
            return;
        };
        let start = self.text.len();
        self.text.push(OBJECT_REPLACEMENT);
        style.range = start..self.text.len();
        style.hidden = false;
        self.runs.push(style);
        self.pieces.push(Piece {
            layout: start..self.text.len(),
            document: at..at,
            kind: PieceKind::InlineObject,
            revision: self.revision_at(at),
        });
        self.objects.push(InlineObject {
            at: start,
            kind,
            width,
            ascent,
            descent,
        });
    }

    /// The innermost tracked change covering document offset `at`.
    fn revision_at(&self, at: usize) -> Option<PieceRevision> {
        crate::revision::covering(self.paragraph.revisions(), at).map(|span| PieceRevision {
            kind: span.kind,
            author: span.author.clone(),
        })
    }

    /// The events this paragraph's generated content produces, sorted by document offset.
    fn events(&self) -> Vec<(usize, Event)> {
        let mut events: Vec<(usize, Event)> = Vec::new();
        for (index, field) in self.paragraph.fields().iter().enumerate() {
            let Some(Some(value)) = self.field_values.get(index) else {
                continue;
            };
            events.push((
                field.result.start,
                Event::Field {
                    end: field.result.end,
                    value: value.clone(),
                },
            ));
        }
        for (index, reference) in self.paragraph.note_references().iter().enumerate() {
            let Some(mark) = self.note_marks.get(index) else {
                continue;
            };
            events.push((reference.at, Event::NoteMark(mark.clone())));
        }
        for (index, drawing) in self.paragraph.drawings().iter().enumerate() {
            if !matches!(drawing.placement, mjx_docx::DrawingPlacement::Inline(_)) {
                continue;
            }
            events.push((
                drawing.at,
                Event::Drawing(
                    index,
                    Emu::from_emu(drawing.width),
                    Emu::from_emu(drawing.height),
                ),
            ));
        }
        for (index, equation) in self.paragraph.equations().iter().enumerate() {
            let Some(laid) = self.equations.get(index) else {
                continue;
            };
            events.push((
                equation.at,
                Event::Equation(index, laid.width, laid.ascent, laid.descent),
            ));
        }
        for span in self.paragraph.revisions() {
            if span.range.is_empty() || self.view.shows(span.kind) {
                continue;
            }
            events.push((
                span.range.start,
                Event::Hidden {
                    end: span.range.end,
                },
            ));
        }
        // Stable by offset, and by the order above within one offset — a note mark at the same byte
        // as a field's start belongs before the field's value, which is where the run stream puts it.
        events.sort_by_key(|(at, _)| *at);
        events
    }

    fn finish(mut self) -> Composition {
        let change_bar = self.view.shows_change_bars()
            && crate::revision::has_content_change(self.paragraph.revisions());
        // The marker comes first, before any of the paragraph's own text, and takes the style of the
        // paragraph's first run — which is where `mjx-docx`'s numbering tier has already put the
        // level's own `w:rPr`.
        if let Some(marker) = self.marker {
            let text = marker.with_suffix();
            self.push_generated(&text, 0, PieceKind::ListMarker, 1.0);
        }
        let events = self.events();
        let length = self.paragraph.text().len();
        let mut at = 0_usize;
        for (offset, event) in events {
            if offset < at {
                // An event inside a stretch already consumed — a note reference inside a field's
                // cached result whose value replaced it, or two overlapping revision spans. Skipped
                // rather than reordered: it belongs to text that is not on the page.
                continue;
            }
            self.push_document(at..offset);
            at = offset;
            match event {
                Event::Field { end, value } => {
                    self.push_generated(&value, offset, PieceKind::FieldResult, 1.0);
                    at = end.max(offset);
                }
                Event::NoteMark(mark) => {
                    self.push_generated(&mark, offset, PieceKind::NoteMark, SUPERSCRIPT_SCALE);
                }
                Event::Drawing(index, width, height) => {
                    self.push_object(
                        offset,
                        InlineObjectKind::Drawing(index),
                        width,
                        height,
                        Emu::ZERO,
                    );
                }
                Event::Equation(index, width, ascent, descent) => {
                    self.push_object(
                        offset,
                        InlineObjectKind::Equation(index),
                        width,
                        ascent,
                        descent,
                    );
                }
                Event::Hidden { end } => at = end.max(offset),
            }
        }
        self.push_document(at..length);
        self.runs.sort_by_key(|run| run.range.start);
        Composition {
            text: self.text,
            runs: self.runs,
            pieces: self.pieces,
            objects: self.objects,
            equations: self.equations.to_vec(),
            style: ParagraphStyle::of(self.paragraph.properties()),
            change_bar,
        }
    }
}
