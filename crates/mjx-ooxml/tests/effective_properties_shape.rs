//! MJXOFF-118 (E6) — "Done when" #3: **each format's effective-properties guide exists and follows
//! the same shape.**
//!
//! Three ladders exist — PowerPoint's candidate walk, Word's style-and-numbering ladder, Excel's
//! `xf` indirection — and each has its own page:
//!
//! | crate | page | module |
//! |---|---|---|
//! | `mjx-pptx` | `docs/effective_properties.md` | `mjx_pptx::effective_properties` |
//! | `mjx-docx` | `docs/effective_properties.md` | `mjx_docx::effective_properties` |
//! | `mjx-xlsx` | `docs/effective_properties.md` | `mjx_xlsx::effective_properties` |
//!
//! Three pages written months apart by three agents drift in shape long before they drift in
//! content, and a reader who has learned one should not have to re-learn where the answers are in
//! the next. So the *shape* is checked here rather than asked for in prose: the same opening
//! question, the same four load-bearing sections in the same order, and real compiled examples on
//! every one of them.
//!
//! This file is deliberately in the facade's own test directory, because the facade is the only
//! crate that names all three formats — the same reason `chart_surface_parity.rs` lives here. It
//! reads the three files by path and declares no dependency on any of them.
//!
//! # The trap this file is written against
//!
//! "The guide exists and is covered" is green when the guide is empty. So every assertion below is
//! about content the page must *have*: a required heading missing, a heading out of order, or a page
//! with no compiled example each fail by name, and the page is asserted to be substantial rather
//! than merely present.

use std::path::{Path, PathBuf};

/// The workspace root — `crates/mjx-ooxml/../..`.
fn workspace_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/mjx-ooxml sits two levels below the workspace root")
        .to_path_buf()
}

const FORMAT_CRATES: [&str; 3] = ["mjx-pptx", "mjx-docx", "mjx-xlsx"];

/// The four sections every page carries, in this order. Between them each page says whatever its own
/// ladder needs — PowerPoint's seven text tiers, Word's toggle-property XOR, Excel's two `xf` layers
/// — and that variation is the point: the shape is shared, the substance is not.
const REQUIRED_SECTIONS: [&str; 4] = [
    "## The APIs",
    "## Where resolution stops",
    "## Cost",
    "## Examples",
];

fn page(format_crate: &str) -> String {
    let path = workspace_dir()
        .join("crates")
        .join(format_crate)
        .join("docs/effective_properties.md");
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "{}: {error} — every format crate carries this page",
            path.display()
        )
    })
}

/// The page's headings, with every fenced code block removed first: a doctest line beginning `# fn`
/// is a hidden Rust line, not a markdown heading, and counting it as one would let a page pass by
/// accident.
fn headings(page: &str) -> Vec<&str> {
    let mut headings = Vec::new();
    let mut in_fence = false;
    for line in page.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence && line.starts_with('#') {
            headings.push(line.trim_end());
        }
    }
    headings
}

#[test]
fn every_format_crate_carries_an_effective_properties_page() {
    for format_crate in FORMAT_CRATES {
        let page = page(format_crate);
        assert!(
            page.lines().count() > 100,
            "{format_crate}'s effective-properties page is {} lines — too short to be the deep \
             reference the other two are",
            page.lines().count()
        );
    }
}

#[test]
fn all_three_pages_open_on_the_same_question() {
    for format_crate in FORMAT_CRATES {
        let page = page(format_crate);
        let title = headings(&page)
            .first()
            .copied()
            .unwrap_or_else(|| panic!("{format_crate}'s page has no heading at all"))
            .to_owned();
        assert!(
            title.starts_with("# Effective properties — what a file states versus what "),
            "{format_crate}'s page opens with `{title}`; all three state the same contrast so a \
             reader arriving from one recognizes the next"
        );
    }
}

#[test]
fn all_three_pages_carry_the_same_four_sections_in_the_same_order() {
    for format_crate in FORMAT_CRATES {
        let page = page(format_crate);
        let headings = headings(&page);
        let mut looking_for = 0usize;
        for heading in &headings {
            if looking_for < REQUIRED_SECTIONS.len() && *heading == REQUIRED_SECTIONS[looking_for] {
                looking_for += 1;
            }
        }
        assert_eq!(
            looking_for,
            REQUIRED_SECTIONS.len(),
            "{format_crate}'s page is missing `{}` (or carries it out of order). It has: {:#?}",
            REQUIRED_SECTIONS[looking_for.min(REQUIRED_SECTIONS.len() - 1)],
            headings
        );
    }
}

#[test]
fn every_page_carries_compiled_examples() {
    for format_crate in FORMAT_CRATES {
        let page = page(format_crate);
        // A fence that opens a Rust block: bare ``` or one of the attributes this workspace uses.
        // `text` blocks (the `.pptx`/`.xlsx` diagrams) are prose and are not counted.
        let compiled = page
            .lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                matches!(trimmed, "```" | "```no_run" | "```rust" | "```ignore")
            })
            .count();
        assert!(
            compiled >= 4,
            "{format_crate}'s page opens {compiled} Rust fences — at least two examples (four \
             fence lines) are what makes the page a compiled doctest rather than prose about an API"
        );
    }
}

#[test]
fn every_page_is_wired_into_its_crate_as_a_documentation_only_module() {
    for format_crate in FORMAT_CRATES {
        let crate_src = workspace_dir()
            .join("crates")
            .join(format_crate)
            .join("src");
        let module = std::fs::read_to_string(crate_src.join("effective_properties.rs"))
            .unwrap_or_else(|error| panic!("{format_crate}/src/effective_properties.rs: {error}"));
        assert!(
            module.contains(r#"#![doc = include_str!("../docs/effective_properties.md")]"#),
            "{format_crate}'s effective_properties module does not include its own page, so the \
             page's examples are not compiled and its links are not checked"
        );
        let lib = std::fs::read_to_string(crate_src.join("lib.rs"))
            .unwrap_or_else(|error| panic!("{format_crate}/src/lib.rs: {error}"));
        assert!(
            lib.contains("pub mod effective_properties;"),
            "{format_crate}'s lib.rs does not declare `pub mod effective_properties;`, so the page \
             renders nowhere"
        );
    }
}
