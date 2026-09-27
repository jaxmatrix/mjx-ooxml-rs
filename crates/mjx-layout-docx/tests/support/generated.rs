//! MJXOFF-177 (R22)'s own fixture builders: numbering definitions, fields with **deliberately
//! stale** cached results, tracked changes, and Office Math.
//!
//! A submodule rather than more of `support/mod.rs` because these four subjects are the only ones
//! that need them and because the field builders exist to write markup that is *wrong on purpose* —
//! which is easier to find, and harder to reuse by accident, when it is in a file of its own.

#![allow(dead_code)]

use mjx_docx::{Document, PartName};

use super::{
    document_from_markup, document_markup, escape, paragraph, run_properties, STYLES_PART,
};

/// `word/numbering.xml`.
pub(crate) const NUMBERING_PART: &str = "/word/numbering.xml";

/// One `w:lvl` of an abstract numbering definition.
///
/// `format` is the `w:numFmt` wire token, `text` the `w:lvlText` template (`%1`, `%2`, …), and
/// `extra` whatever else the level states — a `w:lvlRestart`, a `w:isLgl`, a `w:suff`.
pub(crate) fn numbering_level(
    index: i64,
    start: i64,
    format: &str,
    text: &str,
    extra: &str,
) -> String {
    format!(
        concat!(
            r#"<w:lvl w:ilvl="{index}">"#,
            r#"<w:start w:val="{start}"/>"#,
            r#"<w:numFmt w:val="{format}"/>"#,
            "{extra}",
            r#"<w:lvlText w:val="{text}"/>"#,
            r#"<w:lvlJc w:val="left"/>"#,
            "</w:lvl>",
        ),
        index = index,
        start = start,
        format = format,
        extra = extra,
        text = escape(text)
    )
}

/// A whole `word/numbering.xml`: `definitions` are `w:abstractNum` elements and `instances` are
/// `w:num` ones, in that order, which is the order `CT_Numbering`'s own sequence demands.
pub(crate) fn numbering_part(definitions: &[String], instances: &[String]) -> Vec<u8> {
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
            "{definitions}{instances}",
            "</w:numbering>",
        ),
        definitions = definitions.concat(),
        instances = instances.concat()
    )
    .into_bytes()
}

/// One `w:abstractNum` holding `levels`.
pub(crate) fn abstract_numbering(id: i64, levels: &[String]) -> String {
    format!(
        r#"<w:abstractNum w:abstractNumId="{id}">{}</w:abstractNum>"#,
        levels.concat()
    )
}

/// One `w:num` naming `abstract_id`, with `overrides` as its `w:lvlOverride`s.
pub(crate) fn numbering_instance(id: i64, abstract_id: i64, overrides: &str) -> String {
    format!(r#"<w:num w:numId="{id}"><w:abstractNumId w:val="{abstract_id}"/>{overrides}</w:num>"#)
}

/// A `w:lvlOverride` restarting level `index` at `start`.
pub(crate) fn start_override(index: i64, start: i64) -> String {
    format!(r#"<w:lvlOverride w:ilvl="{index}"><w:startOverride w:val="{start}"/></w:lvlOverride>"#)
}

/// A paragraph in list `numbering_id` at `level`, saying `text`.
pub(crate) fn listed_paragraph(numbering_id: i64, level: i64, text: &str) -> String {
    paragraph(
        &format!(
            r#"<w:numPr><w:ilvl w:val="{level}"/><w:numId w:val="{numbering_id}"/></w:numPr>"#
        ),
        text,
    )
}

/// A paragraph whose `w:pStyle` names `style` — which is how a list is inherited rather than stated.
pub(crate) fn styled_paragraph(style: &str, text: &str) -> String {
    paragraph(&format!(r#"<w:pStyle w:val="{style}"/>"#), text)
}

/// A `word/styles.xml` holding one paragraph style whose own `w:pPr` states a `w:numPr`.
///
/// The fixture for **the numbering trap a document built from Word's Heading styles falls into**: a
/// renderer that reads only a paragraph's own `w:numPr` numbers nothing in such a document, and one
/// that reads the resolved reference numbers it correctly.
pub(crate) fn styles_with_numbered_style(style: &str, numbering_id: i64, level: i64) -> Vec<u8> {
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
            r#"<w:style w:type="paragraph" w:styleId="{style}">"#,
            r#"<w:name w:val="{style}"/>"#,
            r#"<w:pPr><w:numPr><w:ilvl w:val="{level}"/><w:numId w:val="{numbering_id}"/></w:numPr></w:pPr>"#,
            "</w:style>",
            "</w:styles>",
        ),
        style = style,
        level = level,
        numbering_id = numbering_id
    )
    .into_bytes()
}

/// A document of `paragraphs` whose `word/numbering.xml` is `numbering`.
///
/// The part is **created** through `Document::edit_numbering` — which is what wires its relationship
/// and its content type — and its bytes are then replaced, exactly as `document_with_notes` does for
/// a notes part.
pub(crate) fn document_with_numbering(paragraphs: &[String], numbering: Vec<u8>) -> Document {
    document_with_numbering_and_styles(paragraphs, numbering, None)
}

/// The same, with `word/styles.xml` replaced too when `styles` is given.
pub(crate) fn document_with_numbering_and_styles(
    paragraphs: &[String],
    numbering: Vec<u8>,
    styles: Option<Vec<u8>>,
) -> Document {
    let markup = document_markup(paragraphs);
    let mut document = document_from_markup(markup, |_, _| ());
    document
        .edit_numbering(|_, _| ())
        .expect("a numbering part is creatable");
    if styles.is_some() {
        document
            .edit_style_sheet(|_, _| ())
            .expect("a styles part is creatable");
    }
    let bytes = document.save_unchecked().expect("the document saves");
    let mut package = mjx_docx::Package::open(&bytes).expect("the package opens");
    package
        .replace_part_bytes(
            &PartName::new(NUMBERING_PART).expect("a valid part name"),
            numbering,
        )
        .expect("the numbering part is replaceable");
    if let Some(styles) = styles {
        package
            .replace_part_bytes(
                &PartName::new(STYLES_PART).expect("a valid part name"),
                styles,
            )
            .expect("the styles part is replaceable");
    }
    Document::from_package(package).expect("the document reopens")
}

/// One complex field — `begin`, its instruction, `separate`, its **cached result**, `end` — as the
/// inner content of a paragraph.
///
/// The cached result is what a renderer that evaluates nothing displays, so a fixture that wants to
/// prove an evaluation happened writes a **wrong** one here. That is the whole method of
/// `a_stale_field_is_recomputed.rs`.
pub(crate) fn complex_field(instruction: &str, cached: &str) -> String {
    format!(
        concat!(
            r#"<w:r><w:fldChar w:fldCharType="begin"/></w:r>"#,
            r#"<w:r><w:instrText xml:space="preserve">{instruction}</w:instrText></w:r>"#,
            r#"<w:r><w:fldChar w:fldCharType="separate"/></w:r>"#,
            r#"<w:r>{properties}<w:t xml:space="preserve">{cached}</w:t></w:r>"#,
            r#"<w:r><w:fldChar w:fldCharType="end"/></w:r>"#,
        ),
        instruction = escape(instruction),
        properties = run_properties(12.0),
        cached = escape(cached)
    )
}

/// A `w:fldSimple` whose instruction is `instruction` and whose cached result is `cached`.
pub(crate) fn simple_field(instruction: &str, cached: &str) -> String {
    format!(
        r#"<w:fldSimple w:instr="{instruction}"><w:r>{properties}<w:t xml:space="preserve">{cached}</w:t></w:r></w:fldSimple>"#,
        instruction = escape(instruction),
        properties = run_properties(12.0),
        cached = escape(cached)
    )
}

/// A paragraph holding `inner` verbatim — how a field or a tracked change reaches a body.
pub(crate) fn raw_paragraph(inner: &str) -> String {
    format!("<w:p>{inner}</w:p>")
}

/// One ordinary run of `text`, at twelve points.
pub(crate) fn plain_run(text: &str) -> String {
    format!(
        r#"<w:r>{}<w:t xml:space="preserve">{}</w:t></w:r>"#,
        run_properties(12.0),
        escape(text)
    )
}

/// A `w:ins` container holding one run of `text`.
pub(crate) fn inserted(id: i64, author: &str, text: &str) -> String {
    format!(
        r#"<w:ins w:id="{id}" w:author="{author}" w:date="2026-09-09T00:00:00Z">{}</w:ins>"#,
        plain_run(text)
    )
}

/// A `w:del` container holding one run of `w:delText`.
pub(crate) fn deleted(id: i64, author: &str, text: &str) -> String {
    format!(
        concat!(
            r#"<w:del w:id="{id}" w:author="{author}" w:date="2026-09-09T00:00:00Z">"#,
            r#"<w:r>{properties}<w:delText xml:space="preserve">{text}</w:delText></w:r>"#,
            "</w:del>",
        ),
        id = id,
        author = author,
        properties = run_properties(12.0),
        text = escape(text)
    )
}

/// The Office Math namespace declaration a fixture's `m:oMath` needs.
pub(crate) const MATH_NAMESPACE: &str =
    r#" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math""#;

/// An `m:r` of `text`.
pub(crate) fn math_run(text: &str) -> String {
    format!("<m:r><m:t>{}</m:t></m:r>", escape(text))
}

/// An `m:{local}` argument holding `content`.
pub(crate) fn math_argument(local: &str, content: &str) -> String {
    format!("<m:{local}>{content}</m:{local}>")
}

/// An `m:f` whose numerator is `numerator` and denominator `denominator`.
pub(crate) fn math_fraction(numerator: &str, denominator: &str) -> String {
    format!(
        "<m:f>{}{}</m:f>",
        math_argument("num", numerator),
        math_argument("den", denominator)
    )
}

/// An `m:d` around `content`, growing unless `grow` says otherwise.
pub(crate) fn math_delimiter(content: &str, grow: bool) -> String {
    format!(
        r#"<m:d><m:dPr><m:grow m:val="{}"/></m:dPr>{}</m:d>"#,
        if grow { "1" } else { "0" },
        math_argument("e", content)
    )
}

/// An `m:sSup`: `base` with `superscript` after it.
pub(crate) fn math_superscript(base: &str, superscript: &str) -> String {
    format!(
        "<m:sSup>{}{}</m:sSup>",
        math_argument("e", base),
        math_argument("sup", superscript)
    )
}

/// An `m:eqArr` of `rows`.
pub(crate) fn math_equation_array(rows: &[String]) -> String {
    let body: String = rows
        .iter()
        .map(|row| math_argument("e", row))
        .collect::<Vec<_>>()
        .concat();
    format!("<m:eqArr>{body}</m:eqArr>")
}

/// A paragraph holding `before` and then one inline `m:oMath` of `content`.
pub(crate) fn math_paragraph(before: &str, content: &str) -> String {
    let head = if before.is_empty() {
        String::new()
    } else {
        plain_run(before)
    };
    format!(r#"<w:p{MATH_NAMESPACE}>{head}<m:oMath>{content}</m:oMath></w:p>"#)
}

/// A paragraph whose single run holds an **inline** drawing `width` by `height` EMU, after `text`.
///
/// The fixture MJXOFF-176's declared gap needs: before MJXOFF-177 the object contributed neither an
/// advance nor a height, so a line carrying one was measured as if it were not there.
pub(crate) fn inline_drawing_paragraph(text: &str, width: i64, height: i64) -> String {
    format!(
        concat!(
            r#"<w:p{namespaces}>"#,
            "{text}",
            r#"<w:r>{properties}<w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0">"#,
            r#"<wp:extent cx="{width}" cy="{height}"/>"#,
            r#"<wp:docPr id="1" name="Picture"/>"#,
            "</wp:inline></w:drawing></w:r>",
            "</w:p>",
        ),
        namespaces = super::DRAWING_NAMESPACES,
        text = if text.is_empty() {
            String::new()
        } else {
            plain_run(text)
        },
        properties = run_properties(12.0),
        width = width,
        height = height
    )
}
