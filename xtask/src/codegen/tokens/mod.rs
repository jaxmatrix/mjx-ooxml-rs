//! `cargo run -p xtask -- tokens` — the design-token pipeline (MJXOFF-156).
//!
//! # Why there are four artefacts and not one
//!
//! The chrome is HTML and the document canvas is Rust, and **a canvas cannot inherit a CSS custom
//! property**. A token system that stopped at a stylesheet would leave the in-canvas UI — selection
//! handles, guides, rulers, marching ants — visually detached from the application drawn around it.
//! So one source reaches three consumers that share nothing: `ui/tokens/tokens.css` for the
//! Web-Component chrome, `ui/tokens/tokens.ts` for the shell's own logic, and
//! `crates/mjx-tokens/src/generated.rs` for the renderer.
//!
//! The fourth is `ui/tokens/derivations.css` (MJXOFF-271), and it exists because the source has two
//! tiers. Every artefact above carries the **resolved** colour, which is what a canvas, a typed
//! constant and a contrast gate all need — but resolving at generation time freezes it, so a host
//! that sets `--theme-midground` would re-theme nothing. `derivations.css` restates the derived
//! tier as the `color-mix(in srgb, …)` it came from, in terms of the scheme-relative aliases a host
//! actually overrides. Import it after `tokens.css` and the cascade re-derives; the Rust side does
//! the same through `mjx_tokens::Tokens::rederive`, and `ui/tokens/chromium-agreement.mjs` asserts
//! the browser and that one implementation agree about every derived token in both schemes.
//!
//! It is a separate file rather than a second block inside `tokens.css` for two reasons that point
//! the same way: a second declaration of the same property would be a duplicate the drift gates
//! read as a defect, and it would hand every consumer of `tokens.css` a `color-mix()` where it
//! expects a colour.
//!
//! # Why the output is committed
//!
//! The same reason `mjx-ooxml-types` is: a `build.rs` would make the generated Rust invisible in
//! review, unreadable in a stack trace and impossible to diff. `--check` is what keeps a committed
//! artefact honest — it regenerates in memory and refuses if what is on disk is not what the source
//! produces, which is the gate `xtask/tests/tokens.rs` runs.
//!
//! # The trap this pipeline is written against
//!
//! *"The artefacts are generated and committed"* is satisfied by four files nothing reads. All of
//! the real gates are therefore **divergence** gates, and none of them lives here alone:
//!
//! - **Derived, not hand-written** — `--check`, above, plus the emitters' own tests, which add a
//!   token to a source and watch it appear in all of them.
//! - **Equal to each other** — `crates/mjx-tokens/tests/artefacts_agree.rs`, which parses the
//!   emitted CSS and TypeScript and compares every value against the Rust table, and separately
//!   checks that `derivations.css` and `mjx_tokens::DERIVATIONS` describe the same expressions.
//! - **Right about `color-mix()`** — `ui/tokens/chromium-agreement.mjs`, which is the only one that
//!   can tell a correct implementation from a self-consistent one, because the other party to the
//!   comparison is Chromium.

mod emit;
mod model;

use std::path::PathBuf;

use anyhow::{bail, Context, Result};

/// The one hand-edited file in the pipeline.
const SOURCE: &str = "docs/client-platform/data/tokens.json";

/// The four artefacts, relative to the workspace root, in emission order.
const CSS: &str = "ui/tokens/tokens.css";
const DERIVATIONS: &str = "ui/tokens/derivations.css";
const TYPESCRIPT: &str = "ui/tokens/tokens.ts";
const RUST: &str = "crates/mjx-tokens/src/generated.rs";

/// The four rendered artefacts, before anything touches the disk.
pub(crate) struct Artefacts {
    pub(crate) css: String,
    /// The derived tier as `color-mix()` expressions (MJXOFF-271) — the layer that makes a host's
    /// seed override reach every colour mixed from it, through the cascade and with no code.
    pub(crate) derivations: String,
    pub(crate) typescript: String,
    pub(crate) rust: String,
}

/// Reads, checks and renders — the whole pipeline, with no filesystem in it.
pub(crate) fn generate(source: &str) -> Result<Artefacts> {
    let set = model::read(source)?;
    Ok(Artefacts {
        css: emit::css(&set),
        derivations: emit::derivations_css(&set),
        typescript: emit::typescript(&set),
        rust: emit::rust(&set),
    })
}

/// Regenerates the three artefacts, or — with `--check` — refuses if the committed ones are not
/// exactly what the source produces.
///
/// # `--out-dir`
///
/// The source is always read from the workspace, because it is the one committed input. Where the
/// artefacts are *written* — and, with `--check`, which copies are compared — is the **output
/// root**, the workspace unless `--out-dir` names somewhere else.
///
/// That option exists for one reason, and it is worth stating plainly because a flag whose only
/// caller is a test is otherwise a smell: **`xtask/tests/tokens.rs` must be able to exercise the
/// write path without writing into the repository.** A test that regenerates in place truncates
/// `ui/tokens/tokens.css` while `crates/mjx-tokens/tests/artefacts_agree.rs` — a *different test
/// binary*, which `cargo test --workspace` runs as a concurrent process — is reading it. No
/// in-process lock can order two processes, so the only fix that actually closes the hole is for
/// the writer to write somewhere else. See that test's module documentation.
pub fn run(arguments: &[String]) -> Result<()> {
    let mut check = false;
    let mut output_root: Option<PathBuf> = None;
    let mut remaining = arguments.iter();
    while let Some(argument) = remaining.next() {
        match argument.as_str() {
            "--check" => check = true,
            "--out-dir" => {
                let path = remaining.next().with_context(|| {
                    "`--out-dir` needs a directory; usage: `cargo run -p xtask -- tokens \
                     [--check] [--out-dir <directory>]`"
                })?;
                output_root = Some(PathBuf::from(path));
            }
            _ => bail!(
                "unknown arguments {arguments:?}; usage: `cargo run -p xtask -- tokens [--check] \
                 [--out-dir <directory>]`"
            ),
        }
    }

    let root = super::workspace_root();
    let output_root = output_root.unwrap_or_else(|| root.clone());
    let source_path = root.join(SOURCE);
    let source = std::fs::read_to_string(&source_path)
        .with_context(|| format!("reading {}", source_path.display()))?;
    let artefacts = generate(&source).with_context(|| format!("reading {SOURCE}"))?;
    // The Rust artefact goes through `rustfmt` exactly as the schema codegen's does, so
    // `cargo fmt --all --check` and this generator can never disagree about the committed file.
    let rust = super::rustfmt(&artefacts.rust).context("formatting the generated Rust")?;

    let files = [
        (CSS, artefacts.css),
        (DERIVATIONS, artefacts.derivations),
        (TYPESCRIPT, artefacts.typescript),
        (RUST, rust),
    ];

    if check {
        let mut divergences = Vec::new();
        for (relative, expected) in &files {
            let path = output_root.join(relative);
            let actual = std::fs::read_to_string(&path).unwrap_or_default();
            if let Some(report) = first_difference(&actual, expected) {
                divergences.push(format!("{relative}: {report}"));
            }
        }
        if !divergences.is_empty() {
            bail!(
                "the committed design-token artefacts are not what {SOURCE} produces:\n  {}\n\
                 Run `cargo run -p xtask -- tokens` and commit the result. These files are \
                 generated; the source is the only one to edit.",
                divergences.join("\n  ")
            );
        }
        println!("tokens: {} artefacts match {SOURCE}", files.len());
        return Ok(());
    }

    for (relative, contents) in &files {
        let path = output_root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        super::write_plain(&path, contents)?;
    }
    println!("tokens: wrote {CSS}, {DERIVATIONS}, {TYPESCRIPT}, {RUST}");
    Ok(())
}

/// Where two artefacts first differ, in terms a reader can act on: the line number and both lines.
///
/// A bare "these files differ" would be true of a regenerated header as much as of a changed
/// colour, and the first thing anyone would do with it is run a diff by hand.
pub fn first_difference(actual: &str, expected: &str) -> Option<String> {
    if actual == expected {
        return None;
    }
    let mut actual_lines = actual.lines();
    let mut expected_lines = expected.lines();
    let mut at = 1;
    loop {
        match (actual_lines.next(), expected_lines.next()) {
            (None, None) => {
                return Some("the files differ only in their trailing newline".to_owned())
            }
            (Some(left), Some(right)) if left == right => at += 1,
            (left, right) => {
                return Some(format!(
                    "line {at} differs\n    committed: {}\n    generated: {}",
                    left.map_or("<end of file>", str::trim_end),
                    right.map_or("<end of file>", str::trim_end),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed source has to survive its own rules. This is the same read `run` performs,
    /// so a source that would fail the contrast gate fails here first, in a test rather than in a
    /// developer's terminal.
    #[test]
    fn the_committed_source_reads_and_renders() {
        let path = super::super::workspace_root().join(SOURCE);
        let source = std::fs::read_to_string(&path).expect("the committed source is readable");
        let artefacts = generate(&source).expect("the committed source is valid");
        assert!(artefacts.css.contains("--color-green: #2e9e63;"));
        assert!(artefacts.typescript.contains("export interface Palette"));
        assert!(artefacts
            .rust
            .contains("pub const TOKENS: &[TokenIdentity]"));
    }

    /// `--check` is only worth running if it can fail, and it can only fail if this comparison can.
    /// One hex digit is exactly the mutation the ticket asks to be demonstrated by hand.
    #[test]
    fn the_comparison_reports_the_line_a_single_changed_digit_is_on() {
        let generated = "a\n--color-green: #2e9e63;\nc\n";
        let committed = "a\n--color-green: #2e9e64;\nc\n";
        let report = first_difference(committed, generated).expect("must differ");
        assert!(report.contains("line 2 differs"), "{report}");
        assert!(report.contains("#2e9e64"), "{report}");
        assert!(report.contains("#2e9e63"), "{report}");
        assert!(first_difference(generated, generated).is_none());
    }

    #[test]
    fn a_truncated_artefact_is_a_divergence_and_not_a_match() {
        let report = first_difference("a\n", "a\nb\n").expect("must differ");
        assert!(report.contains("<end of file>"), "{report}");
    }
}
