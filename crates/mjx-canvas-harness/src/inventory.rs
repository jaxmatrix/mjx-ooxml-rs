//! The sixty-one in-canvas elements, by number — **the specification, transcribed**.
//!
//! `docs/client-platform/CANVAS_UI_INVENTORY.md` §2 is the list; this module is that list as data,
//! in the same order, with the same numbers and the same titles. The document says it plainly:
//! *"an element that has no harness scene and no plate is not covered, and the harness fails its own
//! completeness check — the inventory is the test."*
//!
//! # ⚠ The trap, and the four counters that answer it
//!
//! *"All 61 elements have a scene"* is satisfied perfectly by sixty-one titled empty canvases. So
//! completeness is asserted about **drawing**, not about registration, and
//! `tests/every_entry_draws.rs` holds every entry to four numbers at once:
//!
//! | Counter | Refuses |
//! |---|---|
//! | `placeholders == 0` | a stand-in outline standing in for the element |
//! | `draw_calls > 0` | a scene that registered and drew nothing |
//! | `covered > `[`MINIMUM_COVERED`] | a scene whose draws all fell outside the frame or were transparent |
//! | `distinct_colors >= `[`MINIMUM_COLOURS`] | **a scene drawn entirely in one ink** |
//!
//! The fourth is R10's own hand-off, and it is the one that matters most here. MJXOFF-165's nested
//! specimen drew its innermost box in the same colour as the group containing it: all three of the
//! other counters were healthy and the box was *invisible*, so a change to it moved no pixel. For
//! sixty-one small overlays drawn on top of a page that is the default failure mode rather than an
//! edge case — an overlay in the page's own colour passes every counter but the last.
//!
//! And [`Draws`] is the fifth: an entry says which *kinds* of command its scene must contain, so a
//! scene that drew the page and forgot the element itself fails naming the entry number.

use mjx_render_oracle::authority::RenderedContent;

use crate::state::Axis;

/// The number of elements the inventory defines. Not a number this crate chose.
pub const ELEMENTS: usize = 61;

/// How many non-transparent pixels a scene must reach before it counts as having drawn.
///
/// The page alone covers most of the frame, so this floor is not what catches an empty scene —
/// [`Draws`] is. It catches a scene that drew *nothing at all*, including its own page, which is
/// what a builder that returned early would produce.
pub const MINIMUM_COVERED: usize = 500;

/// How many distinct colours a scene must contain.
///
/// Four: the backdrop, the page, and at least two more, which is the fewest that can distinguish
/// *"the element is drawn"* from *"the element is drawn in the page's own ink"*. Anti-aliasing
/// inflates this number generously on any real scene; the floor is deliberately low so that it fails
/// only for the reason it is written for.
pub const MINIMUM_COLOURS: usize = 4;

/// Which of the six families an element belongs to.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Family {
    /// §2.1 — eleven elements.
    ObjectSelection,
    /// §2.2 — ten elements.
    TextEditing,
    /// §2.3 — nine elements.
    GridSelection,
    /// §2.4 — thirteen elements.
    Manipulation,
    /// §2.5 — thirteen elements.
    Furniture,
    /// §2.6 — five elements.
    StateAndFeedback,
}

impl Family {
    /// Every family, in the inventory's own order.
    pub const ALL: [Self; 6] = [
        Self::ObjectSelection,
        Self::TextEditing,
        Self::GridSelection,
        Self::Manipulation,
        Self::Furniture,
        Self::StateAndFeedback,
    ];

    /// How the family is named in the harness and in the checklist.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ObjectSelection => "Object selection",
            Self::TextEditing => "Text selection and editing",
            Self::GridSelection => "Grid selection — Excel",
            Self::Manipulation => "Manipulation",
            Self::Furniture => "Document furniture",
            Self::StateAndFeedback => "State and feedback",
        }
    }

    /// The section of `CANVAS_UI_INVENTORY.md` that defines it.
    #[must_use]
    pub const fn section(self) -> &'static str {
        match self {
            Self::ObjectSelection => "2.1",
            Self::TextEditing => "2.2",
            Self::GridSelection => "2.3",
            Self::Manipulation => "2.4",
            Self::Furniture => "2.5",
            Self::StateAndFeedback => "2.6",
        }
    }

    /// How many elements the inventory puts in it.
    #[must_use]
    pub const fn size(self) -> usize {
        match self {
            Self::ObjectSelection => 11,
            Self::TextEditing => 10,
            Self::GridSelection => 9,
            Self::Manipulation | Self::Furniture => 13,
            Self::StateAndFeedback => 5,
        }
    }

    /// The family's stable spelling, for a URL and a filter.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::ObjectSelection => "object-selection",
            Self::TextEditing => "text-editing",
            Self::GridSelection => "grid-selection",
            Self::Manipulation => "manipulation",
            Self::Furniture => "furniture",
            Self::StateAndFeedback => "state-and-feedback",
        }
    }
}

/// A kind of drawing an element's scene must contain.
///
/// **Not a description of the scene — a gate on it.** A `FillPath` and a `StrokePath` are different
/// halves of the painter and different halves of the display list, and an element specified as an
/// outline that is implemented as a filled rectangle is a different element. Checked against the
/// display list rather than against the image, so the failure names a missing command rather than a
/// missing pixel.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Draws {
    /// At least one `FillPath`.
    Fill,
    /// At least one `StrokePath` — the element has an outline of its own.
    Stroke,
    /// A `PushClip` / `Pop` pair — the element is clipped to something.
    Clip,
    /// A `PushOpacity` / `Pop` pair — the element is translucent over what it covers.
    Translucency,
    /// A `PushEffect` / `Pop` pair — the element carries a blur, a shadow or a glow.
    Effect,
    /// A `PushTransform` / `Pop` pair — the element is rotated or skewed.
    Transform,
}

impl Draws {
    /// Every kind, so a sweep cannot miss one.
    pub const ALL: [Self; 6] = [
        Self::Fill,
        Self::Stroke,
        Self::Clip,
        Self::Translucency,
        Self::Effect,
        Self::Transform,
    ];

    /// How the kind is named in a failure.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fill => "a filled path",
            Self::Stroke => "a stroked path",
            Self::Clip => "a clip",
            Self::Translucency => "a translucent group",
            Self::Effect => "an effect",
            Self::Transform => "a transform",
        }
    }
}

/// One element of the inventory.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Entry {
    /// Its number in `CANVAS_UI_INVENTORY.md` §2, from 1 to 61. **The identity of the element**: the
    /// checklist, the plate name, the harness URL and every failure message use it.
    pub number: u8,
    /// The family it belongs to.
    pub family: Family,
    /// Its title, verbatim from the inventory.
    pub title: &'static str,
    /// The stable half of [`Entry::slug`] — a short, hyphenated name for the element.
    ///
    /// Separate from [`Entry::title`] on purpose: a title is prose and may be reworded, and a plate
    /// name may not change without expiring the approval bound to it.
    pub stem: &'static str,
    /// What the harness's scene shows, and what a person auditing it is being asked to judge.
    pub description: &'static str,
    /// What kind of drawing the plate is — **the axis a provider exclusion is declared along**. A
    /// gradient overlay cannot be compared against a LibreOffice reference and a stroked outline
    /// can, and this is the field that says which.
    pub content: RenderedContent,
    /// The command kinds the scene must contain. See [`Draws`].
    pub draws: &'static [Draws],
    /// **Which axes of the state matrix move this element's pixels.**
    ///
    /// Declared here and measured by `tests/the_axes_are_not_identities.rs` in both directions. An
    /// entry that claims to respond to an axis and does not is a toggle that does nothing; one that
    /// responds without saying so is a state nobody was told to audit.
    ///
    /// [`Axis::Scheme`] and [`Axis::Density`] are on every entry by construction — every scene sits
    /// on a themed backdrop and every scene is rasterised at the density in force — so the two
    /// carry their own gates rather than being informative here. [`Axis::Interaction`] and
    /// [`Axis::Input`] are the ones that say something about *this* element.
    ///
    /// # ⚠ Six of these declarations were corrected *from* the measurement, and two say so
    ///
    /// `CHANGELOG.md` (0.0.140) records that the axes suite *"found six disagreements on its first
    /// run"*. Entries 9 and 10 carry a note above their own `responds` line saying why the
    /// measurement was right; **the other four are not recoverable from this tree**, because the
    /// whole crate landed in one commit (`d5a2112`) and the corrections were made before it.
    ///
    /// So read every line here with that in mind. A declaration written from a measurement is
    /// indistinguishable from an independently-authored one, and it **cannot detect that the
    /// measurement was wrong to begin with**: if a scene responds to the interaction axis by
    /// accident, copying that fact into the declaration makes the gate green and the accident
    /// permanent. What the gate *is* strong at is the thing it was built for — a declaration that
    /// stops being true, in either direction, from that point on.
    ///
    /// The one honest way to close the remainder is a person auditing the element and saying what
    /// it *should* respond to, which is `docs/client-platform/CANVAS_UI_AUDIT.md` step 2 and is not
    /// an agent's to do. Audit pass 10 wrote this paragraph rather than guessing at four entries.
    pub responds: &'static [Axis],
}

impl Entry {
    /// The entry's stable, filesystem- and URL-safe name.
    ///
    /// It is the baseline directory's name and the plate's name in the manifest, so it may not
    /// change without expiring an approval — which is why it is derived from the number and a fixed
    /// slug rather than from the title.
    #[must_use]
    pub fn slug(&self) -> String {
        format!("{:02}-{}", self.number, self.stem)
    }

    /// Whether this element's pixels change with the input device — the `touch` badge in the scene
    /// list and `"touch": true` in `/api/inventory`.
    ///
    /// # ⚠ Nineteen, not eleven, and the difference is a distinction rather than a drift
    ///
    /// `CANVAS_UI_INVENTORY.md` §4.1 says *"eleven inventory entries are specifically about touch
    /// behaviour"*, and this answers `true` for **nineteen**. The two count different things:
    ///
    /// * the document's eleven are the entries a person has to open **on a phone** to judge at all
    ///   — a pinch gesture, a long-press, a two-finger rotate;
    /// * [`Axis::Input`] is the wider property *this element is drawn at the input device's size*,
    ///   which is true of every affordance that has a grab region, including eight that a mouse can
    ///   judge perfectly well and that simply get bigger for a thumb.
    ///
    /// The badge is deliberately the wider set: it means *turn on the hit-test overlay and look at
    /// this at both input settings*, which is worth doing for all nineteen. What it promises is that
    /// there is something to look at, and `tests/the_visualiser_shows_the_index.rs` holds that
    /// promise — audit pass 10 added the assertion and it found entry 30 badged with no grab region
    /// at all.
    #[must_use]
    pub fn is_touch_sensitive(&self) -> bool {
        self.responds.contains(&Axis::Input)
    }
}

/// A shorthand for the four axes an element can respond to, so the table below reads as a table.
mod axes {
    use crate::state::Axis;

    /// Theme and density only — an element with no interactive state of its own.
    pub(super) const STATIC: &[Axis] = &[Axis::Scheme, Axis::Density];
    /// Theme, density and the pointer's state.
    pub(super) const INTERACTIVE: &[Axis] = &[Axis::Scheme, Axis::Density, Axis::Interaction];
    /// All four — an element whose affordance is sized for the input device.
    pub(super) const TOUCHABLE: &[Axis] =
        &[Axis::Scheme, Axis::Density, Axis::Interaction, Axis::Input];
    /// Theme, density and the input device, with no interaction state — an element that is sized
    /// for a finger but has no hover of its own.
    pub(super) const SIZED: &[Axis] = &[Axis::Scheme, Axis::Density, Axis::Input];
}

/// **The inventory.** Sixty-one entries, in `CANVAS_UI_INVENTORY.md` §2's own order and with its own
/// numbers, so that a reader holding the document beside this table can check them off one by one.
///
/// The order is load-bearing in one place — [`Entry::number`] must equal the index plus one, and
/// `tests/every_entry_draws.rs` asserts it — because every artefact this crate produces is addressed
/// by number, and a table that had drifted from the document would produce a checklist nobody could
/// use.
pub const INVENTORY: [Entry; ELEMENTS] = [
    // ---------------------------------------------------------------------------------------
    // §2.1 · Object selection (11)
    // ---------------------------------------------------------------------------------------
    Entry {
        number: 1,
        family: Family::ObjectSelection,
        title: "Selection outline — single object",
        stem: "selection-outline-single",
        description: "One object on a page with the selection outline around it. Judge the \
                      outline's weight against the object's own edge, and whether it reads as \
                      selection rather than as part of the drawing.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 2,
        family: Family::ObjectSelection,
        title: "Selection outline — multiple objects, with the union bounds",
        stem: "selection-outline-multiple",
        description: "Three objects selected together: each carries its own outline and the union \
                      bounds enclose all three. Judge whether the two weights are distinguishable \
                      without being noisy.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 3,
        family: Family::ObjectSelection,
        title: "Resize handles — four corner, four edge; hover and active states",
        stem: "resize-handles",
        description: "The eight handles around a selected object. Hover and active change the \
                      east handle only, so a state toggle that changes every handle is visibly \
                      wrong. Turn on the hit-test overlay and judge the grab regions.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 4,
        family: Family::ObjectSelection,
        title: "Rotation handle, its tether, and the live angle readout",
        stem: "rotation-handle",
        description: "The rotation handle above the object, the tether joining it to the top edge, \
                      and the readout that appears while it is turned. The object is rotated in the \
                      active state, so the tether has to follow it.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke, Draws::Transform],
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 5,
        family: Family::ObjectSelection,
        title: "Rotation snap indicator (15° increments, and the shift-constraint state)",
        stem: "rotation-snap",
        description: "The dial of fifteen-degree marks that appears while a rotation is snapping, \
                      with the engaged mark called out. The active state is the shift constraint.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke, Draws::Transform],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 6,
        family: Family::ObjectSelection,
        title: "Shape adjustment handles (the `avLst` control points)",
        stem: "adjustment-handles",
        description: "A rounded rectangle and an arrow with their adjustment handles, in the \
                      diamond form that distinguishes an adjustment from a resize handle. Judge \
                      whether the two are tellable apart at a glance.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 7,
        family: Family::ObjectSelection,
        title: "Connection sites — shown while a connector is being drawn",
        stem: "connection-sites",
        description: "The connection sites on two shapes, shown because a connector is being drawn \
                      from one to the other. The site under the pointer is the engaged one.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 8,
        family: Family::ObjectSelection,
        title: "Connector endpoints, hover targets, and the reroute preview",
        stem: "connector-endpoints",
        description: "An elbow connector between two shapes with both endpoints shown, and the \
                      dashed preview of the route it would take if the drag were released here.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 9,
        family: Family::ObjectSelection,
        title: "Group selection outline versus a selected child inside a group",
        stem: "group-versus-child",
        description: "A group of three children: the group's own outline, and one child selected \
                      inside it with a different one. Judge whether it is obvious which of the two \
                      a drag would move.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke, Draws::Clip],
        // Touch-sized, and the measurement said so before this line did: the selected child carries
        // its own eight handles, and a handle is drawn at the input device's size.
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 10,
        family: Family::ObjectSelection,
        title: "Locked-object indicator",
        stem: "locked-object",
        description: "A locked object beside an unlocked one, both selected. The disabled state is \
                      the one that matters: a locked object shows its handles as inert rather than \
                      hiding them.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke],
        // The inert handles are sized from the input device too. An affordance that is *shown* as
        // unusable still has to be the size the reader would have reached for, or the two objects
        // are not comparable.
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 11,
        family: Family::ObjectSelection,
        title: "Off-canvas / partially-off-slide indicator",
        stem: "off-canvas-indicator",
        description: "An object hanging over the page edge, with the off-slide part dimmed and an \
                      edge marker on the page boundary. The clip is the point: the object is drawn \
                      once and the page edge cuts it.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke, Draws::Clip, Draws::Translucency],
        responds: axes::INTERACTIVE,
    },
    // ---------------------------------------------------------------------------------------
    // §2.2 · Text selection and editing (10)
    // ---------------------------------------------------------------------------------------
    Entry {
        number: 12,
        family: Family::TextEditing,
        title: "Caret — blink cadence, hairline thickness across DPI",
        stem: "caret-hairline",
        description: "The caret in a line of text, repeated at four widths down the page. Judge \
                      the hairline at 1×, 2× and 3×: this is the element the density toggle exists \
                      for. The blink is the harness's own animation, never the plate's.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 13,
        family: Family::TextEditing,
        title: "Caret — bidi split form, and the direction indicator at a script boundary",
        stem: "caret-bidi",
        description: "A line with a right-to-left run inside it: the split caret at the boundary, \
                      with the flag on the side the next character will be inserted on.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 14,
        family: Family::TextEditing,
        title: "Selection fill — single line, multi-line, ragged trailing edge",
        stem: "selection-fill-runs",
        description: "Three selections at once: one within a line, one across three lines with a \
                      ragged last line, and one that ends mid-word. Judge the fill's alpha over \
                      the text underneath it.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Translucency],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 15,
        family: Family::TextEditing,
        title: "Selection fill — across columns, across pages, across a table's cells",
        stem: "selection-fill-across",
        description: "One selection crossing a column boundary, a page gap and a table's cell \
                      edges. Judge whether the fill stays one selection to the eye.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Translucency],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 16,
        family: Family::TextEditing,
        title: "IME composition underline and the candidate-window anchor rectangle",
        stem: "ime-composition",
        description: "A composition run with its two underline weights — the clause being \
                      converted is heavier — and the rectangle the candidate window anchors to.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 17,
        family: Family::TextEditing,
        title: "Spelling, grammar and style squiggles (three weights and colours)",
        stem: "squiggles",
        description: "The three squiggles on three lines, at their real amplitude and period. The \
                      element most likely to be illegible at 1× and lost at 3×.",
        content: RenderedContent::Outline,
        draws: &[Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 18,
        family: Family::TextEditing,
        title: "Hyperlink hover affordance",
        stem: "hyperlink-hover",
        description: "A hyperlink run at rest and under the pointer, with the underline and the \
                      tooltip anchor. The interaction toggle is the whole element.",
        content: RenderedContent::SolidFill,
        // Fill alone, and the absence of `Stroke` is measured rather than assumed. A text underline
        // is a filled rectangle at the metrics the line gives it — not a stroked path — and the one
        // stroke this scene has, the halo around the hovered run, exists only in the engaged states
        // while a plate is taken at rest. The declaration describes the plate.
        draws: &[Draws::Fill],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 19,
        family: Family::TextEditing,
        title: "Text-overflow and autofit indicators inside a shape",
        stem: "text-overflow",
        description: "A shape whose text does not fit: the overflow marker at the bottom edge and \
                      the autofit badge that says the text has been shrunk rather than clipped.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke, Draws::Clip],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 20,
        family: Family::TextEditing,
        title: "Placeholder prompt text (empty placeholder on a slide)",
        stem: "placeholder-prompt",
        description: "Two empty placeholders with their dashed boundary and their prompt lines. \
                      The focused state is what a click produces.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 21,
        family: Family::TextEditing,
        title: "Drag-to-select autoscroll edge indicator",
        stem: "autoscroll-edge",
        description: "The gradient band at the page edge that appears while a drag-selection is \
                      held there, with the direction arrow. A gradient, so a LibreOffice reference \
                      is excluded from it by the provider rather than by this list.",
        content: RenderedContent::GradientFill,
        draws: &[Draws::Fill],
        responds: axes::INTERACTIVE,
    },
    // ---------------------------------------------------------------------------------------
    // §2.3 · Grid selection — Excel (9)
    // ---------------------------------------------------------------------------------------
    Entry {
        number: 22,
        family: Family::GridSelection,
        title: "Active-cell border, distinct from the range border",
        stem: "active-cell-border",
        description: "A range with one active cell inside it. Judge whether the active cell's \
                      border is distinguishable from the range's own without being louder.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 23,
        family: Family::GridSelection,
        title: "Range selection fill and border",
        stem: "range-selection",
        description: "A rectangular range with its translucent fill and its border, over gridlines \
                      that must remain visible through it.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke, Draws::Translucency],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 24,
        family: Family::GridSelection,
        title: "Multi-range (discontiguous) selection",
        stem: "multi-range-selection",
        description: "Three discontiguous ranges selected together, two of them adjacent. Judge \
                      whether two touching ranges still read as two.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke, Draws::Translucency],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 25,
        family: Family::GridSelection,
        title: "Fill handle, and the fill preview with its series tooltip",
        stem: "fill-handle",
        description: "The fill handle at the range's bottom-right corner, and — in the active \
                      state — the preview border of the range being filled into, with its tooltip \
                      anchor. The smallest grab target in the whole inventory.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 26,
        family: Family::GridSelection,
        title: "Row and column header highlight for the current selection",
        stem: "header-highlight",
        description: "The row and column headers highlighted for the selected range, with the \
                      selected headers' own separators emphasised.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 27,
        family: Family::GridSelection,
        title: "Frozen and split pane divider lines",
        stem: "pane-dividers",
        description: "A frozen-pane divider and a split-pane divider on the same sheet. They are \
                      different weights and must be tellable apart.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 28,
        family: Family::GridSelection,
        title: "Merged-cell selection behaviour",
        stem: "merged-cell-selection",
        description: "A merged cell inside a selected range: the range border goes round the merge \
                      rather than through it, and the merge has no interior gridlines.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke, Draws::Translucency],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 29,
        family: Family::GridSelection,
        title: "AutoFilter dropdown affordance in a header cell",
        stem: "autofilter-dropdown",
        description: "Two header cells with the dropdown affordance, one filtered and one not — a \
                      filtered column's affordance carries the funnel. Judge the touch target.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 30,
        family: Family::GridSelection,
        title: "Comment / note indicator triangle",
        stem: "comment-indicator",
        description: "The corner triangle on a cell that carries a note, beside a cell that does \
                      not. Four device pixels on a side at 1×, which is where a triangle stops \
                      being one.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill],
        responds: axes::SIZED,
    },
    // ---------------------------------------------------------------------------------------
    // §2.4 · Manipulation (13)
    // ---------------------------------------------------------------------------------------
    Entry {
        number: 31,
        family: Family::Manipulation,
        title: "Drag ghost / translucent preview",
        stem: "drag-ghost",
        description: "An object being dragged: the original in place and the translucent ghost \
                      under the pointer. Judge the ghost's opacity over both a light and a dark \
                      backdrop.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke, Draws::Translucency],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 32,
        family: Family::Manipulation,
        title: "Smart alignment guides — edge, centre, and equal-spacing",
        stem: "alignment-guides",
        description: "Three objects with an edge guide, a centre guide and the equal-spacing marks \
                      between them, all engaged at once. The element the alignment-guide token \
                      exists for.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 33,
        family: Family::Manipulation,
        title: "Snap indicators — to grid, to guide, to object",
        stem: "snap-indicators",
        description: "The three snap kinds on one page, each with its own marker, so that a reader \
                      can tell which of the three has engaged.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 34,
        family: Family::Manipulation,
        title: "Live dimension and position readout during a drag",
        stem: "drag-readout",
        description: "The readout badge that follows a dragged object, with the leader lines to \
                      the page edges it is measured from. Judge whether it obscures what it \
                      measures.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 35,
        family: Family::Manipulation,
        title: "Resize ghost with its dimension readout and aspect-lock state",
        stem: "resize-ghost",
        description: "A resize in progress: the original bounds, the ghost bounds, the readout, \
                      and the aspect-lock indicator on the corner handle being dragged.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke, Draws::Translucency],
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 36,
        family: Family::Manipulation,
        title: "Crop handles and the out-of-crop darkening overlay",
        stem: "crop-handles",
        description: "An image being cropped: the eight L-shaped crop handles, and the darkening \
                      over the part that will be discarded. Judge the darkening's weight — too \
                      light and the crop is invisible, too heavy and the image is.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke, Draws::Translucency],
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 37,
        family: Family::Manipulation,
        title: "Table row and column resize affordance with its preview line",
        stem: "table-resize",
        description: "A table with the pointer on a column boundary: the affordance on the \
                      boundary, and the preview line running the table's full height.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 38,
        family: Family::Manipulation,
        title: "Table insert affordance (the between-rows/columns control)",
        stem: "table-insert",
        description: "The circled plus that appears between two columns, with the insertion line \
                      it previews. A control that only exists *between* two things, which is why \
                      its target is so easy to get wrong.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 39,
        family: Family::Manipulation,
        title: "Custom-geometry vertex editing — vertices, control points, tangent lines",
        stem: "vertex-editing",
        description: "A curve in vertex-edit mode: square vertices, round control points, the \
                      tangent lines joining them, and one vertex selected. Three affordance shapes \
                      at once, which is the judgement.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 40,
        family: Family::Manipulation,
        title: "Motion-path editing handles and the path preview (PowerPoint animation)",
        stem: "motion-path-editing",
        description: "A motion path with its start and end markers, its dashed preview and the \
                      ghost of the object at the end position.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke, Draws::Translucency],
        responds: axes::TOUCHABLE,
    },
    Entry {
        number: 41,
        family: Family::Manipulation,
        title: "Marching ants (cut/copy indication, and its animation cadence)",
        stem: "marching-ants",
        description: "The marching-ants border around a cut range. The interaction toggle steps \
                      the dash phase, which is how a still plate can say anything about a cadence \
                      at all — the animation itself is the harness's.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 42,
        family: Family::Manipulation,
        title: "Drop indicator — between paragraphs, between slides, into a cell",
        stem: "drop-indicator",
        description: "The three drop indicators together: the caret-bar between paragraphs, the \
                      vertical bar between slides, and the ring around a cell.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 43,
        family: Family::Manipulation,
        title: "Multi-touch transform feedback — pinch scale and two-finger rotate",
        stem: "multi-touch-transform",
        description: "Two touch points on an object with the pinch axis between them, the scale \
                      readout and the rotation arc. **Only judgeable on a touch device** — set the \
                      input toggle to touch, and open the harness on a phone.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke, Draws::Transform],
        responds: axes::TOUCHABLE,
    },
    // ---------------------------------------------------------------------------------------
    // §2.5 · Document furniture (13)
    // ---------------------------------------------------------------------------------------
    Entry {
        number: 44,
        family: Family::Furniture,
        title: "Canvas backdrop, page fill, page border and page shadow",
        stem: "page-and-shadow",
        description: "The four surfaces that make a page read as a lit object on a backdrop. The \
                      shadow is a real `a:outerShdw` through the effect pipeline, at the offset, \
                      blur and alpha the `document.*.page-shadow` token states.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke, Draws::Effect],
        responds: axes::STATIC,
    },
    Entry {
        number: 45,
        family: Family::Furniture,
        title: "Page gap and the page-break indicator (Word)",
        stem: "page-gap-and-break",
        description: "Two pages with the gap between them, the page-break rule, and the same break \
                      in its collapsed form. Judge the gap: too small and the pages merge.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::STATIC,
    },
    Entry {
        number: 46,
        family: Family::Furniture,
        title: "Header and footer dimmed regions with their boundaries",
        stem: "header-footer-regions",
        description: "A page with the header and footer regions dimmed and their boundary rules \
                      drawn. The focused state is header-editing, where the dimming inverts.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke, Draws::Translucency],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 47,
        family: Family::Furniture,
        title: "Text-wrap boundary preview around a floating object",
        stem: "wrap-boundary",
        description: "A floating object over text with its wrap boundary shown, including a tight \
                      wrap that follows the shape rather than the box.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 48,
        family: Family::Furniture,
        title: "Column boundaries and the balanced-column indicator",
        stem: "column-boundaries",
        description: "Three columns with their boundaries and the balance mark at the foot of the \
                      last one. Judge whether the boundaries stay quieter than the text.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::STATIC,
    },
    Entry {
        number: 49,
        family: Family::Furniture,
        title: "Section-break markers",
        stem: "section-break-markers",
        description: "The section-break rule with its label plate, in the two forms a continuous \
                      and a next-page break take.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::STATIC,
    },
    Entry {
        number: 50,
        family: Family::Furniture,
        title: "Footnote separator and continuation notice",
        stem: "footnote-separator",
        description: "The footnote separator rule above a footnote, and the longer continuation \
                      rule that says the note carries over from the previous page.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::STATIC,
    },
    Entry {
        number: 51,
        family: Family::Furniture,
        title: "Non-printing marks — paragraph, space, tab, break, optional hyphen",
        stem: "non-printing-marks",
        description: "The five marks on one line, drawn as geometry rather than as glyphs — see \
                      `crate::scenes` for why no scene in this harness contains text. Judge \
                      whether they are visible without competing with the words.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::STATIC,
    },
    Entry {
        number: 52,
        family: Family::Furniture,
        title: "Field shading and the field-selection state",
        stem: "field-shading",
        description: "Three fields in a paragraph: one at rest, one hovered, one selected. Judge \
                      the shading's weight — it must be visible and must not read as a selection.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Translucency],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 53,
        family: Family::Furniture,
        title: "Bookmark brackets",
        stem: "bookmark-brackets",
        description: "The opening and closing brackets around a bookmarked run, and the collapsed \
                      form of an empty bookmark. Two device pixels wide at 1×.",
        content: RenderedContent::Outline,
        draws: &[Draws::Stroke],
        responds: axes::STATIC,
    },
    Entry {
        number: 54,
        family: Family::Furniture,
        title: "Comment anchor and its connector line to the margin card",
        stem: "comment-anchor",
        description: "A commented run with its anchor, the connector to the margin, and the card \
                      the connector lands on. Judge the connector against the text it crosses.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 55,
        family: Family::Furniture,
        title: "Tracked-change bars and inline insert/delete rendering",
        stem: "tracked-changes",
        description: "An insertion and a deletion on adjacent lines, with the change bars in the \
                      margin. Both tracked-change tokens are `fill-only`: a tracked insertion's own \
                      glyphs stay at the text colour, and this scene must not contradict that.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::STATIC,
    },
    Entry {
        number: 56,
        family: Family::Furniture,
        title: "Slide guides, drawing-canvas boundary, and the safe area",
        stem: "guides-and-safe-area",
        description: "A slide with its centre guides, a drawing-canvas boundary and the safe-area \
                      inset. Three dashed rules at once, which is the judgement: they must be \
                      tellable apart.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::TOUCHABLE,
    },
    // ---------------------------------------------------------------------------------------
    // §2.6 · State and feedback (5)
    // ---------------------------------------------------------------------------------------
    Entry {
        number: 57,
        family: Family::StateAndFeedback,
        title: "Canvas focus ring",
        stem: "canvas-focus-ring",
        description: "The focus ring around the canvas itself, which is what a keyboard user sees \
                      when the document takes focus. The focused state is the element; the default \
                      state is what it must not look like.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 58,
        family: Family::StateAndFeedback,
        title: "Page placeholder for content not yet laid out, and its progressive-refinement tier",
        stem: "page-placeholder",
        description: "A page still being laid out: the skeleton bars, and the tier marker that \
                      says how much of it is real. The active state is the refined tier, so the \
                      two are comparable side by side.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 59,
        family: Family::StateAndFeedback,
        title: "Missing-resource placeholders — absent image, unsupported media",
        stem: "missing-resource",
        description: "The two placeholders side by side: an image that could not be found and a \
                      media type this build cannot play. **They are this harness's own drawing, \
                      not `mjx-scene`'s `PlaceholderGeometry`** — a plate whose `placeholders` \
                      counter was non-zero would be a plate of a stand-in, which the gate refuses.",
        content: RenderedContent::PatternFill,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 60,
        family: Family::StateAndFeedback,
        title: "Font-substitution badge (the user is entitled to know what they see is not what \
                was sent)",
        stem: "font-substitution-badge",
        description: "The badge on a run whose face was substituted, and the marker in the margin \
                      that says a page carries one. `mjx-text` produces the manifest this reports; \
                      this is what a reader is shown.",
        content: RenderedContent::SolidFill,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::INTERACTIVE,
    },
    Entry {
        number: 61,
        family: Family::StateAndFeedback,
        title: "Print-area boundary (Excel) and zoom-dependent hairline rendering",
        stem: "print-area-hairlines",
        description: "The print-area boundary over a grid, with a hairline ladder beside it at \
                      widths of a quarter, a half, three quarters and one point. The density \
                      toggle is the whole point of this scene.",
        content: RenderedContent::Outline,
        draws: &[Draws::Fill, Draws::Stroke],
        responds: axes::STATIC,
    },
];

/// The entry with that number, or `None`.
#[must_use]
pub fn entry(number: u8) -> Option<&'static Entry> {
    INVENTORY.iter().find(|entry| entry.number == number)
}

/// The entry with that slug, or `None`.
#[must_use]
pub fn entry_by_slug(slug: &str) -> Option<&'static Entry> {
    INVENTORY.iter().find(|entry| entry.slug() == slug)
}
