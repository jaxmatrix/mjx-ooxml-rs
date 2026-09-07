//! Every public function, constant, enumeration variant **and struct field** in this crate is named
//! by somebody.
//!
//! # Why this file is a copy, and what would be worse
//!
//! `mjx-paint` and `mjx-layout` carry the same scanner, and MJXOFF-155 §6 asks every crate in this
//! programme for one. It is copied rather than shared because an integration test compiles only
//! into its own crate: a harness in one crate's `tests/` is unreachable from another's, and the
//! alternative is a *published* crate whose only purpose is to be a test — which is the thing
//! `CLAUDE.md`'s three test-only crates are carefully limited to, and which none of them is.
//!
//! # Why a field, and not only a function
//!
//! The defect this instrument exists for is `mjx_scene::SceneMesh::provenance`, which was **written
//! once and read zero times**: not a function nobody called, a field nobody read. The consequence
//! was exactly what this crate ends — a painter could not tell a stand-in shape from the document's
//! own, so a page of placeholders would have compared clean against a golden image of itself.
//!
//! This crate has its own candidates, and they are why the gate is here rather than assumed:
//! [`PresetShapeDefinition::source`](mjx_geometry::PresetShapeDefinition::source) and
//! [`derivation`](mjx_geometry::PresetShapeDefinition::derivation) exist entirely for MJXOFF-203 to
//! read, so a build in which nothing names them is a build in which the next child's main gate has
//! quietly lost its input.
//!
//! # What "reachable" means here, and what it deliberately does not
//!
//! A declaration is reachable when its name appears **somewhere that is not its own declaration** —
//! in `src/`, in `tests/`, or in a doc comment. That is a low bar on purpose: this gate is a net for
//! the symbol nobody has *ever* touched, not a coverage measurement.
//!
//! Two things it cannot see, stated so nobody mistakes green here for more than it is:
//!
//! * **A field that is written and never read** passes if anything names it, including the
//!   constructor that writes it. Catching that needs the identity-value probe, which is what
//!   `the_identity_values_are_not_the_only_values.rs` is, and it is the sharper instrument.
//! * **A trait method** is not scanned: an implementation names it, so every method of every trait
//!   is trivially reachable.
//!
//! # Proved by mutation
//!
//! * Adding `pub const UNUSED_THING: u32 = 1;` to `src/table.rs` → the gate names it.
//! * Adding a variant to `PresetPathStep` that nothing constructs → the gate names it.
//! * Adding a field to `AdjustmentDomain` that nothing reads or writes → the gate names it.
//! * Emptying `tests/` → the count assertion fails, so a gate that scanned nothing cannot pass.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// What a declaration is, for the report.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Kind {
    Callable,
    Constant,
    Variant,
    Field,
}

impl Kind {
    fn describe(self) -> &'static str {
        match self {
            Self::Callable => "function",
            Self::Constant => "constant",
            Self::Variant => "enumeration variant",
            Self::Field => "struct field",
        }
    }
}

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
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

/// The name a `pub fn` or `pub const` line declares.
fn declared_name<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let code = line.trim_start();
    let rest = code.strip_prefix("pub ")?;
    // `pub const fn` is a function, not a constant, and would otherwise be counted twice.
    let rest = rest
        .strip_prefix("const fn ")
        .map(|rest| if keyword == "fn " { Some(rest) } else { None })
        .unwrap_or_else(|| rest.strip_prefix(keyword))?;
    let rest = rest.strip_prefix("async ").unwrap_or(rest);
    let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')?;
    let name = rest.get(..end)?;
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// The name a variant line declares, inside an enumeration body.
fn declared_variant(line: &str) -> Option<&str> {
    let code = line.trim();
    let first = code.chars().next()?;
    if !first.is_ascii_uppercase() {
        return None;
    }
    let end = code
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(code.len());
    let name = code.get(..end)?;
    // A variant line ends in `,`, `{`, `(` or `=`; a type in a field's position would not.
    let tail = code.get(end..)?.trim_start();
    if tail.is_empty()
        || tail.starts_with(',')
        || tail.starts_with('{')
        || tail.starts_with('(')
        || tail.starts_with('=')
    {
        Some(name)
    } else {
        None
    }
}

/// The name a `pub` struct field declares, inside a struct body.
fn declared_field(line: &str) -> Option<&str> {
    let code = line.trim();
    let rest = code.strip_prefix("pub ")?;
    let (name, tail) = rest.split_once(':')?;
    let name = name.trim();
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return None;
    }
    // A field is `pub name: Type,`; a `pub mod`, `pub use` or `pub fn` is caught by the lowercase
    // rule and by the colon, and a `pub trait X: Bound` by the colon coming after a capital.
    let _ = tail;
    Some(name)
}

/// Whether `haystack` names `name` as a whole word.
fn mentions(haystack: &str, name: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut from = 0usize;
    while let Some(offset) = haystack.get(from..).and_then(|rest| rest.find(name)) {
        let at = from + offset;
        let before = at
            .checked_sub(1)
            .and_then(|index| bytes.get(index))
            .copied();
        let after = bytes.get(at + name.len()).copied();
        let is_word = |byte: Option<u8>| {
            byte.is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        };
        if !is_word(before) && !is_word(after) {
            return true;
        }
        from = at + name.len();
    }
    false
}

/// One declaration, and where it was found.
struct Declared {
    name: String,
    kind: Kind,
    file: PathBuf,
    line: usize,
}

/// Scan `sources` for declarations, and everything for mentions of them.
fn scan(sources: &[PathBuf], everything: &[PathBuf]) -> (Vec<Declared>, String) {
    let mut declarations = Vec::new();
    let mut corpus = String::new();
    for path in everything {
        // **This file is not part of the corpus.** Its own documentation names real symbols, and
        // counting those would make the gate its own caller.
        if path
            .file_name()
            .is_some_and(|name| name == "the_public_surface_is_reachable.rs")
        {
            continue;
        }
        let is_source = sources.contains(path);
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let mut enum_depth = 0usize;
        let mut struct_depth = 0usize;
        for (number, line) in text.lines().enumerate() {
            let mut declared = None;
            if is_source {
                if let Some(name) = declared_name(line, "fn ") {
                    declared = Some((name.to_owned(), Kind::Callable));
                } else if let Some(name) = declared_name(line, "const ") {
                    declared = Some((name.to_owned(), Kind::Constant));
                } else if enum_depth == 1 {
                    if let Some(name) = declared_variant(line) {
                        declared = Some((name.to_owned(), Kind::Variant));
                    }
                } else if struct_depth == 1 {
                    if let Some(name) = declared_field(line) {
                        declared = Some((name.to_owned(), Kind::Field));
                    }
                }
            }
            match declared {
                Some((name, kind)) => declarations.push(Declared {
                    name,
                    kind,
                    file: path.clone(),
                    line: number + 1,
                }),
                None => {
                    // The declaration line itself is never evidence of a caller; every other line
                    // is, including a comment — a symbol a doc comment links to has at least been
                    // read by somebody.
                    corpus.push_str(line);
                    corpus.push('\n');
                }
            }

            // Track the bodies *after* the line has been classified, so the `pub enum` or
            // `pub struct` line itself is not mistaken for one of its own members.
            if declared_name(line, "enum ").is_some() && enum_depth == 0 && struct_depth == 0 {
                enum_depth = usize::from(line.contains('{'));
            } else if enum_depth > 0 {
                enum_depth += line.matches('{').count();
                enum_depth = enum_depth.saturating_sub(line.matches('}').count());
            } else if declared_name(line, "struct ").is_some() && struct_depth == 0 {
                struct_depth = usize::from(line.contains('{'));
            } else if struct_depth > 0 {
                struct_depth += line.matches('{').count();
                struct_depth = struct_depth.saturating_sub(line.matches('}').count());
            }
        }
    }
    (declarations, corpus)
}

#[test]
fn every_public_function_constant_variant_and_field_is_named_by_somebody() {
    let root = crate_root();
    let sources = rust_files(&root.join("src"));
    let tests = rust_files(&root.join("tests"));
    assert!(
        sources.len() >= 6 && tests.len() >= 4,
        "the walk found {} source and {} test file(s), which cannot be this crate — a gate that \
         scanned nothing would report nothing unreachable",
        sources.len(),
        tests.len()
    );
    let mut everything = sources.clone();
    everything.extend(tests);

    let (declarations, corpus) = scan(&sources, &everything);
    assert!(
        declarations.len() > 50,
        "only {} declarations were found, which cannot be this crate's surface",
        declarations.len()
    );

    let mut unreachable: Vec<String> = Vec::new();
    for declared in &declarations {
        if mentions(&corpus, &declared.name) {
            continue;
        }
        // A declaration may also be named by *another* declaration line, which the corpus omits.
        // `PaintKind::Solid` named only in `PaintKind::ALL`'s own line is reachable.
        if declarations
            .iter()
            .any(|other| other.name != declared.name && mentions(&other.name, &declared.name))
        {
            continue;
        }
        unreachable.push(format!(
            "{} `{}` at {}:{}",
            declared.kind.describe(),
            declared.name,
            declared.file.display(),
            declared.line
        ));
    }
    unreachable.sort();
    unreachable.dedup();

    assert!(
        unreachable.is_empty(),
        "these public items are declared and named nowhere else in the crate:\n{}\n\n\
         A symbol nothing reaches is either dead — in which case it should go — or is a hand-off \
         waiting to be forgotten, which is how `SceneMesh::provenance` was written once and read \
         zero times. In this crate the hand-off most likely to rot is a seed row's `source` and \
         `derivation`, which exist for MJXOFF-203 and for nothing else.",
        unreachable.join("\n")
    );

    // A summary, so a reviewer can see the gate covers all four kinds rather than trusting it does.
    let kinds: BTreeSet<Kind> = declarations.iter().map(|declared| declared.kind).collect();
    assert_eq!(
        kinds.len(),
        4,
        "the gate must find all four kinds of declaration, and it found {kinds:?} — a scanner that \
         silently stopped seeing fields would pass for ever"
    );
    for kind in [Kind::Callable, Kind::Constant, Kind::Variant, Kind::Field] {
        let count = declarations
            .iter()
            .filter(|declared| declared.kind == kind)
            .count();
        println!("{}: {count}", kind.describe());
        // Two rather than a larger floor: this crate is small and has exactly two public
        // constants — the arc splitting bound and `Derivation::ALL`. The floor is here so a
        // scanner that stopped seeing a *kind* cannot pass, not to assert a surface size.
        assert!(
            count >= 2,
            "only {count} declaration(s) of kind {kind:?}; the scanner has stopped seeing them"
        );
    }
}

#[test]
fn the_gate_can_tell_each_kind_of_declaration_from_the_others() {
    // The instrument, checked. A parser that saw no fields would pass the assertion above for ever
    // while a field sat unread, which is the exact defect this file was added for.
    assert_eq!(
        declared_name("    pub fn to_draw_command(self) -> DrawCommand {", "fn "),
        Some("to_draw_command")
    );
    assert_eq!(
        declared_name("    pub const fn wire(self) -> f32 {", "fn "),
        Some("wire")
    );
    assert_eq!(declared_name("    pub const fn wire(self)", "const "), None);
    assert_eq!(
        declared_name(
            "pub const MAXIMUM_ARC_SEGMENT_RADIANS: f64 = FRAC_PI_2;",
            "const "
        ),
        Some("MAXIMUM_ARC_SEGMENT_RADIANS")
    );
    assert_eq!(declared_name("    fn private(&self) {", "fn "), None);
    assert_eq!(
        declared_name("    pub(crate) fn internal(&self) {", "fn "),
        None
    );

    assert_eq!(declared_variant("    Close,"), Some("Close"));
    assert_eq!(declared_variant("    MoveTo(PresetPoint),"), Some("MoveTo"));
    assert_eq!(declared_variant("    ArcTo {"), Some("ArcTo"));
    assert_eq!(declared_variant("    Refuse = 0,"), Some("Refuse"));
    assert_eq!(declared_variant("    #[default]"), None);
    assert_eq!(declared_variant("    /// A doc comment."), None);
    assert_eq!(
        declared_variant("    pub width: u32,"),
        None,
        "a field is not a variant"
    );

    assert_eq!(declared_field("    pub width: u32,"), Some("width"));
    assert_eq!(
        declared_field("    pub wire_name: String,"),
        Some("wire_name")
    );
    assert_eq!(declared_field("    width: u32,"), None, "a private field");
    assert_eq!(
        declared_field("pub trait GeometryProvider: Send {"),
        None,
        "a trait's supertrait bound is not a field"
    );
    assert_eq!(declared_field("pub mod table;"), None);
    assert_eq!(
        declared_field("pub use table::{definition_of};"),
        None,
        "a re-export is not a field"
    );

    assert!(mentions("let a = width + 1;", "width"));
    assert!(!mentions("let a = widths + 1;", "width"));
    assert!(!mentions("let a = my_width;", "width"));
}
