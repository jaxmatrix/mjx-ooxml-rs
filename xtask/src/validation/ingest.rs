//! What happens to an Office-authored file on the way in (MJXOFF-130).
//!
//! # Why an ingest path exists at all
//!
//! The corpus at [`CORPUS_DIRECTORY`](super::CORPUS_DIRECTORY) is the one thing this project cannot
//! produce for itself: a file real Microsoft Office wrote. It arrives from a person who re-saved
//! something from PowerPoint, Word or Excel, and the question they have at that moment is *"is this
//! file usable, and what does it tell us?"* — before it is committed, not after. This module answers
//! it, and `xtask/tests/office_corpus.rs` asserts the same answers over whatever the directory
//! already holds, so the check a person runs by hand and the check CI runs are one implementation.
//!
//! # Three verdicts, not two, and the line between them
//!
//! An ingested file is **not ours**. That single fact decides which check may fail a build:
//!
//! * [`Verdict::Failed`] — the finding is about *this library*. Round-trip byte identity, the
//!   fidelity tree, the facade's own re-save, a package defect we introduced by saving, a part out
//!   of `xsd:sequence` against our generated tables. Every one of these says we lost or changed
//!   something, and every one fails.
//! * [`Verdict::Reported`] — the finding is about *the file*. A package defect it arrived with, a
//!   part its producer wrote that the ECMA-376 XSDs reject, a namespace this gate does not
//!   categorise. A7b's scope rule is explicit that a file arriving with a defect must still open and
//!   re-save unchanged, and MJXOFF-103 measured the other half: Apache POI 5.5.1 writes an empty
//!   `<c:tx/>` for an unnamed series, which `dml-chart.xsd` rejects. **A red build over someone
//!   else's markup teaches nobody anything**, so these are printed in full and fail nothing.
//! * [`Verdict::Skipped`] — the check could not run: no `References/`, no `xmllint`.
//!
//! # The schema half says nothing about an ignorable extension, and that is deliberate
//!
//! `mjx-schema-gate` validates the **markup-compatibility-resolved** view of a part, because
//! `mc:Ignorable` names attributes the base schema has no declaration for. Resolution removes an
//! ignorable element together with its content — and `sml.xsd`'s `CT_Extension` and
//! `dml-chart.xsd`'s declare their whole content model as a bare `<xsd:any processContents="lax"/>`,
//! whose `minOccurs` therefore defaults to **1**. An `<ext>` whose only child was ignorable was
//! emptied by the resolution and then rejected by the schema:
//!
//! ```text
//! Element '{…/spreadsheetml/2006/main}ext': Missing child element(s). Expected is one of ( {*}*, * ).
//! ```
//!
//! That fired identically on every conformant file carrying an ignorable extension, which is
//! essentially every workbook and every chart Office has written since 2010 — a defect in how MCE
//! resolution and schema validation compose rather than a quirk of anyone's spreadsheet, and so
//! neither a tolerance (those are per file and per message) nor a property of the corpus.
//! **MJXOFF-196** closed it: an extension slot resolution empties is dropped along with the
//! extension it held, because such an element exists only to carry it.
//!
//! Two things about that fix matter when reading a report here.
//!
//! * **The set of elements it applies to is derived from the schemas, not listed by hand.**
//!   `crates/mjx-schema-gate/src/wildcard_slots.rs` computes every element whose content model is
//!   `xsd:any` particles and nothing else and cannot match the empty sequence, and a test asserts
//!   the committed table equals what the pinned XSDs say. It found **five**. This module used to
//!   carry the third by hand — `vml-officeDrawing.xsd`'s `CT_EquationXml`, found by MJXOFF-221 —
//!   with the note "so that a fix cannot stop at two"; a derivation is the general form of that
//!   note, and it also found `sml.xsd`'s `CT_Schema` and `CT_DataBinding`.
//! * **What is left is a residue.** The gate reports nothing about the markup *inside* an ignorable
//!   extension, and it never could: `processContents="lax"` with no loaded schema for that namespace
//!   means a validator handed the content accepts it unread. A report that is silent about an
//!   `<ext>` has not checked it.
//!
//! # No I/O beyond reading the file it was handed
//!
//! Everything below is bytes in, findings out. Office is never run, LibreOffice is never run, and
//! nothing is written into the corpus: committing a file is a person's decision, taken against the
//! redistribution rule in `tests/office-authored/README.md`, and a tool that copied the file in
//! would be taking it for them.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use mjx_ooxml::{Deck, Document, Workbook};
use mjx_opc::Package;

use super::{corpus, Area, ArtefactFormat, AREAS};

/// What one check concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The check ran and held.
    Held,
    /// The check ran and found something that belongs to the file rather than to this library.
    Reported,
    /// The check ran and found something that belongs to **this library**.
    Failed,
    /// The check could not run — a missing tool or schema tree, never a silent absence.
    Skipped,
}

impl Verdict {
    /// The word the report prints.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Held => "held",
            Self::Reported => "reported",
            Self::Failed => "FAILED",
            Self::Skipped => "skipped",
        }
    }
}

/// One check's outcome.
#[derive(Debug)]
pub struct Finding {
    /// What was checked, as the report names it.
    pub check: &'static str,
    /// What it concluded.
    pub verdict: Verdict,
    /// The detail: a count when it held, the whole report when it did not.
    pub detail: String,
}

/// Everything one file's ingestion found.
#[derive(Debug)]
pub struct IngestReport {
    /// The file's name, as it was handed over.
    pub name: String,
    /// The entry this file answers, when one could be determined.
    pub area: Option<&'static Area>,
    /// The format its extension names.
    pub format: ArtefactFormat,
    /// Where it would live if it were committed.
    pub commit_as: Option<PathBuf>,
    /// Every check, in the order they ran.
    pub findings: Vec<Finding>,
}

impl IngestReport {
    /// Every finding whose verdict is [`Verdict::Failed`] — the ones that are this library's.
    #[must_use]
    pub fn failures(&self) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|finding| finding.verdict == Verdict::Failed)
            .collect()
    }

    /// Whether any check ran at all and held. A report of nothing but skips is not a green.
    #[must_use]
    pub fn checks_that_held(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.verdict == Verdict::Held)
            .count()
    }

    /// The finding for one check, by name.
    #[must_use]
    pub fn finding(&self, check: &str) -> Option<&Finding> {
        self.findings.iter().find(|finding| finding.check == check)
    }

    /// The whole report, as the command prints it and as a failing test quotes it.
    #[must_use]
    pub fn render(&self) -> String {
        let mut text = String::new();
        let _ = writeln!(text, "{} — {}", self.name, self.format.extension());
        let _ = writeln!(
            text,
            "  answers      {}",
            match self.area {
                Some(area) => format!("{} ({})", area.id, area.title),
                None => "no entry — pass --area <id> to bind it to one".to_owned(),
            }
        );
        let _ = writeln!(
            text,
            "  commit as    {}",
            match &self.commit_as {
                Some(path) => path.display().to_string(),
                None => "nothing — an unbound file has no slot in the corpus".to_owned(),
            }
        );
        for finding in &self.findings {
            let _ = writeln!(
                text,
                "  {:<12} {:<8} {}",
                finding.check,
                finding.verdict.label(),
                finding.detail
            );
        }
        text
    }
}

/// The area a file name binds to: `v-xlsx-02.xlsx` answers `V-XLSX-02`.
#[must_use]
pub fn area_for_file_name(name: &str) -> Option<&'static Area> {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    AREAS.iter().find(|area| area.id.eq_ignore_ascii_case(stem))
}

/// The area an explicit `--area` token names, whether written as an id or as a bare number and a
/// format.
#[must_use]
pub fn area_for_token(token: &str, format: ArtefactFormat) -> Option<&'static Area> {
    let wanted = token.to_ascii_uppercase();
    AREAS.iter().find(|area| {
        area.format == format
            && (area.id == wanted
                || wanted.trim_start_matches('0') == area.number().trim_start_matches('0'))
    })
}

/// The format a file's extension names.
#[must_use]
pub fn format_for_file_name(name: &str) -> Option<ArtefactFormat> {
    let (_, extension) = name.rsplit_once('.')?;
    ArtefactFormat::parse(&extension.to_ascii_lowercase())
}

/// Reads a file and runs every check over it.
///
/// `area` overrides the binding the file name implies; `None` falls back to the name.
///
/// # Errors
/// If the file cannot be read, or its extension names no format this pass covers. Neither is a
/// finding: a file the walk cannot classify is a mistake to correct, not a verdict to record.
pub fn ingest(path: &Path, area: Option<&'static Area>) -> Result<IngestReport> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .with_context(|| format!("{} names no file", path.display()))?;
    let format = format_for_file_name(&name).with_context(|| {
        format!(
            "{name} has no extension this pass covers. The corpus holds .pptx, .docx and .xlsx, \
             one file per area, named by the area's own id"
        )
    })?;
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(report(
        &name,
        format,
        area.or_else(|| area_for_file_name(&name)),
        &bytes,
    ))
}

/// Runs every check over bytes already in hand.
#[must_use]
pub fn report(
    name: &str,
    format: ArtefactFormat,
    area: Option<&'static Area>,
    bytes: &[u8],
) -> IngestReport {
    let mut findings = Vec::new();

    let package = match Package::open(bytes) {
        Ok(package) => package,
        Err(error) => {
            findings.push(Finding {
                check: "opens",
                verdict: Verdict::Failed,
                detail: format!("Package::open refused it: {error}"),
            });
            return IngestReport {
                name: name.to_owned(),
                area,
                format,
                commit_as: area.map(corpus::original_path),
                findings,
            };
        }
    };
    findings.push(Finding {
        check: "opens",
        verdict: Verdict::Held,
        detail: format!("{} entries, {} bytes", package.entries().len(), bytes.len()),
    });

    // Saved **once** and lent to both checks that need it. A corpus file can be tens of megabytes,
    // and re-saving it per check would triple the cost of the report for no extra information: the
    // bytes an edit-free `save_unchecked` produces are the same bytes every time — the determinism
    // `xtask/tests/validation_harness.rs` already holds the generators to.
    let saved = package.save_unchecked();
    let saved: Result<&[u8], &mjx_opc::OpcError> = saved.as_deref();
    findings.push(detected_format(format, bytes));
    findings.push(container_round_trip(&package, saved));
    findings.push(fidelity_round_trip(&package));
    findings.push(facade_round_trip(format, &package, bytes));
    findings.push(package_invariants(&package, saved));
    findings.push(child_order(name, bytes));
    findings.push(schema_validity(name, bytes));

    IngestReport {
        name: name.to_owned(),
        area,
        format,
        commit_as: area.map(corpus::original_path),
        findings,
    }
}

/// What the main part says the file is, against what its extension claims.
fn detected_format(format: ArtefactFormat, bytes: &[u8]) -> Finding {
    match mjx_ooxml::detect_format(bytes) {
        Err(error) => Finding {
            check: "format",
            verdict: Verdict::Failed,
            detail: format!("detect_format could not classify the main part: {error}"),
        },
        Ok(detected) => {
            let expected = match format {
                ArtefactFormat::Presentation => mjx_ooxml::FormatFamily::Presentation,
                ArtefactFormat::Document => mjx_ooxml::FormatFamily::WordProcessing,
                ArtefactFormat::Workbook => mjx_ooxml::FormatFamily::Spreadsheet,
            };
            if detected.family() == expected {
                Finding {
                    check: "format",
                    verdict: Verdict::Held,
                    detail: format!(
                        "{detected:?}, which is the family .{} names",
                        format.extension()
                    ),
                }
            } else {
                Finding {
                    check: "format",
                    verdict: Verdict::Failed,
                    detail: format!(
                        "the main part says {detected:?} and the name says .{} — rename the file, \
                         or this is a detection defect",
                        format.extension()
                    ),
                }
            }
        }
    }
}

/// The `mjx-opc --test roundtrip` contract: same entries in the same order, every payload verbatim.
///
/// `save_unchecked` rather than `save`, and deliberately: [`Package::save`] runs
/// [`Package::validate`] first, so a file that *arrived* with a package defect could not be
/// round-tripped at all — and A7b's rule is that such a file must still open and re-save unchanged.
/// The defect itself is reported by [`package_invariants`], which is where it belongs.
fn container_round_trip(package: &Package, saved: Result<&[u8], &mjx_opc::OpcError>) -> Finding {
    let saved = match saved {
        Ok(saved) => saved,
        Err(error) => {
            return Finding {
                check: "round-trip",
                verdict: Verdict::Failed,
                detail: format!("saving it back refused: {error}"),
            }
        }
    };
    let reopened = match Package::open(saved) {
        Ok(reopened) => reopened,
        Err(error) => {
            return Finding {
                check: "round-trip",
                verdict: Verdict::Failed,
                detail: format!("the bytes we wrote do not reopen: {error}"),
            }
        }
    };

    let before: Vec<&str> = package.entries().iter().map(|e| e.name.as_str()).collect();
    let after: Vec<&str> = reopened.entries().iter().map(|e| e.name.as_str()).collect();
    if before != after {
        return Finding {
            check: "round-trip",
            verdict: Verdict::Failed,
            detail: format!(
                "the entry set or its order changed: {} entries in, {} out",
                before.len(),
                after.len()
            ),
        };
    }
    for (original, written) in package.entries().iter().zip(reopened.entries()) {
        if original.bytes() != written.bytes() {
            return Finding {
                check: "round-trip",
                verdict: Verdict::Failed,
                detail: format!(
                    "the decompressed payload of {} changed across a save that edited nothing",
                    original.name
                ),
            };
        }
    }
    Finding {
        check: "round-trip",
        verdict: Verdict::Held,
        detail: format!("{} entries, every payload byte-identical", before.len()),
    }
}

/// Whether a ZIP entry carries XML, by name — the rule `mjx-opc`'s `tree_roundtrip` suite uses.
fn is_xml_entry(name: &str) -> bool {
    name.ends_with(".xml") || name.ends_with(".rels") || name.ends_with(".vml")
}

/// The `mjx-opc --test tree_roundtrip` contract: every XML part survives the fidelity tree verbatim.
///
/// This is the check the corpus exists for. Every fixture in `tests/fixtures/` was written by this
/// project or by LibreOffice, so until a file Office wrote goes through it, the reader has only ever
/// been held to markup shaped the way our own writer shapes it.
fn fidelity_round_trip(package: &Package) -> Finding {
    let mut checked = 0usize;
    let mut lost = Vec::new();
    for entry in package.entries() {
        if !is_xml_entry(&entry.name) {
            continue;
        }
        let Some(original) = entry.bytes() else {
            lost.push(format!("{}: no materialized bytes after open", entry.name));
            continue;
        };
        checked += 1;
        match mjx_xml::fidelity::parse(original) {
            Err(error) => lost.push(format!("{}: does not parse: {error}", entry.name)),
            Ok(document) => {
                if mjx_xml::fidelity::serialize_to_vec(&document) != original {
                    lost.push(format!("{}: re-serialized to different bytes", entry.name));
                }
            }
        }
    }
    if lost.is_empty() {
        Finding {
            check: "xml tree",
            verdict: Verdict::Held,
            detail: format!("{checked} XML part(s) parsed and re-serialized byte-identically"),
        }
    } else {
        Finding {
            check: "xml tree",
            verdict: Verdict::Failed,
            detail: format!(
                "{} of {checked} XML part(s) did not survive:\n    {}",
                lost.len(),
                lost.join("\n    ")
            ),
        }
    }
}

/// The same byte-identity contract, asked of the **format model** rather than of the container.
///
/// `Package` alone proves the ZIP and the part table survive. Opening the file as a `Deck`,
/// `Document` or `Workbook` and saving it back proves the format crate's own laziness holds against
/// markup nobody here wrote — which is the part of the promise a hand-crafted fixture cannot test.
fn facade_round_trip(format: ArtefactFormat, package: &Package, bytes: &[u8]) -> Finding {
    let saved = match format {
        ArtefactFormat::Presentation => Deck::open(bytes).and_then(|deck| deck.save_unchecked()),
        ArtefactFormat::Document => {
            Document::open(bytes).and_then(|document| document.save_unchecked())
        }
        ArtefactFormat::Workbook => {
            Workbook::open(bytes).and_then(|workbook| workbook.save_unchecked())
        }
    };
    let saved = match saved {
        Ok(saved) => saved,
        Err(error) => {
            return Finding {
                check: "facade",
                verdict: Verdict::Failed,
                detail: format!("the facade could not open and re-save it: {error}"),
            }
        }
    };
    let reopened = match Package::open(&saved) {
        Ok(reopened) => reopened,
        Err(error) => {
            return Finding {
                check: "facade",
                verdict: Verdict::Failed,
                detail: format!("what the facade wrote does not reopen: {error}"),
            }
        }
    };
    let mut changed = Vec::new();
    for original in package.entries() {
        match reopened
            .entries()
            .iter()
            .find(|entry| entry.name == original.name)
        {
            None => changed.push(format!("{}: gone", original.name)),
            Some(written) if written.bytes() != original.bytes() => {
                changed.push(format!("{}: payload changed", original.name));
            }
            Some(_) => {}
        }
    }
    for written in reopened.entries() {
        if !package
            .entries()
            .iter()
            .any(|entry| entry.name == written.name)
        {
            changed.push(format!("{}: added", written.name));
        }
    }
    if changed.is_empty() {
        Finding {
            check: "facade",
            verdict: Verdict::Held,
            detail: format!(
                "opened and re-saved through the facade, {} entries unchanged",
                package.entries().len()
            ),
        }
    } else {
        Finding {
            check: "facade",
            verdict: Verdict::Failed,
            detail: format!(
                "{} entry/entries changed across an edit-free open and save:\n    {}",
                changed.len(),
                changed.join("\n    ")
            ),
        }
    }
}

/// A7b's package invariants, on the file as it arrived and on the bytes we wrote back.
///
/// A defect the file **arrived** with is reported and never failed: A7b's scope rule is that
/// validation refuses a package *we* edited into that state, and a real Office file with a dangling
/// `r:id` is a test case rather than a bug report. A defect that appears only in what we wrote is
/// the opposite, and fails.
fn package_invariants(package: &Package, saved: Result<&[u8], &mjx_opc::OpcError>) -> Finding {
    let arrived = package.validate().err().map(|defect| defect.to_string());
    let written = saved
        .ok()
        .and_then(|saved| Package::open(saved).ok())
        .and_then(|reopened| reopened.validate().err().map(|defect| defect.to_string()));

    match (arrived, written) {
        (None, None) => Finding {
            check: "package",
            verdict: Verdict::Held,
            detail: "every OPC invariant holds, before and after a save".to_owned(),
        },
        (Some(arrived), Some(written)) if arrived == written => Finding {
            check: "package",
            verdict: Verdict::Reported,
            detail: format!(
                "it arrived with a defect and re-saved carrying exactly that defect, which is what \
                 A7b's scope rule requires: {arrived}"
            ),
        },
        (arrived, written) => Finding {
            check: "package",
            verdict: Verdict::Failed,
            detail: format!(
                "saving changed the package's own verdict — arrived: {}; written: {}",
                arrived.as_deref().unwrap_or("no defect"),
                written.as_deref().unwrap_or("no defect")
            ),
        },
    }
}

/// A7c's child-order audit, over Office's own output.
///
/// This is the strongest check available that the generated `ChildOrder` tables say what Office
/// actually writes: every other input to the audit is markup this project authored from the same
/// tables. A defect here has two readings — our table is wrong, or the producer wrote out of
/// sequence — and both are worth stopping on, so it fails and the message says both.
fn child_order(label: &str, bytes: &[u8]) -> Finding {
    let report = mjx_schema_gate::audit_order_report(label, bytes);
    if !report.defects.is_empty() {
        return Finding {
            check: "child order",
            verdict: Verdict::Failed,
            detail: format!(
                "{} part(s) carry a child out of their complex type's xsd:sequence. Either the \
                 generated table is wrong about this type — which is what auditing a file Office \
                 wrote is for — or the producer wrote out of sequence:\n    {}",
                report.defects.len(),
                report.defects.join("\n    ")
            ),
        };
    }
    let required = mjx_schema_gate::parts_that_must_be_audited(label, bytes);
    let missed = report.missed(&required);
    if !missed.is_empty() {
        return Finding {
            check: "child order",
            verdict: Verdict::Failed,
            detail: format!(
                "{} part(s) are rooted in a schema whose child-order table is generated and the \
                 walk did not audit them, so `root_element` does not name their root — a codegen \
                 gap: {missed:?}",
                missed.len()
            ),
        };
    }
    let vacuous = report.vacuous();
    let detail = format!(
        "{} part(s) audited, {} required by the category tables",
        report.audited.len(),
        required.len()
    );
    if vacuous.is_empty() {
        Finding {
            check: "child order",
            verdict: Verdict::Held,
            detail,
        }
    } else {
        Finding {
            check: "child order",
            verdict: Verdict::Reported,
            detail: format!(
                "{detail}; {} audited vacuously — the tables knew the root's type and recognised \
                 none of its children: {:?}",
                vacuous.len(),
                vacuous.iter().map(|part| &part.name).collect::<Vec<_>>()
            ),
        }
    }
}

/// A1's schema gate, over markup this library did not write.
///
/// **Reported, never failed**, and the reason is the one A7b gives for package defects read the
/// other way round: a deviation here is a statement about the producer's markup. MJXOFF-103 measured
/// the case — Apache POI 5.5.1 writes an empty `<c:tx/>` that `dml-chart.xsd` rejects — and the
/// module documentation above records the one that will fire on essentially every Office workbook
/// and every Office chart, which is a defect of ours in the MCE/validation seam and not of the file.
/// Neither is a reason to redden a build over somebody else's file; both are reasons to print the
/// whole report.
fn schema_validity(label: &str, bytes: &[u8]) -> Finding {
    let Some(harness) = mjx_schema_gate::harness() else {
        return Finding {
            check: "schema",
            verdict: Verdict::Skipped,
            detail: "no References/ tree or no xmllint; MJX_REQUIRE_SCHEMA=1 makes that a failure"
                .to_owned(),
        };
    };
    let rows = mjx_schema_gate::inspect_deck(&harness, label, bytes, &[]);
    let validated = rows
        .iter()
        .filter(|row| row.outcome.validated_against().is_some())
        .count();
    let deviations: Vec<&mjx_schema_gate::PartRow> =
        rows.iter().filter(|row| row.outcome.is_failure()).collect();
    if deviations.is_empty() {
        Finding {
            check: "schema",
            verdict: Verdict::Held,
            detail: format!("{validated} part(s) validated against the ECMA-376 XSDs"),
        }
    } else {
        Finding {
            check: "schema",
            verdict: Verdict::Reported,
            detail: format!(
                "{validated} part(s) validated; {} deviated. A deviation in a file we did not write \
                 is not this library's failure — read it, then decide whether it is the producer's \
                 markup or the MCE/CT_Extension seam this module documents:\n{}",
                deviations.len(),
                deviations
                    .iter()
                    .map(|row| format!("    {}: {}", row.name, row.outcome.describe()))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        }
    }
}
