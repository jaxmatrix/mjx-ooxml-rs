//! Non-mutating MCE resolution: given the namespaces a consumer understands, produce a flattened
//! *view* of the tree — winning `Choice`/`Fallback` selected, `Ignorable`/`ProcessContent` applied,
//! `mc:*` markup stripped — **without** modifying the stored [`RawDocument`].

use std::collections::HashSet;

use mjx_ooxml_core::{Interner, RawAttribute, RawDocument, RawElement, RawName, RawNode};

use crate::scope::NamespaceScope;
use crate::MARKUP_COMPATIBILITY_2006 as MC_NS;

/// The set of namespace URIs a consumer understands (used to pick `Choice`es and to keep otherwise-
/// ignorable content).
#[derive(Debug, Default, Clone)]
pub struct UnderstoodNamespaces {
    set: HashSet<String>,
}

impl UnderstoodNamespaces {
    /// An empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a set from an iterator of URIs.
    pub fn from_uris<I, S>(uris: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            set: uris.into_iter().map(Into::into).collect(),
        }
    }

    /// Adds a URI.
    pub fn insert(&mut self, uri: impl Into<String>) -> &mut Self {
        self.set.insert(uri.into());
        self
    }

    /// Whether `uri` is understood.
    #[must_use]
    pub fn contains(&self, uri: &str) -> bool {
        self.set.contains(uri)
    }
}

/// An error encountered while resolving markup compatibility.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ResolveError {
    /// A namespace listed in `mc:MustUnderstand` is not understood.
    #[error("required namespace is not understood: {0}")]
    MustUnderstand(String),
    /// An `mc:AlternateContent` block was malformed (e.g. a `Choice` without `Requires`).
    #[error("malformed AlternateContent: {0}")]
    MalformedAlternateContent(String),
}

/// A node in the resolved (flattened) view. Comments and PIs are omitted — this is an effective
/// *content* view, not a serialization.
#[derive(Debug)]
pub enum ResolvedNode<'a> {
    /// An element with its `mc:*`/ignored attributes filtered and children flattened.
    Element(ResolvedElement<'a>),
    /// Character data (raw bytes, borrowed from the source tree).
    Text(&'a [u8]),
    /// CDATA content (raw bytes, borrowed from the source tree).
    CData(&'a [u8]),
}

/// A resolved element: borrows its name/attributes from the source tree; children are flattened.
#[derive(Debug)]
pub struct ResolvedElement<'a> {
    /// The originating element (for its name and raw attributes).
    pub source: &'a RawElement,
    /// Kept attributes (in order): `mc:*` and ignored-namespace attributes are removed.
    pub attributes: Vec<&'a RawAttribute>,
    /// Flattened child nodes.
    pub children: Vec<ResolvedNode<'a>>,
}

impl<'a> ResolvedElement<'a> {
    /// The element's name.
    #[must_use]
    pub fn name(&self) -> &'a RawName {
        &self.source.name
    }
}

/// The set of namespace URIs made ignorable / process-content by ancestors, accumulated downward.
#[derive(Debug, Default, Clone)]
struct Context {
    ignorable: HashSet<String>,
    process_content: HashSet<String>,
}

/// Resolves a document's root element against the understood namespaces.
///
/// # Errors
/// Returns [`ResolveError`] on an unsatisfied `mc:MustUnderstand` or malformed `AlternateContent`.
///
/// # Examples
///
/// ```
/// use mjx_mce::{resolve, UnderstoodNamespaces};
/// use mjx_xml::fidelity;
///
/// // An AlternateContent block offering a modern Choice and a legacy Fallback.
/// let xml = br#"<r xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:new="urn:new" xmlns:old="urn:old"><mc:AlternateContent><mc:Choice Requires="new"><new:shape/></mc:Choice><mc:Fallback><old:shape/></mc:Fallback></mc:AlternateContent></r>"#;
/// let doc = fidelity::parse(xml).unwrap();
///
/// // We understand `urn:new`, so the Choice wins over the Fallback.
/// let root = resolve(&doc, &UnderstoodNamespaces::from_uris(["urn:new"])).unwrap();
/// assert_eq!(root.children.len(), 1);
///
/// // We understand neither, so the Fallback is used.
/// let root = resolve(&doc, &UnderstoodNamespaces::new()).unwrap();
/// assert_eq!(root.children.len(), 1);
/// ```
pub fn resolve<'a>(
    doc: &'a RawDocument,
    understood: &UnderstoodNamespaces,
) -> Result<ResolvedElement<'a>, ResolveError> {
    let mut scope = NamespaceScope::new();
    resolve_element(
        &doc.root,
        &doc.interner,
        &mut scope,
        &Context::default(),
        understood,
    )
}

fn resolve_element<'a>(
    element: &'a RawElement,
    interner: &Interner,
    scope: &mut NamespaceScope,
    ctx: &Context,
    understood: &UnderstoodNamespaces,
) -> Result<ResolvedElement<'a>, ResolveError> {
    scope.push_element(element, interner);
    let ctx = extend_context(element, interner, scope, ctx, understood)?;

    let attributes = element
        .attributes
        .iter()
        .filter(|a| keep_attribute(a, interner, scope, &ctx, understood))
        .collect();

    let mut children = Vec::new();
    for child in &element.children {
        resolve_child(child, interner, scope, &ctx, understood, &mut children)?;
    }

    scope.pop();
    Ok(ResolvedElement {
        source: element,
        attributes,
        children,
    })
}

fn resolve_child<'a>(
    child: &'a RawNode,
    interner: &Interner,
    scope: &mut NamespaceScope,
    ctx: &Context,
    understood: &UnderstoodNamespaces,
    out: &mut Vec<ResolvedNode<'a>>,
) -> Result<(), ResolveError> {
    let element = match child {
        RawNode::Element(e) => e,
        RawNode::Text(b) => {
            out.push(ResolvedNode::Text(b));
            return Ok(());
        }
        RawNode::CData(b) => {
            out.push(ResolvedNode::CData(b));
            return Ok(());
        }
        // Comments, PIs, declarations, doctype: not content — dropped from the resolved view.
        _ => return Ok(()),
    };

    let namespace = element_namespace(element, interner);
    let local = interner.resolve(element.name.local);

    if namespace == Some(MC_NS) && local == "AlternateContent" {
        return resolve_alternate_content(element, interner, scope, ctx, understood, out);
    }

    if let Some(uri) = namespace {
        if ctx.ignorable.contains(uri) && !understood.contains(uri) {
            // Ignored element: either hoist its children (ProcessContent) or drop it entirely.
            if ctx.process_content.contains(uri) {
                scope.push_element(element, interner);
                let inner = extend_context(element, interner, scope, ctx, understood)?;
                for grandchild in &element.children {
                    resolve_child(grandchild, interner, scope, &inner, understood, out)?;
                }
                scope.pop();
            }
            return Ok(());
        }
    }

    let resolved = resolve_element(element, interner, scope, ctx, understood)?;
    out.push(ResolvedNode::Element(resolved));
    Ok(())
}

fn resolve_alternate_content<'a>(
    element: &'a RawElement,
    interner: &Interner,
    scope: &mut NamespaceScope,
    ctx: &Context,
    understood: &UnderstoodNamespaces,
    out: &mut Vec<ResolvedNode<'a>>,
) -> Result<(), ResolveError> {
    scope.push_element(element, interner);

    let mut chosen: Option<&RawElement> = None;
    let mut fallback: Option<&RawElement> = None;

    for child in &element.children {
        let RawNode::Element(candidate) = child else {
            continue;
        };
        if element_namespace(candidate, interner) != Some(MC_NS) {
            continue;
        }
        match interner.resolve(candidate.name.local) {
            "Choice" => {
                let requires =
                    unqualified_attr(candidate, interner, "Requires").ok_or_else(|| {
                        ResolveError::MalformedAlternateContent(
                            "Choice without Requires".to_owned(),
                        )
                    })?;
                scope.push_element(candidate, interner);
                let satisfied = requires.split_whitespace().all(|prefix| {
                    scope
                        .resolve_prefix(prefix)
                        .is_some_and(|uri| understood.contains(uri))
                });
                scope.pop();
                if satisfied {
                    chosen = Some(candidate);
                    break;
                }
            }
            "Fallback" if fallback.is_none() => fallback = Some(candidate),
            _ => {}
        }
    }

    if let Some(winner) = chosen.or(fallback) {
        scope.push_element(winner, interner);
        let inner = extend_context(winner, interner, scope, ctx, understood)?;
        for grandchild in &winner.children {
            resolve_child(grandchild, interner, scope, &inner, understood, out)?;
        }
        scope.pop();
    }

    scope.pop();
    Ok(())
}

/// Parses an element's `mc:Ignorable`/`ProcessContent`/`MustUnderstand` (scope must already include
/// this element) and returns the context extended for its subtree. Errors if `MustUnderstand` lists
/// an unknown namespace.
fn extend_context(
    element: &RawElement,
    interner: &Interner,
    scope: &NamespaceScope,
    ctx: &Context,
    understood: &UnderstoodNamespaces,
) -> Result<Context, ResolveError> {
    let mut ignorable = Vec::new();
    let mut process_content = Vec::new();
    let mut must_understand = Vec::new();

    for attr in &element.attributes {
        let Some(prefix) = attr.name.prefix else {
            continue;
        };
        if scope.resolve_prefix(interner.resolve(prefix)) != Some(MC_NS) {
            continue;
        }
        let value = String::from_utf8_lossy(&attr.value);
        let target = match interner.resolve(attr.name.local) {
            "Ignorable" => &mut ignorable,
            "ProcessContent" => &mut process_content,
            "MustUnderstand" => &mut must_understand,
            _ => continue,
        };
        for prefix in value.split_whitespace() {
            if let Some(uri) = scope.resolve_prefix(prefix) {
                target.push(uri.to_owned());
            }
        }
    }

    for uri in &must_understand {
        if !understood.contains(uri) {
            return Err(ResolveError::MustUnderstand(uri.clone()));
        }
    }

    let mut extended = ctx.clone();
    extended.ignorable.extend(ignorable);
    extended.process_content.extend(process_content);
    Ok(extended)
}

/// Whether an attribute survives into the resolved view.
fn keep_attribute(
    attr: &RawAttribute,
    interner: &Interner,
    scope: &NamespaceScope,
    ctx: &Context,
    understood: &UnderstoodNamespaces,
) -> bool {
    match attr.name.prefix.map(|p| interner.resolve(p)) {
        // An `xmlns:foo` declaration: drop only the one binding `mc` to the MC namespace.
        Some("xmlns") => String::from_utf8_lossy(&attr.value) != MC_NS,
        // A prefixed attribute: drop `mc:*` and ignored-namespace attributes.
        Some(prefix) => match scope.resolve_prefix(prefix) {
            Some(uri) if uri == MC_NS => false,
            Some(uri) if ctx.ignorable.contains(uri) && !understood.contains(uri) => false,
            _ => true,
        },
        // Unprefixed attributes are in no namespace — always kept.
        None => true,
    }
}

fn element_namespace<'i>(element: &RawElement, interner: &'i Interner) -> Option<&'i str> {
    element.name.namespace.map(|s| interner.resolve(s))
}

fn unqualified_attr(element: &RawElement, interner: &Interner, name: &str) -> Option<String> {
    element
        .attributes
        .iter()
        .find(|a| a.name.prefix.is_none() && interner.resolve(a.name.local) == name)
        .map(|a| String::from_utf8_lossy(&a.value).into_owned())
}
