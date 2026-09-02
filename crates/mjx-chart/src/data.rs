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
//! `c:formatCode`, a `c:ptCount` or an `c:extLst` this tier does not interpret round-trips
//! byte-for-byte in the `Raw` bucket. The two text-bearing leaves —
//! `c:v` (a cached value) and `c:f` (the formula) — keep their subtree opaque and expose a decoding
//! `text` accessor; a value read as `f64` is **parsed on demand** from that preserved wire text, so
//! nothing is ever reformatted on write.
//!
//! # The four sources a series can name
//!
//! A cache is only one of them. `CT_NumDataSource` (`c:val`, `c:yVal`, `c:bubbleSize`) is a choice of
//! a **reference** (`c:numRef`, a formula plus its cache) or a **literal** (`c:numLit`, the numbers
//! written inline with no workbook behind them); `CT_AxDataSource` (`c:cat`, `c:xVal`) adds the
//! string forms and the **multi-level** category reference (`c:multiLvlStrRef`), whose cache is a
//! stack of label levels rather than one row. All four read here, and the two literal forms are
//! editable exactly like a cache — a literal *is* the data, so rewriting it is rewriting the chart.
//!
//! `c:numLit`/`c:strLit` share the content model of `c:numCache`/`c:strCache` (`CT_NumData` /
//! `CT_StrData`), so they are modeled by the same [`NumberCache`] / [`StringCache`] types: each
//! preserves the element name it was parsed from, so a literal re-emits as `c:numLit`, never as a
//! cache.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};

use crate::build::{
    attr_u32, chart_attr, chart_element, chart_name, chart_text_leaf, element_text, f64_wire,
    fidelity_element_impls, is_chart, set_attr,
};

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
    /// A fresh `c:v` carrying `text` (escaped) — the value a rewritten cache point holds.
    pub(crate) fn new(interner: &mut Interner, text: &str) -> Self {
        let element = chart_text_leaf(interner, "v", text);
        let (name, empty) = (element.name, element.empty);
        let content = element.into_content();
        Self {
            name,
            attributes: content.attributes,
            children: content.children,
            empty,
        }
    }

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
    /// A fresh `c:pt idx="idx"` carrying a single `c:v` of `value` (escaped) — one point of a
    /// rewritten cache.
    pub(crate) fn new(interner: &mut Interner, idx: u32, value: &str) -> Self {
        let name = chart_name(interner, "pt");
        let idx_attr = chart_attr(interner, "idx", &idx.to_string());
        let value = Value::new(interner, value);
        Self {
            name,
            attributes: vec![idx_attr],
            empty: false,
            content: vec![DataPointContent::Value(value)],
        }
    }

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

    /// Rewrites the cache to hold exactly `values` (each formatted as its canonical wire text),
    /// updating `c:ptCount` and leaving `c:formatCode` and any other child untouched. A non-finite
    /// value (`NaN`/`±inf`) has no valid spelling and is skipped.
    pub fn set_values(&mut self, interner: &mut Interner, values: &[f64]) {
        let points = values
            .iter()
            .enumerate()
            .filter_map(|(index, &value)| {
                f64_wire(value)
                    .map(|text| CacheContent::Point(DataPoint::new(interner, index as u32, &text)))
            })
            .collect();
        rebuild_cache_points(&mut self.content, interner, points);
        self.empty = false;
    }

    /// A fresh empty `c:numCache` — what a numeric reference gets when it had no cache before an edit.
    pub(crate) fn empty(interner: &mut Interner) -> Self {
        Self {
            name: chart_name(interner, "numCache"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
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

    /// Rewrites the cache to hold exactly `labels` (each escaped), updating `c:ptCount` and leaving
    /// any other child untouched.
    pub fn set_labels(&mut self, interner: &mut Interner, labels: &[&str]) {
        let points = labels
            .iter()
            .enumerate()
            .map(|(index, label)| {
                CacheContent::Point(DataPoint::new(interner, index as u32, label))
            })
            .collect();
        rebuild_cache_points(&mut self.content, interner, points);
        self.empty = false;
    }

    /// A fresh empty `c:strCache` — what a string reference gets when it had no cache before an edit.
    pub(crate) fn empty(interner: &mut Interner) -> Self {
        Self {
            name: chart_name(interner, "strCache"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }
}

/// Replaces every `c:pt` in a cache's `content` with `points`, keeping `c:formatCode` and any other
/// child, and updating (or inserting) `c:ptCount@val` to the new count. The new points land where
/// the old ones were — after `c:formatCode`/`c:ptCount`, before a trailing `c:extLst`.
fn rebuild_cache_points(
    content: &mut Vec<CacheContent>,
    interner: &mut Interner,
    points: Vec<CacheContent>,
) {
    let count = points.len();
    let mut at = content
        .iter()
        .position(|item| matches!(item, CacheContent::Point(_)))
        .unwrap_or(content.len());
    content.retain(|item| !matches!(item, CacheContent::Point(_)));
    // Only points were removed, and every item before the first point was a non-point, so `at` still
    // indexes the same boundary in the retained list — clamp only for the no-points-at-all case.
    at = at.min(content.len());
    at = set_or_insert_ptcount(content, interner, count, at);
    content.splice(at..at, points);
}

/// Sets `c:ptCount@val` to `count` in place if the cache has one, else inserts a `c:ptCount` at `at`.
/// Returns the index new points should be inserted at — past a freshly inserted `c:ptCount`.
fn set_or_insert_ptcount(
    content: &mut Vec<CacheContent>,
    interner: &mut Interner,
    count: usize,
    at: usize,
) -> usize {
    for item in content.iter_mut() {
        if let CacheContent::Raw(RawNode::Element(element)) = item {
            if is_chart(&element.name, interner)
                && interner.resolve(element.name.local) == "ptCount"
            {
                set_attr(&mut element.attributes, interner, "val", &count.to_string());
                return at;
            }
        }
    }
    let count_attr = chart_attr(interner, "val", &count.to_string());
    let element = chart_element(interner, "ptCount", vec![count_attr], Vec::new());
    content.insert(at, CacheContent::Raw(RawNode::Element(element)));
    at + 1
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

    /// Rewrites the reference's cached values, creating the `c:numCache` if it had none (its `c:f`
    /// formula still names the — now stale — workbook range).
    pub fn set_values(&mut self, interner: &mut Interner, values: &[f64]) {
        if let Some(cache) = self.content.iter_mut().find_map(|item| match item {
            NumberReferenceContent::Cache(cache) => Some(cache),
            _ => None,
        }) {
            cache.set_values(interner, values);
        } else {
            let mut cache = NumberCache::empty(interner);
            cache.set_values(interner, values);
            self.content.push(NumberReferenceContent::Cache(cache));
            self.empty = false;
        }
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

    /// Rewrites the reference's cached labels, creating the `c:strCache` if it had none.
    pub fn set_labels(&mut self, interner: &mut Interner, labels: &[&str]) {
        if let Some(cache) = self.content.iter_mut().find_map(|item| match item {
            StringReferenceContent::Cache(cache) => Some(cache),
            _ => None,
        }) {
            cache.set_labels(interner, labels);
        } else {
            let mut cache = StringCache::empty(interner);
            cache.set_labels(interner, labels);
            self.content.push(StringReferenceContent::Cache(cache));
            self.empty = false;
        }
    }
}

// -------------------------------------------------------------------------------------------------
// c:cat / c:val / c:tx — the series' data sources
// -------------------------------------------------------------------------------------------------

/// One ordered child of a [`NumericData`]: a numeric reference (`c:numRef`), or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumericDataContent {
    /// A numeric workbook reference (`c:numRef`) — a formula plus the cache of what it held.
    Reference(NumberReference),
    /// A numeric literal (`c:numLit`) — the numbers inline, with no workbook behind them. Shares
    /// `CT_NumData` with `c:numCache`, so it is modeled by the same type.
    Literal(NumberCache),
    /// Any other child — an `c:extLst`, whitespace, unknown — preserved verbatim.
    Raw(RawNode),
}

/// `c:val` (`CT_NumDataSource`) — a series' numeric values (its `c:numRef`, with the cache read here).
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct NumericData {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "numRef", variant = Reference, ty = NumberReference),
        child(local = "numLit", variant = Literal, ty = NumberCache)
    )]
    content: Vec<NumericDataContent>,
}

impl NumericData {
    /// The numeric reference (`c:numRef`), or `None` when the source is a literal.
    #[must_use]
    pub fn reference(&self) -> Option<&NumberReference> {
        self.content.iter().find_map(|item| match item {
            NumericDataContent::Reference(reference) => Some(reference),
            _ => None,
        })
    }

    /// The numeric literal (`c:numLit`), or `None` when the source is a workbook reference.
    #[must_use]
    pub fn literal(&self) -> Option<&NumberCache> {
        self.content.iter().find_map(|item| match item {
            NumericDataContent::Literal(literal) => Some(literal),
            _ => None,
        })
    }

    /// The series' numeric values — from the reference's cache, or from the literal. Empty when the
    /// source declares neither.
    #[must_use]
    pub fn values(&self) -> Vec<f64> {
        if let Some(reference) = self.reference() {
            return reference.values();
        }
        self.literal().map(NumberCache::values).unwrap_or_default()
    }

    /// Rewrites the values — through the `c:numRef`'s cache, or through the `c:numLit` when the
    /// source is a literal. Returns `false` (unchanged) when the source declares neither.
    pub fn set_values(&mut self, interner: &mut Interner, values: &[f64]) -> bool {
        for item in &mut self.content {
            match item {
                NumericDataContent::Reference(reference) => {
                    reference.set_values(interner, values);
                    return true;
                }
                NumericDataContent::Literal(literal) => {
                    literal.set_values(interner, values);
                    return true;
                }
                NumericDataContent::Raw(_) => {}
            }
        }
        false
    }
}

/// One ordered child of a [`CategoryData`]: a string or numeric reference, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryDataContent {
    /// A string workbook reference (`c:strRef`) — text categories.
    StringReference(StringReference),
    /// A numeric workbook reference (`c:numRef`) — numeric categories.
    NumberReference(NumberReference),
    /// A multi-level string reference (`c:multiLvlStrRef`) — categories stacked in levels, as a
    /// pivot-style axis draws them.
    MultiLevelStringReference(MultiLevelStringReference),
    /// A string literal (`c:strLit`) — the labels inline, with no workbook behind them.
    StringLiteral(StringCache),
    /// A numeric literal (`c:numLit`) — the numbers inline, with no workbook behind them.
    NumberLiteral(NumberCache),
    /// Any other child — an `c:extLst`, whitespace, unknown — preserved verbatim.
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
        child(local = "numRef", variant = NumberReference, ty = NumberReference),
        child(local = "multiLvlStrRef", variant = MultiLevelStringReference, ty = MultiLevelStringReference),
        child(local = "strLit", variant = StringLiteral, ty = StringCache),
        child(local = "numLit", variant = NumberLiteral, ty = NumberCache)
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

    /// The multi-level reference (`c:multiLvlStrRef`), when the categories are stacked in levels.
    #[must_use]
    pub fn multi_level_reference(&self) -> Option<&MultiLevelStringReference> {
        self.content.iter().find_map(|item| match item {
            CategoryDataContent::MultiLevelStringReference(reference) => Some(reference),
            _ => None,
        })
    }

    /// The string literal (`c:strLit`), when the labels are written inline.
    #[must_use]
    pub fn string_literal(&self) -> Option<&StringCache> {
        self.content.iter().find_map(|item| match item {
            CategoryDataContent::StringLiteral(literal) => Some(literal),
            _ => None,
        })
    }

    /// The numeric literal (`c:numLit`), when the numbers are written inline.
    #[must_use]
    pub fn number_literal(&self) -> Option<&NumberCache> {
        self.content.iter().find_map(|item| match item {
            CategoryDataContent::NumberLiteral(literal) => Some(literal),
            _ => None,
        })
    }

    /// Whether the categories are numeric — a `c:numRef` or a `c:numLit` rather than text. This is
    /// what tells a scatter or bubble series' shared X data from a category axis' labels.
    #[must_use]
    pub fn is_numeric(&self) -> bool {
        self.number_reference().is_some() || self.number_literal().is_some()
    }

    /// The category labels, in order, whichever of the five sources the series names: the string
    /// cache or literal as text, the numeric cache or literal as its cached wire text, and a
    /// multi-level reference as its **first** `c:lvl` in document order (see
    /// [`levels`](Self::levels) for the rest). Empty when the source declares no data.
    #[must_use]
    pub fn labels(&self) -> Vec<String> {
        for item in &self.content {
            match item {
                CategoryDataContent::StringReference(reference) => return reference.labels(),
                CategoryDataContent::NumberReference(reference) => {
                    return reference.cache().map(number_labels).unwrap_or_default();
                }
                CategoryDataContent::MultiLevelStringReference(reference) => {
                    return reference.labels();
                }
                CategoryDataContent::StringLiteral(literal) => return literal.labels(),
                CategoryDataContent::NumberLiteral(literal) => return number_labels(literal),
                CategoryDataContent::Raw(_) => {}
            }
        }
        Vec::new()
    }

    /// Every level of a multi-level category source, in document order — the outer list is the
    /// levels (`c:lvl`), the inner one that level's labels. Empty for any other source.
    ///
    /// ECMA-376 does not say which end of the list is nearest the axis, so this crate does not name
    /// them "inner"/"outer": the order is the document's, and [`labels`](Self::labels) returns the
    /// first level so a multi-level axis still reads as a flat one.
    #[must_use]
    pub fn levels(&self) -> Vec<Vec<String>> {
        self.multi_level_reference()
            .map(MultiLevelStringReference::levels)
            .unwrap_or_default()
    }

    /// The numeric category values, in order — the companion to [`labels`](Self::labels) for numeric
    /// axis data (a scatter series' `c:xVal`, or numeric categories), from a `c:numRef` or a
    /// `c:numLit`. Empty when the data is a string or multi-level source.
    #[must_use]
    pub fn values(&self) -> Vec<f64> {
        if let Some(reference) = self.number_reference() {
            return reference.values();
        }
        self.number_literal()
            .map(NumberCache::values)
            .unwrap_or_default()
    }

    /// Rewrites the category labels — through the `c:strRef`'s cache, or through the `c:strLit` when
    /// the labels are a literal. Returns `false` (unchanged) for a numeric or multi-level source,
    /// which has no string labels to rewrite.
    pub fn set_labels(&mut self, interner: &mut Interner, labels: &[&str]) -> bool {
        for item in &mut self.content {
            match item {
                CategoryDataContent::StringReference(reference) => {
                    reference.set_labels(interner, labels);
                    return true;
                }
                CategoryDataContent::StringLiteral(literal) => {
                    literal.set_labels(interner, labels);
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    /// Rewrites the numeric category values — through the `c:numRef`'s cache, or through the
    /// `c:numLit`. Returns `false` (unchanged) for a string or multi-level source.
    pub fn set_values(&mut self, interner: &mut Interner, values: &[f64]) -> bool {
        for item in &mut self.content {
            match item {
                CategoryDataContent::NumberReference(reference) => {
                    reference.set_values(interner, values);
                    return true;
                }
                CategoryDataContent::NumberLiteral(literal) => {
                    literal.set_values(interner, values);
                    return true;
                }
                _ => {}
            }
        }
        false
    }
}

/// A numeric cache or literal read as label text — each point's `c:v` exactly as it is written, so a
/// number never passes through a reformat on its way to a label.
fn number_labels(cache: &NumberCache) -> Vec<String> {
    cache
        .points()
        .map(|point| point.value_str().unwrap_or_default())
        .collect()
}

// -------------------------------------------------------------------------------------------------
// c:multiLvlStrRef — categories stacked in levels
// -------------------------------------------------------------------------------------------------

/// One ordered child of a [`CategoryLevel`]: a label (`c:pt`), or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryLevelContent {
    /// One label of the level (`c:pt`).
    Point(DataPoint),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `c:lvl` (`CT_Lvl`) — one level of labels of a multi-level category axis.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct CategoryLevel {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "pt", variant = Point, ty = DataPoint))]
    content: Vec<CategoryLevelContent>,
}

impl CategoryLevel {
    /// The level's labelled points, in document order.
    pub fn points(&self) -> impl Iterator<Item = &DataPoint> {
        self.content.iter().filter_map(|item| match item {
            CategoryLevelContent::Point(point) => Some(point),
            CategoryLevelContent::Raw(_) => None,
        })
    }

    /// The level's labels, in document order (a point with no `c:v` reads as an empty string, so
    /// positions stay aligned).
    #[must_use]
    pub fn labels(&self) -> Vec<String> {
        self.points()
            .map(|point| point.value_str().unwrap_or_default())
            .collect()
    }
}

/// One ordered child of a [`MultiLevelStringCache`]: a level (`c:lvl`), or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiLevelStringCacheContent {
    /// One level of labels (`c:lvl`).
    Level(CategoryLevel),
    /// Any other child — `c:ptCount`, `c:extLst`, whitespace, unknown — preserved verbatim.
    Raw(RawNode),
}

/// `c:multiLvlStrCache` (`CT_MultiLvlStrData`) — the cached labels of a multi-level category axis,
/// one `c:lvl` per level.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct MultiLevelStringCache {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "lvl", variant = Level, ty = CategoryLevel))]
    content: Vec<MultiLevelStringCacheContent>,
}

impl MultiLevelStringCache {
    /// The cached levels, in document order.
    pub fn levels(&self) -> impl Iterator<Item = &CategoryLevel> {
        self.content.iter().filter_map(|item| match item {
            MultiLevelStringCacheContent::Level(level) => Some(level),
            MultiLevelStringCacheContent::Raw(_) => None,
        })
    }
}

/// One ordered child of a [`MultiLevelStringReference`]: the formula (`c:f`), the cache
/// (`c:multiLvlStrCache`), or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiLevelStringReferenceContent {
    /// The workbook formula (`c:f`).
    Formula(Formula),
    /// The cached levels (`c:multiLvlStrCache`).
    Cache(MultiLevelStringCache),
    /// Any other child — an `c:extLst`, whitespace, unknown — preserved verbatim.
    Raw(RawNode),
}

/// `c:multiLvlStrRef` (`CT_MultiLvlStrRef`) — a workbook range whose category labels are stacked in
/// levels, and the cache of what those levels last held.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct MultiLevelStringReference {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "f", variant = Formula, ty = Formula),
        child(local = "multiLvlStrCache", variant = Cache, ty = MultiLevelStringCache)
    )]
    content: Vec<MultiLevelStringReferenceContent>,
}

impl MultiLevelStringReference {
    /// The workbook formula (`c:f`), or `None` if it declares none.
    #[must_use]
    pub fn formula(&self) -> Option<&Formula> {
        self.content.iter().find_map(|item| match item {
            MultiLevelStringReferenceContent::Formula(formula) => Some(formula),
            _ => None,
        })
    }

    /// The cache (`c:multiLvlStrCache`), or `None` if it declares none.
    #[must_use]
    pub fn cache(&self) -> Option<&MultiLevelStringCache> {
        self.content.iter().find_map(|item| match item {
            MultiLevelStringReferenceContent::Cache(cache) => Some(cache),
            _ => None,
        })
    }

    /// Every cached level's labels, in document order.
    #[must_use]
    pub fn levels(&self) -> Vec<Vec<String>> {
        self.cache()
            .map(|cache| cache.levels().map(CategoryLevel::labels).collect())
            .unwrap_or_default()
    }

    /// The first cached level's labels, or an empty vector when there is no cache — what a reader
    /// that treats a multi-level axis as a flat one sees.
    #[must_use]
    pub fn labels(&self) -> Vec<String> {
        self.cache()
            .and_then(|cache| cache.levels().next())
            .map(CategoryLevel::labels)
            .unwrap_or_default()
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
