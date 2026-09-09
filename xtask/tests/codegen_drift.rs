//! **Does the committed `mjx-ooxml-types` source still say what the generator would say?**
//! (MJXOFF-224.)
//!
//! # The hole this file exists to close
//!
//! `CLAUDE.md` decides that generated output is *committed, never a `build.rs`*, regenerated on
//! demand with `cargo run -p xtask -- codegen` against a local `References/` tree. That is a
//! deliberate and defensible choice — a `build.rs` would put a 5,000-page specification and a
//! `rustfmt` invocation on every consumer's critical path. Its consequence had never been written
//! down, still less checked:
//!
//! > **Nothing re-derived the committed output, so a generator defect was frozen into the
//! > repository rather than failing on the next build — and the committed file is the only artefact
//! > anyone reads, which makes a defect indistinguishable from a deliberate choice.**
//!
//! `crates/mjx-ooxml-types/src/generated/` is 79,000 lines, every one of them written by
//! `xtask/src/codegen/`. Between the moment a generator change lands and the moment somebody
//! remembers to run `codegen`, the two disagree and nothing says so. The compiler catches the
//! *structural* half of that — rename a generated enum by hand and `mjx-ooxml-types` stops
//! compiling. It catches none of the rest: a wire token, a doc comment recording the original
//! `ST_*` symbol, a rank in a child-order table, a row of `COVERAGE.md`. Those are the parts a
//! reader trusts and no build touches.
//!
//! # Two tiers, because only one of them can run on CI
//!
//! The full check needs the schemas, and **CI never downloads them**:
//! `.github/scripts/fetch-ecma-schemas.sh`'s `ARCHIVES` holds ECMA-376 Part 4 (Transitional) and
//! Part 2 (OPC), while this generator also needs Part 1 — the Strict schema set for the namespace
//! table and `presetShapeDefinitions.xml` for the preset-shape adjustment tables. So:
//!
//! 1. [`the_committed_output_is_what_the_generator_produces_today`] regenerates every artefact in
//!    memory and compares it byte for byte with what is committed. It **skips** when `References/`
//!    is incomplete, the pattern `schema_validity.rs` established; `MJX_REQUIRE_CODEGEN=1` turns
//!    the absence into a failure. This is the whole answer, and it is local-or-gated.
//! 2. Every other test here re-derives the parts of the committed output that need **no schema at
//!    all**, from the generator's own tables and from the committed files. Those run on every push,
//!    and they are what stands between the full check's two runs.
//!
//! Growing `ARCHIVES` so tier 1 can run on CI belongs to MJXOFF-197, which owns the archive change
//! for the same reason (`crates/mjx-dml/tests/guide_formula.rs`'s preset-geometry sweep needs the
//! same Part 1 download). This file states the dependency; it does not take it.
//!
//! # The vacuity trap, in this file's own terms
//!
//! > *A check with nothing to compare passes exactly as a working one does.*
//!
//! Tier 1 skips when the schemas are absent, and a skip is green. Tier 2's derivations are over
//! sets — the module table, the allowlists, the schema stems — and **a set that has become empty
//! passes every assertion over it**. So every test here prints its counts on success, and every
//! loop that could be empty is floored. The floors are phrased as *the walk is still finding
//! things*, never as *the corpus is exactly this size*, so that a floor cannot fire in place of the
//! assertion it guards — `doc_gate.rs` states the same rule and the reason for it.
//!
//! # The mutation register
//!
//! Every test below was made to fail, and each mutation had to be **reachable** — the workspace had
//! to still compile with it applied, or the failure proves nothing about this file.
//!
//! **The first attempt was not reachable, and it is the most useful entry here.** Renaming a
//! generated enum in `crates/mjx-ooxml-types/src/generated/officemath.rs` broke the *build* — three
//! `E0425`s from the `impl` blocks beside it — so the check never ran. That is not a gap: it is the
//! boundary. **The compiler already owns structural edits to generated source; this file owns
//! everything else**, and a later reader should not assume the two overlap. The mutation that does
//! reach the check is a one-word edit to a doc comment, which compiles cleanly and is invisible to
//! every other gate in the workspace.
//!
//! | Mutation (all still compile) | Fires |
//! |---|---|
//! | one word added to a `/// \`ST_Style\`` doc line in the committed `officemath.rs` | the byte-for-byte check, and the `COVERAGE.md` count (the type is no longer parsed) |
//! | `pub(crate) mod drawingml;` widened to `pub mod drawingml;` in the committed `generated/mod.rs` | the module-root check, naming both maps |
//! | the `@generated` banner replaced on `shared.rs` | the directory check |
//! | `all 96 simple types` → `all 95` in `COVERAGE.md` | the count check, naming `spreadsheetml.rs` |
//! | `dml-lockedCanvas`'s child-order row rewritten as `generated — every complex type` | the child-order check |
//! | a `pub type FakeMeasure = i32;` appended to the committed `drawingml.rs` | the curated-re-export check |
//! | `shared`'s `visibility` in `SIMPLE_TYPE_MODULES` set to `pub(crate)` (MJXOFF-225) | the curated-re-export check, which now reaches a third module because it derives its population from that field rather than naming two |

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use xtask::codegen::{self, emit::Selection, CHILD_ORDER_SCHEMAS, SIMPLE_TYPE_MODULES};

/// The workspace root — `xtask/`'s parent.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent directory")
        .to_path_buf()
}

/// The committed generated directory.
fn generated_dir() -> PathBuf {
    workspace_root().join("crates/mjx-ooxml-types/src/generated")
}

/// Reads a committed file, failing with its path rather than with `No such file or directory`.
fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

// ===============================================================================================
// Tier 1 — the whole answer, when the schemas are here
// ===============================================================================================

/// Regenerates every artefact and compares it with what is committed, byte for byte.
///
/// This is the only check that can see a generator change nobody re-ran `codegen` after. It is
/// also the only one that needs `References/`, so it skips when the tree is incomplete and
/// `MJX_REQUIRE_CODEGEN=1` makes that absence a failure instead.
///
/// **What it catches:** any difference at all between the committed bytes and today's generator —
/// a hand-edit of a generated file (including one that still compiles, which is most of them), a
/// naming-table change never regenerated, a schema set that has moved underneath us.
///
/// **What it cannot:** whether the generator is *right*. It compares the committed output against
/// the generator, not against ECMA-376. A defect in `emit.rs` that has always been there is exactly
/// as green here as correctness is. `crates/mjx-ooxml-types/tests/wire.rs`,
/// `crates/mjx-ooxml-types/tests/adjustments.rs` and the schema-validity gates are what test the
/// output against the standard; this one tests it against its own producer.
#[test]
fn the_committed_output_is_what_the_generator_produces_today() {
    let root = workspace_root();
    if !codegen::references_are_present(&root) {
        assert!(
            std::env::var_os("MJX_REQUIRE_CODEGEN").is_none(),
            "MJX_REQUIRE_CODEGEN is set, but the local References/ tree at {} is incomplete — \
             codegen needs ECMA-376 Part 1 (Strict schemas + presetShapeDefinitions.xml) and \
             Part 4 (Transitional schemas)",
            root.join("References").display()
        );
        eprintln!(
            "skipping the codegen drift check: References/ is incomplete at {}",
            root.join("References").display()
        );
        return;
    }

    let artefacts = match codegen::artefacts(&root) {
        Ok(artefacts) => artefacts,
        Err(e) if format!("{e:#}").contains("rustfmt") => {
            assert!(
                std::env::var_os("MJX_REQUIRE_CODEGEN").is_none(),
                "MJX_REQUIRE_CODEGEN is set, but rustfmt could not be run: {e:#}"
            );
            eprintln!("skipping the codegen drift check: rustfmt is not available ({e:#})");
            return;
        }
        Err(e) => panic!("the generator failed: {e:#}"),
    };

    // The anti-vacuity floor: phrased as *the generator is still emitting a file per module plus
    // the four fixed artefacts*, so it cannot fire in place of the comparison below.
    assert!(
        artefacts.len() >= SIMPLE_TYPE_MODULES.len() + 4,
        "the generator produced only {} artefact(s) for {} simple-type modules — it has stopped \
         emitting, and the comparison below would pass over nothing",
        artefacts.len(),
        SIMPLE_TYPE_MODULES.len()
    );

    let mut stale = Vec::new();
    let mut bytes = 0usize;
    for artefact in &artefacts {
        let committed = read(&artefact.path);
        bytes += committed.len();
        if committed != artefact.contents {
            stale.push(
                artefact
                    .path
                    .strip_prefix(&root)
                    .unwrap_or(&artefact.path)
                    .display()
                    .to_string(),
            );
        }
    }
    assert!(
        stale.is_empty(),
        "{} of {} committed artefact(s) are not what the generator produces today:\n  {}\n\
         Run `cargo run -p xtask -- codegen` and commit the result.",
        stale.len(),
        artefacts.len(),
        stale.join("\n  ")
    );
    println!(
        "codegen drift: {} artefact(s), {bytes} committed bytes, all identical to today's generator",
        artefacts.len()
    );
}

// ===============================================================================================
// Tier 2 — what can be re-derived with no schemas at all
// ===============================================================================================

/// Every `UNCOVERED_SCHEMAS` row, and every note on one, is still reachable — MJXOFF-88 §9 B10.
///
/// That table writes prose straight into `COVERAGE.md`, a shipped document. Until MJXOFF-224 the
/// only things checked about a row were that its stem exists in the Transitional set and that no
/// stem appears twice, so **nothing failed when a row's claim stopped being true**: a note reading
/// `not modelled` could outlive the schema being generated, and a note reading `generated — every
/// complex type` could outlive the schema leaving `CHILD_ORDER_SCHEMAS`, at which point the
/// document would report coverage that does not exist. See
/// [`codegen::check_uncovered_schemas_are_live`] for what is and is not enforced — in particular
/// that it cannot tell you a note's *sentence* has stopped being true.
///
/// The generator calls the same function, so a regeneration refuses too; this is what runs it on a
/// machine with no schemas.
#[test]
fn every_uncovered_schema_row_is_still_reachable() {
    assert!(
        codegen::UNCOVERED_SCHEMAS.len() >= 10,
        "only {} uncovered-schema row(s) — the table has emptied, and a check over an empty table \
         passes exactly as a working one does",
        codegen::UNCOVERED_SCHEMAS.len()
    );
    codegen::check_uncovered_schemas_are_live()
        .unwrap_or_else(|e| panic!("`UNCOVERED_SCHEMAS` carries a claim nothing can reach: {e:#}"));
    println!(
        "UNCOVERED_SCHEMAS: {} rows, every one still reachable from COVERAGE.md",
        codegen::UNCOVERED_SCHEMAS.len()
    );
}

/// The committed `generated/mod.rs` declares exactly the modules the generator's own table names,
/// with the visibility that table gives them.
///
/// `generated/mod.rs` is rendered from [`SIMPLE_TYPE_MODULES`] and nothing else, so this comparison
/// needs no schema. It is what stops a module being added to the table, or removed from it, without
/// the regeneration that would make it reachable — the failure the file's own header comment
/// (*"a `pub(crate)` module is re-exported item by item"*) assumes cannot happen.
#[test]
fn the_committed_module_root_declares_exactly_the_generator_s_modules() {
    let committed = read(&generated_dir().join("mod.rs"));
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for line in committed.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("pub(crate) mod ") {
            found.insert(
                rest.trim_end_matches(';').to_owned(),
                "pub(crate)".to_owned(),
            );
        } else if let Some(rest) = line.strip_prefix("pub mod ") {
            found.insert(rest.trim_end_matches(';').to_owned(), "pub".to_owned());
        }
    }
    // `child_order` and `namespaces` are emitted unconditionally rather than from the table.
    let mut expected: BTreeMap<String, String> = BTreeMap::new();
    expected.insert("child_order".to_owned(), "pub(crate)".to_owned());
    expected.insert("namespaces".to_owned(), "pub".to_owned());
    for module in SIMPLE_TYPE_MODULES {
        expected.insert(module.module.to_owned(), module.visibility.to_owned());
    }
    assert!(
        found.len() >= 3,
        "only {} module declaration(s) were parsed out of the committed generated/mod.rs — the \
         parser has stopped matching",
        found.len()
    );
    assert_eq!(
        found, expected,
        "the committed generated/mod.rs and the generator's module table disagree"
    );
    println!(
        "committed generated/mod.rs: {} module declarations, all matching SIMPLE_TYPE_MODULES",
        found.len()
    );
}

/// The committed generated directory holds exactly the files the generator writes, and every one of
/// them carries the `@generated` banner.
///
/// A file left behind by a module that was removed from the table keeps compiling and keeps being
/// read; a generated file that has lost its banner reads as hand-written source somebody may then
/// edit. Neither is visible to anything else in the workspace. The expected set is derived from
/// [`SIMPLE_TYPE_MODULES`], so it needs no schema.
#[test]
fn the_committed_generated_directory_holds_exactly_the_generated_files() {
    let dir = generated_dir();
    let mut on_disk: BTreeSet<String> = BTreeSet::new();
    for entry in std::fs::read_dir(&dir).expect("the generated directory exists") {
        let name = entry.expect("a readable directory entry").file_name();
        on_disk.insert(name.to_string_lossy().into_owned());
    }
    let mut expected: BTreeSet<String> = ["mod.rs", "child_order.rs", "namespaces.rs"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    for module in SIMPLE_TYPE_MODULES {
        expected.insert(format!("{}.rs", module.module));
    }
    assert_eq!(
        on_disk, expected,
        "the committed generated/ directory and the generator's file set disagree"
    );

    let mut banners = 0;
    for name in &on_disk {
        let source = read(&dir.join(name));
        assert!(
            source.starts_with("// @generated by xtask — do not edit."),
            "{name} does not open with the @generated banner, so it reads as hand-written source"
        );
        banners += 1;
    }
    assert!(
        banners >= SIMPLE_TYPE_MODULES.len(),
        "only {banners} generated file(s) were checked for a banner — the walk has stopped finding \
         them"
    );
    println!("committed generated/: {banners} files, every one carrying the @generated banner");
}

/// `COVERAGE.md`'s counts agree with the committed module files they describe.
///
/// `COVERAGE.md` is generated, and its simple-type column is a *count* — `generated — all 96 simple
/// types`. A count is a fact that expires, and this is the only document in the workspace that
/// states these nine. Re-deriving each from the committed module file (one `/// \`ST_…\` — ` doc
/// line per emitted type) closes the gap between the two committed artefacts without needing the
/// schema either of them came from, so the two cannot drift between regenerations.
#[test]
fn the_coverage_document_s_counts_match_the_committed_modules() {
    let coverage = read(&workspace_root().join("crates/mjx-ooxml-types/COVERAGE.md"));
    let mut checked = 0;
    for module in SIMPLE_TYPE_MODULES {
        let source = read(&generated_dir().join(format!("{}.rs", module.module)));
        let emitted = source
            .lines()
            .filter(|l| l.starts_with("/// `ST_") && l.contains("` — "))
            .count();
        assert!(
            emitted > 0,
            "no emitted type was parsed out of the committed {}.rs — the parser has stopped \
             matching",
            module.module
        );
        let row = coverage
            .lines()
            .find(|l| l.starts_with(&format!("| {} | ", module.stem)))
            .unwrap_or_else(|| panic!("COVERAGE.md has no simple-type row for `{}`", module.stem));
        match module.selection {
            Selection::Everything => assert!(
                row.contains(&format!("generated — all {emitted} simple types")),
                "the committed {}.rs holds {emitted} types, but COVERAGE.md says: {row}",
                module.module
            ),
            Selection::Allowlist(list) => {
                assert_eq!(
                    emitted,
                    list.len(),
                    "the committed {}.rs holds {emitted} types but its allowlist names {}",
                    module.module,
                    list.len()
                );
                assert!(
                    row.contains(&format!("partial ({emitted} of the schema's simple types")),
                    "the committed {}.rs holds {emitted} types, but COVERAGE.md says: {row}",
                    module.module
                );
                for name in list {
                    assert!(
                        row.contains(&format!("`{name}`")),
                        "COVERAGE.md's `{}` row does not name the allowlisted `{name}`: {row}",
                        module.stem
                    );
                }
            }
        }
        checked += 1;
    }
    assert_eq!(
        checked,
        SIMPLE_TYPE_MODULES.len(),
        "not every module was checked against COVERAGE.md"
    );
    println!(
        "COVERAGE.md: {checked} simple-type rows, every count re-derived from its module file"
    );
}

/// `COVERAGE.md`'s child-order table reports `generated` for exactly the schemas the generator
/// generates a table for.
///
/// The child-order tables are 59,000 of the crate's 79,000 generated lines and no other document
/// says which schemas they cover. Deriving the claim from [`CHILD_ORDER_SCHEMAS`] means a schema
/// cannot join or leave that list and leave the shipped document saying otherwise — which is the
/// half of `COVERAGE.md` that `mjx-schema-gate`'s
/// `the_declared_owners_agree_with_the_generated_coverage_document` already reads in the other
/// direction, for the schemas *it* categorises.
#[test]
fn the_coverage_document_s_child_order_table_matches_the_generated_schemas() {
    let coverage = read(&workspace_root().join("crates/mjx-ooxml-types/COVERAGE.md"));
    let (_, child_order) = coverage
        .split_once("## Child order")
        .expect("COVERAGE.md has a child-order section");
    let mut generated: BTreeSet<&str> = BTreeSet::new();
    let mut rows = 0;
    for line in child_order.lines() {
        let Some(rest) = line.strip_prefix("| ") else {
            continue;
        };
        let Some((stem, status)) = rest.split_once(" | ") else {
            continue;
        };
        if stem == "Schema" || stem.starts_with("---") {
            continue;
        }
        rows += 1;
        if status.starts_with("generated —") {
            generated.insert(stem);
        }
    }
    assert!(
        rows > generated.len(),
        "only {rows} child-order row(s) were parsed and all of them are `generated` — the parser \
         has stopped matching the uncovered rows"
    );
    assert_eq!(
        generated,
        CHILD_ORDER_SCHEMAS.iter().copied().collect::<BTreeSet<_>>(),
        "COVERAGE.md's child-order table and CHILD_ORDER_SCHEMAS disagree"
    );
    println!(
        "COVERAGE.md: {rows} child-order rows, {} of them generated, matching CHILD_ORDER_SCHEMAS",
        generated.len()
    );
}

/// Every enumeration whose variant names are curated cites the ECMA-376 section they came from.
///
/// `CLAUDE.md` requires that a name whose meaning is not inferable from its token be **sourced from
/// the ECMA-376 Part 1 prose — never guessed**. Nothing in this workspace can read that prose: it
/// ships as a PDF outside a git-ignored tree. So the citation *is* the audit trail, and a curated
/// name with no citation is indistinguishable from a guess.
///
/// The rule is per enumeration rather than per row, because that is how the tables are actually
/// written: one comment introduces a type and names its section, and the rows follow, sometimes with
/// un-cited sub-comments between them (`ST_TextAutonumberScheme` has three). So a type counts as
/// cited when **at least one of its rows sits directly under a comment run containing a `§`**.
///
/// **Type overrides are deliberately outside this gate.** `CLAUDE.md`'s rule for a *type* name is to
/// drop `ST_`/`CT_` and expand abbreviations to full words — mechanical, and needing no prose. Only
/// variant names carry the "source it, do not guess it" obligation.
///
/// **What this cannot do** is tell you the name a cited row chose is the name the cited section
/// gives. An audit against the PDF for MJXOFF-224 found two places where it is not, and both are
/// recorded in `crates/mjx-ooxml-types/docs/guide/what_to_distrust.md` rather than changed, because
/// renaming a generated variant is an API break.
#[test]
fn every_curated_enumeration_cites_the_spec_section_its_names_came_from() {
    let source = read(&workspace_root().join("xtask/src/codegen/spec.rs"));

    let mut table: Option<String> = None;
    let mut run = String::new();
    let mut previous_line_was_comment = false;
    let mut row = String::new();
    let mut cited: BTreeSet<(String, String)> = BTreeSet::new();
    let mut seen: Vec<(String, String)> = Vec::new();
    let mut tables = 0;

    for line in source.lines() {
        let trimmed = line.trim();
        let Some(current) = table.clone() else {
            if let Some(rest) = trimmed
                .strip_prefix("const ")
                .or(trimmed.strip_prefix("pub const "))
            {
                if let Some(name) = rest.split(':').next() {
                    if name.ends_with("VARIANT_OVERRIDES") {
                        table = Some(name.to_owned());
                        tables += 1;
                        run.clear();
                        previous_line_was_comment = false;
                        row.clear();
                    }
                }
            }
            continue;
        };
        if trimmed == "];" {
            table = None;
            continue;
        }
        if trimmed.starts_with("//") {
            // A comment run *replaces* the governing one only when it starts a new run.
            if !previous_line_was_comment {
                run.clear();
            }
            run.push_str(trimmed);
            previous_line_was_comment = true;
            continue;
        }
        previous_line_was_comment = false;
        if trimmed.is_empty() {
            continue;
        }
        if !row.is_empty() {
            row.push(' ');
        }
        row.push_str(trimmed);
        let balanced = row.matches('(').count() <= row.matches(')').count();
        if !(balanced && row.ends_with(',')) {
            continue;
        }
        if let Some(symbol) = simple_type_symbol(&row) {
            let key = (current.clone(), symbol);
            if !seen.contains(&key) {
                seen.push(key.clone());
            }
            if run.contains('§') {
                cited.insert(key);
            }
        }
        row.clear();
    }

    // Anti-vacuity: phrased as *the parser is still finding tables and types*, not as *there are
    // exactly this many*, so it cannot fire in place of the assertion below.
    assert!(
        tables >= 4 && seen.len() >= 80,
        "only {tables} override table(s) and {} enumeration(s) were parsed out of spec.rs — the \
         parser has stopped matching, and the check below would pass over almost nothing",
        seen.len()
    );

    let uncited: Vec<String> = seen
        .iter()
        .filter(|key| !cited.contains(key))
        .map(|(table, symbol)| format!("{symbol} (in {table})"))
        .collect();
    assert!(
        uncited.is_empty(),
        "{} curated enumeration(s) name variants with no ECMA-376 section cited above any of their \
         rows, so nothing distinguishes a name read out of the prose from one invented:\n  {}",
        uncited.len(),
        uncited.join("\n  ")
    );
    println!(
        "naming overrides: {} curated enumerations across {tables} tables, every one citing its \
         ECMA-376 section",
        seen.len()
    );
}

/// The `ST_*` symbol a variant-override row names, if it is one.
fn simple_type_symbol(row: &str) -> Option<String> {
    let start = row.find("\"ST_")? + 1;
    let rest = &row[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

/// Every hand-written curation module re-exports **every** item its generated module declares.
///
/// A module emitted `pub(crate)` is re-exported item by item through a hand-written
/// `crates/mjx-ooxml-types/src/<module>.rs`, so that the crate's public surface is curated rather
/// than whatever the generator happens to emit — [`SIMPLE_TYPE_MODULES`]'s `visibility` field is
/// that decision, and it is what this test reads the population out of. The re-export lists are
/// hand-written, and nothing else fails when the generator emits a type that never reaches them:
/// the item simply becomes unreachable, silently, exactly as if the allowlist had never grown. Both
/// directions are checked, because a name in the list that the generator no longer emits would not
/// compile but a name it emits and the list omits would.
///
/// **The population is derived, not listed.** Until MJXOFF-225 this loop opened with the literal
/// pair `("drawingml", …), ("presentationml", …)` — correct when it was written, and blind to a
/// third module the day one is emitted `pub(crate)`. That is the shape MJXOFF-224 found in
/// `child_order.rs`'s four suites, and `xtask/tests/derived_rosters.rs` is what now looks for it.
#[test]
fn the_curated_re_exports_cover_every_generated_item() {
    let curated: Vec<(&str, String)> = SIMPLE_TYPE_MODULES
        .iter()
        .filter(|module| module.visibility == "pub(crate)")
        .map(|module| {
            (
                module.module,
                format!("crates/mjx-ooxml-types/src/{}.rs", module.module),
            )
        })
        .collect();
    assert!(
        !curated.is_empty(),
        "no module in SIMPLE_TYPE_MODULES is emitted `pub(crate)`, so this test compares nothing \
         — either the curation decision was withdrawn without deleting this test, or the \
         visibility field has stopped saying `pub(crate)`"
    );

    let mut checked = 0;
    for (module, hand_written) in &curated {
        let (module, hand_written) = (*module, hand_written.as_str());
        let generated = read(&generated_dir().join(format!("{module}.rs")));
        let declared: BTreeSet<&str> = generated
            .lines()
            .filter_map(|l| {
                let rest = l
                    .strip_prefix("pub enum ")
                    .or_else(|| l.strip_prefix("pub struct "))
                    .or_else(|| l.strip_prefix("pub type "))
                    .or_else(|| l.strip_prefix("pub fn "))?;
                Some(
                    rest.split(|c: char| !c.is_alphanumeric() && c != '_')
                        .next()
                        .unwrap_or(rest),
                )
            })
            .collect();
        assert!(
            !declared.is_empty(),
            "no public item was parsed out of the committed {module}.rs — the parser has stopped \
             matching"
        );

        let source = read(&workspace_root().join(hand_written));
        let list = source
            .split_once(&format!("pub use crate::generated::{module}::{{"))
            .unwrap_or_else(|| panic!("{hand_written} has no re-export of generated::{module}"))
            .1;
        let list = list
            .split_once("};")
            .expect("the re-export list is terminated")
            .0;
        let re_exported: BTreeSet<&str> = list
            .split(',')
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .collect();

        assert_eq!(
            declared, re_exported,
            "the generated `{module}` module and the curated re-export in {hand_written} disagree"
        );
        checked += declared.len();
    }
    assert!(
        checked >= 30,
        "only {checked} curated re-export(s) were compared — the walk has stopped finding them"
    );
    println!(
        "curated re-exports: {checked} items across {} `pub(crate)` module(s) of the {} \
         SIMPLE_TYPE_MODULES declares, each matching its generated module exactly",
        curated.len(),
        SIMPLE_TYPE_MODULES.len()
    );
}
