//! Reads `xsd:complexType` **content models** out of the reference schemas and flattens each one
//! into the ordered list of child element names the schema permits.
//!
//! This is the input to the committed child-order table (see [`super::child_order`]). It is pure
//! mechanical extraction — no naming, no judgement.
//!
//! # What "rank" means
//!
//! A complex type's content model is a particle tree of `xsd:sequence` / `xsd:choice` / `xsd:group`
//! references / element declarations. Flattening walks that tree and gives every reachable element a
//! **rank**: a position counter that advances across the members of a `sequence` and does *not*
//! advance across the branches of a `choice`. So in `CT_ShapeProperties`
//!
//! ```text
//! sequence( xfrm, EG_Geometry, EG_FillProperties, ln, EG_EffectProperties, scene3d, sp3d, extLst )
//! ```
//!
//! `a:xfrm` is rank 0, the two geometry alternatives share rank 1, the six fill alternatives share
//! rank 2, `a:ln` is rank 3, and so on. Two children with the same rank are alternatives the schema
//! lets stand in the same place, which is exactly the "either an `a:solidFill` or an `a:noFill`, and
//! whichever is there is the one to replace" question a writer asks.
//!
//! Ranks are only *ordering* information when the type's own model is a sequence; a type whose model
//! is `xsd:choice` or `xsd:all` imposes no order at all and is recorded as such rather than given a
//! false one.
//!
//! # `xsd:complexContent` and `xsd:simpleContent`
//!
//! A type may declare its content model *by derivation* instead of directly: `xsd:complexContent`
//! wraps an `xsd:extension` or `xsd:restriction` whose `base` names another complex type. Per
//! [XSD 1.0 §3.4.6](https://www.w3.org/TR/xmlschema-1/#Complex_Type_Definitions), `extension`
//! *appends* the derivation's own particle after the resolved base's; `restriction` *replaces* the
//! base's particle outright with its own (a restriction always restates, alone, whatever subset of
//! the base's model it keeps — the base's particle contributes nothing to a restriction's ranking).
//! Both are folded into a plain [`Particle`] before ranking runs, in [`SchemaSet::resolve_derivation`]
//! — walking and rank assignment never see a [`Particle::Derivation`] node.
//!
//! `xsd:simpleContent` extends or restricts a *simple* type (never a complex one): its content is
//! the base simple type's character data, and XSD permits it to add attributes but never child
//! elements. It therefore always contributes an empty particle — there is nothing to splice.

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use mjx_ooxml_core::{Interner, RawElement, RawNode};

/// The XML Schema namespace — the only namespace the schema documents' own elements live in.
const XSD: &str = "http://www.w3.org/2001/XMLSchema";

/// One particle of a content model.
#[derive(Debug, Clone)]
pub enum Particle {
    /// `xsd:sequence` — its members occur in the order written.
    Sequence(Vec<Particle>),
    /// `xsd:choice` — exactly one of its branches occurs.
    Choice(Vec<Particle>),
    /// `xsd:all` — its members occur in any order.
    All(Vec<Particle>),
    /// `xsd:group ref="…"` — resolved against the referenced schema's named groups.
    GroupReference {
        /// The referenced group's namespace URI.
        namespace: String,
        /// The referenced group's local name (e.g. `EG_FillProperties`).
        name: String,
    },
    /// A declared or referenced element.
    Element(ElementParticle),
    /// `xsd:any` — a wildcard, which names nothing and is therefore not placeable.
    Wildcard,
    /// `xsd:complexContent`'s `xsd:extension` or `xsd:restriction` — a particle derived from a
    /// named base complex type. Resolved into a concrete [`Particle`] by
    /// [`SchemaSet::resolve_derivation`] before ranking; never itself walked or ranked.
    Derivation {
        /// Whether the base's particle is appended before this type's own (`extension`) or
        /// discarded in favour of it (`restriction`).
        kind: DerivationKind,
        /// The base type, as `(namespace, symbol)`.
        base: (String, String),
        /// The particle this type's own `extension`/`restriction` declares, when it declares
        /// one (`None` for, e.g., an extension that adds only attributes).
        own: Option<Box<Particle>>,
    },
}

/// How a `complexContent` type derives its content model from its base type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivationKind {
    /// `xsd:extension` — the base's resolved particle is followed by this type's own.
    Extension,
    /// `xsd:restriction` — this type's own particle stands alone; the base's is discarded.
    Restriction,
}

/// One element occurrence inside a content model.
#[derive(Debug, Clone)]
pub struct ElementParticle {
    /// The element's namespace URI.
    pub namespace: String,
    /// The element's local name.
    pub local: String,
    /// The element's complex type, as `(namespace, symbol)` — `None` for a simple or anonymous type.
    pub complex_type: Option<(String, String)>,
    /// Whether this occurrence may repeat (`maxOccurs` greater than one, here or on an ancestor).
    pub repeatable: bool,
}

/// How a complex type constrains the order of its children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentModel {
    /// The type declares no child elements at all.
    Empty,
    /// `xsd:sequence` — children must appear in the order the schema declares.
    Sequence,
    /// `xsd:choice` — the alternatives may appear in any order the type allows.
    Choice,
    /// `xsd:all` — the members may appear in any order.
    All,
}

/// One flattened child slot of a complex type.
#[derive(Debug, Clone)]
pub struct Slot {
    /// The child element's namespace URI.
    pub namespace: String,
    /// The child element's local name.
    pub local: String,
    /// The child element's own complex type, as `(namespace, symbol)`, when it has one.
    pub complex_type: Option<(String, String)>,
    /// The child's position in the flattened content model (see the [module docs](self)).
    pub rank: u16,
    /// Whether the child may occur more than once in the same slot.
    pub repeatable: bool,
    /// Whether the schema reaches this element at more than one rank. Such a child has no single
    /// position, so it is neither placed nor audited by rank.
    pub ambiguous: bool,
}

/// A complex type's flattened content model.
#[derive(Debug, Clone)]
pub struct FlatType {
    /// The XSD symbol, e.g. `CT_TextListStyle`.
    pub symbol: String,
    /// How the type constrains child order.
    pub model: ContentModel,
    /// Every child element the type can hold, in rank order (ties in declaration order).
    pub slots: Vec<Slot>,
}

/// One parsed schema document.
#[derive(Debug)]
pub struct Schema {
    /// The schema file name, e.g. `dml-main.xsd` — used in the generated provenance comment.
    pub file: String,
    /// The schema's `targetNamespace`.
    pub target_namespace: String,
    /// `prefix` → namespace URI, from the root element's `xmlns:` declarations.
    prefixes: BTreeMap<String, String>,
    /// Named complex types, in document order.
    complex_types: Vec<(String, Option<Particle>)>,
    /// Named model groups (`EG_*`, `Group_*`).
    groups: BTreeMap<String, Particle>,
    /// Global element declarations: local name → type QName (as written).
    global_elements: BTreeMap<String, String>,
}

impl Schema {
    /// The schema's global element declarations, as `(local, type QName)`, sorted by local name.
    pub fn global_elements(&self) -> impl Iterator<Item = (&str, &str)> {
        self.global_elements
            .iter()
            .map(|(local, ty)| (local.as_str(), ty.as_str()))
    }
}

/// Parses one schema document into its named complex types, groups and global elements.
pub fn parse(file: &str, xsd: &[u8]) -> Result<Schema> {
    let document = mjx_xml::fidelity::parse(xsd).with_context(|| format!("parsing {file}"))?;
    let interner = &document.interner;
    let root = &document.root;

    let mut prefixes = BTreeMap::new();
    let mut target_namespace = String::new();
    for attribute in &root.attributes {
        let local = interner.resolve(attribute.name.local);
        let value = String::from_utf8(attribute.value.to_vec())
            .with_context(|| format!("non-UTF-8 attribute in {file}"))?;
        match attribute.name.prefix.map(|p| interner.resolve(p)) {
            Some("xmlns") => {
                prefixes.insert(local.to_owned(), value);
            }
            None if local == "targetNamespace" => target_namespace = value,
            _ => {}
        }
    }
    if target_namespace.is_empty() {
        bail!("{file} declares no targetNamespace");
    }

    let mut schema = Schema {
        file: file.to_owned(),
        target_namespace,
        prefixes,
        complex_types: Vec::new(),
        groups: BTreeMap::new(),
        global_elements: BTreeMap::new(),
    };

    for node in &root.children {
        let RawNode::Element(child) = node else {
            continue;
        };
        if !is_xsd(child, interner) {
            continue;
        }
        let Some(name) = attribute(child, interner, "name") else {
            continue;
        };
        match interner.resolve(child.name.local) {
            "complexType" => {
                let particle = content_particle(child, interner, &schema, false)?;
                schema.complex_types.push((name, particle));
            }
            "group" => {
                if let Some(particle) = content_particle(child, interner, &schema, false)? {
                    schema.groups.insert(name, particle);
                }
            }
            "element" => {
                if let Some(ty) = attribute(child, interner, "type") {
                    schema.global_elements.insert(name, ty);
                }
            }
            _ => {}
        }
    }
    Ok(schema)
}

/// The single content particle of a `complexType` / `group` element, or `None` when it declares no
/// child elements (an attribute-only type).
fn content_particle(
    holder: &RawElement,
    interner: &Interner,
    schema: &Schema,
    repeatable: bool,
) -> Result<Option<Particle>> {
    for node in &holder.children {
        let RawNode::Element(child) = node else {
            continue;
        };
        if !is_xsd(child, interner) {
            continue;
        }
        match interner.resolve(child.name.local) {
            "sequence" | "choice" | "all" | "group" | "element" | "any" => {
                return Ok(Some(particle(child, interner, schema, repeatable)?));
            }
            "complexContent" => {
                return Ok(Some(complex_content_derivation(child, interner, schema)?));
            }
            // simpleContent's content is the base *simple* type's character data — XSD permits
            // adding attributes but never child elements, so it contributes no particle (see the
            // module docs).
            "simpleContent" => return Ok(None),
            _ => {}
        }
    }
    Ok(None)
}

/// Parses an `xsd:complexContent` element's single `xsd:extension`/`xsd:restriction` child into a
/// [`Particle::Derivation`]. The derivation's own particle is found by recursing into
/// [`content_particle`] on that `extension`/`restriction` element — it scans for the same
/// `sequence`/`choice`/`all`/`group`/`element`/`any` children an `xsd:complexType` does, ignoring
/// the `xsd:attribute` declarations an extension commonly adds alongside (or instead of) a particle.
fn complex_content_derivation(
    complex_content: &RawElement,
    interner: &Interner,
    schema: &Schema,
) -> Result<Particle> {
    for node in &complex_content.children {
        let RawNode::Element(child) = node else {
            continue;
        };
        if !is_xsd(child, interner) {
            continue;
        }
        let kind = match interner.resolve(child.name.local) {
            "extension" => DerivationKind::Extension,
            "restriction" => DerivationKind::Restriction,
            _ => continue,
        };
        let base_qname = attribute(child, interner, "base").with_context(|| {
            format!(
                "{}: complexContent extension/restriction with no base",
                schema.file
            )
        })?;
        let base = schema.resolve_qname(&base_qname)?;
        let own = content_particle(child, interner, schema, false)?;
        return Ok(Particle::Derivation {
            kind,
            base,
            own: own.map(Box::new),
        });
    }
    bail!(
        "{}: complexContent with neither extension nor restriction",
        schema.file
    );
}

/// Converts one particle element into a [`Particle`].
fn particle(
    element: &RawElement,
    interner: &Interner,
    schema: &Schema,
    inherited_repeat: bool,
) -> Result<Particle> {
    let repeatable = inherited_repeat || repeats(element, interner);
    let local = interner.resolve(element.name.local);
    match local {
        "sequence" | "choice" | "all" => {
            let mut members = Vec::new();
            for node in &element.children {
                let RawNode::Element(child) = node else {
                    continue;
                };
                if !is_xsd(child, interner) {
                    continue;
                }
                if matches!(
                    interner.resolve(child.name.local),
                    "sequence" | "choice" | "all" | "group" | "element" | "any"
                ) {
                    members.push(particle(child, interner, schema, repeatable)?);
                }
            }
            Ok(match local {
                "sequence" => Particle::Sequence(members),
                "choice" => Particle::Choice(members),
                _ => Particle::All(members),
            })
        }
        "group" => {
            let reference = attribute(element, interner, "ref")
                .with_context(|| format!("{}: xsd:group with no ref", schema.file))?;
            let (namespace, name) = schema.resolve_qname(&reference)?;
            Ok(Particle::GroupReference { namespace, name })
        }
        "any" => Ok(Particle::Wildcard),
        _ => {
            // `element`, either declared locally or referencing a global declaration.
            if let Some(name) = attribute(element, interner, "name") {
                let complex_type = attribute(element, interner, "type")
                    .map(|ty| schema.resolve_qname(&ty))
                    .transpose()?
                    .filter(|(_, symbol)| symbol.starts_with("CT_"));
                return Ok(Particle::Element(ElementParticle {
                    namespace: schema.target_namespace.clone(),
                    local: name,
                    complex_type,
                    repeatable,
                }));
            }
            let reference = attribute(element, interner, "ref").with_context(|| {
                format!("{}: xsd:element with neither name nor ref", schema.file)
            })?;
            let (namespace, local) = schema.resolve_qname(&reference)?;
            Ok(Particle::Element(ElementParticle {
                namespace,
                local,
                // Filled in from the referenced schema's global declaration during flattening.
                complex_type: None,
                repeatable,
            }))
        }
    }
}

impl Schema {
    /// Splits a QName into `(namespace URI, local)`, resolving the prefix against this schema's own
    /// declarations (an unprefixed name is in the target namespace).
    fn resolve_qname(&self, qname: &str) -> Result<(String, String)> {
        match qname.split_once(':') {
            None => Ok((self.target_namespace.clone(), qname.to_owned())),
            Some((prefix, local)) => {
                let namespace = self.prefixes.get(prefix).with_context(|| {
                    format!("{}: unbound prefix `{prefix}` in `{qname}`", self.file)
                })?;
                Ok((namespace.clone(), local.to_owned()))
            }
        }
    }
}

/// Whether an element is in the XML Schema namespace.
fn is_xsd(element: &RawElement, interner: &Interner) -> bool {
    element.name.namespace.map(|n| interner.resolve(n)) == Some(XSD)
}

/// An unprefixed attribute's value.
fn attribute(element: &RawElement, interner: &Interner, local: &str) -> Option<String> {
    element
        .attributes
        .iter()
        .find(|a| a.name.prefix.is_none() && interner.resolve(a.name.local) == local)
        .and_then(|a| String::from_utf8(a.value.to_vec()).ok())
}

/// Whether a particle's `maxOccurs` allows more than one occurrence.
fn repeats(element: &RawElement, interner: &Interner) -> bool {
    match attribute(element, interner, "maxOccurs").as_deref() {
        None | Some("1") => false,
        Some(_) => true,
    }
}

/// Every parsed schema, keyed by target namespace, so cross-schema group and element references
/// resolve.
pub struct SchemaSet {
    schemas: Vec<Schema>,
}

impl SchemaSet {
    /// Builds a set from parsed schemas.
    pub fn new(schemas: Vec<Schema>) -> Self {
        Self { schemas }
    }

    /// The schemas in the set, in the order they were given.
    pub fn schemas(&self) -> &[Schema] {
        &self.schemas
    }

    fn schema(&self, namespace: &str) -> Option<&Schema> {
        self.schemas
            .iter()
            .find(|s| s.target_namespace == namespace)
    }

    /// Flattens every complex type of `schema`, in document order.
    pub fn flatten_schema(&self, schema: &Schema) -> Result<Vec<FlatType>> {
        schema
            .complex_types
            .iter()
            .map(|(symbol, particle)| self.flatten(symbol, particle.as_ref()))
            .collect()
    }

    /// Flattens one complex type's content model.
    fn flatten(&self, symbol: &str, particle: Option<&Particle>) -> Result<FlatType> {
        let Some(particle) = particle else {
            return Ok(FlatType {
                symbol: symbol.to_owned(),
                model: ContentModel::Empty,
                slots: Vec::new(),
            });
        };
        let model = self.model_of(particle, &mut Vec::new())?;
        let mut collected = Vec::new();
        let mut cursor = 0u16;
        self.walk(particle, &mut cursor, &mut collected, &mut Vec::new())?;

        // One element reachable at two different ranks has no single position; record it as
        // ambiguous rather than picking one, so neither placement nor the audit uses it.
        let mut slots: Vec<Slot> = Vec::new();
        for slot in collected {
            match slots
                .iter_mut()
                .find(|s| s.local == slot.local && s.namespace == slot.namespace)
            {
                Some(existing) => {
                    existing.repeatable |= slot.repeatable;
                    if existing.rank != slot.rank {
                        existing.ambiguous = true;
                    }
                    if existing.complex_type.is_none() {
                        existing.complex_type = slot.complex_type;
                    }
                }
                None => slots.push(slot),
            }
        }
        slots.sort_by(|a, b| {
            a.rank
                .cmp(&b.rank)
                .then_with(|| a.namespace.cmp(&b.namespace))
                .then_with(|| a.local.cmp(&b.local))
        });
        Ok(FlatType {
            symbol: symbol.to_owned(),
            model,
            slots,
        })
    }

    /// The type's top-level content model, looking through a single group reference.
    fn model_of(&self, particle: &Particle, seen: &mut Vec<String>) -> Result<ContentModel> {
        Ok(match particle {
            Particle::Sequence(_) => ContentModel::Sequence,
            Particle::Choice(_) => ContentModel::Choice,
            Particle::All(_) => ContentModel::All,
            Particle::Element(_) => ContentModel::Sequence,
            Particle::Wildcard => ContentModel::Empty,
            Particle::GroupReference { namespace, name } => {
                let key = format!("{namespace}#{name}");
                if seen.contains(&key) {
                    bail!("cyclic xsd:group reference at {key}");
                }
                seen.push(key);
                let group = self.group(namespace, name)?;
                self.model_of(group, seen)?
            }
            Particle::Derivation { kind, base, own } => {
                let resolved = self.resolve_derivation(*kind, base, own.as_deref(), seen)?;
                self.model_of(&resolved, seen)?
            }
        })
    }

    fn group(&self, namespace: &str, name: &str) -> Result<&Particle> {
        self.schema(namespace)
            .and_then(|s| s.groups.get(name))
            .with_context(|| format!("unresolved xsd:group `{name}` in `{namespace}`"))
    }

    /// A named complex type's own particle (`None` when it declares no child elements at all —
    /// an attribute-only or empty type), looked up by `(namespace, symbol)` across the set.
    fn complex_type(&self, namespace: &str, symbol: &str) -> Result<&Option<Particle>> {
        let schema = self
            .schema(namespace)
            .with_context(|| format!("unresolved complex type base `{symbol}` in `{namespace}`"))?;
        schema
            .complex_types
            .iter()
            .find(|(name, _)| name == symbol)
            .map(|(_, particle)| particle)
            .with_context(|| format!("{}: unresolved complex type base `{symbol}`", schema.file))
    }

    /// Resolves one `complexContent` derivation into a concrete, `Derivation`-free particle.
    ///
    /// `extension` splices the base's own resolved particle *before* this type's own — recursing
    /// through the base in case it is itself derived, so a multi-level extension chain (e.g.
    /// `wml.xsd`'s `CT_MoveBookmark` → `CT_Bookmark` → `CT_BookmarkRange` → `CT_MarkupRange` →
    /// `CT_Markup`) resolves to the full, correctly ordered chain rather than just its immediate
    /// parent. `restriction` discards the base's particle outright: a restriction always restates,
    /// alone, whatever subset of the base's model it keeps, so the base contributes nothing to a
    /// restriction's ranking.
    fn resolve_derivation(
        &self,
        kind: DerivationKind,
        base: &(String, String),
        own: Option<&Particle>,
        seen: &mut Vec<String>,
    ) -> Result<Particle> {
        match kind {
            DerivationKind::Restriction => Ok(match own {
                Some(particle) => particle.clone(),
                None => Particle::Sequence(Vec::new()),
            }),
            DerivationKind::Extension => {
                let key = format!("extends#{}#{}", base.0, base.1);
                if seen.contains(&key) {
                    bail!("cyclic complexContent base reference at {key}");
                }
                seen.push(key);
                let base_particle = self.complex_type(&base.0, &base.1)?;
                let resolved_base = match base_particle {
                    Some(particle) => Some(self.resolve(particle, seen)?),
                    None => None,
                };
                seen.pop();
                Ok(match (resolved_base, own) {
                    (None, None) => Particle::Sequence(Vec::new()),
                    (None, Some(particle)) => particle.clone(),
                    (Some(resolved), None) => resolved,
                    (Some(resolved), Some(particle)) => {
                        Particle::Sequence(vec![resolved, particle.clone()])
                    }
                })
            }
        }
    }

    /// Resolves a particle to one with no [`Particle::Derivation`] anywhere at its top level —
    /// every other variant is returned unchanged (a nested `Derivation` cannot occur: XSD does not
    /// allow `complexContent`/`simpleContent` inside an `extension`/`restriction`, so
    /// [`complex_content_derivation`]'s `own` particle is always `Derivation`-free already).
    fn resolve(&self, particle: &Particle, seen: &mut Vec<String>) -> Result<Particle> {
        match particle {
            Particle::Derivation { kind, base, own } => {
                self.resolve_derivation(*kind, base, own.as_deref(), seen)
            }
            other => Ok(other.clone()),
        }
    }

    /// Walks a particle, assigning ranks (see the [module docs](self)).
    fn walk(
        &self,
        particle: &Particle,
        cursor: &mut u16,
        out: &mut Vec<Slot>,
        stack: &mut Vec<String>,
    ) -> Result<()> {
        match particle {
            Particle::Wildcard => {}
            Particle::Element(element) => {
                let complex_type = match &element.complex_type {
                    Some(ty) => Some(ty.clone()),
                    None => self.global_element_type(&element.namespace, &element.local)?,
                };
                out.push(Slot {
                    namespace: element.namespace.clone(),
                    local: element.local.clone(),
                    complex_type,
                    rank: *cursor,
                    repeatable: element.repeatable,
                    ambiguous: false,
                });
            }
            Particle::GroupReference { namespace, name } => {
                let key = format!("{namespace}#{name}");
                if stack.contains(&key) {
                    bail!("cyclic xsd:group reference at {key}");
                }
                stack.push(key);
                let group = self.group(namespace, name)?;
                self.walk(group, cursor, out, stack)?;
                stack.pop();
            }
            Particle::Sequence(members) => {
                for (index, member) in members.iter().enumerate() {
                    if index > 0 {
                        *cursor = cursor.saturating_add(1);
                    }
                    self.walk(member, cursor, out, stack)?;
                }
            }
            Particle::Choice(branches) | Particle::All(branches) => {
                let base = *cursor;
                let mut highest = base;
                for branch in branches {
                    *cursor = base;
                    self.walk(branch, cursor, out, stack)?;
                    highest = highest.max(*cursor);
                }
                *cursor = highest;
            }
            Particle::Derivation { kind, base, own } => {
                let resolved = self.resolve_derivation(*kind, base, own.as_deref(), stack)?;
                self.walk(&resolved, cursor, out, stack)?;
            }
        }
        Ok(())
    }

    /// The complex type of a globally declared element, when it has one.
    fn global_element_type(
        &self,
        namespace: &str,
        local: &str,
    ) -> Result<Option<(String, String)>> {
        let Some(schema) = self.schema(namespace) else {
            return Ok(None);
        };
        let Some(qname) = schema.global_elements.get(local) else {
            return Ok(None);
        };
        let (ns, symbol) = schema.resolve_qname(qname)?;
        Ok(symbol.starts_with("CT_").then_some((ns, symbol)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DML: &str = "urn:dml";

    fn schema_set(sources: &[(&str, &str)]) -> SchemaSet {
        let schemas = sources
            .iter()
            .map(|(file, xsd)| parse(file, xsd.as_bytes()).expect("parses"))
            .collect();
        SchemaSet::new(schemas)
    }

    fn dml(body: &str) -> String {
        format!(
            r#"<xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:a="{DML}"
                targetNamespace="{DML}" elementFormDefault="qualified">{body}</xsd:schema>"#
        )
    }

    #[test]
    fn a_sequence_ranks_its_members_in_declaration_order() {
        let set = schema_set(&[(
            "t.xsd",
            &dml(r#"<xsd:complexType name="CT_A">
                     <xsd:sequence>
                       <xsd:element name="first" type="CT_Empty"/>
                       <xsd:element name="second" type="CT_Empty"/>
                       <xsd:element name="third" type="CT_Empty"/>
                     </xsd:sequence>
                   </xsd:complexType>"#),
        )]);
        let flat = set.flatten_schema(&set.schemas()[0]).expect("flattens");
        assert_eq!(flat[0].model, ContentModel::Sequence);
        let ranks: Vec<_> = flat[0]
            .slots
            .iter()
            .map(|s| (s.local.as_str(), s.rank))
            .collect();
        assert_eq!(ranks, vec![("first", 0), ("second", 1), ("third", 2)]);
    }

    #[test]
    fn the_branches_of_a_choice_share_one_rank() {
        let set = schema_set(&[(
            "t.xsd",
            &dml(r#"<xsd:group name="EG_Fill">
                     <xsd:choice>
                       <xsd:element name="noFill" type="CT_Empty"/>
                       <xsd:element name="solidFill" type="CT_Empty"/>
                     </xsd:choice>
                   </xsd:group>
                   <xsd:complexType name="CT_A">
                     <xsd:sequence>
                       <xsd:element name="xfrm" type="CT_Empty"/>
                       <xsd:group ref="EG_Fill" minOccurs="0"/>
                       <xsd:element name="ln" type="CT_Empty"/>
                     </xsd:sequence>
                   </xsd:complexType>"#),
        )]);
        let flat = set.flatten_schema(&set.schemas()[0]).expect("flattens");
        let ranks: Vec<_> = flat[0]
            .slots
            .iter()
            .map(|s| (s.local.as_str(), s.rank))
            .collect();
        assert_eq!(
            ranks,
            vec![("xfrm", 0), ("noFill", 1), ("solidFill", 1), ("ln", 2)]
        );
    }

    #[test]
    fn a_top_level_choice_imposes_no_order() {
        let set = schema_set(&[(
            "t.xsd",
            &dml(r#"<xsd:complexType name="CT_Path">
                     <xsd:choice minOccurs="0" maxOccurs="unbounded">
                       <xsd:element name="moveTo" type="CT_Empty"/>
                       <xsd:element name="lnTo" type="CT_Empty"/>
                     </xsd:choice>
                   </xsd:complexType>"#),
        )]);
        let flat = set.flatten_schema(&set.schemas()[0]).expect("flattens");
        assert_eq!(flat[0].model, ContentModel::Choice);
        assert!(flat[0].slots.iter().all(|s| s.rank == 0 && s.repeatable));
    }

    #[test]
    fn an_all_model_is_recorded_as_unordered() {
        let set = schema_set(&[(
            "t.xsd",
            &dml(r#"<xsd:complexType name="CT_A">
                     <xsd:all>
                       <xsd:element name="one" type="CT_Empty"/>
                       <xsd:element name="two" type="CT_Empty"/>
                     </xsd:all>
                   </xsd:complexType>"#),
        )]);
        let flat = set.flatten_schema(&set.schemas()[0]).expect("flattens");
        assert_eq!(flat[0].model, ContentModel::All);
        assert!(flat[0].slots.iter().all(|s| s.rank == 0));
    }

    #[test]
    fn a_group_reference_resolves_across_schemas() {
        const PML: &str = "urn:pml";
        let set = schema_set(&[
            (
                "dml.xsd",
                &dml(r#"<xsd:group name="EG_Fill">
                         <xsd:choice><xsd:element name="noFill" type="CT_Empty"/></xsd:choice>
                       </xsd:group>"#),
            ),
            (
                "pml.xsd",
                &format!(
                    r#"<xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:a="{DML}"
                        targetNamespace="{PML}" elementFormDefault="qualified">
                        <xsd:complexType name="CT_Bg">
                          <xsd:sequence>
                            <xsd:group ref="a:EG_Fill"/>
                            <xsd:element name="extLst" type="CT_Empty"/>
                          </xsd:sequence>
                        </xsd:complexType>
                       </xsd:schema>"#
                ),
            ),
        ]);
        let flat = set.flatten_schema(&set.schemas()[1]).expect("flattens");
        let slots: Vec<_> = flat[0]
            .slots
            .iter()
            .map(|s| (s.namespace.as_str(), s.local.as_str(), s.rank))
            .collect();
        assert_eq!(slots, vec![(DML, "noFill", 0), (PML, "extLst", 1)]);
    }

    #[test]
    fn an_element_reachable_at_two_ranks_is_marked_ambiguous() {
        let set = schema_set(&[(
            "t.xsd",
            &dml(r#"<xsd:complexType name="CT_A">
                     <xsd:sequence>
                       <xsd:element name="x" type="CT_Empty"/>
                       <xsd:element name="y" type="CT_Empty"/>
                       <xsd:element name="x" type="CT_Empty"/>
                     </xsd:sequence>
                   </xsd:complexType>"#),
        )]);
        let flat = set.flatten_schema(&set.schemas()[0]).expect("flattens");
        let x = flat[0].slots.iter().find(|s| s.local == "x").expect("x");
        assert!(
            x.ambiguous,
            "an element at two ranks has no single position"
        );
        let y = flat[0].slots.iter().find(|s| s.local == "y").expect("y");
        assert!(!y.ambiguous);
    }

    #[test]
    fn an_attribute_only_type_has_an_empty_model() {
        let set = schema_set(&[(
            "t.xsd",
            &dml(r#"<xsd:complexType name="CT_A">
                     <xsd:attribute name="val" type="xsd:string"/>
                   </xsd:complexType>"#),
        )]);
        let flat = set.flatten_schema(&set.schemas()[0]).expect("flattens");
        assert_eq!(flat[0].model, ContentModel::Empty);
        assert!(flat[0].slots.is_empty());
    }

    #[test]
    fn an_element_reference_carries_the_global_declaration_type() {
        let set = schema_set(&[(
            "t.xsd",
            &dml(r#"<xsd:element name="graphic" type="CT_GraphicalObject"/>
                   <xsd:complexType name="CT_A">
                     <xsd:sequence><xsd:element ref="a:graphic"/></xsd:sequence>
                   </xsd:complexType>"#),
        )]);
        let flat = set.flatten_schema(&set.schemas()[0]).expect("flattens");
        let slot = &flat[0].slots[0];
        assert_eq!(slot.local, "graphic");
        assert_eq!(
            slot.complex_type,
            Some((DML.to_owned(), "CT_GraphicalObject".to_owned()))
        );
    }

    /// Finds one flattened type by symbol — the fixtures below declare several, and index-by-
    /// declaration-order would silently start passing for the wrong type if the fixture were
    /// reordered.
    fn find<'a>(flat: &'a [FlatType], symbol: &str) -> &'a FlatType {
        flat.iter()
            .find(|f| f.symbol == symbol)
            .unwrap_or_else(|| panic!("no flattened type named {symbol}"))
    }

    #[test]
    fn extension_splices_the_whole_base_chain_before_the_derived_types_own_children() {
        // Mirrors wml.xsd's shape: a multi-level `complexContent`/`extension` chain (there,
        // CT_MoveBookmark -> CT_Bookmark -> CT_BookmarkRange -> CT_MarkupRange -> CT_Markup; here,
        // CT_Derived -> CT_Mid -> CT_Root), where each level's own children must land *after*
        // everything the level below it contributed — not merely after its immediate parent's.
        let set = schema_set(&[(
            "t.xsd",
            &dml(r#"<xsd:complexType name="CT_Root">
                     <xsd:sequence>
                       <xsd:element name="alpha" type="CT_Empty"/>
                     </xsd:sequence>
                   </xsd:complexType>
                   <xsd:complexType name="CT_Mid">
                     <xsd:complexContent>
                       <xsd:extension base="CT_Root">
                         <xsd:sequence>
                           <xsd:element name="beta" type="CT_Empty"/>
                         </xsd:sequence>
                       </xsd:extension>
                     </xsd:complexContent>
                   </xsd:complexType>
                   <xsd:complexType name="CT_Derived">
                     <xsd:complexContent>
                       <xsd:extension base="CT_Mid">
                         <xsd:sequence>
                           <xsd:element name="gamma" type="CT_Empty"/>
                         </xsd:sequence>
                       </xsd:extension>
                     </xsd:complexContent>
                   </xsd:complexType>"#),
        )]);
        let flat = set.flatten_schema(&set.schemas()[0]).expect("flattens");

        let derived = find(&flat, "CT_Derived");
        assert_eq!(derived.model, ContentModel::Sequence);
        let ranks: Vec<_> = derived
            .slots
            .iter()
            .map(|s| (s.local.as_str(), s.rank))
            .collect();
        assert_eq!(
            ranks,
            vec![("alpha", 0), ("beta", 1), ("gamma", 2)],
            "the base chain's children must precede the derived type's own, oldest ancestor first"
        );

        // The intermediate level splices correctly too, independent of its own child.
        let mid = find(&flat, "CT_Mid");
        let mid_ranks: Vec<_> = mid
            .slots
            .iter()
            .map(|s| (s.local.as_str(), s.rank))
            .collect();
        assert_eq!(mid_ranks, vec![("alpha", 0), ("beta", 1)]);
    }

    #[test]
    fn restriction_replaces_the_base_particle_instead_of_appending() {
        let set = schema_set(&[(
            "t.xsd",
            &dml(r#"<xsd:complexType name="CT_Base">
                     <xsd:sequence>
                       <xsd:element name="alpha" type="CT_Empty"/>
                       <xsd:element name="beta" type="CT_Empty"/>
                       <xsd:element name="gamma" type="CT_Empty"/>
                     </xsd:sequence>
                   </xsd:complexType>
                   <xsd:complexType name="CT_Narrowed">
                     <xsd:complexContent>
                       <xsd:restriction base="CT_Base">
                         <xsd:sequence>
                           <xsd:element name="alpha" type="CT_Empty"/>
                         </xsd:sequence>
                       </xsd:restriction>
                     </xsd:complexContent>
                   </xsd:complexType>"#),
        )]);
        let flat = set.flatten_schema(&set.schemas()[0]).expect("flattens");
        let narrowed = find(&flat, "CT_Narrowed");
        let locals: Vec<_> = narrowed.slots.iter().map(|s| s.local.as_str()).collect();
        assert_eq!(
            locals,
            vec!["alpha"],
            "a restriction must stand alone, never append to the base's own children"
        );
    }

    #[test]
    fn extension_of_an_attribute_only_base_takes_its_own_model() {
        let set = schema_set(&[(
            "t.xsd",
            &dml(r#"<xsd:complexType name="CT_Base">
                     <xsd:attribute name="id" type="xsd:string"/>
                   </xsd:complexType>
                   <xsd:complexType name="CT_Derived">
                     <xsd:complexContent>
                       <xsd:extension base="CT_Base">
                         <xsd:choice>
                           <xsd:element name="x" type="CT_Empty"/>
                           <xsd:element name="y" type="CT_Empty"/>
                         </xsd:choice>
                       </xsd:extension>
                     </xsd:complexContent>
                   </xsd:complexType>"#),
        )]);
        let flat = set.flatten_schema(&set.schemas()[0]).expect("flattens");
        let derived = find(&flat, "CT_Derived");
        assert_eq!(derived.model, ContentModel::Choice);
        assert!(derived.slots.iter().all(|s| s.rank == 0));
    }

    #[test]
    fn simple_content_contributes_no_particle() {
        let set = schema_set(&[(
            "t.xsd",
            &dml(r#"<xsd:complexType name="CT_Text">
                     <xsd:simpleContent>
                       <xsd:extension base="xsd:string">
                         <xsd:attribute name="space" type="xsd:string"/>
                       </xsd:extension>
                     </xsd:simpleContent>
                   </xsd:complexType>"#),
        )]);
        let flat = set.flatten_schema(&set.schemas()[0]).expect("flattens");
        let text = find(&flat, "CT_Text");
        assert_eq!(text.model, ContentModel::Empty);
        assert!(text.slots.is_empty());
    }

    #[test]
    fn a_cyclic_complex_content_base_reference_fails_loudly() {
        let set = schema_set(&[(
            "t.xsd",
            &dml(r#"<xsd:complexType name="CT_A">
                     <xsd:complexContent>
                       <xsd:extension base="CT_B"/>
                     </xsd:complexContent>
                   </xsd:complexType>
                   <xsd:complexType name="CT_B">
                     <xsd:complexContent>
                       <xsd:extension base="CT_A"/>
                     </xsd:complexContent>
                   </xsd:complexType>"#),
        )]);
        let error = set
            .flatten_schema(&set.schemas()[0])
            .expect_err("a cyclic base chain must not silently resolve");
        assert!(
            error.to_string().contains("cyclic"),
            "unexpected error: {error}"
        );
    }
}
