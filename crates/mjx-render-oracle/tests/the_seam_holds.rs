//! **The gate that stands in for a rank this crate does not have** (audit pass 10, G1).
//!
//! # What was and was not enforced before this file
//!
//! `xtask/tests/layering.rs` holds two things about this crate, and both are about edges pointing
//! *into* it: nothing with a rank may reach it in either dependency section, and the two crates
//! that legitimately do — `mjx-reference-pack` and `mjx-canvas-harness` — are asserted to still
//! exist. It holds **nothing whatever** about what this crate depends on, because its rule is a
//! comparison of two ranks and this crate has none.
//!
//! `CLAUDE.md` nevertheless described the property: *"it is the rendering path with no document
//! anywhere in it"*. That sentence was an overstatement, and audit pass 10 corrected it rather than
//! deleting it, because the truth is worth stating precisely:
//!
//! * this crate **does** name `mjx-geometry`, `mjx-dml` and `mjx-ooxml-types`, and it names them
//!   for exactly one reason — the `preset-star` specimen, which exists so that `placeholders == 0`
//!   is asserted about the *real* `PresetGeometryProvider` rather than about a page that happens to
//!   contain no shapes;
//! * and it names **no format crate, no facade and no packaging beyond `ST_PresetShapeType`**,
//!   which is the property that lets a fidelity oracle be built before any format renders.
//!
//! Read the two together and the shape is clear: the oracle knows what a *shape* is and does not
//! know what a *file* is. The pack above it adds the format tier; the harness beside it adds
//! neither. This file is what makes the second half a fact instead of a description.
//!
//! # Why the `mjx-geometry` cost is asserted rather than merely allowed
//!
//! `crates/mjx-paint/tests/the_seam_holds.rs` **forbids** `mjx-geometry` outright, and that is not
//! a contradiction: a painter reads meshes and a provenance, and who decided what a `roundRect`
//! looks like is not its business. An oracle's business is precisely to check that decision. So the
//! edge is legal here and it is spent in one place, and the manifest assertion below is exact so
//! that the cost stays visible where a reader would look for it — a second DrawingML dependency
//! would change this assertion rather than passing silently.
//!
//! # Proved by mutation
//!
//! * Adding `mjx-pptx.workspace = true` to `Cargo.toml` → the manifest assertion fails, naming it.
//! * Adding `use mjx_pptx::Presentation;` to `src/specimen.rs` → the source scan fails, naming the
//!   file and the line.
//! * The shared scanner's two instrument tests are compiled into this binary, so a scanner that had
//!   silently stopped seeing a category is caught here rather than trusted from another crate.

use std::path::{Path, PathBuf};

#[path = "../../mjx-paint/tests/support/manifest.rs"]
mod manifest;
use manifest::{lines_naming, read, rust_files, workspace_dependencies};

/// How many `.rs` files `src/` holds, counted recursively.
const SOURCE_FILE_COUNT: usize = 16;

/// Everything an oracle that has never opened a file may not name.
///
/// The list is deliberately **shorter** than `mjx-canvas-harness`'s by three entries — `mjx-dml`,
/// `mjx-geometry` and `mjx-ooxml-types` are absent, because this crate legitimately builds an
/// `a:prstGeom` and asks the real provider to resolve it. Everything that would make a *document*
/// reachable is here.
const FORBIDDEN: &[&str] = &[
    // The format tier: the three crates that know what a package is.
    "mjx_pptx",
    "mjx-pptx",
    "mjx_docx",
    "mjx-docx",
    "mjx_xlsx",
    "mjx-xlsx",
    // The facade, spelled with its possible followers rather than bare: a bare `mjx_ooxml` is a
    // prefix of `mjx_ooxml_types` and `mjx_ooxml_core`, both of which this crate may name, and
    // would report every legitimate line. `mjx-paint`'s gate could not see `use mjx_ooxml::Deck;`
    // for the opposite version of this mistake until MJXOFF-164.
    "mjx_ooxml::",
    "mjx_ooxml;",
    "mjx_ooxml}",
    "mjx-ooxml\"",
    // The packaging and compatibility tier. `mjx-ooxml-types` is the one exception and is checked
    // by the manifest assertion instead, since a `PresetShapeType` is a name and not a package.
    "mjx_opc",
    "mjx-opc",
    "mjx_mce",
    "mjx-mce",
    "mjx_xml",
    "mjx-xml",
    // The rest of shared markup. `mjx-dml` is allowed and these are not: a chart, a workbook and a
    // legacy drawing are all *content*, and an oracle that rendered one would be an oracle whose
    // specimens came from a document.
    "mjx_sml",
    "mjx-sml",
    "mjx_chart",
    "mjx-chart",
    "mjx_omml",
    "mjx-omml",
    "mjx_vml",
    "mjx-vml",
    // The crate above this one. Nothing may depend on the reference pack in either section, and a
    // `use` of it here would be the first step towards declaring the edge.
    "mjx_reference_pack",
    "mjx-reference-pack",
    "mjx_canvas_harness",
    "mjx-canvas-harness",
];

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

#[test]
fn no_document_is_named_anywhere_in_this_crates_source() {
    // The count first, so a scan that read nothing fails as a count rather than passing as an
    // empty result.
    let files = rust_files(&source_root());
    assert_eq!(
        files.len(),
        SOURCE_FILE_COUNT,
        "the walk found {} source file(s); it is recursive and this count is exact, so either a \
         module was added or the walk stopped early: {files:?}",
        files.len()
    );

    let offences = lines_naming(&source_root(), FORBIDDEN);
    assert!(
        offences.is_empty(),
        "the fidelity oracle knows what a shape is and does not know what a file is, and these \
         lines say otherwise:\n{}\n\
         A specimen is a `FragmentTree` built by hand, so a baseline is not also a test of a box \
         model that does not exist yet. A specimen read out of a `.pptx` would make the oracle \
         unrunnable until the PowerPoint view exists — and `mjx-reference-pack`, which sits above \
         this crate for exactly that reason, is where the format tier belongs.",
        offences.join("\n")
    );

    let scanned: usize = files.iter().map(|path| read(path).lines().count()).sum();
    assert!(
        scanned > 3_000,
        "only {scanned} lines were scanned, which cannot be this crate"
    );
}

#[test]
fn the_manifest_declares_exactly_the_dependencies_the_seam_allows() {
    let manifest = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"));
    assert_eq!(
        workspace_dependencies(&manifest),
        vec![
            // The rendering path.
            "mjx-layout",
            "mjx-scene",
            "mjx-text",
            // The platform boundary. This edge, together with the three DrawingML ones below, is
            // what no ranked crate could declare and therefore why this crate has no rank.
            "mjx-paint",
            // **The `preset-star` specimen's cost, and the whole of it.** One specimen draws a
            // document's own preset shape so that `placeholders == 0` is asserted about the real
            // provider rather than about a page with no shapes in it. `CLAUDE.md` used to say this
            // crate names no format crate *and no document anywhere*; the first half is true and
            // the second was an overstatement, which audit pass 10 corrected. A fourth DrawingML
            // dependency changes this line rather than arriving quietly.
            "mjx-geometry",
            "mjx-dml",
            "mjx-ooxml-core",
            "mjx-ooxml-types",
            // Dependency-free and below everything, so the gallery's document rows come from the
            // committed corpus rather than from a list written here.
            "mjx-fixtures",
        ],
        "this crate has **no rank**, so `xtask/tests/layering.rs` holds nothing about what it \
         depends on — only about what depends on it. A format crate or the facade appearing here \
         would be refused by nothing else in the workspace."
    );
}
