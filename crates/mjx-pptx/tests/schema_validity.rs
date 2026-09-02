//! Schema validity: markup this project ships or authors must never deviate from ECMA-376.
//!
//! Nothing else in the repository validates against the XSDs. The office-open canary
//! (`office_open.rs`) proves a deck *opens* in LibreOffice; it does not prove the markup is *legal*,
//! and LibreOffice tolerated an empty `a:overrideClrMapping` and a `a:scene3d` with no `a:lightRig`
//! across 58 releases. PowerPoint is stricter. This suite closes that gap: it validates
//!
//! * every committed `.pptx` fixture's PresentationML / DrawingML / chart parts, and
//! * **every deck this library authors** — the half that protects future work, because a new
//!   authoring path cannot land invalid markup without a case here going red.
//!
//! The line it draws is *every namespace this project authors*: PresentationML, DrawingML, DrawingML
//! charts, and the two OPC control streams `mjx-opc` rewrites on every single save
//! (`[Content_Types].xml` and `_rels/*.rels`). Markup we only ever preserve — InkML, ActiveX, VML,
//! document properties — is reported as skipped, never validated against a schema it was not written
//! to.
//!
//! Validation is `xmllint --noout --schema` against the ECMA-376 Part 4 **Transitional** schemas and
//! the ECMA-376 Part 2 **OPC** schemas in the git-ignored `References/` tree. `xmllint` is a C tool,
//! which is fine: only *shipped* crates are pure Rust — C tooling is sanctioned for CI and tests (the
//! same rule that lets `office_open.rs` drive LibreOffice).
//!
//! # Skipping
//!
//! The schemas are not committed (`References/` is git-ignored), so when they — or `xmllint` — are
//! absent the suite **skips**, printing a notice and passing, exactly as `office_open.rs` does for a
//! missing LibreOffice. `MJX_REQUIRE_SCHEMA=1` turns any absence into a hard failure so the coverage
//! can never silently disappear. `MJX_SCHEMA_DIR` and `MJX_OPC_SCHEMA_DIR` override where the two
//! schema trees are looked for.
//!
//! **CI runs this suite as a blocking job**, so the skip is a local convenience and never a hole in
//! coverage. The `schema-validity` job in `.github/workflows/ci.yml` downloads the two published
//! ECMA-376 archives, verifies them against the committed SHA-256 manifest
//! `.github/ecma-376-archives.sha256`, extracts them into `References/` via
//! `.github/scripts/fetch-ecma-schemas.sh`, and sets `MJX_REQUIRE_SCHEMA=1` — so a missing tool or
//! schema tree fails the job rather than skipping it. That script is the same one-liner a developer
//! runs to populate `References/` locally; the schemas stay out of the tree because `References/` is
//! git-ignored by a standing rule of this repository.
//!
//! # What is skipped, and why it is never silent
//!
//! Every part is classified and reported; nothing is dropped without a printed reason.
//!
//! * **`mc:AlternateContent`.** Markup Compatibility and Extensibility lives *outside* the base
//!   schema by design (ECMA-376 Part 3), so a part carrying it cannot be validated against `pml.xsd`
//!   as written. Such parts are skipped with the reason named, and
//!   [`mce_parts_are_skipped_with_a_named_reason`] pins exactly which fixture parts that covers, so a
//!   new one cannot appear unnoticed. This shades nothing we write: no authoring path in this
//!   workspace emits `mc:AlternateContent` — it is only ever read.
//! * **Foreign markup.** InkML, ActiveX `ocx`, VML and document properties are markup this project
//!   preserves but never writes. (VML additionally has no validatable root: a `.vml` part's root is a
//!   bare `<xml>` wrapper that the VML schemas declare no global element for, and `vml-main.xsd`
//!   cannot compile without `xml.xsd`, which the Transitional set does not ship.)
//! * **Binary payloads.** Images, OLE objects, embedded workbooks.
//! * **Deviations in inputs we preserve verbatim** — see [`TOLERATED_DEVIATIONS`]. These are matched
//!   error-by-error, so a *new* defect in the same part still fails.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use mjx_dml::{
    AdjustAngle, AdjustCoordinate, Angle, Bevel, BevelPreset, BlipFillMode, Camera, CellBorder,
    CharacterPropertiesSpec, ColorSpec, ConnectionSite, CustomGeometrySpec, DrawCommand,
    EffectListSpec, Emu, FillSpec, Fraction, GlowEffect, GradientStopSpec, GuideSpec, IndentLevel,
    LightRig, LightRigDirection, LightRigType, LineCap, LineDash, LineJoin, LineSpec, LineWidth,
    OnOffStyle, OuterShadowEffect, ParagraphPropertiesSpec, Path2DSpec, PatternType, Point,
    PresetCamera, PresetLineDash, PresetMaterial, Rectangle, RectangleAlignment, Scene3DSpec,
    SchemeColor, Shape3DSpec, ShapeGeometry, TablePart, TableStyleBorder, TableStylePart,
    TextAlignment, TextAnchoring, TextSpacing,
};
use mjx_ooxml_core::{Interner, RawElement, RawNode};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_ooxml_types::presentationml::SlideSizeKind;
use mjx_opc::{Package, PartName, CONTENT_TYPES_ZIP_NAME};
use mjx_pptx::{
    AxisOrientation, CellFormat, CellMargins, Cells, ChartData, ChartKind, Geometry, Hyperlink,
    LegendPosition, Presentation, ShapeBounds, SlideSize, Surface, TableStyleFormat,
};
use mjx_xml::fidelity;

// ---------------------------------------------------------------------------------------------
// Namespaces and schemas
// ---------------------------------------------------------------------------------------------

/// `p:` — PresentationML.
const PRESENTATIONML_NS: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
/// `a:` — DrawingML.
const DRAWINGML_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
/// `c:` — DrawingML charts.
const DRAWINGML_CHART_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
/// SpreadsheetML — the markup inside a chart's embedded workbook, which this project now writes.
const SPREADSHEETML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
/// `mc:` — Markup Compatibility and Extensibility (ECMA-376 Part 3).
const MARKUP_COMPATIBILITY_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
/// The OPC relationships stream (`_rels/*.rels`), written by `mjx-opc` on every save.
const OPC_RELATIONSHIPS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
/// The OPC content-types stream (`[Content_Types].xml`), written by `mjx-opc` on every save.
const OPC_CONTENT_TYPES_NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";

/// Which schema set a namespace's XSD belongs to. The markup schemas are ECMA-376 Part 4
/// (Transitional); the packaging schemas are ECMA-376 Part 2 (OPC) and ship in a separate tree.
#[derive(Clone, Copy)]
enum SchemaSet {
    Markup,
    Packaging,
}

/// The schema governing one namespace: which set it lives in and its file name.
#[derive(Clone, Copy)]
struct SchemaRef {
    set: SchemaSet,
    file: &'static str,
}

/// The schema governing each namespace **this project authors** — the markup formats and the two
/// OPC control streams `mjx-opc` writes on every save. A part rooted in any other namespace is
/// foreign markup this project only ever preserves, and is reported as skipped rather than validated.
///
/// SpreadsheetML is here because an authored chart embeds a whole `.xlsx` workbook: without this arm
/// every part of that workbook would be reported skipped-as-foreign, which is the difference between
/// the gate covering the workbook and only looking as though it does.
fn schema_for_namespace(namespace: &str) -> Option<SchemaRef> {
    let (set, file) = match namespace {
        PRESENTATIONML_NS => (SchemaSet::Markup, "pml.xsd"),
        DRAWINGML_NS => (SchemaSet::Markup, "dml-main.xsd"),
        DRAWINGML_CHART_NS => (SchemaSet::Markup, "dml-chart.xsd"),
        SPREADSHEETML_NS => (SchemaSet::Markup, "sml.xsd"),
        OPC_RELATIONSHIPS_NS => (SchemaSet::Packaging, "opc-relationships.xsd"),
        OPC_CONTENT_TYPES_NS => (SchemaSet::Packaging, "opc-contentTypes.xsd"),
        _ => return None,
    };
    Some(SchemaRef { set, file })
}

// ---------------------------------------------------------------------------------------------
// Tolerated deviations in inputs we preserve verbatim
// ---------------------------------------------------------------------------------------------

/// A schema deviation carried by an *input* this project preserves verbatim rather than markup it
/// writes. Fidelity forbids "fixing" it on the way out, so the suite records it here instead of
/// failing — but every error the validator reports for that part must match, so a *new* defect in
/// the same part is still a failure. A tolerance never applies to a deck this library authors.
struct ToleratedDeviation {
    /// The fixture file name.
    fixture: &'static str,
    /// The absolute part name, e.g. `/ppt/charts/chart1.xml`.
    part: &'static str,
    /// Substring every tolerated validator error line contains.
    error_contains: &'static str,
    /// Why this is not ours to fix.
    reason: &'static str,
}

/// The complete set of deviations this suite tolerates. Deliberately tiny: each entry is a claim
/// that the markup came from somewhere else and that fidelity requires re-emitting it unchanged.
const TOLERATED_DEVIATIONS: &[ToleratedDeviation] = &[ToleratedDeviation {
    fixture: "charts.pptx",
    part: "/ppt/charts/chart1.xml",
    error_contains: "is not a valid value of the atomic type 'xs:unsignedInt'",
    reason: "charts.pptx is python-pptx's template and python-pptx derives c:axId/c:crossAx from a \
             signed hash; the schema says xs:unsignedInt. An input we preserve verbatim, not markup \
             we emit — `authored_charts_are_schema_valid` proves we never write a negative axis id \
             ourselves",
}];

// ---------------------------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------------------------

/// The external tools this suite needs: the validator and the two schema trees.
struct Harness {
    xmllint: PathBuf,
    /// ECMA-376 Part 4, `OfficeOpenXML-XMLSchema-Transitional`.
    markup_schemas: PathBuf,
    /// ECMA-376 Part 2, `OpenPackagingConventions-XMLSchema`.
    packaging_schemas: PathBuf,
}

impl Harness {
    /// The absolute path of the XSD governing a namespace.
    fn schema_path(&self, schema: SchemaRef) -> PathBuf {
        match schema.set {
            SchemaSet::Markup => self.markup_schemas.join(schema.file),
            SchemaSet::Packaging => self.packaging_schemas.join(schema.file),
        }
    }
}

/// Searches `PATH`, then a few well-known locations, for `name`.
fn find_on_path(name: &str) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    ["/usr/bin", "/usr/local/bin", "/opt/homebrew/bin"]
        .iter()
        .map(|dir| Path::new(dir).join(name))
        .find(|p| p.is_file())
}

/// A directory of XSDs, located from an environment override or the git-ignored `References/` tree
/// at the workspace root, and confirmed present by **every** schema this suite validates against.
///
/// Checking all of them, not one marker, is what makes a half-extracted tree skip loudly instead of
/// reporting perfectly good markup as invalid because the schema behind it was missing.
///
/// `References/` is never read as a *committed* test input — its absence skips the suite.
fn find_schema_dir(env_var: &str, default_suffix: &str, markers: &[&str]) -> Option<PathBuf> {
    let candidate = match std::env::var_os(env_var) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../References")
            .join(default_suffix),
    };
    if !markers
        .iter()
        .all(|marker| candidate.join(marker).is_file())
    {
        return None;
    }
    // Canonicalized: the schemas import one another by absolute path, and libxml2 warns about a
    // "duplicate" import when the same file is reached by two spellings of the same directory.
    Some(candidate.canonicalize().unwrap_or(candidate))
}

/// The harness, or `None` when the suite should skip.
///
/// Skipping prints a notice and passes, so the workspace stays green without the reference schemas.
/// `MJX_REQUIRE_SCHEMA` turns a missing tool or schema tree into a hard failure.
fn harness() -> Option<Harness> {
    let xmllint = find_on_path("xmllint");
    let markup = find_schema_dir(
        "MJX_SCHEMA_DIR",
        "ECMA-376-4_5th_edition_december_2016/OfficeOpenXML-XMLSchema-Transitional",
        &["pml.xsd", "dml-main.xsd", "dml-chart.xsd", "sml.xsd"],
    );
    let packaging = find_schema_dir(
        "MJX_OPC_SCHEMA_DIR",
        "ECMA-376-2_5th_edition_december_2021/OpenPackagingConventions-XMLSchema",
        &["opc-relationships.xsd", "opc-contentTypes.xsd"],
    );

    if let (Some(xmllint), Some(markup_schemas), Some(packaging_schemas)) =
        (&xmllint, &markup, &packaging)
    {
        return Some(Harness {
            xmllint: xmllint.clone(),
            markup_schemas: markup_schemas.clone(),
            packaging_schemas: packaging_schemas.clone(),
        });
    }

    let mut missing = Vec::new();
    if xmllint.is_none() {
        missing.push("xmllint");
    }
    if markup.is_none() {
        missing.push("the ECMA-376 Part 4 Transitional schemas (References/ or MJX_SCHEMA_DIR)");
    }
    if packaging.is_none() {
        missing.push("the ECMA-376 Part 2 OPC schemas (References/ or MJX_OPC_SCHEMA_DIR)");
    }
    let missing = missing.join(", ");
    assert!(
        std::env::var_os("MJX_REQUIRE_SCHEMA").is_none(),
        "MJX_REQUIRE_SCHEMA is set but {missing} could not be found"
    );
    eprintln!("skipping schema-validity tests: {missing} not available on this machine");
    None
}

/// A private working directory under the system temp dir, removed on drop.
struct WorkDir(PathBuf);

impl WorkDir {
    fn new(tag: &str) -> Self {
        // Cargo runs the cases in this binary concurrently and several of them inspect the same deck,
        // so the pid alone does not make the name unique — a serial number does. Without it one case
        // clears the directory another is still writing into.
        static SERIAL: AtomicUsize = AtomicUsize::new(0);
        let serial = SERIAL.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("mjx_schema_{tag}_{}_{serial}", std::process::id()));
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

// ---------------------------------------------------------------------------------------------
// Per-part classification
// ---------------------------------------------------------------------------------------------

/// What became of one part.
#[derive(Debug)]
enum PartOutcome {
    /// Validated clean against the named schema.
    Validated(&'static str),
    /// Failed only with errors covered by a [`ToleratedDeviation`].
    Tolerated {
        schema: &'static str,
        reason: &'static str,
    },
    /// Carries `mc:AlternateContent`, which the base schema does not describe.
    SkippedAlternateContent,
    /// Not XML at all (an image, an OLE object, an embedded workbook).
    SkippedBinary(String),
    /// Declared XML, but the classifier could not reach a namespace decision at all.
    SkippedNoNamespace,
    /// XML, but rooted in a namespace this project does not author.
    SkippedForeignNamespace(String),
    /// Failed validation.
    Failed {
        schema: &'static str,
        report: String,
    },
}

impl PartOutcome {
    /// The one-line report entry, always printed, so no skip is ever silent.
    fn describe(&self) -> String {
        match self {
            Self::Validated(schema) => format!("valid ({schema})"),
            Self::Tolerated { schema, reason } => {
                format!("tolerated deviation ({schema}) — {reason}")
            }
            Self::SkippedAlternateContent => "skipped — carries mc:AlternateContent, which lives \
                 outside the base schema by design (ECMA-376 Part 3); no authoring path in this \
                 workspace emits it"
                .to_owned(),
            Self::SkippedBinary(content_type) => {
                format!("skipped — not XML (content type {content_type})")
            }
            Self::SkippedNoNamespace => {
                "skipped — the root element is in no namespace at all".to_owned()
            }
            Self::SkippedForeignNamespace(namespace) => format!(
                "skipped — root element is in {namespace}, which this project does not author"
            ),
            Self::Failed { schema, report } => format!("INVALID ({schema})\n{report}"),
        }
    }
}

/// The content type of an embedded Office package — a chart's workbook. Its payload is a whole OPC
/// container, so the suite opens it and validates the markup *inside* rather than skipping it as a
/// binary blob.
const EMBEDDED_PACKAGE_CONTENT_TYPES: [&str; 1] =
    ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"];

/// Whether a content type names an XML payload. `vmlDrawing` is XML despite the content type not
/// saying so; it is classified as foreign markup a step later, which is the truthful reason.
fn is_xml_content_type(content_type: &str) -> bool {
    content_type.ends_with("+xml")
        || content_type.ends_with("/xml")
        || content_type.ends_with("vmlDrawing")
}

/// Whether any element anywhere in the subtree is in the MCE namespace. Iterative: a part is
/// untrusted input and depth is not bounded by anything we control.
fn carries_alternate_content(root: &RawElement, interner: &Interner) -> bool {
    let mut stack = vec![root];
    while let Some(element) = stack.pop() {
        if element
            .name
            .namespace
            .is_some_and(|ns| interner.resolve(ns) == MARKUP_COMPATIBILITY_NS)
        {
            return true;
        }
        for child in &element.children {
            if let RawNode::Element(child) = child {
                stack.push(child);
            }
        }
    }
    false
}

/// Runs the validator over one file and returns its report when the part fails to validate.
///
/// A schema that fails to *compile* is a harness fault, not a fixture defect, so it panics rather
/// than being reported as invalid markup.
fn xmllint_report(harness: &Harness, schema: SchemaRef, file: &Path) -> Option<String> {
    let schema_path = harness.schema_path(schema);
    let output = Command::new(&harness.xmllint)
        .arg("--noout")
        .arg("--schema")
        .arg(&schema_path)
        .arg(file)
        .output()
        .unwrap_or_else(|e| panic!("running {}: {e}", harness.xmllint.display()));

    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        !stderr.contains("failed to compile"),
        "{} failed to compile — the schema tree is unusable:\n{stderr}",
        schema_path.display()
    );
    if output.status.success() {
        return None;
    }
    // Keep only the validity errors: the trailing "<file> fails to validate" line tells the caller
    // nothing it does not know, and a libxml2 parser *warning* is not a deviation from the schema.
    let report: Vec<&str> = stderr
        .lines()
        .filter(|line| !line.ends_with("fails to validate"))
        .filter(|line| !line.contains("warning :"))
        .collect();
    Some(report.join("\n"))
}

/// Rewrites a validator report so each line names the part rather than the temporary file, and
/// strips the `Schemas validity error :` boilerplate.
fn readable_report(report: &str, temp_file: &Path, part: &str) -> String {
    let prefix = temp_file.display().to_string();
    report
        .lines()
        .map(|line| {
            let line = line.replace(&prefix, part);
            line.replace("Schemas validity error : ", "")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Classifies and validates every part of one package.
///
/// `tolerances` is empty for a deck this library authors: nothing it writes is ever excused.
fn inspect_deck(
    harness: &Harness,
    label: &str,
    bytes: &[u8],
    tolerances: &[&ToleratedDeviation],
) -> Vec<(String, PartOutcome)> {
    let mut outcomes = Vec::new();
    inspect_package(harness, label, bytes, tolerances, "", &mut outcomes);
    outcomes
}

/// Classifies and validates every part of one package, appending to `outcomes`.
///
/// `prefix` names where the package sits: empty for the deck itself, and
/// `/ppt/embeddings/Microsoft_Excel_Sheet1.xlsx!` for a package **embedded inside** it — a chart's
/// workbook, which this library now authors and whose SpreadsheetML must therefore be validated
/// rather than skipped as a binary blob.
fn inspect_package(
    harness: &Harness,
    label: &str,
    bytes: &[u8],
    tolerances: &[&ToleratedDeviation],
    prefix: &str,
    outcomes: &mut Vec<(String, PartOutcome)>,
) {
    let package = Package::open(bytes).unwrap_or_else(|e| panic!("{label}: opening package: {e}"));
    let work = WorkDir::new(&format!("{label}{prefix}").replace(['.', '/', ' ', '!'], "_"));

    // Every ZIP entry, not just the addressable parts: `[Content_Types].xml` is markup `mjx-opc`
    // writes on every save and is exactly the kind of stream a bug would break silently.
    for entry in package.entries() {
        let name = format!("{prefix}/{}", entry.name);
        let Some(payload) = entry.bytes() else {
            panic!("{label}: {name} has no materialized bytes in a freshly opened package");
        };
        // The content-types stream describes every other part and has no content type of its own.
        let content_type = PartName::from_zip_name(&entry.name)
            .ok()
            .and_then(|part| package.content_type_of(&part).map(str::to_owned));

        if let Some(content_type) = content_type {
            if EMBEDDED_PACKAGE_CONTENT_TYPES.contains(&content_type.as_str()) {
                let nested = format!("{name}!");
                inspect_package(harness, label, payload, tolerances, &nested, outcomes);
                continue;
            }
            if !is_xml_content_type(&content_type) {
                outcomes.push((name, PartOutcome::SkippedBinary(content_type)));
                continue;
            }
        } else if entry.name != CONTENT_TYPES_ZIP_NAME {
            panic!("{label}: no content type for {name}");
        }

        let document = fidelity::parse(payload)
            .unwrap_or_else(|e| panic!("{label}: {name} is declared XML but does not parse: {e}"));
        let Some(namespace) = document
            .root
            .name
            .namespace
            .map(|ns| document.interner.resolve(ns).to_owned())
        else {
            outcomes.push((name, PartOutcome::SkippedNoNamespace));
            continue;
        };
        let Some(schema) = schema_for_namespace(&namespace) else {
            outcomes.push((name, PartOutcome::SkippedForeignNamespace(namespace)));
            continue;
        };
        if carries_alternate_content(&document.root, &document.interner) {
            outcomes.push((name, PartOutcome::SkippedAlternateContent));
            continue;
        }

        let file = work.path().join(
            name.trim_start_matches('/')
                .replace(['/', '[', ']', '!'], "_"),
        );
        std::fs::write(&file, payload).expect("write part for validation");
        let outcome = match xmllint_report(harness, schema, &file) {
            None => PartOutcome::Validated(schema.file),
            Some(report) => {
                let tolerance = tolerances.iter().find(|t| {
                    t.part == name
                        && report
                            .lines()
                            .all(|line| line.contains(t.error_contains) || line.trim().is_empty())
                });
                match tolerance {
                    Some(tolerance) => PartOutcome::Tolerated {
                        schema: schema.file,
                        reason: tolerance.reason,
                    },
                    None => PartOutcome::Failed {
                        schema: schema.file,
                        report: readable_report(&report, &file, &name),
                    },
                }
            }
        };
        outcomes.push((name, outcome));
    }
}

/// Prints the per-part report and fails on any invalid part.
///
/// Also fails when *nothing* was validated: a classification bug that skipped every part would
/// otherwise let invalid markup through as a silent pass.
fn assert_outcomes_are_valid(label: &str, outcomes: &[(String, PartOutcome)]) {
    let mut validated = 0usize;
    let mut failures = Vec::new();
    let mut lines = Vec::new();
    for (name, outcome) in outcomes {
        if matches!(
            outcome,
            PartOutcome::Validated(_) | PartOutcome::Tolerated { .. }
        ) {
            validated += 1;
        }
        if let PartOutcome::Failed { .. } = outcome {
            failures.push(format!("{name}: {}", outcome.describe()));
        }
        lines.push(format!("  {name}: {}", outcome.describe()));
    }
    println!("schema validity — {label}\n{}", lines.join("\n"));

    assert!(
        failures.is_empty(),
        "{label}: {} part(s) do not validate against ECMA-376:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        validated > 0,
        "{label}: not one PresentationML/DrawingML part was validated — every part was classified \
         away, which would let invalid markup pass unnoticed"
    );
}

/// Validates a deck this library **authored**. No deviation is tolerated: everything in a deck we
/// write is ours.
fn assert_authored_deck_is_schema_valid(label: &str, bytes: &[u8]) {
    let Some(harness) = harness() else { return };
    let outcomes = inspect_deck(&harness, label, bytes, &[]);
    assert_outcomes_are_valid(label, &outcomes);
}

/// Validates a committed fixture, allowing only the deviations [`TOLERATED_DEVIATIONS`] records for
/// it.
fn assert_fixture_is_schema_valid(name: &str) {
    let Some(harness) = harness() else { return };
    let tolerances: Vec<&ToleratedDeviation> = TOLERATED_DEVIATIONS
        .iter()
        .filter(|t| t.fixture == name)
        .collect();
    let outcomes = inspect_deck(&harness, name, &fixture(name), &tolerances);
    assert_outcomes_are_valid(name, &outcomes);
}

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

/// A valid 2×2 truecolour PNG (76 bytes), inlined so no binary fixture is committed.
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x02, 0x00, 0x00, 0x00, 0xFD, 0xD4, 0x9A,
    0x73, 0x00, 0x00, 0x00, 0x13, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0x78, 0x60, 0x60, 0x60,
    0x90, 0xF0, 0x80, 0x01, 0x88, 0x81, 0x2C, 0x00, 0x25, 0xAE, 0x05, 0x61, 0x56, 0x69, 0x41, 0x72,
    0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

// ---------------------------------------------------------------------------------------------
// The committed fixtures
// ---------------------------------------------------------------------------------------------

#[test]
fn sample_fixture_is_schema_valid() {
    assert_fixture_is_schema_valid("sample.pptx");
}

#[test]
fn tables_fixture_is_schema_valid() {
    assert_fixture_is_schema_valid("tables.pptx");
}

#[test]
fn layouts_fixture_is_schema_valid() {
    assert_fixture_is_schema_valid("layouts.pptx");
}

#[test]
fn notes_fixture_is_schema_valid() {
    assert_fixture_is_schema_valid("notes.pptx");
}

#[test]
fn text_levels_fixture_is_schema_valid() {
    assert_fixture_is_schema_valid("text_levels.pptx");
}

#[test]
fn hyperlinks_fixture_is_schema_valid() {
    assert_fixture_is_schema_valid("hyperlinks.pptx");
}

#[test]
fn effects_theme_fixture_is_schema_valid() {
    assert_fixture_is_schema_valid("effects_theme.pptx");
}

#[test]
fn charts_fixture_is_schema_valid() {
    assert_fixture_is_schema_valid("charts.pptx");
}

#[test]
fn ole_fixture_is_schema_valid() {
    assert_fixture_is_schema_valid("ole.pptx");
}

#[test]
fn ink_fixture_is_schema_valid() {
    assert_fixture_is_schema_valid("ink.pptx");
}

#[test]
fn activex_fixture_is_schema_valid() {
    assert_fixture_is_schema_valid("activex.pptx");
}

#[test]
fn vml_fixture_is_schema_valid() {
    assert_fixture_is_schema_valid("vml.pptx");
}

#[test]
fn every_pptx_fixture_is_covered_by_a_case() {
    // A fixture added without a case here would never be validated. The directory is the source of
    // truth; this fails the moment the two fall out of step.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .expect("read fixtures directory")
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().to_string_lossy().into_owned();
            name.ends_with(".pptx").then_some(name)
        })
        .collect();
    on_disk.sort();

    let mut covered: Vec<String> = [
        "activex.pptx",
        "charts.pptx",
        "effects_theme.pptx",
        "hyperlinks.pptx",
        "ink.pptx",
        "layouts.pptx",
        "notes.pptx",
        "ole.pptx",
        "sample.pptx",
        "tables.pptx",
        "text_levels.pptx",
        "vml.pptx",
    ]
    .iter()
    .map(|&s| s.to_owned())
    .collect();
    covered.sort();

    assert_eq!(
        on_disk, covered,
        "the .pptx fixtures on disk and the cases in this file disagree — add a case for every new \
         fixture, or its markup is never validated"
    );
}

#[test]
fn mce_parts_are_skipped_with_a_named_reason() {
    // The MCE skip must be an inspectable fact, not a silent hole. This pins exactly which fixture
    // parts it covers: a *new* part carrying mc:AlternateContent fails here rather than quietly
    // dropping out of validation.
    let Some(harness) = harness() else { return };

    let mut skipped = Vec::new();
    for name in ["ole.pptx", "ink.pptx", "activex.pptx", "vml.pptx"] {
        for (part, outcome) in inspect_deck(&harness, name, &fixture(name), &[]) {
            if let PartOutcome::SkippedAlternateContent = outcome {
                assert!(
                    outcome.describe().contains("mc:AlternateContent"),
                    "{name}: the skip reason must name mc:AlternateContent"
                );
                skipped.push(format!("{name}{part}"));
            }
        }
    }
    skipped.sort();
    assert_eq!(
        skipped,
        vec![
            "ink.pptx/ppt/slides/slide1.xml".to_owned(),
            "ole.pptx/ppt/slides/slide1.xml".to_owned(),
        ],
        "the set of parts skipped for markup compatibility changed"
    );
}

// ---------------------------------------------------------------------------------------------
// The decks this library authors — the half that protects future work
// ---------------------------------------------------------------------------------------------

/// The four slide extents worth checking: PowerPoint's two defaults, and the two ends of what
/// `ST_SlideSizeCoordinate` permits — the placeholder geometry is rescaled per deck, so a bad
/// rescale shows up as an invalid `a:ext` at one end and not the other.
const BLANK_DECK_SIZES: &[(&str, i64, i64, SlideSizeKind)] = &[
    ("16:9", 12_192_000, 6_858_000, SlideSizeKind::Screen16X9),
    ("4:3", 9_144_000, 6_858_000, SlideSizeKind::Screen4X3),
    ("smallest", 914_400, 914_400, SlideSizeKind::Custom),
    ("largest", 51_206_400, 51_206_400, SlideSizeKind::Custom),
];

#[test]
fn a_blank_deck_is_schema_valid() {
    // `Presentation::blank` writes `presentation.xml`, a theme, a slide master and a slide layout
    // from nothing, plus the `[Content_Types].xml` and four `.rels` parts underneath them. Nothing
    // in this deck came from a file, so every byte of it is this project's to answer for — which is
    // exactly why a committed binary template was refused.
    for (label, width_emu, height_emu, kind) in BLANK_DECK_SIZES.iter().copied() {
        let deck = Presentation::blank(SlideSize {
            width_emu,
            height_emu,
            kind,
        })
        .expect("blank");
        let saved = deck.save().expect("save");
        assert_authored_deck_is_schema_valid(&format!("blank deck ({label})"), &saved);
    }
}

#[test]
fn a_blank_deck_filled_end_to_end_is_schema_valid() {
    // The whole "create a document from nothing" story: a blank deck, a slide built on its own
    // layout, both placeholders filled, a text box and a second slide added — every part authored,
    // none preserved.
    let mut deck = Presentation::blank(SlideSize {
        width_emu: 12_192_000,
        height_emu: 6_858_000,
        kind: SlideSizeKind::Screen16X9,
    })
    .expect("blank");
    let slide = deck
        .add_slide_from_layout(0)
        .expect("add slide from layout");
    deck.set_shape_text_content(slide, 0, "Built from nothing")
        .expect("set the title");
    deck.set_shape_text_content(slide, 1, "First point\nSecond point")
        .expect("set the body");
    deck.add_text_box(
        slide,
        "A text box too",
        ShapeBounds::from_inches(1.0, 5.0, 4.0, 1.0),
    )
    .expect("add text box");
    deck.add_slide_with_text(
        "A second slide",
        ShapeBounds::from_inches(1.0, 1.0, 6.0, 2.0),
    )
    .expect("add slide with text");
    let saved = deck.save().expect("save");
    assert_authored_deck_is_schema_valid("blank deck, filled end to end", &saved);
}

#[test]
fn the_blank_deck_validates_every_part_it_ships() {
    // A classification bug that skipped the new parts would let invalid markup through as a pass,
    // so pin the verdicts: all nine entries are accounted for and the five markup streams are
    // genuinely validated, not skipped.
    let Some(harness) = harness() else { return };
    let deck = Presentation::blank(SlideSize {
        width_emu: 12_192_000,
        height_emu: 6_858_000,
        kind: SlideSizeKind::Screen16X9,
    })
    .expect("blank");
    let saved = deck.save().expect("save");
    let outcomes = inspect_deck(&harness, "blank deck coverage", &saved, &[]);

    let validated: Vec<&str> = outcomes
        .iter()
        .filter(|(_, outcome)| matches!(outcome, PartOutcome::Validated(_)))
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(
        validated,
        [
            "/[Content_Types].xml",
            "/_rels/.rels",
            "/ppt/presentation.xml",
            "/ppt/slideMasters/slideMaster1.xml",
            "/ppt/slideLayouts/slideLayout1.xml",
            "/ppt/theme/theme1.xml",
            "/ppt/_rels/presentation.xml.rels",
            "/ppt/slideMasters/_rels/slideMaster1.xml.rels",
            "/ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        ],
        "every entry of a blank deck must be validated, none skipped"
    );
    assert_eq!(outcomes.len(), validated.len());
}

#[test]
fn an_unedited_deck_saves_schema_valid() {
    // The save path itself: open and re-emit, touching nothing. A regression in the writer that
    // corrupted a part would show up here before any authoring case.
    let pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("saved unedited sample.pptx", &saved);
}

#[test]
fn an_added_text_box_is_schema_valid() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_text_box(
        0,
        "Schema canary\nLine two",
        ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0),
    )
    .expect("add text box");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("added text box", &saved);
}

#[test]
fn an_added_slide_is_schema_valid() {
    // `build::empty_slide_bytes` — a whole new part with its own root and namespaces.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_slide().expect("add slide");
    pres.add_slide_with_text("Second slide", ShapeBounds::from_inches(1.0, 1.0, 5.0, 2.0))
        .expect("add slide with text");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("added slides", &saved);
}

#[test]
fn a_slide_built_from_a_layout_is_schema_valid() {
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let slide = pres.add_slide_from_layout(1).expect("add slide");
    pres.set_shape_text(slide, 0, 0, "Built from a layout")
        .expect("set the title");
    pres.set_shape_text(slide, 1, 0, "The placeholders came with the slide")
        .expect("set the body");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("slide from a layout", &saved);
}

#[test]
fn speaker_notes_are_schema_valid() {
    // `build::empty_notes_slide_bytes` and `build::notes_master_bytes` — sample.pptx has neither a
    // notes slide nor a notes master, so both templates are synthesized here.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.set_notes_text(0, "Speaker notes, written from scratch")
        .expect("set notes");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("speaker notes", &saved);
}

#[test]
fn an_added_picture_is_schema_valid() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let picture = pres
        .add_picture(0, TINY_PNG, ShapeBounds::from_inches(1.0, 1.0, 3.0, 2.0))
        .expect("add picture");
    pres.set_shape_outline(
        0,
        picture,
        &LineSpec {
            fill: Some(FillSpec::solid(ColorSpec::Srgb("203864".into()))),
            width: Some(LineWidth::from_points(3.0)),
            ..LineSpec::new()
        },
    )
    .expect("outline the picture");

    let filled = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(5.0, 1.0, 3.0, 2.0),
        )
        .expect("add shape");
    let rel_id = pres.add_image(0, TINY_PNG).expect("add image");
    pres.set_shape_fill(
        0,
        filled,
        &FillSpec::Blip {
            rel_id,
            mode: BlipFillMode::Stretch,
        },
    )
    .expect("picture fill");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("added picture", &saved);
}

#[test]
fn shape_geometry_fill_outline_and_effects_are_schema_valid() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");

    let preset = pres
        .add_shape(
            0,
            PresetShapeType::RoundedRectangle,
            ShapeBounds::from_inches(0.5, 0.5, 3.0, 1.5),
        )
        .expect("add shape");
    pres.set_shape_geometry(
        0,
        preset,
        Geometry::Preset(ShapeGeometry::RoundedRectangle {
            corner_radius: Fraction::from_ratio(0.3),
        }),
    )
    .expect("set geometry");
    pres.set_shape_fill(
        0,
        preset,
        &FillSpec::linear_gradient(
            vec![
                GradientStopSpec {
                    position: Fraction::from_ratio(0.0),
                    color: ColorSpec::Srgb("FF0000".into()),
                },
                GradientStopSpec {
                    position: Fraction::from_ratio(1.0),
                    color: ColorSpec::Scheme(SchemeColor::Accent1),
                },
            ],
            Angle::from_degrees(45.0),
        ),
    )
    .expect("gradient fill");
    pres.set_shape_outline(
        0,
        preset,
        &LineSpec {
            width: Some(LineWidth::from_points(3.0)),
            cap: Some(LineCap::Round),
            fill: Some(FillSpec::Solid(ColorSpec::Scheme(SchemeColor::Accent1))),
            dash: Some(LineDash::Preset(PresetLineDash::Dash)),
            join: Some(LineJoin::Round),
            ..LineSpec::new()
        },
    )
    .expect("outline");
    pres.set_shape_effects(
        0,
        preset,
        &EffectListSpec {
            glow: Some(GlowEffect {
                color: ColorSpec::Scheme(SchemeColor::Accent1),
                radius: Some(Emu::from_points(5.0)),
            }),
            outer_shadow: Some(OuterShadowEffect {
                color: ColorSpec::Srgb("808080".into()),
                blur_radius: Some(Emu::from_points(4.0)),
                distance: Some(Emu::from_points(3.0)),
                direction: Some(Angle::from_degrees(45.0)),
                scale_x: None,
                scale_y: None,
                skew_x: None,
                skew_y: None,
                alignment: Some(RectangleAlignment::BottomRight),
                rotate_with_shape: Some(false),
            }),
            ..EffectListSpec::new()
        },
    )
    .expect("effects");

    let pattern = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(0.5, 2.5, 3.0, 1.5),
        )
        .expect("add pattern shape");
    pres.set_shape_fill(
        0,
        pattern,
        &FillSpec::pattern(
            PatternType::Percent25,
            ColorSpec::Srgb("000000".into()),
            ColorSpec::Srgb("FFFFFF".into()),
        ),
    )
    .expect("pattern fill");

    let custom = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(4.5, 0.5, 2.0, 2.0),
        )
        .expect("add custom-geometry shape");
    pres.set_shape_geometry(
        0,
        custom,
        Geometry::Custom(CustomGeometrySpec {
            paths: vec![Path2DSpec {
                width: Some(Emu::from_emu(1_828_800)),
                height: Some(Emu::from_emu(1_828_800)),
                commands: vec![
                    DrawCommand::MoveTo(Point::from_emu(914_400, 0)),
                    DrawCommand::LineTo(Point::from_emu(1_828_800, 1_828_800)),
                    DrawCommand::LineTo(Point::from_emu(0, 1_828_800)),
                    DrawCommand::Close,
                ],
                ..Path2DSpec::default()
            }],
            ..CustomGeometrySpec::default()
        }),
    )
    .expect("custom geometry");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("shape geometry, fill, outline and effects", &saved);
}

#[test]
fn a_guide_driven_custom_geometry_is_schema_valid() {
    // `CT_CustomGeometry2D` is a fixed sequence — `avLst`, `gdLst`, `ahLst`, `cxnLst`, `rect`, then
    // the required `pathLst`. The case above authors only the path list; this one authors every
    // auxiliary child, which is the geometry the guide-formula evaluator exists to read, so a
    // misordered or malformed guide list cannot slip out of the writer unnoticed.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 1.0, 2.0, 2.0),
        )
        .expect("add shape");
    pres.set_shape_geometry(
        0,
        idx,
        Geometry::Custom(CustomGeometrySpec {
            adjust_values: vec![GuideSpec {
                name: "adj1".to_owned(),
                formula: "val 25000".to_owned(),
            }],
            guides: vec![GuideSpec {
                name: "apex".to_owned(),
                formula: "*/ w adj1 100000".to_owned(),
            }],
            connection_sites: vec![ConnectionSite {
                angle: AdjustAngle::Guide("3cd4".to_owned()),
                position: Point {
                    x: AdjustCoordinate::Guide("apex".to_owned()),
                    y: AdjustCoordinate::Emu(Emu::from_emu(0)),
                },
            }],
            text_rectangle: Some(Rectangle {
                left: AdjustCoordinate::Guide("l".to_owned()),
                top: AdjustCoordinate::Guide("t".to_owned()),
                right: AdjustCoordinate::Guide("r".to_owned()),
                bottom: AdjustCoordinate::Guide("b".to_owned()),
            }),
            paths: vec![Path2DSpec {
                commands: vec![
                    DrawCommand::MoveTo(Point {
                        x: AdjustCoordinate::Guide("apex".to_owned()),
                        y: AdjustCoordinate::Emu(Emu::from_emu(0)),
                    }),
                    DrawCommand::LineTo(Point::from_emu(1_828_800, 1_828_800)),
                    DrawCommand::LineTo(Point::from_emu(0, 1_828_800)),
                    DrawCommand::Close,
                ],
                ..Path2DSpec::default()
            }],
            ..CustomGeometrySpec::default()
        }),
    )
    .expect("custom geometry");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("a guide-driven custom geometry", &saved);
}

#[test]
fn a_3d_shape_is_schema_valid() {
    // `a:scene3d` is exactly where defect B lived: `CT_Scene3D` requires a camera *and* a light rig,
    // and a scene with only a camera is invalid. Our writer must never emit one.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_shape(
            0,
            PresetShapeType::RoundedRectangle,
            ShapeBounds::from_inches(1.0, 1.0, 3.0, 2.0),
        )
        .expect("add shape");
    pres.set_shape_scene_3d(
        0,
        idx,
        &Scene3DSpec {
            camera: Camera {
                preset: PresetCamera::OrthographicFront,
                field_of_view: None,
                zoom: Some(Fraction::from_ratio(1.0)),
                rotation: None,
            },
            light_rig: LightRig {
                rig: LightRigType::ThreePoint,
                direction: LightRigDirection::Top,
                rotation: None,
            },
        },
    )
    .expect("set scene");
    pres.set_shape_3d_properties(
        0,
        idx,
        &Shape3DSpec {
            z: None,
            extrusion_height: Some(Emu::from_emu(190_500)),
            contour_width: Some(Emu::from_emu(12_700)),
            material: Some(PresetMaterial::Metal),
            bevel_top: Some(Bevel {
                width: Some(Emu::from_emu(76_200)),
                height: Some(Emu::from_emu(38_100)),
                preset: Some(BevelPreset::Circle),
            }),
            bevel_bottom: None,
            extrusion_color: Some(ColorSpec::Srgb("C0C0C0".to_owned())),
            contour_color: Some(ColorSpec::Srgb("404040".to_owned())),
        },
    )
    .expect("set 3-D properties");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("3-D shape", &saved);
}

#[test]
fn grouped_shapes_are_schema_valid() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let first = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0),
        )
        .expect("add first");
    let second = pres
        .add_shape(
            0,
            PresetShapeType::Ellipse,
            ShapeBounds::from_inches(4.0, 1.0, 2.0, 1.0),
        )
        .expect("add second");
    pres.group_shapes(0, &[first.into(), second.into()])
        .expect("group");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("grouped shapes", &saved);
}

#[test]
fn a_created_table_is_schema_valid() {
    // The whole table builder: the graphic frame, the grid, every cell, cell formatting, merges, and
    // growing and shrinking the grid afterwards.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let table = pres
        .add_table(0, 3, 3, ShapeBounds::from_inches(0.5, 1.5, 8.0, 3.0))
        .expect("add table");
    for (row, column, text) in [
        (0, 0, "Region"),
        (0, 1, "Revenue"),
        (0, 2, "Change"),
        (1, 0, "North"),
        (1, 1, "1,204"),
        (1, 2, "+12%"),
        (2, 0, "South"),
        (2, 1, "987"),
        (2, 2, "-3%"),
    ] {
        pres.set_cell_text(0, table, row, column, 0, text)
            .expect("set cell text");
    }
    pres.format_cell_text(
        0,
        table,
        Cells::row(0),
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("bold the header");
    pres.format_cell_paragraphs(
        0,
        table,
        Cells::rectangle(1..3, 1..3),
        &ParagraphPropertiesSpec::new().with_alignment(TextAlignment::Right),
    )
    .expect("align the numbers");
    pres.set_row_height(0, table, 0, Emu::from_points(30.0))
        .expect("taller header row");
    pres.format_cells(
        0,
        table,
        Cells::row(0),
        &CellFormat::new()
            .with_fill(FillSpec::Solid(ColorSpec::Srgb("1F3864".to_owned())))
            .with_border(
                CellBorder::Bottom,
                LineSpec {
                    width: Some(LineWidth::from_emu(19_050)),
                    fill: Some(FillSpec::Solid(ColorSpec::Srgb("FFFFFF".to_owned()))),
                    ..LineSpec::default()
                },
            )
            .with_anchor(TextAnchoring::Center),
    )
    .expect("style the header row");
    pres.format_cells(
        0,
        table,
        Cells::all(),
        &CellFormat::new().with_margins(CellMargins::uniform(Emu::from_points(6.0))),
    )
    .expect("roomier insets");
    pres.merge_cells(0, table, Cells::rectangle(2..3, 1..3))
        .expect("merge the totals");
    pres.set_cell_text(0, table, 2, 1, 0, "984 (-3%)")
        .expect("the merged cell's text");
    pres.insert_column(0, table, 1).expect("insert a column");
    pres.insert_row(0, table, 3).expect("append a row");
    pres.remove_column(0, table, 1).expect("remove the column");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("created table", &saved);
}

#[test]
fn a_created_table_style_is_schema_valid() {
    // `build::table_styles_bytes` plus everything `mjx-dml` appends to it — a brand-new part with a
    // content-type override and a relationship off the presentation.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let table = pres
        .add_table(0, 3, 3, ShapeBounds::from_inches(0.5, 1.5, 8.0, 3.0))
        .expect("add table");
    pres.set_cell_text(0, table, 0, 0, 0, "Region")
        .expect("cell text");
    pres.set_table_part(0, table, TablePart::FirstRow, true)
        .expect("header flag");
    pres.set_table_part(0, table, TablePart::BandedRows, true)
        .expect("banding flag");

    let style_id = "{9A8B7C6D-5E4F-4A3B-8C2D-1E0F9A8B7C6D}";
    pres.create_table_style(style_id, "Report Style")
        .expect("create style");
    pres.format_table_style_part(
        style_id,
        TableStylePart::WholeTable,
        &TableStyleFormat::new()
            .with_border(TableStyleBorder::InsideHorizontal, LineSpec::default()),
    )
    .expect("whole-table borders");
    pres.format_table_style_part(
        style_id,
        TableStylePart::FirstRow,
        &TableStyleFormat::new()
            .with_bold(OnOffStyle::On)
            .with_text_color(ColorSpec::Srgb("FFFFFF".to_owned()))
            .with_fill(FillSpec::solid(ColorSpec::Srgb("1F3864".to_owned())))
            .with_cell_material(PresetMaterial::Metal)
            .with_cell_bevel(Bevel {
                width: Some(Emu::from_emu(76_200)),
                height: Some(Emu::from_emu(38_100)),
                preset: Some(BevelPreset::Circle),
            })
            .with_cell_light_rig(LightRig {
                rig: LightRigType::ThreePoint,
                direction: LightRigDirection::Top,
                rotation: None,
            }),
    )
    .expect("header style");
    pres.format_table_style_part(
        style_id,
        TableStylePart::Band1Horizontal,
        &TableStyleFormat::new().with_fill(FillSpec::solid(ColorSpec::Srgb("D9E1F2".to_owned()))),
    )
    .expect("banded style");
    pres.set_table_style(0, table, style_id)
        .expect("assign style");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("created table style", &saved);
}

/// Every chart kind this library can author. `Stock` is absent: `CT_StockChart` requires three or
/// four series, so it is exercised by its own case rather than with the shared two-series data.
const AUTHORED_CHART_KINDS: [ChartKind; 15] = [
    ChartKind::Bar,
    ChartKind::Bar3D,
    ChartKind::Line,
    ChartKind::Line3D,
    ChartKind::Pie,
    ChartKind::Pie3D,
    ChartKind::OfPie,
    ChartKind::Area,
    ChartKind::Area3D,
    ChartKind::Scatter,
    ChartKind::Doughnut,
    ChartKind::Radar,
    ChartKind::Bubble,
    ChartKind::Surface,
    ChartKind::Surface3D,
];

#[test]
fn authored_charts_are_schema_valid() {
    // `mjx-chart`'s authoring path for every chart kind, in one deck — and with it every embedded
    // workbook, whose SpreadsheetML the harness now validates against `sml.xsd` rather than skipping
    // as a binary blob. This is also the case that proves we never emit the negative `c:axId` that
    // charts.pptx (python-pptx's template) carries: no tolerance applies to an authored deck, so a
    // signed axis id here would fail.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    for (i, kind) in AUTHORED_CHART_KINDS.into_iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let offset = i as f64;
        let chart = ChartData::new(kind)
            .categories(["Q1", "Q2", "Q3"])
            .series("Revenue", [1.0 + offset, 2.5, 3.25])
            .series("Cost", [0.5, 1.5, 2.0]);
        pres.add_chart(0, &chart, ShapeBounds::from_inches(0.5, 0.5, 4.0, 3.0))
            .expect("add chart");
    }
    let stock = ChartData::new(ChartKind::Stock)
        .categories(["Mon", "Tue", "Wed"])
        .series("High", [12.0, 13.0, 11.5])
        .series("Low", [9.0, 9.5, 8.75])
        .series("Close", [11.0, 10.5, 10.0]);
    pres.add_chart(0, &stock, ShapeBounds::from_inches(0.5, 0.5, 4.0, 3.0))
        .expect("add a stock chart");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("authored charts (every kind)", &saved);
}

#[test]
fn an_authored_chart_with_a_title_and_a_legend_is_schema_valid() {
    // The title carries DrawingML rich text inside the chart namespace, and the legend is a whole
    // element `CT_Chart` admits at exactly one position — both are markup this library now writes.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let chart = ChartData::new(ChartKind::Line)
        .categories(["Q1", "Q2", "Q3"])
        .series("Revenue", [1.0, 2.0, 3.0])
        .title("Revenue by quarter")
        .legend(LegendPosition::Bottom);
    pres.add_chart(0, &chart, ShapeBounds::from_inches(0.5, 0.5, 6.0, 4.0))
        .expect("add chart");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("authored chart with a title and a legend", &saved);
}

#[test]
fn an_edited_chart_is_schema_valid() {
    // Editing an authored chart part in place — the series values and categories are rewritten
    // through the model, so the part is re-serialized rather than re-emitted verbatim, and the
    // embedded workbook is regenerated alongside it.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2"])
        .series("Revenue", [1.0, 2.0]);
    let frame = pres
        .add_chart(0, &chart, ShapeBounds::from_inches(0.5, 0.5, 6.0, 4.0))
        .expect("add chart");
    pres.set_chart_series_values(0, frame, 0, &[9.5, 8.25])
        .expect("rewrite the values");
    pres.set_chart_series_categories(0, frame, 0, &["Spring", "Summer"])
        .expect("rewrite the categories");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("edited chart", &saved);
}

#[test]
fn an_edited_chart_axis_legend_title_and_series_style_are_schema_valid() {
    // Every setter this tier adds, on one chart: each inserts an element into a `CT_*` sequence, so
    // a child placed in the wrong position fails here rather than in PowerPoint.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3"])
        .series("Revenue", [1.0, 2.0, 3.0])
        .series("Cost", [0.5, 1.5, 2.5]);
    let frame = pres
        .add_chart(0, &chart, ShapeBounds::from_inches(0.5, 0.5, 6.0, 4.0))
        .expect("add chart");

    pres.set_chart_title(0, frame, Some("Quarterly results"))
        .expect("set the title");
    pres.set_chart_legend(0, frame, Some(LegendPosition::Right))
        .expect("place the legend");
    pres.set_chart_axis_title(0, frame, 0, Some("Quarter"))
        .expect("title the category axis");
    pres.set_chart_axis_title(0, frame, 1, Some("Millions"))
        .expect("title the value axis");
    pres.set_chart_axis_scale(0, frame, 1, Some(0.0), Some(10.0))
        .expect("bound the value axis");
    pres.set_chart_axis_orientation(0, frame, 1, AxisOrientation::MaximumToMinimum)
        .expect("reverse the value axis");
    pres.set_chart_axis_gridlines(0, frame, 1, true, true)
        .expect("rule gridlines");
    pres.set_chart_series_fill(
        0,
        frame,
        0,
        &FillSpec::Solid(ColorSpec::Srgb("4472C4".to_owned())),
    )
    .expect("fill the first series");
    pres.set_chart_series_line(
        0,
        frame,
        1,
        &LineSpec::solid(
            LineWidth::from_points(1.5),
            ColorSpec::Srgb("ED7D31".to_owned()),
        ),
    )
    .expect("outline the second series");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid(
        "edited chart axes, legend, title and series style",
        &saved,
    );
}

#[test]
fn formatted_text_is_schema_valid() {
    // The text model at three scopes — shape-wide, paragraph-wide and one character range — plus the
    // paragraph properties (bullets, indents, spacing) that carry the most attributes.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let slide = pres.add_slide_from_layout(1).expect("add slide");
    pres.set_shape_text(slide, 0, 0, "Formatted title")
        .expect("set the title");
    pres.set_shape_text(slide, 1, 0, "A bulleted line of body text")
        .expect("set the body");
    pres.set_shape_run_properties(
        slide,
        0,
        &CharacterPropertiesSpec::new()
            .with_size_points(32.0)
            .with_color(ColorSpec::Scheme(SchemeColor::Accent1)),
    )
    .expect("size the title");
    pres.set_paragraph_properties(
        slide,
        1,
        0,
        &ParagraphPropertiesSpec::new()
            .with_level(IndentLevel::of(1))
            .with_alignment(TextAlignment::Left)
            .with_left_margin_points(36.0)
            .with_indent_points(-18.0)
            .with_space_before(TextSpacing::points(6.0))
            .with_bullet_character("•"),
    )
    .expect("lay out the body");
    pres.set_text_range_properties(
        slide,
        1,
        0,
        2..10,
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("bold one word");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("formatted text", &saved);
}

#[test]
fn hyperlinks_are_schema_valid() {
    // `a:hlinkClick` on a run and on a shape, external and internal — both add a relationship and an
    // element whose attribute set the schema constrains tightly.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_slide().expect("a slide to jump to");
    let box_idx = pres
        .add_text_box(0, "Visit us", ShapeBounds::from_inches(1.0, 3.0, 4.0, 1.0))
        .expect("add text box");
    pres.set_run_hyperlink(
        0,
        box_idx,
        0,
        0,
        &Hyperlink::Url("https://example.invalid/".to_owned()),
    )
    .expect("run hyperlink");

    let shape = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 4.5, 3.0, 1.0),
        )
        .expect("add shape");
    pres.set_shape_hyperlink(0, shape, &Hyperlink::Slide(1))
        .expect("shape hyperlink");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("hyperlinks", &saved);
}

#[test]
fn transformed_shapes_are_schema_valid() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.set_shape_bounds(0, 0, ShapeBounds::from_inches(0.5, 0.3, 8.0, 1.0))
        .expect("place the title");
    let idx = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 2.0, 3.0, 1.0),
        )
        .expect("add shape");
    let mut transform = pres
        .shape_transform(0, idx)
        .expect("read transform")
        .unwrap_or_default();
    transform.rotation = Some(Angle::from_degrees(30.0));
    transform.flip_horizontal = Some(true);
    transform.flip_vertical = Some(true);
    pres.set_shape_transform(0, idx, &transform)
        .expect("rotate and mirror");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("transformed shapes", &saved);
}

#[test]
fn an_edited_layout_and_a_pruned_deck_are_schema_valid() {
    // Editing a layout (not a slide) and then removing shapes and a slide: the surfaces and the
    // pruning path, which rewrite parts other cases never touch.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    pres.set_shape_fill(
        Surface::Layout(1),
        0,
        &FillSpec::solid(ColorSpec::Srgb("C00000".into())),
    )
    .expect("fill the layout's title");

    let slide = pres.add_slide_from_layout(1).expect("add slide");
    pres.set_shape_text(slide, 0, 0, "Edited and pruned")
        .expect("set the title");
    let doomed = pres
        .add_text_box(
            slide,
            "removed again",
            ShapeBounds::from_inches(5.0, 5.0, 3.0, 1.0),
        )
        .expect("add text box");
    pres.remove_shape(slide, doomed).expect("remove the box");
    pres.remove_slide(0).expect("remove the first slide");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("edited layout, pruned deck", &saved);
}

#[test]
fn a_deck_built_from_every_authoring_path_is_schema_valid() {
    // One deck touched by everything at once. Individually valid parts can still combine into an
    // invalid one — a slide carrying a text box, a shape, a picture, a table and a chart exercises
    // `p:spTree`'s content model, not just each element's.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let slide = pres
        .add_slide_with_text("Everything", ShapeBounds::from_inches(0.5, 0.3, 8.0, 1.0))
        .expect("add slide with text");
    pres.set_notes_text(slide, "Notes for the everything slide")
        .expect("notes");
    pres.add_shape(
        slide,
        PresetShapeType::Ellipse,
        ShapeBounds::from_inches(0.5, 1.5, 2.0, 1.0),
    )
    .expect("shape");
    pres.add_picture(
        slide,
        TINY_PNG,
        ShapeBounds::from_inches(3.0, 1.5, 2.0, 1.0),
    )
    .expect("picture");
    let table = pres
        .add_table(slide, 2, 2, ShapeBounds::from_inches(0.5, 3.0, 4.0, 1.5))
        .expect("table");
    pres.set_cell_text(slide, table, 0, 0, 0, "Cell")
        .expect("cell text");
    let chart = ChartData::new(ChartKind::Line)
        .categories(["A", "B"])
        .series("Series", [1.0, 2.0]);
    pres.add_chart(slide, &chart, ShapeBounds::from_inches(5.0, 3.0, 4.0, 1.5))
        .expect("chart");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("every authoring path in one deck", &saved);
}
