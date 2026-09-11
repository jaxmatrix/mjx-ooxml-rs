# Reading a chart

A chart is a part of its own — `/ppt/charts/chart1.xml`, `/word/charts/chart1.xml`,
`/xl/charts/chart1.xml` — rooted at `c:chartSpace` and referenced by relationship id from whatever
frame draws it. This crate models that part and nothing around it.

## The spine

Four types, and everything else hangs off them.

```text
c:chartSpace   →  ChartSpace
  c:chart      →  Chart
    c:plotArea →  PlotArea
      c:barChart / c:lineChart / …  →  BarChart, LineChart, … (sixteen)
        c:ser  →  Series
```

[`ChartSpace`](crate::ChartSpace) is built from a parsed part with
[`FromXml`](mjx_ooxml_core::FromXml), like every other model in this workspace:

```rust
use mjx_ooxml_core::FromXml;
use mjx_chart::ChartSpace;

let part = br#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart><c:plotArea>
    <c:barChart>
      <c:ser>
        <c:tx><c:strRef><c:f>Sheet1!$B$1</c:f>
          <c:strCache><c:pt idx="0"><c:v>Revenue</c:v></c:pt></c:strCache>
        </c:strRef></c:tx>
        <c:val><c:numRef><c:f>Sheet1!$B$2:$B$3</c:f>
          <c:numCache>
            <c:pt idx="0"><c:v>10</c:v></c:pt>
            <c:pt idx="1"><c:v>20</c:v></c:pt>
          </c:numCache>
        </c:numRef></c:val>
      </c:ser>
    </c:barChart>
  </c:plotArea></c:chart>
</c:chartSpace>"#;

let document = mjx_xml::fidelity::parse(part).expect("well-formed");
let space = ChartSpace::from_xml(&document.root, &document.interner).expect("a chart space");

let bar = space.bar_chart().expect("the bar plot");
let series: Vec<_> = bar.series().collect();
assert_eq!(series.len(), 1);
assert_eq!(series[0].name().as_deref(), Some("Revenue"));
assert_eq!(
    series[0].values().map(mjx_chart::NumericData::values),
    Some(vec![10.0, 20.0])
);
```

## A plot area holds *plots*, not a plot

A combo chart is a `c:plotArea` with more than one plot element in it — bars and a line over the same
axes — and the schema simply allows the sequence. So the two questions a reader actually has are
answered over the plot area rather than over one plot:

* [`PlotArea::chart_kinds`](crate::PlotArea::chart_kinds) — every [`ChartKind`](crate::ChartKind)
  present, in document order.
* [`PlotArea::all_series`](crate::PlotArea::all_series) — every series of every plot, flattened, in
  the order the plots appear. **This is the numbering every other call in the crate uses:**
  `mjx_chart::chart_ops::series_at` and [`WorkbookPatch`](crate::WorkbookPatch)'s `series_idx` both
  index into it, so a series' number does not change when you ask a different question about it.

[`ChartSpace::bar_chart`](crate::ChartSpace::bar_chart) and its siblings are the convenience for the
overwhelmingly common single-plot case, and answer `None` on a chart that has no plot of that kind.

## All sixteen plot types read; six of them are more than a bag

`CT_PlotArea` admits sixteen plot elements and every one of them has a type here —
[`BarChart`](crate::BarChart), [`LineChart`](crate::LineChart), [`PieChart`](crate::PieChart),
[`AreaChart`](crate::AreaChart), [`ScatterChart`](crate::ScatterChart),
[`DoughnutChart`](crate::DoughnutChart), the four three-dimensional forms
([`Bar3DChart`](crate::Bar3DChart), [`Line3DChart`](crate::Line3DChart),
[`Pie3DChart`](crate::Pie3DChart), [`Area3DChart`](crate::Area3DChart)),
[`RadarChart`](crate::RadarChart), [`BubbleChart`](crate::BubbleChart),
[`StockChart`](crate::StockChart), [`OfPieChart`](crate::OfPieChart),
[`SurfaceChart`](crate::SurfaceChart) and [`Surface3DChart`](crate::Surface3DChart). All sixteen read
their series through the same API.

Six carry typed scalars of their own beyond the shared spine — a bar plot's
[`BarDirection`](crate::BarDirection) and [`BarGrouping`](crate::BarGrouping), a scatter plot's
[`ScatterStyle`](crate::ScatterStyle), a radar plot's [`RadarStyle`](crate::RadarStyle), an
of-pie plot's [`OfPieType`](crate::OfPieType), a doughnut's hole size, a pie's first-slice angle.
Everything else a plot element carries stays in its `Raw` bucket and comes back verbatim.

## The four data sources, and why a value is text until you ask

A series' name, categories and values each come from one of four shapes, and
[`Series`](crate::Series) resolves all four behind one accessor:

| Wire | Type | What it is |
|---|---|---|
| `c:numRef` | [`NumberReference`](crate::NumberReference) | a formula plus a cached copy of the numbers it named |
| `c:strRef` | [`StringReference`](crate::StringReference) | the same, for text |
| `c:numLit` / `c:strLit` | [`NumericData`](crate::NumericData) / [`CategoryData`](crate::CategoryData) | literal data with no formula at all |
| `c:multiLvlStrRef` | [`MultiLevelStringReference`](crate::MultiLevelStringReference) | a multi-level category axis, one [`CategoryLevel`](crate::CategoryLevel) per level |

Two leaves carry the actual text: [`Value`](crate::Value) (`c:v`) and [`Formula`](crate::Formula)
(`c:f`). **Both keep their subtree opaque and re-emit it verbatim**, and a number is parsed from that
preserved wire text on demand rather than stored as an `f64` and reformatted. That is why `19.20` in
somebody's file is still `19.20` after an edit somewhere else in the chart, and why a `c:f` naming a
defined name or another workbook survives a library that cannot resolve either.

[`ChartSeriesReferences`](crate::ChartSeriesReferences) is the report of those formulas, and each
field is `Option`: `None` means *that source is a literal*, which is a different fact from an empty
string and the reason the type exists. A series whose values are a `c:numLit` has no cells behind it
and cannot be refreshed from a sheet.

## Reading through a host, not through this crate

Nothing above needs a package, which is what lets one body of code serve three formats. The host
surfaces are thin: `mjx_chart::chart_ops` holds every read and every edit stated once over a
[`ChartSpace`](crate::ChartSpace), and `mjx-pptx`'s and `mjx-docx`'s chart families each resolve their
own kind of address to a chart part and then call the identically-named function there.

The summaries a host hands back are plain values with no borrow into the part —
[`ChartSeriesData`](crate::ChartSeriesData), [`ChartAxisData`](crate::ChartAxisData),
[`ChartLegendData`](crate::ChartLegendData) and their siblings live in this crate for the same
layering reason the guide index gives: `mjx-pptx` and `mjx-docx` are both rank 3.0, so a type both
need cannot live in either.
