//! What every suite here needs: a deterministic font tier, a document built to order, and the
//! shapes a pagination assertion is written in.
//!
//! # No system fonts, ever
//!
//! A layout gate that resolved faces through the platform would assert one thing on a developer's
//! machine and another on CI, and the failure would look like a layout regression. So the resolver
//! these suites build indexes **only** the faces `mjx-text` commits under `assets/fonts/`, and every
//! fixture asks for a family they cover.
//!
//! # Why a document is authored as markup rather than through the `Document` surface
//!
//! Almost everything this crate implements is stated in `w:pPr` — an alignment, a `w:keepNext`, a
//! tab ruler, a line rule — and a pagination fixture needs a *dozen* paragraphs each carrying a
//! different one. Building `word/document.xml` as bytes and replacing it in a blank package is the
//! shortest honest route, and it keeps the fixture legible: a test that asserts something about
//! `w:widowControl` has `w:widowControl` visible three lines above the assertion.
//!
//! `mjx-layout-xlsx`'s own support module makes the same argument for the same reason; this is that
//! argument applied to a different markup.

#![allow(dead_code)]

use std::path::PathBuf;

use mjx_docx::{Document, PageSize, PartName};
use mjx_layout::{
    BoxModel, Constraints, Fragment, FragmentTree, LayoutRect, LayoutSize, PageFragments, PageIndex,
};
use mjx_layout_docx::{
    lay_out, DocumentBoxModel, DocumentFlow, FlowContext, ParagraphLayout, TextEngine,
};
use mjx_ooxml_core::measure::Emu;
use mjx_text::FontResolver;

/// The bundled tier's directory — the faces `mjx-text` commits.
pub(crate) fn bundled_fonts() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../mjx-text/assets/fonts")
}

/// A resolver with the bundled faces and nothing else.
pub(crate) fn resolver() -> FontResolver {
    FontResolver::builder()
        .with_bundled_font_directory(&bundled_fonts())
        .expect("the committed faces index")
        .build()
}

/// A box model over the bundled faces.
pub(crate) fn model() -> DocumentBoxModel {
    DocumentBoxModel::new(resolver())
}

/// A family every bundled face answers, so a fixture never depends on what is installed.
pub(crate) const FAMILY: &str = "Liberation Sans";

/// The main document part of a blank document.
pub(crate) const DOCUMENT_PART: &str = "/word/document.xml";

/// The styles part, which a blank document does **not** have — see `mjx_docx::blank`.
pub(crate) const STYLES_PART: &str = "/word/styles.xml";

/// The settings part, likewise absent from a blank document.
pub(crate) const SETTINGS_PART: &str = "/word/settings.xml";

/// A `w:rPr` naming [`FAMILY`] at `points`.
pub(crate) fn run_properties(points: f64) -> String {
    let half = (points * 2.0).round() as i64;
    format!(
        r#"<w:rPr><w:rFonts w:ascii="{FAMILY}" w:hAnsi="{FAMILY}"/><w:sz w:val="{half}"/></w:rPr>"#
    )
}

/// One paragraph: `properties` is the body of its `w:pPr` (empty for none) and `text` its single
/// run's text.
pub(crate) fn paragraph(properties: &str, text: &str) -> String {
    paragraph_at(properties, text, 11.0)
}

/// The same at a stated point size.
pub(crate) fn paragraph_at(properties: &str, text: &str, points: f64) -> String {
    let ppr = if properties.is_empty() {
        String::new()
    } else {
        format!("<w:pPr>{properties}</w:pPr>")
    };
    format!(
        r#"<w:p>{ppr}<w:r>{}<w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
        run_properties(points),
        escape(text)
    )
}

/// A paragraph whose single run holds `inner` verbatim — for `w:br`, `w:tab`, `w:softHyphen`.
pub(crate) fn paragraph_with_run(properties: &str, inner: &str) -> String {
    let ppr = if properties.is_empty() {
        String::new()
    } else {
        format!("<w:pPr>{properties}</w:pPr>")
    };
    format!(
        r#"<w:p>{ppr}<w:r>{}{inner}</w:r></w:p>"#,
        run_properties(11.0)
    )
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// A `word/document.xml` whose body is `paragraphs`, with a US Letter section at the end.
pub(crate) fn document_markup(paragraphs: &[String]) -> Vec<u8> {
    let body = paragraphs.concat();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>{body}<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr></w:body>
</w:document>"#
    )
    .into_bytes()
}

/// A document whose body is `paragraphs`.
pub(crate) fn document(paragraphs: &[String]) -> Document {
    document_with_settings(paragraphs, |_, _| ())
}

/// The same, with `word/settings.xml` written by `settings` first.
///
/// The settings part is created by editing it — a blank document relates to none — and the document
/// is then saved and reopened so that the authored `word/document.xml` replaces the blank one in a
/// package that already carries the settings.
pub(crate) fn document_with_settings(
    paragraphs: &[String],
    settings: impl FnOnce(&mut mjx_docx::DocumentSettings, &mut mjx_ooxml_core::Interner),
) -> Document {
    let mut blank = Document::blank(PageSize::us_letter()).expect("a blank document");
    blank
        .edit_document_settings(|part, interner| settings(part, interner))
        .expect("a settings part is creatable");
    let bytes = blank.save_unchecked().expect("blank saves");
    let mut package = mjx_docx::Package::open(&bytes).expect("the blank package opens");
    package
        .replace_part_bytes(
            &PartName::new(DOCUMENT_PART).expect("a valid part name"),
            document_markup(paragraphs),
        )
        .expect("the main document part is replaceable");
    Document::from_package(package).expect("the authored document opens")
}

/// The flow a document becomes.
pub(crate) fn flow(document: &mut Document) -> DocumentFlow {
    DocumentFlow::read(document).expect("the document is readable")
}

/// A page `inches` tall and 6.5 inches wide, with no margins, so a fixture can say "four lines fit"
/// and mean it.
pub(crate) fn constraints(width_inches: f64, height_inches: f64) -> Constraints {
    let page = LayoutSize {
        width: Emu::from_inches(width_inches),
        height: Emu::from_inches(height_inches),
    };
    Constraints {
        page,
        content: LayoutRect::from_edges(Emu::ZERO, Emu::ZERO, page.width, page.height),
        columns: 1,
        column_gap: Emu::ZERO,
        base_direction: mjx_text::TextDirection::LeftToRight,
        writing_mode: mjx_layout::WritingMode::HorizontalTopToBottom,
    }
}

/// Every page of `flow`, laid out **by walking**, each from the previous page's checkpoint.
///
/// The reference every resumption assertion is compared against, and the shape a caller actually
/// uses: keep the checkpoints, drop the fragments.
pub(crate) fn walk(
    model: &mut DocumentBoxModel,
    flow: &DocumentFlow,
    constraints: &Constraints,
    limit: usize,
) -> Vec<PageFragments> {
    let mut pages = Vec::new();
    let mut resume = None;
    for number in 0..limit {
        let page = model
            .layout_page(
                flow,
                PageIndex::new(u32::try_from(number).expect("a page number")),
                constraints,
                resume.as_ref(),
            )
            .expect("a page lays out");
        resume = page.continuation().cloned();
        let last = resume.is_none();
        pages.push(page);
        if last {
            break;
        }
    }
    pages
}

/// Which page each paragraph's first line landed on, by walking.
///
/// **The assertion pagination gates are written in.** A gate on a page's *content* is green for an
/// implementation that honours no constraint at all, because the same text is on the same pages in
/// a different order; a gate on which page a named paragraph is on is not.
pub(crate) fn page_of_each_paragraph(
    model: &mut DocumentBoxModel,
    flow: &DocumentFlow,
    constraints: &Constraints,
    limit: usize,
) -> Vec<Option<usize>> {
    let mut answer = vec![None; flow.paragraph_count()];
    for (number, page) in walk(model, flow, constraints, limit).iter().enumerate() {
        for paragraph in paragraphs_on(page.fragments()) {
            if answer.get(paragraph).is_some_and(Option::is_none) {
                answer[paragraph] = Some(number);
            }
        }
    }
    answer
}

/// Which paragraphs a page holds, by the first segment of every fragment's address.
pub(crate) fn paragraphs_on(tree: &FragmentTree) -> Vec<usize> {
    let mut found: Vec<usize> = tree
        .nodes()
        .filter(|(_, node)| matches!(node.fragment(), Fragment::Box(_)))
        .filter_map(|(_, node)| node.source().path().segments().first().copied())
        .map(|index| index as usize)
        .collect();
    found.sort_unstable();
    found.dedup();
    found
}

/// How many lines a page holds.
pub(crate) fn line_count(tree: &FragmentTree) -> usize {
    tree.nodes()
        .filter(|(_, node)| matches!(node.fragment(), Fragment::Line(_)))
        .count()
}

/// A compact, comparable rendering of a tree: one line per fragment, in tree order.
///
/// Every number is an EMU, so two trees that differ by one EMU differ here — which is what a
/// resumption gate needs, because *almost* identical is the failure it is looking for.
pub(crate) fn snapshot(tree: &FragmentTree) -> String {
    let mut out = String::new();
    for (id, node) in tree.nodes() {
        let rect = node.rect();
        out.push_str(&format!(
            "{:>4} {:<9} {:?} [{} {} {} {}]\n",
            id.index(),
            node.fragment().kind_name(),
            node.source().path().segments(),
            rect.left.emu(),
            rect.top.emu(),
            rect.right.emu(),
            rect.bottom.emu(),
        ));
    }
    out
}

/// How tall one line of eleven-point [`FAMILY`] is, measured rather than assumed.
///
/// Every pagination fixture below sizes its page as a multiple of this. A hard-coded height would
/// be a fixture that silently stops testing what it says the day a bundled face is replaced — the
/// page would hold four lines instead of three and every "which page is this paragraph on"
/// assertion would still pass, against a different arrangement.
pub(crate) fn measured_line_height() -> Emu {
    let mut document = document(&[paragraph("", "One line.")]);
    let flow = flow(&mut document);
    let mut model = model();
    let page = model
        .layout_page(&flow, PageIndex::FIRST, &constraints(6.5, 20.0), None)
        .expect("one line lays out");
    let height = page
        .fragments()
        .nodes()
        .find(|(_, node)| matches!(node.fragment(), Fragment::Line(_)))
        .map(|(_, node)| node.rect().height())
        .expect("a line fragment");
    height
}

/// Constraints for a page that holds exactly `lines` lines of eleven-point [`FAMILY`].
///
/// A tenth of a line is added so that rounding cannot make the last line miss by one EMU, and it is
/// far too little to admit another.
pub(crate) fn page_of_lines(lines: i64) -> Constraints {
    let height = measured_line_height();
    let page = LayoutSize {
        width: Emu::from_inches(6.5),
        height: height.times(lines) + height.divided_by(10),
    };
    Constraints {
        page,
        content: LayoutRect::from_edges(Emu::ZERO, Emu::ZERO, page.width, page.height),
        columns: 1,
        column_gap: Emu::ZERO,
        base_direction: mjx_text::TextDirection::LeftToRight,
        writing_mode: mjx_layout::WritingMode::HorizontalTopToBottom,
    }
}

/// Which page the paragraph at `index` lands on, in a document of `paragraphs`.
pub(crate) fn page_of_paragraph(
    paragraphs: &[String],
    constraints: &Constraints,
    index: usize,
) -> Option<usize> {
    let mut document = document(paragraphs);
    let flow = flow(&mut document);
    let mut model = model();
    page_of_each_paragraph(&mut model, &flow, constraints, 40)
        .get(index)
        .copied()
        .flatten()
}

/// Where every glyph run on line `line` of paragraph `paragraph` starts, in EMU from the page's
/// left edge, in draw order.
///
/// **The shape a justification assertion is written in.** A line's *width* is the same under a
/// correct justifier and under one that pads the end and moves nothing, so a width assertion is not
/// an assertion about justification at all. Positions are.
pub(crate) fn glyph_positions(tree: &FragmentTree, paragraph: usize, line: usize) -> Vec<i64> {
    tree.nodes()
        .filter(|(_, node)| matches!(node.fragment(), Fragment::GlyphRun(_)))
        .filter(|(_, node)| {
            let path = node.source().path().segments();
            path.first().copied() == u32::try_from(paragraph).ok()
                && path.get(1).copied() == u32::try_from(line).ok()
        })
        .map(|(_, node)| node.rect().left.emu())
        .collect()
}

/// How many lines paragraph `paragraph` has on `tree`.
pub(crate) fn lines_of(tree: &FragmentTree, paragraph: usize) -> usize {
    tree.nodes()
        .filter(|(_, node)| matches!(node.fragment(), Fragment::Line(_)))
        .filter(|(_, node)| {
            node.source().path().segments().first().copied() == u32::try_from(paragraph).ok()
        })
        .count()
}

/// The one page a document of `paragraphs` lays out into, at `constraints`.
pub(crate) fn one_page(paragraphs: &[String], constraints: &Constraints) -> FragmentTree {
    let mut document = document(paragraphs);
    let flow = flow(&mut document);
    let mut model = model();
    let page = model
        .layout_page(&flow, PageIndex::FIRST, constraints, None)
        .expect("the page lays out");
    page.into_parts().0
}

/// Where every glyph run on one line **would** start if nothing had been moved: the running sum of
/// the runs' own widths, from the line's own leading edge.
///
/// This is what a justification assertion compares against, and it has to be computed from the
/// justified layout itself rather than from a left-aligned one: the two are cut differently (a
/// justified paragraph is cut at every space so its words can move independently), so a
/// left-aligned line is one run where a justified one is seventeen, and the two are not comparable
/// position by position.
pub(crate) fn natural_positions(tree: &FragmentTree, paragraph: usize, line: usize) -> Vec<i64> {
    let widths: Vec<i64> = tree
        .nodes()
        .filter(|(_, node)| matches!(node.fragment(), Fragment::GlyphRun(_)))
        .filter(|(_, node)| {
            let path = node.source().path().segments();
            path.first().copied() == u32::try_from(paragraph).ok()
                && path.get(1).copied() == u32::try_from(line).ok()
        })
        .map(|(_, node)| node.rect().width().emu())
        .collect();
    let start = glyph_positions(tree, paragraph, line)
        .first()
        .copied()
        .unwrap_or(0);
    let mut running = start;
    let mut out = Vec::with_capacity(widths.len());
    for width in widths {
        out.push(running);
        running += width;
    }
    out
}

/// How far each glyph run was moved from where its predecessors' advances would have put it.
pub(crate) fn shifts(tree: &FragmentTree, paragraph: usize, line: usize) -> Vec<i64> {
    glyph_positions(tree, paragraph, line)
        .into_iter()
        .zip(natural_positions(tree, paragraph, line))
        .map(|(actual, natural)| actual - natural)
        .collect()
}

/// One paragraph, laid out through [`mjx_layout_docx::lay_out`] directly.
///
/// The fragment tree carries *where* every run went and not *why*, so the one thing it cannot show
/// is the identity value: a justified line that widened its gaps by zero and a left-aligned line
/// look the same in a tree. `LinePlacement::expansion_each` is what says which, and this is how a
/// suite reaches it.
pub(crate) fn lay_out_one(properties: &str, text: &str, column_inches: f64) -> ParagraphLayout {
    let mut opened = document(&[paragraph(properties, text)]);
    let formatting = opened.formatting().expect("the document resolves");
    let mut fonts = resolver();
    let mut rasteriser = mjx_text::GlyphRasteriser::new();
    let mut shaper = mjx_text::Shaper::new();
    let features = mjx_text::FeatureSet::default();
    let mut engine = TextEngine {
        fonts: &mut fonts,
        rasteriser: &mut rasteriser,
        shaper: &mut shaper,
        features: &features,
    };
    let column = LayoutRect::from_edges(
        Emu::ZERO,
        Emu::ZERO,
        Emu::from_inches(column_inches),
        Emu::from_inches(11.0),
    );
    lay_out(
        &mut engine,
        formatting.paragraphs().first().expect("one paragraph"),
        FlowContext {
            column,
            settings: formatting.settings(),
            hyphenator: None,
        },
    )
    .expect("the paragraph lays out")
}
