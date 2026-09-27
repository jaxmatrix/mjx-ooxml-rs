//! SmartArt: turning a `dgm:dataModel`'s graph into positioned shapes.
//!
//! # The finding MJXOFF-178 was asked to report
//!
//! **`mjx-dml`'s `diagram/` module models the whole of `dml-diagram.xsd` and evaluates none of it.**
//! Every element of all four parts reads back typed — the point-and-connection graph (`DataModel`,
//! `PointList`/`Point`, `ConnectionList`/`Connection`), the whole recursive layout-definition tree
//! (`LayoutNode`, `Algorithm` with all ten of `ST_AlgorithmType`, `AlgorithmParameter`,
//! `Constraint`, `NumericRule`, `ForEachIterator`, `Choose`/`LayoutCondition`/`LayoutOtherwise`,
//! `PresentationOf`, `LayoutShape`), the style definition and the colour transforms — and
//! `crates/mjx-dml/src/diagram/layout.rs` states in its own module docs that *"what is **not** here
//! is a layout engine — code that walks this tree and computes where each point's shape ends up"*.
//! So the markup half is complete and none of the algorithm half existed before this crate. There is
//! nothing here to consume and nothing here to duplicate: this module is the first evaluator.
//!
//! # What this evaluator does, exactly
//!
//! It reads the **data model** — the graph — and lays it out with one of **three** of the ten
//! algorithms, chosen by the `dgm:alg@type` on the layout definition's root node:
//!
//! | `dgm:alg@type` | Rust | What it does here |
//! |---|---|---|
//! | `lin` | [`AlgorithmType::Linear`] | One row (or column) of equal cells. |
//! | `hierRoot`, `hierChild` | [`AlgorithmType::HierarchyRoot`], [`HierarchyChild`](AlgorithmType::HierarchyChild) | A top-down tree: one row per depth, siblings sharing their parent's span. |
//! | `cycle` | [`AlgorithmType::Cycle`] | Equal cells around a circle, first at twelve o'clock. |
//!
//! **The other seven are refused by name**, as [`ChartLayoutError::DiagramAlgorithmNotEvaluated`]:
//! `composite`, `conn`, `pyra`, `sp`, `snake`, and the two that are not algorithms at all. Standing
//! in for a pyramid with a linear row would put a customer's diagram on a screen in a shape their
//! file does not describe, which is worse than drawing nothing and saying so.
//!
//! # And what it deliberately does not do
//!
//! * **`dgm:constrLst` is not evaluated.** ECMA-376 Part 1 §21.4.2.11's constraint language is
//!   sixty-odd constraint types with reference resolution across nodes, and none of it is read here:
//!   spacing comes from this module's own constants. A diagram whose file tightens its gutters will
//!   be laid out with this engine's gutters instead.
//! * **`dgm:ruleLst`, `dgm:choose`, `dgm:forEach` and `dgm:presOf` are not evaluated either.** The
//!   layout definition is consulted for **one** thing — which algorithm its root names — and the
//!   shapes come from the data model's own `parOf` connections. That is why a diagram whose layout
//!   definition branches on a condition is laid out as though it had not.
//! * **Text is not laid out inside a node.** A node is a rectangle; what its `dgm:t` says reaches the
//!   caller on [`DiagramNode::text`], and setting it is the host's.
//!
//! Together those make this a **hierarchy, linear and cycle evaluator**, not a SmartArt engine. The
//! boundary is drawn here rather than discovered later.

use mjx_dml::diagram::data::{DataModel, Point};
use mjx_dml::diagram::LayoutDefinition;
use mjx_layout::{LayoutRect, LayoutSize};
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_core::Interner;
use mjx_ooxml_types::diagram::{AlgorithmType, ConnectionType, PointType};

use crate::error::ChartLayoutError;

/// One node of a laid-out diagram.
#[derive(Clone, Debug, PartialEq)]
pub struct DiagramNode {
    /// The point's `dgm:pt@modelId`, so a caller can find the point this came from.
    pub model_id: String,
    /// Its text, or `None` when it carries none.
    pub text: Option<String>,
    /// How deep it sits: the root is 0.
    pub depth: usize,
    /// Where it goes.
    pub rect: LayoutRect,
}

/// A connector between two laid-out nodes.
#[derive(Clone, Debug, PartialEq)]
pub struct DiagramEdge {
    /// The index of the parent in [`DiagramLayout::nodes`].
    pub from: usize,
    /// The index of the child.
    pub to: usize,
}

/// A laid-out diagram.
#[derive(Clone, Debug, PartialEq)]
pub struct DiagramLayout {
    /// The algorithm that produced it.
    pub algorithm: AlgorithmType,
    /// Its nodes, in a breadth-first walk from the root.
    pub nodes: Vec<DiagramNode>,
    /// Its connectors.
    pub edges: Vec<DiagramEdge>,
}

impl DiagramLayout {
    /// The node whose `modelId` is `id`.
    #[must_use]
    pub fn node(&self, id: &str) -> Option<&DiagramNode> {
        self.nodes.iter().find(|node| node.model_id == id)
    }

    /// The nodes at one depth, in order.
    pub fn at_depth(&self, depth: usize) -> impl Iterator<Item = &DiagramNode> {
        self.nodes.iter().filter(move |node| node.depth == depth)
    }
}

/// The gap between two sibling nodes, as a fraction of the space they share.
///
/// `EngineDerived:` a fifth. `dgm:constrLst` states the real gutters and this engine does not read
/// them (see the module docs), so one number stands for all of them and is stated once here.
const GUTTER: f64 = 0.2;

/// Lays a diagram out inside `frame`.
///
/// `definition` is the `dgm:layoutDef` part; only its root node's `dgm:alg@type` is read. Pass `None`
/// for a diagram whose layout part is absent or unreadable, and the hierarchy algorithm is used —
/// which is what a `parOf` graph with depth in it is.
///
/// # Errors
/// [`ChartLayoutError::DiagramAlgorithmNotEvaluated`] when the definition names one of the seven
/// algorithms this engine does not evaluate, and [`ChartLayoutError::DiagramHasNoPoints`] for a data
/// model with no drawable point in it.
pub fn lay_out_diagram(
    data: &DataModel,
    definition: Option<&LayoutDefinition>,
    interner: &Interner,
    frame: LayoutRect,
) -> Result<DiagramLayout, ChartLayoutError> {
    let algorithm = match definition.and_then(|definition| root_algorithm(definition, interner)) {
        Some(AlgorithmType::Linear) => AlgorithmType::Linear,
        Some(AlgorithmType::Cycle) => AlgorithmType::Cycle,
        Some(AlgorithmType::HierarchyRoot | AlgorithmType::HierarchyChild) | None => {
            AlgorithmType::HierarchyRoot
        }
        Some(other) => {
            return Err(ChartLayoutError::DiagramAlgorithmNotEvaluated {
                algorithm: other.to_wire(),
            })
        }
    };

    let graph = Graph::read(data, interner);
    if graph.nodes.is_empty() {
        return Err(ChartLayoutError::DiagramHasNoPoints);
    }

    let mut nodes = Vec::with_capacity(graph.nodes.len());
    match algorithm {
        AlgorithmType::Linear => linear(&graph, frame, &mut nodes),
        AlgorithmType::Cycle => cycle(&graph, frame, &mut nodes),
        _ => hierarchy(&graph, frame, &mut nodes),
    }
    Ok(DiagramLayout {
        algorithm,
        nodes,
        edges: graph.edges.clone(),
    })
}

/// The algorithm the definition's root node names, or `None` when it names none.
fn root_algorithm(definition: &LayoutDefinition, interner: &Interner) -> Option<AlgorithmType> {
    definition
        .root()?
        .algorithms()
        .find_map(|algorithm| algorithm.algorithm(interner).ok())
}

/// The graph a data model describes, with its points ordered and its parents resolved.
struct Graph {
    /// Every drawable point, in breadth-first order from the roots.
    nodes: Vec<GraphNode>,
    /// Parent-to-child edges, as indices into `nodes`.
    edges: Vec<DiagramEdge>,
}

/// One point of the graph.
struct GraphNode {
    model_id: String,
    text: Option<String>,
    depth: usize,
    children: Vec<usize>,
}

impl Graph {
    /// Reads the point-and-connection graph.
    ///
    /// Only `node` and `asst` points are drawable: a `pres` point is the *presentation* graph the
    /// layout definition builds, a `doc` point is the document root, and the two transition kinds are
    /// connectors rather than shapes. Drawing a `pres` point would draw the diagram twice.
    fn read(data: &DataModel, interner: &Interner) -> Self {
        let points: Vec<&Point> = data
            .points()
            .map(|list| {
                list.points()
                    .filter(|point| {
                        matches!(
                            point.point_type(interner),
                            Ok(PointType::Node | PointType::Assistant)
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let identifiers: Vec<String> = points
            .iter()
            .filter_map(|point| point.model_id(interner).ok().map(|id| id.into_owned()))
            .collect();

        // `parOf` connections, ordered by `srcOrd`, are the tree. Every other connection type
        // describes the presentation graph rather than the data one.
        let mut links: Vec<(String, String, u32)> = Vec::new();
        if let Some(list) = data.connections() {
            for connection in list.connections() {
                if connection.connection_type(interner) != Ok(ConnectionType::ParentOf) {
                    continue;
                }
                let (Ok(source), Ok(destination)) = (
                    connection.source_id(interner),
                    connection.destination_id(interner),
                ) else {
                    continue;
                };
                links.push((
                    source.into_owned(),
                    destination.into_owned(),
                    connection.source_order(interner).unwrap_or(0),
                ));
            }
        }
        links.sort_by_key(|(_, _, order)| *order);

        let index_of = |id: &str| identifiers.iter().position(|known| known == id);
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); identifiers.len()];
        let mut has_parent = vec![false; identifiers.len()];
        for (source, destination, _) in &links {
            let (Some(from), Some(to)) = (index_of(source), index_of(destination)) else {
                continue;
            };
            if from == to || has_parent[to] {
                // A point with two parents is not a tree, and a self-parent is a cycle. Both appear
                // in files that have been round-tripped through a repair tool; keeping the first
                // edge is what stops the walk below from never terminating.
                continue;
            }
            has_parent[to] = true;
            children[from].push(to);
        }

        // Breadth-first from every point with no parent, which is normally one.
        let mut order: Vec<usize> = Vec::with_capacity(identifiers.len());
        let mut depth = vec![0usize; identifiers.len()];
        let mut queue: std::collections::VecDeque<usize> = (0..identifiers.len())
            .filter(|index| !has_parent[*index])
            .collect();
        let mut seen = vec![false; identifiers.len()];
        for index in &queue {
            seen[*index] = true;
        }
        // A `parOf` graph that is a *cycle* has no point without a parent, so the queue above starts
        // empty and every node would be lost. A file round-tripped through a repair tool can carry
        // one. So the walk restarts from the first point it has not reached, as many times as it
        // takes: every point is placed, and the cycle is broken at wherever the restart entered it.
        let mut next_unseen = 0usize;
        loop {
            while let Some(index) = queue.pop_front() {
                order.push(index);
                for child in &children[index] {
                    if seen[*child] {
                        continue;
                    }
                    seen[*child] = true;
                    depth[*child] = depth[index] + 1;
                    queue.push_back(*child);
                }
            }
            while next_unseen < seen.len() && seen[next_unseen] {
                next_unseen += 1;
            }
            let Some(restart) = (next_unseen < seen.len()).then_some(next_unseen) else {
                break;
            };
            seen[restart] = true;
            queue.push_back(restart);
        }

        let position: Vec<Option<usize>> = {
            let mut positions = vec![None; identifiers.len()];
            for (at, index) in order.iter().enumerate() {
                positions[*index] = Some(at);
            }
            positions
        };
        let nodes: Vec<GraphNode> = order
            .iter()
            .map(|index| GraphNode {
                model_id: identifiers[*index].clone(),
                text: points
                    .get(*index)
                    .and_then(|point| point.text_content())
                    .filter(|text| !text.is_empty()),
                depth: depth[*index],
                children: children[*index]
                    .iter()
                    .filter_map(|child| position[*child])
                    .collect(),
            })
            .collect();
        let edges = nodes
            .iter()
            .enumerate()
            .flat_map(|(from, node)| {
                node.children
                    .iter()
                    .map(move |to| DiagramEdge { from, to: *to })
            })
            .collect();
        Self { nodes, edges }
    }
}

/// `lin`: one row of equal cells, in graph order.
fn linear(graph: &Graph, frame: LayoutRect, out: &mut Vec<DiagramNode>) {
    let count = graph.nodes.len().max(1);
    let width = frame.right - frame.left;
    let cell = width.divided_by(i64::try_from(count).unwrap_or(1));
    let gutter = cell.scaled_by(GUTTER / 2.0);
    for (index, node) in graph.nodes.iter().enumerate() {
        let left = frame.left + cell.times(i64::try_from(index).unwrap_or(0));
        out.push(DiagramNode {
            model_id: node.model_id.clone(),
            text: node.text.clone(),
            depth: node.depth,
            rect: LayoutRect::from_edges(
                left + gutter,
                frame.top,
                left + cell - gutter,
                frame.bottom,
            ),
        });
    }
}

/// `cycle`: equal cells around a circle, the first at twelve o'clock and the rest clockwise.
fn cycle(graph: &Graph, frame: LayoutRect, out: &mut Vec<DiagramNode>) {
    let count = graph.nodes.len().max(1);
    let centre_x = (frame.left + frame.right).divided_by(2);
    let centre_y = (frame.top + frame.bottom).divided_by(2);
    let span = (frame.right - frame.left).minimum(frame.bottom - frame.top);
    // The ring's radius is a third of the frame, which leaves room for a node either side of it.
    let radius = span.scaled_by(1.0 / 3.0);
    let size = LayoutSize::new(span.scaled_by(0.28), span.scaled_by(0.28 * (1.0 - GUTTER)));
    for (index, node) in graph.nodes.iter().enumerate() {
        let angle = std::f64::consts::TAU * index as f64 / count as f64;
        let x = centre_x + radius.scaled_by(angle.sin());
        let y = centre_y - radius.scaled_by(angle.cos());
        out.push(DiagramNode {
            model_id: node.model_id.clone(),
            text: node.text.clone(),
            depth: node.depth,
            rect: LayoutRect::from_edges(
                x - size.width.divided_by(2),
                y - size.height.divided_by(2),
                x + size.width.divided_by(2),
                y + size.height.divided_by(2),
            ),
        });
    }
}

/// `hierRoot`/`hierChild`: one row per depth, each node sharing its parent's horizontal span with
/// its siblings **in proportion to how many leaves each of them carries**.
///
/// Splitting a parent's span equally between its children is what makes a three-level tree with
/// uneven branching look wrong: a child with four grandchildren and a child with one would get the
/// same width, and the four would be crushed into a quarter of the row while the one floated in
/// space. Weighting by leaf count is what a real organisation chart does, and it is why
/// `tests/a_hierarchy_is_laid_out.rs` uses uneven branching — an even tree lays out identically under
/// either rule and would prove nothing.
fn hierarchy(graph: &Graph, frame: LayoutRect, out: &mut Vec<DiagramNode>) {
    let depth = graph.nodes.iter().map(|node| node.depth).max().unwrap_or(0) + 1;
    let row = (frame.bottom - frame.top).divided_by(i64::try_from(depth).unwrap_or(1));
    let leaves = leaf_counts(graph);

    // Every node's horizontal span, filled in from the roots down.
    let mut spans: Vec<(Emu, Emu)> = vec![(frame.left, frame.right); graph.nodes.len()];
    let roots: Vec<usize> = (0..graph.nodes.len())
        .filter(|index| graph.nodes[*index].depth == 0)
        .collect();
    distribute(graph, &leaves, &roots, frame.left, frame.right, &mut spans);
    for index in 0..graph.nodes.len() {
        let children = graph.nodes[index].children.clone();
        let (low, high) = spans[index];
        distribute(graph, &leaves, &children, low, high, &mut spans);
    }

    for (index, node) in graph.nodes.iter().enumerate() {
        let (low, high) = spans[index];
        let gutter = (high - low).scaled_by(GUTTER / 2.0);
        let top = frame.top + row.times(i64::try_from(node.depth).unwrap_or(0));
        let vertical = row.scaled_by(GUTTER / 2.0);
        out.push(DiagramNode {
            model_id: node.model_id.clone(),
            text: node.text.clone(),
            depth: node.depth,
            rect: LayoutRect::from_edges(
                low + gutter,
                top + vertical,
                high - gutter,
                top + row - vertical,
            ),
        });
    }
}

/// Splits `low ..= high` between `members`, in proportion to how many leaves each carries.
fn distribute(
    graph: &Graph,
    leaves: &[usize],
    members: &[usize],
    low: Emu,
    high: Emu,
    spans: &mut [(Emu, Emu)],
) {
    let total: usize = members
        .iter()
        .map(|index| leaves.get(*index).copied().unwrap_or(1).max(1))
        .sum();
    if total == 0 || members.is_empty() {
        return;
    }
    let width = high - low;
    let mut cursor = low;
    for index in members {
        let weight = leaves.get(*index).copied().unwrap_or(1).max(1);
        let share = width.scaled_by(weight as f64 / total as f64);
        if let Some(slot) = spans.get_mut(*index) {
            *slot = (cursor, cursor + share);
        }
        cursor += share;
        let _ = graph;
    }
}

/// How many leaves each node of the graph carries, counted from the deepest node upward.
///
/// The walk goes in reverse graph order, which is a reverse breadth-first order and therefore visits
/// every child before its parent — so one pass is enough and no recursion is needed, which is what
/// keeps a diagram with a thousand nodes from a stack overflow.
fn leaf_counts(graph: &Graph) -> Vec<usize> {
    let mut counts = vec![0usize; graph.nodes.len()];
    for index in (0..graph.nodes.len()).rev() {
        let children = &graph.nodes[index].children;
        counts[index] = if children.is_empty() {
            1
        } else {
            children
                .iter()
                .map(|child| counts.get(*child).copied().unwrap_or(1))
                .sum()
        };
    }
    counts
}
