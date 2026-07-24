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
//! Read-only, the common plot types (bar, line, pie, area, scatter, doughnut). Cached data
//! (`c:numCache`/`c:strCache`) is the read path — a literal source (`c:numLit`/`c:strLit`), a
//! multi-level category (`c:multiLvlStrRef`), or an unmodeled plot type (radar, bubble, 3-D, …) rides
//! through the `Raw` bucket and reads as empty/absent for now. Editing (C3) and authoring (C4) are
//! later tiers.

mod build;
mod data;
mod plot;
mod space;

pub use data::{
    CacheContent, CategoryData, CategoryDataContent, DataPoint, DataPointContent, Formula,
    NumberCache, NumberReference, NumberReferenceContent, NumericData, NumericDataContent,
    SeriesText, SeriesTextContent, StringCache, StringReference, StringReferenceContent, Value,
};
pub use plot::{
    AreaChart, BarChart, BarDirection, BarGrouping, ChartKind, DoughnutChart, LineChart, PieChart,
    PlotContent, ScatterChart, Series, SeriesContent,
};
pub use space::{Chart, ChartContent, ChartSpace, ChartSpaceContent, PlotArea, PlotAreaContent};
