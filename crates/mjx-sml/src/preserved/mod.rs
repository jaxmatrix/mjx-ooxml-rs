//! The half of `sml.xsd` this project deliberately does not model — and the narrow, read-only
//! surface that says *what is there* without modelling it.
//!
//! # The measurement
//!
//! `sml.xsd` declares **367** complex types. Nine clusters of them describe features whose data this
//! library neither computes nor refreshes, and together they come to **184** — half the schema:
//!
//! | cluster | `sml.xsd` lines | complex types | root element(s) |
//! |---|---|---|---|
//! | pivot tables and their caches | 560–1712 | **97** | `pivotCacheDefinition`, `pivotCacheRecords`, `pivotTableDefinition` |
//! | shared-workbook revisions | 1860–2117 | 22 | `headers`, `revisions`, `users` |
//! | external workbook references | 3819–3945 | 18 | `externalLink` |
//! | cell metadata | 3189–3355 | 16 | `metadata` |
//! | data connections | 377–559 | 12 | `connections` |
//! | query tables | 1714–1788 | 6 | `queryTable` |
//! | volatile dependencies | 4052–4096 | 5 | `volTypes` |
//! | single-cell XML tables | 3356–3386 | 4 | `singleXmlCells` |
//! | custom XML mappings | 336–376 | 4 | `MapInfo` |
//!
//! # Why the pivot cluster is not modelled, and what would have to change
//!
//! Ninety-seven complex types — **a quarter of the whole schema** — describe one feature, and what
//! they describe is *derived data*: a pivot cache is a snapshot of a source range, and a pivot table
//! is an aggregation over that snapshot. Modelling them faithfully means being able to answer what a
//! consumer would show, and that means evaluating a calculation model this library explicitly does
//! not have (`crates/mjx-sml/src/formula/` reads and rewrites formula *text* and evaluates nothing).
//! A model that held every one of the ninety-seven types and could not say what a single data field
//! aggregated to would be ninety-seven types of decoration over bytes that already round-trip.
//!
//! So the decision, written down rather than deferred: **if the pivot cluster is ever modelled it is
//! a phase of its own**, with a calculation model beneath it. It is not a gap for a later Phase D
//! child to close in an afternoon, and it is not a validation failure — a documented gap never is.
//!
//! The same reasoning, more briefly, for the rest: refreshing a query table, a connection or an
//! external link is **I/O** — opening another workbook, running a query — and evaluating a
//! volatile-dependency graph is calculation. Both are programme non-goals. Revisions, metadata,
//! single-cell tables and XML maps are modellable in principle and simply not worth their weight
//! against the features a caller asks for; they are recorded here so nobody has to re-derive that.
//!
//! # Preserved is not ignored
//!
//! Every part above is carried through a save **byte for byte**, by `mjx-opc`'s part-level
//! copy-on-write, and that is asserted per part kind rather than assumed:
//! `crates/mjx-xlsx/tests/preserved_parts.rs` opens a workbook carrying one of each, edits a cell on
//! the very sheet the pivot table sits on, and compares every one of them against the bytes it went
//! in with. *A part nothing asserts on is a part that silently disappears* — `mjx-vml` sat at 69
//! lines for three phases on the strength of a "later phase" note with no owner, and this module is
//! that note given an owner and a test.
//!
//! # What this module *is*
//!
//! A handful of **read-only identity views**. Each takes the root element of one preserved part,
//! reads the few attributes that answer *"what is this, and what does it point at"*, and hands back
//! an owned value. None of them implements
//! [`ToXml`](mjx_ooxml_core::ToXml), none of them can be constructed from anything but a parsed
//! element, and none of them is ever written back — so there is no path by which reading one can
//! change a byte. That is the difference between this and every other module in this crate, and it
//! is deliberate: a type that can write is a type that has to be complete.
//!
//! [`PivotTableIdentity`] answers the pivot table's name, its cache and its location; the caller
//! gets *"there is a pivot table named Sales on Sheet1 at `A3:D12`, drawing on cache 0"* without a
//! ninety-seven-type model behind it. `mjx-xlsx`'s [`Workbook`] resolves the parts these point at.
//!
//! # Reading a required attribute that is not there is an error, not a substitution
//!
//! Each view follows the crate's rule (`mjx_derive`'s required-attribute accessor states it): an
//! attribute the schema marks `use="required"` has no default, so substituting one would assert
//! something the file does not say. The reader reports
//! [`mjx_ooxml_core::AttributeError::Missing`] instead.
//!
//! A **malformed** value is different from a missing one, and
//! [`PivotTableIdentity::location`](PivotTableIdentity::location) is where the difference shows: the
//! attribute is kept as the text the file wrote and [`range`](PivotTableIdentity::range) answers
//! `None` for one this crate cannot parse. A pivot cache with a `ref` of `"not a range"` is a report
//! about the file, never a panic and never a repaired range.
//!
//! [`Workbook`]: https://docs.rs/mjx-xlsx

mod links;
mod pivot;
mod shared_workbook;
mod xml_maps;

pub use links::{ConnectionIdentity, ExternalLinkIdentity, ExternalLinkTarget, QueryTableIdentity};
pub use pivot::{PivotCacheIdentity, PivotCacheSource, PivotTableIdentity};
pub use shared_workbook::{
    RevisionHeadersIdentity, RevisionSession, SharedWorkbookUser, SharedWorkbookUsersIdentity,
};
pub use xml_maps::{XmlMapIdentity, XmlMapsIdentity};

use mjx_ooxml_core::{AttributeError, FromXmlError, Interner, RawAttribute, RawElement};

use crate::error::SmlError;
use mjx_ooxml_types::namespaces::SML;

/// The prefix `root` binds to the relationship-reference namespace, or `None` when it binds none.
///
/// Every `read_root` in this module takes that prefix rather than assuming `r`, for the reason
/// `crates/mjx-sml/src/leaf.rs` gives: the fidelity reader records an attribute's literal prefix and never
/// resolves it, so `r:id` is only `r:id` under the prefix *this part* declared. `None` means the
/// part can carry no relationship reference at all, however an attribute in it is spelled.
///
/// A caller holding a whole-part model reads the prefix off that model instead — see
/// [`WorkbookPart::relationship_prefix`](crate::WorkbookPart::relationship_prefix). This is the
/// same question asked of a part nothing models.
#[must_use]
pub fn relationship_prefix<'a>(root: &RawElement, interner: &'a Interner) -> Option<&'a str> {
    crate::leaf::namespace_prefix(
        &root.attributes,
        interner,
        crate::leaf::RELATIONSHIP_REFERENCE,
    )
}

/// Whether `element` is the SpreadsheetML element named `local`.
///
/// Both conformance worlds' namespace URIs are accepted, as everywhere else in this crate: a Strict
/// document is read as readily as a Transitional one.
#[must_use]
pub(crate) fn is_sml(element: &RawElement, interner: &Interner, local: &str) -> bool {
    element.name.namespace.is_some_and(|symbol| {
        let uri = interner.resolve(symbol);
        uri == SML.transitional || Some(uri) == SML.strict
    }) && interner.resolve(element.name.local) == local
}

/// The first SpreadsheetML child of `element` named `local`, if any.
#[must_use]
pub(crate) fn sml_child<'a>(
    element: &'a RawElement,
    interner: &Interner,
    local: &str,
) -> Option<&'a RawElement> {
    element.children.iter().find_map(|node| match node {
        mjx_ooxml_core::RawNode::Element(child) if is_sml(child, interner, local) => Some(child),
        _ => None,
    })
}

/// Every SpreadsheetML child of `element` named `local`, in document order.
pub(crate) fn sml_children<'a>(
    element: &'a RawElement,
    interner: &'a Interner,
    local: &'a str,
) -> impl Iterator<Item = &'a RawElement> + 'a {
    element.children.iter().filter_map(move |node| match node {
        mjx_ooxml_core::RawNode::Element(child) if is_sml(child, interner, local) => Some(child),
        _ => None,
    })
}

/// An unprefixed attribute's decoded text, or `None` when it is absent.
///
/// # Errors
/// [`SmlError::Model`] if the value is not UTF-8 or carries a reference that will not decode — both
/// reachable from an untrusted file, and both reported rather than panicked on.
pub(crate) fn optional_text(
    attributes: &[RawAttribute],
    interner: &Interner,
    local: &'static str,
) -> Result<Option<String>, SmlError> {
    let Some(attribute) = mjx_xml::attribute::find(attributes, interner, None, local) else {
        return Ok(None);
    };
    Ok(Some(
        mjx_xml::attribute::decoded_value(attribute, local)
            .map_err(FromXmlError::from)?
            .into_owned(),
    ))
}

/// An unprefixed attribute's decoded text.
///
/// # Errors
/// [`SmlError::Model`] wrapping [`AttributeError::Missing`] when the attribute is absent — every
/// caller uses this only for an attribute the schema marks `use="required"`, where no default
/// exists to substitute — or another [`AttributeError`] if the value is malformed.
pub(crate) fn required_text(
    attributes: &[RawAttribute],
    interner: &Interner,
    local: &'static str,
) -> Result<String, SmlError> {
    optional_text(attributes, interner, local)?
        .ok_or_else(|| FromXmlError::from(AttributeError::Missing { attribute: local }).into())
}

/// An unprefixed attribute parsed as an unsigned integer, or `None` when it is absent.
///
/// # Errors
/// [`SmlError::Model`] if the value is present and is not a `xsd:unsignedInt`, or if it will not
/// decode to text at all.
pub(crate) fn optional_unsigned(
    attributes: &[RawAttribute],
    interner: &Interner,
    local: &'static str,
) -> Result<Option<u32>, SmlError> {
    let Some(text) = optional_text(attributes, interner, local)? else {
        return Ok(None);
    };
    text.trim().parse::<u32>().map(Some).map_err(|error| {
        FromXmlError::from(AttributeError::InvalidValue {
            attribute: local,
            detail: error.to_string(),
        })
        .into()
    })
}

/// An unprefixed attribute parsed as an unsigned integer.
///
/// # Errors
/// [`SmlError::Model`] when a `use="required"` attribute is absent, or when its value is not a
/// `xsd:unsignedInt`.
pub(crate) fn required_unsigned(
    attributes: &[RawAttribute],
    interner: &Interner,
    local: &'static str,
) -> Result<u32, SmlError> {
    optional_unsigned(attributes, interner, local)?
        .ok_or_else(|| FromXmlError::from(AttributeError::Missing { attribute: local }).into())
}

/// An `xsd:boolean` attribute, or `default` when it is absent.
///
/// Read through [`on_off`](mjx_ooxml_types::support::on_off) — the normalizer every `xsd:boolean`
/// attribute in this crate goes through, `CT_PrintOptions@gridLines` included. It accepts `on` and
/// `off` beside XSD's own four spellings, which is a *wider* grammar than `xsd:boolean` states; that
/// is deliberate and crate-wide, because a file that writes `on` is a file this library reads rather
/// than rejects, and nothing here ever writes the value back.
///
/// # Errors
/// [`SmlError::Model`] for a value none of the six spellings match, or one that will not decode to
/// text.
pub(crate) fn optional_boolean(
    attributes: &[RawAttribute],
    interner: &Interner,
    local: &'static str,
    default: bool,
) -> Result<bool, SmlError> {
    let Some(text) = optional_text(attributes, interner, local)? else {
        return Ok(default);
    };
    mjx_ooxml_types::support::on_off::from_wire(&text).ok_or_else(|| {
        FromXmlError::from(AttributeError::InvalidValue {
            attribute: local,
            detail: format!("{text:?} is not a boolean"),
        })
        .into()
    })
}

/// The relationship identifier `element` carries, under whichever prefix the part binds the
/// relationship-reference namespace to.
///
/// The prefix is the producer's choice — see `crates/mjx-sml/src/leaf.rs`'s own reasoning — so it is resolved
/// from the declarations in scope on the part's root and passed down rather than assumed to be `r`.
///
/// # Errors
/// [`SmlError::Model`] if the value will not decode.
pub(crate) fn relationship_id(
    element: &RawElement,
    interner: &Interner,
    reference_prefix: Option<&str>,
) -> Result<Option<String>, SmlError> {
    Ok(
        crate::leaf::read_relationship_id(&element.attributes, interner, reference_prefix)
            .map_err(FromXmlError::from)?,
    )
}
