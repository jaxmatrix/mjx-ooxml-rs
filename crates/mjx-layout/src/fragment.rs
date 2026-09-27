//! [`FragmentTree`] — what every box model produces and everything above this layer consumes.
//!
//! # This is the seam
//!
//! Above a `FragmentTree`, nothing has heard of OOXML. Scene building, tessellation, painting,
//! hit-testing, selection, caret movement, comment anchoring, accessibility and every exporter are
//! written against these six fragment kinds and a [`SourceRef`], and none of them can tell a
//! `.docx` from a Markdown file from the plain-text box model in this crate's own tests. That is the
//! whole point: swap the box model and the rest of the stack keeps working.
//!
//! Below it, a box model may do anything at all — absolute placement, reflow, a grid, a CSS
//! formatting context — as long as it says where things went in this vocabulary.
//!
//! # Why a flat arena and not a tree of boxes
//!
//! A page of prose is tens of thousands of fragments and a long document is millions. A
//! `Box<Fragment>` per node would be one allocation each, one pointer chase per step, and a
//! recursive drop deep enough to overflow the stack on a pathological nesting. So the tree is a
//! `Vec` of nodes plus parent/child/sibling indices: one allocation for the whole page, iteration in
//! memory order for the painter, no recursion anywhere, and a page that drops in one `free`.
//!
//! Two side tables carry what is *shared* rather than per-node. Every line and every glyph run
//! inside one rotated shape has the same transform and the same clip, so the nodes hold a 4-byte
//! index into a [`Transform`] table whose entry 0 is always the identity, and an optional index into
//! a clip table. A page with no rotation and no clipping stores one transform in total.
//!
//! # Local coordinates, and where the page-space answer comes from
//!
//! A node's `rect` is in the coordinate space of its own [`Transform`] — for the overwhelmingly
//! common identity transform that is page space, and for a rotated shape it is the shape's own
//! space, where the rectangle is still a rectangle. [`FragmentTree::page_bounds`] maps it out to the
//! axis-aligned box that certainly contains it, which is what the spatial index is built from.
//! Storing only the mapped box would lose the shape's own geometry, and storing both on every node
//! would spend 32 bytes a node to cache a computation that is free when the transform is the
//! identity.
//!
//! # Order is meaning
//!
//! **Children are in paint order**: the last child is drawn last and is therefore on top, which is
//! what makes the *last* answer of a hit test the one a click belongs to.
//!
//! **The children of a [`LineFragment`] are in visual order** — left to right on the page, whatever
//! the paragraph's direction — because that is the order they are painted in. Their `SourceRef`s are
//! in *logical* order, and a right-to-left line's are therefore descending. Both facts are needed:
//! the visual order draws the line, and the logical order moves the caret through it. This is the
//! contract half of `mjx-text`'s rule that a run is **shaped in `logical_runs` order and placed in
//! `visual_runs` order`.

use mjx_ooxml_core::measure::Emu;
use mjx_text::{BidiLevel, FaceId, ShapedRun, TextDirection};

use crate::measure::{LayoutPoint, LayoutRect, Transform};
use crate::source::SourceRef;

/// A node's position in a [`FragmentTree`].
///
/// An index rather than a reference, so that a fragment can be named in a spatial index, a hit-test
/// result, a selection or a cache without borrowing the tree.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FragmentId(u32);

impl FragmentId {
    /// The index, for a diagnostic message.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }

    /// The identifier of the `index`th node, saturating rather than wrapping on a tree that cannot
    /// exist. Crate-private: an identifier a caller invented would name a fragment in no tree.
    pub(crate) fn from_index(index: usize) -> Self {
        Self(u32::try_from(index).unwrap_or(u32::MAX))
    }
}

/// Which [`Transform`] a node's coordinates are in. Entry 0 is always [`Transform::IDENTITY`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TransformId(u32);

impl TransformId {
    /// The identity, which every node that is not inside a rotated or scaled space uses.
    pub const IDENTITY: Self = Self(0);
}

/// Which clip rectangle a node is drawn under, in the same coordinate space as its own `rect`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ClipId(u32);

/// A handle to whatever paints a box or a shape — a fill, an outline, a shadow stack.
///
/// **Opaque here, deliberately.** Decoration is where a box model's own vocabulary would leak into
/// the seam: DrawingML's fill/line/effect model is not CSS's is not a plain-text renderer's. The box
/// model issues these numbers and the layer that resolves them is the one that also holds the box
/// model — a scene builder pairs a `mjx-layout-pptx` fragment tree with `mjx-dml`'s resolved fill,
/// and neither this crate nor the fragment tree needs to know that.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DecorationRef(u64);

impl DecorationRef {
    /// The handle numbered `number`.
    #[must_use]
    pub const fn new(number: u64) -> Self {
        Self(number)
    }

    /// The number, for the table that resolves it.
    #[must_use]
    pub const fn number(self) -> u64 {
        self.0
    }
}

/// A handle to an image's pixels, resolved the same way a [`DecorationRef`] is.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ImageRef(u64);

impl ImageRef {
    /// The handle numbered `number`.
    #[must_use]
    pub const fn new(number: u64) -> Self {
        Self(number)
    }

    /// The number, for the table that resolves it.
    #[must_use]
    pub const fn number(self) -> u64 {
        self.0
    }
}

/// A handle to a shape's outline, resolved the same way a [`DecorationRef`] is.
///
/// This is `docs/UI_PLATFORM_PLAN.md` §4 L4's `GeometryProvider` seam seen from below: a box model
/// says *this shape, at this size*, and the scene builder asks whatever provider it holds for the
/// paths. Which is why nothing here is a path.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct GeometryRef(u64);

impl GeometryRef {
    /// The handle numbered `number`.
    #[must_use]
    pub const fn new(number: u64) -> Self {
        Self(number)
    }

    /// The number, for the provider that resolves it.
    #[must_use]
    pub const fn number(self) -> u64 {
        self.0
    }
}

/// Where a cell sits in its table's grid.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TableCell {
    /// Which column its left edge is in, counted from zero across the whole table.
    pub column: u16,
    /// Which row its top edge is in, counted from zero across the **whole table**, not from the top
    /// of this page — so a row on the fourth page of a split table still says which row it is.
    pub row: u32,
    /// How many columns it spans; one for an unmerged cell.
    pub column_span: u16,
    /// How many rows it spans; one for an unmerged cell.
    pub row_span: u32,
}

/// A rectangular area with a decoration — the general container, and the fragment every other kind
/// hangs under.
///
/// The node's own `rect` is the **border box**: the outer edge of the border, which is the rectangle
/// a background paints and a hit test answers with. Padding and content boxes are the box model's
/// own business and reach the seam as the positions of the fragments inside.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BoxFragment {
    /// What paints it, if anything.
    pub decoration: Option<DecorationRef>,
    /// Where it sits in its table, when it is a table cell.
    ///
    /// A cell has to be addressable *as a cell* for a hit test to say "row 4, column 2" rather than
    /// "some box", which is what a spreadsheet's whole interaction model is and what a Word table's
    /// selection needs. It is on `BoxFragment` rather than a seventh fragment kind because a cell is
    /// a box in every respect except that it knows its coordinates.
    pub cell: Option<TableCell>,
}

/// One line of text: where its baseline is, how far the tallest thing on it reaches, and which way
/// it reads.
///
/// The glyphs are its **children**, in visual order. A line is a fragment of its own rather than a
/// property of the runs because everything that acts on a line acts on all of it at once: clicking
/// past the end of a line, extending a selection by a line, the underline that spans a whole line,
/// vertical caret movement, and a screen reader's line granularity.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LineFragment {
    /// Where the baseline sits, measured **down from the top of the line's own `rect`**.
    ///
    /// Relative rather than absolute so that moving a line — which justification, vertical
    /// alignment and a page break all do — is one edit to the rect and nothing else.
    pub baseline: Emu,
    /// How far above the baseline the line reaches: the largest ascent of anything on it.
    pub ascent: Emu,
    /// How far below the baseline it reaches: the largest descent of anything on it.
    pub descent: Emu,
    /// Which way the line reads as a whole — the paragraph's resolved base direction, which is what
    /// decides which margin it starts at and where an odd last line hangs.
    pub base_direction: TextDirection,
    /// The trailing width that was allowed to hang past the measure, if any.
    ///
    /// `w:overflowPunct` lets a line-final comma or full stop extend past the column rather than
    /// pushing the character before it onto the next line. The line's `rect` includes it — it *is*
    /// drawn there — and this says how much of the right-hand edge was not counted when the line was
    /// fitted, which a justification pass and a column-balancing pass both need.
    pub hanging_width: Emu,
}

/// A run of shaped glyphs, at one size in one face, drawn from one origin.
///
/// It carries the [`ShapedRun`] the text engine produced rather than a list of positions, and that
/// is deliberate. Positions are **resolution-dependent**: `mjx-text`'s
/// [`place_run`](mjx_text::place_run) puts every glyph on a whole pixel at a quantised scale, and
/// which pixel that is depends on the zoom. A fragment tree that carried pixels would have to be
/// rebuilt on every zoom step, which would throw away the checkpoint machinery this crate exists to
/// provide. So layout decides *what text, in what face, at what size, with its pen starting here*,
/// and the layer that knows the device scale calls `place_run` with the run and the origin.
#[derive(Clone, PartialEq, Debug)]
pub struct GlyphRunFragment {
    /// Which face, as the rasteriser that will draw it numbered the face.
    pub face: FaceId,
    /// The shaped glyphs, in draw order — the order [`ShapedRun::glyphs`] is already in, left to
    /// right whatever the run's direction.
    pub run: ShapedRun,
    /// Where the pen starts: on the baseline, at the run's leading edge in **visual** order.
    pub origin: LayoutPoint,
    /// Which way the run is written.
    pub direction: TextDirection,
    /// The UAX #9 level it was resolved at, which a selection needs in order to know that a visually
    /// contiguous highlight may be two logical ranges.
    pub level: BidiLevel,
}

/// A picture, placed.
#[derive(Clone, PartialEq, Debug)]
pub struct ImageFragment {
    /// Which image.
    pub image: ImageRef,
    /// Which part of it is shown, as fractions of the whole image, or `None` for all of it.
    ///
    /// A crop is a fraction rather than a length because the fragment does not know the image's
    /// pixel dimensions and must not have to: `a:srcRect` is in thousandths of a percent and CSS's
    /// `object-view-box` is in percentages, and both reduce to this.
    pub crop: Option<UnitRect>,
}

/// A rectangle in fractions of something else — `0.0` is one edge and `1.0` the other.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct UnitRect {
    /// The left edge, as a fraction of the width.
    pub left: f64,
    /// The top edge, as a fraction of the height.
    pub top: f64,
    /// The right edge, as a fraction of the width.
    pub right: f64,
    /// The bottom edge, as a fraction of the height.
    pub bottom: f64,
}

/// A shape whose outline comes from a geometry provider, placed and decorated.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ShapeFragment {
    /// Which outline, at the node's own `rect`.
    pub geometry: GeometryRef,
    /// What paints it, if anything.
    pub decoration: Option<DecorationRef>,
}

/// A table, or the part of one that is on this page.
///
/// Its cells are [`BoxFragment`] children carrying a [`TableCell`]. What is here is the part a cell
/// cannot say: how wide the grid is, which of its rows repeat as headers, and whether this is the
/// whole table or a piece of one — the three facts a reader needs in order to be told "continued"
/// and a screen reader needs in order to announce a header row again.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TableFragment {
    /// How many columns the whole table has.
    pub columns: u16,
    /// Which rows of the whole table are on this page, counted from zero at the table's first row.
    pub rows: std::ops::Range<u32>,
    /// How many of the table's leading rows repeat at the top of each page it continues onto.
    pub header_rows: u16,
    /// Whether the table began on an earlier page.
    pub continued_from_previous_page: bool,
    /// Whether it carries on onto a later one.
    pub continues_on_next_page: bool,
}

/// What a fragment is.
///
/// Six kinds, and the list is closed on purpose: it is the vocabulary every layer above this one is
/// written against, so a seventh kind is a change to every consumer. Anything a box model wants to
/// say that is not one of these is said with a [`DecorationRef`], a [`GeometryRef`] or an
/// [`ImageRef`], all of which the box model's own companion resolves.
#[derive(Clone, PartialEq, Debug)]
pub enum Fragment {
    /// A rectangular area — a page, a shape's body, a paragraph, a table cell.
    Box(BoxFragment),
    /// One line of text.
    Line(LineFragment),
    /// A run of shaped glyphs.
    GlyphRun(GlyphRunFragment),
    /// A picture.
    Image(ImageFragment),
    /// A shape with an outline.
    Shape(ShapeFragment),
    /// A table, or the part of one on this page.
    Table(TableFragment),
}

impl Fragment {
    /// A short name for the kind, for a diagnostic message.
    #[must_use]
    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::Box(_) => "box",
            Self::Line(_) => "line",
            Self::GlyphRun(_) => "glyph run",
            Self::Image(_) => "image",
            Self::Shape(_) => "shape",
            Self::Table(_) => "table",
        }
    }
}

/// One node of a [`FragmentTree`].
#[derive(Clone, PartialEq, Debug)]
pub struct FragmentNode {
    source: SourceRef,
    rect: LayoutRect,
    transform: TransformId,
    clip: Option<ClipId>,
    fragment: Fragment,
    parent: Option<FragmentId>,
    first_child: Option<FragmentId>,
    last_child: Option<FragmentId>,
    next_sibling: Option<FragmentId>,
}

impl FragmentNode {
    /// Where in the document it came from.
    #[must_use]
    pub const fn source(&self) -> &SourceRef {
        &self.source
    }

    /// Its rectangle, in the coordinate space of its own [`FragmentNode::transform`].
    #[must_use]
    pub const fn rect(&self) -> LayoutRect {
        self.rect
    }

    /// Which transform its coordinates are in.
    #[must_use]
    pub const fn transform(&self) -> TransformId {
        self.transform
    }

    /// Which clip it is drawn under, if any.
    #[must_use]
    pub const fn clip(&self) -> Option<ClipId> {
        self.clip
    }

    /// What it is.
    #[must_use]
    pub const fn fragment(&self) -> &Fragment {
        &self.fragment
    }

    /// Its parent, or `None` if it is a root.
    #[must_use]
    pub const fn parent(&self) -> Option<FragmentId> {
        self.parent
    }

    /// Its first child in paint order, if it has one.
    #[must_use]
    pub const fn first_child(&self) -> Option<FragmentId> {
        self.first_child
    }

    /// Its last child in paint order — the one on top — if it has one.
    #[must_use]
    pub const fn last_child(&self) -> Option<FragmentId> {
        self.last_child
    }

    /// The next node under the same parent, if there is one.
    #[must_use]
    pub const fn next_sibling(&self) -> Option<FragmentId> {
        self.next_sibling
    }
}

/// Everything one page of layout produced, and where it all is.
///
/// Built with a [`FragmentTreeBuilder`] and immutable afterwards. A tree that could be edited would
/// need the spatial index rebuilt on every edit and would let a caller move a fragment out from
/// under a hit-test result; re-laying a page out is what an edit does instead, which is what
/// [`BoxModel::invalidate`](crate::BoxModel::invalidate) is for.
#[derive(Clone, PartialEq, Debug)]
pub struct FragmentTree {
    nodes: Vec<FragmentNode>,
    roots: Vec<FragmentId>,
    transforms: Vec<Transform>,
    clips: Vec<LayoutRect>,
}

impl FragmentTree {
    /// How many fragments there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the page produced nothing at all — an empty slide, a blank page.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The top-level fragments, in paint order.
    #[must_use]
    pub fn roots(&self) -> &[FragmentId] {
        &self.roots
    }

    /// One node, or `None` for an identifier from another tree.
    #[must_use]
    pub fn node(&self, id: FragmentId) -> Option<&FragmentNode> {
        self.nodes.get(id.0 as usize)
    }

    /// Every node, in the order they were added — which is document order within each subtree, and
    /// is the order a painter walks.
    pub fn nodes(&self) -> impl Iterator<Item = (FragmentId, &FragmentNode)> {
        self.nodes.iter().enumerate().map(|(index, node)| {
            // The vector's length is bounded by `FragmentTreeBuilder`, which refuses to grow past
            // `u32::MAX`, so the cast is exact.
            (FragmentId(index as u32), node)
        })
    }

    /// Every identifier, in the same order.
    pub fn ids(&self) -> impl Iterator<Item = FragmentId> + '_ {
        (0..self.nodes.len()).map(|index| FragmentId(index as u32))
    }

    /// The transform a node's coordinates are in.
    #[must_use]
    pub fn transform(&self, id: TransformId) -> Transform {
        self.transforms
            .get(id.0 as usize)
            .copied()
            .unwrap_or(Transform::IDENTITY)
    }

    /// A clip rectangle, in the coordinate space of the node that named it.
    #[must_use]
    pub fn clip(&self, id: ClipId) -> Option<LayoutRect> {
        self.clips.get(id.0 as usize).copied()
    }

    /// A node's axis-aligned bounding box **in page space**, which is what the spatial index is
    /// keyed on and what a viewport cull compares against.
    ///
    /// Equal to the node's own `rect` whenever its transform is the identity, which is every node on
    /// a page with no rotation.
    #[must_use]
    pub fn page_bounds(&self, id: FragmentId) -> Option<LayoutRect> {
        let node = self.node(id)?;
        Some(self.transform(node.transform).map_rect_bounds(node.rect))
    }

    /// Whether `point`, in page space, is inside the node — exactly, through its transform rather
    /// than through its bounding box.
    ///
    /// This is the narrow phase a hit test runs after the spatial index has offered a candidate. For
    /// an identity transform it is one rectangle test; for a rotated one it maps the point back into
    /// the node's own space, where the rectangle is a rectangle again. A transform that collapses
    /// the plane — a scale of zero — contains nothing, which is the honest answer for a shape with
    /// no area.
    #[must_use]
    pub fn contains_page_point(&self, id: FragmentId, point: LayoutPoint) -> bool {
        let Some(node) = self.node(id) else {
            return false;
        };
        let transform = self.transform(node.transform);
        if transform.is_identity() {
            return node.rect.contains(point);
        }
        let Some(inverse) = transform.inverse() else {
            return false;
        };
        node.rect.contains(inverse.apply(point))
    }

    /// The children of `id`, in paint order.
    pub fn children(&self, id: FragmentId) -> Children<'_> {
        Children {
            tree: self,
            next: self.node(id).and_then(|node| node.first_child),
        }
    }

    /// The node's ancestors, closest first.
    pub fn ancestors(&self, id: FragmentId) -> Ancestors<'_> {
        Ancestors {
            tree: self,
            next: self.node(id).and_then(|node| node.parent),
        }
    }

    /// How much memory the tree holds, for the byte-budgeted cache page fragments live in.
    ///
    /// Counts the three vectors' allocations. It does not count the glyphs behind a
    /// [`ShapedRun`]'s `Arc`, and it says so rather than guessing: those are shared with the
    /// shaped-run cache, so charging them to a page would count the same bytes once per page that
    /// draws the same word.
    #[must_use]
    pub fn heap_bytes(&self) -> usize {
        self.nodes.capacity() * std::mem::size_of::<FragmentNode>()
            + self.roots.capacity() * std::mem::size_of::<FragmentId>()
            + self.transforms.capacity() * std::mem::size_of::<Transform>()
            + self.clips.capacity() * std::mem::size_of::<LayoutRect>()
    }
}

/// The children of one node, in paint order.
#[derive(Debug)]
pub struct Children<'a> {
    tree: &'a FragmentTree,
    next: Option<FragmentId>,
}

impl Iterator for Children<'_> {
    type Item = FragmentId;

    fn next(&mut self) -> Option<FragmentId> {
        let current = self.next?;
        self.next = self.tree.node(current).and_then(|node| node.next_sibling);
        Some(current)
    }
}

/// A node's ancestors, closest first.
#[derive(Debug)]
pub struct Ancestors<'a> {
    tree: &'a FragmentTree,
    next: Option<FragmentId>,
}

impl Iterator for Ancestors<'_> {
    type Item = FragmentId;

    fn next(&mut self) -> Option<FragmentId> {
        let current = self.next?;
        self.next = self.tree.node(current).and_then(|node| node.parent);
        Some(current)
    }
}

/// Builds a [`FragmentTree`] one fragment at a time, in paint order.
///
/// A builder rather than a constructor because a box model produces fragments as it goes and does
/// not know how many there will be, and because appending to a flat vector is the one shape that
/// stays a single allocation while it grows.
///
/// # A full tree stops growing rather than panicking
///
/// A `FragmentId` is a `u32`, so a page may hold about four billion fragments. A document that
/// reaches that has a defect in it, and the honest response is to stop adding rather than to abort:
/// [`FragmentTreeBuilder::push`] returns `None` once the tree is full, and a box model that ignores
/// the `None` produces a truncated page instead of a crash.
#[derive(Clone, Debug)]
pub struct FragmentTreeBuilder {
    nodes: Vec<FragmentNode>,
    roots: Vec<FragmentId>,
    transforms: Vec<Transform>,
    clips: Vec<LayoutRect>,
}

impl FragmentTreeBuilder {
    /// An empty builder, with the identity as transform 0.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            roots: Vec::new(),
            transforms: vec![Transform::IDENTITY],
            clips: Vec::new(),
        }
    }

    /// An empty builder with room for `fragments` nodes already reserved.
    #[must_use]
    pub fn with_capacity(fragments: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(fragments),
            roots: Vec::new(),
            transforms: vec![Transform::IDENTITY],
            clips: Vec::new(),
        }
    }

    /// How many fragments have been added.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether nothing has been added.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Register a transform and get the identifier the nodes inside it use.
    ///
    /// The identity is always [`TransformId::IDENTITY`] and is never registered again, so a page
    /// with no rotation holds exactly one transform.
    pub fn transform(&mut self, transform: Transform) -> TransformId {
        if transform.is_identity() {
            return TransformId::IDENTITY;
        }
        if let Some(existing) = self
            .transforms
            .iter()
            .position(|candidate| *candidate == transform)
        {
            // The vector never grows past `u32::MAX`, so the cast is exact.
            return TransformId(existing as u32);
        }
        if self.transforms.len() >= u32::MAX as usize {
            return TransformId::IDENTITY;
        }
        let id = TransformId(self.transforms.len() as u32);
        self.transforms.push(transform);
        id
    }

    /// Register a clip rectangle and get the identifier the nodes under it use.
    ///
    /// Returns `None` for an empty rectangle: a clip that encloses nothing would hide the subtree
    /// entirely, and a box model that means that says it by not emitting the subtree.
    pub fn clip(&mut self, rect: LayoutRect) -> Option<ClipId> {
        if rect.is_empty() || self.clips.len() >= u32::MAX as usize {
            return None;
        }
        if let Some(existing) = self.clips.iter().position(|candidate| *candidate == rect) {
            return Some(ClipId(existing as u32));
        }
        let id = ClipId(self.clips.len() as u32);
        self.clips.push(rect);
        Some(id)
    }

    /// Add a fragment under `parent`, or as a root when `parent` is `None`.
    ///
    /// Returns the new fragment's identifier, or `None` when the tree is full or `parent` names no
    /// node in this builder.
    pub fn push(
        &mut self,
        parent: Option<FragmentId>,
        source: SourceRef,
        rect: LayoutRect,
        transform: TransformId,
        clip: Option<ClipId>,
        fragment: Fragment,
    ) -> Option<FragmentId> {
        if self.nodes.len() >= u32::MAX as usize {
            return None;
        }
        if let Some(parent) = parent {
            self.nodes.get(parent.0 as usize)?;
        }
        // The length was just bounded below `u32::MAX`, so the cast is exact.
        let id = FragmentId(self.nodes.len() as u32);
        self.nodes.push(FragmentNode {
            source,
            rect,
            transform,
            clip,
            fragment,
            parent,
            first_child: None,
            last_child: None,
            next_sibling: None,
        });
        match parent {
            None => self.roots.push(id),
            Some(parent) => {
                // Checked above, so this cannot be `None`; written as a `let else` rather than an
                // index because no layout path may panic.
                let Some(parent_node) = self.nodes.get_mut(parent.0 as usize) else {
                    return Some(id);
                };
                match parent_node.last_child.replace(id) {
                    None => parent_node.first_child = Some(id),
                    Some(previous) => {
                        if let Some(previous_node) = self.nodes.get_mut(previous.0 as usize) {
                            previous_node.next_sibling = Some(id);
                        }
                    }
                }
            }
        }
        Some(id)
    }

    /// Add a fragment under `parent` with the identity transform and no clip — the common case.
    pub fn push_simple(
        &mut self,
        parent: Option<FragmentId>,
        source: SourceRef,
        rect: LayoutRect,
        fragment: Fragment,
    ) -> Option<FragmentId> {
        self.push(parent, source, rect, TransformId::IDENTITY, None, fragment)
    }

    /// The finished tree.
    #[must_use]
    pub fn finish(self) -> FragmentTree {
        FragmentTree {
            nodes: self.nodes,
            roots: self.roots,
            transforms: self.transforms,
            clips: self.clips,
        }
    }
}

impl Default for FragmentTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
