//! **First, even and odd headers, all three different, asserted on pages one, two and three.**
//!
//! # The identity-value trap, in this child's terms
//!
//! A document with one header for every page exercises exactly one value of the selection, and a
//! build that ignored `w:titlePg` and `w:evenAndOddHeaders` entirely would pass every assertion made
//! about it. So every fixture here states **three distinct headers** and asserts which stream each
//! of pages one, two and three shows — and, because a stream index is a number a reader would not
//! notice being wrong, the three are also given **different heights**, so the body area moves too.
//!
//! # And the half that is not a selection
//!
//! A header lays out in its own band and its height comes off the body's. A fixture whose headers
//! are empty makes that untestable, so these are not: the tall header pushes the body's first line
//! down the page, which is asserted directly.

mod support;

use mjx_docx::{Document, HeaderFooterType, PartName, SectionLocation};
use mjx_layout::{BoxModel, Fragment, PageIndex};
use mjx_layout_docx::PageReport;
use mjx_ooxml_core::measure::Emu;
use support::{constraints, flow, model, FAMILY};

/// A `w:hdr` part holding one paragraph of `text` at `points`.
fn header_markup(local: &str, text: &str, points: f64) -> Vec<u8> {
    #[allow(clippy::cast_possible_truncation)]
    let half = (points * 2.0).round() as i64;
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<w:{local} xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
            r#"<w:p><w:r><w:rPr><w:rFonts w:ascii="{family}" w:hAnsi="{family}"/>"#,
            r#"<w:sz w:val="{half}"/></w:rPr><w:t xml:space="preserve">{text}</w:t></w:r></w:p>"#,
            r#"</w:{local}>"#,
        ),
        local = local,
        family = FAMILY,
        half = half,
        text = text
    )
    .into_bytes()
}

/// The three headers this suite uses: distinct text, and deliberately distinct heights.
const HEADERS: [(HeaderFooterType, &str, f64); 3] = [
    (HeaderFooterType::First, "First page header", 36.0),
    (HeaderFooterType::Even, "Even page header", 11.0),
    (HeaderFooterType::Default, "Odd page header", 11.0),
];

/// A document of `paragraphs`, with the three headers wired to the body-level section.
///
/// `section` is the body `w:sectPr`'s own content and `even_and_odd` writes
/// `w:settings/w:evenAndOddHeaders`, which is the document-wide half of the selection.
fn document_with_headers(paragraphs: &[String], section: &str, even_and_odd: bool) -> Document {
    let markup = support::document_markup_with(paragraphs, section);
    let mut document = support::document_from_markup(markup, |settings, interner| {
        settings.set_even_and_odd_headers(interner, Some(even_and_odd));
    });
    let mut parts: Vec<(PartName, Vec<u8>)> = Vec::new();
    for (kind, text, points) in HEADERS {
        let part = document
            .create_header(SectionLocation::Body, kind)
            .expect("a header part");
        parts.push((part, header_markup("hdr", text, points)));
    }
    let bytes = document.save_unchecked().expect("the document saves");
    let mut package = mjx_docx::Package::open(&bytes).expect("the package opens");
    for (part, content) in parts {
        package
            .replace_part_bytes(&part, content)
            .expect("the header part is replaceable");
    }
    Document::from_package(package).expect("the document reopens")
}

/// A small sheet, so that thirty short paragraphs really do become several pages.
///
/// The **section** states it rather than the caller's constraints, because MJXOFF-175 made the
/// document's own `w:sectPr` outrank them — which is the whole point of `crate::section` and is why
/// a fixture that wants a small page has to say so in the file.
fn small_page() -> String {
    support::page_geometry(6.5, 2.5, 0.25)
}

/// Thirty short paragraphs — several pages of the sheet above.
fn body() -> Vec<String> {
    (0..30)
        .map(|index| support::paragraph("", &format!("Paragraph {index}.")))
        .collect()
}

/// Every page's report, with the **name of the part** each showed as its header.
///
/// The name and not the stream index: a stream index is an offset into one document's own table, and
/// two documents that differ in `w:titlePg` do not build that table in the same order — so
/// comparing indices across two documents compares nothing. This is the mistake this helper exists
/// to make impossible.
fn reports(
    mut document: Document,
    area: &mjx_layout::Constraints,
    limit: usize,
) -> Vec<(PageReport, Option<String>)> {
    let flow = flow(&mut document);
    let mut model = model();
    let mut out = Vec::new();
    let mut resume = None;
    for number in 0..limit {
        let Ok(page) = model.layout_page(
            &flow,
            PageIndex::new(u32::try_from(number).expect("a page number")),
            area,
            resume.as_ref(),
        ) else {
            break;
        };
        resume = page.continuation().cloned();
        let report = model.last_page().clone();
        let part = report.header.and_then(|stream| {
            flow.formatting()
                .header_footer_stream(stream)
                .map(|held| held.part().as_str().to_owned())
        });
        out.push((report, part));
        if resume.is_none() {
            break;
        }
    }
    out
}

/// **The assertion the ticket asks for.** Three headers, three pages, three different streams.
#[test]
fn the_first_even_and_odd_headers_each_appear_on_their_own_page() {
    let geometry = small_page();
    let document = document_with_headers(&body(), &format!("{geometry}<w:titlePg/>"), true);
    let pages = reports(document, &constraints(6.5, 2.5), 8);
    assert!(pages.len() >= 3, "three pages at least: {}", pages.len());

    let shown: Vec<Option<String>> = pages.iter().take(3).map(|(_, part)| part.clone()).collect();
    assert!(
        shown.iter().all(Option::is_some),
        "every page shows a header: {shown:?}"
    );
    assert_ne!(shown[0], shown[1], "page one is the first-page header");
    assert_ne!(shown[1], shown[2], "page two is the even header");
    assert_ne!(shown[0], shown[2], "page three is the odd one");
}

/// `w:titlePg` off makes page one show the **odd** header, which is §17.10.6's own sentence: a
/// first-page reference that exists is ignored when the flag is not set.
#[test]
fn title_page_off_makes_page_one_show_the_odd_header() {
    let geometry = small_page();
    let area = constraints(6.5, 2.5);
    let on = reports(
        document_with_headers(&body(), &format!("{geometry}<w:titlePg/>"), true),
        &area,
        8,
    );
    let off = reports(document_with_headers(&body(), &geometry, true), &area, 8);
    assert_ne!(
        on[0].1, off[0].1,
        "the flag must change which header page one shows"
    );
    assert_eq!(
        off[0].1, off[2].1,
        "and with it off, page one shows exactly what page three does"
    );
}

/// `w:evenAndOddHeaders` off makes page two show the odd header — §17.10.1, the same shape as
/// `w:titlePg` and for the even variant.
#[test]
fn even_and_odd_headers_off_makes_page_two_show_the_odd_header() {
    let geometry = small_page();
    let area = constraints(6.5, 2.5);
    let on = reports(
        document_with_headers(&body(), &format!("{geometry}<w:titlePg/>"), true),
        &area,
        8,
    );
    let off = reports(
        document_with_headers(&body(), &format!("{geometry}<w:titlePg/>"), false),
        &area,
        8,
    );
    assert_ne!(on[1].1, off[1].1);
    assert_eq!(
        off[1].1, off[2].1,
        "with the setting off there is no even header at all"
    );
}

/// **A header changes the body's available height**, which is the half of this that no selection
/// assertion can see.
#[test]
fn a_tall_header_pushes_the_body_down_the_page() {
    let geometry = small_page();
    let area = constraints(6.5, 2.5);
    // The first-page header is 36 point and the odd one is 11, and `w:pgMar@header` is half an inch
    // — so the tall one grows past the quarter-inch top margin further than the short one does.
    let pages = reports(
        document_with_headers(&body(), &format!("{geometry}<w:titlePg/>"), true),
        &area,
        8,
    );
    assert!(
        pages[0].0.body.top > pages[2].0.body.top,
        "the 36-point first-page header must push the body further down than the 11-point odd one: \
         {:?} against {:?}",
        pages[0].0.body.top,
        pages[2].0.body.top
    );
    assert!(
        pages[0].0.body.height() < pages[2].0.body.height(),
        "and the body it leaves is therefore shorter"
    );
}

/// The header's own text is on the page, as fragments in its own part.
#[test]
fn the_header_draws_its_own_text_in_its_own_part() {
    let geometry = small_page();
    let mut document = document_with_headers(&body(), &format!("{geometry}<w:titlePg/>"), true);
    let flow = flow(&mut document);
    let mut model = model();
    let page = model
        .layout_page(&flow, PageIndex::FIRST, &constraints(6.5, 2.5), None)
        .expect("page one lays out");
    let header_runs = page
        .fragments()
        .nodes()
        .filter(|(_, node)| matches!(node.fragment(), Fragment::GlyphRun(_)))
        .filter(|(_, node)| node.source().part() == mjx_layout_docx::address::HEADER)
        .count();
    assert!(
        header_runs > 0,
        "the header's glyphs must be on the page, in the header part rather than the body's"
    );
    let header_top = page
        .fragments()
        .nodes()
        .filter(|(_, node)| node.source().part() == mjx_layout_docx::address::HEADER)
        .map(|(_, node)| node.rect().top)
        .min()
        .expect("a header fragment");
    assert!(
        header_top < Emu::from_inches(0.75),
        "and it sits in the margin above the body, at `w:pgMar@header`: {header_top:?}"
    );
}
