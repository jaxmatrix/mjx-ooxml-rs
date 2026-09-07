//! **The gate that replaces this crate's rank.**
//!
//! # Why a source gate, and why it matters more here than anywhere else
//!
//! `mjx-layout` has a file of this name and it is a *second* line of defence: the layering test
//! already refuses an edge from rank 1.6 to a format crate, and the source gate catches the mistake
//! one commit earlier, in the crate that made it.
//!
//! **Here there is no first line.** `xtask/tests/layering.rs` refuses only an edge that points up or
//! sideways, and `mjx-paint` is at rank 5.5 — above the facade — so *every crate in the workspace is
//! a legal dependency of this one*. The rank is still right, and for a reason that has nothing to do
//! with what this crate may depend on: it is what stops anything in the document graph depending on
//! **it**, which is what keeps a GPU out of `bindings/mjx-python`. But the architecture's second
//! seam — *below a display list, nothing has heard of a font, a layout algorithm or a document* —
//! has **nothing else holding it**. `mjx-scene` was given rank 1.7 so the layering test could refuse
//! its illegal edge by name; no rank can do that job for a painter, in either direction.
//!
//! So this file is it.
//!
//! # The one exemption, and why it is not a hole
//!
//! MJXOFF-163 asks for two things that cannot both be had literally: **upload only what changed**,
//! which means R04's own `GlyphAtlas::take_delta`, and **name no crate below the display list**. A
//! display list carries no pixels — `mjx-scene` says so, and copies `AtlasPlacement`'s eight numbers
//! rather than re-exporting `mjx_text::AtlasEntry` precisely so a cached list stays readable after
//! the page it named is gone — so the live atlas cannot be reached through `mjx-scene`.
//!
//! The resolution, stated rather than omitted: `mjx-paint` owns the contract (`AtlasSource`) and
//! **exactly one file** adapts `mjx_text::GlyphAtlas` to it. This gate forbids `mjx-text` everywhere
//! else **and asserts it does appear there**, so the exemption can neither spread nor rot into a
//! permission nothing uses. The manifest is asserted exactly, so the cost is visible where a reader
//! would look for it.
//!
//! # Proved by mutation
//!
//! * Adding `mjx-layout` to the manifest → the manifest assertion fails, naming it.
//! * Adding `use mjx_layout::FragmentTree;` to `src/plan.rs` → the source scan fails, naming the
//!   file and the line.
//! * Naming `mjx_text` in `src/painter.rs` → the source scan fails, and says which file is allowed
//!   to.
//! * Emptying `src/glyph_atlas.rs` of its `mjx_text` uses → the *reverse* assertion fails, which is
//!   what stops the exemption becoming dead.
//! * Adding a module to `src/` without touching [`SOURCE_FILE_COUNT`] → the count fails, so a new
//!   file cannot slip past the scan.

use std::path::{Path, PathBuf};

/// How many `.rs` files `src/` holds, counted recursively.
///
/// Exact rather than a floor, for the reason `mjx-layout`'s copy gives: a `>=` would pass on a scan
/// that quietly stopped, and MJXOFF-155 §8 lists the non-recursive walk as a recurring defect in
/// this project.
const SOURCE_FILE_COUNT: usize = 15;

/// The one file that may name the font engine.
const ATLAS_ADAPTER: &str = "glyph_atlas.rs";

/// The marker a hand-written `unsafe` line must carry.
///
/// The same string `.github/workflows/ci.yml` greps for. Asserted here as well as there because a
/// gate that only runs in continuous integration tells you one round trip late.
const UNSAFE_MARKER: &str = "MJX-PAINT-SURFACE-UNSAFE";

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
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

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
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
fn nothing_below_the_display_list_is_named_outside_the_one_adapter() {
    // Everything the seam forbids. `mjx-layout` and `mjx-dml` are the two the ticket names; the
    // format crates, the facade and the packaging tier are here because a painter that reached any
    // of them would have learned what a `.docx` is, which is the thing the whole client platform is
    // organised to prevent.
    const FORBIDDEN: &[&str] = &[
        "mjx_layout",
        "mjx-layout",
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
        "mjx_pptx",
        "mjx-pptx",
        "mjx_docx",
        "mjx-docx",
        "mjx_xlsx",
        "mjx-xlsx",
        "mjx_ooxml_types",
        "mjx-ooxml-types",
        "mjx_ooxml_core",
        "mjx-ooxml-core",
        "mjx_opc",
        "mjx-opc",
        "mjx_mce",
        "mjx-mce",
        "mjx_xml",
        "mjx-xml",
    ];

    let files = scanned_sources();
    let mut scanned_lines = 0usize;
    for path in &files {
        let text = read(path);
        for (number, line) in text.lines().enumerate() {
            scanned_lines += 1;
            // Prose may name them, and does at length: explaining *why* a painter may not reach a
            // format crate is exactly the reasoning that stops the next person doing it. Code may
            // not, and a `use`, a path or a type name is never inside a comment.
            if line.trim_start().starts_with("//") {
                continue;
            }
            for forbidden in FORBIDDEN {
                assert!(
                    !line.contains(forbidden),
                    "{}:{} names `{forbidden}`. Rank 5.5 makes that edge *legal*, which is why this \
                     file exists: below a display list nothing has heard of a font, a layout \
                     algorithm or a document, and at this rank nothing but this assertion is \
                     holding it. If the painter genuinely needs something from there, the seam has \
                     leaked and `mjx-scene` owns the fix.",
                    path.display(),
                    number + 1
                );
            }
        }
    }
    assert!(
        scanned_lines > 3_000,
        "only {scanned_lines} lines were scanned, which cannot be this crate"
    );
}

#[test]
fn the_font_engine_is_named_in_exactly_one_file_and_that_file_uses_it() {
    let files = scanned_sources();
    let mut naming: Vec<String> = Vec::new();
    let mut adapter_mentions = 0usize;

    for path in &files {
        let is_adapter = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == ATLAS_ADAPTER);
        let text = read(path);
        for line in text.lines() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if !line.contains("mjx_text") && !line.contains("mjx-text") {
                continue;
            }
            if is_adapter {
                adapter_mentions += 1;
            } else {
                naming.push(format!("{}: {}", path.display(), line.trim()));
            }
        }
    }

    assert!(
        naming.is_empty(),
        "only `src/{ATLAS_ADAPTER}` may name the font engine, and these lines do too:\n{}\n\
         `mjx-scene` re-exports `BitmapFormat`, `DeviceScale`, `Hinting`, `ScaleBucket` and \
         `TextDirection` for exactly this reason — write `use mjx_scene::BitmapFormat;` instead. \
         The one exemption exists because a display list carries no pixels and the atlas delta \
         therefore cannot be reached through it; it is not a general permission.",
        naming.join("\n")
    );
    assert!(
        adapter_mentions > 0,
        "`src/{ATLAS_ADAPTER}` is exempted from this gate and no longer names the font engine at \
         all. An exemption nothing uses is a hole rather than a decision: either the adapter has \
         moved, in which case this gate must follow it, or it is dead and both should go."
    );
}

#[test]
fn the_manifest_declares_exactly_the_dependencies_the_seam_allows() {
    let manifest = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"));
    let mut in_dependencies = false;
    let mut declared: Vec<&str> = Vec::new();
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            // `[package]` inherits `version`, `edition` and the rest from the workspace with the
            // same spelling, so the section is tracked rather than the whole file scanned.
            in_dependencies = trimmed == "[dependencies]";
            continue;
        }
        if !in_dependencies {
            continue;
        }
        if let Some((name, _)) = trimmed.split_once(".workspace = true") {
            declared.push(name.trim());
        }
    }
    assert_eq!(
        declared,
        vec![
            "mjx-scene",
            "mjx-tokens",
            "mjx-text",
            "thiserror",
            "wgpu",
            "pollster",
        ],
        "this crate depends on the display list, the design tokens, the font engine (in one file \
         only — see the test above), an error derive and the graphics stack. `mjx-layout`, \
         `mjx-dml`, a format crate or the facade appearing here means the seam has leaked, and the \
         layering test cannot refuse any of them at rank 5.5."
    );
}

#[test]
fn there_is_exactly_one_hand_written_unsafe_and_it_carries_its_marker() {
    let files = scanned_sources();
    let mut marked = Vec::new();
    let mut unmarked = Vec::new();

    for path in &files {
        let text = read(path);
        for (number, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") || code.starts_with("//!") {
                continue;
            }
            // The crate-level attribute is the permission, not a use of it.
            if line.contains("#![allow(unsafe_code)]") {
                continue;
            }
            if !names_the_keyword(line) {
                continue;
            }
            if line.contains(UNSAFE_MARKER) {
                marked.push(format!("{}:{}", path.display(), number + 1));
            } else {
                unmarked.push(format!("{}:{} — {}", path.display(), number + 1, code));
            }
        }
    }

    assert!(
        unmarked.is_empty(),
        "hand-written `unsafe` without the `{UNSAFE_MARKER}` marker:\n{}\n\
         This crate carries `#![allow(unsafe_code)]` on the stated grounds that there is exactly \
         one unsafe block in it — the surface created from a window handle the shell supplied. \
         Adding another needs its own justification and its own review, and this gate and the CI \
         grep both updated to permit it explicitly.",
        unmarked.join("\n")
    );
    assert_eq!(
        marked.len(),
        1,
        "the crate claims exactly one hand-written `unsafe`; {} line(s) carry the marker: {marked:?}",
        marked.len()
    );
}

/// Whether `line` uses the **keyword** `unsafe`, rather than merely containing those six letters.
///
/// `create_surface_unsafe` and `SurfaceTargetUnsafe` are identifiers, not unsafe blocks, and a scan
/// that could not tell the difference would report the very call this crate exists to make. The rule
/// is a word boundary on both sides where `_` counts as a word character — which is exactly what GNU
/// `grep`'s `\b` does, so this and the continuous-integration grep agree by construction rather than
/// by coincidence.
fn names_the_keyword(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut from = 0usize;
    while let Some(offset) = line.get(from..).and_then(|rest| rest.find("unsafe")) {
        let at = from + offset;
        let before = at
            .checked_sub(1)
            .and_then(|index| bytes.get(index))
            .copied();
        let after = bytes.get(at + "unsafe".len()).copied();
        let is_word = |byte: Option<u8>| {
            byte.is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        };
        if !is_word(before) && !is_word(after) {
            return true;
        }
        from = at + "unsafe".len();
    }
    false
}

#[test]
fn the_keyword_scan_can_tell_a_keyword_from_an_identifier() {
    // The gate's own instrument, checked. A scan that matched `create_surface_unsafe` would report
    // the one call this crate is built around and fail for ever; one that matched nothing would
    // report none and pass for ever. Both are worth a line each to refuse.
    assert!(names_the_keyword(
        "        let surface = unsafe /* marker */ {"
    ));
    assert!(names_the_keyword("unsafe impl Send for Anything {}"));
    assert!(!names_the_keyword("instance.create_surface_unsafe(target)"));
    assert!(!names_the_keyword("wgpu::SurfaceTargetUnsafe::RawHandle"));
    assert!(!names_the_keyword("let unsafely = 1;"));
}

#[test]
fn nothing_on_a_painting_path_panics() {
    // A malformed display list, a lost device and a driver that refuses a texture all produce a
    // bad-looking frame or a typed error, never a crash: a painter that can panic turns a document
    // somebody is editing into a closed window.
    const FORBIDDEN: &[&str] = &[
        ".unwrap()",
        ".expect(",
        "panic!(",
        "todo!(",
        "unimplemented!(",
        "unreachable!(",
    ];

    let files = scanned_sources();
    for path in &files {
        let text = read(path);
        for (number, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            for forbidden in FORBIDDEN {
                assert!(
                    !line.contains(forbidden),
                    "{}:{} uses `{forbidden}`: {code}",
                    path.display(),
                    number + 1
                );
            }
        }
    }
}
