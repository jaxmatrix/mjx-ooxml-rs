//! Authoring a brand-new chart part — the write counterpart of the read model and the cache edits
//! (see [`crate`] docs).
//!
//! A caller describes a chart with [`ChartData`] — a kind, the shared category labels, one or more
//! named series, and optionally a title and a legend — and [`ChartData::to_part_bytes`] serializes a
//! complete `c:chartSpace` part from it. Every one of the sixteen plot types
//! [`ChartKind`] names can be authored.
//!
//! The part carries the cached data (`c:strCache`/`c:numCache`) plus synthesized `c:f` formulas
//! naming where that data lives. Those formulas are not fiction: the companion
//! [`EmbeddedWorkbook`](crate::EmbeddedWorkbook) writes the workbook they name, laid out to match
//! cell for cell, and [`to_part_bytes_linking_workbook`](ChartData::to_part_bytes_linking_workbook)
//! adds the `c:externalData` that binds the two. A chart authored that way opens in PowerPoint's
//! **Edit Data** on the numbers it actually draws.
//!
//! ```
//! use mjx_chart::{ChartData, ChartKind, ChartSpace, LegendPosition};
//! use mjx_ooxml_core::FromXml;
//!
//! let chart = ChartData::new(ChartKind::Bar)
//!     .categories(["Q1", "Q2", "Q3"])
//!     .series("Revenue", [10.0, 20.0, 15.0])
//!     .series("Cost", [5.0, 8.0, 7.0])
//!     .title("Quarterly results")
//!     .legend(LegendPosition::Bottom);
//! let bytes = chart.to_part_bytes();
//!
//! // Authoring closes back through the read model.
//! let doc = mjx_xml::fidelity::parse(&bytes).unwrap();
//! let space = ChartSpace::from_xml(&doc.root, &doc.interner).unwrap();
//! assert_eq!(space.chart_kind(), Some(ChartKind::Bar));
//! assert_eq!(
//!     space.chart().and_then(|chart| chart.title_text()).as_deref(),
//!     Some("Quarterly results")
//! );
//! ```

use mjx_ooxml_core::{Interner, RawDocument, RawElement, RawNode, ToXml};
use mjx_ooxml_types::namespaces::{DML_CHART, DML_MAIN, SHARED_RELATIONSHIP_REFERENCE};

use crate::axis::{build_axis, AxisKind, AxisPosition, ChartTitle, Legend, LegendPosition};
use crate::build::{
    chart_attr, chart_element, chart_text_leaf, chart_val_leaf, f64_wire, namespace_declaration,
};
use crate::decoration::{DataLabelSpec, DataLabels};
use crate::plot::ChartKind;
use crate::workbook::column_letters;

/// The XML declaration a fresh chart part opens with, matching what Office writes (the inner bytes of
/// `<?xml … ?>`; the writer adds the delimiters).
const XML_DECLARATION: &[u8] = br#"xml version="1.0" encoding="UTF-8" standalone="yes""#;

/// The category axis id, referenced by a plot's first `c:axId` and its `c:catAx`.
///
/// Axis ids need only be unique within a chart part, so three fixed constants suffice — and they are
/// **`u32`**, deliberately. `charts.pptx` (python-pptx's template) carries *negative* axis ids
/// because python-pptx derives them from a signed hash; the schema types `c:axId@val` as
/// `xs:unsignedInt`. That is an input this library preserves verbatim, never markup it emits.
const CATEGORY_AXIS_ID: u32 = 111_111_111;
/// The value axis id, referenced by the plot's second `c:axId` and its `c:valAx`.
const VALUE_AXIS_ID: u32 = 222_222_222;
/// The series (depth) axis id of a three-dimensional or surface plot, referenced by its third
/// `c:axId` and its `c:serAx`.
const SERIES_AXIS_ID: u32 = 333_333_333;

/// The number of series a stock plot requires — `CT_StockChart` declares `ser` as `minOccurs="3"
/// maxOccurs="4"` (open, high, low, close, with open optional).
const STOCK_SERIES: std::ops::RangeInclusive<usize> = 3..=4;

/// Why a [`ChartData`] cannot be written as a chart part.
///
/// Authoring is checked before anything is written, so a rejected description leaves the document
/// untouched. Both cases are about the *shape* of the data, not its values: a chart with nothing to
/// draw is not something PowerPoint will open, and a plot type whose schema constrains its series
/// count cannot be given the wrong number of them without emitting markup that fails validation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ChartDataError {
    /// The chart has nothing to draw — no series, or every series empty.
    #[error("the chart has no data to draw: no series, or every series is empty")]
    NoData,
    /// The plot type requires a different number of series than the description carries.
    #[error(
        "a {kind} chart requires between {minimum} and {maximum} series, but {actual} were given"
    )]
    SeriesCount {
        /// The plot's element name (`stockChart`) — its exact wire spelling.
        kind: &'static str,
        /// The fewest series the schema admits.
        minimum: usize,
        /// The most series the schema admits.
        maximum: usize,
        /// How many the description carries.
        actual: usize,
    },
    /// The plot type's series (or the plot itself) declares no such child, so writing one would
    /// emit markup that fails schema validation.
    ///
    /// `CT_PieSer` declares no `c:trendline` and no `c:errBars`; `CT_SurfaceSer` declares no
    /// decoration at all; `CT_SurfaceChart` declares no plot-level `c:dLbls`.
    #[error(
        "a {plot} chart's series cannot carry a `c:{element}`: its {series_type} declares none"
    )]
    DecorationNotAllowed {
        /// The plot's element name (`pieChart`) — its exact wire spelling.
        plot: &'static str,
        /// The child that was asked for, without its `c:` prefix.
        element: &'static str,
        /// The XSD symbol of the complex type that declares no such child.
        series_type: &'static str,
    },
    /// A per-point edit named a point the series does not have.
    ///
    /// `c:dPt` and `c:dLbl` are anchored by index into the series, so an index at or past its point
    /// count addresses nothing. This is checked before anything is written, which is also what stops
    /// a hostile `c:idx` in a file from being propagated into markup this library authors.
    #[error("point {index} is out of range: the series has {count} point(s)")]
    DataPointOutOfRange {
        /// The index that was asked for.
        index: u32,
        /// How many points the series has.
        count: usize,
    },
    /// A setting was asked for at a tier whose schema does not declare it — `c:showLeaderLines` on
    /// one point's `c:dLbl`, which only the container form `Group_DLbls` admits.
    #[error("`c:{element}` is not a child of `c:{parent}`")]
    SettingNotAtThisTier {
        /// The setting's element name, without its `c:` prefix.
        element: &'static str,
        /// The element it was asked of, without its `c:` prefix.
        parent: &'static str,
    },
    /// A polynomial trendline's order is outside what `ST_Order` admits (2 to 6).
    #[error("a polynomial trendline's order must be between 2 and 6, but {order} was given")]
    TrendlineOrderOutOfRange {
        /// The order that was asked for.
        order: u8,
    },
    /// A moving average's period is below what `ST_Period` admits (2 upwards).
    #[error("a moving average's period must be at least 2, but {period} was given")]
    TrendlinePeriodOutOfRange {
        /// The period that was asked for.
        period: u32,
    },
    /// A measure has no XML spelling: `xsd:double` admits neither `NaN` nor an infinity in the
    /// spellings OOXML uses, so writing one would produce a part that fails validation.
    #[error("`c:{element}` was given a non-finite value, which has no XML spelling")]
    NonFiniteMeasure {
        /// The element the value was destined for, without its `c:` prefix.
        element: &'static str,
    },
    /// Custom error bars were asked for with neither `c:plus` nor `c:minus`. `ST_ErrValType`'s
    /// `cust` says the length "shall be determined by the Plus and Minus elements"; without either,
    /// nothing determines it.
    #[error("custom error bars need at least one of `c:plus` and `c:minus`")]
    CustomErrorBarsNeedValues,
}

/// One named series of a chart: its name and its cached numeric values.
#[derive(Debug, Clone)]
struct ChartSeries {
    name: String,
    values: Vec<f64>,
}

/// A description of a chart to author — its [kind](ChartKind), the category labels every series
/// shares, its series, and optionally a title and a legend — built up fluently and serialized to a
/// chart part with [`to_part_bytes`](Self::to_part_bytes).
///
/// The categories are the shared category-axis labels. For a [scatter](ChartKind::Scatter) or
/// [bubble](ChartKind::Bubble) chart they are the shared X values: each is parsed as a number, and a
/// label that does not parse falls back to its position, so `["1", "2", "3"]` gives X = 1, 2, 3.
#[derive(Debug, Clone)]
pub struct ChartData {
    kind: ChartKind,
    categories: Vec<String>,
    series: Vec<ChartSeries>,
    title: Option<String>,
    legend: Option<LegendPosition>,
    data_labels: Option<DataLabelSpec>,
}

impl ChartData {
    /// Starts a chart of `kind` with no categories and no series.
    #[must_use]
    pub fn new(kind: ChartKind) -> Self {
        Self {
            kind,
            categories: Vec::new(),
            series: Vec::new(),
            title: None,
            legend: None,
            data_labels: None,
        }
    }

    /// Sets the shared category labels (replacing any set before). For a scatter or bubble chart
    /// these are the shared X values (see the type docs).
    #[must_use]
    pub fn categories<I, S>(mut self, categories: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.categories = categories.into_iter().map(Into::into).collect();
        self
    }

    /// Appends a series named `name` with cached `values` (one per category).
    #[must_use]
    pub fn series<S, I>(mut self, name: S, values: I) -> Self
    where
        S: Into<String>,
        I: IntoIterator<Item = f64>,
    {
        self.series.push(ChartSeries {
            name: name.into(),
            values: values.into_iter().collect(),
        });
        self
    }

    /// Gives the chart a heading (`c:title`).
    #[must_use]
    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Gives the chart a legend at `position` (`c:legend`). Without this call the chart has none.
    #[must_use]
    pub fn legend(mut self, position: LegendPosition) -> Self {
        self.legend = Some(position);
        self
    }

    /// Labels every point the chart draws, with `spec`'s settings (`c:dLbls` at the plot tier).
    ///
    /// This is the outermost of the three tiers: every series takes these unless it states
    /// something of its own. Without this call the chart carries no `c:dLbls` and the application
    /// draws no labels.
    ///
    /// The two surface plots declare no `c:dLbls` — [`validate`](Self::validate) refuses the
    /// combination rather than writing markup that fails the schema.
    ///
    /// ```
    /// use mjx_chart::{ChartData, ChartKind, DataLabelPosition, DataLabelSpec};
    ///
    /// let chart = ChartData::new(ChartKind::Bar)
    ///     .categories(["Q1", "Q2"])
    ///     .series("Revenue", [10.0, 20.0])
    ///     .data_labels(
    ///         DataLabelSpec::new()
    ///             .value(true)
    ///             .position(DataLabelPosition::OutsideEnd)
    ///             .number_format("#,##0"),
    ///     );
    /// assert!(chart.validate().is_ok());
    /// ```
    #[must_use]
    pub fn data_labels(mut self, spec: DataLabelSpec) -> Self {
        self.data_labels = Some(spec);
        self
    }

    /// The chart's kind.
    #[must_use]
    pub fn kind(&self) -> ChartKind {
        self.kind
    }

    /// Whether the chart has nothing to draw — no series, or every series empty. A caller that adds a
    /// chart to a document rejects this (a chart with no data is not something PowerPoint will open).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.series.iter().all(|series| series.values.is_empty())
    }

    /// Checks that the description can be written as a schema-valid chart part.
    ///
    /// # Errors
    /// [`ChartDataError::NoData`] when there is nothing to draw,
    /// [`ChartDataError::SeriesCount`] when the plot type constrains its series count and this
    /// description does not satisfy it, or [`ChartDataError::DecorationNotAllowed`] when data
    /// labels were asked for on a plot type whose schema declares none.
    pub fn validate(&self) -> Result<(), ChartDataError> {
        if self.is_empty() {
            return Err(ChartDataError::NoData);
        }
        if self.data_labels.is_some() && !self.kind.admits_plot_child("dLbls") {
            return Err(ChartDataError::DecorationNotAllowed {
                plot: self.kind.element_local_name(),
                element: "dLbls",
                series_type: self.kind.plot_child_order().symbol,
            });
        }
        if self.kind == ChartKind::Stock && !STOCK_SERIES.contains(&self.series.len()) {
            return Err(ChartDataError::SeriesCount {
                kind: self.kind.element_local_name(),
                minimum: *STOCK_SERIES.start(),
                maximum: *STOCK_SERIES.end(),
                actual: self.series.len(),
            });
        }
        Ok(())
    }

    /// The name of each series, in order — the header row of the workbook that backs the chart.
    pub fn series_names(&self) -> impl Iterator<Item = &str> {
        self.series.iter().map(|series| series.name.as_str())
    }

    /// The values of each series, in order — one column of the workbook that backs the chart.
    pub fn series_values(&self) -> impl Iterator<Item = &[f64]> {
        self.series.iter().map(|series| series.values.as_slice())
    }

    /// How many category labels the description carries.
    #[must_use]
    pub fn category_count(&self) -> usize {
        self.categories.len()
    }

    /// The length of the longest series — the number of data rows the chart actually needs.
    #[must_use]
    pub fn longest_series(&self) -> usize {
        self.series
            .iter()
            .map(|series| series.values.len())
            .max()
            .unwrap_or(0)
    }

    /// The category label at `index`, or `None` past the last.
    #[must_use]
    pub fn category_label(&self, index: usize) -> Option<&str> {
        self.categories.get(index).map(String::as_str)
    }

    /// The category at `index` as a number: its label parsed as one, or its position when the label
    /// is absent or not numeric. This is what a scatter or bubble plot's X value is.
    #[must_use]
    pub fn category_number(&self, index: usize) -> f64 {
        // A category index is small (one per row of a chart's data), so the cast is exact.
        #[allow(clippy::cast_precision_loss)]
        let position = index as f64;
        self.categories
            .get(index)
            .and_then(|label| label.trim().parse::<f64>().ok())
            .unwrap_or(position)
    }

    /// Serializes a complete `c:chartSpace` chart part to bytes, ready to store as
    /// `/ppt/charts/chartN.xml`.
    ///
    /// The part carries cached data only and **no** `c:externalData`: it renders everywhere, but
    /// PowerPoint's Edit Data has nothing to open. Use
    /// [`to_part_bytes_linking_workbook`](Self::to_part_bytes_linking_workbook) together with
    /// [`EmbeddedWorkbook`](crate::EmbeddedWorkbook) to author a chart that does.
    #[must_use]
    pub fn to_part_bytes(&self) -> Vec<u8> {
        self.serialize(None)
    }

    /// Serializes a complete `c:chartSpace` chart part that names its embedded workbook by
    /// `workbook_rel_id` — a `c:externalData r:id="…"` with `c:autoUpdate="0"`, exactly as Office
    /// writes it.
    ///
    /// The caller is responsible for storing the workbook part
    /// ([`EmbeddedWorkbook::to_package_bytes`](crate::EmbeddedWorkbook::to_package_bytes)) and for
    /// relating it from the chart part under that id; this only writes the reference.
    #[must_use]
    pub fn to_part_bytes_linking_workbook(&self, workbook_rel_id: &str) -> Vec<u8> {
        self.serialize(Some(workbook_rel_id))
    }

    /// Serializes the part, optionally naming an embedded workbook.
    fn serialize(&self, workbook_rel_id: Option<&str>) -> Vec<u8> {
        let mut interner = Interner::new();
        let root = self.build_chart_space(&mut interner, workbook_rel_id);
        let doc = RawDocument::new(
            interner,
            false,
            vec![
                RawNode::Declaration(XML_DECLARATION.into()),
                RawNode::Text(Box::from(&b"\n"[..])),
            ],
            root,
            Vec::new(),
        );
        mjx_xml::fidelity::serialize_to_vec(&doc)
    }

    /// Builds the `c:chartSpace` root, declaring the namespaces its elements use.
    fn build_chart_space(
        &self,
        interner: &mut Interner,
        workbook_rel_id: Option<&str>,
    ) -> RawElement {
        let namespaces = vec![
            namespace_declaration(interner, "c", DML_CHART.transitional),
            namespace_declaration(interner, "a", DML_MAIN.transitional),
            namespace_declaration(interner, "r", SHARED_RELATIONSHIP_REFERENCE.transitional),
        ];
        let mut children = vec![el(self.build_chart(interner))];
        if let Some(rel_id) = workbook_rel_id {
            children.push(el(build_external_data(interner, rel_id)));
        }
        chart_element(interner, "chartSpace", namespaces, children)
    }

    /// Builds `c:chart` — the title (or the flag that suppresses Office's own), the plot area, the
    /// legend, and the two render flags Office writes.
    fn build_chart(&self, interner: &mut Interner) -> RawElement {
        let mut children = Vec::new();
        match &self.title {
            Some(text) => {
                let title = ChartTitle::new(interner, text);
                children.push(el(mjx_ooxml_core::ToXml::to_xml(&title, interner)));
                children.push(el(chart_val_leaf(interner, "autoTitleDeleted", "0")));
            }
            None => children.push(el(chart_val_leaf(interner, "autoTitleDeleted", "1"))),
        }
        children.push(el(self.build_plot_area(interner)));
        if let Some(position) = self.legend {
            let legend = Legend::new(interner, position);
            children.push(el(mjx_ooxml_core::ToXml::to_xml(&legend, interner)));
        }
        children.push(el(chart_val_leaf(interner, "plotVisOnly", "1")));
        children.push(el(chart_val_leaf(interner, "dispBlanksAs", "gap")));
        chart_element(interner, "chart", Vec::new(), children)
    }

    /// Builds `c:plotArea` — the typed plot element, then the axes it names (the pie family names
    /// none).
    fn build_plot_area(&self, interner: &mut Interner) -> RawElement {
        let mut children = vec![el(self.build_plot(interner))];
        // Scatter and bubble draw against two *value* axes — X along the bottom, Y up the left —
        // where every other axis-bearing kind puts a category axis along the bottom.
        let horizontal = if self.kind.uses_xy_data() {
            AxisKind::Value
        } else {
            AxisKind::Category
        };
        match self.kind.axis_count() {
            0 => {}
            2 => {
                children.push(el(build_axis(
                    interner,
                    horizontal,
                    CATEGORY_AXIS_ID,
                    VALUE_AXIS_ID,
                    AxisPosition::Bottom,
                )));
                children.push(el(build_axis(
                    interner,
                    AxisKind::Value,
                    VALUE_AXIS_ID,
                    CATEGORY_AXIS_ID,
                    AxisPosition::Left,
                )));
            }
            _ => {
                children.push(el(build_axis(
                    interner,
                    horizontal,
                    CATEGORY_AXIS_ID,
                    VALUE_AXIS_ID,
                    AxisPosition::Bottom,
                )));
                children.push(el(build_axis(
                    interner,
                    AxisKind::Value,
                    VALUE_AXIS_ID,
                    CATEGORY_AXIS_ID,
                    AxisPosition::Left,
                )));
                children.push(el(build_axis(
                    interner,
                    AxisKind::Series,
                    SERIES_AXIS_ID,
                    VALUE_AXIS_ID,
                    AxisPosition::Bottom,
                )));
            }
        }
        chart_element(interner, "plotArea", Vec::new(), children)
    }

    /// Builds the typed plot element (`c:barChart`, `c:radarChart`, …): the scalars its schema
    /// requires or Office always writes, its series, and its `c:axId` references.
    fn build_plot(&self, interner: &mut Interner) -> RawElement {
        let mut children: Vec<RawNode> = Vec::new();
        // The leading scalars, in `CT_*Chart` sequence order. Anything `minOccurs="1"` here is not
        // optional: omitting it makes the part fail schema validation, not merely render oddly.
        match self.kind {
            ChartKind::Bar | ChartKind::Bar3D => {
                children.push(el(chart_val_leaf(interner, "barDir", "col")));
                children.push(el(chart_val_leaf(interner, "grouping", "clustered")));
            }
            ChartKind::Line | ChartKind::Line3D | ChartKind::Area | ChartKind::Area3D => {
                children.push(el(chart_val_leaf(interner, "grouping", "standard")));
            }
            ChartKind::Pie | ChartKind::Pie3D | ChartKind::Doughnut | ChartKind::Bubble => {
                children.push(el(chart_val_leaf(interner, "varyColors", "1")));
            }
            ChartKind::OfPie => {
                children.push(el(chart_val_leaf(interner, "ofPieType", "pie")));
                children.push(el(chart_val_leaf(interner, "varyColors", "1")));
            }
            ChartKind::Scatter => {
                children.push(el(chart_val_leaf(interner, "scatterStyle", "lineMarker")));
            }
            ChartKind::Radar => {
                children.push(el(chart_val_leaf(interner, "radarStyle", "marker")));
                children.push(el(chart_val_leaf(interner, "varyColors", "0")));
            }
            ChartKind::Surface | ChartKind::Surface3D => {
                children.push(el(chart_val_leaf(interner, "wireframe", "0")));
            }
            ChartKind::Stock => {}
        }

        for (index, series) in self.series.iter().enumerate() {
            children.push(el(self.build_series(interner, index, series)));
        }

        // `c:dLbls` follows the series run in every `CT_*Chart` that declares it, and precedes the
        // type-specific tail below.
        if let Some(spec) = &self.data_labels {
            let labels = DataLabels::new(interner, spec);
            children.push(el(labels.to_xml(interner)));
        }

        // The trailing scalars, then the axis ids.
        if self.kind == ChartKind::Doughnut {
            children.push(el(chart_val_leaf(interner, "holeSize", "50")));
        }
        for id in [CATEGORY_AXIS_ID, VALUE_AXIS_ID, SERIES_AXIS_ID]
            .into_iter()
            .take(self.kind.axis_count())
        {
            children.push(el(chart_val_leaf(interner, "axId", &id.to_string())));
        }

        chart_element(
            interner,
            self.kind.element_local_name(),
            Vec::new(),
            children,
        )
    }

    /// Builds one `c:ser` — its `c:idx`/`c:order` header, its literal name, and its data sources
    /// (`c:cat`+`c:val` for a category/value plot, `c:xVal`+`c:yVal` for scatter and bubble).
    fn build_series(
        &self,
        interner: &mut Interner,
        index: usize,
        series: &ChartSeries,
    ) -> RawElement {
        let mut children = vec![
            el(chart_val_leaf(interner, "idx", &index.to_string())),
            el(chart_val_leaf(interner, "order", &index.to_string())),
            el(build_series_name(interner, &series.name)),
        ];
        let count = series.values.len();
        if self.kind.uses_xy_data() {
            let x_values: Vec<f64> = (0..count).map(|i| self.category_number(i)).collect();
            children.push(el(build_num_data(
                interner,
                "xVal",
                &category_formula(count),
                &x_values,
            )));
            children.push(el(build_num_data(
                interner,
                "yVal",
                &value_formula(index, count),
                &series.values,
            )));
            // `c:bubbleSize` is optional and this description carries no third channel: emitting a
            // size we invented would be data the caller never gave. Office draws uniform bubbles.
        } else {
            children.push(el(self.build_categories(interner)));
            children.push(el(build_num_data(
                interner,
                "val",
                &value_formula(index, count),
                &series.values,
            )));
        }
        chart_element(interner, "ser", Vec::new(), children)
    }

    /// Builds `c:cat` — a `c:strRef` naming the category cells and caching their labels.
    fn build_categories(&self, interner: &mut Interner) -> RawElement {
        let formula = chart_text_leaf(interner, "f", &category_formula(self.categories.len()));
        let cache = build_str_cache(interner, &self.categories);
        let reference = chart_element(interner, "strRef", Vec::new(), vec![el(formula), el(cache)]);
        chart_element(interner, "cat", Vec::new(), vec![el(reference)])
    }
}

/// Wraps a built element as a child node.
fn el(element: RawElement) -> RawNode {
    RawNode::Element(element)
}

/// Builds `c:externalData r:id="…"` with `c:autoUpdate val="0"` — the chart's reference to the
/// embedded workbook that backs it, which is what PowerPoint's Edit Data opens.
fn build_external_data(interner: &mut Interner, rel_id: &str) -> RawElement {
    let auto_update = chart_val_leaf(interner, "autoUpdate", "0");
    let id = mjx_ooxml_core::RawAttribute {
        name: mjx_ooxml_core::RawName {
            prefix: Some(interner.intern("r")),
            local: interner.intern("id"),
            namespace: Some(interner.intern(SHARED_RELATIONSHIP_REFERENCE.transitional)),
        },
        value: mjx_xml::text::escape_attribute(rel_id).as_bytes().into(),
        quote: mjx_ooxml_core::QuoteStyle::Double,
    };
    chart_element(interner, "externalData", vec![id], vec![el(auto_update)])
}

/// Builds `c:tx` holding a literal series name (`<c:tx><c:v>name</c:v></c:tx>`).
fn build_series_name(interner: &mut Interner, name: &str) -> RawElement {
    let value = chart_text_leaf(interner, "v", name);
    chart_element(interner, "tx", Vec::new(), vec![el(value)])
}

/// Builds a numeric data source `<c:local><c:numRef><c:f>…</c:f><c:numCache>…</c:numCache></c:numRef>
/// </c:local>` — the shape of `c:val`, `c:xVal` and `c:yVal`.
fn build_num_data(
    interner: &mut Interner,
    local: &str,
    formula: &str,
    values: &[f64],
) -> RawElement {
    let f = chart_text_leaf(interner, "f", formula);
    let cache = build_num_cache(interner, values);
    let reference = chart_element(interner, "numRef", Vec::new(), vec![el(f), el(cache)]);
    chart_element(interner, local, Vec::new(), vec![el(reference)])
}

/// Builds `c:numCache` — a `General` format, a `c:ptCount`, and one `c:pt` per value. A non-finite
/// value has no XML spelling, so it is cached as `0` to keep the point count and indices aligned.
fn build_num_cache(interner: &mut Interner, values: &[f64]) -> RawElement {
    let mut children = vec![
        el(chart_text_leaf(interner, "formatCode", "General")),
        el(chart_val_leaf(
            interner,
            "ptCount",
            &values.len().to_string(),
        )),
    ];
    for (index, &value) in values.iter().enumerate() {
        let text = f64_wire(value).unwrap_or_else(|| "0".to_owned());
        children.push(el(build_point(interner, index, &text)));
    }
    chart_element(interner, "numCache", Vec::new(), children)
}

/// Builds `c:strCache` — a `c:ptCount` and one `c:pt` per label.
fn build_str_cache(interner: &mut Interner, labels: &[String]) -> RawElement {
    let mut children = vec![el(chart_val_leaf(
        interner,
        "ptCount",
        &labels.len().to_string(),
    ))];
    for (index, label) in labels.iter().enumerate() {
        children.push(el(build_point(interner, index, label)));
    }
    chart_element(interner, "strCache", Vec::new(), children)
}

/// Builds one cache point `<c:pt idx="index"><c:v>value</c:v></c:pt>`.
fn build_point(interner: &mut Interner, index: usize, value: &str) -> RawElement {
    let idx = chart_attr(interner, "idx", &index.to_string());
    let v = chart_text_leaf(interner, "v", value);
    chart_element(interner, "pt", vec![idx], vec![el(v)])
}

/// The formula for the category cells: `Sheet1!$A$2:$A$N` — the exact range the companion embedded
/// workbook writes those labels into.
fn category_formula(count: usize) -> String {
    format!("Sheet1!$A$2:$A${}", count + 1)
}

/// The formula for series `series_index`'s value cells: `Sheet1!$B$2:$B$N` for the first series,
/// `$C$…` for the second, and so on (column `A` is the categories).
fn value_formula(series_index: usize, count: usize) -> String {
    let column = column_letters(series_index + 1);
    format!("Sheet1!${column}$2:${column}${}", count + 1)
}
