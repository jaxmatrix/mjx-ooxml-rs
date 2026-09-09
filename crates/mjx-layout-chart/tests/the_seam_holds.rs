//! **A chart engine reads no package, resolves no outline, and knows no format** — held here,
//! because rank 3.55 cannot hold any of the three.
//!
//! # What rank 3.55 buys, and what it does not
//!
//! `xtask/tests/layering.rs` puts this crate one step *below* the three box models, and that is the
//! whole design: at 3.6 an edge between any two box models is sideways and refused, so a chart
//! engine placed beside them would be reachable from none of them. At 3.55 all three reach it and it
//! reaches none of them, and "built once" is a property of the graph. The rank also stops the format
//! tier (3.0), `mjx-chart` (2.2), `mjx-session` (3.5) and `mjx-layout` (1.6) reaching *it*.
//!
//! It buys **nothing at all in the other direction.** The layering gate refuses an edge only when it
//! points up or sideways, so `mjx-layout-chart -> mjx-pptx` (3.0), `-> mjx-geometry` (2.5) and
//! `-> mjx-scene` (1.7) are legal downward edges and always will be. So the three properties this
//! crate has to hold are held here:
//!
//! 1. **It never opens a package.** The seam is *bytes*: `chart_part_bytes` is already public on all
//!    three format surfaces, and this crate parses the part. That is what makes the refusal
//!    affordable, and it is why the format crates are refused in **both** sections rather than
//!    permitted in `[dev-dependencies]` the way `mjx-scene-xlsx` permits `mjx-xlsx`. A suite here
//!    that opened a `.pptx` to get at a chart part would be a suite proving the engine can do the
//!    one thing its position exists to stop it doing — and it would prove it in the crate whose
//!    whole value is that a chart is read the same way from three places.
//! 2. **It resolves no outline.** A pie slice reaches a painter as a `GeometryRef` the host issued,
//!    exactly as a preset shape does from `mjx-layout-pptx`, so `mjx-geometry` is refused for the
//!    reason `crates/mjx-layout-pptx/tests/the_seam_holds.rs` refuses it: a layout engine says
//!    *this shape, at this size* and stops.
//! 3. **It never builds a display list and never paints.** `mjx-scene` and `mjx-paint` are absent
//!    for the reason every box model's gate gives — a layout stage that built a display list would
//!    have merged two stages the architecture separates on purpose.
//!
//! # And the one the layering gate genuinely does refuse, restated
//!
//! `mjx-layout-chart -> mjx-layout-pptx` is refused by rank, because 3.6 is above 3.55. It is
//! repeated below anyway, because the reason is a *design* one rather than an arithmetic one: the
//! day somebody decides the chart engine belongs at 3.7 "so it can reach the box models", the gate
//! would go quiet and the property would not.

use std::path::{Path, PathBuf};

/// How many `.rs` files `src/` holds. Exact rather than a floor, so adding a module is a deliberate
/// act that touches this number.
const SOURCE_FILE_COUNT: usize = 12;

/// Every crate this one may name in `[dependencies]`, exactly.
const PERMITTED_DEPENDENCIES: &[&str] = &[
    "mjx-chart",
    "mjx-dml",
    "mjx-layout",
    "mjx-ooxml-core",
    "mjx-ooxml-types",
    "mjx-xml",
    "thiserror",
];

/// The crates this one must never name in **either** dependency section.
///
/// The three format crates are here rather than permitted below, which is the difference between
/// this gate and `mjx-scene-xlsx`'s: that crate legitimately opens a workbook in a test, and this
/// one must not open anything. A suite that reached a chart part through `mjx_pptx::Presentation`
/// would be exercising one host's route into the engine, in the crate whose value is that all three
/// take the same one.
const FORBIDDEN: &[&str] = &[
    "mjx-pptx",
    "mjx-docx",
    "mjx-xlsx",
    "mjx-opc",
    "mjx-sml",
    "mjx-layout-pptx",
    "mjx-layout-docx",
    "mjx-layout-xlsx",
    "mjx-scene",
    "mjx-scene-pptx",
    "mjx-scene-xlsx",
    "mjx-geometry",
    "mjx-session",
    "mjx-view",
    "mjx-ooxml",
    "mjx-paint",
    "mjx-render-oracle",
    "mjx-reference-pack",
    "mjx-canvas-harness",
];

/// The identifiers the source must never name, whatever the manifest says.
const FORBIDDEN_IDENTIFIERS: &[&str] = &[
    "mjx_pptx",
    "mjx_docx",
    "mjx_xlsx",
    "mjx_opc",
    "mjx_geometry",
    "mjx_scene",
    "mjx_paint",
    "mjx_view",
    "mjx_session",
    "mjx_layout_pptx",
    "mjx_layout_docx",
    "mjx_layout_xlsx",
];

fn manifest() -> String {
    read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

/// Every `.rs` file under `directory`, recursively, sorted.
fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = std::fs::read_dir(&current)
            .unwrap_or_else(|error| panic!("reading {}: {error}", current.display()));
        for entry in entries {
            let entry = entry.expect("a directory entry");
            let path = entry.path();
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

/// The lines of `section`, up to the next section header.
fn section_lines<'a>(text: &'a str, section: &str) -> Vec<&'a str> {
    let mut lines = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == section;
            continue;
        }
        if inside && !trimmed.is_empty() && !trimmed.starts_with('#') {
            lines.push(trimmed);
        }
    }
    lines
}

/// The crate a manifest line names, or `None` for a line that names none.
fn dependency_name(line: &str) -> Option<&str> {
    let name = line.split(['=', '.']).next()?.trim();
    (!name.is_empty()).then_some(name)
}

#[test]
fn the_dependency_list_is_exactly_what_it_should_be() {
    let manifest = manifest();
    let mut named: Vec<&str> = section_lines(&manifest, "[dependencies]")
        .into_iter()
        .filter_map(dependency_name)
        .collect();
    named.sort_unstable();
    named.dedup();

    assert_eq!(
        named, PERMITTED_DEPENDENCIES,
        "`mjx-layout-chart`'s `[dependencies]` changed. Every entry has a written reason in the \
         manifest and a rank below 3.55; adding one is a decision, and this list is where it is \
         recorded."
    );
}

#[test]
fn the_engine_names_no_format_crate_in_either_section() {
    let manifest = manifest();
    for forbidden in FORBIDDEN {
        for section in ["[dependencies]", "[dev-dependencies]"] {
            let named = section_lines(&manifest, section)
                .into_iter()
                .filter_map(dependency_name)
                .any(|name| name == *forbidden);
            assert!(
                !named,
                "`mjx-layout-chart` names `{forbidden}` in `{section}`. See this file's own \
                 documentation for which of the three seams that breaks — and note that the format \
                 crates are refused in `[dev-dependencies]` too, deliberately: a suite that reached \
                 a chart part through one host would be exercising that host's route into an engine \
                 whose entire value is that all three take the same one."
            );
        }
    }
}

#[test]
fn the_source_names_no_document_no_outline_and_no_painter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files = rust_files(&root);
    assert_eq!(
        files.len(),
        SOURCE_FILE_COUNT,
        "`src/` holds {} `.rs` files rather than {SOURCE_FILE_COUNT}. Adding a module is fine; \
         update this number so that it stays a deliberate act.",
        files.len()
    );

    for file in &files {
        let code = read(file);
        // The documentation names `mjx_pptx::Presentation` and `mjx-layout-xlsx`'s number-format
        // module deliberately — the first is where a host gets the bytes, the second is the finding
        // about why `c:numFmt` is not applied — so only *code* is scanned.
        let stripped: String = code
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for forbidden in FORBIDDEN_IDENTIFIERS {
            assert!(
                !stripped.contains(forbidden),
                "{} names `{forbidden}` outside a comment. See this file's own documentation.",
                file.display()
            );
        }
    }
}

/// The three box models are the *consumers*, and this crate must never name one.
///
/// Refused by rank as well — 3.6 is above 3.55 — and repeated because the arithmetic could change
/// and the design could not. MJXOFF-176 hit the mirror image of this: it was told to consume
/// `mjx-layout-pptx`'s shape layout from a crate that could not reach it, and reported the ticket
/// unfollowable rather than moving the layout somewhere it would fit.
#[test]
fn the_engine_never_names_one_of_its_three_consumers() {
    let manifest = manifest();
    for section in ["[dependencies]", "[dev-dependencies]"] {
        for consumer in ["mjx-layout-pptx", "mjx-layout-docx", "mjx-layout-xlsx"] {
            let named = section_lines(&manifest, section)
                .into_iter()
                .filter_map(dependency_name)
                .any(|name| name == consumer);
            assert!(
                !named,
                "`mjx-layout-chart` names `{consumer}` in `{section}`. A chart engine that could \
                 name one box model would be a chart engine the other two could not have, which is \
                 exactly the duplication this crate exists to prevent."
            );
        }
    }
}

/// The crate compiles with no format crate present, which is the strongest form of the first seam.
///
/// This is a statement about the manifest rather than a second build: `[dependencies]` is checked
/// exactly above, and none of the seven entries reaches a format crate transitively — `mjx-chart`
/// depends on `mjx-sml` and `mjx-dml`, both shared markup, and stops there. Naming that here is what
/// makes the reader's question — *could this pull in a `.pptx` reader by accident?* — answerable
/// without opening seven manifests.
#[test]
fn no_permitted_dependency_reaches_a_format_crate() {
    for crate_name in PERMITTED_DEPENDENCIES {
        if !crate_name.starts_with("mjx-") {
            continue;
        }
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(crate_name)
            .join("Cargo.toml");
        if !path.exists() {
            continue;
        }
        let manifest = read(&path);
        for line in section_lines(&manifest, "[dependencies]") {
            let Some(name) = dependency_name(line) else {
                continue;
            };
            assert!(
                !matches!(name, "mjx-pptx" | "mjx-docx" | "mjx-xlsx"),
                "`{crate_name}` depends on `{name}`, so `mjx-layout-chart` reaches a format crate \
                 transitively. The seam is that the engine is handed bytes; a transitive edge would \
                 make that a convention rather than a fact."
            );
        }
    }
}
