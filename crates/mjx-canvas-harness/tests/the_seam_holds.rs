//! **The gate that stands in for a rank this crate does not have** (audit pass 10, G1).
//!
//! # Why a rank cannot do this job here, in either direction
//!
//! `xtask/tests/layering.rs` refuses an edge that points **up or sideways**, and it can only do
//! that between two crates that both have a rank. This crate has none: it sits above the whole
//! document graph, beside `mjx-reference-pack`, because it names `mjx-paint` (5.5) and
//! `mjx-render-oracle` together. The layering test therefore holds exactly one thing about it —
//! *nothing may depend on it* — and holds **nothing at all** about what it depends on. It would
//! accept `mjx-canvas-harness -> mjx-pptx` without a word.
//!
//! `CLAUDE.md` states the property as though something enforced it: *"It names **no** format crate
//! and no geometry table."* Until this file, **nothing did**. That is the same shape as `mjx-paint`
//! at rank 5.5, where the answer was an explicit manifest gate; it is that answer again, for the
//! same reason.
//!
//! # What the property is *for*, which is not layering hygiene
//!
//! `crates/mjx-canvas-harness/src/lib.rs` says it: *"sixty-one synthetic scenes, each a
//! `FragmentTree` built by hand. **The harness needs no document and no fixture**, which is the
//! property that lets in-canvas design be settled while the format renderers are still being
//! built."* A harness that reached a format crate would be one whose scenes could be built from a
//! `.pptx` — and the first time a scene was, this crate would stop being runnable before the
//! PowerPoint view exists, which is the whole point of it.
//!
//! `mjx-geometry` is refused for the reason `crates/mjx-paint/tests/the_seam_holds.rs` refuses it:
//! a preset shape table *is* DrawingML, `crates/mjx-canvas-harness/src/scenes/stage.rs` says in its
//! own comment that it writes `rounded_rectangle` out by hand rather than reaching for one, and
//! that comment was the only thing holding it.
//!
//! # ⚠ How this differs from `mjx-paint`'s, and why it is looser in exactly one place
//!
//! `mjx-paint` forbids everything below the display list — `mjx-layout`, `mjx-text`, the packaging
//! tier — because a painter that had heard of a font would be a second interpretation. This crate
//! **is** the layer above all of those and legitimately names `mjx-layout`, `mjx-scene`,
//! `mjx-text`, `mjx-tokens` and `mjx-ooxml-core`: it builds fragment trees by hand and rasterises
//! them. What it may not have heard of is a **document**, which is the format tier, the facade, the
//! packaging tier and the shared-markup tier — including the preset geometry table.
//!
//! # Proved by mutation
//!
//! Each of these was run and observed to fail before this file was committed:
//!
//! * Adding `mjx-pptx.workspace = true` to `Cargo.toml` → the manifest assertion fails, naming it.
//! * Adding `use mjx_dml::PresetGeometry;` to `src/scenes/stage.rs` → the source scan fails, naming
//!   the file and the line.
//! * Adding a module to `src/` without touching [`SOURCE_FILE_COUNT`] → the count fails, so a new
//!   file cannot slip past the scan.
//! * `manifest::the_source_scanner_finds_an_offence_and_ignores_a_comment` and
//!   `manifest::the_scanner_reads_every_table_and_both_spellings` are the **instrument's** own
//!   tests, compiled into this binary from the shared module: a manifest scanner that silently
//!   stopped seeing a category, or a source scanner that read nothing, would pass forever.

use std::path::{Path, PathBuf};

#[path = "../../mjx-paint/tests/support/manifest.rs"]
mod manifest;
use manifest::{lines_naming, read, rust_files, workspace_dependencies};

/// How many `.rs` files `src/` holds, counted recursively.
///
/// Exact rather than a floor, for the reason `mjx-paint`'s copy gives: a `>=` would pass on a scan
/// that quietly stopped, and this crate's `src/scenes/` subdirectory makes a non-recursive walk an
/// easy mistake to make and a silent one to keep.
const SOURCE_FILE_COUNT: usize = 19;

/// Everything a harness that has never heard of a document may not name.
///
/// The three tiers, in order: the format crates and the facade (*"what a `.pptx` is"*), the shared
/// markup including the preset geometry table (*"what a shape looks like"*), and the packaging tier
/// (*"what a part is"*). `mjx-ooxml-core` is deliberately **absent** — `Emu` and `Angle` are
/// measures, this crate's manifest declares it with that reason written out, and a `LayoutRect` is
/// spelled in them.
const FORBIDDEN: &[&str] = &[
    // The format tier and the facade. `mjx_ooxml` is spelled with its possible followers rather
    // than bare, because a bare `mjx_ooxml` is a prefix of `mjx_ooxml_core` — which is allowed —
    // and would report every legitimate line in the crate. `mjx-paint`'s gate learned this the
    // hard way, in the opposite direction: it listed `mjx_ooxml_types` and `mjx_ooxml_core` and
    // could not see `use mjx_ooxml::Deck;` at all.
    "mjx_pptx",
    "mjx-pptx",
    "mjx_docx",
    "mjx-docx",
    "mjx_xlsx",
    "mjx-xlsx",
    "mjx_ooxml::",
    "mjx_ooxml;",
    "mjx_ooxml}",
    "mjx-ooxml\"",
    "mjx-ooxml.",
    // Shared markup, and the preset geometry table above it.
    "mjx_geometry",
    "mjx-geometry",
    "mjx_dml",
    "mjx-dml",
    "mjx_sml",
    "mjx-sml",
    "mjx_chart",
    "mjx-chart",
    "mjx_omml",
    "mjx-omml",
    "mjx_vml",
    "mjx-vml",
    // The packaging tier. A harness that opened a package would need a package.
    "mjx_ooxml_types",
    "mjx-ooxml-types",
    "mjx_opc",
    "mjx-opc",
    "mjx_mce",
    "mjx-mce",
    "mjx_xml",
    "mjx-xml",
    // And the test-only crates below the graph, which would give this one a fixture corpus and
    // therefore a document.
    "mjx_fixtures",
    "mjx-fixtures",
    "mjx_schema_gate",
    "mjx-schema-gate",
];

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// The files, with the exact-count assertion every scan below shares.
fn scanned_sources() -> Vec<PathBuf> {
    let files = rust_files(&source_root());
    assert_eq!(
        files.len(),
        SOURCE_FILE_COUNT,
        "the walk found {} source file(s); it is recursive and this count is exact, so either a \
         module was added or the walk stopped early: {files:?}",
        files.len()
    );
    files
}

#[test]
fn no_document_is_named_anywhere_in_this_crates_source() {
    // The count assertion first, so a scan that read nothing fails as a count rather than passing
    // as an empty result.
    let files = scanned_sources();
    let offences = lines_naming(&source_root(), FORBIDDEN);
    assert!(
        offences.is_empty(),
        "this harness has never heard of a document, and these lines say otherwise:\n{}\n\
         Sixty-one scenes are `FragmentTree`s built by hand precisely so the in-canvas design can \
         be settled while the format renderers are still being built. A scene that came from a \
         `.pptx` would make this crate unrunnable until the PowerPoint view exists. If a scene \
         genuinely needs a shape, `src/scenes/stage.rs` writes the path out — see its comment on \
         `rounded_rectangle`.",
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
            // The two edges no ranked crate may declare together, which is what puts this crate
            // above the graph in the first place.
            "mjx-render-oracle",
            "mjx-paint",
            // The rendering path, and nothing below the display list is forbidden here: this crate
            // is the layer that builds fragment trees and rasterises them.
            "mjx-scene",
            "mjx-layout",
            "mjx-text",
            "mjx-ooxml-core",
            "mjx-tokens",
            "thiserror",
        ],
        "the source scan above cannot see a dependency that is declared and not yet used, and a \
         format crate sitting unused in this manifest is one `use` away from being used. \
         `xtask/tests/layering.rs` cannot refuse any of these: this crate has no rank, so its \
         downward rule has nothing to compare against and would accept `mjx-pptx` here without a \
         word."
    );
}

/// The harness's own no-panic rule, which is narrower than the painter's and is not nothing.
///
/// A harness *is* allowed to `panic!` — it is a developer tool, and `plates.rs` and `main.rs` both
/// report failure as a `Result<(), String>` that ends in `ExitCode::FAILURE`. What may not panic is
/// the **server**: a person is auditing sixty-one elements in a browser over a sitting, and a
/// thread that unwinds on a malformed query string takes the answer with it and leaves the page
/// spinning. `tests/the_router_answers.rs` drives the malformed cases; this refuses the shape.
#[test]
fn nothing_on_a_request_path_panics() {
    const FORBIDDEN_IN_SERVER: &[&str] = &[
        ".unwrap()",
        ".expect(",
        "panic!(",
        "todo!(",
        "unimplemented!(",
        "unreachable!(",
    ];
    let server = source_root().join("server.rs");
    let text = read(&server);
    for (number, line) in text.lines().enumerate() {
        let code = line.trim_start();
        if code.starts_with("//") {
            continue;
        }
        for forbidden in FORBIDDEN_IN_SERVER {
            assert!(
                !line.contains(forbidden),
                "{}:{} uses `{forbidden}`: {code}\nA request is untrusted input — a query string a \
                 browser sent while a drag was in flight, a body from a probe — and a thread that \
                 unwinds on one leaves the page it was answering spinning.",
                server.display(),
                number + 1
            );
        }
    }
}
