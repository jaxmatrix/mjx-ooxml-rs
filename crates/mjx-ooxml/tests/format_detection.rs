//! `detect_format` reads the package, not the filename.
//!
//! Every case here is proved on **bytes that have no name at all**: the fixtures are read into
//! `Vec<u8>` and the `.pptm` and `.potx` cases are authored by rewriting one content-type override on
//! a real `.pptx` package. If detection were reading an extension it could not answer any of them,
//! and if it were guessing from the markup it would call all three of `.pptx`, `.pptm` and `.potx`
//! the same thing — which is the failure this test is here to catch.
//!
//! This is the one test file in the crate that names a lower crate, and only to *build* the input:
//! the repository holds no `.pptm` or `.potx` fixture, and detection is not worth proving against a
//! fixture that was hand-crafted to pass.

use mjx_ooxml::{detect_format, Deck, ErrorCode, Format, FormatFamily};
use mjx_opc::{Package, PartName};

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

/// The same PresentationML package, re-declared under a different main-part content type — which is
/// the entire difference between a `.pptx`, a `.pptm` and a `.potx` on disk.
fn presentation_declared_as(content_type: &str) -> Vec<u8> {
    let mut package = Package::open(&fixture("sample.pptx")).expect("the sample package");
    let main = PartName::new("/ppt/presentation.xml").expect("the main part name");
    package
        .set_content_type_override(&main, content_type)
        .expect("re-declaring the main part");
    package.save_unchecked().expect("re-saving the package")
}

#[test]
fn a_presentation_is_detected() {
    assert_eq!(
        detect_format(&fixture("sample.pptx")).expect("a format"),
        Format::Presentation
    );
}

/// The two cases a filename check cannot reach: identical markup, different declaration.
#[test]
fn macro_enabled_and_template_presentations_are_told_apart() {
    let pptm = presentation_declared_as(
        "application/vnd.ms-powerpoint.presentation.macroEnabled.main+xml",
    );
    let potx = presentation_declared_as(
        "application/vnd.openxmlformats-officedocument.presentationml.template.main+xml",
    );

    assert_eq!(
        detect_format(&pptm).expect("a format"),
        Format::PresentationMacroEnabled
    );
    assert_eq!(
        detect_format(&potx).expect("a format"),
        Format::PresentationTemplate
    );

    // The three answers differ, though the shape trees are byte-identical.
    assert_ne!(
        detect_format(&pptm).expect("a format"),
        detect_format(&potx).expect("a format")
    );
    assert_eq!(
        detect_format(&fixture("sample.pptx")).expect("a format"),
        Format::Presentation
    );

    // And each carries the convention a caller would name the file by.
    assert_eq!(
        Format::PresentationMacroEnabled.conventional_extension(),
        "pptm"
    );
    assert_eq!(
        Format::PresentationTemplate.conventional_extension(),
        "potx"
    );

    // All three are PresentationML, so all three open.
    for bytes in [&pptm, &potx, &fixture("sample.pptx")] {
        assert!(detect_format(bytes).expect("a format").is_editable());
        Deck::open(bytes).expect("a PresentationML package opens whatever it is declared as");
    }
}

#[test]
fn a_word_document_is_detected_and_refused() {
    let bytes = fixture("sample.docx");
    let format = detect_format(&bytes).expect("a format");
    assert_eq!(format, Format::Document);
    assert_eq!(format.family(), FormatFamily::WordProcessing);
    assert!(!format.is_editable());

    let refused = Deck::open(&bytes).expect_err("a Word document is not a deck");
    assert_eq!(refused.code(), ErrorCode::UnsupportedFormat);
    // The message must name the format, not report a parse failure.
    assert!(
        refused.to_string().contains("Document"),
        "unhelpful refusal: {refused}"
    );
}

#[test]
fn an_excel_workbook_is_detected_and_refused() {
    let bytes = fixture("sample.xlsx");
    let format = detect_format(&bytes).expect("a format");
    assert_eq!(format, Format::Workbook);
    assert_eq!(format.family(), FormatFamily::Spreadsheet);
    assert!(!format.is_editable());
    assert_eq!(format.conventional_extension(), "xlsx");

    assert_eq!(
        Deck::open(&bytes)
            .expect_err("an Excel workbook is not a deck")
            .code(),
        ErrorCode::UnsupportedFormat
    );
}

/// A deck remembers what it was opened as, and saving does not silently rewrite it.
#[test]
fn a_template_stays_a_template_across_a_round_trip() {
    let potx = presentation_declared_as(
        "application/vnd.openxmlformats-officedocument.presentationml.template.main+xml",
    );
    let mut deck = Deck::open(&potx).expect("a template opens");
    assert_eq!(deck.format(), Format::PresentationTemplate);

    deck.add_slide().expect("a slide");
    let saved = deck.save().expect("saving");
    assert_eq!(
        detect_format(&saved).expect("a format"),
        Format::PresentationTemplate,
        "saving must not re-declare the main part"
    );
}

#[test]
fn bytes_that_are_not_a_package_are_an_io_failure() {
    let error = detect_format(b"not a zip at all").expect_err("no format");
    assert_eq!(error.code(), ErrorCode::Io);
    assert!(error.detail().is_empty());
}

/// A blank deck built from nothing declares itself a presentation, like any other.
#[test]
fn a_blank_deck_saves_as_a_presentation() {
    let deck = Deck::blank(mjx_ooxml::SlideSize::widescreen()).expect("a blank deck");
    assert_eq!(deck.format(), Format::Presentation);
    let saved = deck.save().expect("saving");
    assert_eq!(
        detect_format(&saved).expect("a format"),
        Format::Presentation
    );
}

/// `Deck::save` inherits `Presentation::save`'s validation rather than routing around it.
///
/// A facade that widened what a caller could break would be a regression, so this is proved on a
/// deck that is genuinely invalid: a package carrying a root relationship whose target names a part
/// it does not hold — `PackageDefect::RelationshipTargetMissing`, which A7b made `save` refuse.
///
/// The two halves matter equally. `save` must fail, or the check was skipped; `save_unchecked` must
/// succeed on the same deck, or the failure proves nothing about *which* call `save` makes.
#[test]
fn save_refuses_an_invalid_deck_and_save_unchecked_does_not() {
    use mjx_opc::{Relationship, TargetMode};

    let mut package = Package::open(&fixture("sample.pptx")).expect("the sample package");
    package
        .add_relationship(
            None,
            Relationship {
                id: "rIdDangling".to_owned(),
                rel_type:
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties"
                        .to_owned(),
                target: "docProps/nothing-is-here.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("adding a dangling relationship");
    let broken = package
        .save_unchecked()
        .expect("writing the broken package");

    // It still opens: the fault is in the packaging graph, not in the PresentationML.
    let deck = Deck::open(&broken).expect("a broken package still opens");

    let refused = deck.save().expect_err("save must refuse an invalid deck");
    assert_eq!(refused.code(), ErrorCode::InvalidDocument);
    assert_eq!(
        deck.validate()
            .expect_err("validate must report the same defect")
            .code(),
        ErrorCode::InvalidDocument
    );

    // And the deliberate override still writes it — so the refusal above was the check, not an
    // unrelated failure in the writer.
    deck.save_unchecked()
        .expect("save_unchecked is the deliberate override");
}
