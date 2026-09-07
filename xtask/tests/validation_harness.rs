//! Every artefact `validation-artefacts` produces, held to the three gates a machine *can* answer
//! before a human ever opens one (MJXOFF-122).
//!
//! # What this file asserts, and why each assertion is here
//!
//! 1. **The generator runs, from the built binary, and writes what it said it wrote.** The binary is
//!    invoked through `CARGO_BIN_EXE_xtask` rather than through a nested `cargo run`, so the run
//!    this file measures is the one a developer gets.
//! 2. **Schema validity, package validity and child order** over every artefact, with the ordering
//!    audit's per-part `elements_visited` counts printed — *an audit that visits nothing passes
//!    vacuously*, so the counts are asserted, not merely produced.
//! 3. **Skips are named.** `PartOutcome::SkippedPreservedForeign` is indistinguishable from passing
//!    at a glance, so the labels skipped across the whole generated set are pinned: a *new* skip
//!    fails here rather than quietly widening what the gate tolerates.
//! 4. **Determinism.** Two runs into two directories produce byte-identical files. Without this the
//!    three-language comparison in the two bindings is meaningless and every future diff is noise.
//! 5. **The edit variants skipped by name**, and the run said so on stdout — never a silent absence.
//! 6. **One LibreOffice conversion per format**, as a canary. One conversion of the artefacts this
//!    harness produced, deliberately not a sweep: a conversion proves a file opens and proves
//!    nothing about what it looks like. **No verdict is recorded anywhere in this file.**
//!
//! # The trap
//!
//! Every clause here is phrased so that it would fail if the work were not done. "Every artefact is
//! schema-valid" is green when there are no artefacts, so the artefact count is pinned to the
//! catalogue first, and the catalogue's own size is pinned below.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use mjx_opc::Package;
use mjx_schema_gate::{
    assert_authored_deck_is_schema_valid, audit_deck_order, audit_order_report, harness,
    inspect_deck, outcome_table, PartOutcome,
};
use xtask::validation::{corpus_directory, original_for, Area, Variant, AREAS};

/// Where this file's artefacts go. Its own directory, so the determinism test's two runs and this
/// one cannot race each other inside one test binary.
fn output_directory(tag: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../target")
        .join(format!("validation-artefacts-{tag}"))
}

/// Runs the generator into a fresh directory and returns `(directory, stdout)`.
fn generate(tag: &str) -> (PathBuf, String) {
    let directory = output_directory(tag);
    // A stale artefact from an earlier run would let "every artefact is named by an entry" pass on
    // yesterday's output, so the directory is emptied first.
    let _ = std::fs::remove_dir_all(&directory);
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("validation-artefacts")
        .arg("--out")
        .arg(&directory)
        .output()
        .expect("running the xtask binary");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "validation-artefacts failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (directory, stdout)
}

/// The one run every gate below reads, produced once for the whole test binary.
fn generated() -> &'static (PathBuf, String) {
    static ONCE: OnceLock<(PathBuf, String)> = OnceLock::new();
    ONCE.get_or_init(|| generate("harness"))
}

/// Every file in a directory, sorted.
fn files_in(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(directory)
        .unwrap_or_else(|e| panic!("reading {}: {e}", directory.display()))
        .map(|entry| {
            entry
                .expect("a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

// -------------------------------------------------------------------------------------------
// The catalogue itself
// -------------------------------------------------------------------------------------------

#[test]
fn the_catalogue_is_well_formed_and_not_empty() {
    // A floor, not a guess about size: the catalogue has eight PresentationML areas and six each of
    // WordprocessingML and SpreadsheetML today (MJXOFF-128 added `V-PPTX-07` and `V-PPTX-08`), and a
    // change that empties it would otherwise make every gate below pass vacuously.
    assert!(
        AREAS.len() >= 20,
        "the catalogue has shrunk to {} areas; every gate in this file weakens with it",
        AREAS.len()
    );

    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for area in AREAS {
        assert!(ids.insert(area.id), "duplicate entry id {}", area.id);
        let expected_prefix = format!("V-{}-", area.format.extension().to_ascii_uppercase());
        assert!(
            area.id.starts_with(&expected_prefix),
            "{} is a {} area, so its id must start with {expected_prefix}",
            area.id,
            area.format.extension()
        );
        let number = area.number();
        assert_eq!(
            number.len(),
            2,
            "{}'s position must be zero-padded to two digits, not {number:?}",
            area.id
        );
        assert!(
            number.chars().all(|c| c.is_ascii_digit()),
            "{}'s position must be digits, not {number:?}",
            area.id
        );
        for variant in Variant::all() {
            assert!(
                names.insert(area.artefact_name(variant)),
                "two areas want the same artefact name: {}",
                area.artefact_name(variant)
            );
        }
    }
}

// -------------------------------------------------------------------------------------------
// What an artefact inherits from the file it was edited from
// -------------------------------------------------------------------------------------------

/// The defects an area's Office-authored original **arrived with**.
///
/// # Why this exists (MJXOFF-130)
///
/// An `authored` artefact is ours to the last byte and nothing in it is excused. An `edited` one is
/// mostly *somebody else's file* — this library opened an original, changed one thing, and re-emitted
/// every untouched part verbatim, which is the whole promise. Holding that artefact to a
/// tolerance-free gate faults **the producer's markup**, and A7b's scope rule says the opposite in
/// as many words: a file that arrives with a defect must still open and re-save unchanged.
///
/// Measured, not hypothesised: with `sample.xlsx` standing in the corpus slot,
/// `every_generated_artefact_is_schema_valid_and_in_child_order` failed on
/// `v-xlsx-02-edited.xlsx` for a `workbookPr@dateCompatibility` **LibreOffice** wrote — a deviation
/// `crates/mjx-schema-gate/src/tolerances.rs` already records for that fixture, arriving here
/// through a path that consults no tolerance list.
///
/// So the rule for an edited artefact is *no **new** defect*: everything the original already had is
/// subtracted, and anything left is ours.
#[derive(Default)]
struct Inherited {
    /// Schema deviations, as `part: outcome` lines.
    schema: BTreeSet<String>,
    /// The package defect the original arrived with, if any.
    package: Option<String>,
    /// Child-order defects, with a fixed label so the two reports are comparable.
    order: BTreeSet<String>,
}

/// A label both halves of a comparison use, so the messages differ only where the defects do.
const INHERITED_LABEL: &str = "the Office-authored original";

impl Inherited {
    /// What this area's original arrived with — empty for an authored artefact, and empty when the
    /// corpus has no original for the area.
    fn of(area: &Area, variant: Variant) -> Self {
        if variant != Variant::Edited {
            return Self::default();
        }
        let Some(original) = original_for(area).expect("reading the Office-authored corpus") else {
            return Self::default();
        };
        Self {
            schema: harness()
                .map(|harness| {
                    inspect_deck(&harness, INHERITED_LABEL, &original, &[])
                        .iter()
                        .filter(|row| row.outcome.is_failure())
                        .map(|row| format!("{}: {}", row.name, row.outcome.describe()))
                        .collect()
                })
                .unwrap_or_default(),
            package: Package::open(&original)
                .ok()
                .and_then(|package| package.validate().err())
                .map(|defect| defect.to_string()),
            order: audit_order_report(INHERITED_LABEL, &original)
                .defects
                .into_iter()
                .collect(),
        }
    }
}

/// How many areas the corpus holds an original for — the number of `edited` artefacts a run writes.
///
/// Every count in this file is stated against this rather than against `AREAS.len()` alone. The
/// corpus was empty when MJXOFF-122 wrote these gates, and a bare `AREAS.len()` would have started
/// failing the day the first Office-authored original landed: a gate that breaks on the work it is
/// waiting for.
fn areas_with_an_original() -> usize {
    AREAS
        .iter()
        .filter(|area| {
            original_for(area)
                .expect("reading the Office-authored corpus")
                .is_some()
        })
        .count()
}

/// How many artefacts a run writes: one per area, plus one per area with an original.
fn expected_artefacts() -> usize {
    AREAS.len() + areas_with_an_original()
}

// -------------------------------------------------------------------------------------------
// The three machine gates
// -------------------------------------------------------------------------------------------

#[test]
fn every_generated_artefact_is_schema_valid_and_in_child_order() {
    let (directory, _) = generated();
    let mut checked = 0usize;
    for area in AREAS {
        for variant in Variant::all() {
            let name = area.artefact_name(variant);
            let path = directory.join(&name);
            if !path.is_file() {
                continue;
            }
            let bytes = std::fs::read(&path).expect("reading a generated artefact");
            let inherited = Inherited::of(area, variant);
            if inherited.schema.is_empty() && inherited.order.is_empty() {
                // Both halves: the child-order audit always, `xmllint` when `References/` is
                // present. Nothing is excused, which is right for every byte we wrote.
                assert_authored_deck_is_schema_valid(&name, &bytes);
            } else {
                assert_no_new_defect(&name, &bytes, &inherited);
            }
            checked += 1;
        }
    }
    assert_eq!(
        checked,
        expected_artefacts(),
        "every area must contribute its authored artefact to the schema gate, and every area with \
         an Office-authored original its edited one too"
    );
    println!("schema gate: {checked} generated artefact(s) validated");
}

/// Holds an *edited* artefact to "no **new** defect", subtracting what its original arrived with.
///
/// # Panics
/// On a schema deviation or an ordering defect the original did not already have.
fn assert_no_new_defect(name: &str, bytes: &[u8], inherited: &Inherited) {
    let order = audit_order_report(INHERITED_LABEL, bytes);
    let new_order: Vec<&String> = order
        .defects
        .iter()
        .filter(|defect| !inherited.order.contains(*defect))
        .collect();
    assert!(
        new_order.is_empty(),
        "{name}: editing introduced {} child-order defect(s) the Office-authored original did not \
         have:\n{}",
        new_order.len(),
        new_order
            .iter()
            .map(|defect| format!("  {defect}"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let Some(harness) = harness() else { return };
    let rows = inspect_deck(&harness, INHERITED_LABEL, bytes, &[]);
    let new_schema: Vec<String> = rows
        .iter()
        .filter(|row| row.outcome.is_failure())
        .map(|row| format!("{}: {}", row.name, row.outcome.describe()))
        .filter(|line| !inherited.schema.contains(line))
        .collect();
    assert!(
        new_schema.is_empty(),
        "{name}: editing introduced {} schema deviation(s) the Office-authored original did not \
         have:\n{}",
        new_schema.len(),
        new_schema.join("\n")
    );
    println!(
        "schema gate: {name} adds no defect to the {} its original arrived with",
        inherited.schema.len() + inherited.order.len()
    );
}

#[test]
fn every_generated_artefact_is_a_valid_package() {
    let (directory, _) = generated();
    let mut checked = 0usize;
    for area in AREAS {
        for variant in Variant::all() {
            let name = area.artefact_name(variant);
            let path = directory.join(&name);
            if !path.is_file() {
                continue;
            }
            let bytes = std::fs::read(&path).expect("reading a generated artefact");
            let package = Package::open(&bytes).unwrap_or_else(|e| panic!("{name}: opening: {e}"));
            // The same rule as the schema gate: an edited artefact is mostly its original's bytes,
            // and a package defect the original *arrived* with is not one this library introduced.
            let inherited = Inherited::of(area, variant).package;
            let found = package.validate().err().map(|defect| defect.to_string());
            assert!(
                found.is_none() || found == inherited,
                "{name}: package invariant: {}",
                found.unwrap_or_default()
            );
            checked += 1;
        }
    }
    assert_eq!(checked, expected_artefacts());
    println!("package validator: {checked} generated artefact(s) validated");
}

#[test]
fn the_ordering_audit_visits_real_structure_in_every_artefact() {
    let (directory, _) = generated();
    let mut audited_parts = 0usize;
    for area in AREAS {
        for variant in Variant::all() {
            let name = area.artefact_name(variant);
            let path = directory.join(&name);
            if !path.is_file() {
                continue;
            }
            let bytes = std::fs::read(&path).expect("reading a generated artefact");
            // `audit_deck_order` panics on the first defect, which is right for markup we authored
            // and wrong for an original's; `assert_no_new_defect` above owns that comparison, so
            // here an inherited defect is stepped over rather than re-raised.
            let inherited = Inherited::of(area, variant).order;
            let report = audit_order_report(INHERITED_LABEL, &bytes);
            let audited = if report.defects.iter().all(|d| inherited.contains(d)) {
                report.audited
            } else {
                audit_deck_order(&name, &bytes)
            };
            assert!(
                !audited.is_empty(),
                "{name}: the ordering audit found no part whose root the tables know"
            );
            for part in &audited {
                assert!(
                    part.elements_visited >= part.floor(),
                    "{name} {}: the audit visited {} element(s) with a floor of {} — an audit that \
                     visits nothing passes vacuously",
                    part.name,
                    part.elements_visited,
                    part.floor()
                );
                println!(
                    "order  {name:<28} {:<48} visited {:>5}  root children {:>3}",
                    part.name, part.elements_visited, part.root_child_elements
                );
            }
            audited_parts += audited.len();
        }
    }
    // A floor over the whole set rather than per part: the per-part floor above is what makes each
    // audit non-vacuous, and this is what makes *the sweep* non-vacuous.
    assert!(
        audited_parts >= AREAS.len() * 2,
        "only {audited_parts} part(s) were audited across {} artefacts",
        AREAS.len()
    );
    println!("ordering audit: {audited_parts} part(s) audited across the generated set");
}

/// Every `SkippedPreservedForeign` label the generated set produces, pinned.
///
/// A skip is indistinguishable from a pass at a glance, so the *set* is asserted rather than the
/// count: a part that starts skipping under a label not listed here fails, which is the only way a
/// new silent skip becomes visible.
const EXPECTED_FOREIGN_SKIPS: &[&str] = &["a VML drawing part"];

#[test]
fn every_skip_in_the_generated_set_is_reported_by_name() {
    let Some(harness) = harness() else {
        println!("schema harness unavailable — the skip report needs `References/` and `xmllint`");
        return;
    };
    let (directory, _) = generated();
    let mut skipped_labels = BTreeSet::new();
    let mut binary_skips = 0usize;
    for area in AREAS {
        for variant in Variant::all() {
            let name = area.artefact_name(variant);
            let path = directory.join(&name);
            if !path.is_file() {
                continue;
            }
            let bytes = std::fs::read(&path).expect("reading a generated artefact");
            let rows = inspect_deck(&harness, &name, &bytes, &[]);
            print!("{}", outcome_table(&name, &rows));
            for row in &rows {
                match &row.outcome {
                    PartOutcome::SkippedPreservedForeign { label, .. } => {
                        skipped_labels.insert(*label);
                    }
                    PartOutcome::SkippedBinary(_) => binary_skips += 1,
                    PartOutcome::Uncategorised { namespace } => panic!(
                        "{name} {}: root namespace {namespace:?} is on no list",
                        row.name
                    ),
                    _ => {}
                }
            }
        }
    }
    let expected: BTreeSet<&str> = EXPECTED_FOREIGN_SKIPS.iter().copied().collect();
    assert!(
        skipped_labels.is_subset(&expected),
        "a part in the generated set skipped under a label this file does not pin: {:?}",
        skipped_labels.difference(&expected).collect::<Vec<_>>()
    );
    println!(
        "skips: preserved-foreign {skipped_labels:?}, binary payloads {binary_skips} — all named"
    );
}

// -------------------------------------------------------------------------------------------
// Determinism
// -------------------------------------------------------------------------------------------

#[test]
fn two_runs_produce_byte_identical_artefacts() {
    let (first, _) = generate("determinism-a");
    let (second, _) = generate("determinism-b");
    let names = files_in(&first);
    assert_eq!(
        names,
        files_in(&second),
        "the two runs wrote different sets of files"
    );
    // One artefact per area, plus one more for every area the Office-authored corpus has an
    // original for. Stated as a *derived* count rather than as `AREAS.len()`: the corpus was empty
    // when this case was written and the plain count would have started failing the day MJXOFF-130's
    // first file landed — a gate that breaks on the work it is waiting for.
    let originals = AREAS
        .iter()
        .filter(|area| {
            original_for(area)
                .expect("reading the Office-authored corpus")
                .is_some()
        })
        .count();
    assert_eq!(
        names.len(),
        AREAS.len() + originals,
        "a run must write one artefact per area, and a second for each of the {originals} area(s)          the Office-authored corpus holds an original for"
    );
    for name in &names {
        let a = std::fs::read(first.join(name)).expect("first run");
        let b = std::fs::read(second.join(name)).expect("second run");
        // Reported as the first differing offset rather than as `assert_eq!` on two `Vec<u8>`s: a
        // package is tens of kilobytes and the default message prints both of them, which buries
        // the one fact a reader needs. Measured — the determinism mutation printed 40 KB of bytes.
        if a != b {
            let offset = a
                .iter()
                .zip(&b)
                .position(|(left, right)| left != right)
                .unwrap_or_else(|| a.len().min(b.len()));
            panic!(
                "{name} differs between two runs of the same command — the three-language \
                 comparison is meaningless without byte-identity. First difference at byte \
                 {offset} of {} / {}",
                a.len(),
                b.len()
            );
        }
    }
    println!(
        "determinism: {} artefact(s) byte-identical across two runs",
        names.len()
    );
}

// -------------------------------------------------------------------------------------------
// The edit variants, and what the run said about them
// -------------------------------------------------------------------------------------------

#[test]
fn an_edit_variant_without_an_original_skips_by_name() {
    let (directory, stdout) = generated();
    let mut absent = 0usize;
    for area in AREAS {
        let has_original = original_for(area)
            .expect("reading the Office-authored corpus")
            .is_some();
        let artefact = directory.join(area.artefact_name(Variant::Edited));
        assert_eq!(
            artefact.is_file(),
            has_original,
            "{}: the edited artefact must exist exactly when {} does",
            area.id,
            xtask::validation::original_path(area).display()
        );
        if !has_original {
            absent += 1;
            assert!(
                stdout.contains(area.id),
                "{} skipped without naming itself on stdout — a silent skip is the failure mode \
                 this whole harness is written against",
                area.id
            );
        }
    }
    if absent > 0 {
        assert!(
            stdout.contains("SKIPPED"),
            "areas were skipped and the run did not say so"
        );
        println!(
            "edit variants: {absent} area(s) skipped by name; the corpus at {} is MJXOFF-130's",
            corpus_directory().display()
        );
    }
}

// -------------------------------------------------------------------------------------------
// The LibreOffice canary — one conversion per format, never a sweep, never a verdict
// -------------------------------------------------------------------------------------------

/// Locates a LibreOffice command, the same way `crates/*/tests/office_open.rs` does.
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
    [
        "/usr/bin/soffice",
        "/usr/bin/libreoffice",
        "/Applications/LibreOffice.app/Contents/MacOS/soffice",
        "/opt/libreoffice/program/soffice",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|p| p.is_file())
}

#[test]
fn one_artefact_per_format_opens_in_libreoffice() {
    let Some(soffice) = find_soffice() else {
        assert!(
            std::env::var_os("MJX_REQUIRE_SOFFICE").is_none(),
            "MJX_REQUIRE_SOFFICE is set but no soffice/libreoffice binary was found"
        );
        println!("no soffice/libreoffice binary — the canary skipped");
        return;
    };
    let (directory, _) = generated();
    let work = output_directory("canary-pdf");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("canary working directory");

    // **One artefact per format, deliberately not a sweep.** A conversion says a file opens; it says
    // nothing about what it looks like, and sweeping the whole set through LibreOffice would buy a
    // longer run rather than a stronger claim.
    let mut converted = 0usize;
    for format in xtask::validation::ArtefactFormat::all() {
        let area = AREAS
            .iter()
            .find(|area| area.format == format)
            .expect("every format has at least one area");
        let name = area.artefact_name(Variant::Authored);
        let source = directory.join(&name);
        let status = Command::new(&soffice)
            .args([
                "--headless",
                "--norestore",
                "--invisible",
                "--convert-to",
                "pdf",
                "--outdir",
            ])
            .arg(&work)
            .arg(&source)
            .env("HOME", &work)
            .output()
            .expect("running soffice");
        // soffice's exit code is unreliable; the produced PDF is the signal, which is the rule
        // `crates/mjx-pptx/tests/office_open.rs` already established.
        let pdf = work.join(format!(
            "{}.pdf",
            name.trim_end_matches(&format!(".{}", format.extension()))
        ));
        assert!(
            pdf.is_file(),
            "{name}: LibreOffice produced no PDF (status {:?})\n{}",
            status.status,
            String::from_utf8_lossy(&status.stderr)
        );
        let head = std::fs::read(&pdf).expect("reading the produced PDF");
        assert!(
            head.starts_with(b"%PDF-"),
            "{name}: the produced file is not a PDF"
        );
        println!(
            "canary: {name} opened in LibreOffice ({} bytes of PDF)",
            head.len()
        );
        converted += 1;
    }
    assert_eq!(converted, 3, "one conversion per format");
    let _ = std::fs::remove_dir_all(&work);
}
