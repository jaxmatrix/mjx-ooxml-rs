//! Every public function, constant **and enum variant** this crate exports has a caller.
//!
//! # Why this gate, and why it goes further than R05's
//!
//! MJXOFF-155 §9 item 10 records *dead public API* as a **pattern** rather than an instance, and
//! says the cheap systemic fix is a gate rather than a list. `mjx-layout` built the first one
//! (`crates/mjx-layout/tests/public_surface_is_reachable.rs`) and it found four orphans on its first
//! run.
//!
//! It counts functions and constants and **does not see enum variants**, and that is the larger
//! hole. A crate like this one is mostly enumerations: nine commands, four paint kinds, seven effect
//! kinds, eleven dash patterns, fifty-four preset patterns. A variant nobody constructs and nobody
//! matches is a claim about the vocabulary that has never been exercised — and unlike a `pub fn`, it
//! costs nothing to add, so they accumulate silently. This gate sees them.
//!
//! # What "reachable" means here, and the two different rules
//!
//! * A **function or constant** is reachable when its name appears somewhere other than its own
//!   declaration. A method call is `.name(`, so a bare-identifier search is the right rule.
//! * A **variant** is reachable when its name appears after a `::` — `Command::Pop`, `Self::Blank`.
//!   The stricter rule is necessary, not decorative: several enumerations here have a `None`
//!   variant, and a bare-identifier search would count every `Option::None` in the crate as its
//!   caller and pass a variant nobody has ever constructed.
//!
//! Both are **name** checks rather than call-graph analysis. They cannot tell a call from a mention
//! in a `use`, and they cannot tell `Self::None` on one enumeration from `Self::None` on another.
//! What they catch is the thing that actually happens — a symbol produced, exported, and then never
//! written down again anywhere — and they catch it on the next `cargo test` rather than on the next
//! audit.
//!
//! # The complement is the point
//!
//! **The gate is a reason to keep the surface small.** Every `pub fn` and every variant here either
//! has a caller and a test or has to be deleted, so the crate cannot accumulate a complete-looking
//! API nobody has run. That is also why this crate has *one* [`ResourceIndex`](mjx_scene::ResourceIndex)
//! rather than nine typed index newtypes.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Names that are allowed to have no caller, each with the reason.
///
/// **Kept empty on purpose.** A list is what this gate exists to replace; an entry here is a
/// standing exception and should be argued for in the same breath as it is added.
const ALLOWED_WITHOUT_A_CALLER: &[(&str, &str)] = &[];

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

/// The identifier that follows `keyword` on `line`, if the line declares one publicly.
fn declared_name<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let code = line.trim_start();
    let rest = code.strip_prefix("pub ")?;
    let rest = rest.strip_prefix(keyword).or_else(|| {
        // `pub const fn` is a function, not a constant, and would otherwise be counted twice.
        rest.strip_prefix("const ")?.strip_prefix(keyword)
    })?;
    let name: &str = rest
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .find(|piece| !piece.is_empty())?;
    Some(name)
}

/// The variant `line` declares, if it is a variant line inside an enumeration body.
///
/// A variant is an identifier beginning with a capital, at the start of the line's code, followed by
/// a comma, a brace, a parenthesis or an `=`. That excludes attributes, doc comments, the `impl`
/// lines the scanner never sees inside an enumeration body, and the `Self::…` of a `match`.
fn declared_variant(line: &str) -> Option<&str> {
    let code = line.trim_start();
    if code.starts_with("//") || code.starts_with('#') {
        return None;
    }
    let mut characters = code.char_indices();
    let (_, first) = characters.next()?;
    if !first.is_ascii_uppercase() {
        return None;
    }
    let end = characters
        .find(|(_, character)| !character.is_alphanumeric() && *character != '_')
        .map(|(at, _)| at)?;
    let name = code.get(..end)?;
    let rest = code.get(end..)?.trim_start();
    if rest.starts_with(',')
        || rest.starts_with('{')
        || rest.starts_with('(')
        || rest.starts_with('=')
    {
        Some(name)
    } else {
        None
    }
}

/// Whether `haystack` contains `name` as a whole identifier rather than as a substring.
fn mentions(haystack: &str, name: &str) -> bool {
    positions(haystack, name).next().is_some()
}

/// Whether `haystack` contains `name` as a whole identifier **preceded by `::`**.
fn mentions_as_a_path(haystack: &str, name: &str) -> bool {
    positions(haystack, name).any(|start| {
        start >= 2
            && haystack
                .get(start - 2..start)
                .is_some_and(|before| before == "::")
    })
}

/// Where `name` occurs in `haystack` as a whole identifier.
fn positions<'a>(haystack: &'a str, name: &'a str) -> impl Iterator<Item = usize> + 'a {
    let mut from = 0_usize;
    std::iter::from_fn(move || {
        while let Some(offset) = haystack.get(from..)?.find(name) {
            let start = from + offset;
            let end = start + name.len();
            from = end;
            let before = haystack
                .get(..start)?
                .chars()
                .next_back()
                .is_none_or(|character| !character.is_alphanumeric() && character != '_');
            let after = haystack
                .get(end..)?
                .chars()
                .next()
                .is_none_or(|character| !character.is_alphanumeric() && character != '_');
            if before && after {
                return Some(start);
            }
        }
        None
    })
}

/// What one file's scan produced: the declarations in it, and every line that could mention one.
struct Scan {
    /// `(name, path, line, kind)`.
    declarations: Vec<(String, PathBuf, usize, Kind)>,
    corpus: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    /// A `pub fn` or a `pub const fn`.
    Callable,
    /// A `pub const`.
    Constant,
    /// A variant of a `pub enum`.
    Variant,
}

impl Kind {
    fn describe(self) -> &'static str {
        match self {
            Self::Callable => "function",
            Self::Constant => "constant",
            Self::Variant => "enum variant",
        }
    }
}

/// Scan `sources` for declarations and `everything` for mentions of them.
fn scan(sources: &[PathBuf], everything: &[PathBuf]) -> Scan {
    let mut declarations = Vec::new();
    let mut corpus = String::new();
    for path in everything {
        // **This file is not part of the corpus.** Its self-check names real variants
        // (`FillStyle::None`, `Command::Pop`) as `::`-prefixed strings, and counting those would
        // make the gate its own caller.
        if path
            .file_name()
            .is_some_and(|name| name == "public_surface_is_reachable.rs")
        {
            continue;
        }
        let is_source = sources.contains(path);
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
        // How deep inside a `pub enum` body the scanner currently is; zero means it is not in one.
        let mut enum_depth = 0_usize;
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
                }
            }
            if let Some((name, kind)) = declared {
                declarations.push((name, path.clone(), number + 1, kind));
            } else {
                // The declaration line itself is never evidence of a caller; every other line is,
                // including a comment, because a symbol a doc comment links to has at least been
                // read by somebody.
                corpus.push_str(line);
                corpus.push('\n');
            }

            // Track the enumeration body *after* the line has been classified, so the `pub enum`
            // line itself is not mistaken for a variant.
            if declared_name(line, "enum ").is_some() && enum_depth == 0 {
                enum_depth = usize::from(line.contains('{'));
            } else if enum_depth > 0 {
                enum_depth += line.matches('{').count();
                enum_depth = enum_depth.saturating_sub(line.matches('}').count());
            }
        }
    }
    Scan {
        declarations,
        corpus,
    }
}

#[test]
fn every_public_function_constant_and_variant_has_a_caller() {
    let root = crate_root();
    let sources = rust_files(&root.join("src"));
    let tests = rust_files(&root.join("tests"));
    assert!(
        sources.len() >= 10 && tests.len() >= 4,
        "the walk found {} source and {} test file(s), which cannot be this crate",
        sources.len(),
        tests.len()
    );

    let everything: Vec<PathBuf> = sources.iter().chain(tests.iter()).cloned().collect();
    let Scan {
        declarations,
        corpus,
    } = scan(&sources, &everything);

    let functions = declarations
        .iter()
        .filter(|(_, _, _, kind)| *kind == Kind::Callable)
        .count();
    let variants = declarations
        .iter()
        .filter(|(_, _, _, kind)| *kind == Kind::Variant)
        .count();
    assert!(
        functions > 60,
        "only {functions} public functions were found; the scan is not reading the crate"
    );
    assert!(
        variants > 100,
        "only {variants} public enum variants were found, and the 54 preset patterns alone are more \
         than that — the variant scan is not seeing enumeration bodies"
    );

    let exempt: BTreeSet<&str> = ALLOWED_WITHOUT_A_CALLER
        .iter()
        .map(|(name, _)| *name)
        .collect();
    let mut orphans: Vec<String> = Vec::new();
    for (name, path, line, kind) in &declarations {
        if exempt.contains(name.as_str()) {
            continue;
        }
        let reachable = match kind {
            Kind::Callable | Kind::Constant => mentions(&corpus, name),
            Kind::Variant => mentions_as_a_path(&corpus, name),
        };
        if !reachable {
            orphans.push(format!(
                "{}:{line} — the {} `{name}` is exported and named nowhere else",
                path.display(),
                kind.describe()
            ));
        }
    }

    assert!(
        orphans.is_empty(),
        "this crate is the seam four painters and two exporters are written against, and an \
         exported symbol with no caller is an unverified claim about it. Either give each of these \
         a caller and a test, or delete it:\n  {}",
        orphans.join("\n  ")
    );
}

#[test]
fn the_gate_can_tell_a_named_symbol_from_an_unnamed_one() {
    // The gate rests on four small functions, so all four are checked directly — otherwise a parser
    // that recognised nothing would report no orphans and pass for ever.
    assert_eq!(
        declared_name("    pub fn record_count(&self) -> u32 {", "fn "),
        Some("record_count")
    );
    assert_eq!(
        declared_name("    pub const fn opcode(&self) -> u8 {", "fn "),
        Some("opcode")
    );
    assert_eq!(
        declared_name("pub const HEADER_BYTES: usize = 32;", "const "),
        Some("HEADER_BYTES")
    );
    assert_eq!(declared_name("    fn private_helper() {", "fn "), None);
    assert_eq!(
        declared_name("    pub(crate) fn write_u32(into: &mut Vec<u8>)", "fn "),
        None,
        "only a fully public symbol is a claim about the seam"
    );

    assert_eq!(declared_variant("    Pop,"), Some("Pop"));
    assert_eq!(declared_variant("    Commands = 1,"), Some("Commands"));
    assert_eq!(declared_variant("    Solid(Color),"), Some("Solid"));
    assert_eq!(declared_variant("    FillPath {"), Some("FillPath"));
    assert_eq!(
        declared_variant("    /// A doc comment, Capitalised, with a comma,"),
        None
    );
    assert_eq!(declared_variant("    #[default]"), None);
    assert_eq!(
        declared_variant("    pub geometry: ResourceIndex,"),
        None,
        "a struct field is not a variant"
    );
    assert_eq!(
        declared_variant("        Self::Pop => opcode::POP,"),
        None,
        "a match arm names a variant, it does not declare one"
    );

    assert!(mentions("a.record_count()", "record_count"));
    assert!(mentions("use crate::record_count;", "record_count"));
    assert!(
        !mentions("a.record_counts()", "record_count"),
        "a substring is not a mention, or every short name would look reachable"
    );
    assert!(!mentions("my_record_count", "record_count"));

    assert!(mentions_as_a_path("FillStyle::None", "None"));
    assert!(mentions_as_a_path("matches!(self, Self::None)", "None"));
    assert!(
        !mentions_as_a_path("        return None;", "None"),
        "a bare `None` is `Option`'s, and counting it would pass every `None` variant in the \
         crate without anybody ever constructing one — which is the whole reason variants get the \
         stricter rule"
    );
    assert!(!mentions_as_a_path("Some(value)", "Some"));

    // And the exception list is empty, which is what makes this a gate rather than a list.
    assert!(
        ALLOWED_WITHOUT_A_CALLER.is_empty(),
        "an exception must be argued for: {ALLOWED_WITHOUT_A_CALLER:?}"
    );
}
