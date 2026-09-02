//! A chart's decoration — data labels and their three tiers, per-point formatting, trendlines and
//! error bars (MJX-116, part 5).
//!
//! All four families were preserved verbatim and readable not at all. The cases below read each off
//! markup shaped like Office's, author each from scratch, edit each in place, and assert that every
//! insertion lands where the generated `CT_*` sequence puts it — a child in the wrong position is
//! schema-invalid, and the reader would not notice.
//!
//! # How the inputs discriminate
//!
//! An inheritance test whose tiers agree proves nothing: a merge that ignored the middle tier
//! entirely would still pass. So [`THREE_TIER_CHART`] gives **every tier a different value for the
//! same setting** — the plot centres the label, the series pushes it to the outside end, and one
//! point pulls it inside — and leaves each tier silent about settings the tier above states, so the
//! per-setting fall-through is what is being read. The second series states nothing at all, which
//! is how the tests prove the first series' settings do not leak sideways.

use mjx_chart::{
    ChartData, ChartDataError, ChartKind, ChartSpace, DanglingPointReference, DataLabelPosition,
    DataLabelSpec, ErrorBarDirection, ErrorBarSpec, ErrorBarType, ErrorValueType, TrendlineKind,
    TrendlineSpec,
};
use mjx_dml::{ColorSpec, FillSpec, LineSpec, LineWidth};
use mjx_ooxml_core::{FromXml, RawDocument, ToXml};
use mjx_ooxml_types::child_order::{audit_tree, find};
use mjx_ooxml_types::namespaces::DML_CHART;
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

/// Asserts that every element of `xml` whose complex type the generated tables name carries its
/// children in the type's `xsd:sequence` — the same gate `mjx-pptx`'s `schema_validity` suite runs
/// over a whole deck, run here on one part.
fn assert_in_schema_order(label: &str, xml: &str) {
    let doc = fidelity::parse(xml.as_bytes()).expect("parses");
    let order = find(DML_CHART.transitional, "CT_ChartSpace").expect("CT_ChartSpace is tabulated");
    let audit = audit_tree(order, &doc.root, &doc.interner);
    assert!(
        audit.defect.is_none(),
        "{label}: {}",
        audit.defect.expect("just checked")
    );
    assert!(
        audit.elements_visited > 1,
        "{label}: the order audit visited {} element(s) — it is passing vacuously",
        audit.elements_visited
    );
}

// -------------------------------------------------------------------------------------------------
// The fixtures
// -------------------------------------------------------------------------------------------------

/// A bar chart whose data labels are stated at all three tiers, each **disagreeing** with the tier
/// above it on the settings it states:
///
/// | setting | plot | series 0 | point 1 |
/// |---|---|---|---|
/// | `showVal` | `1` | — | `0` |
/// | `showCatName` | `0` | `1` | — |
/// | `dLblPos` | `ctr` | `outEnd` | `inEnd` |
/// | `separator` | `"; "` | — | `" \| "` |
/// | `numFmt` | `0.0` | — | — |
/// | `showLeaderLines` | `1` | — | n/a |
///
/// Series 1 carries no `c:dLbls` at all, so it must resolve to the plot tier alone.
const THREE_TIER_CHART: &str = concat!(
    r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">"#,
    r#"<c:chart><c:plotArea>"#,
    r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/>"#,
    // series 0 — its own c:dLbls, with a per-point override for point 1
    r#"<c:ser><c:idx val="0"/><c:order val="0"/><c:tx><c:v>Revenue</c:v></c:tx>"#,
    r#"<c:dLbls>"#,
    r#"<c:dLbl><c:idx val="1"/><c:numFmt formatCode="0.0" sourceLinked="0"/><c:dLblPos val="inEnd"/><c:showVal val="0"/><c:separator> | </c:separator></c:dLbl>"#,
    r#"<c:dLblPos val="outEnd"/><c:showCatName val="1"/>"#,
    r#"</c:dLbls>"#,
    r#"<c:cat><c:strLit><c:ptCount val="3"/><c:pt idx="0"><c:v>Q1</c:v></c:pt><c:pt idx="1"><c:v>Q2</c:v></c:pt><c:pt idx="2"><c:v>Q3</c:v></c:pt></c:strLit></c:cat>"#,
    r#"<c:val><c:numLit><c:ptCount val="3"/><c:pt idx="0"><c:v>10</c:v></c:pt><c:pt idx="1"><c:v>20</c:v></c:pt><c:pt idx="2"><c:v>15</c:v></c:pt></c:numLit></c:val>"#,
    r#"</c:ser>"#,
    // series 1 — no c:dLbls of its own
    r#"<c:ser><c:idx val="1"/><c:order val="1"/><c:tx><c:v>Cost</c:v></c:tx>"#,
    r#"<c:cat><c:strLit><c:ptCount val="3"/><c:pt idx="0"><c:v>Q1</c:v></c:pt><c:pt idx="1"><c:v>Q2</c:v></c:pt><c:pt idx="2"><c:v>Q3</c:v></c:pt></c:strLit></c:cat>"#,
    r#"<c:val><c:numLit><c:ptCount val="3"/><c:pt idx="0"><c:v>5</c:v></c:pt><c:pt idx="1"><c:v>8</c:v></c:pt><c:pt idx="2"><c:v>7</c:v></c:pt></c:numLit></c:val>"#,
    r#"</c:ser>"#,
    // the plot tier
    r#"<c:dLbls><c:numFmt formatCode="0.0" sourceLinked="0"/><c:dLblPos val="ctr"/>"#,
    r#"<c:showVal val="1"/><c:showCatName val="0"/><c:separator>; </c:separator>"#,
    r#"<c:showLeaderLines val="1"/></c:dLbls>"#,
    r#"<c:axId val="111"/><c:axId val="222"/></c:barChart>"#,
    r#"</c:plotArea></c:chart></c:chartSpace>"#,
);

/// A line chart whose one series carries per-point formatting anchored to points 0 and 3, a label
/// override for point 3, a trendline and error bars — the four families in one part.
///
/// The two `c:dPt` are at list positions 0 and 1 but name points **0 and 3**, so an implementation
/// that addressed them by position rather than by `c:idx` would read the second as point 1.
const DECORATED_CHART: &str = concat!(
    r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">"#,
    r#"<c:chart><c:plotArea>"#,
    r#"<c:lineChart><c:grouping val="standard"/>"#,
    r#"<c:ser><c:idx val="0"/><c:order val="0"/><c:tx><c:v>Revenue</c:v></c:tx>"#,
    r#"<c:dPt><c:idx val="0"/><c:spPr><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></c:spPr></c:dPt>"#,
    r#"<c:dPt><c:idx val="3"/><c:explosion val="25"/><c:spPr><a:solidFill><a:srgbClr val="0000FF"/></a:solidFill></c:spPr></c:dPt>"#,
    r#"<c:dLbls><c:dLbl><c:idx val="3"/><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Peak</a:t></a:r></a:p></c:rich></c:tx><c:showVal val="1"/></c:dLbl><c:showVal val="0"/></c:dLbls>"#,
    r#"<c:trendline><c:name>Fit</c:name><c:trendlineType val="poly"/><c:order val="3"/><c:forward val="2.5"/><c:backward val="1"/><c:intercept val="0.5"/><c:dispRSqr val="1"/><c:dispEq val="1"/></c:trendline>"#,
    r#"<c:trendline><c:trendlineType val="movingAvg"/><c:period val="4"/></c:trendline>"#,
    r#"<c:errBars><c:errDir val="y"/><c:errBarType val="both"/><c:errValType val="cust"/><c:noEndCap val="1"/>"#,
    r#"<c:plus><c:numLit><c:formatCode>General</c:formatCode><c:ptCount val="4"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt><c:pt idx="2"><c:v>3</c:v></c:pt><c:pt idx="3"><c:v>4</c:v></c:pt></c:numLit></c:plus>"#,
    r#"<c:minus><c:numLit><c:formatCode>General</c:formatCode><c:ptCount val="4"/><c:pt idx="0"><c:v>0.5</c:v></c:pt><c:pt idx="1"><c:v>1</c:v></c:pt><c:pt idx="2"><c:v>1.5</c:v></c:pt><c:pt idx="3"><c:v>2</c:v></c:pt></c:numLit></c:minus>"#,
    r#"</c:errBars>"#,
    r#"<c:cat><c:strLit><c:ptCount val="4"/><c:pt idx="0"><c:v>Q1</c:v></c:pt><c:pt idx="1"><c:v>Q2</c:v></c:pt><c:pt idx="2"><c:v>Q3</c:v></c:pt><c:pt idx="3"><c:v>Q4</c:v></c:pt></c:strLit></c:cat>"#,
    r#"<c:val><c:numLit><c:ptCount val="4"/><c:pt idx="0"><c:v>10</c:v></c:pt><c:pt idx="1"><c:v>20</c:v></c:pt><c:pt idx="2"><c:v>15</c:v></c:pt><c:pt idx="3"><c:v>30</c:v></c:pt></c:numLit></c:val>"#,
    r#"</c:ser>"#,
    r#"<c:axId val="111"/><c:axId val="222"/></c:lineChart>"#,
    r#"</c:plotArea></c:chart></c:chartSpace>"#,
);

/// A chart whose per-point decoration is anchored past the end of a two-point series, and whose
/// third anchor is not a number at all — the shapes a hostile or corrupt file carries.
const HOSTILE_ANCHORS: &str = concat!(
    r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">"#,
    r#"<c:chart><c:plotArea>"#,
    r#"<c:barChart><c:barDir val="col"/>"#,
    r#"<c:ser><c:idx val="0"/><c:order val="0"/>"#,
    r#"<c:dPt><c:idx val="99"/><c:spPr/></c:dPt>"#,
    r#"<c:dPt><c:idx val="-1"/><c:spPr/></c:dPt>"#,
    r#"<c:dPt><c:idx val="4294967296"/><c:spPr/></c:dPt>"#,
    r#"<c:dLbls><c:dLbl><c:idx val="7"/><c:showVal val="1"/></c:dLbl><c:showVal val="0"/></c:dLbls>"#,
    r#"<c:val><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt></c:numLit></c:val>"#,
    r#"</c:ser>"#,
    r#"<c:axId val="111"/><c:axId val="222"/></c:barChart>"#,
    r#"</c:plotArea></c:chart></c:chartSpace>"#,
);

// -------------------------------------------------------------------------------------------------
// Reading
// -------------------------------------------------------------------------------------------------

#[test]
fn every_data_label_setting_reads_at_the_tier_that_states_it() {
    let (space, doc) = parse(THREE_TIER_CHART);
    let bar = space.bar_chart().expect("c:barChart");

    let plot = bar.data_labels().expect("plot-level c:dLbls");
    let plot = plot.settings(&doc.interner);
    assert_eq!(plot.shows_value, Some(true));
    assert_eq!(plot.shows_category_name, Some(false));
    assert_eq!(plot.position, Some(DataLabelPosition::Center));
    assert_eq!(plot.separator.as_deref(), Some("; "));
    assert_eq!(plot.number_format.as_deref(), Some("0.0"));
    assert_eq!(plot.shows_leader_lines, Some(true));

    let series = bar.series_at(0).expect("series 0");
    let series_labels = series.data_labels().expect("series-level c:dLbls");
    let stated = series_labels.settings(&doc.interner);
    // The series states exactly two settings and is silent about the rest.
    assert_eq!(stated.shows_category_name, Some(true));
    assert_eq!(stated.position, Some(DataLabelPosition::OutsideEnd));
    assert_eq!(stated.shows_value, None);
    assert_eq!(stated.separator, None);
    assert_eq!(stated.number_format, None);

    let point = series_labels
        .label_for_point(&doc.interner, 1)
        .expect("an override for point 1");
    assert_eq!(point.index(&doc.interner), Some(1));
    let point = point.settings(&doc.interner);
    assert_eq!(point.shows_value, Some(false));
    assert_eq!(point.position, Some(DataLabelPosition::InsideEnd));
    assert_eq!(point.separator.as_deref(), Some(" | "));
    assert_eq!(point.shows_category_name, None);

    // A point with no override of its own.
    assert!(series_labels.label_for_point(&doc.interner, 0).is_none());
    assert!(series_labels.label_for_point(&doc.interner, 2).is_none());
}

#[test]
fn the_three_tiers_merge_per_setting_and_the_middle_one_is_not_skipped() {
    let (space, doc) = parse(THREE_TIER_CHART);
    let bar = space.bar_chart().expect("c:barChart");

    // Series tier over plot tier. Every assertion below names a value that differs from the tier it
    // is *not* coming from, so a merge that dropped a tier could not produce this set.
    let series = bar.resolved_data_labels(&doc.interner, 0, None);
    assert_eq!(
        series.shows_category_name,
        Some(true),
        "the series says 1 where the plot says 0 — the middle tier must win"
    );
    assert_eq!(
        series.position,
        Some(DataLabelPosition::OutsideEnd),
        "the series says outEnd where the plot says ctr"
    );
    assert_eq!(
        series.shows_value,
        Some(true),
        "the series is silent, so the plot's 1 falls through"
    );
    assert_eq!(series.separator.as_deref(), Some("; "));
    assert_eq!(series.number_format.as_deref(), Some("0.0"));
    assert_eq!(series.shows_leader_lines, Some(true));

    // Point tier over series tier over plot tier.
    let point = bar.resolved_data_labels(&doc.interner, 0, Some(1));
    assert_eq!(
        point.shows_value,
        Some(false),
        "the point says 0 where the plot says 1 and the series is silent"
    );
    assert_eq!(
        point.position,
        Some(DataLabelPosition::InsideEnd),
        "the point says inEnd where the series says outEnd and the plot says ctr"
    );
    assert_eq!(
        point.shows_category_name,
        Some(true),
        "the point is silent, so the SERIES' 1 must survive — not the plot's 0"
    );
    assert_eq!(point.separator.as_deref(), Some(" | "));
    assert_eq!(
        point.number_format.as_deref(),
        Some("0.0"),
        "neither the point nor the series states a format, so the plot's falls all the way through"
    );

    // A point with no override of its own resolves exactly as its series does — which is also what
    // proves the override is found by `c:idx` and not by its position in the list.
    assert_eq!(bar.resolved_data_labels(&doc.interner, 0, Some(0)), series);
    assert_eq!(bar.resolved_data_labels(&doc.interner, 0, Some(2)), series);

    // A series that states nothing takes the plot tier whole — the first series' settings do not
    // leak sideways.
    let other = bar.resolved_data_labels(&doc.interner, 1, None);
    assert_eq!(
        other.shows_category_name,
        Some(false),
        "series 1 states nothing, so it takes the plot's 0 — not series 0's 1"
    );
    assert_eq!(other.position, Some(DataLabelPosition::Center));
    assert_eq!(other.shows_value, Some(true));
}

#[test]
fn per_point_formatting_trendlines_and_error_bars_all_read() {
    let (space, doc) = parse(DECORATED_CHART);
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("one series");

    assert_eq!(series.point_count(&doc.interner), 4);

    // `c:dPt` is addressed by `c:idx`, not by list position: the second element in the list names
    // point 3.
    let formats: Vec<_> = series.point_formats().collect();
    assert_eq!(formats.len(), 2);
    assert_eq!(formats[0].index(&doc.interner), Some(0));
    assert_eq!(formats[1].index(&doc.interner), Some(3));
    assert!(series.point_format(&doc.interner, 1).is_none());
    let highlighted = series
        .point_format(&doc.interner, 3)
        .expect("point 3 is formatted");
    assert_eq!(highlighted.explosion(&doc.interner), Some(25));
    assert_eq!(
        highlighted.fill(&doc.interner),
        Some(FillSpec::Solid(ColorSpec::Srgb("0000FF".into())))
    );

    // The label override for point 3 carries its own words.
    let labels = series.data_labels().expect("series c:dLbls");
    let override_3 = labels
        .label_for_point(&doc.interner, 3)
        .expect("an override for point 3");
    assert_eq!(override_3.text().as_deref(), Some("Peak"));

    let trendlines: Vec<_> = series.trendlines().collect();
    assert_eq!(trendlines.len(), 2);
    assert_eq!(
        trendlines[0].kind(&doc.interner),
        Some(TrendlineKind::Polynomial)
    );
    assert_eq!(trendlines[0].name(&doc.interner).as_deref(), Some("Fit"));
    assert_eq!(trendlines[0].order(&doc.interner), Some(3));
    assert_eq!(trendlines[0].forward_periods(&doc.interner), Some(2.5));
    assert_eq!(trendlines[0].backward_periods(&doc.interner), Some(1.0));
    assert_eq!(trendlines[0].intercept(&doc.interner), Some(0.5));
    assert_eq!(trendlines[0].displays_equation(&doc.interner), Some(true));
    assert_eq!(trendlines[0].displays_r_squared(&doc.interner), Some(true));
    assert_eq!(
        trendlines[1].kind(&doc.interner),
        Some(TrendlineKind::MovingAverage)
    );
    assert_eq!(trendlines[1].period(&doc.interner), Some(4));
    assert_eq!(trendlines[1].order(&doc.interner), None);

    let bars: Vec<_> = series.error_bars().collect();
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].direction(&doc.interner), Some(ErrorBarDirection::Y));
    assert_eq!(bars[0].bar_type(&doc.interner), Some(ErrorBarType::Both));
    assert_eq!(
        bars[0].value_type(&doc.interner),
        Some(ErrorValueType::Custom)
    );
    assert_eq!(bars[0].has_no_end_cap(&doc.interner), Some(true));
    assert_eq!(bars[0].plus_values(), vec![1.0, 2.0, 3.0, 4.0]);
    assert_eq!(bars[0].minus_values(), vec![0.5, 1.0, 1.5, 2.0]);
}

#[test]
fn a_decorated_chart_round_trips_byte_for_byte() {
    for (label, xml) in [
        ("three tiers", THREE_TIER_CHART),
        ("four families", DECORATED_CHART),
        ("hostile anchors", HOSTILE_ANCHORS),
    ] {
        let (space, doc) = parse(xml);
        assert_eq!(serialize(&space, doc), xml, "{label} did not round-trip");
    }
}

// -------------------------------------------------------------------------------------------------
// Authoring
// -------------------------------------------------------------------------------------------------

#[test]
fn an_authored_chart_can_label_itself() {
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2"])
        .series("Revenue", [10.0, 20.0])
        .data_labels(
            DataLabelSpec::new()
                .value(true)
                .category_name(true)
                .position(DataLabelPosition::OutsideEnd)
                .separator("; ")
                .number_format("#,##0")
                .leader_lines(true),
        );
    let bytes = chart.to_part_bytes();
    let xml = String::from_utf8(bytes).expect("utf-8");
    assert_in_schema_order("authored chart with data labels", &xml);

    let (space, doc) = parse(&xml);
    let bar = space.bar_chart().expect("c:barChart");
    let settings = bar.resolved_data_labels(&doc.interner, 0, None);
    assert_eq!(settings.shows_value, Some(true));
    assert_eq!(settings.shows_category_name, Some(true));
    assert_eq!(settings.position, Some(DataLabelPosition::OutsideEnd));
    assert_eq!(settings.separator.as_deref(), Some("; "));
    assert_eq!(settings.number_format.as_deref(), Some("#,##0"));
    assert_eq!(settings.shows_leader_lines, Some(true));
}

#[test]
fn authoring_labels_on_a_surface_chart_is_refused_before_anything_is_written() {
    // `CT_SurfaceChart` reaches `EG_SurfaceChartShared`, which declares no `c:dLbls` at all.
    for kind in [ChartKind::Surface, ChartKind::Surface3D] {
        let chart = ChartData::new(kind)
            .categories(["Q1", "Q2"])
            .series("Revenue", [10.0, 20.0])
            .data_labels(DataLabelSpec::new().value(true));
        assert_eq!(
            chart.validate(),
            Err(ChartDataError::DecorationNotAllowed {
                plot: kind.element_local_name(),
                element: "dLbls",
                series_type: kind.plot_child_order().symbol,
            }),
            "{kind:?} must refuse plot-level labels"
        );
    }
    // …and every other kind accepts them.
    for kind in [
        ChartKind::Bar,
        ChartKind::Line,
        ChartKind::Pie,
        ChartKind::Area,
        ChartKind::Scatter,
        ChartKind::Doughnut,
        ChartKind::Radar,
        ChartKind::Bubble,
        ChartKind::OfPie,
    ] {
        let chart = ChartData::new(kind)
            .categories(["Q1", "Q2"])
            .series("Revenue", [10.0, 20.0])
            .data_labels(DataLabelSpec::new().value(true));
        assert_eq!(chart.validate(), Ok(()), "{kind:?} must accept labels");
    }
}

#[test]
fn a_decoration_edit_places_every_child_at_its_schema_rank() {
    // Every setter below is called in the **reverse** of the order its complex type declares. If
    // placement were an append — or a no-op — the audit would fault the result. This is the case
    // that distinguishes a writer that consumes the ordering table from one that happens to be
    // handed its children in the right order already.
    let (mut space, mut doc) = parse(THREE_TIER_CHART);
    {
        let mut decoration = space.series_decoration_mut(1).expect("series 1");
        decoration
            .set_data_labels(
                &mut doc.interner,
                &DataLabelSpec::new()
                    .separator(" / ")
                    .bubble_size(false)
                    .percentage(true)
                    .series_name(true)
                    .legend_key(true)
                    .position(DataLabelPosition::BestFit)
                    .number_format("0%"),
            )
            .expect("a bar series admits data labels");
        // Points first, error bars before the trendline, the trendline before the point formats —
        // the exact reverse of `CT_BarSer`'s sequence.
        decoration
            .set_point_label(&mut doc.interner, 2, &DataLabelSpec::new().value(true))
            .expect("point 2 exists");
        decoration
            .set_point_label(&mut doc.interner, 0, &DataLabelSpec::new().value(false))
            .expect("point 0 exists");
        decoration
            .set_error_bars(
                &mut doc.interner,
                &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::StandardError, 1.0)
                    .direction(ErrorBarDirection::Y),
            )
            .expect("a bar series admits error bars");
        decoration
            .add_trendline(
                &mut doc.interner,
                &TrendlineSpec::new(TrendlineKind::Polynomial)
                    .polynomial_order(3)
                    .projection(2.0, 1.0)
                    .display(true, true),
            )
            .expect("a bar series admits a trendline");
        decoration
            .set_point_fill(
                &mut doc.interner,
                2,
                &FillSpec::Solid(ColorSpec::Srgb("FF0000".into())),
            )
            .expect("point 2 exists");
        decoration
            .set_point_fill(
                &mut doc.interner,
                0,
                &FillSpec::Solid(ColorSpec::Srgb("00FF00".into())),
            )
            .expect("point 0 exists");
    }
    let xml = serialize(&space, doc);
    assert_in_schema_order("a series decorated in reverse schema order", &xml);

    // The `c:dPt` run is written before the `c:dLbls`, and each run is in ascending `c:idx`. The
    // search is scoped to the series that was edited — the first series carries a `c:dLbls` of its
    // own, and a document-wide `find` would compare against that one instead.
    let edited = &xml[xml.find("<c:v>Cost</c:v>").expect("the edited series")..];
    let first_point = edited
        .find(r#"<c:dPt><c:idx val="0"/>"#)
        .expect("c:dPt idx 0");
    let second_point = edited
        .find(r#"<c:dPt><c:idx val="2"/>"#)
        .expect("c:dPt idx 2");
    let labels = edited.find("<c:dLbls>").expect("c:dLbls");
    let trendline = edited.find("<c:trendline>").expect("c:trendline");
    let error_bars = edited.find("<c:errBars>").expect("c:errBars");
    assert!(
        first_point < second_point && second_point < labels,
        "the c:dPt run must precede c:dLbls and be in ascending idx order, got:\n{edited}"
    );
    assert!(
        labels < trendline && trendline < error_bars,
        "CT_BarSer orders dLbls, then trendline, then errBars, got:\n{edited}"
    );

    let point_label_0 = edited
        .find(r#"<c:dLbl><c:idx val="0"/>"#)
        .expect("c:dLbl idx 0");
    let point_label_2 = edited
        .find(r#"<c:dLbl><c:idx val="2"/>"#)
        .expect("c:dLbl idx 2");
    assert!(
        labels < point_label_0 && point_label_0 < point_label_2,
        "the c:dLbl run opens CT_DLbls and must be in ascending idx order, got:\n{edited}"
    );

    // …and the tree still reads back as what was asked for.
    let (space, doc) = parse(&xml);
    let bar = space.bar_chart().expect("c:barChart");
    let resolved = bar.resolved_data_labels(&doc.interner, 1, Some(2));
    assert_eq!(resolved.shows_value, Some(true));
    assert_eq!(resolved.shows_percentage, Some(true));
    assert_eq!(resolved.number_format.as_deref(), Some("0%"));
    let series = bar.series_at(1).expect("series 1");
    assert_eq!(series.trendlines().count(), 1);
    assert_eq!(series.error_bars().count(), 1);
    assert_eq!(series.point_formats().count(), 2);
}

// -------------------------------------------------------------------------------------------------
// Editing
// -------------------------------------------------------------------------------------------------

#[test]
fn a_series_can_be_switched_from_value_to_percentage_without_touching_the_others() {
    let (mut space, mut doc) = parse(THREE_TIER_CHART);
    {
        let mut decoration = space.series_decoration_mut(0).expect("series 0");
        decoration
            .set_data_labels(
                &mut doc.interner,
                &DataLabelSpec::new().value(false).percentage(true),
            )
            .expect("a bar series admits data labels");
    }
    let xml = serialize(&space, doc);
    assert_in_schema_order("a series switched to percentage", &xml);

    let (space, doc) = parse(&xml);
    let bar = space.bar_chart().expect("c:barChart");
    let series0 = bar.resolved_data_labels(&doc.interner, 0, None);
    assert_eq!(series0.shows_value, Some(false));
    assert_eq!(series0.shows_percentage, Some(true));
    // Everything the series already said, and everything it inherits, survives the edit.
    assert_eq!(series0.shows_category_name, Some(true));
    assert_eq!(series0.position, Some(DataLabelPosition::OutsideEnd));
    assert_eq!(series0.number_format.as_deref(), Some("0.0"));
    // The other series is untouched.
    let series1 = bar.resolved_data_labels(&doc.interner, 1, None);
    assert_eq!(series1.shows_value, Some(true));
    assert_eq!(series1.shows_percentage, None);
}

#[test]
fn deleting_labels_at_one_tier_clears_the_settings_that_stood_in_their_place() {
    let (mut space, mut doc) = parse(THREE_TIER_CHART);
    {
        let mut decoration = space.series_decoration_mut(0).expect("series 0");
        decoration
            .delete_data_labels(&mut doc.interner)
            .expect("a bar series admits data labels");
    }
    let xml = serialize(&space, doc);
    assert_in_schema_order("a series whose labels are deleted", &xml);
    assert!(
        xml.contains(r#"<c:dLbls><c:delete val="1"/></c:dLbls>"#),
        "a deleted c:dLbls carries c:delete and nothing else, got:\n{xml}"
    );

    let (space, doc) = parse(&xml);
    let bar = space.bar_chart().expect("c:barChart");
    let resolved = bar.resolved_data_labels(&doc.interner, 0, None);
    assert_eq!(resolved.deleted, Some(true));
    // A deleted tier inherits nothing: `CT_DLbls` puts `c:delete` and the settings group in one
    // `xsd:choice`, so a deleted element cannot also carry a position.
    assert_eq!(resolved.position, None);
    assert_eq!(resolved.shows_value, None);
    // The plot tier itself is untouched, so the other series still labels its points.
    assert_eq!(
        bar.resolved_data_labels(&doc.interner, 1, None).shows_value,
        Some(true)
    );
}

#[test]
fn one_point_can_be_silenced_while_the_rest_of_the_series_keeps_its_labels() {
    let (mut space, mut doc) = parse(THREE_TIER_CHART);
    {
        let mut decoration = space.series_decoration_mut(0).expect("series 0");
        decoration
            .delete_point_label(&mut doc.interner, 2)
            .expect("point 2 exists");
    }
    let xml = serialize(&space, doc);
    assert_in_schema_order("one point silenced", &xml);

    let (space, doc) = parse(&xml);
    let bar = space.bar_chart().expect("c:barChart");
    assert_eq!(
        bar.resolved_data_labels(&doc.interner, 0, Some(2)).deleted,
        Some(true)
    );
    // Its neighbours are unaffected, and point 1's own override still stands.
    assert_eq!(
        bar.resolved_data_labels(&doc.interner, 0, Some(0)).deleted,
        None
    );
    assert_eq!(
        bar.resolved_data_labels(&doc.interner, 0, Some(1)).position,
        Some(DataLabelPosition::InsideEnd)
    );
}

#[test]
fn removing_a_tier_returns_it_to_what_it_inherits() {
    // Removing one point's override.
    let (mut space, doc) = parse(THREE_TIER_CHART);
    {
        let mut decoration = space.series_decoration_mut(0).expect("series 0");
        assert!(decoration.remove_point_label(&doc.interner, 1));
        assert!(
            !decoration.remove_point_label(&doc.interner, 1),
            "removing it twice must answer false, not remove a neighbour"
        );
    }
    let xml = serialize(&space, doc);
    let (space, doc) = parse(&xml);
    let bar = space.bar_chart().expect("c:barChart");
    assert_eq!(
        bar.resolved_data_labels(&doc.interner, 0, Some(1)),
        bar.resolved_data_labels(&doc.interner, 0, None),
        "point 1 now resolves exactly as its series does"
    );

    // Removing the whole series tier.
    let (mut space, doc) = parse(THREE_TIER_CHART);
    {
        let mut decoration = space.series_decoration_mut(0).expect("series 0");
        assert!(decoration.remove_data_labels());
        assert!(!decoration.remove_data_labels());
    }
    let xml = serialize(&space, doc);
    assert!(
        !xml.contains(r#"<c:dLblPos val="outEnd"/>"#),
        "the series' own settings went with its c:dLbls"
    );
    let (space, doc) = parse(&xml);
    let bar = space.bar_chart().expect("c:barChart");
    assert_eq!(
        bar.resolved_data_labels(&doc.interner, 0, None),
        bar.resolved_data_labels(&doc.interner, 1, None),
        "with its own c:dLbls gone the series takes the plot tier whole"
    );
    assert_eq!(
        bar.resolved_data_labels(&doc.interner, 0, None).position,
        Some(DataLabelPosition::Center),
        "…which is the plot's ctr, not the series' outEnd"
    );

    // Removing the plot tier leaves the series tier standing on its own.
    let (mut space, mut doc) = parse(THREE_TIER_CHART);
    {
        let area = space.plot_area_mut().expect("plot area");
        assert!(area.remove_plot_data_labels(0));
        assert!(!area.remove_plot_data_labels(0));
        assert!(!area.remove_plot_data_labels(1), "there is only one plot");
    }
    let _ = &mut doc.interner;
    let xml = serialize(&space, doc);
    let (space, doc) = parse(&xml);
    let bar = space.bar_chart().expect("c:barChart");
    let series0 = bar.resolved_data_labels(&doc.interner, 0, None);
    assert_eq!(series0.position, Some(DataLabelPosition::OutsideEnd));
    assert_eq!(series0.shows_category_name, Some(true));
    assert_eq!(
        series0.shows_value, None,
        "the plot said 1 and nothing else does, so nothing does now"
    );
    assert_eq!(
        bar.resolved_data_labels(&doc.interner, 1, None),
        Default::default(),
        "series 1 stated nothing and now inherits nothing"
    );
}

#[test]
fn a_trendline_and_error_bars_can_be_edited_in_place() {
    let (mut space, mut doc) = parse(DECORATED_CHART);
    {
        let mut decoration = space.series_decoration_mut(0).expect("series 0");
        // Replacing the y error bars in place, not adding a second set.
        decoration
            .set_error_bars(
                &mut doc.interner,
                &ErrorBarSpec::fixed(ErrorBarType::Plus, ErrorValueType::StandardDeviation, 2.0)
                    .direction(ErrorBarDirection::Y)
                    .no_end_cap(false),
            )
            .expect("a line series admits error bars");
    }
    let xml = serialize(&space, doc);
    assert_in_schema_order("edited error bars", &xml);

    let (space, doc) = parse(&xml);
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("one series");
    let bars: Vec<_> = series.error_bars().collect();
    assert_eq!(
        bars.len(),
        1,
        "the existing y bars were replaced, not joined"
    );
    assert_eq!(bars[0].bar_type(&doc.interner), Some(ErrorBarType::Plus));
    assert_eq!(
        bars[0].value_type(&doc.interner),
        Some(ErrorValueType::StandardDeviation)
    );
    assert_eq!(bars[0].value(&doc.interner), Some(2.0));
    assert_eq!(bars[0].has_no_end_cap(&doc.interner), Some(false));
    // The custom sources are gone: the spec named none, and a standard-deviation bar reads `c:val`.
    assert!(bars[0].plus_values().is_empty());
    assert!(bars[0].minus_values().is_empty());

    // The trendlines are untouched by an error-bar edit.
    assert_eq!(series.trendlines().count(), 2);
}

#[test]
fn a_trendlines_kind_and_display_flags_can_be_changed() {
    let (mut space, mut doc) = parse(DECORATED_CHART);
    {
        let mut decoration = space.series_decoration_mut(0).expect("series 0");
        assert_eq!(decoration.remove_trendlines(), 2);
        decoration
            .add_trendline(
                &mut doc.interner,
                &TrendlineSpec::new(TrendlineKind::Exponential)
                    .name("Growth")
                    .display(false, true),
            )
            .expect("a line series admits a trendline");
    }
    let xml = serialize(&space, doc);
    assert_in_schema_order("a replaced trendline", &xml);

    let (space, doc) = parse(&xml);
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("one series");
    let trendlines: Vec<_> = series.trendlines().collect();
    assert_eq!(trendlines.len(), 1);
    assert_eq!(
        trendlines[0].kind(&doc.interner),
        Some(TrendlineKind::Exponential)
    );
    assert_eq!(trendlines[0].name(&doc.interner).as_deref(), Some("Growth"));
    assert_eq!(trendlines[0].displays_equation(&doc.interner), Some(false));
    assert_eq!(trendlines[0].displays_r_squared(&doc.interner), Some(true));
}

#[test]
fn a_trendline_is_rewritten_in_place_without_disturbing_its_neighbour() {
    let (mut space, mut doc) = parse(DECORATED_CHART);
    {
        let mut decoration = space.series_decoration_mut(0).expect("series 0");
        assert!(decoration
            .set_trendline(
                &mut doc.interner,
                0,
                &TrendlineSpec::new(TrendlineKind::Logarithmic)
                    .name("Log fit")
                    .display(true, false),
            )
            .expect("a valid spec"));
        assert!(
            !decoration
                .set_trendline(
                    &mut doc.interner,
                    9,
                    &TrendlineSpec::new(TrendlineKind::Linear)
                )
                .expect("a valid spec"),
            "a trendline index past the end changes nothing"
        );
    }
    let xml = serialize(&space, doc);
    assert_in_schema_order("a trendline rewritten in place", &xml);

    let (space, doc) = parse(&xml);
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("one series");
    let trendlines: Vec<_> = series.trendlines().collect();
    assert_eq!(trendlines.len(), 2, "the edit replaced, it did not append");
    assert_eq!(
        trendlines[0].kind(&doc.interner),
        Some(TrendlineKind::Logarithmic)
    );
    assert_eq!(
        trendlines[0].name(&doc.interner).as_deref(),
        Some("Log fit")
    );
    assert_eq!(trendlines[0].displays_equation(&doc.interner), Some(true));
    assert_eq!(trendlines[0].displays_r_squared(&doc.interner), Some(false));
    // The polynomial settings the new spec does not state are cleared, not left behind to
    // contradict the new kind.
    assert_eq!(trendlines[0].order(&doc.interner), None);
    assert_eq!(trendlines[0].forward_periods(&doc.interner), None);
    assert_eq!(trendlines[0].intercept(&doc.interner), None);
    // The second trendline is untouched.
    assert_eq!(
        trendlines[1].kind(&doc.interner),
        Some(TrendlineKind::MovingAverage)
    );
    assert_eq!(trendlines[1].period(&doc.interner), Some(4));
}

#[test]
fn a_points_outline_and_explosion_can_be_edited() {
    let (mut space, mut doc) = parse(DECORATED_CHART);
    {
        let mut decoration = space.series_decoration_mut(0).expect("series 0");
        decoration
            .set_point_line(
                &mut doc.interner,
                3,
                &LineSpec {
                    width: Some(LineWidth::from_points(2.0)),
                    fill: Some(FillSpec::Solid(ColorSpec::Srgb("112233".into()))),
                    ..LineSpec::default()
                },
            )
            .expect("point 3 exists");
        decoration
            .point_format_mut(&mut doc.interner, 3)
            .expect("point 3 exists")
            .set_explosion(&mut doc.interner, Some(40));
    }
    let xml = serialize(&space, doc);
    assert_in_schema_order("an edited point format", &xml);

    let (space, doc) = parse(&xml);
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("one series");
    let format = series
        .point_format(&doc.interner, 3)
        .expect("point 3 is formatted");
    assert_eq!(format.explosion(&doc.interner), Some(40));
    assert!(format.line(&doc.interner).is_some());
    // Its fill — set before this edit — is still there.
    assert!(format.fill(&doc.interner).is_some());
    // Point 0's formatting is untouched.
    assert_eq!(
        series
            .point_format(&doc.interner, 0)
            .and_then(|format| format.fill(&doc.interner)),
        Some(FillSpec::Solid(ColorSpec::Srgb("FF0000".into())))
    );
}

// -------------------------------------------------------------------------------------------------
// `c:idx` anchoring — the subtle part
// -------------------------------------------------------------------------------------------------

#[test]
fn shortening_a_series_never_re_points_its_decoration() {
    let (mut space, mut doc) = parse(DECORATED_CHART);
    {
        let series = space.series_mut(0).expect("series 0");
        assert_eq!(series.point_count(&doc.interner), 4);
        assert!(series.set_values(&mut doc.interner, &[11.0, 12.0]));
    }
    let xml = serialize(&space, doc);

    // The markup still says point 3 — not point 1, which is what a positional rebuild would write.
    assert!(
        xml.contains(r#"<c:dPt><c:idx val="3"/>"#),
        "the c:dPt anchored to point 3 was renumbered:\n{xml}"
    );
    assert!(
        xml.contains(r#"<c:dLbl><c:idx val="3"/>"#),
        "the c:dLbl anchored to point 3 was renumbered:\n{xml}"
    );

    let (space, doc) = parse(&xml);
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("one series");
    assert_eq!(series.point_count(&doc.interner), 2);
    // The blue point is still addressed as 3, and point 1 is still unformatted.
    assert_eq!(
        series
            .point_format(&doc.interner, 3)
            .and_then(|format| format.explosion(&doc.interner)),
        Some(25)
    );
    assert!(series.point_format(&doc.interner, 1).is_none());

    // …and the now-dangling anchors are reported rather than silently kept or silently dropped.
    let dangling = series.decoration_beyond_data(&doc.interner);
    assert_eq!(
        dangling,
        vec![
            DanglingPointReference {
                element: "dPt",
                index: 3
            },
            DanglingPointReference {
                element: "dLbl",
                index: 3
            },
        ]
    );
}

#[test]
fn dangling_decoration_is_dropped_only_when_a_caller_asks() {
    let (mut space, mut doc) = parse(DECORATED_CHART);
    {
        let series = space.series_mut(0).expect("series 0");
        assert!(series.set_values(&mut doc.interner, &[11.0, 12.0]));
    }
    let dropped = {
        let mut decoration = space.series_decoration_mut(0).expect("series 0");
        decoration.drop_decoration_beyond_data(&doc.interner)
    };
    assert_eq!(dropped, 2, "one c:dPt and one c:dLbl named point 3");

    let xml = serialize(&space, doc);
    assert_in_schema_order("a series whose dangling decoration was dropped", &xml);
    assert!(!xml.contains(r#"<c:dPt><c:idx val="3"/>"#));
    assert!(!xml.contains(r#"<c:dLbl><c:idx val="3"/>"#));

    let (space, doc) = parse(&xml);
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("one series");
    assert!(series.decoration_beyond_data(&doc.interner).is_empty());
    // Point 0's formatting — which still addresses real data — is untouched.
    assert!(series.point_format(&doc.interner, 0).is_some());
    assert_eq!(
        series.trendlines().count(),
        2,
        "trendlines are not per-point"
    );
}

#[test]
fn a_hostile_index_is_read_without_panicking_and_never_written_into_new_markup() {
    let (space, doc) = parse(HOSTILE_ANCHORS);
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("one series");
    assert_eq!(series.point_count(&doc.interner), 2);

    let indices: Vec<_> = series
        .point_formats()
        .map(|format| format.index(&doc.interner))
        .collect();
    assert_eq!(
        indices,
        vec![Some(99), None, None],
        "`-1` and a value past u32::MAX are not indices; they address no point"
    );

    // Only the parsable out-of-range anchors are reported as dangling — an unparsable one addresses
    // nothing at all, so calling it "beyond the data" would be inventing a claim.
    assert_eq!(
        series.decoration_beyond_data(&doc.interner),
        vec![
            DanglingPointReference {
                element: "dPt",
                index: 99
            },
            DanglingPointReference {
                element: "dLbl",
                index: 7
            },
        ]
    );

    // Every point lookup misses, and none of them panics.
    for point in [0_u32, 1, 7, 99, u32::MAX] {
        let _ = series.point_format(&doc.interner, point);
        let _ = series
            .data_labels()
            .and_then(|labels| labels.label_for_point(&doc.interner, point));
    }
}

#[test]
fn writing_past_the_end_of_a_series_is_refused_rather_than_written() {
    let (mut space, mut doc) = parse(HOSTILE_ANCHORS);
    let mut decoration = space.series_decoration_mut(0).expect("series 0");
    let red = FillSpec::Solid(ColorSpec::Srgb("FF0000".into()));
    for point in [2_u32, 99, u32::MAX] {
        assert_eq!(
            decoration.set_point_fill(&mut doc.interner, point, &red),
            Err(ChartDataError::DataPointOutOfRange {
                index: point,
                count: 2
            }),
            "point {point} of a two-point series must be refused"
        );
        assert_eq!(
            decoration.set_point_label(&mut doc.interner, point, &DataLabelSpec::new().value(true)),
            Err(ChartDataError::DataPointOutOfRange {
                index: point,
                count: 2
            })
        );
    }
    // A point the series does have is accepted.
    assert_eq!(
        decoration.set_point_fill(&mut doc.interner, 1, &red),
        Ok(())
    );

    // The refusals wrote nothing: the file still carries exactly the three `c:dPt` it opened with,
    // plus the one this test asked for.
    let xml = {
        let _ = decoration;
        serialize(&space, doc)
    };
    assert!(!xml.contains(r#"<c:idx val="2"/>"#));
    assert!(xml.contains(r#"<c:dPt><c:idx val="1"/>"#));
}

// -------------------------------------------------------------------------------------------------
// What the schema does not admit
// -------------------------------------------------------------------------------------------------

#[test]
fn a_series_type_that_declares_no_trendline_refuses_one() {
    // `CT_PieSer` declares `dPt` and `dLbls` but neither `trendline` nor `errBars`.
    for kind in [
        ChartKind::Pie,
        ChartKind::Pie3D,
        ChartKind::Doughnut,
        ChartKind::OfPie,
        ChartKind::Radar,
        ChartKind::Surface,
        ChartKind::Surface3D,
    ] {
        assert!(
            !kind.admits_series_child("trendline"),
            "{kind:?} must not admit a trendline"
        );
        assert!(
            !kind.admits_series_child("errBars"),
            "{kind:?} must not admit error bars"
        );
    }
    for kind in [
        ChartKind::Bar,
        ChartKind::Bar3D,
        ChartKind::Line,
        ChartKind::Line3D,
        ChartKind::Stock,
        ChartKind::Area,
        ChartKind::Area3D,
        ChartKind::Scatter,
        ChartKind::Bubble,
    ] {
        assert!(
            kind.admits_series_child("trendline"),
            "{kind:?} must admit a trendline"
        );
    }
    // Only the surface series declares no decoration whatever.
    for kind in [ChartKind::Surface, ChartKind::Surface3D] {
        assert!(!kind.admits_series_child("dLbls"));
        assert!(!kind.admits_series_child("dPt"));
    }

    // And the refusal is a typed error, raised before anything is written.
    let pie = concat!(
        r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">"#,
        r#"<c:chart><c:plotArea><c:pieChart>"#,
        r#"<c:ser><c:idx val="0"/><c:order val="0"/>"#,
        r#"<c:val><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt></c:numLit></c:val>"#,
        r#"</c:ser></c:pieChart></c:plotArea></c:chart></c:chartSpace>"#,
    );
    let (mut space, mut doc) = parse(pie);
    let before = serialize(&space, fidelity::parse(pie.as_bytes()).expect("parses"));
    {
        let mut decoration = space.series_decoration_mut(0).expect("series 0");
        assert_eq!(
            decoration.add_trendline(
                &mut doc.interner,
                &TrendlineSpec::new(TrendlineKind::Linear)
            ),
            Err(ChartDataError::DecorationNotAllowed {
                plot: "pieChart",
                element: "trendline",
                series_type: "CT_PieSer",
            })
        );
        assert_eq!(
            decoration.set_error_bars(
                &mut doc.interner,
                &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::FixedValue, 1.0)
            ),
            Err(ChartDataError::DecorationNotAllowed {
                plot: "pieChart",
                element: "errBars",
                series_type: "CT_PieSer",
            })
        );
        // …but a pie slice may still be labelled and coloured.
        assert_eq!(
            decoration.set_data_labels(&mut doc.interner, &DataLabelSpec::new().percentage(true)),
            Ok(())
        );
    }
    assert_eq!(
        before, pie,
        "the refusals must leave the part exactly as it was"
    );
}

#[test]
fn leader_lines_are_refused_on_one_points_label() {
    // `Group_DLbls` declares `c:showLeaderLines`; `Group_DLbl` does not. Leader lines are drawn for
    // a whole series' labels, not for one of them.
    let (mut space, mut doc) = parse(THREE_TIER_CHART);
    let mut decoration = space.series_decoration_mut(0).expect("series 0");
    assert_eq!(
        decoration.set_point_label(
            &mut doc.interner,
            0,
            &DataLabelSpec::new().value(true).leader_lines(true)
        ),
        Err(ChartDataError::SettingNotAtThisTier {
            element: "showLeaderLines",
            parent: "dLbl",
        })
    );
    // The container tier accepts it.
    assert_eq!(
        decoration.set_data_labels(&mut doc.interner, &DataLabelSpec::new().leader_lines(false)),
        Ok(())
    );
}

#[test]
fn a_trendline_or_error_bar_outside_its_simple_types_range_is_refused() {
    for order in [0_u8, 1, 7, 255] {
        assert_eq!(
            TrendlineSpec::new(TrendlineKind::Polynomial)
                .polynomial_order(order)
                .validate(),
            Err(ChartDataError::TrendlineOrderOutOfRange { order }),
            "ST_Order admits 2 to 6, so {order} must be refused"
        );
    }
    for order in [2_u8, 3, 4, 5, 6] {
        assert_eq!(
            TrendlineSpec::new(TrendlineKind::Polynomial)
                .polynomial_order(order)
                .validate(),
            Ok(())
        );
    }
    for period in [0_u32, 1] {
        assert_eq!(
            TrendlineSpec::new(TrendlineKind::MovingAverage)
                .moving_average_period(period)
                .validate(),
            Err(ChartDataError::TrendlinePeriodOutOfRange { period })
        );
    }
    assert_eq!(
        TrendlineSpec::new(TrendlineKind::MovingAverage)
            .moving_average_period(2)
            .validate(),
        Ok(())
    );
    assert_eq!(
        TrendlineSpec::new(TrendlineKind::Linear)
            .intercept(f64::NAN)
            .validate(),
        Err(ChartDataError::NonFiniteMeasure {
            element: "intercept"
        })
    );

    // Custom error bars whose length nothing determines.
    let mut spec = ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::Custom, 1.0);
    spec.value = None;
    assert_eq!(
        spec.validate(),
        Err(ChartDataError::CustomErrorBarsNeedValues)
    );
    assert_eq!(
        ErrorBarSpec::custom(ErrorBarType::Both, vec![1.0], vec![1.0]).validate(),
        Ok(())
    );
    assert_eq!(
        ErrorBarSpec::fixed(
            ErrorBarType::Both,
            ErrorValueType::FixedValue,
            f64::INFINITY
        )
        .validate(),
        Err(ChartDataError::NonFiniteMeasure { element: "val" })
    );
}

#[test]
fn every_series_type_places_shape_properties_alike() {
    // `Series::shape_properties_mut` places `c:spPr` by the bar series' order for every kind. That
    // is only correct because `EG_SerShared` opens all eight `CT_*Ser` types, so `c:spPr` has the
    // same rank in each — this is the assertion that keeps that true.
    let ranks: Vec<_> = [
        ChartKind::Bar,
        ChartKind::Line,
        ChartKind::Pie,
        ChartKind::Area,
        ChartKind::Scatter,
        ChartKind::Radar,
        ChartKind::Bubble,
        ChartKind::Surface,
    ]
    .into_iter()
    .map(|kind| kind.series_child_order().rank_of(None, "spPr"))
    .collect();
    assert!(
        ranks.windows(2).all(|pair| pair[0] == pair[1]) && ranks[0].is_some(),
        "c:spPr must sit at one rank in every CT_*Ser, got {ranks:?}"
    );

    // The decoration children are exactly what does *not* share a rank — which is why they are
    // placed by the owning plot's kind.
    let point_ranks: Vec<_> = [ChartKind::Bar, ChartKind::Pie]
        .into_iter()
        .map(|kind| kind.series_child_order().rank_of(None, "dPt"))
        .collect();
    assert_ne!(
        point_ranks[0], point_ranks[1],
        "CT_BarSer and CT_PieSer must place c:dPt differently — otherwise this test proves nothing"
    );
}

#[test]
fn the_wire_tokens_are_exactly_what_the_schema_spells() {
    for (position, wire) in [
        (DataLabelPosition::BestFit, "bestFit"),
        (DataLabelPosition::Bottom, "b"),
        (DataLabelPosition::Center, "ctr"),
        (DataLabelPosition::InsideBase, "inBase"),
        (DataLabelPosition::InsideEnd, "inEnd"),
        (DataLabelPosition::Left, "l"),
        (DataLabelPosition::OutsideEnd, "outEnd"),
        (DataLabelPosition::Right, "r"),
        (DataLabelPosition::Top, "t"),
    ] {
        assert_eq!(position.to_wire(), wire);
        assert_eq!(DataLabelPosition::from_wire(wire), Some(position));
    }
    for (kind, wire) in [
        (TrendlineKind::Exponential, "exp"),
        (TrendlineKind::Linear, "linear"),
        (TrendlineKind::Logarithmic, "log"),
        (TrendlineKind::MovingAverage, "movingAvg"),
        (TrendlineKind::Polynomial, "poly"),
        (TrendlineKind::Power, "power"),
    ] {
        assert_eq!(kind.to_wire(), wire);
        assert_eq!(TrendlineKind::from_wire(wire), Some(kind));
    }
    for (value, wire) in [
        (ErrorValueType::Custom, "cust"),
        (ErrorValueType::FixedValue, "fixedVal"),
        (ErrorValueType::Percentage, "percentage"),
        (ErrorValueType::StandardDeviation, "stdDev"),
        (ErrorValueType::StandardError, "stdErr"),
    ] {
        assert_eq!(value.to_wire(), wire);
        assert_eq!(ErrorValueType::from_wire(wire), Some(value));
    }
    for (bar, wire) in [
        (ErrorBarType::Both, "both"),
        (ErrorBarType::Minus, "minus"),
        (ErrorBarType::Plus, "plus"),
    ] {
        assert_eq!(bar.to_wire(), wire);
        assert_eq!(ErrorBarType::from_wire(wire), Some(bar));
    }
    for (direction, wire) in [(ErrorBarDirection::X, "x"), (ErrorBarDirection::Y, "y")] {
        assert_eq!(direction.to_wire(), wire);
        assert_eq!(ErrorBarDirection::from_wire(wire), Some(direction));
    }
    // A token the schema does not admit reads as nothing rather than as a guess.
    assert_eq!(DataLabelPosition::from_wire("middle"), None);
    assert_eq!(TrendlineKind::from_wire("quadratic"), None);
    assert_eq!(ErrorValueType::from_wire("custom"), None);
}

#[test]
fn a_boolean_child_with_no_val_reads_as_its_schema_default() {
    // `CT_Boolean` defaults `@val` to `true`, `CT_TrendlineType` to `linear`, `CT_ErrBarType` to
    // `both`, `CT_ErrValType` to `fixedVal`, `CT_Order`/`CT_Period` to 2. A file that leans on those
    // defaults is not a file that says nothing.
    let xml = concat!(
        r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">"#,
        r#"<c:chart><c:plotArea><c:barChart>"#,
        r#"<c:ser><c:idx val="0"/><c:order val="0"/>"#,
        r#"<c:dLbls><c:showVal/></c:dLbls>"#,
        r#"<c:trendline><c:trendlineType/><c:order/><c:period/></c:trendline>"#,
        r#"<c:errBars><c:errBarType/><c:errValType/></c:errBars>"#,
        r#"<c:val><c:numLit><c:ptCount val="1"/><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val>"#,
        r#"</c:ser></c:barChart></c:plotArea></c:chart></c:chartSpace>"#,
    );
    let (space, doc) = parse(xml);
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("one series");
    assert_eq!(
        series
            .data_labels()
            .expect("c:dLbls")
            .settings(&doc.interner)
            .shows_value,
        Some(true)
    );
    let trendline = series.trendlines().next().expect("c:trendline");
    assert_eq!(trendline.kind(&doc.interner), Some(TrendlineKind::Linear));
    assert_eq!(trendline.order(&doc.interner), Some(2));
    assert_eq!(trendline.period(&doc.interner), Some(2));
    let bars = series.error_bars().next().expect("c:errBars");
    assert_eq!(bars.bar_type(&doc.interner), Some(ErrorBarType::Both));
    assert_eq!(
        bars.value_type(&doc.interner),
        Some(ErrorValueType::FixedValue)
    );
    assert_eq!(serialize(&space, doc), xml, "and it still round-trips");
}
