//! The Phase-2 exit gate: a `.pptx` we *construct* must actually open in a real Office
//! implementation. We drive LibreOffice headless to convert the deck to PDF and assert a valid PDF
//! came out — soffice's exit code is unreliable, so the produced PDF is the real signal that the
//! document parsed and rendered.
//!
//! When no `soffice`/`libreoffice` binary is found the test **skips** (prints a notice and passes),
//! so the suite stays green on machines without LibreOffice. CI sets `MJX_REQUIRE_SOFFICE=1`, which
//! turns a missing binary into a hard failure so coverage can never silently disappear.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use mjx_pptx::{Presentation, ShapeBounds};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

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
    /// The deck converted to a valid PDF.
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

/// Converts `pptx` to PDF with LibreOffice and asserts a valid PDF resulted. `tag` names the
/// temporary working directory. Skips (returning [`Outcome::Skipped`]) when LibreOffice is absent
/// unless `MJX_REQUIRE_SOFFICE` is set, in which case a missing binary panics.
fn convert_opens(pptx: &[u8], tag: &str) -> Outcome {
    let Some(soffice) = find_soffice() else {
        if std::env::var_os("MJX_REQUIRE_SOFFICE").is_some() {
            panic!("MJX_REQUIRE_SOFFICE is set but no soffice/libreoffice binary was found");
        }
        eprintln!("skipping office-open test `{tag}`: no soffice/libreoffice on this machine");
        return Outcome::Skipped;
    };

    let work = WorkDir::new(tag);
    let input = work.path().join("input.pptx");
    std::fs::write(&input, pptx).expect("write input pptx");
    let profile = work.path().join("profile");
    let user_installation = format!("-env:UserInstallation=file://{}", profile.display());

    let mut child = Command::new(&soffice)
        .arg("--headless")
        .arg("--norestore")
        .arg(&user_installation)
        .arg("--convert-to")
        .arg("pdf:impress_pdf_Export")
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
fn unmodified_fixture_opens() {
    // Baseline: isolates "our edit broke it" from "fixture or soffice is broken".
    let _ = convert_opens(&fixture("sample.pptx"), "baseline");
}

#[test]
fn deck_with_added_text_box_opens() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_text_box(
        0,
        "Canary\nLine two",
        ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0),
    )
    .expect("add text box");
    let saved = pres.save().expect("save");
    // The constructed deck must open in LibreOffice.
    let _ = convert_opens(&saved, "added_text_box");
}

#[test]
fn deck_with_added_slide_opens() {
    // Exercises the whole add-slide construction (empty-slide template + the four package touches)
    // through a real Office implementation — the strongest check the new slide is valid.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_slide_with_text("Second slide", ShapeBounds::from_inches(1.0, 1.0, 5.0, 2.0))
        .expect("add slide with text");
    let saved = pres.save().expect("save");
    let _ = convert_opens(&saved, "added_slide");
}
