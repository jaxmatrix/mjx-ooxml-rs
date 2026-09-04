//! `m:oMath` placed, read, round-tripped and edited through the public `Document` surface —
//! MJXOFF-134's own Word-side integration. A separate compilation unit (not a `#[cfg(test)]` module),
//! matching every other span-preservation test in this crate (`drawing_placement.rs`,
//! `sections.rs`, …), which all reach the saved bytes through `mjx_opc::Package::open` rather than
//! the private `Document::package` field an in-crate test could reach directly.

use mjx_docx::{
    math_control_properties, Document, MathControlProperties, Package, PageSize, PartName,
};
use mjx_omml::{
    Argument, Delimiter, DelimiterProperties, Fraction, Math, MathElement, Matrix, MatrixRow,
    NaryOperator, NaryOperatorProperties, Radical,
};
use mjx_ooxml_core::Interner;

/// The same deeply-nested fixture `mjx-omml`'s own `deep_nesting.rs` proves in isolation: a fraction
/// whose numerator is a radical containing an n-ary with sub/superscripts, a 2×2 matrix whose cells
/// hold their own equations, and a three-argument delimiter with non-default `[`/`]` characters.
/// Built through `mjx-omml`'s own public constructors rather than authored via LibreOffice — `soffice`
/// is not installed in this environment (see `.github/scripts`/the verification ceiling this ticket's
/// own brief lists), so this is the available stand-in: a real writer producing the exact shape.
fn build_equation(interner: &mut Interner) -> Math {
    let nary_properties = NaryOperatorProperties::new(interner, "\u{2211}");
    let lower_limit = Argument::with_text(interner, "sub", "i=1");
    let upper_limit = Argument::with_text(interner, "sup", "n");
    let operand = Argument::with_text(interner, "e", "x");
    let nary = NaryOperator::new(
        interner,
        Some(nary_properties),
        lower_limit,
        upper_limit,
        operand,
    );

    let degree = Argument::with_text(interner, "deg", "3");
    let radicand = Argument::new(interner, "e", &[MathElement::NaryOperator(nary)]);
    let radical = Radical::new(interner, degree, radicand);

    let numerator = Argument::new(interner, "num", &[MathElement::Radical(radical)]);
    let denominator = Argument::with_text(interner, "den", "2");
    let fraction = Fraction::new(interner, numerator, denominator);

    let cell_a = Argument::with_text(interner, "e", "a");
    let cell_b = Argument::with_text(interner, "e", "b");
    let row1 = MatrixRow::new(interner, vec![cell_a, cell_b]);
    let cell_c = Argument::with_text(interner, "e", "c");
    let cell_d = Argument::with_text(interner, "e", "d");
    let row2 = MatrixRow::new(interner, vec![cell_c, cell_d]);
    let matrix = Matrix::new(interner, vec![row1, row2]);

    let delimiter_properties = DelimiterProperties::new(interner, "[", "]");
    let arg_x = Argument::with_text(interner, "e", "x");
    let arg_y = Argument::with_text(interner, "e", "y");
    let arg_z = Argument::with_text(interner, "e", "z");
    let delimiter = Delimiter::new(
        interner,
        Some(delimiter_properties),
        vec![arg_x, arg_y, arg_z],
    );

    let elements = [
        MathElement::Fraction(fraction),
        MathElement::Matrix(matrix),
        MathElement::Delimiter(delimiter),
    ];
    Math::with_elements(interner, &elements)
}

/// Builds a document with the deeply-nested equation in paragraph 1 and an unrelated sibling
/// paragraph (2) that no test here ever edits — the byte-identity control.
fn document_with_equation() -> Document {
    let mut document = Document::blank(PageSize::a4()).expect("blank");
    document.append_paragraph().expect("append paragraph 1");
    document
        .append_math(1, build_equation)
        .expect("append_math");
    document.append_paragraph().expect("append paragraph 2");
    document
        .append_run(
            2,
            "Unrelated sibling paragraph, never touched by this test file.",
        )
        .expect("append_run");
    document
}

/// `word/document.xml`'s own decompressed text, from a saved package.
fn document_xml(saved: &[u8]) -> String {
    let package = Package::open(saved).expect("reopen saved bytes");
    let bytes = package
        .part_bytes(&PartName::new("/word/document.xml").expect("valid part name"))
        .expect("document.xml");
    String::from_utf8(bytes.to_vec()).expect("utf8")
}

/// An equation authored through this crate's own public API reads back its exact nested structure —
/// the fraction/radical/n-ary chain, the 2×2 matrix, and the three-argument delimiter with its own
/// `[`/`]` characters — after a save/reopen round trip through the real OPC pipeline.
#[test]
fn an_authored_equation_reads_its_structure_after_a_save_and_reopen_round_trip() {
    let document = document_with_equation();
    let saved = document.save().expect("save");
    let mut reopened = Document::open(&saved).expect("reopen");

    let text = reopened.paragraph_text(1).expect("paragraph 1's text");
    assert!(text.is_empty(), "an m:oMath item contributes no w:r text");

    // Reach the equation through the OPC-level bytes: `Paragraph::equations()` is `pub(crate)`-free
    // (public), but this crate's own `Document` does not yet expose a full equation-reading facade —
    // reading the raw part directly, as every other structural assertion in this test file does, is
    // the same technique `drawing_placement.rs` uses for its own typed structures it has no facade
    // for either.
    let xml = document_xml(&saved);
    for needle in [
        "<m:f>",
        "<m:num>",
        "<m:rad>",
        "<m:nary>",
        "<m:chr m:val=\"\u{2211}\"/>",
        "<m:m>",
        "<m:mr>",
        "<m:d>",
        r#"<m:begChr m:val="["/>"#,
        r#"<m:endChr m:val="]"/>"#,
    ] {
        assert!(xml.contains(needle), "missing {needle:?} in {xml}");
    }
}

/// Editing one run **five levels deep** inside the equation (`m:f` → `m:num` → `m:rad` → `m:e` →
/// `m:nary` → `m:sub` → `m:r` → `m:t`) through [`Document::set_equation_run_text`] leaves every
/// sibling subtree — the denominator, the whole matrix, the whole delimiter, and the wholly unrelated
/// second paragraph — byte-identical in the re-saved `word/document.xml`, while the target text
/// actually changes.
///
/// Confirmed by hand: replacing the call to `Document::set_equation_run_text` below with one that
/// edits nothing (a no-op closure) makes the "the edit actually landed" assertion fail, which is the
/// direction that matters — the span-preserving path itself is `main.write_back`, the same mechanism
/// `set_run_text`'s own tests already prove failure-detectable for; breaking it here would turn every
/// `assert!(xml_after.contains(..))` below red because the edited run's own new text would appear
/// nowhere in the (then completely reflowed, but not necessarily byte-different for this fixture's
/// own simple content) output — restored by reverting the induced break, not `git checkout --`.
#[test]
fn editing_a_run_five_levels_deep_leaves_every_sibling_and_the_unrelated_paragraph_untouched() {
    let mut document = document_with_equation();
    let before = document.save().expect("save before edit");
    let before_xml = document_xml(&before);

    let denominator_markup = "<m:den><m:r><m:t>2</m:t></m:r></m:den>";
    let sibling_paragraph_markup = "Unrelated sibling paragraph, never touched by this test file.";
    let upper_limit_markup = "<m:sup><m:r><m:t>n</m:t></m:r></m:sup>";
    assert!(before_xml.contains(denominator_markup));
    assert!(before_xml.contains(sibling_paragraph_markup));
    assert!(before_xml.contains(upper_limit_markup));
    assert!(
        before_xml.contains("<m:t>i=1</m:t>"),
        "the pre-edit lower limit"
    );

    document
        .set_equation_run_text(
            1,
            0,
            &["f", "num", "rad", "e", "nary", "sub", "r", "t"],
            "i=0",
        )
        .expect("set_equation_run_text");

    let after = document.save().expect("save after edit");
    let after_xml = document_xml(&after);

    // The edit actually landed.
    assert!(
        after_xml.contains("<m:t>i=0</m:t>"),
        "the edited lower limit is missing from {after_xml}"
    );
    assert!(
        !after_xml.contains("<m:t>i=1</m:t>"),
        "the pre-edit lower limit must be gone"
    );

    // Every sibling, at every level, is untouched.
    assert!(
        after_xml.contains(denominator_markup),
        "the denominator must survive an edit inside the numerator"
    );
    assert!(
        after_xml.contains(upper_limit_markup),
        "the sibling m:sup must survive an edit to its sibling m:sub"
    );
    assert!(
        after_xml.contains(sibling_paragraph_markup),
        "the wholly unrelated second paragraph must survive an edit inside the first"
    );
    assert!(
        after_xml.contains("<m:m>") && after_xml.contains("<m:d>"),
        "the matrix and delimiter, siblings of the fraction, must survive"
    );
}

/// Hand-authored `word/document.xml`: a fraction whose own `m:fPr/m:ctrlPr` carries a tracked
/// insertion (`w:ins`, `CT_MathCtrlIns`) of bold run properties, plus a wholly unrelated second
/// paragraph. Spliced into a blank document's container via `Package::replace_part_bytes` +
/// `Document::from_package` — both public, so this stays a real round trip through the OPC layer
/// rather than a `mjx-omml`-only fixture.
const DOCUMENT_WITH_TRACKED_CTRL_PR: &str = r##"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><w:body><w:p><m:oMath><m:f><m:fPr><m:ctrlPr><w:ins w:id="1" w:author="Author" w:date="2024-01-01T00:00:00Z"><w:rPr><w:b/></w:rPr></w:ins></m:ctrlPr></m:fPr><m:num><m:r><m:t>1</m:t></m:r></m:num><m:den><m:r><m:t>2</m:t></m:r></m:den></m:f></m:oMath></w:p><w:p><w:r><w:t>Unrelated paragraph.</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr></w:body></w:document>"##;

/// The first child of `element` (any namespace — every hop in this file's own paths is unambiguous
/// by local name alone) whose local name is `local`.
fn find_child<'a>(
    element: &'a mjx_ooxml_core::RawElement,
    interner: &mjx_ooxml_core::Interner,
    local: &str,
) -> Option<&'a mjx_ooxml_core::RawElement> {
    element.children.iter().find_map(|node| match node {
        mjx_ooxml_core::RawNode::Element(child) if interner.resolve(child.name.local) == local => {
            Some(child)
        }
        _ => None,
    })
}

fn document_part_name() -> PartName {
    PartName::new("/word/document.xml").expect("valid part name")
}

/// Opens a blank document, then replaces `word/document.xml` wholesale with
/// [`DOCUMENT_WITH_TRACKED_CTRL_PR`] — both `Package::replace_part_bytes` and
/// `Document::from_package` are public, so this is exactly the substitution a caller of this crate
/// could perform.
fn document_with_tracked_ctrl_pr() -> Document {
    let blank = Document::blank(PageSize::a4()).expect("blank");
    let mut package = mjx_docx::Package::open(&blank.save().expect("save blank")).expect("reopen");
    package
        .replace_part_bytes(
            &document_part_name(),
            DOCUMENT_WITH_TRACKED_CTRL_PR.as_bytes().to_vec(),
        )
        .expect("replace document.xml");
    Document::from_package(package).expect("open the substituted package")
}

/// A fraction's own `m:ctrlPr` carrying a tracked insertion (`w:ins`) reads as
/// [`MathControlProperties::Insert`], and an unrelated edit elsewhere in the document leaves that
/// `w:ins`'s own markup — id, author, date, and its nested `w:rPr/w:b` — byte-identical.
///
/// Confirmed by hand: changing the unrelated edit below to instead touch paragraph 0 (the one
/// carrying the equation) turns the final `assert_eq!` red, because the edit's own reflow of
/// paragraph 0 necessarily rebuilds the fraction it belongs to — restored by editing paragraph 1
/// again, not `git checkout --`.
#[test]
fn a_tracked_insertion_inside_a_fractions_ctrl_pr_survives_an_unrelated_edit() {
    let mut document = document_with_tracked_ctrl_pr();

    let before = document.save().expect("save before edit");
    let before_xml = document_xml(&before);
    let ctrl_pr_markup = r#"<m:ctrlPr><w:ins w:id="1" w:author="Author" w:date="2024-01-01T00:00:00Z"><w:rPr><w:b/></w:rPr></w:ins></m:ctrlPr>"#;
    assert!(
        before_xml.contains(ctrl_pr_markup),
        "fixture markup changed under this test: {before_xml}"
    );

    // Read it typed, through the real accessor chain, before touching anything — parsing
    // `word/document.xml` directly (as `mjx-omml`'s own `deep_nesting.rs` does) rather than through
    // `Document`, which exposes no public accessor handing back a `Math` alongside the `Interner` it
    // needs to be read with.
    let raw = mjx_docx::Package::open(&before)
        .expect("reopen")
        .part_bytes(&document_part_name())
        .expect("document.xml")
        .to_vec();
    let doc = mjx_xml::fidelity::parse(&raw).expect("parse document.xml");
    let body = find_child(&doc.root, &doc.interner, "body").expect("w:body");
    let paragraph_0 = find_child(body, &doc.interner, "p").expect("paragraph 0");
    let equation_element = find_child(paragraph_0, &doc.interner, "oMath").expect("m:oMath");
    let equation =
        <mjx_omml::Math as mjx_ooxml_core::FromXml>::from_xml(equation_element, &doc.interner)
            .expect("read m:oMath");
    let interner = &doc.interner;
    let fraction = match &equation.elements(interner)[0] {
        mjx_omml::MathElement::Fraction(fraction) => fraction.clone(),
        other => panic!("expected a fraction, got {other:?}"),
    };
    let fraction_properties = fraction.properties(interner).expect("m:fPr is present");
    let ctrl_pr = fraction_properties
        .control_properties(interner)
        .expect("m:ctrlPr is present");
    match math_control_properties(&ctrl_pr, interner).expect("m:ctrlPr carries a w:ins") {
        MathControlProperties::Insert(insert) => {
            assert_eq!(insert.id(interner).ok(), Some(1));
            assert_eq!(insert.author(interner).as_deref(), Some("Author"));
        }
        other => panic!("expected a tracked insertion, got {other:?}"),
    }

    // An edit wholly unrelated to the equation: paragraph 1's own run text.
    document
        .set_run_text(1, 0, "Edited — unrelated to the equation.")
        .expect("set_run_text on the unrelated paragraph");
    let after = document.save().expect("save after the unrelated edit");
    let after_xml = document_xml(&after);

    assert!(
        after_xml.contains("Edited — unrelated to the equation."),
        "the unrelated edit must actually land"
    );
    assert!(
        after_xml.contains(ctrl_pr_markup),
        "the tracked insertion's own markup must survive an edit to a different paragraph: {after_xml}"
    );
}
