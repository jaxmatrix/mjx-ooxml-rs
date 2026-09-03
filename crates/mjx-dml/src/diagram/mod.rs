//! DrawingML Diagrams (SmartArt, the `dgm:` namespace, `dml-diagram.xsd`) — the four parts a
//! `p:graphicFrame`'s [`DiagramRelationshipIds`](https://docs.rs/mjx-pptx) names: data, layout
//! definition, style and colours.
//!
//! # Where the line falls
//!
//! A SmartArt diagram is two things bolted together: a **graph** (which nodes exist, how they
//! connect, what each one says) and a **layout algorithm** (where a consumer draws each node given
//! that graph). This module models the graph and the algorithm's *markup* — every element and
//! attribute `dml-diagram.xsd` declares reads back typed or, for a handful of externally-defined
//! DrawingML formatting groups this crate does not yet model standalone (`spPr`, `style`, `txPr`,
//! `bg`, `whole`, `scene3d`, `sp3d`), preserves verbatim. It does **not** run a `dgm:layoutDef` to
//! compute where a consumer would draw each shape — that is a rendering concern, deliberately out of
//! scope; see `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` for the full modelled-vs-preserved
//! accounting, including the four "header"/"header list" complex types (`colorsDefHdr`,
//! `colorsDefHdrLst`, `styleDefHdr`, `styleDefHdrLst`, and their `layoutDef` equivalents), which
//! belong to a diagram-gallery catalog part this project neither authors nor reads and so are not
//! given their own Rust type.
//!
//! # The load-bearing part
//!
//! [`data`] is the point-and-connection graph: [`DataModel`], [`PointList`]/[`Point`],
//! [`ConnectionList`]/[`Connection`]. A diagram external callers write with `add_diagram`
//! (`mjx-pptx`) reads back through this module as that graph, not as an opaque byte blob — see
//! `crates/mjx-pptx/tests/diagram_read_back.rs`.
//!
//! # Two symbol tables, one wire shape
//!
//! `dml-diagram.xsd` declares the same `<title lang="" val=""/>` / `<desc .../>` / `<cat .../>` /
//! `<catLst>...</catLst>` wire shapes **three times** under three different complex-type symbols —
//! `CT_CTName`/`CT_Name`/`CT_SDName` for the colours/layout/style parts respectively, and likewise
//! for `CT_*Description`/`CT_*Category`/`CT_*Categories`. All three name the exact same element
//! locally (`title`, `desc`, `cat`, `catLst`) with an identical content model, so this module gives
//! them **one** Rust type each ([`DiagramTitle`], [`DiagramDescription`], [`DiagramCategory`],
//! [`DiagramCategoryList`]) rather than three redundant near-duplicates — see [`common`].
//!
//! # Not flattened at the crate root
//!
//! Unlike this crate's other modules, `diagram`'s contents are not re-exported at `mjx_dml::*`: its
//! graph vocabulary (`Point`, `Connection`, …) collides by name with [`crate::geometry`]'s own
//! `Point` and would either shadow it or force a rename nobody asked for. A caller reaches this
//! module's types as `mjx_dml::diagram::Point`, the same way it already reaches, say,
//! `mjx_dml::table::TableCell`.

pub mod colors;
pub mod common;
pub mod data;
pub mod layout;
pub mod style;
mod support;

pub use colors::{ColorList, ColorTransform, StyleLabelColors};
pub use common::{DiagramCategory, DiagramCategoryList, DiagramDescription, DiagramTitle};
pub use data::{
    Connection, ConnectionList, ConnectionListContent, DataModel, DataModelContent,
    ElementPropertySet, ElementPropertySetContent, LayoutVariables, Point, PointContent, PointList,
    PointListContent,
};
pub use layout::{
    Algorithm, AlgorithmContent, AlgorithmParameter, Choose, ChooseContent, Constraint,
    ConstraintList, ConstraintListContent, ForEachIterator, LayoutBranchContent, LayoutCondition,
    LayoutDefinition, LayoutDefinitionContent, LayoutNode, LayoutNodeContent, LayoutOtherwise,
    LayoutShape, LayoutShapeAdjustment, LayoutShapeAdjustmentList,
    LayoutShapeAdjustmentListContent, LayoutShapeContent, NumericRule, PresentationOf, RuleList,
    RuleListContent, SampleData,
};
pub use style::{StyleDefinition, StyleDefinitionContent, StyleLabel};

pub use mjx_ooxml_types::diagram::{
    AlgorithmType, ArrowheadStyle, AutoTextRotation, AxisType, BendPoint, BoolOperator, Breakpoint,
    CenterShapeMapping, ChildAlignment, ChildDirection, ChildOrderType, ClrAppMethod,
    ConnectionType, ConnectorDimension, ConnectorPoint, ConnectorRouting, ConstraintRelationship,
    ConstraintType, ContinueDirection, DiagramHorizontalAlignment, DiagramTextAlignment,
    DiagramTextFlowOrigin, ElementType, FallbackDimension, FlowDirection, FunctionOperator,
    FunctionType, GrowDirection, HierarchyAlignment, HierarchyBranchStyle, HueDirection,
    LayoutVerticalAlignment, LevelAnimation, LinearDirection, NodeHorizontalAlignment,
    NodeVerticalAlignment, OneByOneAnimation, OutputShapeType, ParameterId, PointType,
    PyramidAccentPosition, PyramidAccentTextMargin, ResizeHandleBehavior, RotationPath,
    SecondaryChildAlignment, SecondaryLinearDirection, StartingElement, TextAnchorHorizontal,
    TextAnchorVertical, TextBlockDirection, TraversalDirection,
};
