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
//! * Adding `mjx-ooxml` to the manifest with a `use` in `src/plan.rs` → both the manifest assertion
//!   and the source scan fail, naming the facade. **Before MJXOFF-164 neither did**, which is the
//!   first of the three holes below.
//!
//! # Three holes MJXOFF-164 closed, and why they mattered more than they look
//!
//! This file is the only thing standing at rank 5.5, so a hole in it is a hole in the architecture
//! rather than in a test. An audit of MJXOFF-163 found three, and they compose into one commit that
//! reaches the facade:
//!
//! 1. **The forbidden list did not contain the facade.** Its own comment three lines above said
//!    *"the format crates, the facade and the packaging tier are here"*, and `mjx_ooxml` was not:
//!    `mjx_ooxml_types` and `mjx_ooxml_core` were, but `use mjx_ooxml::Deck;` contains neither
//!    substring and passed. The one edge the rank exists to make *impossible for others* was the
//!    one this gate could not see.
//! 2. **The manifest scan read one dependency table.** It tracked `[dependencies]` alone, so this
//!    crate's own `[target.'cfg(target_arch = "wasm32")'.dependencies]` — which is *in this file* —
//!    was invisible, and a dependency declared there was unchecked.
//! 3. **It read one spelling.** Only `name.workspace = true`, not `name = { workspace = true }`,
//!    which Cargo treats identically.
//!
//! None of the three is reachable by the layering test either, because at rank 5.5 every one of
//! those edges points *down* and is legal. All three are closed below, and the third assertion of
//! this file's own instrument test now proves the parser sees both spellings.

use std::path::{Path, PathBuf};

/// How many `.rs` files `src/` holds, counted recursively.
///
/// Exact rather than a floor, for the reason `mjx-layout`'s copy gives: a `>=` would pass on a scan
/// that quietly stopped, and MJXOFF-155 §8 lists the non-recursive walk as a recurring defect in
/// this project.
const SOURCE_FILE_COUNT: usize = 23;

/// The files that may name the font engine, and nothing else may.
///
/// # Why this grew from one to two, deliberately
///
/// MJXOFF-163 had exactly one: `glyph_atlas.rs`, which adapts `mjx_text::GlyphAtlas` to this
/// crate's [`AtlasSource`](mjx_paint::AtlasSource) because a display list carries no pixels and the
/// atlas's delta therefore cannot be reached through it.
///
/// MJXOFF-164 needs a **second** thing from the font engine and it is not the atlas: a PDF embeds a
/// font file and an SVG draws glyph outlines, and neither the file nor the outlines can be reached
/// through a display list either — a list records where a glyph's *pixels* are, and it does not
/// record the face.
///
/// The three ways that could have gone:
///
/// * delete this assertion, which trades the only gate at rank 5.5 for a convenience;
/// * put the exporter's font handling in `glyph_atlas.rs`, which passes unchanged and leaves a file
///   called "glyph atlas" embedding font files, where no reader would look for it;
/// * **name a second file here, with the reason** — which is what this is. Each permitted file is
///   still asserted to *use* the permission, so neither can rot; every other file is still refused;
///   and the exemption is still enumerated in the one place a reader would look for it.
const FONT_ENGINE_ADAPTERS: &[&str] = &["glyph_atlas.rs", "font_source.rs"];

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
        // **Added by MJXOFF-202, and the rank cannot do it.** `mjx-geometry` is at 2.5 and this
        // crate is at 5.5, so `mjx-paint -> mjx-geometry` points *down* and `xtask/tests/layering.rs`
        // would allow it. It holds the preset shape path tables — DrawingML, by construction — so a
        // painter that named it would be a painter that knows what an `a:prstGeom` is, and could
        // build its own `GeometryProvider` instead of being handed one. That is precisely what the
        // display list exists to spare it: a painter reads meshes and a provenance, and who decided
        // what a `roundRect` looks like is not its business.
        "mjx_geometry",
        "mjx-geometry",
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
        // **The facade, missing until MJXOFF-164.** `mjx_ooxml_types` and `mjx_ooxml_core` were
        // both listed and neither is a substring of `mjx_ooxml`, so `use mjx_ooxml::Deck;` passed
        // this scan: the one edge rank 5.5 exists to forbid for everybody else was the one edge
        // this gate could not see. It is spelled with its two possible followers rather than bare,
        // because a bare `mjx_ooxml` is a prefix of the two lines below it and would report them.
        "mjx_ooxml::",
        "mjx_ooxml;",
        "mjx_ooxml}",
        "mjx-ooxml\"",
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
    let mut mentions: std::collections::BTreeMap<&str, usize> = FONT_ENGINE_ADAPTERS
        .iter()
        .map(|name| (*name, 0usize))
        .collect();

    for path in &files {
        let adapter = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| {
                FONT_ENGINE_ADAPTERS
                    .iter()
                    .find(|allowed| **allowed == name)
            });
        let text = read(path);
        for line in text.lines() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if !line.contains("mjx_text") && !line.contains("mjx-text") {
                continue;
            }
            match adapter {
                Some(name) => *mentions.entry(*name).or_default() += 1,
                None => naming.push(format!("{}: {}", path.display(), line.trim())),
            }
        }
    }

    assert!(
        naming.is_empty(),
        "only {FONT_ENGINE_ADAPTERS:?} may name the font engine, and these lines do too:\n{}\n\
         `mjx-scene` re-exports `BitmapFormat`, `DeviceScale`, `Hinting`, `ScaleBucket` and \
         `TextDirection` for exactly this reason — write `use mjx_scene::BitmapFormat;` instead. \
         The two exemptions exist because a display list carries neither pixels nor faces, and \
         neither the atlas delta nor a face can therefore be reached through it; they are not a \
         general permission.",
        naming.join("\n")
    );
    for (name, count) in &mentions {
        assert!(
            *count > 0,
            "`src/{name}` is exempted from this gate and no longer names the font engine at all. \
             An exemption nothing uses is a hole rather than a decision: either the adapter has \
             moved, in which case this gate must follow it, or it is dead and both should go."
        );
    }
}

#[test]
fn the_manifest_declares_exactly_the_dependencies_the_seam_allows() {
    let manifest = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"));
    assert_eq!(
        workspace_dependencies(&manifest),
        vec![
            "mjx-scene",
            "mjx-tokens",
            "mjx-text",
            "thiserror",
            "wgpu",
            "pollster",
            "tiny-skia",
            // The browser build's own table. Until MJXOFF-164 this scan read `[dependencies]`
            // alone and never saw this line — in the very file the gate is about.
            "wgpu (target cfg(target_arch = \"wasm32\"))",
            // Dev-dependencies are scanned too. They cannot reach a shipped binary, but a
            // dev-dependency on a format crate would let a *test* in this crate learn what a
            // `.docx` is, and a painter's fixtures would then be documents.
            "wgpu (dev)",
            "mjx-allocation-counter (dev)",
        ],
        "this crate depends on the display list, the design tokens, the font engine (in the two \
         files the test above names), an error derive, the graphics stack and the software \
         rasteriser. `mjx-layout`, `mjx-dml`, a format crate or the facade appearing here means \
         the seam has leaked, and the layering test cannot refuse any of them at rank 5.5."
    );
}

/// Every workspace dependency the manifest declares, in **every** table, in **both** spellings.
///
/// # What this had to be taught, and why each half mattered
///
/// **Every table.** MJXOFF-163's version tracked `[dependencies]` and nothing else, so this crate's
/// own `[target.'cfg(target_arch = "wasm32")'.dependencies]` — a table that is *in the file the gate
/// is about* — was invisible, and so was `[dev-dependencies]`.
///
/// **Both spellings.** It matched only `name.workspace = true`. Cargo treats
/// `name = { workspace = true }` identically, and the second is what a dependency with any extra key
/// has to be written as — including `wgpu = { workspace = true, features = ["webgl"] }`, which is
/// the one this crate actually has.
///
/// Together they were a one-commit escape to the facade that neither this gate nor the layering test
/// could see. Each entry is tagged with the table it came from, so a dependency that *moved* between
/// tables changes the assertion rather than passing silently.
fn workspace_dependencies(manifest: &str) -> Vec<String> {
    let mut table: Option<String> = None;
    let mut declared = Vec::new();
    for line in manifest.lines() {
        let trimmed = line.trim();
        if let Some(header) = trimmed
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            table = classify(header);
            continue;
        }
        let Some(suffix) = table.as_deref() else {
            continue;
        };
        let Some(name) = declares_a_workspace_dependency(trimmed) else {
            continue;
        };
        declared.push(if suffix.is_empty() {
            name.to_owned()
        } else {
            format!("{name} ({suffix})")
        });
    }
    declared
}

/// What a table header means for this scan, or `None` for a table that declares no dependencies.
///
/// A `[target.'...'.dependencies]` keeps its condition in the tag, because *"`wgpu` on `wasm32`"*
/// and *"`wgpu` everywhere"* are different declarations, and an assertion that could not tell them
/// apart would accept a dependency moved from one to the other.
fn classify(header: &str) -> Option<String> {
    match header {
        "dependencies" => return Some(String::new()),
        "dev-dependencies" => return Some("dev".to_owned()),
        "build-dependencies" => return Some("build".to_owned()),
        _ => {}
    }
    let rest = header.strip_prefix("target.")?;
    let (condition, kind) = rest.rsplit_once('.')?;
    let condition = condition.trim_matches('\'').trim_matches('"');
    match kind {
        "dependencies" => Some(format!("target {condition}")),
        "dev-dependencies" => Some(format!("dev, target {condition}")),
        _ => None,
    }
}

/// The name a line declares as a workspace dependency, in either spelling.
fn declares_a_workspace_dependency(line: &str) -> Option<&str> {
    if let Some((name, _)) = line.split_once(".workspace = true") {
        return Some(name.trim());
    }
    let (name, rest) = line.split_once('=')?;
    let rest = rest.trim();
    if !rest.starts_with('{') || !rest.contains("workspace = true") {
        return None;
    }
    Some(name.trim())
}

#[test]
fn the_manifest_scanner_reads_every_table_and_both_spellings() {
    // The gate's own instrument, checked — the same way the keyword scan below is. A parser that saw
    // one table would pass the assertion above for ever while a dependency sat unread in another,
    // which is exactly what happened before MJXOFF-164.
    const SAMPLE: &str = concat!(
        "[package]\n",
        "name = \"x\"\n",
        "version.workspace = true\n",
        "[dependencies]\n",
        "mjx-scene.workspace = true\n",
        "thiserror = { workspace = true }\n",
        "serde = \"1\"\n",
        "[target.'cfg(target_arch = \"wasm32\")'.dependencies]\n",
        "wgpu = { workspace = true, features = [\"webgl\"] }\n",
        "[dev-dependencies]\n",
        "mjx-fixtures.workspace = true\n",
        "[[test]]\n",
        "name = \"harnessless\"\n"
    );
    assert_eq!(
        workspace_dependencies(SAMPLE),
        vec![
            "mjx-scene",
            "thiserror",
            "wgpu (target cfg(target_arch = \"wasm32\"))",
            "mjx-fixtures (dev)",
        ],
        "the scanner must read every dependency table and both spellings, and must not mistake \
         `[package]`'s inherited `version.workspace = true` or a `[[test]]` section for one"
    );
    assert_eq!(declares_a_workspace_dependency("serde = \"1\""), None);
    assert_eq!(
        declares_a_workspace_dependency("wgpu = { version = \"30\" }"),
        None,
        "a table without `workspace = true` in it is not a workspace dependency"
    );
    assert_eq!(classify("package"), None);
    assert_eq!(classify("dependencies"), Some(String::new()));
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
