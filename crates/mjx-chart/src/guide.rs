// Documentation-only module tree: each page is prose in `docs/guide/*.md`, so it reads on a source
// host as well as on the rendered docs page. No page declares an item. Mirrors `mjx_pptx::guide`'s,
// `mjx_docx::guide`'s, `mjx_xlsx::guide`'s, `mjx_ooxml::guide`'s, `mjx_opc::guide`'s,
// `mjx_dml::guide`'s and `mjx_sml::guide`'s shape — see any of them for why each module imports the
// crate's public vocabulary: so that the guide's intra-doc links resolve.
//
// This set covers **three crates**, not one: `mjx-chart`, `mjx-omml` and `mjx-vml` are the whole of
// layering rank 2.2, the markup that sits on top of DrawingML and SpreadsheetML, and a reader needs
// the relation between them rather than three unrelated tours (MJXOFF-221). The other two pages are
// `mjx_omml::guide` and `mjx_vml::guide`, hosted by their own crates because a same-rank crate cannot
// be depended on and therefore cannot be linked into — the same reason `mjx_opc::guide`'s sixth page
// lives in `mjx-mce`.
#![doc = include_str!("../docs/guide/README.md")]

/// Everything a guide page may link to, in one place.
macro_rules! guide_vocabulary {
    () => {
        #[allow(unused_imports)]
        use crate::{
            apply_workbook_patch, chart_ops, embedded_workbook_for_chart_data,
            embedded_workbook_for_chart_space, embedded_workbook_part, plan_workbook_patch,
            Area3DChart, AreaChart, Axis, AxisContent, AxisKind, AxisOrientation, AxisPosition,
            Bar3DChart, BarChart, BarDirection, BarGrouping, BlankDisplay, BubbleChart,
            CacheContent, CategoryData, CategoryDataContent, CategoryLevel, CategoryLevelContent,
            Chart, ChartAccessError, ChartAxisData, ChartContent, ChartData, ChartDataError,
            ChartErrorBarData, ChartKind, ChartLabelScope, ChartLegendData, ChartPointFormatData,
            ChartRanges, ChartSeriesData, ChartSeriesRange, ChartSeriesReferences, ChartSpace,
            ChartSpaceContent, ChartTitle, ChartTitleContent, ChartTrendlineData,
            ChartWorkbookError, DanglingPointReference, DataLabel, DataLabelContent,
            DataLabelPosition, DataLabelSettings, DataLabelSpec, DataLabels, DataLabelsContent,
            DataPoint, DataPointContent, DataPointFormat, DataPointFormatContent, DoughnutChart,
            ErrorBarDirection, ErrorBarSpec, ErrorBarType, ErrorBars, ErrorBarsContent,
            ErrorValueType, Formula, Gridlines, Legend, LegendPosition, Line3DChart, LineChart,
            MultiLevelStringCache, MultiLevelStringCacheContent, MultiLevelStringReference,
            MultiLevelStringReferenceContent, NumberCache, NumberReference, NumberReferenceContent,
            NumericData, NumericDataContent, OfPieChart, OfPieType, Pie3DChart, PieChart, PlotArea,
            PlotAreaContent, PlotContent, RadarChart, RadarStyle, ReferenceProblem, Scaling,
            ScatterChart, ScatterStyle, Series, SeriesContent, SeriesDecoration, SeriesGrouping,
            SeriesShapeProperties, SeriesText, SeriesTextContent, StockChart, StringCache,
            StringReference, StringReferenceContent, Surface3DChart, SurfaceChart,
            TickLabelPosition, TickMark, TitleText, TitleTextContent, Trendline, TrendlineContent,
            TrendlineKind, TrendlineSpec, Value, WorkbookPatch, WorkbookPatchPlan,
        };
    };
}

guide_vocabulary!();

/// The chart-space spine, the sixteen plot types, a series' four data sources, and how a host reads
/// one.
pub mod reading_a_chart {
    #![doc = include_str!("../docs/guide/reading_a_chart.md")]
    guide_vocabulary!();
}

/// The furniture around the plot, the three tiers data labels inherit over, and the refusals the
/// schema itself supplies.
pub mod axes_titles_and_decoration {
    #![doc = include_str!("../docs/guide/axes_titles_and_decoration.md")]
    guide_vocabulary!();
}

/// Building a chart from a description, what `validate` refuses, and the two places a chart's data
/// can live.
pub mod authoring_a_chart {
    #![doc = include_str!("../docs/guide/authoring_a_chart.md")]
    guide_vocabulary!();
}

/// The producer's own spreadsheet: why a data edit patches rather than regenerates, and the eight
/// reference shapes it refuses.
pub mod the_embedded_workbook {
    #![doc = include_str!("../docs/guide/the_embedded_workbook.md")]
    guide_vocabulary!();
}

/// The two serialization mechanisms across all three crates, what checks each of them, and what no
/// gate here can see.
pub mod fidelity_and_gaps {
    #![doc = include_str!("../docs/guide/fidelity_and_gaps.md")]
    guide_vocabulary!();
}
