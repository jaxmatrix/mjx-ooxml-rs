//! `mjx-chart` — DrawingML charts (shared by all formats).
//!
//! A chart lives in its own part (`/ppt/charts/chartN.xml`), rooted at `c:chartSpace`, which a
//! `p:graphicFrame` references by relationship id. This crate models that part. Tier **C1** derives
//! the chart-space spine `c:chartSpace → c:chart → c:plotArea` and one plot type end to end — the
//! bar/column plot (`c:barChart`) — with read-only accessors for a chart's kind, its series, and
//! each series' category labels and values.
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
//! # Scope of C1
//!
//! Read-only, bar plot only. Cached data (`c:numCache`/`c:strCache`) is the read path — a literal
//! source (`c:numLit`/`c:strLit`) or a multi-level category (`c:multiLvlStrRef`) rides through the
//! `Raw` bucket and reads as empty for now. Editing (C3) and authoring (C4) are later tiers.

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
    BarChart, BarChartContent, BarDirection, BarGrouping, ChartKind, Series, SeriesContent,
};
pub use space::{Chart, ChartContent, ChartSpace, ChartSpaceContent, PlotArea, PlotAreaContent};
