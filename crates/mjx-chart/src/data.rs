//! The chart **data layer** — how a series names its source cells and caches the values Office last
//! read from them.
//!
//! A series does not embed its numbers directly; it points at a workbook range (`c:f`, a formula
//! like `Sheet1!$B$2:$B$4`) and carries a **cache** of what that range held when the file was saved
//! (`c:numCache` / `c:strCache`), so a reader that cannot open the embedded workbook still has the
//! values to draw. This tier reads the cache.
//!
//! ```xml
//! <c:val>
//!   <c:numRef>
//!     <c:f>Sheet1!$B$2:$B$4</c:f>
//!     <c:numCache>
//!       <c:formatCode>General</c:formatCode>
//!       <c:ptCount val="3"/>
//!       <c:pt idx="0"><c:v>19.2</c:v></c:pt>
//!       …
//!     </c:numCache>
//!   </c:numRef>
//! </c:val>
//! ```
//!
//! # Fidelity
//!
//! Every container here is the ordered-`content` + `Raw` catch-all shape (see [`crate`] docs), so a
//! `c:formatCode`, a `c:ptCount`, a literal `c:numLit`/`c:strLit` source or an `c:extLst` this tier
//! does not interpret round-trips byte-for-byte in the `Raw` bucket. The two text-bearing leaves —
//! `c:v` (a cached value) and `c:f` (the formula) — keep their subtree opaque and expose a decoding
//! `text` accessor; a value read as `f64` is **parsed on demand** from that preserved wire text, so
//! nothing is ever reformatted on write.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};

use crate::build::{attr_u32, element_text, fidelity_element_impls};

// -------------------------------------------------------------------------------------------------
// Text-bearing leaves — opaque, byte-verbatim
// -------------------------------------------------------------------------------------------------

/// `c:v` (`CT_NumVal`/`CT_StrVal` value) — one cached cell value, as its exact wire text.
///
/// Kept opaque so its bytes re-emit unchanged; [`value`](Self::text) decodes them for reading, and
/// the numeric interpretation is parsed on demand by [`DataPoint::value_f64`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(Value);

impl Value {
    /// The decoded text of the value (entities resolved).
    #[must_use]
    pub fn text(&self) -> String {
        element_text(&self.children)
    }
}

/// `c:f` (`CT_...Ref/@f`) — the workbook formula a reference names, e.g. `Sheet1!$B$2:$B$4`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formula {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(Formula);

impl Formula {
    /// The decoded formula text (entities resolved).
    #[must_use]
    pub fn text(&self) -> String {
        element_text(&self.children)
    }
}

// -------------------------------------------------------------------------------------------------
// c:pt — one cached point
// -------------------------------------------------------------------------------------------------

/// One ordered child of a [`DataPoint`]: its typed value (`c:v`), or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataPointContent {
    /// The point's value (`c:v`).
    Value(Value),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `c:pt` (`CT_NumVal`/`CT_StrVal`) — one cached point, its position given by `@idx`.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct DataPoint {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "v", variant = Value, ty = Value))]
    content: Vec<DataPointContent>,
}

impl DataPoint {
    /// The point's index within its cache (`@idx`) — the position it occupies among the values.
    #[must_use]
    pub fn index(&self, interner: &Interner) -> Option<u32> {
        attr_u32(&self.attributes, interner, "idx")
    }

    /// The point's value element (`c:v`), or `None` if it declares none.
    #[must_use]
    pub fn value(&self) -> Option<&Value> {
        self.content.iter().find_map(|item| match item {
            DataPointContent::Value(value) => Some(value),
            DataPointContent::Raw(_) => None,
        })
    }

    /// The point's value as decoded text, or `None` if it has no `c:v`.
    #[must_use]
    pub fn value_str(&self) -> Option<String> {
        self.value().map(Value::text)
    }

    /// The point's value parsed as an `f64`, or `None` if it has no `c:v` or the text does not parse.
    #[must_use]
    pub fn value_f64(&self) -> Option<f64> {
        self.value()
            .and_then(|value| value.text().trim().parse().ok())
    }
}

// -------------------------------------------------------------------------------------------------
// c:numCache / c:strCache — the cached values
// -------------------------------------------------------------------------------------------------

/// One ordered child of a [`NumberCache`]/[`StringCache`]: a typed point (`c:pt`), or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheContent {
    /// A cached point (`c:pt`).
    Point(DataPoint),
    /// Any other child — `c:formatCode`, `c:ptCount`, whitespace, unknown — preserved verbatim.
    Raw(RawNode),
}

/// `c:numCache` (`CT_NumData`) — the cached numeric values of a reference.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct NumberCache {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "pt", variant = Point, ty = DataPoint))]
    content: Vec<CacheContent>,
}

impl NumberCache {
    /// The cached points, in document order.
    pub fn points(&self) -> impl Iterator<Item = &DataPoint> {
        self.content.iter().filter_map(|item| match item {
            CacheContent::Point(point) => Some(point),
            CacheContent::Raw(_) => None,
        })
    }

    /// The cached values, in document order — each point's `c:v` parsed as `f64` (a point whose value
    /// does not parse is skipped).
    #[must_use]
    pub fn values(&self) -> Vec<f64> {
        self.points().filter_map(DataPoint::value_f64).collect()
    }
}

/// `c:strCache` (`CT_StrData`) — the cached string values of a reference.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct StringCache {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "pt", variant = Point, ty = DataPoint))]
    content: Vec<CacheContent>,
}

impl StringCache {
    /// The cached points, in document order.
    pub fn points(&self) -> impl Iterator<Item = &DataPoint> {
        self.content.iter().filter_map(|item| match item {
            CacheContent::Point(point) => Some(point),
            CacheContent::Raw(_) => None,
        })
    }

    /// The cached labels, in document order — each point's `c:v` as decoded text (a point with no
    /// value reads as an empty string, so positions stay aligned).
    #[must_use]
    pub fn labels(&self) -> Vec<String> {
        self.points()
            .map(|point| point.value_str().unwrap_or_default())
            .collect()
    }
}

// -------------------------------------------------------------------------------------------------
// c:numRef / c:strRef — a formula plus its cache
// -------------------------------------------------------------------------------------------------

/// One ordered child of a [`NumberReference`]: the formula (`c:f`), the cache (`c:numCache`), or an
/// opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberReferenceContent {
    /// The workbook formula (`c:f`).
    Formula(Formula),
    /// The cached values (`c:numCache`).
    Cache(NumberCache),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `c:numRef` (`CT_NumRef`) — a workbook range and the numeric cache of what it last held.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct NumberReference {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "f", variant = Formula, ty = Formula),
        child(local = "numCache", variant = Cache, ty = NumberCache)
    )]
    content: Vec<NumberReferenceContent>,
}

impl NumberReference {
    /// The workbook formula (`c:f`), or `None` if it declares none.
    #[must_use]
    pub fn formula(&self) -> Option<&Formula> {
        self.content.iter().find_map(|item| match item {
            NumberReferenceContent::Formula(formula) => Some(formula),
            _ => None,
        })
    }

    /// The numeric cache (`c:numCache`), or `None` if it declares none.
    #[must_use]
    pub fn cache(&self) -> Option<&NumberCache> {
        self.content.iter().find_map(|item| match item {
            NumberReferenceContent::Cache(cache) => Some(cache),
            _ => None,
        })
    }

    /// The cached numeric values, or an empty vector if there is no cache.
    #[must_use]
    pub fn values(&self) -> Vec<f64> {
        self.cache().map(NumberCache::values).unwrap_or_default()
    }
}

/// One ordered child of a [`StringReference`]: the formula (`c:f`), the cache (`c:strCache`), or an
/// opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringReferenceContent {
    /// The workbook formula (`c:f`).
    Formula(Formula),
    /// The cached strings (`c:strCache`).
    Cache(StringCache),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `c:strRef` (`CT_StrRef`) — a workbook range and the string cache of what it last held.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct StringReference {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "f", variant = Formula, ty = Formula),
        child(local = "strCache", variant = Cache, ty = StringCache)
    )]
    content: Vec<StringReferenceContent>,
}

impl StringReference {
    /// The workbook formula (`c:f`), or `None` if it declares none.
    #[must_use]
    pub fn formula(&self) -> Option<&Formula> {
        self.content.iter().find_map(|item| match item {
            StringReferenceContent::Formula(formula) => Some(formula),
            _ => None,
        })
    }

    /// The string cache (`c:strCache`), or `None` if it declares none.
    #[must_use]
    pub fn cache(&self) -> Option<&StringCache> {
        self.content.iter().find_map(|item| match item {
            StringReferenceContent::Cache(cache) => Some(cache),
            _ => None,
        })
    }

    /// The cached labels, or an empty vector if there is no cache.
    #[must_use]
    pub fn labels(&self) -> Vec<String> {
        self.cache().map(StringCache::labels).unwrap_or_default()
    }
}

// -------------------------------------------------------------------------------------------------
// c:cat / c:val / c:tx — the series' data sources
// -------------------------------------------------------------------------------------------------

/// One ordered child of a [`NumericData`]: a numeric reference (`c:numRef`), or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumericDataContent {
    /// A numeric workbook reference (`c:numRef`).
    Reference(NumberReference),
    /// Any other source — a literal `c:numLit` this tier does not model — or an opaque node.
    Raw(RawNode),
}

/// `c:val` (`CT_NumDataSource`) — a series' numeric values (its `c:numRef`, with the cache read here).
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct NumericData {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "numRef", variant = Reference, ty = NumberReference))]
    content: Vec<NumericDataContent>,
}

impl NumericData {
    /// The numeric reference (`c:numRef`), or `None` for a source this tier does not model.
    #[must_use]
    pub fn reference(&self) -> Option<&NumberReference> {
        self.content.iter().find_map(|item| match item {
            NumericDataContent::Reference(reference) => Some(reference),
            NumericDataContent::Raw(_) => None,
        })
    }

    /// The series' cached numeric values, or an empty vector when there is no cached `c:numRef`.
    #[must_use]
    pub fn values(&self) -> Vec<f64> {
        self.reference()
            .map(NumberReference::values)
            .unwrap_or_default()
    }
}

/// One ordered child of a [`CategoryData`]: a string or numeric reference, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryDataContent {
    /// A string workbook reference (`c:strRef`) — text categories.
    StringReference(StringReference),
    /// A numeric workbook reference (`c:numRef`) — numeric categories.
    NumberReference(NumberReference),
    /// Any other source — `c:multiLvlStrRef`, a literal, an `c:extLst` — preserved verbatim.
    Raw(RawNode),
}

/// `c:cat` (`CT_AxDataSource`) — a series' category axis labels (its `c:strRef` or `c:numRef`).
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct CategoryData {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "strRef", variant = StringReference, ty = StringReference),
        child(local = "numRef", variant = NumberReference, ty = NumberReference)
    )]
    content: Vec<CategoryDataContent>,
}

impl CategoryData {
    /// The string reference (`c:strRef`), when the categories are text.
    #[must_use]
    pub fn string_reference(&self) -> Option<&StringReference> {
        self.content.iter().find_map(|item| match item {
            CategoryDataContent::StringReference(reference) => Some(reference),
            _ => None,
        })
    }

    /// The numeric reference (`c:numRef`), when the categories are numbers.
    #[must_use]
    pub fn number_reference(&self) -> Option<&NumberReference> {
        self.content.iter().find_map(|item| match item {
            CategoryDataContent::NumberReference(reference) => Some(reference),
            _ => None,
        })
    }

    /// The category labels, in order — from the string cache, or the numeric cache rendered as its
    /// cached wire text. Empty when the categories come from a source this tier does not model.
    #[must_use]
    pub fn labels(&self) -> Vec<String> {
        if let Some(reference) = self.string_reference() {
            return reference.labels();
        }
        if let Some(reference) = self.number_reference() {
            return reference
                .cache()
                .map(|cache| {
                    cache
                        .points()
                        .map(|point| point.value_str().unwrap_or_default())
                        .collect()
                })
                .unwrap_or_default();
        }
        Vec::new()
    }
}

/// One ordered child of a [`SeriesText`]: a string reference (`c:strRef`), a literal value (`c:v`),
/// or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeriesTextContent {
    /// A string workbook reference (`c:strRef`) naming the series-name cell.
    Reference(StringReference),
    /// A literal series name (`c:v`), used when the name is not a workbook reference.
    Value(Value),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `c:tx` (`CT_SerTx`) — a series' name, either a workbook reference or a literal string.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct SeriesText {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "strRef", variant = Reference, ty = StringReference),
        child(local = "v", variant = Value, ty = Value)
    )]
    content: Vec<SeriesTextContent>,
}

impl SeriesText {
    /// The series name — a literal `c:v`, else the first cached string of its `c:strRef`. `None` when
    /// neither is present.
    #[must_use]
    pub fn text(&self) -> Option<String> {
        for item in &self.content {
            match item {
                SeriesTextContent::Value(value) => return Some(value.text()),
                SeriesTextContent::Reference(reference) => {
                    return reference.labels().into_iter().next();
                }
                SeriesTextContent::Raw(_) => {}
            }
        }
        None
    }
}
