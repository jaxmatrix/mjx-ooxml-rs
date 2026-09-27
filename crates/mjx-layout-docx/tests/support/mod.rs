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

/// MJXOFF-177 (R22)'s own builders: numbering definitions, deliberately stale fields, tracked
/// changes and Office Math.
pub(crate) mod generated;

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

/// A `word/document.xml` whose body is `paragraphs`, ended by a section that states **no page
/// geometry at all**.
///
/// # Why the section is empty, and why that is not laziness
///
/// MJXOFF-175 made the document's own `w:sectPr` outrank the caller's `Constraints` — which is the
/// only honest answer once a document can change its page size half way through, because the caller
/// cannot know which section page 200 is in. A fixture that stated US Letter would therefore be laid
/// out on US Letter however small a page the test asked for, and every "how many lines fit"
/// assertion in this crate would silently stop testing what it says.
///
/// So a fixture states its geometry **only when the geometry is the subject**: [`document_with`] is
/// how a section test does that, and this is the one that leaves the page to the caller.
pub(crate) fn document_markup(paragraphs: &[String]) -> Vec<u8> {
    document_markup_with(paragraphs, "")
}

/// The same, with `section` as the body-level `w:sectPr`'s own content.
pub(crate) fn document_markup_with(paragraphs: &[String], section: &str) -> Vec<u8> {
    let body = paragraphs.concat();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<w:body>{body}<w:sectPr>{section}</w:sectPr></w:body>
</w:document>"#
    )
    .into_bytes()
}

/// A `w:pgSz`/`w:pgMar` pair for a page `width` by `height` inches with `margin`-inch margins.
pub(crate) fn page_geometry(width: f64, height: f64, margin: f64) -> String {
    #[allow(clippy::cast_possible_truncation)]
    let twips = |inches: f64| (inches * 1440.0).round() as i64;
    format!(
        r#"<w:pgSz w:w="{}" w:h="{}"/><w:pgMar w:top="{}" w:right="{}" w:bottom="{}" w:left="{}" w:header="720" w:footer="720" w:gutter="0"/>"#,
        twips(width),
        twips(height),
        twips(margin),
        twips(margin),
        twips(margin),
        twips(margin)
    )
}

/// A document whose body is `paragraphs`.
pub(crate) fn document(paragraphs: &[String]) -> Document {
    document_with_settings(paragraphs, |_, _| ())
}

/// A document whose body is `paragraphs` and whose body-level `w:sectPr` holds `section`.
pub(crate) fn document_with(paragraphs: &[String], section: &str) -> Document {
    document_from_markup(document_markup_with(paragraphs, section), |_, _| ())
}

/// A document built from whole `word/document.xml` bytes — how a multi-section fixture states two
/// `w:sectPr`s, which `document_markup_with` cannot because it writes only the body-level one.
pub(crate) fn document_from_bytes(markup: Vec<u8>) -> Document {
    document_from_markup(markup, |_, _| ())
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
    document_from_markup(document_markup(paragraphs), settings)
}

/// A document whose `word/document.xml` is `markup`, with `word/settings.xml` written by `settings`.
pub(crate) fn document_from_markup(
    markup: Vec<u8>,
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
            markup,
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

/// Which page each **block**'s first unit landed on, by walking.
///
/// Indexed by block rather than by paragraph since MJXOFF-176, because that is what a fragment's own
/// address carries and what a `FlowPosition` names. The two coincide for a document with no table in
/// it, which is every fixture written before that child.
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
    let mut answer = vec![None; flow.block_count()];
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
///
/// # The part is printed, and MJXOFF-175 is why
///
/// MJXOFF-174 had one content stream and could leave the part implicit. A document has five, and a
/// header's third paragraph and the body's third paragraph have **the same path** — so a snapshot
/// that printed only the path would show a header's fragments as the body's, and a baseline could
/// not tell a header that had gone missing from one that had been drawn twice.
pub(crate) fn snapshot(tree: &FragmentTree) -> String {
    let mut out = String::new();
    for (id, node) in tree.nodes() {
        let rect = node.rect();
        out.push_str(&format!(
            "{:>4} {:<9} part {} {:?} [{} {} {} {}]\n",
            id.index(),
            node.fragment().kind_name(),
            node.source().part().number(),
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
        FlowContext::plain(column, formatting.settings(), None),
    )
    .expect("the paragraph lays out")
}

/// Where the first line of `document`'s body sits, in EMU from the top of the page.
///
/// The shape a vertical-alignment assertion is written in: `w:vAlign` moves content that is already
/// laid out, so the thing that changes is a *position* and not a page assignment.
pub(crate) fn first_line_top(mut document: Document, constraints: &Constraints) -> Emu {
    let flow = flow(&mut document);
    let mut model = model();
    let page = model
        .layout_page(&flow, PageIndex::FIRST, constraints, None)
        .expect("the page lays out");
    let top = page
        .fragments()
        .nodes()
        .find(|(_, node)| matches!(node.fragment(), Fragment::Line(_)))
        .map_or(Emu::ZERO, |(_, node)| node.rect().top);
    top
}

// -------------------------------------------------------------------------------------------
// Footnotes and endnotes.
//
// One builder for both, because `wml.xsd` gives `w:footnote` and `w:endnote` the identical
// `CT_FtnEdn` type and the two parts the identical shape — the discriminant is the *part*, not the
// content model, which is the same reason `mjx-docx` keeps `Footnotes` and `Endnotes` apart as
// types while sharing `FootnoteEndnote`. Three suites need these, and three copies of one XML
// template is how two of them quietly stop agreeing about what a separator is.
// -------------------------------------------------------------------------------------------

/// `word/footnotes.xml`.
pub(crate) const FOOTNOTES_PART: &str = "/word/footnotes.xml";

/// `word/endnotes.xml`.
pub(crate) const ENDNOTES_PART: &str = "/word/endnotes.xml";

/// One `w:footnote` or `w:endnote` of `lines` paragraphs, at eleven point in [`FAMILY`].
///
/// `kind` is `w:type` — `None` for a user's own note, which is what an absent attribute means
/// (§17.11.10), and `Some("separator")`/`Some("continuationSeparator")` for Word's own furniture.
pub(crate) fn note_entry(local: &str, id: i64, kind: Option<&str>, lines: &[&str]) -> String {
    let attributes = match kind {
        Some(kind) => format!(r#" w:type="{kind}""#),
        None => String::new(),
    };
    let body: String = lines
        .iter()
        .map(|line| {
            format!(
                r#"<w:p><w:r>{}<w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
                run_properties(11.0),
                escape(line)
            )
        })
        .collect();
    format!(r#"<w:{local}{attributes} w:id="{id}">{body}</w:{local}>"#)
}

/// A whole notes part: Word's two rules, then `notes`.
///
/// The reserved entries are **always** written, because a page's separator is drawn from them and a
/// fixture without one would make the separator untestable while looking complete.
pub(crate) fn notes_part(footnotes: bool, notes: &[String]) -> Vec<u8> {
    let (root, local) = if footnotes {
        ("footnotes", "footnote")
    } else {
        ("endnotes", "endnote")
    };
    let body = notes.concat();
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<w:{root} xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
            "{separator}{continuation}{body}",
            "</w:{root}>",
        ),
        root = root,
        separator = note_entry(local, -1, Some("separator"), &["_"]),
        continuation = note_entry(local, 0, Some("continuationSeparator"), &["_"]),
        body = body
    )
    .into_bytes()
}

/// A document of `paragraphs` whose notes part holds `notes`.
///
/// The part is **created** through `Document::edit_footnotes`/`edit_endnotes` — which is what wires
/// its relationship and its content type — and its content is then replaced wholesale, so that the
/// separator's own height is a number the fixture chose rather than one `mjx-docx`'s seed decided.
pub(crate) fn document_with_notes(
    paragraphs: &[String],
    section: &str,
    footnotes: bool,
    notes: &[String],
) -> Document {
    let markup = document_markup_with(paragraphs, section);
    let mut document = document_from_markup(markup, |_, _| ());
    if footnotes {
        document
            .edit_footnotes(|_, _| ())
            .expect("a footnotes part is creatable");
    } else {
        document
            .edit_endnotes(|_, _| ())
            .expect("an endnotes part is creatable");
    }
    let bytes = document.save_unchecked().expect("the document saves");
    let mut package = mjx_docx::Package::open(&bytes).expect("the package opens");
    let name = if footnotes {
        FOOTNOTES_PART
    } else {
        ENDNOTES_PART
    };
    package
        .replace_part_bytes(
            &PartName::new(name).expect("a valid part name"),
            notes_part(footnotes, notes),
        )
        .expect("the notes part is replaceable");
    Document::from_package(package).expect("the document reopens")
}

/// The same with exactly one user footnote, of `lines`.
pub(crate) fn document_with_footnote(
    paragraphs: &[String],
    section: &str,
    id: i64,
    lines: &[&str],
) -> Document {
    document_with_notes(
        paragraphs,
        section,
        true,
        &[note_entry("footnote", id, None, lines)],
    )
}

/// A paragraph whose single run carries a footnote (or endnote) reference after its text.
pub(crate) fn referencing_note(footnotes: bool, id: i64, text: &str) -> String {
    let local = if footnotes {
        "footnoteReference"
    } else {
        "endnoteReference"
    };
    paragraph_with_run(
        "",
        &format!(
            r#"<w:t xml:space="preserve">{}</w:t><w:{local} w:id="{id}"/>"#,
            escape(text)
        ),
    )
}

// -------------------------------------------------------------------------------------------
// MJXOFF-176 (R21): tables and floating objects.
// -------------------------------------------------------------------------------------------

/// One `w:tc`: `properties` is the body of its `w:tcPr` (empty for none) and `content` its blocks.
pub(crate) fn cell(properties: &str, content: &str) -> String {
    let tcpr = if properties.is_empty() {
        String::new()
    } else {
        format!("<w:tcPr>{properties}</w:tcPr>")
    };
    format!("<w:tc>{tcpr}{content}</w:tc>")
}

/// A cell holding one paragraph of `text`.
pub(crate) fn text_cell(properties: &str, text: &str) -> String {
    cell(properties, &paragraph("", text))
}

/// One `w:tr`: `properties` is the body of its `w:trPr` and `cells` its cells.
pub(crate) fn row(properties: &str, cells: &[String]) -> String {
    let trpr = if properties.is_empty() {
        String::new()
    } else {
        format!("<w:trPr>{properties}</w:trPr>")
    };
    format!("<w:tr>{trpr}{}</w:tr>", cells.concat())
}

/// One `w:tbl`: `properties` is the body of its `w:tblPr`, `grid` the column widths in twips.
pub(crate) fn table(properties: &str, grid: &[i64], rows: &[String]) -> String {
    let columns: String = grid
        .iter()
        .map(|width| format!(r#"<w:gridCol w:w="{width}"/>"#))
        .collect();
    format!(
        "<w:tbl><w:tblPr>{properties}</w:tblPr><w:tblGrid>{columns}</w:tblGrid>{}</w:tbl>",
        rows.concat()
    )
}

/// The namespace declarations a `w:drawing` needs when it is written into a document that declares
/// only `w:` and `r:` — which is what [`document_markup`] writes.
pub(crate) const DRAWING_NAMESPACES: &str = concat!(
    r#" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing""#,
    r#" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#,
);

/// A paragraph whose run carries one anchored `w:drawing`, followed by `text`.
///
/// `width` and `height` are in EMU; `position` is the body of `wp:positionH` and `wp:positionV`
/// together; `wrap` is the whole `wp:wrap*` element.
pub(crate) fn floating_paragraph(
    properties: &str,
    text: &str,
    width: i64,
    height: i64,
    position: &str,
    wrap: &str,
) -> String {
    let ppr = if properties.is_empty() {
        String::new()
    } else {
        format!("<w:pPr>{properties}</w:pPr>")
    };
    format!(
        r#"<w:p>{ppr}<w:r>{rpr}<w:drawing{ns}><wp:anchor distT="0" distB="0" distL="0" distR="0" simplePos="0" relativeHeight="1" behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/>{position}<wp:extent cx="{width}" cy="{height}"/><wp:effectExtent l="0" t="0" r="0" b="0"/>{wrap}<wp:docPr id="1" name="Object"/><a:graphic><a:graphicData uri="urn:test"/></a:graphic></wp:anchor></w:drawing></w:r><w:r>{rpr}<w:t xml:space="preserve">{escaped}</w:t></w:r></w:p>"#,
        rpr = run_properties(11.0),
        ns = DRAWING_NAMESPACES,
        escaped = escape(text),
    )
}

/// A `wp:positionH`/`wp:positionV` pair placing an object at `(x, y)` EMU from the column and the
/// paragraph respectively — the anchoring a picture dropped into a paragraph gets.
pub(crate) fn offset_position(x: i64, y: i64) -> String {
    format!(
        r#"<wp:positionH relativeFrom="column"><wp:posOffset>{x}</wp:posOffset></wp:positionH><wp:positionV relativeFrom="paragraph"><wp:posOffset>{y}</wp:posOffset></wp:positionV>"#
    )
}

/// A `wp:wrapSquare` of `side`.
pub(crate) fn square_wrap(side: &str) -> String {
    format!(r#"<wp:wrapSquare wrapText="{side}" distT="0" distB="0" distL="0" distR="0"/>"#)
}

/// A `wp:wrapTight` (or `wp:wrapThrough`, when `through`) around `polygon`, whose points are in the
/// `0..21600` space Word writes.
pub(crate) fn polygon_wrap(side: &str, through: bool, polygon: &[(i64, i64)]) -> String {
    let local = if through { "wrapThrough" } else { "wrapTight" };
    let mut points = String::new();
    for (index, (x, y)) in polygon.iter().enumerate() {
        if index == 0 {
            points.push_str(&format!(r#"<wp:start x="{x}" y="{y}"/>"#));
        } else {
            points.push_str(&format!(r#"<wp:lineTo x="{x}" y="{y}"/>"#));
        }
    }
    format!(
        r#"<wp:{local} wrapText="{side}" distL="0" distR="0"><wp:wrapPolygon edited="0">{points}</wp:wrapPolygon></wp:{local}>"#
    )
}

/// The one paragraph of `document`, laid out at `column_inches` against `exclusions`.
pub(crate) fn lay_out_one_around(
    properties: &str,
    text: &str,
    column_inches: f64,
    exclusions: &[mjx_layout_docx::Exclusion],
) -> ParagraphLayout {
    let mut document = document(&[paragraph(properties, text)]);
    let formatting = document.formatting().expect("the document resolves");
    let mut resolver = resolver();
    let mut rasteriser = mjx_text::GlyphRasteriser::new();
    let mut shaper = mjx_text::Shaper::new();
    let features = mjx_text::FeatureSet::default();
    let mut engine = TextEngine {
        fonts: &mut resolver,
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
            top: Emu::ZERO,
            exclusions,
        },
    )
    .expect("the paragraph lays out")
}

/// Every `mjx_layout::TableCell` a page's fragments carry, as `(row, column, span)`, in tree order.
pub(crate) fn cells_on(tree: &FragmentTree) -> Vec<(u32, u16, u16)> {
    tree.nodes()
        .filter_map(|(_, node)| match node.fragment() {
            Fragment::Box(fragment) => fragment
                .cell
                .as_ref()
                .map(|cell| (cell.row, cell.column, cell.column_span)),
            _ => None,
        })
        .collect()
}

/// Which table rows a page holds, in ascending order and without repeats.
pub(crate) fn rows_on(tree: &FragmentTree) -> Vec<u32> {
    let mut rows: Vec<u32> = cells_on(tree).into_iter().map(|cell| cell.0).collect();
    rows.sort_unstable();
    rows.dedup();
    rows
}

/// The exclusions the floats of `markup`'s first paragraph contribute, in a column `width_inches`
/// wide.
///
/// **The whole path a document actually takes**: the markup is parsed, the residency resolves the
/// `wp:anchor` to plain numbers, and [`mjx_layout_docx::place_float`] turns those into geometry. A
/// suite that built an `Exclusion` by hand would assert the geometry and prove nothing about the
/// file.
pub(crate) fn float_exclusions(
    markup: String,
    width_inches: f64,
) -> Vec<mjx_layout_docx::Exclusion> {
    let mut document = document(&[markup]);
    let formatting = document.formatting().expect("the document resolves");
    let paragraph = formatting.paragraphs().first().expect("one paragraph");
    let frame = mjx_layout_docx::Anchorage::contained(
        Emu::from_inches(width_inches),
        Emu::from_inches(11.0),
        Emu::ZERO,
    );
    paragraph
        .drawings()
        .iter()
        .enumerate()
        .filter_map(|(index, drawing)| mjx_layout_docx::place_float(drawing, 0, index, frame))
        .filter_map(|placed| placed.exclusion)
        .collect()
}
