//! The data sources a series can name beyond a cached workbook reference (MJX-116, part 3).
//!
//! `CT_NumDataSource` is a choice of `c:numRef` **or** `c:numLit`; `CT_AxDataSource` adds
//! `c:strLit`, `c:numLit` and `c:multiLvlStrRef`. Until this tier only the reference forms read, so
//! a chart whose data is written inline — which is exactly what a chart with no workbook behind it
//! looks like — read as empty. All of them read here, and the two literal forms are editable: a
//! literal *is* the data, so rewriting it is rewriting the chart.

use mjx_chart::ChartSpace;
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

/// Wraps one `c:ser` body in the chart-space spine and a bar plot.
fn wrap(series_body: &str) -> String {
    format!(
        r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart><c:plotArea><c:barChart><c:barDir val="col"/><c:ser><c:idx val="0"/><c:order val="0"/>{series_body}</c:ser></c:barChart></c:plotArea></c:chart></c:chartSpace>"#
    )
}

const STRING_LITERAL_CATEGORIES: &str = r#"<c:cat><c:strLit><c:ptCount val="3"/><c:pt idx="0"><c:v>North</c:v></c:pt><c:pt idx="1"><c:v>South</c:v></c:pt><c:pt idx="2"><c:v>West</c:v></c:pt></c:strLit></c:cat>"#;
const NUMBER_LITERAL_VALUES: &str = r#"<c:val><c:numLit><c:formatCode>General</c:formatCode><c:ptCount val="3"/><c:pt idx="0"><c:v>19.2</c:v></c:pt><c:pt idx="1"><c:v>21.4</c:v></c:pt><c:pt idx="2"><c:v>16.7</c:v></c:pt></c:numLit></c:val>"#;

#[test]
fn literal_categories_and_values_read() {
    let xml = wrap(&format!(
        "{STRING_LITERAL_CATEGORIES}{NUMBER_LITERAL_VALUES}"
    ));
    let (space, doc) = parse(&xml);
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("a series");

    assert_eq!(
        series.categories().expect("c:cat").labels(),
        vec!["North", "South", "West"],
        "a c:strLit is the data, not a gap"
    );
    assert_eq!(
        series.values().expect("c:val").values(),
        vec![19.2, 21.4, 16.7],
        "a c:numLit is the data, not a gap"
    );
    // Nothing about the source's shape is invented: it is a literal, not a reference.
    assert!(series.values().expect("c:val").reference().is_none());
    assert!(series.values().expect("c:val").literal().is_some());
    assert_eq!(
        serialize(&space, doc),
        xml,
        "a literal round-trips verbatim"
    );
}

#[test]
fn numeric_literal_categories_read_as_numbers_and_as_labels() {
    let categories = r#"<c:cat><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>1.5</c:v></c:pt><c:pt idx="1"><c:v>2.5</c:v></c:pt></c:numLit></c:cat>"#;
    let (space, _doc) = parse(&wrap(&format!("{categories}{NUMBER_LITERAL_VALUES}")));
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("a series");
    let categories = series.categories().expect("c:cat");

    assert!(categories.is_numeric(), "a c:numLit source is numeric");
    assert_eq!(categories.values(), vec![1.5, 2.5]);
    assert_eq!(
        categories.labels(),
        vec!["1.5", "2.5"],
        "read as labels, a number is its exact wire text — never reformatted"
    );
}

#[test]
fn a_literal_source_is_editable() {
    let xml = wrap(&format!(
        "{STRING_LITERAL_CATEGORIES}{NUMBER_LITERAL_VALUES}"
    ));
    let (mut space, mut doc) = parse(&xml);
    {
        let series = space.series_mut(0).expect("a series");
        assert!(
            series.set_values(&mut doc.interner, &[1.0, 2.0]),
            "a literal is the data, so it can be rewritten"
        );
        assert!(
            series.set_categories(&mut doc.interner, &["A", "B"]),
            "so can a string literal"
        );
    }
    let out = serialize(&space, doc);
    assert!(
        out.contains(r#"<c:numLit><c:formatCode>General</c:formatCode><c:ptCount val="2"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt></c:numLit>"#),
        "the literal is rewritten in place, keeping its format code: {out}"
    );
    assert!(
        out.contains(r#"<c:strLit><c:ptCount val="2"/><c:pt idx="0"><c:v>A</c:v></c:pt><c:pt idx="1"><c:v>B</c:v></c:pt></c:strLit>"#),
        "and so is the string literal: {out}"
    );

    // Reading it back agrees with what was written.
    let (space, _) = parse(&out);
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("a series");
    assert_eq!(series.values().expect("c:val").values(), vec![1.0, 2.0]);
    assert_eq!(series.categories().expect("c:cat").labels(), vec!["A", "B"]);
}

#[test]
fn multi_level_categories_read_every_level() {
    let categories = r#"<c:cat><c:multiLvlStrRef><c:f>Sheet1!$A$2:$B$5</c:f><c:multiLvlStrCache><c:ptCount val="4"/><c:lvl><c:pt idx="0"><c:v>Jan</c:v></c:pt><c:pt idx="1"><c:v>Feb</c:v></c:pt><c:pt idx="2"><c:v>Mar</c:v></c:pt><c:pt idx="3"><c:v>Apr</c:v></c:pt></c:lvl><c:lvl><c:pt idx="0"><c:v>Q1</c:v></c:pt><c:pt idx="3"><c:v>Q2</c:v></c:pt></c:lvl></c:multiLvlStrCache></c:multiLvlStrRef></c:cat>"#;
    let xml = wrap(&format!("{categories}{NUMBER_LITERAL_VALUES}"));
    let (space, doc) = parse(&xml);
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("a series");
    let categories = series.categories().expect("c:cat");

    assert_eq!(
        categories.levels(),
        vec![
            vec![
                "Jan".to_owned(),
                "Feb".to_owned(),
                "Mar".to_owned(),
                "Apr".to_owned()
            ],
            vec!["Q1".to_owned(), "Q2".to_owned()],
        ],
        "every c:lvl reads, in document order"
    );
    assert_eq!(
        categories.labels(),
        vec!["Jan", "Feb", "Mar", "Apr"],
        "a multi-level axis read flat is its first level"
    );
    assert_eq!(
        categories
            .multi_level_reference()
            .and_then(mjx_chart::MultiLevelStringReference::formula)
            .map(mjx_chart::Formula::text)
            .as_deref(),
        Some("Sheet1!$A$2:$B$5"),
        "the range it names reads too"
    );
    assert!(
        !categories.is_numeric(),
        "a multi-level source is string data"
    );
    assert_eq!(serialize(&space, doc), xml, "and it round-trips verbatim");
}

#[test]
fn a_multi_level_or_numeric_source_refuses_a_label_rewrite() {
    // There is no string cache to rewrite, and inventing one would change what the chart draws from
    // under the caller: the setter says so instead.
    let categories =
        r#"<c:cat><c:multiLvlStrRef><c:f>Sheet1!$A$2:$B$5</c:f></c:multiLvlStrRef></c:cat>"#;
    let (mut space, mut doc) = parse(&wrap(&format!("{categories}{NUMBER_LITERAL_VALUES}")));
    assert!(
        !space
            .series_mut(0)
            .expect("a series")
            .set_categories(&mut doc.interner, &["A"]),
        "a multi-level source has no flat labels to rewrite"
    );
}

#[test]
fn an_empty_multi_level_cache_reads_as_no_labels_not_as_a_panic() {
    let categories = r#"<c:cat><c:multiLvlStrRef><c:f>Sheet1!$A$1</c:f><c:multiLvlStrCache/></c:multiLvlStrRef></c:cat>"#;
    let (space, _doc) = parse(&wrap(&format!("{categories}{NUMBER_LITERAL_VALUES}")));
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("a series");
    assert!(series.categories().expect("c:cat").labels().is_empty());
    assert!(series.categories().expect("c:cat").levels().is_empty());
}
