//! The MJXOFF-98 exit gate: a `.docx` we *construct* must actually open in a real Office
//! implementation. We drive LibreOffice headless to convert the document to PDF and assert a valid
//! PDF came out — soffice's exit code is unreliable, so the produced PDF is the real signal that the
//! document parsed and rendered.
//!
//! When no `soffice`/`libreoffice` binary is found the test **skips** (prints a notice and passes),
//! so the suite stays green on machines without LibreOffice. CI sets `MJX_REQUIRE_SOFFICE=1`, which
//! turns a missing binary into a hard failure so coverage can never silently disappear. This is
//! `crates/mjx-pptx/tests/office_open.rs`'s own mechanism, restated here rather than shared across a
//! sideways crate edge — see that file's doc comment for why an integration test cannot reach a
//! harness living in another crate's `tests/` directory.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use mjx_docx::{Document, PageSize};

/// Locates a LibreOffice command, searching `PATH` then a few well-known install locations.
fn find_soffice() -> Option<PathBuf> {
    let names = ["soffice", "libreoffice"];
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            for name in names {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    let known = [
        "/usr/bin/soffice",
        "/usr/bin/libreoffice",
        "/Applications/LibreOffice.app/Contents/MacOS/soffice",
        "/opt/libreoffice/program/soffice",
    ];
    known.iter().map(PathBuf::from).find(|p| p.is_file())
}

/// Result of an attempted conversion.
enum Outcome {
    /// The document converted to a valid PDF.
    Opened,
    /// No LibreOffice was available and the environment did not require it.
    Skipped,
}

/// A private working directory under the system temp dir, removed on drop.
struct WorkDir(PathBuf);

impl WorkDir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("mjx_office_{tag}_{}", std::process::id()));
        // Fresh: clear any leftovers from a previous crashed run.
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create work dir");
        Self(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Converts `docx` to PDF with LibreOffice and asserts a valid PDF resulted. `tag` names the
/// temporary working directory. Skips (returning [`Outcome::Skipped`]) when LibreOffice is absent
/// unless `MJX_REQUIRE_SOFFICE` is set, in which case a missing binary panics.
fn convert_opens(docx: &[u8], tag: &str) -> Outcome {
    let Some(soffice) = find_soffice() else {
        if std::env::var_os("MJX_REQUIRE_SOFFICE").is_some() {
            panic!("MJX_REQUIRE_SOFFICE is set but no soffice/libreoffice binary was found");
        }
        eprintln!("skipping office-open test `{tag}`: no soffice/libreoffice on this machine");
        return Outcome::Skipped;
    };

    let work = WorkDir::new(tag);
    let input = work.path().join("input.docx");
    std::fs::write(&input, docx).expect("write input docx");
    let profile = work.path().join("profile");
    let user_installation = format!("-env:UserInstallation=file://{}", profile.display());

    let mut child = Command::new(&soffice)
        .arg("--headless")
        .arg("--norestore")
        .arg(&user_installation)
        .arg("--convert-to")
        .arg("pdf:writer_pdf_Export")
        .arg("--outdir")
        .arg(work.path())
        .arg(&input)
        .spawn()
        .unwrap_or_else(|e| panic!("spawning {}: {e}", soffice.display()));

    // soffice may fork/detach; wait for the PDF to appear (or the child to exit) with a hard cap.
    let output_pdf = work.path().join("input.pdf");
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        if output_pdf.is_file() {
            break;
        }
        match child.try_wait() {
            Ok(Some(_status)) => {
                // Child exited; give the filesystem a moment, then stop waiting.
                if output_pdf.is_file() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
                break;
            }
            Ok(None) => {}
            Err(e) => panic!("waiting on soffice: {e}"),
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "soffice did not produce {} within the timeout",
                output_pdf.display()
            );
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let _ = child.wait();

    let pdf = std::fs::read(&output_pdf)
        .unwrap_or_else(|e| panic!("reading produced pdf {}: {e}", output_pdf.display()));
    assert!(
        pdf.len() > 100,
        "produced pdf is implausibly small ({} bytes)",
        pdf.len()
    );
    assert!(
        pdf.starts_with(b"%PDF"),
        "produced file does not start with the %PDF signature"
    );
    Outcome::Opened
}

#[test]
fn a_blank_document_opens() {
    // The strongest claim this child makes: a `.docx` built from *nothing* — no template, no
    // fixture, not one byte taken from a file — is a document a real Office implementation accepts.
    // Schema validity is necessary and not sufficient: a document can satisfy every XSD and still be
    // refused for a broken relationship graph or a missing required part, which is what this catches.
    let document = Document::blank(PageSize::a4()).expect("blank");
    let _ = convert_opens(&document.save().expect("save"), "blank_document_a4");
}

#[test]
fn a_blank_us_letter_document_opens() {
    // The other named default, and the other orientation this crate ships: landscape swaps
    // `w:pgSz`'s `w`/`h` and writes `w:orient="landscape"`, which is exactly the kind of markup a
    // real consumer either honours or complains about — LibreOffice is the check that it honours it.
    let document = Document::blank(PageSize::us_letter().landscape()).expect("blank");
    let _ = convert_opens(
        &document.save().expect("save"),
        "blank_document_us_letter_landscape",
    );
}

#[test]
fn a_blank_document_with_added_text_opens() {
    // The story a caller actually follows: blank, a paragraph appended, a run of text put in it,
    // saved. Every part here is markup this library wrote.
    let mut document = Document::blank(PageSize::a4()).expect("blank");
    document.append_paragraph().expect("append paragraph");
    document
        .append_run(1, "Built from nothing, opened by a real implementation.")
        .expect("append run");
    let _ = convert_opens(&document.save().expect("save"), "blank_document_with_text");
}

#[test]
fn unmodified_fixture_opens() {
    // Baseline: isolates "our edit broke it" from "fixture or soffice is broken".
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join("sample.docx");
    let bytes =
        std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()));
    let _ = convert_opens(&bytes, "baseline");
}
