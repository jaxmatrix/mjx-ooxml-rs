//! The WordprocessingML areas — `V-DOCX-01` … `V-DOCX-06`.
//!
//! Every function here is a `mjx_ooxml::Document` caller and names no crate below the facade. See
//! [`super::presentation`] for the shape both halves of every area take.

use anyhow::{Context, Result};
use mjx_ooxml::{
    ChartWrap, Document, HeaderFooterType, HyperlinkTarget, LegendPosition, MergedCellType,
    PageMargins, PageSize, SectionLocation, WrapText, DEFAULT_PLACEHOLDER_IMAGE,
};

/// The content type and extension of [`DEFAULT_PLACEHOLDER_IMAGE`], which `Document` asks for
/// separately because a `.docx` image part is named by its extension in `[Content_Types].xml`.
const PLACEHOLDER_CONTENT_TYPE: &str = "image/png";
/// The extension that goes with [`PLACEHOLDER_CONTENT_TYPE`].
const PLACEHOLDER_EXTENSION: &str = "png";

/// One inch, in EMU — the unit `Document`'s drawing calls take.
const INCH: i64 = 914_400;

/// A blank A4 document. `Document::blank` writes one *empty* paragraph, so the first line of text is
/// always an `append_run` rather than a `set_run_text`.
fn blank() -> Result<Document> {
    Document::blank(PageSize::a4()).context("blank document")
}

fn opened(original: &[u8]) -> Result<Document> {
    Document::open(original).context("opening the Office-authored original")
}

/// The index of the paragraph this area's content starts at: the last one, so an edited original
/// keeps everything it already had and gains ours after it.
fn append_heading(document: &mut Document, text: &str) -> Result<u32> {
    let existing = document.paragraph_count().context("paragraph count")?;
    // A blank document's single paragraph is empty, so it is written into rather than appended to.
    let at = if existing == 1 && document.run_count(0)? == 0 {
        0
    } else {
        document.append_paragraph().context("append paragraph")?;
        existing
    };
    document.append_run(at, text).context("append run")?;
    Ok(at)
}

/// Appends a paragraph with one run and returns its index.
fn append_line(document: &mut Document, text: &str) -> Result<u32> {
    let at = document.paragraph_count().context("paragraph count")?;
    document.append_paragraph().context("append paragraph")?;
    document.append_run(at, text).context("append run")?;
    Ok(at)
}

// -------------------------------------------------------------------------------------------
// V-DOCX-01 — paragraphs, runs and what they inherit
// -------------------------------------------------------------------------------------------

fn write_text_areas(document: &mut Document) -> Result<()> {
    append_heading(document, "Quarterly Review")?;
    append_line(
        document,
        "This paragraph states nothing: every property it renders with is inherited.",
    )?;
    let stated = append_line(document, "North America: +12%")?;
    append_line(document, "EMEA: +8%")?;
    // Read the resolution back, so the harness output shows what this library believes before a
    // reviewer opens the file. It is not a claim about Office.
    let _resolved = document
        .effective_run_properties(stated, 0)
        .context("effective run properties")?;
    Ok(())
}

/// `V-DOCX-01` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_text_and_inheritance() -> Result<Vec<u8>> {
    let mut document = blank()?;
    write_text_areas(&mut document)?;
    Ok(document.save()?)
}

/// `V-DOCX-01` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_text_and_inheritance(original: &[u8]) -> Result<Vec<u8>> {
    let mut document = opened(original)?;
    write_text_areas(&mut document)?;
    Ok(document.save()?)
}

// -------------------------------------------------------------------------------------------
// V-DOCX-02 — sections, headers and footers
// -------------------------------------------------------------------------------------------

fn write_section_areas(document: &mut Document) -> Result<()> {
    append_heading(document, "Sections, headers and footers")?;
    document.set_section_page_size(
        SectionLocation::Body,
        Some(PageSize::us_letter().landscape()),
    )?;
    document.set_section_page_margins(SectionLocation::Body, Some(PageMargins::NORMAL))?;
    document.set_header_text(
        SectionLocation::Body,
        HeaderFooterType::Default,
        "Quarterly Review — Internal",
    )?;
    document.set_footer_text(
        SectionLocation::Body,
        HeaderFooterType::Default,
        "Page footer, default type",
    )?;
    document.set_header_text(
        SectionLocation::Body,
        HeaderFooterType::First,
        "First page header",
    )?;
    Ok(())
}

/// `V-DOCX-02` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_sections_and_headers() -> Result<Vec<u8>> {
    let mut document = blank()?;
    write_section_areas(&mut document)?;
    Ok(document.save()?)
}

/// `V-DOCX-02` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_sections_and_headers(original: &[u8]) -> Result<Vec<u8>> {
    let mut document = opened(original)?;
    write_section_areas(&mut document)?;
    Ok(document.save()?)
}

// -------------------------------------------------------------------------------------------
// V-DOCX-03 — tables
// -------------------------------------------------------------------------------------------

fn write_table_areas(document: &mut Document) -> Result<()> {
    append_heading(document, "Tables")?;
    let table = document.append_table(3, 3).context("append table")?;
    for (row, cells) in [
        ["Region", "Revenue", "Growth"],
        ["North America", "4.2", "+12%"],
        ["EMEA", "3.1", "+8%"],
    ]
    .iter()
    .enumerate()
    {
        for (column, text) in cells.iter().enumerate() {
            document.set_cell_text(table, u32::try_from(row)?, u32::try_from(column)?, text)?;
        }
    }
    // A horizontal span across the last two columns of the last row, and a vertical merge down the
    // first column — the two merge shapes WordprocessingML spells differently.
    document.set_cell_span(table, 2, 1, Some(2))?;
    document.set_cell_vertical_merge(table, 1, 0, Some(MergedCellType::Restart))?;
    document.set_cell_vertical_merge(table, 2, 0, Some(MergedCellType::Continue))?;
    Ok(())
}

/// `V-DOCX-03` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_tables() -> Result<Vec<u8>> {
    let mut document = blank()?;
    write_table_areas(&mut document)?;
    Ok(document.save()?)
}

/// `V-DOCX-03` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_tables(original: &[u8]) -> Result<Vec<u8>> {
    let mut document = opened(original)?;
    write_table_areas(&mut document)?;
    Ok(document.save()?)
}

// -------------------------------------------------------------------------------------------
// V-DOCX-04 — charts
// -------------------------------------------------------------------------------------------

fn write_chart_areas(document: &mut Document) -> Result<()> {
    let anchor = append_heading(document, "Charts")?;
    let chart = document
        .add_chart(
            anchor.into(),
            &super::presentation::quarterly_chart(),
            6 * INCH,
            3 * INCH,
            "Inline chart",
        )
        .context("inline chart")?;
    document.set_chart_title(chart, Some("Revenue by quarter"))?;
    document.set_chart_legend(chart, Some(LegendPosition::Bottom))?;
    document.set_chart_axis_title(chart, 0, Some("Quarter"))?;

    let floating_anchor = append_line(document, "A floating chart follows this paragraph.")?;
    let floating = document
        .add_floating_chart(
            floating_anchor.into(),
            &super::presentation::quarterly_chart(),
            INCH / 2,
            INCH / 2,
            4 * INCH,
            2 * INCH,
            ChartWrap::Square(WrapText::BothSides),
            "Floating chart",
        )
        .context("floating chart")?;
    document.set_chart_title(floating, Some("The same numbers, wrapped square"))?;
    Ok(())
}

/// `V-DOCX-04` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_charts() -> Result<Vec<u8>> {
    let mut document = blank()?;
    write_chart_areas(&mut document)?;
    Ok(document.save()?)
}

/// `V-DOCX-04` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_charts(original: &[u8]) -> Result<Vec<u8>> {
    let mut document = opened(original)?;
    write_chart_areas(&mut document)?;
    Ok(document.save()?)
}

// -------------------------------------------------------------------------------------------
// V-DOCX-05 — pictures
// -------------------------------------------------------------------------------------------

fn write_picture_areas(document: &mut Document) -> Result<()> {
    let anchor = append_heading(document, "Pictures")?;
    document
        .add_inline_picture(
            anchor,
            DEFAULT_PLACEHOLDER_IMAGE.to_vec(),
            PLACEHOLDER_CONTENT_TYPE,
            PLACEHOLDER_EXTENSION,
            2 * INCH,
            2 * INCH,
            "Placeholder",
        )
        .context("inline picture")?;
    let second = append_line(document, "A second picture, half the size:")?;
    document
        .add_inline_picture(
            second,
            DEFAULT_PLACEHOLDER_IMAGE.to_vec(),
            PLACEHOLDER_CONTENT_TYPE,
            PLACEHOLDER_EXTENSION,
            INCH,
            INCH,
            "Placeholder, small",
        )
        .context("second inline picture")?;
    Ok(())
}

/// `V-DOCX-05` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_pictures() -> Result<Vec<u8>> {
    let mut document = blank()?;
    write_picture_areas(&mut document)?;
    Ok(document.save()?)
}

/// `V-DOCX-05` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_pictures(original: &[u8]) -> Result<Vec<u8>> {
    let mut document = opened(original)?;
    write_picture_areas(&mut document)?;
    Ok(document.save()?)
}

// -------------------------------------------------------------------------------------------
// V-DOCX-06 — notes, comments and hyperlinks
// -------------------------------------------------------------------------------------------

fn write_note_areas(document: &mut Document) -> Result<()> {
    let anchor = append_heading(document, "Notes, comments and links")?;
    document.add_footnote(anchor, "Figures are unaudited and subject to revision.")?;
    document.add_endnote(anchor, "Prepared by the validation harness.")?;
    document.add_comment(
        anchor,
        "Reviewer",
        Some("R"),
        "Confirm the North America figure before publishing.",
    )?;

    let link_paragraph = append_line(document, "Full figures: ")?;
    document.insert_hyperlink(
        link_paragraph,
        1,
        "investor relations page",
        &HyperlinkTarget::Url("https://example.com/investors".to_owned()),
    )?;

    Ok(())
}

/// `V-DOCX-06` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_notes_comments_links() -> Result<Vec<u8>> {
    let mut document = blank()?;
    write_note_areas(&mut document)?;
    Ok(document.save()?)
}

/// `V-DOCX-06` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_notes_comments_links(original: &[u8]) -> Result<Vec<u8>> {
    let mut document = opened(original)?;
    write_note_areas(&mut document)?;
    Ok(document.save()?)
}
