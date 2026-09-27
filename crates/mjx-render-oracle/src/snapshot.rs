//! Tiers one and two: what a fragment tree and a display list look like as text a person can read
//! and a diff can name a line of.
//!
//! # Why a text snapshot rather than a hash
//!
//! A digest of a fragment tree answers *"it changed"*, which is the least useful true statement a
//! gate can make. MJXOFF-165's requirement is stronger and specific: **a broken line break should
//! surface as a fragment diff, not a mysterious image.** So a snapshot is one line per node and one
//! line per command, in a fixed order, and a failure quotes the first line that differs together
//! with both sides of it. That is the difference between *"the tree changed"* and *"line 4: node 2's
//! rect is 152400 wide and was 101600"*.
//!
//! # Determinism
//!
//! Every number is written from an integer or from a `f32` at full precision, in tree and wire
//! order. No hash-map iteration, no addresses, no floating-point formatting that could round
//! differently — a snapshot has to be identical on every machine or it is not a baseline.
//!
//! # The two tiers are *not* two views of one thing
//!
//! Tier one is built from the [`FragmentTree`] alone and never sees a colour: a `BoxFragment`
//! carries a [`DecorationRef`](mjx_layout::DecorationRef), which is a bare number. Tier two is built
//! from the [`DisplayList`], which is where the palette's answers have been resolved into paints. A
//! change to a colour therefore moves tier two and cannot move tier one, and that asymmetry is what
//! [`crate::tiers`] localises with.

use mjx_layout::{Fragment, FragmentId, FragmentTree};
use mjx_scene::{Command, DisplayList, Geometry, Paint, ResourceIndex, SectionKind};

/// A snapshot: ordered lines, compared line by line.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Snapshot {
    lines: Vec<String>,
}

impl Snapshot {
    /// A snapshot from lines already in order.
    #[must_use]
    pub fn from_lines(lines: Vec<String>) -> Self {
        Self { lines }
    }

    /// The snapshot a committed file holds.
    ///
    /// Trailing whitespace is stripped from every line and a trailing blank line is dropped, so that
    /// an editor which adds or removes a final newline cannot expire an approval.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut lines: Vec<String> = text
            .lines()
            .map(|line| line.trim_end().to_owned())
            .collect();
        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        Self { lines }
    }

    /// The lines.
    #[must_use]
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// The text a file holds: every line, each followed by a newline.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        for line in &self.lines {
            text.push_str(line);
            text.push('\n');
        }
        text
    }

    /// The first line at which two snapshots differ, quoted from both sides, or `None` when they are
    /// the same.
    ///
    /// **One line, not a whole diff.** A snapshot of a page is hundreds of lines and a failure that
    /// printed all of them would be scrolled past; the first divergence is where a reader has to
    /// look, and every later one is usually its consequence.
    #[must_use]
    pub fn first_difference(&self, other: &Self) -> Option<String> {
        for (number, (mine, theirs)) in self.lines.iter().zip(other.lines.iter()).enumerate() {
            if mine != theirs {
                return Some(format!(
                    "line {}: expected `{theirs}`, got `{mine}`",
                    number + 1
                ));
            }
        }
        match self.lines.len().cmp(&other.lines.len()) {
            std::cmp::Ordering::Equal => None,
            std::cmp::Ordering::Greater => Some(format!(
                "line {}: the baseline ends here and this run adds `{}` ({} lines against {})",
                other.lines.len() + 1,
                self.lines.get(other.lines.len()).map_or("", String::as_str),
                self.lines.len(),
                other.lines.len()
            )),
            std::cmp::Ordering::Less => Some(format!(
                "line {}: this run ends here and the baseline has `{}` ({} lines against {})",
                self.lines.len() + 1,
                other.lines.get(self.lines.len()).map_or("", String::as_str),
                self.lines.len(),
                other.lines.len()
            )),
        }
    }
}

/// **Tier one.** One line per fragment, in depth-first tree order.
///
/// Every line carries the node's identifier, its depth, its kind, its border box in EMU, its
/// transform and clip identifiers, its source path — and the *handles* it names, never what those
/// handles resolve to.
#[must_use]
pub fn fragments(tree: &FragmentTree) -> Snapshot {
    let mut lines = Vec::with_capacity(tree.len() + 2);
    lines.push(format!(
        "fragment-tree nodes={} roots={}",
        tree.len(),
        tree.roots().len()
    ));

    // An explicit stack, never recursion: a fragment tree's depth is a document's, and `mjx-layout`
    // and `mjx-scene` both walk theirs this way for the same reason.
    let mut stack: Vec<(FragmentId, usize)> = Vec::new();
    for root in tree.roots().iter().rev() {
        stack.push((*root, 0));
    }
    while let Some((id, depth)) = stack.pop() {
        let Some(node) = tree.node(id) else { continue };
        let rect = node.rect();
        let path: Vec<String> = node
            .source()
            .path()
            .segments()
            .iter()
            .map(u32::to_string)
            .collect();
        let transform = format!("{:?}", node.transform());
        let clip = node
            .clip()
            .map_or_else(|| "none".to_owned(), |clip| format!("{clip:?}"));
        let detail = detail(node.fragment());
        let path = path.join("/");
        lines.push(format!(
            "{:indent$}#{} {} rect=[{},{},{},{}] transform={transform} clip={clip} part={} \
             path=/{path} {detail}",
            "",
            id.index(),
            node.fragment().kind_name(),
            rect.left.emu(),
            rect.top.emu(),
            rect.right.emu(),
            rect.bottom.emu(),
            node.source().part().number(),
            indent = depth * 2,
        ));
        let mut children: Vec<FragmentId> = tree.children(id).collect();
        children.reverse();
        for child in children {
            stack.push((child, depth + 1));
        }
    }
    Snapshot::from_lines(lines)
}

/// The kind-specific half of a fragment's line.
fn detail(fragment: &Fragment) -> String {
    match fragment {
        Fragment::Box(box_fragment) => format!(
            "decoration={} cell={}",
            box_fragment
                .decoration
                .map_or(-1i128, |d| i128::from(d.number())),
            box_fragment.cell.map_or_else(
                || "none".to_owned(),
                |cell| format!(
                    "{}:{}+{}x{}",
                    cell.column, cell.row, cell.column_span, cell.row_span
                )
            )
        ),
        Fragment::Line(line) => format!(
            "baseline={} ascent={} descent={} direction={:?} hanging={}",
            line.baseline.emu(),
            line.ascent.emu(),
            line.descent.emu(),
            line.base_direction,
            line.hanging_width.emu()
        ),
        Fragment::GlyphRun(run) => format!(
            "face={} origin=[{},{}] direction={:?} level={:?} glyphs={}",
            run.face.as_u32(),
            run.origin.x.emu(),
            run.origin.y.emu(),
            run.direction,
            run.level,
            run.run.glyphs().len()
        ),
        Fragment::Image(image) => format!(
            "image={} crop={}",
            image.image.number(),
            image.crop.map_or_else(
                || "none".to_owned(),
                |crop| format!("{},{},{},{}", crop.left, crop.top, crop.right, crop.bottom)
            )
        ),
        Fragment::Shape(shape) => format!(
            "geometry={} decoration={}",
            shape.geometry.number(),
            shape.decoration.map_or(-1i128, |d| i128::from(d.number()))
        ),
        Fragment::Table(table) => format!(
            "columns={} rows={}..{} headers={} continued={} continues={}",
            table.columns,
            table.rows.start,
            table.rows.end,
            table.header_rows,
            table.continued_from_previous_page,
            table.continues_on_next_page
        ),
    }
}

/// **Tier two.** The page, every resource table's size, and one line per command with the resources
/// it names *resolved*.
///
/// Resolved rather than left as indices on purpose: `FillPath geometry=3 paint=1` would be identical
/// whatever colour paint one is, and the tier that is supposed to catch a scene-building change
/// would not catch a recolour. So the paint's own value is on the line.
#[must_use]
pub fn commands(list: &DisplayList) -> Snapshot {
    let (width, height) = list.page_size();
    let mut lines = vec![format!(
        "display-list page={width}x{height} scale={} bytes={}",
        list.device_scale().pixels_per_point(),
        list.byte_len()
    )];
    for kind in SectionKind::ALL {
        let count = list.record_count(kind);
        if count > 0 {
            lines.push(format!("section {kind} count={count}"));
        }
    }
    for (number, command) in list.commands().enumerate() {
        lines.push(format!("{number:04} {}", describe(list, &command)));
    }
    Snapshot::from_lines(lines)
}

/// One command, with whatever it names spelled out.
fn describe(list: &DisplayList, command: &Command) -> String {
    match command {
        Command::PushTransform(index) => match list.transform(*index) {
            Some(transform) => format!(
                "push-transform [{} {} {} {} {} {}]",
                transform.scale_x,
                transform.shear_y,
                transform.shear_x,
                transform.scale_y,
                transform.translate_x,
                transform.translate_y
            ),
            None => format!("push-transform <{} is not in the table>", index.index()),
        },
        Command::PushClip(index) => match list.clip(*index) {
            Some(clip) => format!(
                "push-clip [{},{},{},{}] geometry={}",
                clip.bounds.left,
                clip.bounds.top,
                clip.bounds.right,
                clip.bounds.bottom,
                clip.geometry
                    .map_or_else(|| "none".to_owned(), |index| index.index().to_string())
            ),
            None => format!("push-clip <{} is not in the table>", index.index()),
        },
        Command::PushOpacity(opacity) => format!("push-opacity {opacity}"),
        Command::PushEffect(index) => format!("push-effect {}", index.index()),
        Command::Pop => "pop".to_owned(),
        Command::FillPath { geometry, paint } => format!(
            "fill {} with {}",
            geometry_of(list, *geometry),
            paint_of(list, *paint)
        ),
        Command::StrokePath { geometry, stroke } => format!(
            "stroke {} with {}",
            geometry_of(list, *geometry),
            match list.stroke(*stroke) {
                Some(stroke) => format!(
                    "width={} cap={:?} join={:?} dash={:?} alignment={:?} compound={:?} paint={}",
                    stroke.width,
                    stroke.cap,
                    stroke.join,
                    stroke.dash,
                    stroke.alignment,
                    stroke.compound,
                    paint_of(list, stroke.paint)
                ),
                None => format!("<stroke {} is not in the table>", stroke.index()),
            }
        ),
        Command::DrawGlyphs { run, paint } => {
            format!("glyphs run={} with {}", run.index(), paint_of(list, *paint))
        }
        Command::DrawImage { image, destination } => format!(
            "image {} into [{},{},{},{}]",
            image.index(),
            destination.left,
            destination.top,
            destination.right,
            destination.bottom
        ),
    }
}

/// One geometry, described by what it is rather than by its index.
fn geometry_of(list: &DisplayList, index: ResourceIndex) -> String {
    match list.geometry(index) {
        Some(Geometry::Rectangle(rect)) => format!(
            "rect[{},{},{},{}]",
            rect.left, rect.top, rect.right, rect.bottom
        ),
        Some(Geometry::Path {
            commands,
            fill_rule,
            bounds,
        }) => format!(
            "path[steps={} rule={fill_rule:?} bounds={},{},{},{}]",
            commands.len(),
            bounds.left,
            bounds.top,
            bounds.right,
            bounds.bottom
        ),
        Some(Geometry::Unresolved { outline, bounds }) => format!(
            "outline[{outline:#x} at {},{},{},{}]",
            bounds.left, bounds.top, bounds.right, bounds.bottom
        ),
        None => format!("<geometry {} is not in the table>", index.index()),
    }
}

/// One paint, described by its value.
fn paint_of(list: &DisplayList, index: ResourceIndex) -> String {
    match list.paint(index) {
        Some(Paint::Solid(color)) => format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            color.red, color.green, color.blue, color.alpha
        ),
        Some(Paint::Gradient(gradient)) => match list.gradient(gradient) {
            Some(gradient) => {
                let stops: Vec<String> = gradient
                    .stops
                    .iter()
                    .map(|stop| {
                        format!(
                            "{}@#{:02x}{:02x}{:02x}{:02x}",
                            stop.position_in_ten_thousandths,
                            stop.color.red,
                            stop.color.green,
                            stop.color.blue,
                            stop.color.alpha
                        )
                    })
                    .collect();
                format!(
                    "gradient[{:?} angle={} stops={}]",
                    gradient.kind,
                    gradient.angle,
                    stops.join(",")
                )
            }
            None => format!("<gradient {} is not in the table>", gradient.index()),
        },
        Some(Paint::Pattern {
            preset,
            foreground,
            background,
        }) => format!(
            "pattern[{preset:?} #{:02x}{:02x}{:02x} on #{:02x}{:02x}{:02x}]",
            foreground.red,
            foreground.green,
            foreground.blue,
            background.red,
            background.green,
            background.blue
        ),
        Some(Paint::Image(image)) => format!("image[{}]", image.index()),
        None => format!("<paint {} is not in the table>", index.index()),
    }
}
