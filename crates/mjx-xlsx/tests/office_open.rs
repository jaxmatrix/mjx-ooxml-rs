//! The MJXOFF-112 exit gate: an `.xlsx` this library *constructs* must actually open in a real
//! Office implementation. We drive LibreOffice headless to convert the workbook to PDF and assert a
//! valid PDF came out — soffice's exit code is unreliable, so the produced PDF is the real signal
//! that the workbook parsed and rendered.
//!
//! # Why this file exists at all
//!
//! Schema validity is necessary and **not sufficient**. `crates/mjx-xlsx/tests/schema_gate.rs`
//! proves every part `Workbook::blank` authors satisfies `sml.xsd` and the OPC schemas; a package
//! can satisfy every XSD and still be refused for a broken relationship graph, a missing required
//! part or a content type nothing claims. This is what catches that, and Excel is the least
//! forgiving of the three formats about exactly those things.
//!
//! **The CI job that runs this has to name `-p mjx-xlsx`.** MJXOFF-98 found the failure mode
//! directly: the step named `-p mjx-pptx` alone, Word's four cases had never executed anywhere, and
//! the job reported green throughout. `.github/workflows/ci.yml`'s `office-open` step now names all
//! three crates, and `libreoffice-calc` is installed beside Impress and Writer — `soffice` cannot
//! convert an `.xlsx` to PDF with only the other two components present.
//!
//! When no `soffice`/`libreoffice` binary is found the test **skips** (prints a notice and passes),
//! so the suite stays green on machines without LibreOffice. CI sets `MJX_REQUIRE_SOFFICE=1`, which
//! turns a missing binary into a hard failure so coverage can never silently disappear. The harness
//! below is `crates/mjx-docx/tests/office_open.rs`'s, restated rather than shared across a sideways
//! crate edge — see `crates/mjx-pptx/tests/office_open.rs` for why an integration test cannot reach
//! a harness living in another crate's `tests/` directory.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use mjx_sml::write::{CellFormatSpec, CellFormatTarget, PatternFillSpec};
use mjx_sml::{CellReference, CellValue, FontProperties};
use mjx_xlsx::Workbook;

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

/// Converts `xlsx` to PDF with LibreOffice and asserts a valid PDF resulted. `tag` names the
/// temporary working directory. Skips (returning [`Outcome::Skipped`]) when LibreOffice is absent
/// unless `MJX_REQUIRE_SOFFICE` is set, in which case a missing binary panics.
fn convert_opens(xlsx: &[u8], tag: &str) -> Outcome {
    let Some(soffice) = find_soffice() else {
        if std::env::var_os("MJX_REQUIRE_SOFFICE").is_some() {
            panic!("MJX_REQUIRE_SOFFICE is set but no soffice/libreoffice binary was found");
        }
        eprintln!("skipping office-open test `{tag}`: no soffice/libreoffice on this machine");
        return Outcome::Skipped;
    };

    let work = WorkDir::new(tag);
    let input = work.path().join("input.xlsx");
    std::fs::write(&input, xlsx).expect("write input xlsx");
    let profile = work.path().join("profile");
    let user_installation = format!("-env:UserInstallation=file://{}", profile.display());

    let mut child = Command::new(&soffice)
        .arg("--headless")
        .arg("--norestore")
        .arg(&user_installation)
        .arg("--convert-to")
        .arg("pdf:calc_pdf_Export")
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
fn a_blank_workbook_opens() {
    // The strongest claim this child makes: an `.xlsx` built from **nothing** — no template, no
    // fixture, not one byte taken from a file — is a workbook a real implementation accepts.
    //
    // It is also the case that says whether `crates/mjx-sml/src/write/`'s reading of the schema is
    // right. `CT_Workbook` requires a `sheets` with at least one `sheet`, and `CT_Worksheet` a
    // `sheetData`; a "minimal shell" that stopped short of either would still be a well-formed ZIP
    // full of well-formed XML, and this is what refuses it.
    let workbook = Workbook::blank().expect("a blank workbook is authored");
    let _ = convert_opens(&workbook.save().expect("it saves"), "blank_workbook");
}

#[test]
fn a_workbook_authored_from_nothing_and_filled_in_opens() {
    // The story a caller actually follows: blank, a second tab, a shared string, every cell type,
    // and a format built out of an appended font, fill and `xf`. Every part here is markup this
    // library wrote, and the style indices are the thing most likely to be silently wrong — a
    // dangling `@fontId` or `@fillId` is not a schema error and is exactly what a renderer trips on.
    let mut workbook = Workbook::blank().expect("authored");
    let north = workbook.intern_shared_string("North").expect("interns");
    let bold = workbook
        .append_font(&FontProperties {
            font_name: Some("Calibri".to_owned()),
            size_in_points: Some(11.0),
            bold: Some(true),
            ..FontProperties::default()
        })
        .expect("appends");
    let yellow = workbook
        .append_pattern_fill(&PatternFillSpec::solid("FFFF00"))
        .expect("appends");
    let highlight = workbook
        .append_cell_format(
            CellFormatTarget::CellFormats,
            &CellFormatSpec {
                font_index: Some(bold),
                fill_index: Some(yellow),
                applies_font: Some(true),
                applies_fill: Some(true),
                ..CellFormatSpec::skeleton_cell_format()
            },
        )
        .expect("appends");

    let at = |address: &str| CellReference::parse(address).expect("a literal address");
    for (address, value) in [
        ("A1", CellValue::SharedString(north)),
        ("B1", CellValue::Number(19.25)),
        ("C1", CellValue::Boolean(true)),
        ("D1", CellValue::Error("#N/A")),
        ("E1", CellValue::InlineString("in the cell")),
    ] {
        workbook
            .set_cell_value(0, at(address), value)
            .expect("the store accepts the value");
    }
    workbook
        .set_cell_style(0, at("B1"), Some(highlight))
        .expect("the store accepts the style");
    let second = workbook.add_sheet("Data").expect("a second tab");
    workbook
        .set_cell_value(second, at("A1"), CellValue::Number(7.0))
        .expect("the store accepts the value");

    let _ = convert_opens(&workbook.save().expect("saves"), "authored_workbook");
}

#[test]
fn unmodified_fixture_opens() {
    // Baseline: isolates "what we authored is broken" from "the fixture or soffice is broken".
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join("sample.xlsx");
    let bytes =
        std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()));
    let _ = convert_opens(&bytes, "baseline");
}

#[test]
fn a_workbook_opened_and_saved_unchanged_opens() {
    // The other half of the baseline: a fixture this library re-emitted. If the case above passes
    // and this one fails, the packaging layer is what broke it rather than anything authored.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join("sample.xlsx");
    let bytes =
        std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()));
    let workbook = Workbook::open(&bytes).expect("opens");
    let _ = convert_opens(&workbook.save().expect("saves"), "round_tripped_fixture");
}
