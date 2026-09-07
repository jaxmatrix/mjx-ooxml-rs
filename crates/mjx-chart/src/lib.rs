//! `mjx-chart` — DrawingML charts (shared by all formats).
//!
//! A chart lives in its own part (`/ppt/charts/chartN.xml`), rooted at `c:chartSpace`, which a
//! `p:graphicFrame` references by relationship id. This crate models that part. It derives the
//! chart-space spine `c:chartSpace → c:chart → c:plotArea` and the common plot types — bar
//! (`c:barChart`), line, pie, area, scatter and doughnut — with read-only accessors for a chart's
//! kind(s), its series, and each series' category labels and values (or X/Y data, for scatter). A
//! plot area may hold more than one plot (a combo chart), read through [`PlotArea::chart_kinds`] and
//! [`PlotArea::all_series`].
//!
//! ```no_run
//! use mjx_ooxml_core::FromXml;
//! # fn demo(chart_part_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
//! let doc = mjx_xml::fidelity::parse(chart_part_bytes)?;
//! let space = mjx_chart::ChartSpace::from_xml(&doc.root, &doc.interner)?;
//! if let Some(bar) = space.bar_chart() {
//!     for series in bar.series() {
//!         let name = series.name().unwrap_or_default();
//!         let labels = series.categories().map(|c| c.labels()).unwrap_or_default();
//!         let values = series.values().map(|v| v.values()).unwrap_or_default();
//!         println!("{name}: {labels:?} = {values:?}");
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Fidelity
//!
//! Every modeled container keeps an ordered `content` list whose variants are the typed children
//! plus a `Raw` catch-all, so anything this tier does not interpret — other plot types, the axes,
//! styling, a literal data source, an `extLst`, an unknown attribute — round-trips **byte-for-byte**.
//! The two text-bearing leaves (`c:v`, `c:f`) keep their subtree opaque and re-emit it verbatim; a
//! value read as a number is parsed on demand from that preserved wire text, never reformatted. This
//! mirrors the DrawingML table model in `mjx-dml`.
//!
//! # Scope
//!
//! **All sixteen** plot types `CT_PlotArea` admits — bar, line, pie, area, scatter, doughnut, the
//! four three-dimensional forms, radar, bubble, stock, pie-of-pie and the two surface forms — read
//! their series through one API. All four data sources read: a workbook reference and its cache
//! (`c:numRef`/`c:strRef`), a literal (`c:numLit`/`c:strLit`), and a multi-level category
//! (`c:multiLvlStrRef`). The axes, their scaling and titles, the gridlines, the chart title, the
//! legend and the series' own fill and outline all have a typed surface — [`Axis`], [`Scaling`],
//! [`ChartTitle`], [`Legend`] and [`SeriesShapeProperties`].
//!
//! A chart's **decoration** has a typed surface too — the data labels ([`DataLabels`],
//! [`DataLabel`]), the per-point formatting ([`DataPointFormat`]), the trendlines ([`Trendline`])
//! and the error bars ([`ErrorBars`]). Data labels resolve over three tiers — the plot's `c:dLbls`,
//! the series' own, and one point's `c:dLbl` — merged **per setting** by
//! [`DataLabelSettings::inherit`]. Writing decoration goes through [`SeriesDecoration`], which binds
//! a series to the kind of plot that holds it: `CT_PieSer` declares no `c:trendline`, and `CT_BarSer`
//! and `CT_PieSer` place `c:dPt` at different ranks, so both the placement and the refusal follow
//! from the schema rather than from a list written here.
//!
//! Authoring writes any of the sixteen kinds ([`ChartData`]) **together with the embedded workbook**
//! that PowerPoint's Edit Data opens ([`embedded_workbook_for_chart_data`]) — and, with
//! [`ChartData::data_labels`], a chart that labels itself. That workbook is a real `.xlsx` package
//! written by `mjx-sml`; this crate lays out the grid and nothing else.
//!
//! A chart does **not** have to have one. [`ChartData::ranges`] points the `c:f` formulas at cells
//! in a host workbook instead ([`ChartRanges`], [`ChartSeriesRange`]), which is what a chart on a
//! worksheet does: there is no embedded copy, the formulas name the sheets the chart lives among,
//! and the cells are the source. Resolving such a reference against a package is `mjx-xlsx`'s —
//! this crate writes the text and reads it back, and has never resolved a reference in its life.

mod author;
mod axis;
mod build;
mod data;
mod decoration;
mod embedding;
mod ops;
mod plot;
mod space;
mod view;

pub use author::{ChartData, ChartDataError, ChartRanges, ChartSeriesRange};
pub use axis::{
    Axis, AxisContent, AxisKind, AxisOrientation, AxisPosition, BlankDisplay, ChartTitle,
    ChartTitleContent, Gridlines, Legend, LegendPosition, Scaling, TickLabelPosition, TickMark,
    TitleText, TitleTextContent,
};
pub use data::{
    CacheContent, CategoryData, CategoryDataContent, CategoryLevel, CategoryLevelContent,
    DataPoint, DataPointContent, Formula, MultiLevelStringCache, MultiLevelStringCacheContent,
    MultiLevelStringReference, MultiLevelStringReferenceContent, NumberCache, NumberReference,
    NumberReferenceContent, NumericData, NumericDataContent, SeriesText, SeriesTextContent,
    StringCache, StringReference, StringReferenceContent, Value,
};
pub use decoration::{
    DanglingPointReference, DataLabel, DataLabelContent, DataLabelPosition, DataLabelSettings,
    DataLabelSpec, DataLabels, DataLabelsContent, DataPointFormat, DataPointFormatContent,
    ErrorBarDirection, ErrorBarSpec, ErrorBarType, ErrorBars, ErrorBarsContent, ErrorValueType,
    Trendline, TrendlineContent, TrendlineKind, TrendlineSpec,
};
pub use embedding::{
    apply_workbook_patch, embedded_workbook_for_chart_data, embedded_workbook_for_chart_space,
    embedded_workbook_part, plan_workbook_patch, ChartWorkbookError, ReferenceProblem,
    WorkbookPatch, WorkbookPatchPlan,
};
pub use ops::ChartAccessError;
pub use plot::{
    Area3DChart, AreaChart, Bar3DChart, BarChart, BarDirection, BarGrouping, BubbleChart,
    ChartKind, DoughnutChart, Line3DChart, LineChart, OfPieChart, OfPieType, Pie3DChart, PieChart,
    PlotContent, RadarChart, RadarStyle, ScatterChart, ScatterStyle, Series, SeriesContent,
    SeriesDecoration, SeriesGrouping, SeriesShapeProperties, StockChart, Surface3DChart,
    SurfaceChart,
};
pub use space::{Chart, ChartContent, ChartSpace, ChartSpaceContent, PlotArea, PlotAreaContent};
pub use view::{
    ChartAxisData, ChartErrorBarData, ChartLabelScope, ChartLegendData, ChartPointFormatData,
    ChartSeriesData, ChartSeriesReferences, ChartTrendlineData,
};

/// Every read and every edit a host surface performs on a chart, stated once over a [`ChartSpace`].
///
/// `mjx-pptx`'s `Presentation` chart family and `mjx-docx`'s `Document` chart family are both thin
/// wrappers around this module: each resolves its own kind of address to a chart part, then calls
/// the identically-named function here. See the module documentation for why the shared body sits
/// in this crate rather than in either format crate.
pub mod chart_ops {
    pub use crate::ops::{
        add_trendline, axes, dangling_decoration, data_label_tier, data_labels,
        drop_dangling_decoration, error_bars, kinds, legend, point_formats, point_label_text,
        remove_data_labels, remove_error_bars, remove_point_format, remove_trendlines, series,
        series_at, series_fill, series_references, set_axis_gridlines, set_axis_orientation,
        set_axis_scale, set_axis_title, set_data_labels, set_error_bars, set_legend,
        set_point_explosion, set_point_fill, set_point_line, set_series_categories,
        set_series_fill, set_series_line, set_series_values, set_title, set_trendline, style_id,
        suppress_data_labels, title, trendlines,
    };
}
