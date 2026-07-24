//! Authoring a brand-new chart part — the write counterpart of the C1/C2 read model and the C3 cache
//! edits (see [`crate`] docs).
//!
//! A caller describes a chart with [`ChartData`] — a kind, the shared category labels, and one or
//! more named series — and [`ChartData::to_part_bytes`] serializes a complete `c:chartSpace` part
//! from it. The part carries **cached data only** (`c:strCache`/`c:numCache`), plus synthesized
//! `c:f` formulas so the references are schema-valid: there is **no embedded workbook**, so the
//! chart renders everywhere from its cache while PowerPoint's "Edit Data" is degraded until the
//! embedded-workbook follow-up lands.
//!
//! ```
//! use mjx_chart::{ChartData, ChartKind, ChartSpace};
//! use mjx_ooxml_core::FromXml;
//!
//! let chart = ChartData::new(ChartKind::Bar)
//!     .categories(["Q1", "Q2", "Q3"])
//!     .series("Revenue", [10.0, 20.0, 15.0])
//!     .series("Cost", [5.0, 8.0, 7.0]);
//! let bytes = chart.to_part_bytes();
//!
//! // Authoring closes back through the read model.
//! let doc = mjx_xml::fidelity::parse(&bytes).unwrap();
//! let space = ChartSpace::from_xml(&doc.root, &doc.interner).unwrap();
//! assert_eq!(space.chart_kind(), Some(ChartKind::Bar));
//! ```

use mjx_ooxml_core::{Interner, RawDocument, RawElement, RawNode};
use mjx_ooxml_types::namespaces::{DML_CHART, DML_MAIN, SHARED_RELATIONSHIP_REFERENCE};

use crate::build::{
    chart_attr, chart_element, chart_text_leaf, chart_val_leaf, f64_wire, namespace_declaration,
};
use crate::plot::ChartKind;

/// The XML declaration a fresh chart part opens with, matching what Office writes (the inner bytes of
/// `<?xml … ?>`; the writer adds the delimiters).
const XML_DECLARATION: &[u8] = br#"xml version="1.0" encoding="UTF-8" standalone="yes""#;

/// The category axis id, referenced by a bar/line/area plot's first `c:axId` and its `c:catAx`. Axis
/// ids need only be unique within a chart part, so two fixed constants suffice.
const CATEGORY_AXIS_ID: u32 = 111_111_111;
/// The value axis id, referenced by the plot's second `c:axId` and its `c:valAx`.
const VALUE_AXIS_ID: u32 = 222_222_222;

/// One named series of a chart: its name and its cached numeric values.
#[derive(Debug, Clone)]
struct ChartSeries {
    name: String,
    values: Vec<f64>,
}

/// A description of a chart to author — its [kind](ChartKind), the category labels every series
/// shares, and its series — built up fluently and serialized to a chart part with
/// [`to_part_bytes`](Self::to_part_bytes).
///
/// The categories are the shared category-axis labels (bar, line, pie, area, doughnut). For a
/// [scatter](ChartKind::Scatter) chart they are the shared X values: each is parsed as a number, and
/// a label that does not parse falls back to its position, so `["1", "2", "3"]` gives X = 1, 2, 3.
#[derive(Debug, Clone)]
pub struct ChartData {
    kind: ChartKind,
    categories: Vec<String>,
    series: Vec<ChartSeries>,
}

impl ChartData {
    /// Starts a chart of `kind` with no categories and no series.
    #[must_use]
    pub fn new(kind: ChartKind) -> Self {
        Self {
            kind,
            categories: Vec::new(),
            series: Vec::new(),
        }
    }

    /// Sets the shared category labels (replacing any set before). For a scatter chart these are the
    /// shared X values (see the type docs).
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

    /// Serializes a complete `c:chartSpace` chart part (cached data only) to bytes, ready to store as
    /// `/ppt/charts/chartN.xml`.
    #[must_use]
    pub fn to_part_bytes(&self) -> Vec<u8> {
        let mut interner = Interner::new();
        let root = self.build_chart_space(&mut interner);
        let doc = RawDocument {
            interner,
            bom: false,
            prologue: vec![
                RawNode::Declaration(XML_DECLARATION.into()),
                RawNode::Text(Box::from(&b"\n"[..])),
            ],
            root,
            epilogue: Vec::new(),
        };
        mjx_xml::fidelity::serialize_to_vec(&doc)
    }

    /// Builds the `c:chartSpace` root, declaring the namespaces its `c:` elements use.
    fn build_chart_space(&self, interner: &mut Interner) -> RawElement {
        let namespaces = vec![
            namespace_declaration(interner, "c", DML_CHART.transitional),
            namespace_declaration(interner, "a", DML_MAIN.transitional),
            namespace_declaration(interner, "r", SHARED_RELATIONSHIP_REFERENCE.transitional),
        ];
        let chart = self.build_chart(interner);
        chart_element(interner, "chartSpace", namespaces, vec![el(chart)])
    }

    /// Builds `c:chart` — `c:autoTitleDeleted`, the plot area, and the two render flags Office writes.
    fn build_chart(&self, interner: &mut Interner) -> RawElement {
        let children = vec![
            el(chart_val_leaf(interner, "autoTitleDeleted", "1")),
            el(self.build_plot_area(interner)),
            el(chart_val_leaf(interner, "plotVisOnly", "1")),
            el(chart_val_leaf(interner, "dispBlanksAs", "gap")),
        ];
        chart_element(interner, "chart", Vec::new(), children)
    }

    /// Builds `c:plotArea` — the typed plot element, then the axes (for the kinds that use them; pie
    /// and doughnut have none).
    fn build_plot_area(&self, interner: &mut Interner) -> RawElement {
        let mut children = vec![el(self.build_plot(interner))];
        match self.kind {
            ChartKind::Bar | ChartKind::Line | ChartKind::Area => {
                children.push(el(build_axis(
                    interner,
                    "catAx",
                    CATEGORY_AXIS_ID,
                    VALUE_AXIS_ID,
                    "b",
                )));
                children.push(el(build_axis(
                    interner,
                    "valAx",
                    VALUE_AXIS_ID,
                    CATEGORY_AXIS_ID,
                    "l",
                )));
            }
            // Scatter draws against two value axes: X along the bottom, Y up the left.
            ChartKind::Scatter => {
                children.push(el(build_axis(
                    interner,
                    "valAx",
                    CATEGORY_AXIS_ID,
                    VALUE_AXIS_ID,
                    "b",
                )));
                children.push(el(build_axis(
                    interner,
                    "valAx",
                    VALUE_AXIS_ID,
                    CATEGORY_AXIS_ID,
                    "l",
                )));
            }
            ChartKind::Pie | ChartKind::Doughnut => {}
        }
        chart_element(interner, "plotArea", Vec::new(), children)
    }

    /// Builds the typed plot element (`c:barChart`, `c:lineChart`, …): its type-specific scalars, its
    /// series, and — for the axis-bearing kinds — its two `c:axId` references.
    fn build_plot(&self, interner: &mut Interner) -> RawElement {
        let mut children: Vec<RawNode> = Vec::new();
        let local = match self.kind {
            ChartKind::Bar => {
                children.push(el(chart_val_leaf(interner, "barDir", "col")));
                children.push(el(chart_val_leaf(interner, "grouping", "clustered")));
                "barChart"
            }
            ChartKind::Line => {
                children.push(el(chart_val_leaf(interner, "grouping", "standard")));
                "lineChart"
            }
            ChartKind::Area => {
                children.push(el(chart_val_leaf(interner, "grouping", "standard")));
                "areaChart"
            }
            ChartKind::Pie => {
                children.push(el(chart_val_leaf(interner, "varyColors", "1")));
                "pieChart"
            }
            ChartKind::Doughnut => {
                children.push(el(chart_val_leaf(interner, "varyColors", "1")));
                "doughnutChart"
            }
            ChartKind::Scatter => {
                children.push(el(chart_val_leaf(interner, "scatterStyle", "lineMarker")));
                "scatterChart"
            }
        };

        for (index, series) in self.series.iter().enumerate() {
            children.push(el(self.build_series(interner, index, series)));
        }

        // A doughnut's hole size follows its series; then the axis-bearing kinds name their two axes.
        if self.kind == ChartKind::Doughnut {
            children.push(el(chart_val_leaf(interner, "holeSize", "50")));
        }
        if uses_axes(self.kind) {
            children.push(el(chart_val_leaf(
                interner,
                "axId",
                &CATEGORY_AXIS_ID.to_string(),
            )));
            children.push(el(chart_val_leaf(
                interner,
                "axId",
                &VALUE_AXIS_ID.to_string(),
            )));
        }

        chart_element(interner, local, Vec::new(), children)
    }

    /// Builds one `c:ser` — its `c:idx`/`c:order` header, its literal name, and its data sources
    /// (`c:cat`+`c:val` for a category/value plot, `c:xVal`+`c:yVal` for scatter).
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
        if self.kind == ChartKind::Scatter {
            let x_values: Vec<f64> = (0..count).map(|i| self.x_value(i)).collect();
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

    /// The X value of scatter point `index`: its category label parsed as a number, or its position
    /// when the label is absent or not numeric.
    fn x_value(&self, index: usize) -> f64 {
        self.categories
            .get(index)
            .and_then(|label| label.trim().parse::<f64>().ok())
            .unwrap_or(index as f64)
    }
}

/// Whether a plot of `kind` draws against a pair of axes (and so names them with `c:axId`). Pie and
/// doughnut do not.
fn uses_axes(kind: ChartKind) -> bool {
    matches!(
        kind,
        ChartKind::Bar | ChartKind::Line | ChartKind::Area | ChartKind::Scatter
    )
}

/// Wraps a built element as a child node.
fn el(element: RawElement) -> RawNode {
    RawNode::Element(element)
}

/// Builds `c:tx` holding a literal series name (`<c:tx><c:v>name</c:v></c:tx>`) — no workbook
/// reference, since the name is authored directly.
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

/// Builds a minimal axis (`c:catAx`/`c:valAx`) that renders: its id, `minMax` scaling, `c:delete=0`,
/// its position, and the partner axis it crosses.
fn build_axis(
    interner: &mut Interner,
    local: &str,
    axis_id: u32,
    cross_axis_id: u32,
    axis_position: &str,
) -> RawElement {
    let orientation = chart_val_leaf(interner, "orientation", "minMax");
    let scaling = chart_element(interner, "scaling", Vec::new(), vec![el(orientation)]);
    let children = vec![
        el(chart_val_leaf(interner, "axId", &axis_id.to_string())),
        el(scaling),
        el(chart_val_leaf(interner, "delete", "0")),
        el(chart_val_leaf(interner, "axPos", axis_position)),
        el(chart_val_leaf(
            interner,
            "crossAx",
            &cross_axis_id.to_string(),
        )),
    ];
    chart_element(interner, local, Vec::new(), children)
}

/// The synthesized formula for the category cells: `Sheet1!$A$2:$A$N`. There is no workbook behind
/// it — it makes the reference schema-valid and names where the data would live.
fn category_formula(count: usize) -> String {
    format!("Sheet1!$A$2:$A${}", count + 1)
}

/// The synthesized formula for series `series_index`'s value cells: `Sheet1!$B$2:$B$N` for the first
/// series, `$C$…` for the second, and so on (column `A` is the categories).
fn value_formula(series_index: usize, count: usize) -> String {
    let column = column_letter(series_index + 1);
    format!("Sheet1!${column}$2:${column}${}", count + 1)
}

/// The spreadsheet column letters for a 0-based column index (`0` → `A`, `25` → `Z`, `26` → `AA`).
fn column_letter(mut index: usize) -> String {
    let mut letters = Vec::new();
    loop {
        letters.push(b'A' + (index % 26) as u8);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    letters.reverse();
    // Every pushed byte is an ASCII uppercase letter, so this is always valid UTF-8.
    String::from_utf8(letters).unwrap_or_default()
}
