//! **The user guide's site is generated, and what it generates is checked.** (MJXOFF-281.)
//!
//! # Why this file is written against properties rather than against a committed rendering
//!
//! Every other generator in this workspace commits its output and compares — `codegen_drift.rs`
//! against `mjx-ooxml-types`, `guide_examples.rs` against the blocks in the guide. This one cannot,
//! and the difference is deliberate rather than a shortcut: the site's content tree is **generated
//! and git-ignored**, following the precedent that `bindings/mjx-wasm/npm/README.md` is generated
//! and ignored while its `package.json` is committed. Committing a rendering of pages that are
//! themselves committed would put the same sentences in the repository twice, and the second copy
//! is the one that goes stale.
//!
//! So the gate runs [`xtask::docs_site::render`] and asks what came out. That is also the only
//! thing an integration test *can* do here — it compiles into its own crate and cannot see a
//! binary's modules, which is `xtask/src/lib.rs`'s standing reason for a library target at all.
//!
//! # The trap this file is written against, in its own terms
//!
//! > *A gate over a generator that produced nothing passes every property it states.*
//!
//! `doc_gate.rs`'s §7 shape, in its generation form. Four things are done about it:
//!
//! 1. **Every check prints its counts on success**, so "ran" and "skipped quietly" are different
//!    outcomes on the terminal.
//! 2. **Every floor is phrased as *the scanner has stopped matching***, never as *the corpus is
//!    exactly this size* — a floor pinned to today's total fires before the assertion it guards and
//!    hides the mutation that was meant to prove it.
//! 3. **Both populations are derived, and from different places.** The site's page set comes from
//!    the `include_str!` graph in `crates/mjx-ooxml/src/guide.rs`; the expectation it is compared
//!    against comes from `crates/mjx-ooxml/docs/guide/` on disk. Two derivations of one fact,
//!    compared in both directions, is what makes the comparison worth running — a test that
//!    compared the graph against itself would prove nothing, which is `validation_index.rs`'s own
//!    warning.
//! 4. **The tab expectation is per example, in page order**, rather than a total. A total is green
//!    when one example loses a half and another gains one.
//!
//! # What is checked
//!
//! * [`every_facade_guide_page_produces_exactly_one_site_page`] — both directions.
//! * [`every_example_produces_one_tab_group_of_the_size_its_markers_say`] — three tabs, or one when
//!   the marker declares the block Rust-only.
//! * [`no_hidden_rustdoc_line_survives_into_an_emitted_block`] — the `#` lines rustdoc compiles and
//!   hides are scaffolding, and a generic renderer prints them.
//! * [`no_unresolved_item_path_survives_as_a_link_target`] — `](crate::Deck::save)` is a link no
//!   renderer outside rustdoc can follow.
//! * [`the_site_declares_no_cargo_package`] — the layering rule's own condition for this directory
//!   being invisible to `cargo metadata`.
//!
//! # What it cannot check
//!
//! That the emitted MDX *renders*. That is a Docusaurus build, in a toolchain this crate cannot
//! run, and a textual gate claiming to verify it would be exactly the nominal check the four points
//! above are written against. What establishes it is `npm run build` in `site/`, whose
//! `onBrokenLinks` and `onBrokenAnchors` are both `throw` — so a link this generator resolved to a
//! route that does not exist fails there rather than shipping.
//!
//! # The mutation register
//!
//! Every test below was made to fail by a reachable mutation; the verbatim output is in the pull
//! request for MJXOFF-281.
//!
//! | Mutation | Fails |
//! |---|---|
//! | comment out one `pub mod`'s `include_str!` in the facade's guide module | [`every_facade_guide_page_produces_exactly_one_site_page`], on the page the site then loses |
//! | delete one language's marker from a guide page | [`every_example_produces_one_tab_group_of_the_size_its_markers_say`], naming the example and both counts |
//! | make the hidden-line filter keep `# ` lines | [`no_hidden_rustdoc_line_survives_into_an_emitted_block`] |
//! | make an unresolvable item path keep its link | [`no_unresolved_item_path_survives_as_a_link_target`] |
//! | add a `Cargo.toml` under `site/` | [`the_site_declares_no_cargo_package`] |

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use xtask::docs_site::{self, RenderedPage};
use xtask::guide_examples;

/// The workspace root — `xtask/`'s parent.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// Where the facade's guide pages live. The other derivation of the site's page set.
const FACADE_GUIDE_DIRECTORY: &str = "crates/mjx-ooxml/docs/guide";

/// Every page of the site, rendered — the subject of every test here.
fn rendered() -> Vec<RenderedPage> {
    docs_site::render(&repository_root()).unwrap_or_else(|error| {
        panic!("generating the site: {error:#}");
    })
}

/// The `.md` files the facade's guide directory holds, derived from the filesystem.
///
/// Deliberately **not** the `include_str!` graph the generator reads. Comparing the graph against
/// itself would be two lists from one source, which proves nothing; comparing it against the
/// directory catches a page committed and never wired in, and a module wired to a page that moved.
fn facade_guide_pages(root: &Path) -> BTreeSet<String> {
    let directory = root.join(FACADE_GUIDE_DIRECTORY);
    let entries = std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("reading {}: {error}", directory.display()));
    let mut pages = BTreeSet::new();
    for entry in entries {
        let entry = entry.unwrap_or_else(|error| panic!("reading a guide page: {error}"));
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".md") {
            pages.insert(format!("{FACADE_GUIDE_DIRECTORY}/{name}"));
        }
    }
    pages
}

// ===============================================================================================
// The page set, in both directions
// ===============================================================================================

/// Every facade guide page is on the site exactly once, and every site page under the guide came
/// from one.
#[test]
fn every_facade_guide_page_produces_exactly_one_site_page() {
    let root = repository_root();
    let expected = facade_guide_pages(&root);
    let pages = docs_site::site_pages(&root).unwrap_or_else(|error| {
        panic!("deriving the site's pages: {error:#}");
    });

    let mut produced: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for page in &pages {
        if page
            .source
            .starts_with(&format!("{FACADE_GUIDE_DIRECTORY}/"))
        {
            produced
                .entry(page.source.clone())
                .or_default()
                .push(page.output());
        }
    }

    println!(
        "facade guide pages on disk: {}, site pages produced from them: {}, site pages in total: {}",
        expected.len(),
        produced.len(),
        pages.len()
    );

    // The floor, before either direction. Both comparisons are green when both sides are empty.
    assert!(
        expected.len() > 3,
        "only {} page(s) were found under {FACADE_GUIDE_DIRECTORY}, so the directory walk has \
         stopped matching and both directions below would pass on nothing",
        expected.len()
    );
    assert!(
        pages.len() > expected.len(),
        "the generator produced {} page(s) from {} guide page(s), so the `include_str!` walk has \
         stopped matching — the site has an installation section and a walkthrough per program on \
         top of the guide",
        pages.len(),
        expected.len()
    );

    // Direction 1: every guide page is on the site.
    let missing: Vec<&String> = expected
        .iter()
        .filter(|page| !produced.contains_key(*page))
        .collect();
    assert!(
        missing.is_empty(),
        "{} guide page(s) produce no site page. A page committed under \
         {FACADE_GUIDE_DIRECTORY} and not wired into `crates/mjx-ooxml/src/guide.rs` is a page \
         rustdoc does not render either:\n  {}",
        missing.len(),
        missing
            .iter()
            .map(|page| page.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );

    // Direction 2: every site page under the guide came from a guide page that exists.
    let stray: Vec<&String> = produced
        .keys()
        .filter(|page| !expected.contains(*page))
        .collect();
    assert!(
        stray.is_empty(),
        "{} site page(s) name a source that is not a guide page:\n  {}",
        stray.len(),
        stray
            .iter()
            .map(|page| page.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );

    // And exactly one, not two: a page reachable twice would render twice under two routes.
    let duplicated: Vec<String> = produced
        .iter()
        .filter(|(_, outputs)| outputs.len() != 1)
        .map(|(page, outputs)| format!("{page} -> {}", outputs.join(", ")))
        .collect();
    assert!(
        duplicated.is_empty(),
        "{} guide page(s) produce more than one site page:\n  {}",
        duplicated.len(),
        duplicated.join("\n  ")
    );
}

// ===============================================================================================
// The tab groups
// ===============================================================================================

/// One `<Tabs>` block found in a rendered page: where it starts, and how many tabs it holds.
struct TabGroup {
    /// The one-based line its opener sits on, so a failure can be navigated to.
    line: usize,
    /// How many `<TabItem>` openers it holds.
    tabs: usize,
}

/// Every tab group in one rendered page, in the order they appear.
fn tab_groups(text: &str) -> Vec<TabGroup> {
    let mut groups: Vec<TabGroup> = Vec::new();
    let mut open: Option<TabGroup> = None;
    for (index, line) in text.lines().enumerate() {
        if line.starts_with("<Tabs") {
            open = Some(TabGroup {
                line: index + 1,
                tabs: 0,
            });
        } else if line.starts_with("<TabItem ") {
            if let Some(group) = open.as_mut() {
                group.tabs += 1;
            }
        } else if line.starts_with("</Tabs>") {
            if let Some(group) = open.take() {
                groups.push(group);
            }
        }
    }
    groups
}

/// Every example produces one tab group, and its tab count is the number of halves its markers show.
///
/// The expectation is derived per example and compared **in page order**, rather than as a total.
/// A total is green when one example loses its Python half and another gains a fourth block, which
/// is precisely the drift the guide's own marker gate exists to prevent.
#[test]
fn every_example_produces_one_tab_group_of_the_size_its_markers_say() {
    let root = repository_root();
    let pages = rendered();
    let mut examples = 0usize;
    let mut tabs = 0usize;
    let mut rust_only = 0usize;
    let mut wrong: Vec<String> = Vec::new();

    for page in &pages {
        if !page.source.ends_with(".md") {
            continue;
        }
        let source = std::fs::read_to_string(root.join(&page.source))
            .unwrap_or_else(|error| panic!("reading {}: {error}", page.source));
        let markers = guide_examples::markers_in(&source, &page.source)
            .unwrap_or_else(|error| panic!("reading the markers of {}: {error:#}", page.source));

        // Run-length encode the markers by example name: consecutive markers sharing a name are
        // the halves of one example, which is exactly the grouping the generator applies.
        let mut expected: Vec<(String, usize, bool)> = Vec::new();
        for marker in &markers {
            match expected.last_mut() {
                Some((name, count, _)) if *name == marker.name => *count += 1,
                _ => expected.push((marker.name.clone(), 1, marker.is_rust_only())),
            }
        }

        let found = tab_groups(&page.text);
        if found.len() != expected.len() {
            wrong.push(format!(
                "{}: {} marked example(s) and {} tab group(s)",
                page.source,
                expected.len(),
                found.len()
            ));
            continue;
        }
        for ((name, count, is_rust_only), group) in expected.iter().zip(&found) {
            examples += 1;
            tabs += group.tabs;
            if *is_rust_only {
                rust_only += 1;
            }
            let wanted = if *is_rust_only { 1 } else { 3 };
            if *count != wanted {
                wrong.push(format!(
                    "{}: `{name}` is shown by {count} marker(s), and an example is shown in all \
                     three languages or declares itself Rust-only and is shown in one",
                    page.source
                ));
            }
            if group.tabs != *count {
                wrong.push(format!(
                    "{}:{}: `{name}` has {} marker(s) and {} tab(s)",
                    page.source, group.line, count, group.tabs
                ));
            }
        }
    }

    println!(
        "examples rendered as tab groups: {examples}, tabs between them: {tabs}, of which \
         {rust_only} group(s) are declared Rust-only"
    );
    // The floor, stated as *the marker scanner has stopped matching* — never as a total. See this
    // file's header for why an exact figure here would hide the mutation it is meant to guard.
    assert!(
        examples > 10 && tabs > examples,
        "only {examples} example(s) over {tabs} tab(s) were found, so the marker scanner has \
         stopped matching and the comparison above passed on almost nothing"
    );
    assert!(
        rust_only > 0,
        "no `rust-only` example was found, so the marker scanner has stopped matching that form \
         and the one-tab arm above is unexercised"
    );
    assert!(
        wrong.is_empty(),
        "{} example(s) do not render as the tab group their markers describe:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );
}

// ===============================================================================================
// What must not survive the rendering
// ===============================================================================================

/// Whether a fence tag names Rust, so a `#` line inside it is rustdoc's rather than a comment.
fn is_rust_fence(tag: &str) -> bool {
    let tag = tag.split_whitespace().next().unwrap_or("").trim();
    tag.is_empty()
        || tag.split(',').any(|word| {
            matches!(
                word.trim(),
                "rust" | "no_run" | "ignore" | "should_panic" | "compile_fail"
            )
        })
}

/// Every line of a rendered page, paired with the fence tag it sits inside, if any.
fn lines_by_fence(text: &str) -> Vec<(usize, Option<String>, &str)> {
    let mut out = Vec::new();
    let mut fence: Option<String> = None;
    for (index, line) in text.lines().enumerate() {
        if let Some(tag) = line.strip_prefix("```") {
            fence = match fence {
                Some(_) => None,
                None => Some(tag.to_owned()),
            };
            continue;
        }
        out.push((index + 1, fence.clone(), line));
    }
    out
}

/// No block a reader sees still carries the `#` lines rustdoc compiles and hides.
///
/// The rule is per fence rather than per line, and that is the whole difficulty: `# ` opens a
/// comment in Python and a heading in markdown, and only inside a Rust fence is it scaffolding.
#[test]
fn no_hidden_rustdoc_line_survives_into_an_emitted_block() {
    let pages = rendered();
    let mut rust_lines = 0usize;
    let mut other_lines = 0usize;
    let mut hidden: Vec<String> = Vec::new();

    for page in &pages {
        for (number, fence, line) in lines_by_fence(&page.text) {
            let Some(tag) = fence else {
                continue;
            };
            if !is_rust_fence(&tag) {
                other_lines += 1;
                continue;
            }
            rust_lines += 1;
            let trimmed = line.trim_start();
            if trimmed == "#" || trimmed.starts_with("# ") {
                hidden.push(format!("{}:{number}: {line}", page.path));
            }
        }
    }

    println!(
        "lines inside a Rust fence: {rust_lines}, inside another language's: {other_lines}, \
         across {} page(s)",
        pages.len()
    );
    // The floor is over the *scan*, not over the corpus: if the fence walk stops recognising Rust
    // blocks it reports zero hidden lines while checking none of them.
    assert!(
        rust_lines > 100 && other_lines > 100,
        "the fence walk found {rust_lines} Rust line(s) and {other_lines} other(s), so it has \
         stopped matching and the assertion below passed on almost nothing"
    );
    assert!(
        hidden.is_empty(),
        "{} line(s) inside a Rust block are rustdoc's hidden scaffolding, which no renderer \
         outside rustdoc hides:\n  {}",
        hidden.len(),
        hidden.join("\n  ")
    );
}

/// No link a reader can click still points at a rustdoc item path.
///
/// Scoped to prose, because a fence may legitimately contain one: the walkthrough pages show the
/// three real source files verbatim, and a `//!` comment inside one of them is allowed to hold the
/// intra-doc links rustdoc will resolve when it renders *that* crate.
#[test]
fn no_unresolved_item_path_survives_as_a_link_target() {
    let pages = rendered();
    let mut targets = 0usize;
    let mut unresolved: Vec<String> = Vec::new();

    for page in &pages {
        for (number, fence, line) in lines_by_fence(&page.text) {
            if fence.is_some() {
                continue;
            }
            for fragment in line.split("](").skip(1) {
                let Some(end) = fragment.find(')') else {
                    continue;
                };
                let target = &fragment[..end];
                targets += 1;
                if target.contains("::") {
                    unresolved.push(format!("{}:{number}: ]({target})", page.path));
                }
            }
        }
    }

    println!(
        "link targets in prose: {targets}, across {} page(s)",
        pages.len()
    );
    assert!(
        targets > 20,
        "only {targets} link target(s) were found in the rendered prose, so the scan has stopped \
         matching and the assertion below passed on almost nothing"
    );
    assert!(
        unresolved.is_empty(),
        "{} link target(s) are rustdoc item paths, which only rustdoc can follow. Resolve them in \
         `xtask/src/docs_site.rs` or drop the link and keep the text:\n  {}",
        unresolved.len(),
        unresolved.join("\n  ")
    );
}

// ===============================================================================================
// The site stays outside the crate graph
// ===============================================================================================

/// The site declares no Cargo package, which is what keeps it invisible to `cargo metadata`.
///
/// `xtask/tests/layering.rs` reads the real dependency graph out of `cargo metadata --no-deps` and
/// holds every member to a rank. A manifest under `site/` would put a JavaScript project into that
/// graph with no rank to give it, and the layering rule is not one this repository bends.
#[test]
fn the_site_declares_no_cargo_package() {
    let root = repository_root().join(docs_site::SITE_ROOT);
    let mut manifests = Vec::new();
    let mut visited = 0usize;
    walk_for_manifests(&root, &root, &mut manifests, &mut visited);
    println!(
        "directories walked under {}: {visited}",
        docs_site::SITE_ROOT
    );
    assert!(
        visited > 2,
        "only {visited} directory(ies) were walked under {}, so the walk has stopped matching and \
         the assertion below passed on nothing",
        docs_site::SITE_ROOT
    );
    assert!(
        manifests.is_empty(),
        "{} Cargo manifest(s) under {}: it would join the workspace graph that \
         `xtask/tests/layering.rs` ranks, and there is no rank for a documentation site:\n  {}",
        manifests.len(),
        docs_site::SITE_ROOT,
        manifests.join("\n  ")
    );
}

/// The recursive half of [`the_site_declares_no_cargo_package`].
///
/// `node_modules` is skipped: a manifest a dependency ships is that dependency's, and it is not in
/// this repository. Nothing else is skipped, deliberately.
fn walk_for_manifests(root: &Path, directory: &Path, found: &mut Vec<String>, visited: &mut usize) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    *visited += 1;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            if name != "node_modules" {
                walk_for_manifests(root, &path, found, visited);
            }
        } else if name == "Cargo.toml" {
            found.push(
                path.strip_prefix(root)
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
            );
        }
    }
}
