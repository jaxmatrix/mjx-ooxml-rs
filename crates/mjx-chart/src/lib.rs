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
//! Authoring writes any of the sixteen kinds ([`ChartData`]) **together with the embedded workbook**
//! that PowerPoint's Edit Data opens ([`EmbeddedWorkbook`]).

mod author;
mod axis;
mod build;
mod data;
mod plot;
mod space;
mod workbook;

pub use author::{ChartData, ChartDataError};
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
pub use plot::{
    Area3DChart, AreaChart, Bar3DChart, BarChart, BarDirection, BarGrouping, BubbleChart,
    ChartKind, DoughnutChart, Line3DChart, LineChart, OfPieChart, OfPieType, Pie3DChart, PieChart,
    PlotContent, RadarChart, RadarStyle, ScatterChart, ScatterStyle, Series, SeriesContent,
    SeriesGrouping, SeriesShapeProperties, StockChart, Surface3DChart, SurfaceChart,
};
pub use space::{Chart, ChartContent, ChartSpace, ChartSpaceContent, PlotArea, PlotAreaContent};
pub use workbook::{
    EmbeddedWorkbook, WorkbookCell, CONTENT_TYPE_WORKBOOK_PACKAGE, DEFAULT_SHEET_NAME,
};
