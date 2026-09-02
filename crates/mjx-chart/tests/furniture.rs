//! The chart's furniture — axes, scaling, titles, gridlines, the legend, and the styling that
//! decides what a series looks like (MJX-116, part 4).
//!
//! Every one of these was preserved verbatim and readable not at all. The tests below read each of
//! them off markup shaped like Office's, write each of them, and assert that a setter inserting an
//! element puts it where `EG_AxShared` / `CT_Chart` says it goes — a child in the wrong position is
//! schema-invalid, and the reader would not notice.

use mjx_chart::{
    AxisKind, AxisOrientation, AxisPosition, BlankDisplay, ChartSpace, LegendPosition,
    TickLabelPosition, TickMark,
};
use mjx_dml::{ColorSpec, FillSpec, LineSpec, LineWidth};
use mjx_ooxml_core::{FromXml, RawDocument, ToXml};
use mjx_xml::fidelity;

fn parse(xml: &str) -> (ChartSpace, RawDocument) {
    let doc = fidelity::parse(xml.as_bytes()).expect("fragment parses");
    let space = ChartSpace::from_xml(&doc.root, &doc.interner).expect("from_xml");
    (space, doc)
}

fn serialize(space: &ChartSpace, mut doc: RawDocument) -> String {
    doc.root = space.to_xml(&mut doc.interner);
    String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8")
}

/// A chart with one bar series, a value axis carrying every setting `EG_AxShared` admits, a
/// category axis, a title and a legend — shaped the way Office writes one.
const FURNISHED_CHART: &str = concat!(
    r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">"#,
    r#"<c:roundedCorners val="0"/><c:style val="2"/>"#,
    r#"<c:chart>"#,
    r#"<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Quarterly results</a:t></a:r></a:p></c:rich></c:tx><c:overlay val="0"/></c:title>"#,
    r#"<c:autoTitleDeleted val="0"/>"#,
    r#"<c:plotArea>"#,
    r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:varyColors val="0"/>"#,
    r#"<c:ser><c:idx val="0"/><c:order val="0"/><c:tx><c:v>Revenue</c:v></c:tx>"#,
    r#"<c:spPr><a:solidFill><a:srgbClr val="4472C4"/></a:solidFill><a:ln w="19050"><a:solidFill><a:srgbClr val="ED7D31"/></a:solidFill></a:ln></c:spPr>"#,
    r#"<c:cat><c:strLit><c:ptCount val="2"/><c:pt idx="0"><c:v>Q1</c:v></c:pt><c:pt idx="1"><c:v>Q2</c:v></c:pt></c:strLit></c:cat>"#,
    r#"<c:val><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>10</c:v></c:pt><c:pt idx="1"><c:v>20</c:v></c:pt></c:numLit></c:val>"#,
    r#"</c:ser><c:gapWidth val="150"/><c:overlap val="-27"/><c:axId val="111"/><c:axId val="222"/></c:barChart>"#,
    r#"<c:catAx><c:axId val="111"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="222"/></c:catAx>"#,
    r#"<c:valAx><c:axId val="222"/><c:scaling><c:logBase val="10"/><c:orientation val="maxMin"/><c:max val="100"/><c:min val="-5.5"/></c:scaling><c:delete val="1"/><c:axPos val="l"/><c:majorGridlines/><c:minorGridlines/>"#,
    r#"<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Millions</a:t></a:r></a:p></c:rich></c:tx></c:title>"#,
    r#"<c:numFmt formatCode="0.00%" sourceLinked="0"/><c:majorTickMark val="out"/><c:minorTickMark val="none"/><c:tickLblPos val="nextTo"/><c:crossAx val="111"/></c:valAx>"#,
    r#"</c:plotArea>"#,
    r#"<c:legend><c:legendPos val="tr"/><c:overlay val="1"/></c:legend>"#,
    r#"<c:plotVisOnly val="1"/><c:dispBlanksAs val="span"/>"#,
    r#"</c:chart></c:chartSpace>"#,
);

#[test]
fn every_axis_setting_reads() {
    let (space, doc) = parse(FURNISHED_CHART);
    let area = space.plot_area().expect("plot area");
    let axes: Vec<_> = area.axes().collect();
    assert_eq!(axes.len(), 2);

    let (kind, category) = axes[0];
    assert_eq!(kind, AxisKind::Category);
    assert_eq!(category.kind(&doc.interner), Some(AxisKind::Category));
    assert_eq!(category.axis_id(&doc.interner), Some(111));
    assert_eq!(category.cross_axis_id(&doc.interner), Some(222));
    assert_eq!(category.position(&doc.interner), Some(AxisPosition::Bottom));
    assert_eq!(category.is_deleted(&doc.interner), Some(false));
    assert!(!category.has_major_gridlines());
    assert_eq!(category.title_text(), None);

    let (kind, value) = axes[1];
    assert_eq!(kind, AxisKind::Value);
    assert_eq!(value.axis_id(&doc.interner), Some(222));
    assert_eq!(value.position(&doc.interner), Some(AxisPosition::Left));
    assert_eq!(value.is_deleted(&doc.interner), Some(true));
    assert_eq!(value.title_text().as_deref(), Some("Millions"));
    assert!(value.has_major_gridlines());
    assert!(value.has_minor_gridlines());
    assert_eq!(
        value.major_tick_mark(&doc.interner),
        Some(TickMark::Outside)
    );
    assert_eq!(value.minor_tick_mark(&doc.interner), Some(TickMark::None));
    assert_eq!(
        value.tick_label_position(&doc.interner),
        Some(TickLabelPosition::NextToAxis)
    );
    assert_eq!(value.number_format(&doc.interner), Some("0.00%"));

    let scaling = value.scaling().expect("c:scaling");
    assert_eq!(
        scaling.orientation(&doc.interner),
        Some(AxisOrientation::MaximumToMinimum)
    );
    assert_eq!(scaling.minimum(&doc.interner), Some(-5.5));
    assert_eq!(scaling.maximum(&doc.interner), Some(100.0));
    assert_eq!(scaling.logarithm_base(&doc.interner), Some(10.0));
}

#[test]
fn the_title_legend_and_chart_level_styling_read() {
    let (space, doc) = parse(FURNISHED_CHART);
    let chart = space.chart().expect("c:chart");

    assert_eq!(chart.title_text().as_deref(), Some("Quarterly results"));
    assert_eq!(chart.auto_title_deleted(&doc.interner), Some(false));
    assert_eq!(
        chart.display_blanks_as(&doc.interner),
        Some(BlankDisplay::Span)
    );
    assert_eq!(chart.plots_visible_cells_only(&doc.interner), Some(true));

    let legend = chart.legend().expect("c:legend");
    assert_eq!(
        legend.position(&doc.interner),
        Some(LegendPosition::TopRight)
    );
    assert_eq!(legend.overlays_plot(&doc.interner), Some(true));

    assert_eq!(space.style_id(&doc.interner), Some(2));
    assert_eq!(space.has_rounded_corners(&doc.interner), Some(false));
}

#[test]
fn the_series_fill_and_outline_read() {
    let (space, doc) = parse(FURNISHED_CHART);
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("a series");

    assert_eq!(
        series.fill(&doc.interner),
        Some(FillSpec::Solid(ColorSpec::Srgb("4472C4".to_owned()))),
        "the chart-level styling that decides what a series looks like"
    );
    let line = series.line(&doc.interner).expect("an outline");
    assert_eq!(line.width, Some(LineWidth::from_emu(19_050)));
    assert_eq!(
        line.fill,
        Some(FillSpec::Solid(ColorSpec::Srgb("ED7D31".to_owned())))
    );

    let plot = space.bar_chart().expect("bar plot");
    assert_eq!(plot.gap_width(&doc.interner), Some(150));
    assert_eq!(plot.overlap(&doc.interner), Some(-27));
    assert_eq!(plot.vary_colors(&doc.interner), Some(false));
    assert_eq!(plot.axis_ids(&doc.interner), vec![111, 222]);
}

#[test]
fn the_whole_furnished_chart_round_trips_byte_for_byte() {
    let (space, doc) = parse(FURNISHED_CHART);
    assert_eq!(
        serialize(&space, doc),
        FURNISHED_CHART,
        "reading the furniture must not change what is written back"
    );
}

#[test]
fn a_setter_inserts_its_element_where_the_schema_puts_it() {
    // A bare axis with no title, no gridlines and no bounds — every setter here has to insert.
    let bare = concat!(
        r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:plotArea>"#,
        r#"<c:barChart><c:barDir val="col"/><c:ser><c:idx val="0"/><c:order val="0"/><c:val><c:numLit><c:ptCount val="1"/><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser></c:barChart>"#,
        r#"<c:valAx><c:axId val="222"/><c:scaling/><c:axPos val="l"/><c:crossAx val="111"/></c:valAx>"#,
        r#"</c:plotArea><c:plotVisOnly val="1"/></c:chart></c:chartSpace>"#,
    );
    let (mut space, mut doc) = parse(bare);
    {
        let area = space.plot_area_mut().expect("plot area");
        let axis = area.axis_mut(0).expect("the value axis");
        axis.set_major_gridlines(&mut doc.interner, true);
        axis.set_title(&mut doc.interner, Some("Millions"));
        let scaling = axis.scaling_mut(&mut doc.interner);
        scaling.set_minimum(&mut doc.interner, Some(0.0));
        scaling.set_maximum(&mut doc.interner, Some(50.0));
    }
    let out = serialize(&space, doc);

    // `EG_AxShared` order: axId, scaling, delete, axPos, majorGridlines, minorGridlines, title …
    let axis_start = out.find("<c:valAx>").expect("the value axis survives");
    let axis = &out[axis_start..];
    let position = |needle: &str| {
        axis.find(needle)
            .unwrap_or_else(|| panic!("missing {needle}"))
    };
    assert!(
        position("<c:axPos") < position("<c:majorGridlines"),
        "gridlines follow the axis position: {axis}"
    );
    assert!(
        position("<c:majorGridlines") < position("<c:title"),
        "the title follows the gridlines: {axis}"
    );
    assert!(
        position("<c:title") < position("<c:crossAx"),
        "and precedes the crossing axis: {axis}"
    );
    // `CT_Scaling` order: logBase, orientation, max, min.
    assert!(
        axis.find("<c:max").expect("max") < axis.find("<c:min").expect("min"),
        "c:max precedes c:min, as CT_Scaling says: {axis}"
    );

    // And it reads back as written.
    let (space, doc) = parse(&out);
    let axis = space
        .plot_area()
        .expect("plot area")
        .axes()
        .next()
        .expect("the axis")
        .1;
    assert_eq!(axis.title_text().as_deref(), Some("Millions"));
    assert!(axis.has_major_gridlines());
    assert_eq!(
        axis.scaling().expect("scaling").minimum(&doc.interner),
        Some(0.0)
    );
    assert_eq!(
        axis.scaling().expect("scaling").maximum(&doc.interner),
        Some(50.0)
    );
}

#[test]
fn clearing_a_bound_removes_the_element_rather_than_writing_a_blank() {
    let (mut space, mut doc) = parse(FURNISHED_CHART);
    {
        let area = space.plot_area_mut().expect("plot area");
        let axis = area.axis_mut(1).expect("the value axis");
        axis.scaling_mut(&mut doc.interner)
            .set_minimum(&mut doc.interner, None);
        axis.set_major_gridlines(&mut doc.interner, false);
        axis.set_title(&mut doc.interner, None);
    }
    let out = serialize(&space, doc);
    assert!(
        !out.contains(r#"<c:min val="-5.5"/>"#),
        "the lower bound is gone, not blanked: {out}"
    );
    assert!(
        out.contains(r#"<c:max val="100"/>"#),
        "the upper bound is untouched: {out}"
    );
    assert!(
        !out.contains("<c:majorGridlines/>"),
        "the major gridlines are gone: {out}"
    );
    assert!(
        out.contains("<c:minorGridlines/>"),
        "the minor gridlines are untouched: {out}"
    );
    assert!(
        out.contains("<c:t>Quarterly results</c:t>") || out.contains(">Quarterly results<"),
        "the chart's own title survives removing the axis title: {out}"
    );
    assert!(!out.contains(">Millions<"), "the axis title is gone: {out}");
}

#[test]
fn setting_the_chart_title_clears_the_flag_that_would_hide_it() {
    // A chart that declares `c:autoTitleDeleted="1"` draws no title however many it carries.
    let bare = concat!(
        r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:autoTitleDeleted val="1"/><c:plotArea>"#,
        r#"<c:barChart><c:barDir val="col"/></c:barChart></c:plotArea><c:plotVisOnly val="1"/></c:chart></c:chartSpace>"#,
    );
    let (mut space, mut doc) = parse(bare);
    space
        .chart_mut()
        .expect("c:chart")
        .set_title(&mut doc.interner, Some("Hello"));
    let out = serialize(&space, doc);

    assert!(
        out.contains(r#"<c:autoTitleDeleted val="0"/>"#),
        "the flag is cleared: {out}"
    );
    let title = out.find("<c:title").expect("a title");
    let flag = out.find("<c:autoTitleDeleted").expect("the flag");
    let plot_area = out.find("<c:plotArea").expect("the plot area");
    assert!(
        title < flag && flag < plot_area,
        "CT_Chart order is title, autoTitleDeleted, …, plotArea: {out}"
    );

    let (space, _doc) = parse(&out);
    assert_eq!(
        space
            .chart()
            .and_then(mjx_chart::Chart::title_text)
            .as_deref(),
        Some("Hello")
    );
}

#[test]
fn a_legend_is_added_after_the_plot_area_and_moved_in_place() {
    let bare = concat!(
        r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart><c:plotArea>"#,
        r#"<c:barChart><c:barDir val="col"/></c:barChart></c:plotArea><c:plotVisOnly val="1"/></c:chart></c:chartSpace>"#,
    );
    let (mut space, mut doc) = parse(bare);
    space
        .chart_mut()
        .expect("c:chart")
        .set_legend(&mut doc.interner, Some(LegendPosition::Bottom));
    let out = serialize(&space, doc);
    let plot_area = out.find("</c:plotArea>").expect("plot area");
    let legend = out.find("<c:legend>").expect("legend");
    let visible = out.find("<c:plotVisOnly").expect("plotVisOnly");
    assert!(
        plot_area < legend && legend < visible,
        "CT_Chart order is plotArea, legend, plotVisOnly: {out}"
    );

    // Moving it rewrites the position in place rather than adding a second element.
    let (mut space, mut doc) = parse(&out);
    space
        .chart_mut()
        .expect("c:chart")
        .set_legend(&mut doc.interner, Some(LegendPosition::Left));
    let moved = serialize(&space, doc);
    assert_eq!(moved.matches("<c:legend>").count(), 1, "{moved}");
    assert!(moved.contains(r#"<c:legendPos val="l"/>"#), "{moved}");

    // And removing it takes the whole element.
    let (mut space, mut doc) = parse(&moved);
    space
        .chart_mut()
        .expect("c:chart")
        .set_legend(&mut doc.interner, None);
    let removed = serialize(&space, doc);
    assert!(!removed.contains("<c:legend"), "{removed}");
}

#[test]
fn a_series_fill_is_written_into_its_shape_properties() {
    let bare = concat!(
        r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:plotArea>"#,
        r#"<c:barChart><c:barDir val="col"/><c:ser><c:idx val="0"/><c:order val="0"/><c:tx><c:v>S</c:v></c:tx>"#,
        r#"<c:val><c:numLit><c:ptCount val="1"/><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser>"#,
        r#"</c:barChart></c:plotArea></c:chart></c:chartSpace>"#,
    );
    let (mut space, mut doc) = parse(bare);
    {
        let series = space.series_mut(0).expect("a series");
        series.set_fill(
            &mut doc.interner,
            &FillSpec::Solid(ColorSpec::Srgb("70AD47".to_owned())),
        );
        series.set_line(
            &mut doc.interner,
            &LineSpec::solid(
                LineWidth::from_points(2.0),
                ColorSpec::Srgb("000000".to_owned()),
            ),
        );
    }
    let out = serialize(&space, doc);

    // `EG_SerShared` puts `c:spPr` after `c:tx` and before the data sources; within `c:spPr` the
    // fill precedes the outline.
    let tx = out.find("<c:tx>").expect("tx");
    let sp_pr = out.find("<c:spPr>").expect("spPr");
    let val = out.find("<c:val>").expect("val");
    assert!(tx < sp_pr && sp_pr < val, "{out}");
    assert!(
        out.find("<a:solidFill>").expect("fill") < out.find("<a:ln").expect("line"),
        "{out}"
    );

    let (space, doc) = parse(&out);
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("a series");
    assert_eq!(
        series.fill(&doc.interner),
        Some(FillSpec::Solid(ColorSpec::Srgb("70AD47".to_owned())))
    );
    assert_eq!(
        series.line(&doc.interner).expect("a line").width,
        Some(LineWidth::from_points(2.0))
    );

    // Setting the fill again replaces it rather than stacking a second one.
    let (mut space, mut doc) = parse(&out);
    space
        .series_mut(0)
        .expect("a series")
        .set_fill(&mut doc.interner, &FillSpec::None);
    let again = serialize(&space, doc);
    assert_eq!(again.matches("<c:spPr>").count(), 1, "{again}");
    assert!(
        !again.contains("<a:solidFill><a:srgbClr val=\"70AD47\"/></a:solidFill>"),
        "{again}"
    );
    assert!(again.contains("<a:noFill/>"), "{again}");
}

#[test]
fn writing_drawingml_into_a_chart_that_never_declared_it_binds_the_prefix() {
    // A chart part is free not to declare `xmlns:a` — nothing forces it to until something writes
    // DrawingML into it. Writing a title without binding the prefix would emit unreadable markup.
    let bare = concat!(
        r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart><c:plotArea>"#,
        r#"<c:barChart><c:barDir val="col"/></c:barChart></c:plotArea></c:chart></c:chartSpace>"#,
    );
    let (mut space, mut doc) = parse(bare);
    assert!(
        space.ensure_drawingml_namespace(&mut doc.interner),
        "the prefix was not bound, so it is added"
    );
    assert!(
        !space.ensure_drawingml_namespace(&mut doc.interner),
        "and adding it twice is a no-op"
    );
    space
        .chart_mut()
        .expect("c:chart")
        .set_title(&mut doc.interner, Some("Bound"));
    let out = serialize(&space, doc);
    assert!(
        out.contains(r#"xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#),
        "{out}"
    );
    // The written title parses back — which it could not if `a:` were unbound.
    let reparsed = fidelity::parse(out.as_bytes()).expect("the written part parses");
    let space = ChartSpace::from_xml(&reparsed.root, &reparsed.interner).expect("from_xml");
    assert_eq!(
        space
            .chart()
            .and_then(mjx_chart::Chart::title_text)
            .as_deref(),
        Some("Bound")
    );
}
