//! `dgm:layoutDef` (`CT_DiagramDefinition`) — the Diagram Layout Definition part: the algorithm
//! tree that positions a diagram's points.
//!
//! # The markup is modeled; running it is not
//!
//! Every element the schema declares here is typed: the recursive [`LayoutNode`] tree, its
//! [`Algorithm`]s and their [`AlgorithmParameter`]s, its [`Constraint`]s and [`NumericRule`]s, its
//! conditional [`Choose`]/[`LayoutCondition`]/[`LayoutOtherwise`]. What is **not** here is a layout
//! *engine* — code that walks this tree and computes where each point's shape ends up. That is a
//! rendering concern, ECMA-376 Part 1 §21.4's algorithms are individually intricate (ten of them,
//! several parameterised by more than a dozen constraint types), and no part of this project renders
//! a slide yet. See `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` for where this line is drawn
//! and why.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{
    Enumeration, FromXml as _, Interner, Number, RawAttribute, RawName, RawNode, Text,
};
use mjx_ooxml_types::{diagram, support::OnOff};

use super::common::{DiagramCategoryList, DiagramDescription, DiagramTitle};
use super::data::LayoutVariables;
use super::support::dgm_name;
use crate::build::fidelity_element_impls;

use diagram::{
    AlgorithmType, BoolOperator, ConstraintRelationship, ConstraintType, ElementType, ParameterId,
};

// ---------------------------------------------------------------------------------------------
// dgm:param (CT_Parameter) and dgm:alg (CT_Algorithm)
// ---------------------------------------------------------------------------------------------

/// `dgm:param` (`CT_Parameter`) — one named argument to an [`Algorithm`]. ECMA-376 Part 1
/// §21.4.2.30 *CT_Parameter*.
///
/// `@type` ([`ParameterId`]) names *which* parameter this is; `@val` is its value. The schema types
/// `@val` as `ST_ParameterVal`, a union of over thirty simple types (plus `xsd:int`/`double`/
/// `boolean`/`string`) whose legal shape depends on `@type` — deciding *which* member a given
/// `@type` expects is exactly the algorithm-interpretation this crate does not do (see the
/// [module docs](self)), so `@val` is kept as its wire string rather than parsed into one of the
/// union's thirty-odd members.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "type", codec = Enumeration<ParameterId>, accessor = parameter, required))]
#[xml(attribute(local = "val", codec = Text, accessor = value, required))]
pub struct AlgorithmParameter {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}
fidelity_element_impls!(AlgorithmParameter);

impl AlgorithmParameter {
    /// A fresh `dgm:param` naming `parameter` and carrying `value` as its wire string.
    #[must_use]
    pub fn new(interner: &mut Interner, parameter: ParameterId, value: &str) -> Self {
        let mut element = Self {
            name: dgm_name(interner, "param"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        element.set_parameter(interner, parameter);
        element.set_value(interner, value);
        element
    }
}

/// One ordered child of an [`Algorithm`]: a typed [`AlgorithmParameter`], or an opaque node
/// (`dgm:extLst`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlgorithmContent {
    /// A parameter (`dgm:param`).
    Parameter(AlgorithmParameter),
    /// Any other child — `dgm:extLst`, whitespace, or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `dgm:alg` (`CT_Algorithm`) — which of the ten layout algorithms a [`LayoutNode`] runs
/// ([`AlgorithmType`]), and its parameters. ECMA-376 Part 1 §21.4.2.2 *CT_Algorithm*.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_DIAGRAM)]
#[xml(attribute(local = "type", codec = Enumeration<AlgorithmType>, accessor = algorithm, required))]
#[xml(attribute(local = "rev", codec = Number<u32>, accessor = revision))]
pub struct Algorithm {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "param", variant = Parameter, ty = AlgorithmParameter))]
    content: Vec<AlgorithmContent>,
}

impl Algorithm {
    /// A fresh `dgm:alg` running `algorithm`, with no parameters.
    #[must_use]
    pub fn new(interner: &mut Interner, algorithm: AlgorithmType) -> Self {
        let mut element = Self {
            name: dgm_name(interner, "alg"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        element.set_algorithm(interner, algorithm);
        element
    }

    /// The algorithm's parameters, in order (opaque children are skipped).
    pub fn parameters(&self) -> impl Iterator<Item = &AlgorithmParameter> {
        self.content.iter().filter_map(|item| match item {
            AlgorithmContent::Parameter(parameter) => Some(parameter),
            AlgorithmContent::Raw(_) => None,
        })
    }

    /// Appends `parameter` at the end of the algorithm's parameter list.
    pub fn push_parameter(&mut self, parameter: AlgorithmParameter) {
        self.content.push(AlgorithmContent::Parameter(parameter));
        self.empty = false;
    }
}

// ---------------------------------------------------------------------------------------------
// dgm:adj (CT_Adj) and dgm:adjLst (CT_AdjLst)
// ---------------------------------------------------------------------------------------------

/// `dgm:adj` (`CT_Adj`) — one adjustment value of a [`LayoutShape`]'s output geometry, by its
/// 1-based index into that geometry's adjustment list. ECMA-376 Part 1 §21.4.2.1 *CT_Adj
/// (Shape Adjust)*.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "idx", codec = Number<u32>, accessor = index, required))]
#[xml(attribute(local = "val", codec = Number<f64>, accessor = value, required))]
pub struct LayoutShapeAdjustment {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}
fidelity_element_impls!(LayoutShapeAdjustment);

impl LayoutShapeAdjustment {
    /// A fresh `dgm:adj` setting the geometry's `index`-th (1-based) adjustment to `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, index: u32, value: f64) -> Self {
        let mut element = Self {
            name: dgm_name(interner, "adj"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        element.set_index(interner, index);
        element.set_value(interner, value);
        element
    }
}

/// `dgm:adjLst` (`CT_AdjLst`) — a [`LayoutShape`]'s output-geometry adjustments, in order.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_DIAGRAM)]
pub struct LayoutShapeAdjustmentList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "adj", variant = Adjustment, ty = LayoutShapeAdjustment))]
    content: Vec<LayoutShapeAdjustmentListContent>,
}

/// One ordered child of a [`LayoutShapeAdjustmentList`]: a typed [`LayoutShapeAdjustment`], or an
/// opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutShapeAdjustmentListContent {
    /// An adjustment (`dgm:adj`).
    Adjustment(LayoutShapeAdjustment),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

impl LayoutShapeAdjustmentList {
    /// The adjustments, in order (opaque children are skipped).
    pub fn adjustments(&self) -> impl Iterator<Item = &LayoutShapeAdjustment> {
        self.content.iter().filter_map(|item| match item {
            LayoutShapeAdjustmentListContent::Adjustment(adjustment) => Some(adjustment),
            LayoutShapeAdjustmentListContent::Raw(_) => None,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// dgm:shape (CT_Shape)
// ---------------------------------------------------------------------------------------------

/// `dgm:shape` (`CT_Shape`) — the output geometry a [`LayoutNode`] draws its point as: a preset
/// shape or `none`/`conn` ([`ST_LayoutShapeType`](diagram::LayoutShapeType), a union of DrawingML's
/// own `a:ST_ShapeType` with two diagram-specific values), with its own adjustment overrides.
/// ECMA-376 Part 1 §21.4.2.42 *CT_Shape*.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_DIAGRAM)]
#[xml(attribute(local = "rot", codec = Number<f64>, accessor = rotation))]
#[xml(attribute(local = "type", codec = Text, accessor = shape_type))]
#[xml(attribute(local = "blip", prefix = "r", codec = Text, accessor = image_relationship))]
#[xml(attribute(local = "zOrderOff", codec = Number<i32>, accessor = z_order_offset))]
#[xml(attribute(local = "hideGeom", codec = OnOff, accessor = hide_geometry))]
#[xml(attribute(local = "lkTxEntry", codec = OnOff, accessor = lock_text_entry))]
#[xml(attribute(local = "blipPhldr", codec = OnOff, accessor = is_image_placeholder))]
pub struct LayoutShape {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "adjLst", variant = Adjustments, ty = LayoutShapeAdjustmentList))]
    content: Vec<LayoutShapeContent>,
}

/// One ordered child of a [`LayoutShape`]: its typed adjustment list, or an opaque node
/// (`dgm:extLst`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutShapeContent {
    /// The shape's adjustment overrides (`dgm:adjLst`).
    Adjustments(LayoutShapeAdjustmentList),
    /// Any other child — `dgm:extLst`, whitespace, or an unknown element — preserved verbatim.
    Raw(RawNode),
}

impl LayoutShape {
    /// A fresh, empty `dgm:shape` — no type stated, which the schema takes to mean `none`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: dgm_name(interner, "shape"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The shape's adjustment overrides (`dgm:adjLst`), or `None` if it declares none.
    #[must_use]
    pub fn adjustments(&self) -> Option<&LayoutShapeAdjustmentList> {
        self.content.iter().find_map(|item| match item {
            LayoutShapeContent::Adjustments(list) => Some(list),
            LayoutShapeContent::Raw(_) => None,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// dgm:presOf (CT_PresentationOf)
// ---------------------------------------------------------------------------------------------

/// `dgm:presOf` (`CT_PresentationOf`) — which points, along which axis of the data model, this
/// layout node presents. Its five iterator attributes (`@axis`, `@ptType`, `@hideLastTrans`, `@st`,
/// `@cnt`, `@step`) are each an `xsd:list` in the schema (`ST_AxisTypes`, `ST_ElementTypes`,
/// `ST_Booleans`, `ST_Ints`, `ST_UnsignedInts`, `ST_Ints`); this crate stores each as its
/// space-separated wire string rather than parsing the list, for the same reason `dgm:param@val` is
/// kept as a string — see the [module docs](self). ECMA-376 Part 1 §21.4.2.34 *CT_PresentationOf*.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "axis", codec = Text, accessor = axis))]
#[xml(attribute(local = "ptType", codec = Text, accessor = point_types))]
#[xml(attribute(local = "hideLastTrans", codec = Text, accessor = hide_last_transition))]
#[xml(attribute(local = "st", codec = Text, accessor = start))]
#[xml(attribute(local = "cnt", codec = Text, accessor = count))]
#[xml(attribute(local = "step", codec = Text, accessor = step))]
pub struct PresentationOf {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}
fidelity_element_impls!(PresentationOf);

impl PresentationOf {
    /// A fresh, empty `dgm:presOf` — every iterator attribute at its schema default (present axis,
    /// all point types, from the first, unbounded, step one).
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: dgm_name(interner, "presOf"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// dgm:constr (CT_Constraint), dgm:constrLst (CT_Constraints)
// ---------------------------------------------------------------------------------------------

/// `dgm:constr` (`CT_Constraint`) — a sizing/spacing rule a layout node applies, optionally relative
/// to another element's constraint (`@refType`/`@refFor`/`@refForName`/`@refPtType`), combined by
/// `@op` with `@val`/`@fact`. ECMA-376 Part 1 §21.4.2.11 *CT_Constraint*.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "type", codec = Enumeration<ConstraintType>, accessor = constraint, required))]
#[xml(attribute(local = "for", codec = Enumeration<ConstraintRelationship>, accessor = relationship))]
#[xml(attribute(local = "forName", codec = Text, accessor = name_reference))]
#[xml(attribute(local = "ptType", codec = Enumeration<ElementType>, accessor = point_type))]
#[xml(attribute(local = "refType", codec = Enumeration<ConstraintType>, accessor = reference_constraint))]
#[xml(attribute(local = "refFor", codec = Enumeration<ConstraintRelationship>, accessor = reference_relationship))]
#[xml(attribute(local = "refForName", codec = Text, accessor = reference_name))]
#[xml(attribute(local = "refPtType", codec = Enumeration<ElementType>, accessor = reference_point_type))]
#[xml(attribute(local = "op", codec = Enumeration<BoolOperator>, accessor = operator))]
#[xml(attribute(local = "val", codec = Number<f64>, accessor = value))]
#[xml(attribute(local = "fact", codec = Number<f64>, accessor = factor))]
pub struct Constraint {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}
fidelity_element_impls!(Constraint);

impl Constraint {
    /// A fresh `dgm:constr` of `constraint`, with every other attribute at its schema default.
    #[must_use]
    pub fn new(interner: &mut Interner, constraint: ConstraintType) -> Self {
        let mut element = Self {
            name: dgm_name(interner, "constr"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        element.set_constraint(interner, constraint);
        element
    }
}

/// `dgm:constrLst` (`CT_Constraints`) — a layout node's constraints, in order.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_DIAGRAM)]
pub struct ConstraintList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "constr", variant = Constraint, ty = Constraint))]
    content: Vec<ConstraintListContent>,
}

/// One ordered child of a [`ConstraintList`]: a typed [`Constraint`], or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintListContent {
    /// A constraint (`dgm:constr`).
    Constraint(Constraint),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

impl ConstraintList {
    /// The constraints, in order (opaque children are skipped).
    pub fn constraints(&self) -> impl Iterator<Item = &Constraint> {
        self.content.iter().filter_map(|item| match item {
            ConstraintListContent::Constraint(constraint) => Some(constraint),
            ConstraintListContent::Raw(_) => None,
        })
    }

    /// Appends `constraint` at the end of the list.
    pub fn push(&mut self, constraint: Constraint) {
        self.content
            .push(ConstraintListContent::Constraint(constraint));
        self.empty = false;
    }
}

// ---------------------------------------------------------------------------------------------
// dgm:rule (CT_NumericRule), dgm:ruleLst (CT_Rules)
// ---------------------------------------------------------------------------------------------

/// `dgm:rule` (`CT_NumericRule`) — a numeric bound (`@val`, scaled by `@fact`, capped at `@max`) a
/// layout node's rendering must respect — e.g. the minimum font size a `tx` algorithm shrinks text
/// to before it stops shrinking. ECMA-376 Part 1 §21.4.2.33 *CT_NumericRule*.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "type", codec = Enumeration<ConstraintType>, accessor = constraint, required))]
#[xml(attribute(local = "for", codec = Enumeration<ConstraintRelationship>, accessor = relationship))]
#[xml(attribute(local = "forName", codec = Text, accessor = name_reference))]
#[xml(attribute(local = "ptType", codec = Enumeration<ElementType>, accessor = point_type))]
#[xml(attribute(local = "val", codec = Number<f64>, accessor = value))]
#[xml(attribute(local = "fact", codec = Number<f64>, accessor = factor))]
#[xml(attribute(local = "max", codec = Number<f64>, accessor = maximum))]
pub struct NumericRule {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}
fidelity_element_impls!(NumericRule);

impl NumericRule {
    /// A fresh `dgm:rule` bounding `constraint`, with every other attribute at its schema default
    /// (`val`/`fact`/`max` all `NaN`, meaning unset — an actual `NaN` bound is not a legal rule).
    #[must_use]
    pub fn new(interner: &mut Interner, constraint: ConstraintType) -> Self {
        let mut element = Self {
            name: dgm_name(interner, "rule"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        element.set_constraint(interner, constraint);
        element
    }
}

/// `dgm:ruleLst` (`CT_Rules`) — a layout node's numeric rules, in order.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_DIAGRAM)]
pub struct RuleList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "rule", variant = Rule, ty = NumericRule))]
    content: Vec<RuleListContent>,
}

/// One ordered child of a [`RuleList`]: a typed [`NumericRule`], or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleListContent {
    /// A rule (`dgm:rule`).
    Rule(NumericRule),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

impl RuleList {
    /// The rules, in order (opaque children are skipped).
    pub fn rules(&self) -> impl Iterator<Item = &NumericRule> {
        self.content.iter().filter_map(|item| match item {
            RuleListContent::Rule(rule) => Some(rule),
            RuleListContent::Raw(_) => None,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// dgm:layoutNode (CT_LayoutNode), dgm:forEach (CT_ForEach), dgm:if / dgm:else (CT_When /
// CT_Otherwise), dgm:choose (CT_Choose)
// ---------------------------------------------------------------------------------------------
//
// `CT_LayoutNode`, `CT_ForEach`, `CT_When` and `CT_Otherwise` declare the *same* `xsd:choice` of
// alternatives (`CT_LayoutNode` alone adds an eleventh, `dgm:varLst`) — the recursive tree an
// algorithm walks. Because the choice is unbounded and unordered (an `xsd:choice`, not a
// `xsd:sequence`: ECMA-376 states no order among these, and `mjx_ooxml_types::child_order` agrees —
// see the [module docs](self)), the eight alternatives every one of the four container types allows
// are named identically in two content enums — [`LayoutNodeContent`] (nine variants: the eight, plus
// `dgm:varLst`, since only a [`LayoutNode`] allows layout variable overrides) and
// [`LayoutBranchContent`] (the eight, for [`ForEachIterator`]/[`LayoutCondition`]/
// [`LayoutOtherwise`]) — rather than one. The generated `ToXml` match is exhaustive over a struct's
// own content type, so a single enum shared by a type that cannot carry `varLst` would need a
// `Variables` arm it can never produce; two enums keep every match provably exhaustive instead of
// relying on a variant nothing constructs.

/// One ordered child of a [`LayoutNode`] — the ten alternatives `xsd:choice` allows it (the eight
/// [`LayoutBranchContent`] also allows, plus `dgm:varLst` — only a `LayoutNode` allows layout
/// variable overrides), or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutNodeContent {
    /// An algorithm (`dgm:alg`).
    Algorithm(Algorithm),
    /// An output shape (`dgm:shape`).
    Shape(LayoutShape),
    /// A presentation binding (`dgm:presOf`).
    PresentationOf(PresentationOf),
    /// Sizing/spacing constraints (`dgm:constrLst`).
    Constraints(ConstraintList),
    /// Numeric rules (`dgm:ruleLst`).
    Rules(RuleList),
    /// Layout variable overrides (`dgm:varLst`) — only ever a child of a [`LayoutNode`].
    Variables(LayoutVariables),
    /// A repeated sub-tree (`dgm:forEach`).
    ForEach(ForEachIterator),
    /// A nested layout node (`dgm:layoutNode`).
    LayoutNode(LayoutNode),
    /// A conditional sub-tree (`dgm:choose`).
    Choose(Choose),
    /// Any other child — `dgm:extLst`, whitespace, or an unknown element — preserved verbatim.
    Raw(RawNode),
}

impl LayoutNodeContent {
    /// The nested algorithms, output shape, constraints, rules and layout nodes are read the same
    /// way from every one of the four container types below, so their shared accessors live here.
    fn algorithms(content: &[Self]) -> impl Iterator<Item = &Algorithm> {
        content.iter().filter_map(|item| match item {
            Self::Algorithm(algorithm) => Some(algorithm),
            _ => None,
        })
    }

    fn shape(content: &[Self]) -> Option<&LayoutShape> {
        content.iter().find_map(|item| match item {
            Self::Shape(shape) => Some(shape),
            _ => None,
        })
    }

    fn presentation_of(content: &[Self]) -> Option<&PresentationOf> {
        content.iter().find_map(|item| match item {
            Self::PresentationOf(presentation_of) => Some(presentation_of),
            _ => None,
        })
    }

    fn constraints(content: &[Self]) -> Option<&ConstraintList> {
        content.iter().find_map(|item| match item {
            Self::Constraints(constraints) => Some(constraints),
            _ => None,
        })
    }

    fn rules(content: &[Self]) -> Option<&RuleList> {
        content.iter().find_map(|item| match item {
            Self::Rules(rules) => Some(rules),
            _ => None,
        })
    }

    fn for_each(content: &[Self]) -> impl Iterator<Item = &ForEachIterator> {
        content.iter().filter_map(|item| match item {
            Self::ForEach(for_each) => Some(for_each),
            _ => None,
        })
    }

    fn layout_nodes(content: &[Self]) -> impl Iterator<Item = &LayoutNode> {
        content.iter().filter_map(|item| match item {
            Self::LayoutNode(node) => Some(node),
            _ => None,
        })
    }

    fn choose(content: &[Self]) -> impl Iterator<Item = &Choose> {
        content.iter().filter_map(|item| match item {
            Self::Choose(choose) => Some(choose),
            _ => None,
        })
    }
}

/// One ordered child of a [`ForEachIterator`], a [`LayoutCondition`] or a [`LayoutOtherwise`] — the
/// same eight alternatives [`LayoutNodeContent`] allows a [`LayoutNode`], minus `dgm:varLst` (only a
/// `LayoutNode` allows layout variable overrides), or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutBranchContent {
    /// An algorithm (`dgm:alg`).
    Algorithm(Algorithm),
    /// An output shape (`dgm:shape`).
    Shape(LayoutShape),
    /// A presentation binding (`dgm:presOf`).
    PresentationOf(PresentationOf),
    /// Sizing/spacing constraints (`dgm:constrLst`).
    Constraints(ConstraintList),
    /// Numeric rules (`dgm:ruleLst`).
    Rules(RuleList),
    /// A repeated sub-tree (`dgm:forEach`).
    ForEach(ForEachIterator),
    /// A nested layout node (`dgm:layoutNode`).
    LayoutNode(LayoutNode),
    /// A conditional sub-tree (`dgm:choose`).
    Choose(Choose),
    /// Any other child — `dgm:extLst`, whitespace, or an unknown element — preserved verbatim.
    Raw(RawNode),
}

impl LayoutBranchContent {
    fn layout_nodes(content: &[Self]) -> impl Iterator<Item = &LayoutNode> {
        content.iter().filter_map(|item| match item {
            Self::LayoutNode(node) => Some(node),
            _ => None,
        })
    }
}

/// `dgm:layoutNode` (`CT_LayoutNode`) — one node of the layout algorithm tree: an algorithm to run,
/// the output shape it draws, which points it presents, its sizing constraints and numeric rules,
/// layout variable overrides, and nested `forEach`/`layoutNode`/`choose` children.
///
/// This tree, walked top to bottom, is what a layout *engine* would execute to place a diagram's
/// shapes — this crate models the tree faithfully but does not walk it. See the
/// [module docs](self). ECMA-376 Part 1 §21.4.2.25 *CT_LayoutNode*.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_DIAGRAM)]
#[xml(attribute(local = "name", codec = Text, accessor = node_name))]
#[xml(attribute(local = "styleLbl", codec = Text, accessor = style_label))]
#[xml(attribute(local = "chOrder", codec = Enumeration<diagram::ChildOrderType>, accessor = child_order))]
#[xml(attribute(local = "moveWith", codec = Text, accessor = move_with))]
pub struct LayoutNode {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "alg", variant = Algorithm, ty = Algorithm),
        child(local = "shape", variant = Shape, ty = LayoutShape),
        child(local = "presOf", variant = PresentationOf, ty = PresentationOf),
        child(local = "constrLst", variant = Constraints, ty = ConstraintList),
        child(local = "ruleLst", variant = Rules, ty = RuleList),
        child(local = "varLst", variant = Variables, ty = LayoutVariables),
        child(local = "forEach", variant = ForEach, ty = ForEachIterator),
        child(local = "layoutNode", variant = LayoutNode, ty = LayoutNode),
        child(local = "choose", variant = Choose, ty = Choose)
    )]
    content: Vec<LayoutNodeContent>,
}

impl LayoutNode {
    /// A fresh, empty `dgm:layoutNode` named `node_name` (`@name`).
    #[must_use]
    pub fn new(interner: &mut Interner, node_name: &str) -> Self {
        let mut node = Self {
            name: dgm_name(interner, "layoutNode"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        node.set_node_name(interner, Some(node_name));
        node
    }

    /// The node's algorithms, in order (a well-formed node has at most one; the choice allows more).
    pub fn algorithms(&self) -> impl Iterator<Item = &Algorithm> {
        LayoutNodeContent::algorithms(&self.content)
    }
    /// The node's output shape (`dgm:shape`), or `None`.
    #[must_use]
    pub fn shape(&self) -> Option<&LayoutShape> {
        LayoutNodeContent::shape(&self.content)
    }
    /// The node's presentation binding (`dgm:presOf`), or `None`.
    #[must_use]
    pub fn presentation_of(&self) -> Option<&PresentationOf> {
        LayoutNodeContent::presentation_of(&self.content)
    }
    /// The node's constraints (`dgm:constrLst`), or `None`.
    #[must_use]
    pub fn constraints(&self) -> Option<&ConstraintList> {
        LayoutNodeContent::constraints(&self.content)
    }
    /// The node's numeric rules (`dgm:ruleLst`), or `None`.
    #[must_use]
    pub fn rules(&self) -> Option<&RuleList> {
        LayoutNodeContent::rules(&self.content)
    }
    /// The node's layout variable overrides (`dgm:varLst`), or `None`.
    #[must_use]
    pub fn variables(&self) -> Option<&LayoutVariables> {
        self.content.iter().find_map(|item| match item {
            LayoutNodeContent::Variables(variables) => Some(variables),
            _ => None,
        })
    }
    /// The node's `forEach` children, in order.
    pub fn for_each(&self) -> impl Iterator<Item = &ForEachIterator> {
        LayoutNodeContent::for_each(&self.content)
    }
    /// The node's nested `layoutNode` children, in order.
    pub fn layout_nodes(&self) -> impl Iterator<Item = &LayoutNode> {
        LayoutNodeContent::layout_nodes(&self.content)
    }
    /// The node's `choose` children, in order.
    pub fn choose(&self) -> impl Iterator<Item = &Choose> {
        LayoutNodeContent::choose(&self.content)
    }
    /// Appends `child` at the end of this node's content.
    pub fn push(&mut self, child: LayoutNodeContent) {
        self.content.push(child);
        self.empty = false;
    }
    /// The node's ordered content, verbatim.
    #[must_use]
    pub fn content(&self) -> &[LayoutNodeContent] {
        &self.content
    }
}

/// `dgm:forEach` (`CT_ForEach`) — repeats its content once per point the iterator attributes select
/// (`@ref` names a `dgm:layoutNode` whose iterator this one continues from). ECMA-376 Part 1
/// §21.4.2.20 *CT_ForEach*.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_DIAGRAM)]
#[xml(attribute(local = "name", codec = Text, accessor = node_name))]
#[xml(attribute(local = "ref", codec = Text, accessor = reference))]
#[xml(attribute(local = "axis", codec = Text, accessor = axis))]
#[xml(attribute(local = "ptType", codec = Text, accessor = point_types))]
#[xml(attribute(local = "hideLastTrans", codec = Text, accessor = hide_last_transition))]
#[xml(attribute(local = "st", codec = Text, accessor = start))]
#[xml(attribute(local = "cnt", codec = Text, accessor = count))]
#[xml(attribute(local = "step", codec = Text, accessor = step))]
pub struct ForEachIterator {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "alg", variant = Algorithm, ty = Algorithm),
        child(local = "shape", variant = Shape, ty = LayoutShape),
        child(local = "presOf", variant = PresentationOf, ty = PresentationOf),
        child(local = "constrLst", variant = Constraints, ty = ConstraintList),
        child(local = "ruleLst", variant = Rules, ty = RuleList),
        child(local = "forEach", variant = ForEach, ty = ForEachIterator),
        child(local = "layoutNode", variant = LayoutNode, ty = LayoutNode),
        child(local = "choose", variant = Choose, ty = Choose)
    )]
    content: Vec<LayoutBranchContent>,
}

impl ForEachIterator {
    /// The iterator's nested `layoutNode` children, in order.
    pub fn layout_nodes(&self) -> impl Iterator<Item = &LayoutNode> {
        LayoutBranchContent::layout_nodes(&self.content)
    }
    /// The iterator's ordered content, verbatim.
    #[must_use]
    pub fn content(&self) -> &[LayoutBranchContent] {
        &self.content
    }
}

/// `dgm:if` (`CT_When`) — one branch of a [`Choose`], taken when its function/operator/value test
/// holds. ECMA-376 Part 1 §21.4.2.45 *CT_When*.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_DIAGRAM)]
#[xml(attribute(local = "name", codec = Text, accessor = node_name))]
#[xml(attribute(local = "axis", codec = Text, accessor = axis))]
#[xml(attribute(local = "ptType", codec = Text, accessor = point_types))]
#[xml(attribute(local = "hideLastTrans", codec = Text, accessor = hide_last_transition))]
#[xml(attribute(local = "st", codec = Text, accessor = start))]
#[xml(attribute(local = "cnt", codec = Text, accessor = count))]
#[xml(attribute(local = "step", codec = Text, accessor = step))]
#[xml(attribute(local = "func", codec = Enumeration<diagram::FunctionType>, accessor = function, required))]
#[xml(attribute(local = "arg", codec = Text, accessor = argument))]
#[xml(attribute(local = "op", codec = Enumeration<diagram::FunctionOperator>, accessor = operator, required))]
#[xml(attribute(local = "val", codec = Text, accessor = value, required))]
pub struct LayoutCondition {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "alg", variant = Algorithm, ty = Algorithm),
        child(local = "shape", variant = Shape, ty = LayoutShape),
        child(local = "presOf", variant = PresentationOf, ty = PresentationOf),
        child(local = "constrLst", variant = Constraints, ty = ConstraintList),
        child(local = "ruleLst", variant = Rules, ty = RuleList),
        child(local = "forEach", variant = ForEach, ty = ForEachIterator),
        child(local = "layoutNode", variant = LayoutNode, ty = LayoutNode),
        child(local = "choose", variant = Choose, ty = Choose)
    )]
    content: Vec<LayoutBranchContent>,
}

impl LayoutCondition {
    /// The condition's ordered content, verbatim.
    #[must_use]
    pub fn content(&self) -> &[LayoutBranchContent] {
        &self.content
    }
}

/// `dgm:else` (`CT_Otherwise`) — the branch of a [`Choose`] taken when none of its `dgm:if`
/// branches match. ECMA-376 Part 1 §21.4.2.28 *CT_Otherwise*.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_DIAGRAM)]
#[xml(attribute(local = "name", codec = Text, accessor = node_name))]
pub struct LayoutOtherwise {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "alg", variant = Algorithm, ty = Algorithm),
        child(local = "shape", variant = Shape, ty = LayoutShape),
        child(local = "presOf", variant = PresentationOf, ty = PresentationOf),
        child(local = "constrLst", variant = Constraints, ty = ConstraintList),
        child(local = "ruleLst", variant = Rules, ty = RuleList),
        child(local = "forEach", variant = ForEach, ty = ForEachIterator),
        child(local = "layoutNode", variant = LayoutNode, ty = LayoutNode),
        child(local = "choose", variant = Choose, ty = Choose)
    )]
    content: Vec<LayoutBranchContent>,
}

impl LayoutOtherwise {
    /// The branch's ordered content, verbatim.
    #[must_use]
    pub fn content(&self) -> &[LayoutBranchContent] {
        &self.content
    }
}

/// One ordered child of a [`Choose`]: a typed `if`/`else` branch, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChooseContent {
    /// A condition branch (`dgm:if`).
    If(LayoutCondition),
    /// The default branch (`dgm:else`).
    Else(LayoutOtherwise),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `dgm:choose` (`CT_Choose`) — one or more `dgm:if` branches, tested in order, and an optional
/// `dgm:else` fallback. Unlike the recursive choice above, this is a genuine `xsd:sequence` (`if`
/// one-or-more, then `else` zero-or-one), so order here *is* validity. ECMA-376 Part 1 §21.4.2.12
/// *CT_Choose*.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_DIAGRAM)]
#[xml(attribute(local = "name", codec = Text, accessor = node_name))]
pub struct Choose {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "if", variant = If, ty = LayoutCondition),
        child(local = "else", variant = Else, ty = LayoutOtherwise)
    )]
    content: Vec<ChooseContent>,
}

impl Choose {
    /// The `dgm:if` branches, in the order they are tested.
    pub fn conditions(&self) -> impl Iterator<Item = &LayoutCondition> {
        self.content.iter().filter_map(|item| match item {
            ChooseContent::If(condition) => Some(condition),
            _ => None,
        })
    }
    /// The `dgm:else` fallback, or `None` if this choice declares none.
    #[must_use]
    pub fn otherwise(&self) -> Option<&LayoutOtherwise> {
        self.content.iter().find_map(|item| match item {
            ChooseContent::Else(otherwise) => Some(otherwise),
            _ => None,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// dgm:sampData / dgm:styleData / dgm:clrData (CT_SampleData)
// ---------------------------------------------------------------------------------------------

/// `dgm:sampData` / `dgm:styleData` / `dgm:clrData` (`CT_SampleData`) — the sample data model a
/// layout/style/colour gallery preview draws with; `@useDef` selects the built-in default sample
/// instead of the one this element carries. All three elements share this one complex type.
/// ECMA-376 Part 1 §21.4.2.37 *CT_SampleData*.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "useDef", codec = OnOff, accessor = use_default))]
pub struct SampleData {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}
fidelity_element_impls!(SampleData);

impl SampleData {
    /// The sample's `dgm:dataModel`, or `None` if it declares none (a well-formed element carrying
    /// `@useDef="true"` typically has none). This is a **preview** graph — the same
    /// [`DataModel`](super::data::DataModel) type the diagram's actual data part uses.
    #[must_use]
    pub fn data_model(&self, interner: &Interner) -> Option<super::data::DataModel> {
        super::support::dgm_child(&self.children, interner, "dataModel")
            .and_then(|element| super::data::DataModel::from_xml(element, interner).ok())
    }
}

// ---------------------------------------------------------------------------------------------
// dgm:layoutDef (CT_DiagramDefinition) — the layout definition part's root
// ---------------------------------------------------------------------------------------------

/// One ordered child of a [`LayoutDefinition`]: its typed members, or an opaque node
/// (`dgm:extLst`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutDefinitionContent {
    /// A display name (`dgm:title`) — repeatable, one per locale.
    Title(DiagramTitle),
    /// A description (`dgm:desc`) — repeatable, one per locale.
    Description(DiagramDescription),
    /// The gallery categories this layout belongs to (`dgm:catLst`).
    Categories(DiagramCategoryList),
    /// The sample data the gallery preview draws (`dgm:sampData`).
    SampleData(SampleData),
    /// The sample data a style-gallery preview draws using this layout (`dgm:styleData`).
    StyleSampleData(SampleData),
    /// The sample data a colour-gallery preview draws using this layout (`dgm:clrData`).
    ColorSampleData(SampleData),
    /// The algorithm tree's root (`dgm:layoutNode`).
    Root(LayoutNode),
    /// Any other child — `dgm:extLst`, whitespace, or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `dgm:layoutDef` (`CT_DiagramDefinition`) — the root of the Diagram Layout Definition part: the
/// display name/description/gallery category this layout is offered under, its sample data, and the
/// algorithm tree ([`LayoutNode`]) that positions the diagram's points. ECMA-376 Part 1 §21.4.2.19
/// *CT_DiagramDefinition*.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_DIAGRAM)]
#[xml(attribute(local = "uniqueId", codec = Text, accessor = unique_id))]
#[xml(attribute(local = "minVer", codec = Text, accessor = minimum_version))]
#[xml(attribute(local = "defStyle", codec = Text, accessor = default_style))]
pub struct LayoutDefinition {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "title", variant = Title, ty = DiagramTitle),
        child(local = "desc", variant = Description, ty = DiagramDescription),
        child(local = "catLst", variant = Categories, ty = DiagramCategoryList),
        child(local = "sampData", variant = SampleData, ty = SampleData),
        child(local = "styleData", variant = StyleSampleData, ty = SampleData),
        child(local = "clrData", variant = ColorSampleData, ty = SampleData),
        child(local = "layoutNode", variant = Root, ty = LayoutNode)
    )]
    content: Vec<LayoutDefinitionContent>,
}

impl LayoutDefinition {
    /// A fresh `dgm:layoutDef` naming `unique_id` (`@uniqueId`), with `root` as its algorithm tree —
    /// `CT_DiagramDefinition`'s one required child.
    #[must_use]
    pub fn new(interner: &mut Interner, unique_id: &str, root: LayoutNode) -> Self {
        let mut definition = Self {
            name: dgm_name(interner, "layoutDef"),
            attributes: Vec::new(),
            empty: false,
            content: vec![LayoutDefinitionContent::Root(root)],
        };
        definition.set_unique_id(interner, Some(unique_id));
        definition
    }

    /// The layout's display names (`dgm:title`), one per locale.
    pub fn titles(&self) -> impl Iterator<Item = &DiagramTitle> {
        self.content.iter().filter_map(|item| match item {
            LayoutDefinitionContent::Title(title) => Some(title),
            _ => None,
        })
    }
    /// The layout's gallery categories (`dgm:catLst`), or `None` if it declares none.
    #[must_use]
    pub fn categories(&self) -> Option<&DiagramCategoryList> {
        self.content.iter().find_map(|item| match item {
            LayoutDefinitionContent::Categories(categories) => Some(categories),
            _ => None,
        })
    }
    /// The algorithm tree's root (`dgm:layoutNode`), or `None` on a definition malformed enough to
    /// omit the schema's one required child.
    #[must_use]
    pub fn root(&self) -> Option<&LayoutNode> {
        self.content.iter().find_map(|item| match item {
            LayoutDefinitionContent::Root(root) => Some(root),
            _ => None,
        })
    }
    /// The definition's ordered content, verbatim.
    #[must_use]
    pub fn content(&self) -> &[LayoutDefinitionContent] {
        &self.content
    }
}
