//! Nowhere in this workspace does anything build `PlaceholderGeometry` without saying why.
//!
//! MJX-STAND-IN: this file *is* the census, so it names the stand-in in the needles it searches
//! for; it constructs one only inside `a_synthetic_file`, the negative control that shows the gate
//! able to fail.
//!
//! # Why a census and not a sweep
//!
//! MJXOFF-206's gate reads *"every consumer that constructs a provider is updated, or names why it
//! still wants the placeholder"*, and a sweep satisfies that once. The next file to reach for the
//! stand-in gets no such reading, and a renderer that draws framed crossed rectangles for a
//! document it could have drawn properly is not a failure anything reports — it is a picture
//! somebody looks at and believes.
//!
//! So the claim is checked from the file system, in two halves:
//!
//! 1. **Exactly one *shipped* file constructs the stand-in**, and it is
//!    [`crates/mjx-geometry/src/provider.rs`] — the `UnknownShapePolicy::StandIn` fall-through, which
//!    is the honest answer for a handle nobody registered, a preset ECMA-376 defines no geometry
//!    for, or a shape whose own formulas are singular at the adjustments in force. **That is the
//!    whole of "the placeholder is gone":** no `src/` path outside that one arm can put a stand-in
//!    on a page.
//! 2. **Every other file that constructs one declares a reason**, on a line carrying the marker
//!    [`MARKER`]. Tests that are *about* the stand-in legitimately want it — the tessellator's
//!    suites need a provider that answers every handle and none of them care what it draws — and
//!    the marker is where that is written down rather than assumed.
//!
//! The idiom is `mjx-paint`'s: `MJX-PAINT-SURFACE-UNSAFE` is grepped for by CI so that the one
//! hand-written `unsafe` block cannot quietly become two. *"A claim CI does not check is a claim
//! that quietly stops being true"* is that job's own comment, and it is the reason this file is a
//! test rather than a paragraph.
//!
//! # What this does not claim
//!
//! Not that the stand-in is unreachable — it must stay reachable, and
//! `the_provider_is_wired_in.rs::the_stand_in_is_still_reachable_and_is_still_the_scenes_own`
//! asserts that it is. Deleting `PlaceholderGeometry` would replace a *visible* placeholder with a
//! silent nothing, which is the one answer `mjx-scene`'s seam forbids.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The marker a file carries to say why it builds a stand-in.
///
/// Searched for as a whole line's substring, so it may sit in module documentation, in an ordinary
/// comment or beside the construction itself. What matters is that a reader who greps for the
/// stand-in finds a sentence rather than a call.
const MARKER: &str = "MJX-STAND-IN:";

/// How much prose a reason has to be before it counts as one.
///
/// Sixty characters is about a line of English. A marker followed by nothing is the same defect as
/// no marker, and one followed by `see above` is worse, because it looks answered.
const REASON_CHARACTERS: usize = 60;

/// The spellings that *build* a stand-in, as opposed to naming its type.
///
/// `PlaceholderGeometry` alone would match the type's own definition, every `use` that imports it
/// and every sentence that mentions it — which is most of the reason this workspace's prose is
/// worth reading. Only a construction can put one on a page.
const CONSTRUCTIONS: &[&str] = &[
    "PlaceholderGeometry::new(",
    "PlaceholderGeometry::default(",
    "PlaceholderGeometry {",
];

/// The one shipped file allowed to build a stand-in, and what it is.
const THE_ONE_SHIPPED_CONSTRUCTION: &str = "crates/mjx-geometry/src/provider.rs";

/// Directories a walk never enters: build output, version control, and the git-ignored reference
/// tree, which is a **symlink** in this worktree and must not be followed.
const NOT_SOURCE: &[&str] = &[
    "target",
    ".git",
    ".github",
    "node_modules",
    "References",
    ".claude",
    "dist",
    "npm",
];

/// The workspace root, checked to be one rather than assumed.
fn workspace_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root resolves");
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("a root manifest");
    assert!(
        manifest.contains("[workspace]"),
        "{} is not the workspace root, so this census would walk the wrong tree and pass by \
         finding nothing",
        root.display()
    );
    root
}

/// Every `.rs` file under `root`, relative to it, in a stable order.
fn rust_files(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            // `symlink_metadata`, so a symlinked directory is a file to this walk and is never
            // descended into.
            let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if metadata.is_dir() {
                if !NOT_SOURCE.contains(&name.as_str()) {
                    stack.push(path);
                }
            } else if metadata.is_file() && name.ends_with(".rs") {
                let relative = path
                    .strip_prefix(root)
                    .expect("every walked path is under the root")
                    .to_string_lossy()
                    .replace('\\', "/");
                found.push(relative);
            }
        }
    }
    found.sort();
    found
}

/// The line shapes that *mention* the type where a needle would otherwise match: the type's own
/// definition and its `impl` blocks.
///
/// `PlaceholderGeometry {` is a construction in an expression and a block header in
/// `impl PlaceholderGeometry {`, and the difference is the keyword in front. Without this the
/// census names `crates/mjx-scene/src/provider.rs` — the file that *defines* the stand-in — as a
/// second shipped construction, which found this list rather than a defect.
const DECLARATIONS: &[&str] = &["impl ", "struct ", "enum ", "trait ", "type "];

/// Whether `line` builds a stand-in, ignoring comments and declarations.
///
/// A doc example that shows the stand-in is documentation and not a render, so a `//` line does not
/// count — which is why `crates/mjx-paint/src/lib.rs`'s frame example is not a second shipped
/// construction. It is still prose that can go stale, and it says in the example itself why the
/// painter may not name the real provider.
fn constructs_a_stand_in(line: &str) -> bool {
    let code = line.trim_start();
    if code.starts_with("//") {
        return false;
    }
    let code = code.strip_prefix("pub ").unwrap_or(code);
    if DECLARATIONS.iter().any(|keyword| code.starts_with(keyword)) {
        return false;
    }
    CONSTRUCTIONS.iter().any(|needle| code.contains(needle))
}

/// The reason `contents` declares, if it declares one long enough to be a reason.
fn reason_in(contents: &str) -> Option<&str> {
    contents.lines().find_map(|line| {
        let (_, after) = line.split_once(MARKER)?;
        let reason = after.trim();
        (reason.chars().count() >= REASON_CHARACTERS).then_some(reason)
    })
}

#[test]
fn only_the_providers_own_fall_through_builds_a_stand_in_in_shipped_code() {
    let root = workspace_root();
    let files = rust_files(&root);
    assert!(
        files.len() > 200,
        "the walk found only {} Rust files, which is not this workspace — a census that walks \
         nothing passes by finding nothing",
        files.len()
    );

    let mut shipped = BTreeSet::new();
    for relative in &files {
        if !relative.contains("/src/") {
            continue;
        }
        let contents = std::fs::read_to_string(root.join(relative)).expect("a readable file");
        if contents.lines().any(constructs_a_stand_in) {
            shipped.insert(relative.clone());
        }
    }

    assert_eq!(
        shipped,
        BTreeSet::from([THE_ONE_SHIPPED_CONSTRUCTION.to_owned()]),
        "shipped code builds a stand-in somewhere other than the provider's own \
         `UnknownShapePolicy::StandIn` fall-through. That is what MJXOFF-206 removed: a render path \
         that can substitute a framed crossed rectangle for a shape it could have drawn is a page \
         somebody looks at and believes"
    );
}

#[test]
fn every_file_that_builds_a_stand_in_says_why() {
    let root = workspace_root();
    let mut without_a_reason = Vec::new();
    let mut declared = 0usize;

    for relative in rust_files(&root) {
        let contents = std::fs::read_to_string(root.join(&relative)).expect("a readable file");
        if !contents.lines().any(constructs_a_stand_in) {
            continue;
        }
        match reason_in(&contents) {
            Some(_) => declared += 1,
            None => without_a_reason.push(relative),
        }
    }

    assert!(
        without_a_reason.is_empty(),
        "these files build a stand-in and do not say why. Add a line carrying `{MARKER}` and at \
         least {REASON_CHARACTERS} characters of reason — a suite that is *about* the stand-in has \
         a good one, and writing it down is what stops the next file inheriting the habit:\n  {}",
        without_a_reason.join("\n  ")
    );
    assert!(
        declared >= 10,
        "only {declared} file(s) were found to build a stand-in at all. This workspace has more \
         than that, so the search is not finding them and every assertion above is vacuous"
    );
}

// -------------------------------------------------------------------------------------------
// The census shown able to fail
// -------------------------------------------------------------------------------------------

/// A file's contents, for the negative control.
///
/// MJX-STAND-IN: the strings below are the inputs the gate's own predicates are tested against,
/// so the census can be shown to fail rather than merely observed to pass.
fn a_synthetic_file(construction: &str, marker: &str) -> String {
    format!("//! A module.\n{marker}\nfn draw() {{\n    let g = {construction};\n}}\n")
}

#[test]
fn the_census_can_fail() {
    // Three ways a file can be wrong, and one way it can be right. Without this the two suites
    // above would pass on a predicate that answered `false` to everything.
    let good = format!(
        "//! {MARKER} this suite is about the stand-in itself and needs a provider that answers \
         every handle, whatever it draws."
    );

    let built = a_synthetic_file("PlaceholderGeometry::new()", &good);
    assert!(built.lines().any(constructs_a_stand_in));
    assert!(reason_in(&built).is_some());

    // No marker at all.
    let unnamed = a_synthetic_file("PlaceholderGeometry::new()", "// nothing to declare");
    assert!(unnamed.lines().any(constructs_a_stand_in));
    assert!(
        reason_in(&unnamed).is_none(),
        "a file with no marker was accepted"
    );

    // A marker with a reason too short to be one.
    let terse = a_synthetic_file(
        "PlaceholderGeometry::new()",
        &format!("// {MARKER} see above"),
    );
    assert!(
        reason_in(&terse).is_none(),
        "`see above` was accepted as a reason, so the marker could be satisfied by typing it"
    );

    // Naming the type is not building one, and neither is showing it in a doc example.
    assert!(!constructs_a_stand_in(
        "use mjx_scene::PlaceholderGeometry;"
    ));
    assert!(!constructs_a_stand_in(
        "/// See [`PlaceholderGeometry`] for the stand-in."
    ));
    assert!(!constructs_a_stand_in(
        "//! let geometry = PlaceholderGeometry::new();"
    ));
    assert!(!constructs_a_stand_in("pub struct PlaceholderGeometry;"));
    // The distinction that cost this file a false positive: a block header is not a construction.
    assert!(!constructs_a_stand_in("impl PlaceholderGeometry {"));
    assert!(!constructs_a_stand_in(
        "impl GeometryProvider for PlaceholderGeometry {"
    ));
    assert!(constructs_a_stand_in(
        "    let g = PlaceholderGeometry { };"
    ));
    // …and each spelling that *is* one is recognised, so the needle list is not two dead entries
    // and a live one.
    for needle in CONSTRUCTIONS {
        assert!(
            constructs_a_stand_in(&format!("    let g = {needle});")),
            "`{needle}` is in the needle list and matches nothing"
        );
    }
}
