//! **No layout path panics**, held two ways: by scanning the source for the constructs that would,
//! and by laying out documents that say things no well-formed file says.
//!
//! A document comes from an untrusted file, and `CLAUDE.md`'s rule is that library code returns
//! typed errors rather than unwrapping. The scan is the strong half — it catches a panic on a path
//! no fixture happens to reach — and the adversarial documents are the half that proves the scan is
//! measuring something real.

mod support;

use mjx_layout::{BoxModel, PageIndex};
use support::{constraints, document, flow, model, paragraph, paragraph_with_run};

/// The constructs that panic, as they appear in code rather than in prose.
const PANICKING: &[&str] = &[
    ".unwrap()",
    ".expect(",
    "panic!(",
    "unreachable!(",
    "todo!(",
    "unimplemented!(",
    "assert!(",
    "assert_eq!(",
];

fn source_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rust_files(directory: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in std::fs::read_dir(&current).expect("a readable directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|suffix| suffix == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

#[test]
fn no_source_line_can_panic() {
    let mut offences = Vec::new();
    for file in rust_files(&source_root()) {
        let text = std::fs::read_to_string(&file).expect("a readable source file");
        for (number, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            for construct in PANICKING {
                if line.contains(construct) {
                    offences.push(format!(
                        "{}:{}: `{construct}`\n    {}",
                        file.display(),
                        number + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    assert!(
        offences.is_empty(),
        "a document comes from an untrusted file, so no layout path may panic:\n{}",
        offences.join("\n")
    );
}

/// A document that says something no well-formed file says, laid out at a normal page.
fn survives(name: &str, paragraphs: &[String]) {
    let mut opened = document(paragraphs);
    let flow = flow(&mut opened);
    let mut model = model();
    let mut resume = None;
    for number in 0..40_u32 {
        let Ok(page) = model.layout_page(
            &flow,
            PageIndex::new(number),
            &constraints(6.5, 3.0),
            resume.as_ref(),
        ) else {
            // An error is a fine answer; a panic is not, and reaching this line means one did not
            // happen.
            return;
        };
        resume = page.continuation().cloned();
        if resume.is_none() {
            return;
        }
    }
    // Reaching forty pages without ending is itself a fine answer for a pathological document; what
    // matters is that nothing panicked on the way.
    let _ = name;
}

#[test]
fn pathological_documents_are_laid_out_rather_than_refused() {
    survives("an empty body", &[]);
    survives("an empty paragraph", &["<w:p/>".to_owned()]);
    survives(
        "a paragraph with no text at all",
        &["<w:p><w:r/></w:p>".to_owned()],
    );
    survives(
        "an indent wider than the page",
        &[paragraph(r#"<w:ind w:left="99999"/>"#, "text")],
    );
    survives(
        "a negative indent",
        &[paragraph(r#"<w:ind w:left="-99999"/>"#, "text")],
    );
    survives(
        "a line rule of zero",
        &[paragraph(
            r#"<w:spacing w:line="0" w:lineRule="exact"/>"#,
            "text",
        )],
    );
    survives(
        "a negative line rule",
        &[paragraph(
            r#"<w:spacing w:line="-500" w:lineRule="exact"/>"#,
            "text",
        )],
    );
    survives(
        "a tab stop at a negative position",
        &[paragraph_with_run(
            r#"<w:tabs><w:tab w:val="left" w:pos="-1440"/></w:tabs>"#,
            r#"<w:t>a</w:t><w:tab/><w:t>b</w:t>"#,
        )],
    );
    survives(
        "a measure stated as a universal measure",
        &[paragraph(r#"<w:ind w:left="0.5in"/>"#, "text")],
    );
    survives(
        "a measure that is not a measure",
        &[paragraph(r#"<w:ind w:left="banana"/>"#, "text")],
    );
    survives(
        "a font size that is not a number",
        &[
            r#"<w:p><w:r><w:rPr><w:sz w:val="wide"/></w:rPr><w:t>text</w:t></w:r></w:p>"#
                .to_owned(),
        ],
    );
    survives(
        "a hundred hard breaks in one paragraph",
        &[paragraph_with_run(
            "",
            &std::iter::repeat_n(r#"<w:t>x</w:t><w:br w:type="page"/>"#, 100).collect::<String>(),
        )],
    );
    survives(
        "every constraint at once",
        &[paragraph(
            "<w:keepNext/><w:keepLines/><w:pageBreakBefore/><w:widowControl/>",
            "text",
        )],
    );
    survives(
        "text made entirely of soft hyphens",
        &[paragraph_with_run(
            "",
            &std::iter::repeat_n("<w:softHyphen/>", 200).collect::<String>(),
        )],
    );
    survives(
        "a paragraph of nothing but spaces",
        &[paragraph(r#"<w:jc w:val="both"/>"#, &" ".repeat(500))],
    );
    survives(
        "a paragraph of one word ten thousand characters long",
        &[paragraph("", &"x".repeat(10_000))],
    );
    survives(
        "a right-to-left paragraph of Latin text",
        &[paragraph(
            r#"<w:bidi/><w:jc w:val="both"/>"#,
            "left to right",
        )],
    );
}

/// A page far past the end of the document is refused rather than looped towards.
#[test]
fn a_page_past_the_end_is_refused() {
    let mut opened = document(&[paragraph("", "One short paragraph.")]);
    let flow = flow(&mut opened);
    let mut model = model();
    let refused = model.layout_page(&flow, PageIndex::new(5_000), &constraints(6.5, 11.0), None);
    assert!(refused.is_err(), "there is no page 5,001");
}
