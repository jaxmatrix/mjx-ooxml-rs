//! `x:externalReferences`, `x:pivotCaches` and `x:functionGroups` — three lists in `CT_Workbook`'s
//! sequence that point *outward*, at parts and at capabilities this child deliberately does not
//! model.
//!
//! # References are modelled; what they reach is not
//!
//! `CT_ExternalReference` (`sml.xsd:4355`) and `CT_PivotCache` (`4368`) are one relationship id
//! each. The parts they name — `xl/externalLinks/externalLink1.xml`, `xl/pivotCache/*` — are
//! MJXOFF-133's (D18) to write down and nothing here's to interpret. But the *elements* sit in the
//! workbook's own `xsd:sequence`, at ranks 7 and 12, so a workbook that carries them and a writer
//! that dropped them would produce a file that has quietly lost its external links. They are
//! modelled here so that they are emitted in order and survive an edit to anything else, and for no
//! other reason.
//!
//! Everything those parts hold is still preserved regardless, by `mjx-opc`'s part-level
//! copy-on-write: a part nothing models is re-emitted verbatim.
//!
//! # `CT_FunctionGroups`, and a schema oddity worth naming
//!
//! `CT_FunctionGroups` (`sml.xsd:4419`) declares `<xsd:sequence maxOccurs="unbounded">` around a
//! single `<xsd:element name="functionGroup" minOccurs="0"/>` — a repeated sequence of one optional
//! element, which is a long way of writing "any number of `functionGroup`s". `@builtInGroupCount`
//! says how many of the groups are the consumer's own built-ins rather than add-in registered ones;
//! it is a **count the file states**, and this crate reports it without checking it against the
//! number of children, exactly as it reports `sst/@count`.

use mjx_ooxml_core::{Number, RawAttribute, RawName, RawNode, Text};

use super::leaf::{attribute_bag, bag_without_declared_attributes, relationship_reference};

bag_without_declared_attributes! {
    /// `x:externalReference` (`CT_ExternalReference`) — one relationship to an external-link part.
    ///
    /// `r:id` is the element's only attribute, and resolving it is `mjx-xlsx`'s: see
    /// [`relationship_id`](Self::relationship_id).
    ExternalReference, "externalReference"
}

relationship_reference!(ExternalReference);

attribute_bag! {
    /// `x:pivotCache` (`CT_PivotCache`) — one pivot cache: the id a `pivotTableDefinition` refers to
    /// it by, and the relationship to its definition part.
    #[xml(attribute(local = "cacheId", codec = Number<u32>, accessor = cache_id, required))]
    PivotCache, "pivotCache"
}

relationship_reference!(PivotCache);

attribute_bag! {
    /// `x:functionGroup` (`CT_FunctionGroup`) — one named group a worksheet function belongs to.
    #[xml(attribute(local = "name", codec = Text, accessor = name))]
    FunctionGroup, "functionGroup"
}

/// `x:externalReferences` (`CT_ExternalReferences`) — the external-link references, in document
/// order, which is the order their one-based indices in a formula's `[1]Sheet1!A1` run in.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = SML)]
pub struct ExternalReferences {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "externalReference", variant = Reference, ty = ExternalReference))]
    content: Vec<ExternalReferencesContent>,
}

/// One child of [`ExternalReferences`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalReferencesContent {
    /// `x:externalReference`.
    Reference(ExternalReference),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl ExternalReferences {
    /// Builds an empty `x:externalReferences`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut mjx_ooxml_core::Interner, prefix: Option<&str>) -> Self {
        Self {
            name: super::leaf::sml_name(interner, prefix, "externalReferences"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every `x:externalReference`, in document order.
    pub fn references(&self) -> impl Iterator<Item = &ExternalReference> + '_ {
        self.content.iter().filter_map(|item| match item {
            ExternalReferencesContent::Reference(reference) => Some(reference),
            ExternalReferencesContent::Raw(_) => None,
        })
    }

    /// Appends a reference after the ones already present.
    pub fn push(&mut self, reference: ExternalReference) {
        self.content
            .push(ExternalReferencesContent::Reference(reference));
    }
}

/// `x:pivotCaches` (`CT_PivotCaches`) — the workbook's pivot caches, in document order.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = SML)]
pub struct PivotCaches {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "pivotCache", variant = Cache, ty = PivotCache))]
    content: Vec<PivotCachesContent>,
}

/// One child of [`PivotCaches`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PivotCachesContent {
    /// `x:pivotCache`.
    Cache(PivotCache),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl PivotCaches {
    /// Builds an empty `x:pivotCaches`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut mjx_ooxml_core::Interner, prefix: Option<&str>) -> Self {
        Self {
            name: super::leaf::sml_name(interner, prefix, "pivotCaches"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every `x:pivotCache`, in document order.
    pub fn caches(&self) -> impl Iterator<Item = &PivotCache> + '_ {
        self.content.iter().filter_map(|item| match item {
            PivotCachesContent::Cache(cache) => Some(cache),
            PivotCachesContent::Raw(_) => None,
        })
    }

    /// Appends a cache after the ones already present.
    pub fn push(&mut self, cache: PivotCache) {
        self.content.push(PivotCachesContent::Cache(cache));
    }
}

/// `x:functionGroups` (`CT_FunctionGroups`) — the function groups, plus the count of built-in ones
/// the file states.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "builtInGroupCount", codec = Number<u32>, accessor = built_in_group_count, default = 16))]
pub struct FunctionGroups {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "functionGroup", variant = Group, ty = FunctionGroup))]
    content: Vec<FunctionGroupsContent>,
}

/// One child of [`FunctionGroups`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionGroupsContent {
    /// `x:functionGroup`.
    Group(FunctionGroup),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl FunctionGroups {
    /// Builds an empty `x:functionGroups`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut mjx_ooxml_core::Interner, prefix: Option<&str>) -> Self {
        Self {
            name: super::leaf::sml_name(interner, prefix, "functionGroups"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every `x:functionGroup`, in document order — which is the order their one-based
    /// `functionGroupId`s run in.
    pub fn groups(&self) -> impl Iterator<Item = &FunctionGroup> + '_ {
        self.content.iter().filter_map(|item| match item {
            FunctionGroupsContent::Group(group) => Some(group),
            FunctionGroupsContent::Raw(_) => None,
        })
    }

    /// Appends a group after the ones already present.
    pub fn push(&mut self, group: FunctionGroup) {
        self.content.push(FunctionGroupsContent::Group(group));
    }
}
