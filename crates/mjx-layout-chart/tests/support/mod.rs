//! Chart parts, written by hand.
//!
//! # Why the fixtures are hand-written XML and not authored through `mjx-chart`
//!
//! `mjx_chart::ChartData` authors a chart from a list of `f64`s, and it is the right tool for a
//! caller who wants a chart. It is the wrong tool for a *fixture*, because what these suites need is
//! control over the awkward corners — a series with no `c:spPr` at all, a `c:pt` with a hole in its
//! `c:idx` run, an axis stating a `c:majorUnit` of seven, a `c:numCache` whose values are all zero.
//! Authoring writes a well-formed chart; a fixture has to be able to write a nearly-well-formed one.
//!
//! Everything here is a `c:chartSpace` a real producer could have written, and every one goes through
//! the same `ChartModel::read` a host calls.

#![allow(dead_code)]

use mjx_layout::LayoutRect;
use mjx_ooxml_core::measure::Emu;

/// A frame four inches by three — the size a chart occupies on a slide, and the frame every geometry
/// assertion in these suites is made against.
pub(crate) fn frame() -> LayoutRect {
    LayoutRect::from_edges(
        Emu::ZERO,
        Emu::ZERO,
        Emu::from_inches(4.0),
        Emu::from_inches(3.0),
    )
}

/// The XML declaration and the `c:chartSpace` wrapper every fixture shares.
fn wrap(body: &str) -> Vec<u8> {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>{body}</c:chart>
</c:chartSpace>"#
    )
    .into_bytes()
}

/// One `c:numRef` cache of `values`, with the point at each index that has one.
fn number_cache(values: &[Option<f64>]) -> String {
    let points: String = values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            value.map(|value| format!(r#"<c:pt idx="{index}"><c:v>{value}</c:v></c:pt>"#))
        })
        .collect();
    format!(
        r#"<c:numRef><c:f>Sheet1!$B$2:$B${}</c:f><c:numCache><c:formatCode>General</c:formatCode><c:ptCount val="{}"/>{points}</c:numCache></c:numRef>"#,
        values.len() + 1,
        values.len()
    )
}

/// One `c:strRef` cache of `labels`.
fn string_cache(labels: &[&str]) -> String {
    let points: String = labels
        .iter()
        .enumerate()
        .map(|(index, label)| format!(r#"<c:pt idx="{index}"><c:v>{label}</c:v></c:pt>"#))
        .collect();
    format!(
        r#"<c:strRef><c:f>Sheet1!$A$2:$A${}</c:f><c:strCache><c:ptCount val="{}"/>{points}</c:strCache></c:strRef>"#,
        labels.len() + 1,
        labels.len()
    )
}

/// One series with a name, categories and values, and **no `c:spPr` at all** — which is the case
/// that exercises theme resolution and the one a chart authored in Office actually writes.
pub(crate) fn series(
    index: u32,
    name: &str,
    categories: &[&str],
    values: &[Option<f64>],
) -> String {
    format!(
        r#"<c:ser><c:idx val="{index}"/><c:order val="{index}"/>
        <c:tx><c:strRef><c:f>Sheet1!$B$1</c:f><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>{name}</c:v></c:pt></c:strCache></c:strRef></c:tx>
        <c:cat>{}</c:cat><c:val>{}</c:val></c:ser>"#,
        string_cache(categories),
        number_cache(values)
    )
}

/// The same series with an explicit solid fill, for the half of the theme test that proves a stated
/// fill is not overridden.
pub(crate) fn series_filled(
    index: u32,
    name: &str,
    categories: &[&str],
    values: &[Option<f64>],
    hex: &str,
) -> String {
    format!(
        r#"<c:ser><c:idx val="{index}"/><c:order val="{index}"/>
        <c:tx><c:strRef><c:f>Sheet1!$B$1</c:f><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>{name}</c:v></c:pt></c:strCache></c:strRef></c:tx>
        <c:spPr><a:solidFill><a:srgbClr val="{hex}"/></a:solidFill></c:spPr>
        <c:cat>{}</c:cat><c:val>{}</c:val></c:ser>"#,
        string_cache(categories),
        number_cache(values)
    )
}

/// A scatter series: `c:xVal` and `c:yVal` rather than `c:cat` and `c:val`.
pub(crate) fn scatter_series(index: u32, name: &str, xs: &[f64], ys: &[f64]) -> String {
    let x_labels: Vec<String> = xs.iter().map(f64::to_string).collect();
    let x_refs: Vec<&str> = x_labels.iter().map(String::as_str).collect();
    let y_values: Vec<Option<f64>> = ys.iter().map(|y| Some(*y)).collect();
    format!(
        r#"<c:ser><c:idx val="{index}"/><c:order val="{index}"/>
        <c:tx><c:strRef><c:f>Sheet1!$B$1</c:f><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>{name}</c:v></c:pt></c:strCache></c:strRef></c:tx>
        <c:xVal>{}</c:xVal><c:yVal>{}</c:yVal></c:ser>"#,
        string_cache(&x_refs),
        number_cache(&y_values)
    )
}

/// A bubble series: `c:xVal`, `c:yVal` and `c:bubbleSize`.
pub(crate) fn bubble_series(
    index: u32,
    name: &str,
    xs: &[f64],
    ys: &[f64],
    sizes: &[f64],
) -> String {
    let x_labels: Vec<String> = xs.iter().map(f64::to_string).collect();
    let x_refs: Vec<&str> = x_labels.iter().map(String::as_str).collect();
    let y_values: Vec<Option<f64>> = ys.iter().map(|y| Some(*y)).collect();
    let size_values: Vec<Option<f64>> = sizes.iter().map(|size| Some(*size)).collect();
    format!(
        r#"<c:ser><c:idx val="{index}"/><c:order val="{index}"/>
        <c:tx><c:strRef><c:f>Sheet1!$B$1</c:f><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>{name}</c:v></c:pt></c:strCache></c:strRef></c:tx>
        <c:xVal>{}</c:xVal><c:yVal>{}</c:yVal><c:bubbleSize>{}</c:bubbleSize></c:ser>"#,
        string_cache(&x_refs),
        number_cache(&y_values),
        number_cache(&size_values)
    )
}

/// The two axes a category chart declares, with `extra` spliced into the value axis.
pub(crate) fn axes(extra: &str) -> String {
    format!(
        r#"<c:catAx><c:axId val="111"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="222"/><c:crossBetween val="between"/></c:catAx>
        <c:valAx><c:axId val="222"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:crossAx val="111"/>{extra}</c:valAx>"#
    )
}

/// A bar chart of `series`, grouped as stated.
pub(crate) fn bar_chart(grouping: &str, series: &[String], axis_extra: &str) -> Vec<u8> {
    wrap(&format!(
        r#"<c:plotArea><c:layout/><c:barChart><c:barDir val="col"/><c:grouping val="{grouping}"/><c:varyColors val="0"/>{}
        <c:axId val="111"/><c:axId val="222"/></c:barChart>{}</c:plotArea>"#,
        series.concat(),
        axes(axis_extra)
    ))
}

/// The same bar chart with its bars running **across** — `c:barDir val="bar"` — and its two axes
/// swapped over, which is what a real horizontal bar chart writes.
pub(crate) fn horizontal_bar_chart(series: &[String]) -> Vec<u8> {
    let axes = r#"<c:catAx><c:axId val="111"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:crossAx val="222"/><c:crossBetween val="between"/></c:catAx>
        <c:valAx><c:axId val="222"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="111"/></c:valAx>"#;
    wrap(&format!(
        r#"<c:plotArea><c:layout/><c:barChart><c:barDir val="bar"/><c:grouping val="clustered"/><c:varyColors val="0"/>{}
        <c:axId val="111"/><c:axId val="222"/></c:barChart>{axes}</c:plotArea>"#,
        series.concat()
    ))
}

/// A line chart of `series`.
pub(crate) fn line_chart(series: &[String]) -> Vec<u8> {
    wrap(&format!(
        r#"<c:plotArea><c:layout/><c:lineChart><c:grouping val="standard"/><c:varyColors val="0"/>{}
        <c:axId val="111"/><c:axId val="222"/></c:lineChart>{}</c:plotArea>"#,
        series.concat(),
        axes("")
    ))
}

/// A pie chart of one series.
pub(crate) fn pie_chart(series: &str) -> Vec<u8> {
    wrap(&format!(
        r#"<c:plotArea><c:layout/><c:pieChart><c:varyColors val="1"/>{series}<c:firstSliceAng val="0"/></c:pieChart></c:plotArea>"#
    ))
}

/// A scatter chart of `series`, with two value axes.
pub(crate) fn scatter_chart(series: &[String]) -> Vec<u8> {
    wrap(&format!(
        r#"<c:plotArea><c:layout/><c:scatterChart><c:scatterStyle val="lineMarker"/><c:varyColors val="0"/>{}
        <c:axId val="111"/><c:axId val="222"/></c:scatterChart>
        <c:valAx><c:axId val="222"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:crossAx val="111"/></c:valAx>
        <c:valAx><c:axId val="111"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="222"/></c:valAx></c:plotArea>"#,
        series.concat()
    ))
}

/// A chart with a title and a legend, for the negotiation suite.
pub(crate) fn titled_bar_chart(title: &str, legend: &str, series: &[String]) -> Vec<u8> {
    wrap(&format!(
        r#"<c:title><c:tx><c:rich><a:bodyPr/><a:p><a:r><a:t>{title}</a:t></a:r></a:p></c:rich></c:tx><c:overlay val="0"/></c:title>
        <c:autoTitleDeleted val="0"/>
        <c:plotArea><c:layout/><c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:varyColors val="0"/>{}
        <c:axId val="111"/><c:axId val="222"/></c:barChart>{}</c:plotArea>
        <c:legend><c:legendPos val="{legend}"/><c:overlay val="0"/></c:legend>"#,
        series.concat(),
        axes("")
    ))
}

/// Values with none missing.
pub(crate) fn values(numbers: &[f64]) -> Vec<Option<f64>> {
    numbers.iter().map(|value| Some(*value)).collect()
}

/// A plot area holding one plot element of `local`, with `body` inside it and the two shared axes
/// after it. Used by the family sweep, which has to reach all sixteen plot elements.
pub(crate) fn plot_of(local: &str, body: &str, with_axes: bool) -> Vec<u8> {
    let axes = if with_axes {
        format!(
            r#"<c:axId val="111"/><c:axId val="222"/></c:{local}>{}"#,
            axes("")
        )
    } else {
        format!("</c:{local}>")
    };
    wrap(&format!(
        r#"<c:plotArea><c:layout/><c:{local}>{body}{axes}</c:plotArea>"#
    ))
}

/// The gridline, label and decoration furniture spliced into a bar chart, for the suite that proves
/// each of them reaches the geometry.
pub(crate) fn decorated_bar_chart() -> Vec<u8> {
    let series = format!(
        r#"<c:ser><c:idx val="0"/><c:order val="0"/>
        <c:tx><c:strRef><c:f>Sheet1!$B$1</c:f><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>Revenue</c:v></c:pt></c:strCache></c:strRef></c:tx>
        <c:dLbls><c:showLegendKey val="0"/><c:showVal val="1"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/><c:showBubbleSize val="0"/></c:dLbls>
        <c:trendline><c:trendlineType val="linear"/></c:trendline>
        <c:errBars><c:errDir val="y"/><c:errBarType val="both"/><c:errValType val="percentage"/><c:noEndCap val="0"/><c:val val="10"/></c:errBars>
        <c:cat>{}</c:cat><c:val>{}</c:val></c:ser>"#,
        string_cache(&["Q1", "Q2", "Q3", "Q4"]),
        number_cache(&[Some(10.0), Some(20.0), Some(30.0), Some(40.0)])
    );
    let axes = r#"<c:catAx><c:axId val="111"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/>
           <c:title><c:tx><c:rich><a:bodyPr/><a:p><a:r><a:t>Quarter</a:t></a:r></a:p></c:rich></c:tx></c:title>
           <c:crossAx val="222"/><c:crossBetween val="between"/><c:tickLblSkip val="2"/></c:catAx>
        <c:valAx><c:axId val="222"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/>
           <c:majorGridlines/><c:minorGridlines/>
           <c:title><c:tx><c:rich><a:bodyPr/><a:p><a:r><a:t>Pounds</a:t></a:r></a:p></c:rich></c:tx></c:title>
           <c:majorTickMark val="out"/><c:crossAx val="111"/></c:valAx>"#;
    wrap(&format!(
        r#"<c:title><c:tx><c:rich><a:bodyPr/><a:p><a:r><a:t>Quarterly revenue</a:t></a:r></a:p></c:rich></c:tx><c:overlay val="0"/></c:title>
        <c:autoTitleDeleted val="0"/>
        <c:plotArea><c:layout/><c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:varyColors val="0"/>{series}
        <c:gapWidth val="150"/><c:axId val="111"/><c:axId val="222"/></c:barChart>{axes}</c:plotArea>
        <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>"#
    ))
}
