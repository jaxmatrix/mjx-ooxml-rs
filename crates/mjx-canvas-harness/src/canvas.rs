//! The drawing surface every scene is written against, and the palette every colour comes out of.
//!
//! # Why a canvas of its own rather than a `FragmentTreeBuilder`
//!
//! A [`FragmentTree`] is finished when it is finished: there is no way to reopen one and add a node.
//! The hit-test visualiser has to draw overlays that are *derived from the spatial index of the tree
//! without them* — otherwise the overlay is the topmost thing at its own centre and the visualiser
//! would be checking itself — so the scene has to be materialised **twice**, once to be measured and
//! once to be drawn. [`Canvas`] is therefore plain data: a vector of pending nodes, a decoration
//! table and an outline table, from which [`Canvas::tree`] mints a fresh tree as often as it is
//! asked.
//!
//! # Everything is in points, and the scale is applied once
//!
//! A rectangle is in points because a fragment tree is in EMU and EMU is a point measure; a path is
//! in points because it is the same drawing at every density and only the raster changes. The one
//! place the density enters is [`Canvas::outline`]'s answer, where a stored path is multiplied by
//! [`Canvas::pixels_per_point`] — which is what makes hairline widths *scale* at 2× and 3× rather
//! than staying the same number of device pixels. `tests/the_axes_are_not_identities.rs` measures
//! exactly that.
//!
//! # ⚠ No scene contains text, and the decision is inherited rather than taken
//!
//! MJXOFF-165's specimen module states it: a golden image containing glyph coverage is a golden
//! image of *a rasteriser version and an installed face*, and a patch release of `swash` that moved
//! one pixel of one stem would expire every approval for a reason that has nothing to do with
//! fidelity. It applies here with more force, not less — a caret plate is a plate of a caret, and a
//! plate of a caret *in a line of real glyphs* is a plate of the font stack. So where a scene needs
//! text it draws [`Canvas::text_line`]: grey bars at the metrics of a line of words. The caret, the
//! selection fill, the squiggle and the bookmark bracket are all real; what they sit on is not, and
//! that is deliberate.

use mjx_layout::{
    BoxFragment, DecorationRef, Fragment, FragmentId, FragmentTree, FragmentTreeBuilder,
    GeometryRef, LayoutRect, PartId, ShapeFragment, SourcePath, SourceRef, Transform,
};
use mjx_ooxml_core::measure::{Angle, Emu};
use mjx_scene::{
    Color, Decoration, FillRule, FillStyle, Geometry, GeometryProvider, OutlineProvenance,
    PathCommand, ResolvedOutline, ResourceResolver, SceneError, ScenePoint, SceneRect, StrokeStyle,
};
use mjx_tokens::{ColorScheme, DocumentColors, ThemeColors, Tokens};

use crate::state::{Input, Interaction, State};

/// The stage every scene is drawn on, in points.
///
/// One size for the whole inventory, so a plate's dimensions are never what differs between two
/// entries — the same reason `mjx-render-oracle`'s corpus has one page size.
pub const STAGE: (f64, f64) = (300.0, 200.0);

/// The page inside the stage, in points: the white rectangle a document is on.
pub const PAGE: Rect = Rect {
    x: 18.0,
    y: 12.0,
    width: 264.0,
    height: 176.0,
};

/// A point on the stage, in typographic points.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Pt {
    /// Points from the stage's left edge.
    pub x: f64,
    /// Points from its top edge.
    pub y: f64,
}

/// A point, spelled shortly, because a path is a list of them.
#[must_use]
pub const fn pt(x: f64, y: f64) -> Pt {
    Pt { x, y }
}

/// A rectangle on the stage, in points.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rect {
    /// Points from the stage's left edge.
    pub x: f64,
    /// Points from its top edge.
    pub y: f64,
    /// How wide.
    pub width: f64,
    /// How tall.
    pub height: f64,
}

impl Rect {
    /// A rectangle from its origin and size.
    #[must_use]
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// A square of `side` points centred on `centre`.
    #[must_use]
    pub fn centred(centre: Pt, side: f64) -> Self {
        Self::new(centre.x - side / 2.0, centre.y - side / 2.0, side, side)
    }

    /// The right edge.
    #[must_use]
    pub fn right(self) -> f64 {
        self.x + self.width
    }

    /// The bottom edge.
    #[must_use]
    pub fn bottom(self) -> f64 {
        self.y + self.height
    }

    /// The middle.
    #[must_use]
    pub fn centre(self) -> Pt {
        pt(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// The same rectangle grown by `amount` points on every side. A negative amount shrinks it.
    #[must_use]
    pub fn inflated(self, amount: f64) -> Self {
        Self::new(
            self.x - amount,
            self.y - amount,
            self.width + amount * 2.0,
            self.height + amount * 2.0,
        )
    }

    /// The same rectangle moved.
    #[must_use]
    pub fn offset(self, dx: f64, dy: f64) -> Self {
        Self::new(self.x + dx, self.y + dy, self.width, self.height)
    }

    /// The rectangle as the fragment tree wants it.
    #[must_use]
    pub fn to_layout(self) -> LayoutRect {
        LayoutRect::from_edges(
            Emu::from_points(self.x),
            Emu::from_points(self.y),
            Emu::from_points(self.right()),
            Emu::from_points(self.bottom()),
        )
    }

    /// The eight positions a resize handle sits at, clockwise from the top-left corner.
    #[must_use]
    pub fn handles(self) -> [Pt; 8] {
        let centre = self.centre();
        [
            pt(self.x, self.y),
            pt(centre.x, self.y),
            pt(self.right(), self.y),
            pt(self.right(), centre.y),
            pt(self.right(), self.bottom()),
            pt(centre.x, self.bottom()),
            pt(self.x, self.bottom()),
            pt(self.x, centre.y),
        ]
    }
}

/// A node the canvas has recorded but not yet turned into a fragment.
#[derive(Clone, PartialEq, Debug)]
struct Pending {
    parent: Option<usize>,
    rect: Rect,
    transform: Option<Transform>,
    clip: Option<Rect>,
    kind: Kind,
}

/// What kind of fragment a pending node becomes.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    /// A box: a rectangle with a decoration.
    Box(Option<usize>),
    /// A shape: an outline handle with a decoration.
    Shape(usize, Option<usize>),
}

/// A handle on a node the canvas has drawn — what a grab region and a hit test are recorded against.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NodeId(usize);

impl NodeId {
    /// Which node this is, in the order the canvas drew them — and therefore which
    /// [`mjx_layout::FragmentId`] it becomes, since [`Canvas::tree`] pushes them in that order.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// One grab region: **a node, not a rectangle**.
///
/// The rectangle is deliberately absent. `docs/client-platform/CANVAS_UI_INVENTORY.md` §3 asks for a
/// hit-test visualiser and MJXOFF-166 adds *"proved against the spatial index, not against a second
/// region computation"* — so a grab region records **which fragment is grabbed** and how far past it
/// the grab reaches, and the rectangle is
/// [`mjx_layout::SpatialIndex::bounds_of`] inflated by that. There is one answer to *"where is this
/// element"* in the workspace, and it is the index's.
#[derive(Clone, PartialEq, Debug)]
pub struct Grab {
    /// The node whose bounds the region is measured from.
    pub node: NodeId,
    /// What it grabs, for the overlay's own legend and for a failure message.
    pub label: String,
    /// How far past the node's own bounds the region reaches, in points.
    pub padding: f64,
}

/// The drawing surface a scene is written against.
#[derive(Clone, PartialEq, Debug)]
pub struct Canvas {
    nodes: Vec<Pending>,
    decorations: Vec<Decoration>,
    outlines: Vec<(Vec<PathCommand>, FillRule, &'static str)>,
    grabs: Vec<Grab>,
    pixels_per_point: f32,
    state: State,
    ink: Ink,
}

impl Canvas {
    /// A canvas for one point of the state matrix, with the palette `tokens` gives that point.
    #[must_use]
    pub fn new(tokens: &Tokens, state: State) -> Self {
        Self {
            nodes: Vec::new(),
            decorations: Vec::new(),
            outlines: Vec::new(),
            grabs: Vec::new(),
            pixels_per_point: state.density.device_scale().pixels_per_point(),
            state,
            ink: Ink::new(tokens, state),
        }
    }

    /// The palette.
    #[must_use]
    pub fn ink(&self) -> &Ink {
        &self.ink
    }

    /// Which point of the matrix this canvas is drawing.
    #[must_use]
    pub fn state(&self) -> State {
        self.state
    }

    /// How many device pixels there are to a point — the one place density enters a drawing.
    #[must_use]
    pub fn pixels_per_point(&self) -> f32 {
        self.pixels_per_point
    }

    /// Register a decoration and return the handle a fragment carries.
    fn decoration(&mut self, decoration: Decoration) -> usize {
        self.decorations.push(decoration);
        self.decorations.len() - 1
    }

    /// Register an outline and return the handle a shape fragment carries.
    fn outline_handle(
        &mut self,
        commands: Vec<PathCommand>,
        fill_rule: FillRule,
        label: &'static str,
    ) -> usize {
        self.outlines.push((commands, fill_rule, label));
        self.outlines.len() - 1
    }

    /// Add a node and return its handle.
    fn push(&mut self, parent: Option<NodeId>, rect: Rect, kind: Kind) -> NodeId {
        self.nodes.push(Pending {
            parent: parent.map(|id| id.0),
            rect,
            transform: None,
            clip: None,
            kind,
        });
        NodeId(self.nodes.len() - 1)
    }

    /// A filled and/or stroked rectangle.
    pub fn rect(&mut self, parent: Option<NodeId>, rect: Rect, decoration: Decoration) -> NodeId {
        let handle = self.decoration(decoration);
        self.push(parent, rect, Kind::Box(Some(handle)))
    }

    /// A group: a node that draws nothing itself and holds children.
    ///
    /// `opacity` below one becomes a `PushOpacity`, and `clip` becomes a `PushClip`, which is how a
    /// scene declaring [`crate::inventory::Draws::Translucency`] or
    /// [`crate::inventory::Draws::Clip`] satisfies its own gate.
    pub fn group(
        &mut self,
        parent: Option<NodeId>,
        rect: Rect,
        opacity: f32,
        clip: Option<Rect>,
    ) -> NodeId {
        let decoration = Decoration {
            opacity,
            ..Decoration::none()
        };
        let handle = self.decoration(decoration);
        let id = self.push(parent, rect, Kind::Box(Some(handle)));
        self.nodes[id.0].clip = clip;
        id
    }

    /// An explicit path, drawn inside `bounds`.
    ///
    /// The commands are in **points** and are multiplied by [`Canvas::pixels_per_point`] when the
    /// provider is asked for them, which is why a scene never mentions a device pixel.
    pub fn path(
        &mut self,
        parent: Option<NodeId>,
        bounds: Rect,
        commands: Vec<PathCommand>,
        fill_rule: FillRule,
        label: &'static str,
        decoration: Decoration,
    ) -> NodeId {
        let outline = self.outline_handle(commands, fill_rule, label);
        let handle = self.decoration(decoration);
        self.push(parent, bounds, Kind::Shape(outline, Some(handle)))
    }

    /// A polyline through `points`, stroked, optionally closed.
    pub fn polyline(
        &mut self,
        parent: Option<NodeId>,
        points: &[Pt],
        close: bool,
        label: &'static str,
        stroke: StrokeStyle,
    ) -> NodeId {
        let bounds = bounds_of(points);
        let mut commands = Vec::with_capacity(points.len() + 1);
        for (index, point) in points.iter().enumerate() {
            let scene = ScenePoint::new(point.x as f32, point.y as f32);
            commands.push(if index == 0 {
                PathCommand::MoveTo(scene)
            } else {
                PathCommand::LineTo(scene)
            });
        }
        if close {
            commands.push(PathCommand::Close);
        }
        self.path(
            parent,
            bounds,
            commands,
            FillRule::NonZero,
            label,
            Decoration {
                stroke: Some(stroke),
                ..Decoration::none()
            },
        )
    }

    /// A straight line, stroked.
    pub fn line(
        &mut self,
        parent: Option<NodeId>,
        from: Pt,
        to: Pt,
        label: &'static str,
        stroke: StrokeStyle,
    ) -> NodeId {
        self.polyline(parent, &[from, to], false, label, stroke)
    }

    /// A filled polygon through `points`.
    pub fn polygon(
        &mut self,
        parent: Option<NodeId>,
        points: &[Pt],
        label: &'static str,
        decoration: Decoration,
    ) -> NodeId {
        let bounds = bounds_of(points);
        let mut commands = Vec::with_capacity(points.len() + 1);
        for (index, point) in points.iter().enumerate() {
            let scene = ScenePoint::new(point.x as f32, point.y as f32);
            commands.push(if index == 0 {
                PathCommand::MoveTo(scene)
            } else {
                PathCommand::LineTo(scene)
            });
        }
        commands.push(PathCommand::Close);
        self.path(
            parent,
            bounds,
            commands,
            FillRule::NonZero,
            label,
            decoration,
        )
    }

    /// A circle, as four cubic segments.
    ///
    /// Cubic rather than quadratic because a quarter-circle's cubic approximation is exact to one
    /// part in two thousand at the magic constant below and a quadratic one is not — and a
    /// connection site drawn as a visible polygon is a connection site a reviewer reports as a
    /// defect in the tessellator.
    pub fn circle(
        &mut self,
        parent: Option<NodeId>,
        centre: Pt,
        radius: f64,
        label: &'static str,
        decoration: Decoration,
    ) -> NodeId {
        /// The cubic circle constant, `4·(√2 − 1)/3`.
        const KAPPA: f64 = 0.552_284_749_83;
        let offset = radius * KAPPA;
        let (cx, cy) = (centre.x, centre.y);
        let point = |x: f64, y: f64| ScenePoint::new(x as f32, y as f32);
        let commands = vec![
            PathCommand::MoveTo(point(cx, cy - radius)),
            PathCommand::CubicTo {
                first_control: point(cx + offset, cy - radius),
                second_control: point(cx + radius, cy - offset),
                end: point(cx + radius, cy),
            },
            PathCommand::CubicTo {
                first_control: point(cx + radius, cy + offset),
                second_control: point(cx + offset, cy + radius),
                end: point(cx, cy + radius),
            },
            PathCommand::CubicTo {
                first_control: point(cx - offset, cy + radius),
                second_control: point(cx - radius, cy + offset),
                end: point(cx - radius, cy),
            },
            PathCommand::CubicTo {
                first_control: point(cx - radius, cy - offset),
                second_control: point(cx - offset, cy - radius),
                end: point(cx, cy - radius),
            },
            PathCommand::Close,
        ];
        let bounds = Rect::centred(centre, radius * 2.0);
        self.path(
            parent,
            bounds,
            commands,
            FillRule::NonZero,
            label,
            decoration,
        )
    }

    /// Rotate a node and everything under it, about the middle of its own box.
    ///
    /// # Panics
    ///
    /// Never: `node` came from this canvas, so it indexes a node this canvas holds.
    pub fn rotate(&mut self, node: NodeId, degrees: f64) {
        let rect = self.nodes[node.0].rect;
        let centre = rect.centre();
        self.nodes[node.0].transform = Some(Transform::rotation_about(
            Angle::from_degrees(degrees),
            mjx_layout::LayoutPoint::new(Emu::from_points(centre.x), Emu::from_points(centre.y)),
        ));
    }

    /// Record that `node` is grabbable, and how far past its own bounds the grab reaches.
    ///
    /// The padding comes from the input device in force, so the overlay a person judges at
    /// `input=touch` is the region a finger would actually hit.
    pub fn grab(&mut self, node: NodeId, label: impl Into<String>) {
        self.grabs.push(Grab {
            node,
            label: label.into(),
            padding: self.state.input.grab_padding(),
        });
    }

    /// Every grab region recorded, in the order it was drawn.
    #[must_use]
    pub fn grabs(&self) -> &[Grab] {
        &self.grabs
    }

    /// How many nodes have been drawn.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether nothing has been drawn.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The fragment tree, and the identifier each node became.
    ///
    /// Minted fresh on every call: the visualiser needs the tree without its overlays in order to
    /// measure them, and the tree with them in order to draw them.
    ///
    /// # Panics
    ///
    /// Never on a canvas this crate built: a parent is always pushed before its children, so the
    /// builder has already minted every parent named here.
    #[must_use]
    pub fn tree(&self) -> (FragmentTree, Vec<FragmentId>) {
        let mut builder = FragmentTreeBuilder::with_capacity(self.nodes.len());
        let mut ids: Vec<FragmentId> = Vec::with_capacity(self.nodes.len());
        for (index, node) in self.nodes.iter().enumerate() {
            let parent = node.parent.map(|at| ids[at]);
            let transform = node
                .transform
                .map_or(mjx_layout::TransformId::IDENTITY, |transform| {
                    builder.transform(transform)
                });
            let clip = node.clip.and_then(|rect| builder.clip(rect.to_layout()));
            let fragment = match node.kind {
                Kind::Box(decoration) => Fragment::Box(BoxFragment {
                    decoration: decoration.map(|handle| DecorationRef::new(handle as u64 + 1)),
                    cell: None,
                }),
                Kind::Shape(outline, decoration) => Fragment::Shape(ShapeFragment {
                    geometry: GeometryRef::new(outline as u64 + 1),
                    decoration: decoration.map(|handle| DecorationRef::new(handle as u64 + 1)),
                }),
            };
            let id = builder
                .push(
                    parent,
                    SourceRef::node(PartId::new(1), SourcePath::new(&[index as u32])),
                    node.rect.to_layout(),
                    transform,
                    clip,
                    fragment,
                )
                .expect("a canvas node");
            ids.push(id);
        }
        (builder.finish(), ids)
    }
}

/// The smallest rectangle containing every point, with a point of slack so that a stroke centred on
/// a hairline is inside the bounds a viewport cull would test.
fn bounds_of(points: &[Pt]) -> Rect {
    let Some(first) = points.first() else {
        return Rect::new(0.0, 0.0, 0.0, 0.0);
    };
    let (mut left, mut top, mut right, mut bottom) = (first.x, first.y, first.x, first.y);
    for point in points {
        left = left.min(point.x);
        top = top.min(point.y);
        right = right.max(point.x);
        bottom = bottom.max(point.y);
    }
    Rect::new(
        left - 1.0,
        top - 1.0,
        right - left + 2.0,
        bottom - top + 2.0,
    )
}

impl ResourceResolver for Canvas {
    fn decoration(&self, reference: DecorationRef) -> Option<Decoration> {
        let handle = reference.number().checked_sub(1)?;
        self.decorations.get(usize::try_from(handle).ok()?).cloned()
    }

    fn text_decoration(&self, _source: &SourceRef) -> Option<Decoration> {
        // No scene contains text; see this module's own documentation. Answering `None` keeps that
        // true rather than merely stated — a scene that grew a glyph run would draw it in
        // `mjx_scene::DEFAULT_TEXT_COLOR` and visibly change, rather than quietly acquiring a
        // dependency on an installed face.
        None
    }

    fn image(&self, _reference: mjx_layout::ImageRef) -> Option<mjx_scene::Image> {
        None
    }
}

impl GeometryProvider for Canvas {
    fn outline(&self, outline: u64, _within: SceneRect) -> Result<ResolvedOutline, SceneError> {
        let handle = outline
            .checked_sub(1)
            .and_then(|handle| usize::try_from(handle).ok())
            .and_then(|handle| self.outlines.get(handle))
            .ok_or(SceneError::UnresolvedOutline { outline })?;
        let (commands, fill_rule, label) = handle;
        let scale = self.pixels_per_point;
        let scaled = commands
            .iter()
            .map(|command| scale_command(*command, scale))
            .collect();
        Ok(ResolvedOutline {
            commands: scaled,
            fill_rule: *fill_rule,
            label: (*label).to_owned(),
            // **`Document`, never `Placeholder`.** Every plate this harness takes asserts
            // `DrawReport::placeholders == 0`; an outline answered as a stand-in would be a plate of
            // `mjx-scene`'s framed crossed rectangle rather than of the element, and the gate would
            // say so.
            provenance: OutlineProvenance::Document,
        })
    }
}

/// One path step from points into device pixels.
fn scale_command(command: PathCommand, scale: f32) -> PathCommand {
    let point = |value: ScenePoint| ScenePoint::new(value.x * scale, value.y * scale);
    match command {
        PathCommand::MoveTo(at) => PathCommand::MoveTo(point(at)),
        PathCommand::LineTo(at) => PathCommand::LineTo(point(at)),
        PathCommand::QuadraticTo { control, end } => PathCommand::QuadraticTo {
            control: point(control),
            end: point(end),
        },
        PathCommand::CubicTo {
            first_control,
            second_control,
            end,
        } => PathCommand::CubicTo {
            first_control: point(first_control),
            second_control: point(second_control),
            end: point(end),
        },
        PathCommand::Close => PathCommand::Close,
    }
}

/// Whether a display list's geometry is unresolved — the shape a scene would take if the canvas
/// stopped answering. Used by the completeness gate rather than by a scene.
#[must_use]
pub fn is_unresolved(geometry: &Geometry) -> bool {
    matches!(geometry, Geometry::Unresolved { .. })
}

// -------------------------------------------------------------------------------------------
// The palette
// -------------------------------------------------------------------------------------------

/// Every colour a scene may use, read from the generated design tokens and folded with the
/// interaction state.
///
/// # ⚠ Why no scene writes a hexadecimal colour
///
/// `docs/client-platform/CANVAS_UI_INVENTORY.md` §3 asks for *"a live token editor — every design
/// token adjustable at runtime, because the point of the audit is to tweak"*. A token editor whose
/// changes do not reach the canvas is a text editor with extra steps, so the canvas has to read the
/// tokens rather than a literal. That is the whole of why this type exists, and it is why
/// `tests/the_token_editor_writes_back.rs` can change a token and see sixty-one images move.
#[derive(Clone, PartialEq, Debug)]
pub struct Ink {
    /// The document-surface palette for the scheme in force — backdrop, page, selection, guides.
    pub document: DocumentColors,
    /// The semantic application palette for the same scheme — accent, borders, text.
    pub theme: ThemeColors,
    /// Which point of the matrix these were folded for.
    pub state: State,
}

impl Ink {
    /// The palette `tokens` gives at `state`.
    #[must_use]
    pub fn new(tokens: &Tokens, state: State) -> Self {
        Self {
            document: tokens.document.scheme(state.scheme).clone(),
            theme: tokens.theme.scheme(state.scheme).clone(),
            state,
        }
    }

    /// Which scheme this is.
    #[must_use]
    pub fn scheme(&self) -> ColorScheme {
        self.state.scheme
    }

    /// The canvas behind the page.
    #[must_use]
    pub fn backdrop(&self) -> Color {
        self.document.backdrop
    }

    /// The page itself — true white in both schemes, by the token source's own decision.
    #[must_use]
    pub fn page(&self) -> Color {
        self.document.page
    }

    /// The page's own edge.
    #[must_use]
    pub fn page_border(&self) -> Color {
        self.document.page_border
    }

    /// The colour a selection outline, a handle or an active affordance is drawn in — **the one
    /// colour the interaction axis moves**.
    ///
    /// At rest it is `document.*.selection-handle`. Under the pointer it brightens to the theme's
    /// accent, while it is dragged it darkens to the pressed accent, and disabled it goes to the
    /// secondary text colour: an inert affordance must still be visible, because a locked object
    /// that shows no handles at all is indistinguishable from one that is not selected.
    #[must_use]
    pub fn accent(&self) -> Color {
        match self.state.interaction {
            Interaction::Default => self.document.selection_handle,
            Interaction::Hover => self.theme.accent,
            Interaction::Active => self.theme.accent_pressed,
            Interaction::Focused => self.theme.accent,
            Interaction::Disabled => self.theme.text_secondary,
        }
    }

    /// The accent's quiet surface — a highlighted header, a hovered field, a shaded region.
    #[must_use]
    pub fn accent_surface(&self) -> Color {
        match self.state.interaction {
            Interaction::Disabled => with_alpha(self.theme.text_secondary, 0x24),
            _ => self.document.selection_fill,
        }
    }

    /// The selection fill **at full alpha**, to be drawn inside a group at
    /// [`Ink::selection_opacity`].
    ///
    /// # ⚠ Why a selection is a translucent *group* rather than a translucent *colour*
    ///
    /// A selection spanning three lines is three rectangles that abut, and two rectangles at
    /// eighteen percent alpha drawn one over the other are not eighteen percent — they are
    /// thirty-three, and the seam between two lines of one selection is visibly darker than the
    /// lines themselves. Drawing them opaque inside one group whose opacity is the token's alpha
    /// composites the union once and has no seam.
    ///
    /// It also makes the display list say what is true: `PushOpacity` is the command for *"this
    /// subtree is translucent as a whole"*, and an entry that declares
    /// [`crate::inventory::Draws::Translucency`] here is declaring something the painter really
    /// does rather than a group opacity of 0.999 put there to satisfy a gate.
    #[must_use]
    pub fn selection_colour(&self) -> Color {
        match self.state.interaction {
            Interaction::Disabled => with_alpha(self.theme.text_secondary, 0xff),
            _ => with_alpha(self.document.selection_fill, 0xff),
        }
    }

    /// How opaque a selection's group is — the alpha the `document.*.selection-fill` token carries.
    #[must_use]
    pub fn selection_opacity(&self) -> f32 {
        f32::from(self.document.selection_fill.alpha) / 255.0
    }

    /// The guide colour — alignment guides, snap marks, the rotation dial.
    #[must_use]
    pub fn guide(&self) -> Color {
        self.document.alignment_guide
    }

    /// Grid and ruler lines.
    #[must_use]
    pub fn grid(&self) -> Color {
        self.document.grid_line
    }

    /// A comment anchor and its connector.
    #[must_use]
    pub fn comment(&self) -> Color {
        self.document.comment_anchor
    }

    /// A tracked insertion's bar and rule.
    #[must_use]
    pub fn insertion(&self) -> Color {
        self.document.tracked_change_insert
    }

    /// A tracked deletion's bar and rule.
    #[must_use]
    pub fn deletion(&self) -> Color {
        self.document.tracked_change_delete
    }

    /// The primary text colour — what a bar standing in for a line of words is drawn in.
    #[must_use]
    pub fn text(&self) -> Color {
        self.theme.text_primary
    }

    /// The secondary text colour — a prompt, a dimmed region, a non-printing mark.
    #[must_use]
    pub fn muted(&self) -> Color {
        self.theme.text_secondary
    }

    /// An ordinary border.
    #[must_use]
    pub fn border(&self) -> Color {
        self.theme.border
    }

    /// A quiet border.
    #[must_use]
    pub fn border_subtle(&self) -> Color {
        self.theme.border_subtle
    }

    /// The colour a document object is drawn in — the thing the in-canvas UI is *about*, which must
    /// never be mistaken for the UI itself.
    #[must_use]
    pub fn object(&self) -> Color {
        self.theme.secondary_surface
    }

    /// That object's own edge.
    #[must_use]
    pub fn object_border(&self) -> Color {
        self.theme.secondary_accent
    }

    /// A raised surface — a readout badge, a margin card, a tooltip.
    ///
    /// Named after the token it answers with rather than after the concept, because
    /// `clippy::misnamed_getters` reads a method called `surface` returning `surface_raised` as a
    /// typo and is right to: the theme has both, and a badge wants the raised one.
    #[must_use]
    pub fn raised_surface(&self) -> Color {
        self.theme.surface_raised
    }

    /// A warning colour — a missing resource, an overflow, a substituted face.
    #[must_use]
    pub fn warning(&self) -> Color {
        self.theme.secondary_accent
    }

    /// How wide a stroke is, in points, given its resting width.
    ///
    /// Engaged affordances thicken by a third. **Not by a whole point**: a selection outline that
    /// jumps from one point to two under the pointer reads as a different element rather than as
    /// the same one responding.
    #[must_use]
    pub fn width(&self, resting: f64) -> f64 {
        if self.state.interaction.is_engaged() {
            resting * 4.0 / 3.0
        } else {
            resting
        }
    }

    /// How wide a drawn affordance is, in points — the input device's size, grown while engaged.
    #[must_use]
    pub fn affordance(&self) -> f64 {
        let base = self.state.input.affordance_size();
        match self.state.interaction {
            Interaction::Hover => base * 1.15,
            Interaction::Active => base * 1.3,
            _ => base,
        }
    }

    /// How far past a drawn affordance its grab region reaches, in points.
    #[must_use]
    pub fn grab_padding(&self) -> f64 {
        self.state.input.grab_padding()
    }

    /// Whether the element is inert.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.state.interaction == Interaction::Disabled
    }

    /// Whether the element holds keyboard focus.
    #[must_use]
    pub fn is_focused(&self) -> bool {
        self.state.interaction == Interaction::Focused
    }

    /// Whether the input device is a finger.
    #[must_use]
    pub fn is_touch(&self) -> bool {
        self.state.input == Input::Touch
    }
}

/// A colour at a different alpha.
#[must_use]
pub const fn with_alpha(color: Color, alpha: u8) -> Color {
    Color { alpha, ..color }
}

/// A decoration that is one solid fill.
#[must_use]
pub fn filled(color: Color) -> Decoration {
    Decoration::filled(FillStyle::Solid(color))
}

/// A stroke of `points` typographic points, in the device pixels a display list is written in.
///
/// # ⚠ The one conversion in this crate, and why it is a function rather than a habit
///
/// A [`mjx_scene::StrokeStyle`]'s width is in **device pixels** — `build_scene` passes it through
/// untouched, deliberately, because a display list is a device-space document. A scene that wrote
/// `StrokeStyle::solid(1.0, ...)` would therefore draw a one-*pixel* line at every density: crisp at
/// 1×, invisibly thin at 3×, and identical in all three. Every scene states its widths in points and
/// calls this, and `tests/the_axes_are_not_identities.rs` measures the ratio between the densities
/// and fails on any scene whose strokes did not scale.
#[must_use]
pub fn stroke(scale: f32, color: Color, points: f64) -> StrokeStyle {
    StrokeStyle::solid((points as f32) * scale, color)
}

/// A decoration that is one solid fill and one solid stroke of `points` points.
#[must_use]
pub fn filled_and_stroked(scale: f32, fill: Color, edge: Color, points: f64) -> Decoration {
    Decoration {
        fill: FillStyle::Solid(fill),
        stroke: Some(stroke(scale, edge, points)),
        ..Decoration::none()
    }
}

/// A decoration that is one solid stroke of `points` points and no fill.
#[must_use]
pub fn stroked(scale: f32, color: Color, points: f64) -> Decoration {
    Decoration {
        stroke: Some(stroke(scale, color, points)),
        ..Decoration::none()
    }
}

/// A dashed stroke of `points` points.
#[must_use]
pub fn dashed_stroke(
    scale: f32,
    color: Color,
    points: f64,
    dash: mjx_scene::DashPattern,
) -> StrokeStyle {
    let mut style = stroke(scale, color, points);
    style.dash = dash;
    style
}

/// A decoration that is one dashed stroke of `points` points and no fill.
#[must_use]
pub fn dashed(scale: f32, color: Color, points: f64, dash: mjx_scene::DashPattern) -> Decoration {
    Decoration {
        stroke: Some(dashed_stroke(scale, color, points, dash)),
        ..Decoration::none()
    }
}
